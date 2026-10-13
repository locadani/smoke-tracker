//! Users: accounts, passwords, languages and expiry.

use chrono::{DateTime, Utc};
use smoke_domain::DomainError;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::permissions::check_expiry;
use smoke_domain::records::{NewUser, PasswordHash, User};
use smoke_domain::repos::{StoreError, StoreResult, UserRepo, UserWrites};
use smoke_domain::types::{Locale, Role, UserId, Username};
use sqlx::SqliteConnection;

use crate::convert::{canonical, corrupt, instant, micros, opt_instant, parse};
use crate::error::IntoStore;
use crate::sessions;
use crate::{SqliteStore, SqliteTx};

struct UserRow {
    id: i64,
    username: String,
    password_hash: Option<String>,
    role: String,
    locale: String,
    expires_at: Option<i64>,
    created_at: i64,
}

impl UserRow {
    fn into_user(self) -> StoreResult<User> {
        let username = Username::new(&self.username).map_err(corrupt)?;
        canonical("username", &self.username, username.as_str())?;
        Ok(User {
            id: UserId(self.id),
            username,
            password_hash: self.password_hash.map(PasswordHash::new),
            role: parse(&self.role)?,
            locale: parse(&self.locale)?,
            expires_at: opt_instant(self.expires_at)?,
            created_at: instant(self.created_at)?,
        })
    }
}

async fn fetch(conn: &mut SqliteConnection, id: UserId) -> StoreResult<Option<User>> {
    sqlx::query_as!(
        UserRow,
        "SELECT id, username, password_hash, role, locale, expires_at, created_at \
         FROM users WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .map(UserRow::into_user)
    .transpose()
}

async fn require(conn: &mut SqliteConnection, id: UserId) -> StoreResult<User> {
    fetch(conn, id).await?.ok_or(StoreError::NotFound {
        entity: AuditEntity::User,
        id: id.0,
    })
}

pub(crate) async fn fetch_all(conn: &mut SqliteConnection) -> StoreResult<Vec<User>> {
    sqlx::query_as!(
        UserRow,
        "SELECT id, username, password_hash, role, locale, expires_at, created_at \
         FROM users ORDER BY username"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(UserRow::into_user)
    .collect()
}

/// Re-reads the user after an update and audits the change.
async fn audited(tx: &mut SqliteTx, action: AuditAction, before: User) -> StoreResult<User> {
    let after = require(tx.conn(), before.id).await?;
    tx.record(action, Some(&before), &after).await?;
    Ok(after)
}

impl UserRepo for SqliteStore {
    async fn user(&self, id: UserId) -> StoreResult<Option<User>> {
        fetch(&mut *self.read().await?, id).await
    }

    async fn user_by_username(&self, username: &Username) -> StoreResult<Option<User>> {
        sqlx::query_as!(
            UserRow,
            "SELECT id, username, password_hash, role, locale, expires_at, created_at \
             FROM users WHERE username = $1",
            username.as_str(),
        )
        .fetch_optional(&mut *self.read().await?)
        .await
        .db()?
        .map(UserRow::into_user)
        .transpose()
    }

    async fn users(&self) -> StoreResult<Vec<User>> {
        fetch_all(&mut *self.read().await?).await
    }
}

impl UserWrites for SqliteTx {
    async fn create_user(&mut self, new: NewUser) -> StoreResult<User> {
        let now = self.now();
        check_expiry(new.role, new.expires_at, now)?;
        let id = sqlx::query_scalar!(
            "INSERT INTO users (username, password_hash, role, locale, expires_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
            new.username.as_str(),
            new.password_hash.as_ref().map(PasswordHash::as_str),
            new.role.as_str(),
            new.locale.as_str(),
            new.expires_at.map(micros),
            micros(now),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let user = require(self.conn(), UserId(id)).await?;
        self.record(AuditAction::Create, None, &user).await?;
        Ok(user)
    }

    async fn set_password(&mut self, id: UserId, hash: PasswordHash) -> StoreResult<User> {
        let before = require(self.conn(), id).await?;
        sqlx::query!(
            "UPDATE users SET password_hash = $1 WHERE id = $2",
            hash.as_str(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        sessions::delete_for_user(self.conn(), id).await?;
        audited(self, AuditAction::PasswordChange, before).await
    }

    async fn set_locale(&mut self, id: UserId, locale: Locale) -> StoreResult<User> {
        let before = require(self.conn(), id).await?;
        sqlx::query!(
            "UPDATE users SET locale = $1 WHERE id = $2",
            locale.as_str(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Update, before).await
    }

    async fn expire_user(&mut self, id: UserId) -> StoreResult<User> {
        let before = require(self.conn(), id).await?;
        if before.role == Role::Resident {
            return Err(DomainError::ResidentsNeverExpire.into());
        }
        let now = micros(self.now());
        sqlx::query!("UPDATE users SET expires_at = $1 WHERE id = $2", now, id.0)
            .execute(self.conn())
            .await
            .db()?;
        sessions::delete_for_user(self.conn(), id).await?;
        audited(self, AuditAction::Expire, before).await
    }

    async fn reactivate_user(
        &mut self,
        id: UserId,
        expires_at: Option<DateTime<Utc>>,
    ) -> StoreResult<User> {
        let before = require(self.conn(), id).await?;
        check_expiry(before.role, expires_at, self.now())?;
        sqlx::query!(
            "UPDATE users SET expires_at = $1 WHERE id = $2",
            expires_at.map(micros),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Reactivate, before).await
    }
}
