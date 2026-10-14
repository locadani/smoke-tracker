//! Exercise 12: a report that normalises its input. Vectors, sorting and `Option`.
//!
//! Practises: `docs/learning/04-logging-flow.md`, section 4.
//! Mirrors: `smoke_domain::records::ReportValues`.
//! Run: `cargo test --test ex12_report_values`
//!
//! Hint: `new` takes ownership of the vectors (`mut rooms`), sorts and de-duplicates them, and
//! stores them in a struct whose fields are private, so every `ReportValues` that exists is
//! already in canonical form. `sort_unstable_by_key`, `dedup`, `windows(2)`, `Iterator::max`,
//! `Option::map`, `Option::filter` and `str::trim` are the tools. A `ReportValues` is the only
//! way to build the data, so the getters can simply return borrowed slices.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomLevel {
    pub room: u32,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportError {
    /// A room appears twice; the number is its id.
    RoomListedTwice(u32),
    /// A level above 10; the number is the level.
    InvalidLevel(u8),
    /// A note of more than `NOTE_MAX_CHARS` characters; the number is its length.
    NoteTooLong(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportValues {
    rooms: Vec<RoomLevel>,
    symptoms: Vec<u32>,
    note: Option<String>,
}

impl ReportValues {
    pub const NOTE_MAX_CHARS: usize = 2000;

    /// Rooms sorted by room id (and no room twice), symptoms sorted and unique, the note
    /// trimmed with a blank note meaning no note. Checks, in this order: levels, duplicate
    /// rooms, the note's length in characters (after trimming).
    pub fn new(
        mut rooms: Vec<RoomLevel>,
        mut symptoms: Vec<u32>,
        note: Option<&str>,
    ) -> Result<Self, ReportError> {
        todo!()
    }

    pub fn rooms(&self) -> &[RoomLevel] {
        todo!()
    }

    pub fn symptoms(&self) -> &[u32] {
        todo!()
    }

    pub fn note(&self) -> Option<&str> {
        todo!()
    }

    /// The level reported for `room`; `None` when the room was not checked.
    pub fn level_in(&self, room: u32) -> Option<u8> {
        todo!()
    }

    /// The report's intensity: its highest room level, 0 without rooms.
    pub fn intensity(&self) -> u8 {
        todo!()
    }

    /// A no-smell report: every room listed is at 0, or none is listed (the whole home).
    pub fn is_no_smell(&self) -> bool {
        todo!()
    }
}
