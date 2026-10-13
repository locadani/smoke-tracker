//! Database values ↔ domain values. Instants are microseconds since the Unix epoch.

use std::fmt::Display;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use smoke_domain::DomainError;
use smoke_domain::repos::{StoreError, StoreResult};

pub(crate) fn micros(t: DateTime<Utc>) -> i64 {
    t.timestamp_micros()
}

pub(crate) fn instant(value: i64) -> StoreResult<DateTime<Utc>> {
    DateTime::from_timestamp_micros(value)
        .ok_or_else(|| corrupt(format!("timestamp {value} is out of range")))
}

pub(crate) fn opt_instant(value: Option<i64>) -> StoreResult<Option<DateTime<Utc>>> {
    value.map(instant).transpose()
}

pub(crate) fn parse<T: FromStr<Err = DomainError>>(text: &str) -> StoreResult<T> {
    text.parse().map_err(corrupt)
}

pub(crate) fn corrupt(reason: impl Display) -> StoreError {
    StoreError::Corrupt(reason.to_string())
}
