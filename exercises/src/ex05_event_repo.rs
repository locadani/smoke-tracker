//! Exercise 5: a repository trait, an in-memory implementation and a generic function.
//!
//! Practises: `docs/learning/02-storage.md`, section 1.
//! Mirrors: `smoke_domain::repos::{EventRepo, EventWrites}` and the `events_one_open` index.
//! Run: `cargo test --test ex05_event_repo`
//!
//! Hint: `MemoryEvents` keeps a `Vec<Event>`; ids are 1, 2, 3... in creation order. The database
//! enforces "at most one open event" with a unique index; here `create_event` must enforce it
//! by returning `Conflict::AnotherEventOpen`. `close_open_event` is generic: it works with ANY
//! `EventRepo`, so write it using only the trait's methods, never `MemoryEvents`' fields.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub id: u32,
    pub owner: u32,
    pub opened_at: u64,
    pub closed_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    /// Another event is already open.
    AnotherEventOpen,
    /// Closing an event that is already closed.
    EventNotOpen,
    /// No event has this id.
    UnknownEvent,
    /// Closing before the event opened (the table's `closed_at >= opened_at` check).
    RuleViolated,
}

pub trait EventRepo {
    /// The event that is open, if any.
    fn open_event(&self) -> Option<Event>;
    fn event(&self, id: u32) -> Option<Event>;
    /// Opens a new event, or `Conflict::AnotherEventOpen`.
    fn create_event(&mut self, owner: u32, opened_at: u64) -> Result<Event, Conflict>;
    /// Closes event `id` at `at`. Unknown id: `UnknownEvent`; already closed: `EventNotOpen`;
    /// `at` before it opened: `RuleViolated`. Checked in that order.
    fn close_event(&mut self, id: u32, at: u64) -> Result<Event, Conflict>;
}

#[derive(Debug, Default)]
pub struct MemoryEvents {
    events: Vec<Event>,
}

impl EventRepo for MemoryEvents {
    fn open_event(&self) -> Option<Event> {
        todo!()
    }

    fn event(&self, id: u32) -> Option<Event> {
        todo!()
    }

    fn create_event(&mut self, owner: u32, opened_at: u64) -> Result<Event, Conflict> {
        todo!()
    }

    fn close_event(&mut self, id: u32, at: u64) -> Result<Event, Conflict> {
        todo!()
    }
}

/// Closes the open event at `at`, if there is one, and returns it. Nothing open is not an
/// error: `Ok(None)`.
pub fn close_open_event<R: EventRepo>(repo: &mut R, at: u64) -> Result<Option<Event>, Conflict> {
    todo!()
}
