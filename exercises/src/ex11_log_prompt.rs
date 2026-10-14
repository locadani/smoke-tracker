//! Exercise 11: which question does the Log screen ask? Enums, tuples and `match`.
//!
//! Practises: `docs/learning/04-logging-flow.md`, section 2.
//! Mirrors: `smoke_domain::events::{classify_observed, log_prompt, no_smell_prompt,
//! new_event_state, large_gap_indices}` and `smoke_domain::schedule::DailySchedule`.
//! Run: `cargo test --test ex11_log_prompt`
//!
//! Hint: match on a tuple of everything the decision depends on, one arm per case, and let the
//! compiler check that every combination is covered: `match (observed, open, unseen) { ... }`.
//! Times are seconds since the start of a day-zero UTC calendar; no time zones here.

pub const BACKDATE_THRESHOLD: u64 = 10 * 60;
pub const FUTURE_TOLERANCE: u64 = 60;
pub const LARGE_GAP: u64 = 3 * 60 * 60;
pub const DAY: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedTime {
    Now,
    Backdated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    ObservedInFuture,
}

/// More than `FUTURE_TOLERANCE` ahead of `now` is an error (phone clocks drift a little);
/// more than `BACKDATE_THRESHOLD` behind is `Backdated`; anything between is `Now`. Be careful
/// not to subtract below zero from a `u64`: use `saturating_sub`.
pub fn classify_observed(observed_at: u64, now: u64) -> Result<ObservedTime, TimeError> {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSummary {
    pub id: u32,
    pub opened_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogPrompt {
    /// Nothing to ask.
    Direct,
    /// "Same smell still going?" about the open event.
    SameSmell(EventSummary),
    /// "Which event was this?" for a backdated report.
    WhichEvent,
    /// "This event was auto-closed: was it still going?"
    AutoClosed(EventSummary),
}

/// A backdated report always asks which event. Otherwise an open event is asked about first,
/// then an auto-closed one nobody has seen, else nothing.
pub fn log_prompt(
    observed: ObservedTime,
    open_event: Option<EventSummary>,
    unseen_auto_closed: Option<EventSummary>,
) -> LogPrompt {
    todo!()
}

/// A no-smell report logged now asks nothing (it joins the open event or stands alone); a
/// backdated one asks which event.
pub fn no_smell_prompt(observed: ObservedTime) -> LogPrompt {
    todo!()
}

/// A daily boundary such as 04:00, as seconds after midnight UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyBoundary {
    pub offset: u64,
}

impl DailyBoundary {
    /// The most recent boundary at or before `now`: today's if it has passed, else yesterday's.
    pub fn last_before(&self, now: u64) -> u64 {
        todo!()
    }

    /// Whether a boundary lies in `(since, now]`.
    pub fn passed_since(&self, since: u64, now: u64) -> bool {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewEventState {
    Open,
    ClosedBackdated,
}

/// At most one event is open at a time and nothing stays open across a boundary: a new event
/// is born closed when another one is open or a boundary has passed since `observed_at`.
pub fn new_event_state(
    observed_at: u64,
    now: u64,
    boundary: &DailyBoundary,
    another_event_open: bool,
) -> NewEventState {
    todo!()
}

/// The indices of reports that follow a gap longer than `LARGE_GAP`, in a list sorted by time.
/// `[0, 100, 100 + LARGE_GAP + 1]` gives `[2]`. Use `windows(2)`.
pub fn large_gap_indices(observed_times: &[u64]) -> Vec<usize> {
    todo!()
}
