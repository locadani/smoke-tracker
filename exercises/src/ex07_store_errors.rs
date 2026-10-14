//! Exercise 7: error types, `From` and the `?` operator.
//!
//! Practises: `docs/learning/02-storage.md`, section 5.
//! Mirrors: `smoke_domain::repos::{Conflict, StoreError}` and the helpers in
//! `smoke_storage_sqlite::{error, convert}` (`unique_conflict`, `corrupt`).
//! Run: `cargo test --test ex07_store_errors`
//!
//! Hint: `?` on a `Result<T, E>` returns early with `Err(From::from(e))`, so a `From` impl is
//! all it takes to convert one error type into another. `iter.map(f).collect::<Result<Vec<_>, _>>()`
//! stops at the first `Err`. For `Error::source`, return the boxed error as `&dyn Error`.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    UsernameTaken,
    AnotherEventOpen,
    ChainForked,
    Duplicate,
}

/// SQLite words a unique-constraint failure like `UNIQUE constraint failed: users.username`.
/// Which column or index is named decides the conflict; anything unrecognised is `Duplicate`:
///
/// - mentions `users.username`: `UsernameTaken`
/// - mentions `events_one_open`: `AnotherEventOpen`
/// - mentions `audit_log.`: `ChainForked`
pub fn unique_conflict(message: &str) -> Conflict {
    todo!()
}

#[derive(Debug)]
pub enum StoreError {
    Conflict(Conflict),
    /// Stored data breaks a rule the app guarantees.
    Corrupt(String),
    /// The database itself failed; the original error is kept as the source.
    Backend(Box<dyn Error + Send + Sync>),
}

impl fmt::Display for StoreError {
    /// `Conflict`: "that write conflicts with stored data"; `Corrupt`: "stored data is corrupt:
    /// <reason>"; `Backend`: "database error" (the details are in `source`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for StoreError {
    /// Only `Backend` has a source.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        todo!()
    }
}

impl From<Conflict> for StoreError {
    fn from(conflict: Conflict) -> Self {
        todo!()
    }
}

/// A `Corrupt` error carrying `reason`'s text.
pub fn corrupt(reason: impl fmt::Display) -> StoreError {
    todo!()
}

/// A level as stored: an `i64` from 0 to 10. Anything else is `Corrupt`, for example
/// "level 11 is out of range" or "level -1 is out of range". Use `u8::try_from` and `?`.
pub fn read_level(stored: i64) -> Result<u8, StoreError> {
    todo!()
}

/// All the levels, or the first corrupt one. Use `collect::<Result<_, _>>()`.
pub fn read_levels(stored: &[i64]) -> Result<Vec<u8>, StoreError> {
    todo!()
}

/// A level from text such as a form field or a column read as `TEXT`: parse it, then check it
/// like `read_level`. A parse failure is `Corrupt` too. Use `?` twice and `corrupt` for the
/// parse error.
pub fn read_level_text(stored: &str) -> Result<u8, StoreError> {
    todo!()
}
