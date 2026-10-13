//! The logging use cases (spec §7). Each one runs inside a write transaction the caller begins
//! and commits, so every write it makes — closing the open event, opening a new one, the report
//! and its PM2.5 reading — is audited and applied together or not at all. Permissions are
//! checked here, against the data read inside the transaction.

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::DomainError;
use crate::events::{
    NewEventState, ObservedTime, can_join_backdated, classify_observed, new_event_state,
    validate_manual_close,
};
use crate::permissions::{Action, Actor, can};
use crate::records::{
    Closure, Event, NewEvent, NewReading, NewReport, Pm25Reading, ReportValues, SmellReport,
};
use crate::repos::{EventWrites, ReadingWrites, ReportWrites, StoreError, StoreResult};
use crate::schedule::DailySchedule;
use crate::types::{CloseReason, EventId, Pm25, ReportId, UserId};

#[derive(Debug, Error)]
pub enum LogError {
    #[error("not allowed")]
    Forbidden,
    /// No such event or report, or the report was deleted.
    #[error("not found")]
    NotFound,
    /// Reopening while another event is open would break the one-open-event rule.
    #[error("another event is open")]
    AnotherEventOpen,
    /// The answer to the event question no longer fits the data (another phone closed or
    /// opened an event meanwhile): ask again rather than guess.
    #[error("the events changed since the question was asked")]
    Stale,
    /// The time lies outside the event: before it started, or on another day.
    #[error("the time is outside the event")]
    OutsideEvent,
    /// A no-smell report never opens an event (spec §7.1).
    #[error("a no-smell report cannot start an event")]
    NoSmellCannotOpen,
    /// A check stays a no-smell report: a smell is logged as a new report (spec §7.1).
    #[error("a check cannot be corrected to a smell")]
    CheckCannotSmell,
    #[error(transparent)]
    Invalid(#[from] DomainError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// The answer to "Same smell still going?", "Which event was this?" or the auto-close notice.
/// A no-smell report logged now ignores it: it joins the open event or stands alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventChoice {
    /// Start a new event: nothing was asked, "No, it had stopped", or "New event". `replacing`
    /// is the event that was open when the answer was given (`None` if none was), so an event
    /// opened meanwhile by another phone is never closed by a stale answer. For a backdated
    /// no-smell report: no event, a check.
    New { replacing: Option<EventId> },
    /// Join this event: "Yes, update", or the event picked for a past report.
    Join(EventId),
    /// "It was still going": reopen this auto-closed event and join it.
    Continue(EventId),
}

/// What the Log form says about a smell.
#[derive(Debug, Clone, PartialEq)]
pub struct SmellEntry {
    pub observed_at: DateTime<Utc>,
    pub values: ReportValues,
    /// Logged together with the report, in the same event.
    pub pm25: Option<Pm25>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Logged {
    pub report: SmellReport,
    pub reading: Option<Pm25Reading>,
}

/// Closes `event` at its last report (or where it opened, if it has none): the end used by
/// both the "new event" answer and the 04:00 sweep.
pub(crate) async fn close_at_last_report<T: EventWrites>(
    tx: &mut T,
    event: &Event,
    reason: CloseReason,
) -> StoreResult<Event> {
    let last_report = tx.latest_report_at(event.id).await?;
    let at = last_report.map_or(event.opened_at, |at| at.max(event.opened_at));
    tx.close_event(event.id, Closure { at, reason }).await
}

/// Creates the event a report starts: open, or already closed at `at` when another event is
/// open or a 04:00 boundary has passed (spec §7.5).
async fn open_new_event<T: EventWrites>(
    tx: &mut T,
    owner: UserId,
    at: DateTime<Utc>,
    schedule: &DailySchedule,
    now: DateTime<Utc>,
    another_open: bool,
) -> StoreResult<EventId> {
    let closed = match new_event_state(at, now, schedule, another_open) {
        NewEventState::Open => None,
        NewEventState::ClosedBackdated => Some(Closure {
            at,
            reason: CloseReason::Backdated,
        }),
    };
    let new = NewEvent {
        owner_id: owner,
        opened_at: at,
        closed,
    };
    Ok(tx.create_event(new).await?.id)
}

/// Logs a smell report, and its PM2.5 reading if there is one, into the event `choice` names
/// (spec §7.1, §7.2, §7.4, §7.5). A no-smell report never opens or closes an event: logged now
/// it joins the open event, if any; backdated it joins the event picked, or none.
pub async fn log_smell<T>(
    tx: &mut T,
    actor: &Actor,
    entry: SmellEntry,
    choice: EventChoice,
    schedule: &DailySchedule,
    now: DateTime<Utc>,
) -> Result<Logged, LogError>
where
    T: EventWrites + ReportWrites + ReadingWrites,
{
    if !can(actor, Action::LogEntry, now) {
        return Err(LogError::Forbidden);
    }
    let observed_at = entry.observed_at;
    let observed = classify_observed(observed_at, now)?;
    let open = tx.open_event().await?;
    let event_id = match (entry.values.is_no_smell(), observed, choice) {
        (true, ObservedTime::Now, _) => open.map(|event| event.id),
        (true, ObservedTime::Backdated, EventChoice::New { .. }) => None,
        (true, ObservedTime::Backdated, EventChoice::Continue(_)) => return Err(LogError::Stale),
        (_, _, EventChoice::Join(id)) => Some(join(tx, id, observed, observed_at, schedule).await?),
        (false, _, EventChoice::New { replacing }) => {
            if open.as_ref().map(|e| e.id) != replacing {
                return Err(LogError::Stale);
            }
            let another_open = match (observed, &open) {
                (ObservedTime::Now, Some(open)) => {
                    close_at_last_report(tx, open, CloseReason::NewEvent).await?;
                    false
                }
                (_, open) => open.is_some(),
            };
            Some(open_new_event(tx, actor.id, observed_at, schedule, now, another_open).await?)
        }
        (false, _, EventChoice::Continue(id)) => {
            let event = tx.event(id).await?.ok_or(LogError::Stale)?;
            let auto_closed = event.closed.map(|c| c.reason) == Some(CloseReason::Auto0400);
            if observed != ObservedTime::Now || open.is_some() || !auto_closed {
                return Err(LogError::Stale);
            }
            if !can(
                actor,
                Action::ManageEvent {
                    owner: event.owner_id,
                },
                now,
            ) {
                return Err(LogError::Forbidden);
            }
            tx.reopen_event(id).await?;
            Some(id)
        }
    };
    let report = tx
        .create_report(NewReport {
            event_id,
            user_id: actor.id,
            observed_at,
            values: entry.values,
        })
        .await?;
    let reading = match entry.pm25 {
        Some(value) => Some(
            tx.create_reading(NewReading {
                user_id: actor.id,
                event_id,
                observed_at,
                value,
            })
            .await?,
        ),
        None => None,
    };
    Ok(Logged { report, reading })
}

/// Joins the event `id` names: running now, or one the past report can belong to (its end moves
/// if the report is later).
async fn join<T: EventWrites>(
    tx: &mut T,
    id: EventId,
    observed: ObservedTime,
    observed_at: DateTime<Utc>,
    schedule: &DailySchedule,
) -> Result<EventId, LogError> {
    let event = tx.event(id).await?.ok_or(LogError::Stale)?;
    match observed {
        ObservedTime::Now if !event.is_open() => return Err(LogError::Stale),
        ObservedTime::Now => {}
        ObservedTime::Backdated => {
            if !can_join_backdated(&event, observed_at, schedule) {
                return Err(LogError::OutsideEvent);
            }
            extend_to(tx, &event, observed_at).await?;
        }
    }
    Ok(id)
}

/// Logs a PM2.5 reading on its own ("Log PM2.5 only", spec §7.6). It joins the event whose span
/// contains its time, if any, and never opens, extends or closes an event.
pub async fn log_reading<T>(
    tx: &mut T,
    actor: &Actor,
    observed_at: DateTime<Utc>,
    value: Pm25,
    now: DateTime<Utc>,
) -> Result<Pm25Reading, LogError>
where
    T: EventWrites + ReadingWrites,
{
    if !can(actor, Action::LogEntry, now) {
        return Err(LogError::Forbidden);
    }
    classify_observed(observed_at, now)?;
    let event_id = tx.event_at(observed_at).await?.map(|event| event.id);
    let reading = tx
        .create_reading(NewReading {
            user_id: actor.id,
            event_id,
            observed_at,
            value,
        })
        .await?;
    Ok(reading)
}

/// A report after the end of a closed event moves the end to the report (spec §7.5).
pub(crate) async fn extend_to<T: EventWrites>(
    tx: &mut T,
    event: &Event,
    at: DateTime<Utc>,
) -> StoreResult<()> {
    if let Some(closed) = event.closed
        && at > closed.at
    {
        tx.move_event_end(event.id, at).await?;
    }
    Ok(())
}

/// Where "Move to another event" puts a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveTarget {
    Event(EventId),
    New,
}

/// Closes an open event by hand at `closed_at`, which must lie between its last report and now
/// (spec §7.3).
pub async fn close_event<T: EventWrites>(
    tx: &mut T,
    actor: &Actor,
    id: EventId,
    closed_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Event, LogError> {
    let event = manageable_event(tx, actor, id, now).await?;
    if !event.is_open() {
        return Err(LogError::Stale);
    }
    let last_report = tx.latest_report_at(id).await?.unwrap_or(event.opened_at);
    validate_manual_close(closed_at, last_report.max(event.opened_at), now)?;
    let closure = Closure {
        at: closed_at,
        reason: CloseReason::Manual,
    };
    Ok(tx.close_event(id, closure).await?)
}

/// Reopens a closed event, if no other event is open.
pub async fn reopen_event<T: EventWrites>(
    tx: &mut T,
    actor: &Actor,
    id: EventId,
    now: DateTime<Utc>,
) -> Result<Event, LogError> {
    let event = manageable_event(tx, actor, id, now).await?;
    if event.is_open() {
        return Err(LogError::Stale);
    }
    if tx.open_event().await?.is_some() {
        return Err(LogError::AnotherEventOpen);
    }
    Ok(tx.reopen_event(id).await?)
}

/// "Correct a mistake" (spec §7.7): new values and observed time for a report. The time must
/// still belong to the report's event; after a closed event's end, the end moves with it. A
/// check (no event) keeps every level at 0 (spec §7.1).
pub async fn correct_report<T: EventWrites + ReportWrites>(
    tx: &mut T,
    actor: &Actor,
    id: ReportId,
    observed_at: DateTime<Utc>,
    values: ReportValues,
    schedule: &DailySchedule,
    now: DateTime<Utc>,
) -> Result<SmellReport, LogError> {
    let report = editable_report(tx, actor, id, now).await?;
    classify_observed(observed_at, now)?;
    match report.event_id {
        Some(event_id) => {
            let event = tx.event(event_id).await?.ok_or(LogError::NotFound)?;
            if !can_join_backdated(&event, observed_at, schedule) {
                return Err(LogError::OutsideEvent);
            }
            extend_to(tx, &event, observed_at).await?;
        }
        None if !values.is_no_smell() => return Err(LogError::CheckCannotSmell),
        None => {}
    }
    Ok(tx.correct_report(id, observed_at, values).await?)
}

/// "Move to another event" (spec §7.7): into an event that was running when the report was
/// observed (its end moves if needed), or into a new event owned by the report's author. A
/// no-smell report never opens one (spec §7.1).
pub async fn move_report<T: EventWrites + ReportWrites>(
    tx: &mut T,
    actor: &Actor,
    id: ReportId,
    to: MoveTarget,
    schedule: &DailySchedule,
    now: DateTime<Utc>,
) -> Result<SmellReport, LogError> {
    let report = editable_report(tx, actor, id, now).await?;
    let target = match to {
        MoveTarget::Event(target) if Some(target) == report.event_id => return Ok(report),
        MoveTarget::Event(target) => {
            let event = tx.event(target).await?.ok_or(LogError::NotFound)?;
            if !can_join_backdated(&event, report.observed_at, schedule) {
                return Err(LogError::OutsideEvent);
            }
            extend_to(tx, &event, report.observed_at).await?;
            target
        }
        MoveTarget::New if report.values.is_no_smell() => return Err(LogError::NoSmellCannotOpen),
        MoveTarget::New => {
            let another_open = tx.open_event().await?.is_some();
            let (owner, at) = (report.user_id, report.observed_at);
            open_new_event(tx, owner, at, schedule, now, another_open).await?
        }
    };
    Ok(tx.move_report(id, target).await?)
}

/// The event, if it exists and `actor` may close or reopen it.
async fn manageable_event<T: EventWrites>(
    tx: &mut T,
    actor: &Actor,
    id: EventId,
    now: DateTime<Utc>,
) -> Result<Event, LogError> {
    let event = tx.event(id).await?.ok_or(LogError::NotFound)?;
    if can(
        actor,
        Action::ManageEvent {
            owner: event.owner_id,
        },
        now,
    ) {
        Ok(event)
    } else {
        Err(LogError::Forbidden)
    }
}

/// The report, if it exists, is not deleted, and `actor` may correct or move it.
async fn editable_report<T: ReportWrites>(
    tx: &mut T,
    actor: &Actor,
    id: ReportId,
    now: DateTime<Utc>,
) -> Result<SmellReport, LogError> {
    let report = tx
        .report(id)
        .await?
        .filter(|report| !report.is_deleted())
        .ok_or(LogError::NotFound)?;
    if can(
        actor,
        Action::EditEntry {
            owner: report.user_id,
        },
        now,
    ) {
        Ok(report)
    } else {
        Err(LogError::Forbidden)
    }
}
