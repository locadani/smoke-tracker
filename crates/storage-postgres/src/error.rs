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
            ErrorKind::UniqueViolation => return unique_conflict(db.constraint()).into(),
            ErrorKind::ForeignKeyViolation => return Conflict::UnknownReference.into(),
            _ => {}
        }
    }
    StoreError::Backend(Box::new(error))
}

/// Postgres reports the violated constraint or index by name; the migration names them.
fn unique_conflict(constraint: Option<&str>) -> Conflict {
    match constraint {
        Some("users_username_unique") => Conflict::UsernameTaken,
        Some("events_one_open") => Conflict::AnotherEventOpen,
        Some("audit_log_prev_hash_unique" | "audit_log_hash_unique") => Conflict::ChainForked,
        _ => Conflict::Duplicate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_violations_map_to_their_conflict() {
        assert_eq!(
            unique_conflict(Some("users_username_unique")),
            Conflict::UsernameTaken
        );
        assert_eq!(
            unique_conflict(Some("events_one_open")),
            Conflict::AnotherEventOpen
        );
        assert_eq!(
            unique_conflict(Some("audit_log_prev_hash_unique")),
            Conflict::ChainForked
        );
        assert_eq!(
            unique_conflict(Some("rooms_i18n_key_unique")),
            Conflict::Duplicate
        );
        assert_eq!(unique_conflict(None), Conflict::Duplicate);
    }
}
