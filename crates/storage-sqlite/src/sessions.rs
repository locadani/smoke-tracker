//! Login sessions. Bookkeeping, not evidence: not audited.

use chrono::{DateTime, Utc};
use smoke_domain::records::StoredSession;
use smoke_domain::repos::{SessionRepo, StoreResult};
use smoke_domain::types::UserId;
use sqlx::SqliteConnection;

use crate::SqliteStore;
use crate::convert::{instant, micros};
use crate::error::IntoStore;

struct SessionRow {
    id_hash: String,
    user_id: Option<i64>,
    data: String,
    expires_at: i64,
}

impl SessionRow {
    fn into_session(self) -> StoreResult<StoredSession> {
        Ok(StoredSession {
            id_hash: self.id_hash,
            user_id: self.user_id.map(UserId),
            data: self.data,
            expires_at: instant(self.expires_at)?,
        })
    }
}

/// Ends every session of `user`; called inside the transaction that changes their password or
/// expires them.
pub(crate) async fn delete_for_user(conn: &mut SqliteConnection, user: UserId) -> StoreResult<()> {
    sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user.0)
        .execute(conn)
        .await
        .db()?;
    Ok(())
}

impl SessionRepo for SqliteStore {
    async fn session(
        &self,
        id_hash: &str,
        now: DateTime<Utc>,
    ) -> StoreResult<Option<StoredSession>> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id_hash, user_id, data, expires_at FROM sessions \
             WHERE id_hash = $1 AND expires_at > $2",
            id_hash,
            micros(now),
        )
        .fetch_optional(&mut *self.read().await?)
        .await
        .db()?
        .map(SessionRow::into_session)
        .transpose()
    }

    async fn create_session(&self, session: &StoredSession) -> StoreResult<()> {
        sqlx::query!(
            "INSERT INTO sessions (id_hash, user_id, data, expires_at) VALUES ($1, $2, $3, $4)",
            session.id_hash,
            session.user_id.map(|u| u.0),
            session.data,
            micros(session.expires_at),
        )
        .execute(&mut *self.write().await?)
        .await
        .db()?;
        Ok(())
    }

    async fn save_session(&self, session: &StoredSession) -> StoreResult<()> {
        sqlx::query!(
            "INSERT INTO sessions (id_hash, user_id, data, expires_at) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (id_hash) DO UPDATE SET user_id = excluded.user_id, \
             data = excluded.data, expires_at = excluded.expires_at",
            session.id_hash,
            session.user_id.map(|u| u.0),
            session.data,
            micros(session.expires_at),
        )
        .execute(&mut *self.write().await?)
        .await
        .db()?;
        Ok(())
    }

    async fn delete_session(&self, id_hash: &str) -> StoreResult<()> {
        sqlx::query!("DELETE FROM sessions WHERE id_hash = $1", id_hash)
            .execute(&mut *self.write().await?)
            .await
            .db()?;
        Ok(())
    }

    async fn delete_expired_sessions(&self, now: DateTime<Utc>) -> StoreResult<u64> {
        let done = sqlx::query!("DELETE FROM sessions WHERE expires_at <= $1", micros(now))
            .execute(&mut *self.write().await?)
            .await
            .db()?;
        Ok(done.rows_affected())
    }
}
