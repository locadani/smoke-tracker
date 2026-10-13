//! The logging use cases (spec §7). Each one runs inside a write transaction the caller begins
//! and commits, so every write it makes — closing the open event, opening a new one, the report
//! and its PM2.5 reading — is audited and applied together or not at all. Permissions are
//! checked here, against the data read inside the transaction.

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::DomainError;
use crate::events::{
    NewEventState, ObservedTime, can_join_backdated, classify_observed, new_event_state,
};
use crate::permissions::{Action, Actor, can};
use crate::records::{
    Closure, Event, NewEvent, NewReading, NewReport, Pm25Reading, ReportValues, SmellReport,
};
use crate::repos::{EventWrites, ReadingWrites, ReportWrites, StoreError, StoreResult};
use crate::schedule::DailySchedule;
use crate::types::{CloseReason, EventId, Pm25};

#[derive(Debug, Error)]
pub enum LogError {
    #[error("not allowed")]
    Forbidden,
    /// The answer to the event question no longer fits the data (another phone closed or
    /// opened an event meanwhile): ask again rather than guess.
    #[error("the events changed since the question was asked")]
    Stale,
    /// The time lies outside the event: before it started, or on another day.
    #[error("the time is outside the event")]
    OutsideEvent,
    #[error(transparent)]
    Invalid(#[from] DomainError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// The answer to "Same smell still going?", "Which event was this?" or the auto-close notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventChoice {
    /// Start a new event: nothing was asked, "No, it had stopped", or "New event".
    New,
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

/// Logs a smell report, and its PM2.5 reading if there is one, into the event `choice` names
/// (spec §7.1, §7.2, §7.4, §7.5).
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
    let event_id = match choice {
        EventChoice::New => {
            let another_open = match (observed, &open) {
                (ObservedTime::Now, Some(open)) => {
                    close_at_last_report(tx, open, CloseReason::NewEvent).await?;
                    false
                }
                (_, open) => open.is_some(),
            };
            let closed = match new_event_state(observed_at, now, schedule, another_open) {
                NewEventState::Open => None,
                NewEventState::ClosedBackdated => Some(Closure {
                    at: observed_at,
                    reason: CloseReason::Backdated,
                }),
            };
            let new = NewEvent {
                owner_id: actor.id,
                opened_at: observed_at,
                closed,
            };
            tx.create_event(new).await?.id
        }
        EventChoice::Join(id) => {
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
            id
        }
        EventChoice::Continue(id) => {
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
            id
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
                event_id: Some(event_id),
                observed_at,
                value,
            })
            .await?,
        ),
        None => None,
    };
    Ok(Logged { report, reading })
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
