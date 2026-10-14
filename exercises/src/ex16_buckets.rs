//! Exercise 16: counting per day or per week. `BTreeMap`, `std::iter::successors` and the
//! entry API.
//!
//! Practises: `docs/learning/06-graphs.md`, section 2.
//! Mirrors: `smoke_domain::graphs::{Period, events_per_bucket}`.
//! Run: `cargo test --test ex16_buckets`
//!
//! Hint: a *bucket* is the first day of the day or week a date falls in, so a graph can show
//! one bar per bucket. Days are numbers (day 0 = 1970-01-01, a Thursday); a week runs Monday to
//! Sunday and is named by its Monday. `std::iter::successors(Some(first), |&d| Some(d + step))`
//! produces first, first + step, first + 2 * step... and `take_while` stops it.
//! In `events_per_bucket`, `BTreeMap::entry(event).and_modify(|d| *d = (*d).min(day))
//! .or_insert(day)` keeps the earliest day per event.

use std::collections::BTreeMap;

/// Given: 0 for Monday up to 6 for Sunday.
pub fn weekday_from_monday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// `len` days from `first`, counted per day.
    Days { first: i64, len: i64 },
    /// `len` days from `first`, counted per week (Monday to Sunday).
    Weeks { first: i64, len: i64 },
}

impl Period {
    /// The first day after the period.
    pub fn end(&self) -> i64 {
        todo!()
    }

    /// The first day of the bucket that holds `day`: the day itself, or the Monday of its week.
    pub fn bucket_of(&self, day: i64) -> i64 {
        todo!()
    }

    /// The first day of every bucket, in order. For weeks the first bucket may start before
    /// the period does (the Monday before `first`); no bucket starts at or after `end`.
    pub fn buckets(&self) -> Vec<i64> {
        todo!()
    }
}

/// The number of events in each bucket of `period`, every bucket listed (also with 0), in
/// order. `reports` are `(event, day)` pairs; an event counts ONCE, in the bucket of its
/// earliest report; a report without an event (`None`, a clean check) counts for nothing; an
/// event whose earliest day is outside the period is left out.
pub fn events_per_bucket(period: Period, reports: &[(Option<u32>, i64)]) -> Vec<(i64, usize)> {
    todo!()
}
