//! Exercise 17: turning reports into chart numbers. `BTreeMap`, `or_default`, sorting and
//! `Option` ordering.
//!
//! Practises: `docs/learning/06-graphs.md`, section 3.
//! Mirrors: `smoke_domain::graphs::{intensity_lines, room_hours, symptom_counts, area_levels,
//! LevelTally}`.
//! Run: `cargo test --test ex17_series`
//!
//! Hint: group with a `BTreeMap` (its iteration is sorted by key, so the output is in a stable
//! order for free): `map.entry(key).or_default()` gives `&mut V` for a key, inserting
//! `V::default()` the first time. `sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)))` sorts
//! descending by one field and then ascending by another. `Option<T>` orders `None` below every
//! `Some`, so `highest.max(Some(level))` keeps the larger of "nothing yet" and a level.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Inside,
    Common,
}

/// One smell report, as far as the graphs care.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub user: u32,
    /// Seconds since the epoch.
    pub at: u64,
    /// The local hour of the day, 0 to 23.
    pub hour: usize,
    /// `(room, level)` pairs; empty means the whole home was fine.
    pub rooms: Vec<(u32, u8)>,
    pub symptoms: Vec<u32>,
}

/// One person's points as `(time, intensity)`, in time order. A report's intensity is its
/// highest room level, 0 without rooms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonLine {
    pub user: u32,
    pub points: Vec<(u64, u8)>,
}

/// One line per person, ordered by user id.
pub fn intensity_lines(reports: &[Report]) -> Vec<PersonLine> {
    todo!()
}

/// The levels one room was given at one hour: how many reports checked it (a clean check, level
/// 0, included) and the sum of their levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LevelTally {
    pub checks: u32,
    pub total: u32,
}

impl LevelTally {
    /// The average level, or `None` when nobody checked: `total / checks` as `f64`.
    pub fn mean(&self) -> Option<f64> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomHours {
    pub room: u32,
    pub hours: [LevelTally; 24],
}

/// A row per room that at least one report names, ordered by room id; each row has the tallies
/// of the 24 hours. A report naming no room adds nothing.
pub fn room_hours(reports: &[Report]) -> Vec<RoomHours> {
    todo!()
}

/// How many reports name each symptom, most frequent first, then by symptom id. Symptoms
/// nobody reported are left out.
pub fn symptom_counts(reports: &[Report]) -> Vec<(u32, usize)> {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AreaLevels {
    pub inside: Option<u8>,
    pub common: Option<u8>,
}

/// The highest room level reported inside the home and in common areas. `areas` says which
/// area each room is in; a room missing from it is skipped. `None` where no room of that area
/// was checked.
pub fn area_levels(reports: &[Report], areas: &[(u32, Area)]) -> AreaLevels {
    todo!()
}
