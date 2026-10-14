//! Exercise 14: when does an account expire? Enums that carry data, and `match` guards.
//!
//! Practises: `docs/learning/05-people-and-pwa.md`, section 2.
//! Mirrors: `smoke_domain::people::ExpiryPick::resolve` and
//! `smoke_domain::permissions::check_expiry`.
//! Run: `cargo test --test ex14_expiry_pick`
//!
//! Hint: time here is seconds since 1970-01-01 00:00 UTC, and a day is `DAY` seconds; no time
//! zones or daylight saving (the app uses `chrono` and `DailySchedule` for those). Day number
//! `d` is the `d`-th day since the epoch, and 1970-01-01 was a Thursday. For `check_expiry`,
//! `match (role, expires_at)` with a guard (`(_, Some(at)) if at <= now => ...`) reads like the
//! rules.

use crate::ex02_permissions::Role;

pub const DAY: u64 = 24 * 60 * 60;

/// 0 for Monday up to 6 for Sunday, for the day with this number (day 0 was a Thursday).
pub fn weekday_from_monday(day: u64) -> u64 {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryPick {
    OneDay,
    /// Until the start of the coming Monday; on a Monday, the next one.
    Weekend,
    OneWeek,
    TwoWeeks,
    /// Until the end of the day with this number (so that whole day is included).
    Until(u64),
    Never,
}

impl ExpiryPick {
    /// The second the account expires, `None` for never. `OneDay`, `OneWeek` and `TwoWeeks`
    /// count from `now`.
    pub fn resolve(self, now: u64) -> Option<u64> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryError {
    ResidentsNeverExpire,
    GuestsMustExpire,
    ExpiryNotInFuture,
}

/// Residents never expire; guests always do; a viewer may or may not. Any expiry must lie in
/// the future (strictly after `now`). Checked in this order: resident, guest, future.
pub fn check_expiry(role: Role, expires_at: Option<u64>, now: u64) -> Result<(), ExpiryError> {
    todo!()
}
