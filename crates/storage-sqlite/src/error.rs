//! Turns sqlx errors into the storage boundary's errors.

use smoke_domain::repos::{Conflict, StoreError, StoreResult};
use sqlx::error::ErrorKind;

pub(crate) trait IntoStore<T> {
    fn db(self) -> StoreResult<T>;
}

impl<T> IntoStore<T> for Result<T, sqlx::Error> {
    fn db(self) -> StoreResult<T> {
        self.map_err(into_store_error)
    }
}

fn into_store_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &error {
        match db.kind() {
            ErrorKind::UniqueViolation => return unique_conflict(db.message()).into(),
            ErrorKind::ForeignKeyViolation => return Conflict::UnknownReference.into(),
            ErrorKind::CheckViolation => return Conflict::RuleViolated.into(),
            _ => {}
        }
    }
    StoreError::Backend(Box::new(error))
}

/// SQLite names the violated columns or index in the message, e.g.
/// `UNIQUE constraint failed: users.username`.
fn unique_conflict(message: &str) -> Conflict {
    if message.contains("users.username") {
        Conflict::UsernameTaken
    } else if message.contains("events_one_open") {
        Conflict::AnotherEventOpen
    } else if message.contains("audit_log.") {
        Conflict::ChainForked
    } else {
        Conflict::Duplicate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_violations_map_to_their_conflict() {
        assert_eq!(
            unique_conflict("UNIQUE constraint failed: users.username"),
            Conflict::UsernameTaken
        );
        assert_eq!(
            unique_conflict("UNIQUE constraint failed: index 'events_one_open'"),
            Conflict::AnotherEventOpen
        );
        assert_eq!(
            unique_conflict("UNIQUE constraint failed: audit_log.prev_hash"),
            Conflict::ChainForked
        );
        assert_eq!(
            unique_conflict("UNIQUE constraint failed: rooms.i18n_key"),
            Conflict::Duplicate
        );
    }
}
