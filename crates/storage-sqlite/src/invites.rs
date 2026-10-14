//! Invites: one-time links for account setup and password reset (spec §8.3). Only the token's
//! SHA-256 is stored, and it is never read back into a record.

use chrono::{DateTime, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{Invite, NewInvite};
use smoke_domain::repos::{Conflict, InviteRepo, InviteWrites, StoreError, StoreResult};
use smoke_domain::types::{InviteId, TokenHash, UserId};
use sqlx::SqliteConnection;

use crate::convert::{instant, micros, opt_instant, parse};
use crate::error::IntoStore;
use crate::{SqliteStore, SqliteTx};

struct InviteRow {
    id: i64,
    user_id: i64,
    purpose: String,
    created_by: i64,
    created_at: i64,
    expires_at: i64,
    used_at: Option<i64>,
}

impl InviteRow {
    fn into_invite(self) -> StoreResult<Invite> {
        Ok(Invite {
            id: InviteId(self.id),
            user_id: UserId(self.user_id),
            purpose: parse(&self.purpose)?,
            created_by: UserId(self.created_by),
            created_at: instant(self.created_at)?,
            expires_at: instant(self.expires_at)?,
            used_at: opt_instant(self.used_at)?,
        })
    }
}

async fn require(conn: &mut SqliteConnection, id: InviteId) -> StoreResult<Invite> {
    sqlx::query_as!(
        InviteRow,
        "SELECT id, user_id, purpose, created_by, created_at, expires_at, used_at \
         FROM invites WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .ok_or(StoreError::NotFound {
        entity: AuditEntity::Invite,
        id: id.0,
    })?
    .into_invite()
}

async fn by_token(conn: &mut SqliteConnection, token: &TokenHash) -> StoreResult<Option<Invite>> {
    sqlx::query_as!(
        InviteRow,
        "SELECT id, user_id, purpose, created_by, created_at, expires_at, used_at \
         FROM invites WHERE token_hash = $1",
        token.as_str(),
    )
    .fetch_optional(conn)
    .await
    .db()?
    .map(InviteRow::into_invite)
    .transpose()
}

impl InviteRepo for SqliteStore {
    async fn invite_by_token(&self, token: &TokenHash) -> StoreResult<Option<Invite>> {
        by_token(&mut *self.read().await?, token).await
    }

    async fn delete_expired_invites(&self, now: DateTime<Utc>) -> StoreResult<u64> {
        let done = sqlx::query!("DELETE FROM invites WHERE expires_at <= $1", micros(now))
            .execute(&mut *self.write().await?)
            .await
            .db()?;
        Ok(done.rows_affected())
    }
}

impl InviteWrites for SqliteTx {
    async fn create_invite(&mut self, new: NewInvite) -> StoreResult<Invite> {
        sqlx::query!(
            "DELETE FROM invites WHERE user_id = $1 AND used_at IS NULL",
            new.user_id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        let id = sqlx::query_scalar!(
            "INSERT INTO invites (user_id, purpose, token_hash, created_by, created_at, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id AS \"id!\"",
            new.user_id.0,
            new.purpose.as_str(),
            new.token_hash.as_str(),
            new.created_by.0,
            micros(self.now()),
            micros(new.expires_at),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let invite = require(self.conn(), InviteId(id)).await?;
        self.record(AuditAction::Invite, None, &invite).await?;
        Ok(invite)
    }

    async fn invite_by_token(&mut self, token: &TokenHash) -> StoreResult<Option<Invite>> {
        by_token(self.conn(), token).await
    }

    async fn use_invite(&mut self, id: InviteId) -> StoreResult<Invite> {
        let before = require(self.conn(), id).await?;
        if before.used_at.is_some() {
            return Err(Conflict::InviteUsed.into());
        }
        sqlx::query!(
            "UPDATE invites SET used_at = $1 WHERE id = $2",
            micros(self.now()),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        let after = require(self.conn(), id).await?;
        self.record(AuditAction::Update, Some(&before), &after)
            .await?;
        Ok(after)
    }
}
