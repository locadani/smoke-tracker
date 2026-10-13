//! Server-side sessions (spec §8.4): `tower-sessions` keeps the cookie and the session data;
//! this module stores them through the domain's [`SessionRepo`], so both database backends
//! work without a second database driver.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use smoke_domain::clock::Clock;
use smoke_domain::records::StoredSession;
use smoke_domain::repos::{Conflict, SessionRepo, StoreError};
use smoke_domain::types::UserId;
use tower_sessions::cookie::SameSite;
use tower_sessions::cookie::time::{Duration, OffsetDateTime};
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{self, ExpiredDeletion, SessionStore};
use tower_sessions::{Expiry, SessionManagerLayer};

use crate::config::BaseUrl;

/// The signed-in user's id, as an `i64`.
pub(crate) const USER_ID_KEY: &str = "user_id";
/// Over https the cookie is `__Host-` prefixed: Secure, `Path=/`, no `Domain`.
pub(crate) const SECURE_COOKIE: &str = "__Host-session";
/// Over plain http (localhost only) browsers refuse `Secure` cookies.
pub(crate) const LOCAL_COOKIE: &str = "session";
pub(crate) const LIFETIME_DAYS: i64 = 30;

/// Sessions live 30 days from their last save.
pub(crate) fn layer<S>(
    store: S,
    clock: Arc<dyn Clock>,
    base_url: &BaseUrl,
) -> SessionManagerLayer<DbSessions<S>>
where
    S: SessionRepo + Clone + 'static,
{
    let name = if base_url.is_https() {
        SECURE_COOKIE
    } else {
        LOCAL_COOKIE
    };
    SessionManagerLayer::new(DbSessions::new(store, clock))
        .with_name(name)
        .with_secure(base_url.is_https())
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_path("/")
        .with_expiry(Expiry::OnInactivity(Duration::days(LIFETIME_DAYS)))
}

/// A `tower-sessions` store over any storage backend. Public so the binary can run its
/// [`ExpiredDeletion`] sweep.
#[derive(Clone)]
pub struct DbSessions<S> {
    store: S,
    clock: Arc<dyn Clock>,
}

impl<S> DbSessions<S> {
    pub fn new(store: S, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }
}

impl<S> fmt::Debug for DbSessions<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DbSessions")
    }
}

/// The database key of a session: SHA-256 of its id, so a copy of the database holds no
/// usable session ids.
fn id_hash(id: &Id) -> String {
    hex::encode(Sha256::digest(id.0.to_le_bytes()))
}

fn backend(error: impl fmt::Display) -> session_store::Error {
    session_store::Error::Backend(error.to_string())
}

fn to_stored(record: &Record) -> session_store::Result<StoredSession> {
    let micros = record.expiry_date.unix_timestamp_nanos() / 1_000;
    let expires_at = i64::try_from(micros)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_micros)
        .ok_or_else(|| session_store::Error::Encode("expiry out of range".to_owned()))?;
    Ok(StoredSession {
        id_hash: id_hash(&record.id),
        user_id: record
            .data
            .get(USER_ID_KEY)
            .and_then(serde_json::Value::as_i64)
            .map(UserId),
        data: serde_json::to_string(&record.data)
            .map_err(|e| session_store::Error::Encode(e.to_string()))?,
        expires_at,
    })
}

fn to_record(id: Id, stored: StoredSession) -> session_store::Result<Record> {
    let nanos = i128::from(stored.expires_at.timestamp_micros()) * 1_000;
    Ok(Record {
        id,
        data: serde_json::from_str(&stored.data)
            .map_err(|e| session_store::Error::Decode(e.to_string()))?,
        expiry_date: OffsetDateTime::from_unix_timestamp_nanos(nanos)
            .map_err(|e| session_store::Error::Decode(e.to_string()))?,
    })
}

#[async_trait]
impl<S> SessionStore for DbSessions<S>
where
    S: SessionRepo + Clone + 'static,
{
    /// Picks a fresh id on the (astronomically unlikely) collision with a stored one.
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        loop {
            match self.store.create_session(&to_stored(record)?).await {
                Ok(()) => return Ok(()),
                Err(StoreError::Conflict(Conflict::Duplicate)) => record.id = Id::default(),
                Err(error) => return Err(backend(error)),
            }
        }
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        self.store
            .save_session(&to_stored(record)?)
            .await
            .map_err(backend)
    }

    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        self.store
            .session(&id_hash(id), self.clock.now())
            .await
            .map_err(backend)?
            .map(|stored| to_record(*id, stored))
            .transpose()
    }

    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        self.store
            .delete_session(&id_hash(id))
            .await
            .map_err(backend)
    }
}

/// tower-sessions' interface for removing expired sessions (spec §11). The binary calls it
/// from its own hourly loop rather than `continuously_delete_expired`, which skips the run at
/// startup and stops for good after the first error.
#[async_trait]
impl<S> ExpiredDeletion for DbSessions<S>
where
    S: SessionRepo + Clone + 'static,
{
    async fn delete_expired(&self) -> session_store::Result<()> {
        let removed = self
            .store
            .delete_expired_sessions(self.clock.now())
            .await
            .map_err(backend)?;
        if removed > 0 {
            tracing::info!(removed, "deleted expired sessions");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn the_stored_key_is_a_hash_not_the_cookie_value() {
        let id = Id(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
        let key = id_hash(&id);
        assert_eq!(key.len(), 64);
        assert!(!key.contains(&id.to_string()));
        assert_eq!(key, id_hash(&id));
        assert_ne!(key, id_hash(&Id(1)));
    }

    #[test]
    fn records_round_trip_through_the_stored_form() {
        let record = Record {
            id: Id(42),
            data: HashMap::from([
                (USER_ID_KEY.to_owned(), serde_json::json!(7)),
                ("note".to_owned(), serde_json::json!("kept as JSON")),
            ]),
            expiry_date: OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        };
        let stored = to_stored(&record).unwrap();
        assert_eq!(stored.user_id, Some(UserId(7)));
        assert_eq!(stored.expires_at.timestamp(), 1_800_000_000);
        assert_eq!(to_record(Id(42), stored).unwrap(), record);
    }

    #[test]
    fn a_session_without_a_user_stores_no_user_id() {
        let record = Record {
            id: Id(1),
            data: HashMap::new(),
            expiry_date: OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        };
        assert_eq!(to_stored(&record).unwrap().user_id, None);
    }
}
