//! The 04:00 auto-close sweep (spec §7.3, §11): a catch-up job, so the result is the same
//! whether it runs at 04:00 sharp or hours later after the host slept.

use chrono::{DateTime, Utc};

use crate::records::Closure;
use crate::repos::{Commit, EventWrites, JobRepo, JobWrites, StoreResult, Transactional};
use crate::schedule::DailySchedule;
use crate::types::{CloseReason, EventId};

/// The `jobs` row that remembers the last sweep.
pub const AUTO_CLOSE_JOB: &str = "auto_close_sweep";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepOutcome {
    /// No boundary has passed since the last sweep.
    NotDue,
    /// The sweep ran; `closed` is the event it closed, if one was open across a boundary.
    Ran { closed: Option<EventId> },
}

/// Runs the sweep if a boundary has passed since the last run (or it never ran). An open event
/// that started before the latest boundary is closed at its last report (`auto_0400`); one
/// that started after it is left open. The close and the new `last_run_at` commit together.
pub async fn auto_close<S>(
    store: &S,
    schedule: &DailySchedule,
    now: DateTime<Utc>,
) -> StoreResult<SweepOutcome>
where
    S: Transactional<Tx: EventWrites + JobWrites> + JobRepo,
{
    if let Some(last) = store.job_last_run(AUTO_CLOSE_JOB).await?
        && !schedule.boundary_passed_since(last, now)
    {
        return Ok(SweepOutcome::NotDue);
    }
    let mut tx = store.begin(None).await?;
    let mut closed = None;
    if let Some(event) = tx.open_event().await?
        && schedule.boundary_passed_since(event.opened_at, now)
    {
        let last_report = tx.latest_report_at(event.id).await?;
        let at = last_report.map_or(event.opened_at, |at| at.max(event.opened_at));
        let reason = CloseReason::Auto0400;
        tx.close_event(event.id, Closure { at, reason }).await?;
        closed = Some(event.id);
    }
    tx.record_job_run(AUTO_CLOSE_JOB, now).await?;
    tx.commit().await?;
    Ok(SweepOutcome::Ran { closed })
}
