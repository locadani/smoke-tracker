//! Event lifecycle rules (spec §7) that need no database.

use chrono::{DateTime, TimeDelta, Utc};

use crate::DomainError;
use crate::permissions::{Action, Actor, can};
use crate::records::Event;
use crate::schedule::DailySchedule;
use crate::types::{EventId, Role};

/// Reports observed longer ago than this count as backdated.
pub const BACKDATE_THRESHOLD: TimeDelta = TimeDelta::minutes(10);

/// Phone clocks drift; times this far ahead of the server still count as valid.
pub const FUTURE_TOLERANCE: TimeDelta = TimeDelta::minutes(1);

/// Consecutive reports in one event further apart than this get a warning.
pub const LARGE_GAP: TimeDelta = TimeDelta::hours(3);

/// Whether a report describes the present or the past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedTime {
    Now,
    Backdated,
}

pub fn classify_observed(
    observed_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<ObservedTime, DomainError> {
    if observed_at > now + FUTURE_TOLERANCE {
        Err(DomainError::ObservedInFuture)
    } else if observed_at < now - BACKDATE_THRESHOLD {
        Ok(ObservedTime::Backdated)
    } else {
        Ok(ObservedTime::Now)
    }
}

/// The minimum an event prompt needs to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSummary {
    pub id: EventId,
    pub opened_at: DateTime<Utc>,
}

/// The question to ask before saving a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogPrompt {
    /// Nothing to ask: the report starts a new event.
    Direct,
    /// "Same smell still going?" about the open event.
    SameSmell(EventSummary),
    /// "Which event was this?" for a backdated report.
    WhichEvent,
    /// "This event was auto-closed — was it still going?"
    AutoClosed(EventSummary),
}

/// Picks the prompt. A backdated report always asks which event; otherwise an open event takes
/// priority over a pending auto-close notice, which the UI then shows only as a banner.
pub fn log_prompt(
    observed: ObservedTime,
    open_event: Option<EventSummary>,
    unseen_auto_closed: Option<EventSummary>,
) -> LogPrompt {
    match (observed, open_event, unseen_auto_closed) {
        (ObservedTime::Backdated, _, _) => LogPrompt::WhichEvent,
        (ObservedTime::Now, Some(open), _) => LogPrompt::SameSmell(open),
        (ObservedTime::Now, None, Some(closed)) => LogPrompt::AutoClosed(closed),
        (ObservedTime::Now, None, None) => LogPrompt::Direct,
    }
}

/// Whether a newly created event starts open or already closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewEventState {
    Open,
    ClosedBackdated,
}

/// At most one event is open at a time, and nothing stays open across a 04:00 boundary.
///
/// When the user answers "No, it had stopped: new event" to the same-smell prompt, close the
/// open event first and then pass `another_event_open = false`; otherwise the new event is
/// wrongly created already closed.
pub fn new_event_state(
    observed_at: DateTime<Utc>,
    now: DateTime<Utc>,
    schedule: &DailySchedule,
    another_event_open: bool,
) -> NewEventState {
    if another_event_open || schedule.boundary_passed_since(observed_at, now) {
        NewEventState::ClosedBackdated
    } else {
        NewEventState::Open
    }
}

pub fn validate_manual_close(
    closed_at: DateTime<Utc>,
    last_report_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), DomainError> {
    if closed_at < last_report_at {
        Err(DomainError::CloseBeforeLastReport)
    } else if closed_at > now + FUTURE_TOLERANCE {
        Err(DomainError::CloseInFuture)
    } else {
        Ok(())
    }
}

/// Whether a report observed at `observed_at` (in the past) can belong to `event`: the event had
/// started by then and was still running on that local day. "Which event was this?" offers
/// exactly these events, and joining or moving into any other is refused.
pub fn can_join_backdated(
    event: &Event,
    observed_at: DateTime<Utc>,
    schedule: &DailySchedule,
) -> bool {
    let (day_start, _) = schedule.day_span(schedule.local_date(observed_at));
    event.opened_at <= observed_at && event.closed.is_none_or(|closed| closed.at >= day_start)
}

/// Who is told that `owner`'s event was auto-closed (spec §7.4): the owner, and residents when
/// the owner is a guest; only while they can log, since the notice is part of logging.
pub fn auto_close_notice_is_for(viewer: &Actor, owner: &Actor, now: DateTime<Utc>) -> bool {
    can(viewer, Action::LogEntry, now)
        && (viewer.id == owner.id || (viewer.role == Role::Resident && owner.role == Role::Guest))
}

/// Indices of reports that follow a gap longer than [`LARGE_GAP`]. `observed_times` must be
/// sorted ascending.
pub fn large_gap_indices(observed_times: &[DateTime<Utc>]) -> Vec<usize> {
    observed_times
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[1] - pair[0] > LARGE_GAP)
        .map(|(i, _)| i + 1)
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono_tz::Europe::Rome;

    use super::*;
    use crate::records::Closure;
    use crate::types::{CloseReason, UserId};

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn now() -> DateTime<Utc> {
        t("2026-01-15T20:00:00Z")
    }

    fn summary(id: i64) -> EventSummary {
        EventSummary {
            id: EventId(id),
            opened_at: t("2026-01-15T18:05:00Z"),
        }
    }

    fn schedule() -> DailySchedule {
        DailySchedule::new(Rome, 4).unwrap()
    }

    // --- classify_observed ---

    #[test]
    fn recent_times_count_as_now() {
        assert_eq!(classify_observed(now(), now()), Ok(ObservedTime::Now));
        assert_eq!(
            classify_observed(now() - TimeDelta::minutes(9), now()),
            Ok(ObservedTime::Now)
        );
        assert_eq!(
            classify_observed(now() - TimeDelta::minutes(10), now()),
            Ok(ObservedTime::Now)
        );
    }

    #[test]
    fn older_times_count_as_backdated() {
        assert_eq!(
            classify_observed(now() - TimeDelta::minutes(11), now()),
            Ok(ObservedTime::Backdated)
        );
        assert_eq!(
            classify_observed(now() - TimeDelta::days(400), now()),
            Ok(ObservedTime::Backdated)
        );
    }

    #[test]
    fn slightly_fast_phone_clocks_are_tolerated() {
        assert_eq!(
            classify_observed(now() + TimeDelta::seconds(30), now()),
            Ok(ObservedTime::Now)
        );
    }

    #[test]
    fn future_times_are_rejected() {
        assert_eq!(
            classify_observed(now() + TimeDelta::minutes(2), now()),
            Err(DomainError::ObservedInFuture)
        );
    }

    #[test]
    fn the_future_tolerance_is_inclusive_to_the_second() {
        assert_eq!(
            classify_observed(now() + FUTURE_TOLERANCE, now()),
            Ok(ObservedTime::Now)
        );
        assert_eq!(
            classify_observed(now() + FUTURE_TOLERANCE + TimeDelta::seconds(1), now()),
            Err(DomainError::ObservedInFuture)
        );
    }

    // --- log_prompt ---

    #[test]
    fn backdated_logging_always_asks_which_event() {
        assert_eq!(
            log_prompt(ObservedTime::Backdated, Some(summary(1)), Some(summary(2))),
            LogPrompt::WhichEvent
        );
        assert_eq!(
            log_prompt(ObservedTime::Backdated, None, None),
            LogPrompt::WhichEvent
        );
    }

    #[test]
    fn an_open_event_asks_same_smell_and_wins_over_a_notice() {
        assert_eq!(
            log_prompt(ObservedTime::Now, Some(summary(1)), None),
            LogPrompt::SameSmell(summary(1))
        );
        assert_eq!(
            log_prompt(ObservedTime::Now, Some(summary(1)), Some(summary(2))),
            LogPrompt::SameSmell(summary(1))
        );
    }

    #[test]
    fn an_unseen_auto_close_is_offered_when_nothing_is_open() {
        assert_eq!(
            log_prompt(ObservedTime::Now, None, Some(summary(2))),
            LogPrompt::AutoClosed(summary(2))
        );
    }

    #[test]
    fn nothing_open_and_nothing_pending_logs_directly() {
        assert_eq!(log_prompt(ObservedTime::Now, None, None), LogPrompt::Direct);
    }

    // --- new_event_state ---

    #[test]
    fn a_new_event_logged_now_is_open() {
        assert_eq!(
            new_event_state(now(), now(), &schedule(), false),
            NewEventState::Open
        );
    }

    #[test]
    fn a_new_event_is_closed_if_another_is_already_open() {
        assert_eq!(
            new_event_state(t("2026-01-15T10:00:00Z"), now(), &schedule(), true),
            NewEventState::ClosedBackdated
        );
    }

    #[test]
    fn a_backdated_event_stays_open_if_no_four_am_has_passed() {
        // Observed 02:00 local, now 03:30 local, same night.
        assert_eq!(
            new_event_state(
                t("2026-01-15T01:00:00Z"),
                t("2026-01-15T02:30:00Z"),
                &schedule(),
                false
            ),
            NewEventState::Open
        );
    }

    #[test]
    fn a_backdated_event_across_four_am_is_created_closed() {
        assert_eq!(
            new_event_state(
                t("2026-01-14T20:00:00Z"),
                t("2026-01-15T08:00:00Z"),
                &schedule(),
                false
            ),
            NewEventState::ClosedBackdated
        );
    }

    // --- validate_manual_close ---

    #[test]
    fn closing_at_or_after_the_last_report_and_not_in_the_future_is_valid() {
        let last = t("2026-01-15T19:00:00Z");
        assert_eq!(validate_manual_close(last, last, now()), Ok(()));
        assert_eq!(validate_manual_close(now(), last, now()), Ok(()));
    }

    #[test]
    fn closing_before_the_last_report_is_rejected() {
        let last = t("2026-01-15T19:00:00Z");
        assert_eq!(
            validate_manual_close(last - TimeDelta::minutes(1), last, now()),
            Err(DomainError::CloseBeforeLastReport)
        );
    }

    #[test]
    fn closing_in_the_future_is_rejected() {
        let last = t("2026-01-15T19:00:00Z");
        assert_eq!(
            validate_manual_close(now() + TimeDelta::minutes(5), last, now()),
            Err(DomainError::CloseInFuture)
        );
    }

    #[test]
    fn closing_exactly_at_the_future_tolerance_is_allowed_but_not_a_second_later() {
        let last = t("2026-01-15T19:00:00Z");
        assert_eq!(
            validate_manual_close(now() + FUTURE_TOLERANCE, last, now()),
            Ok(())
        );
        assert_eq!(
            validate_manual_close(
                now() + FUTURE_TOLERANCE + TimeDelta::seconds(1),
                last,
                now()
            ),
            Err(DomainError::CloseInFuture)
        );
    }

    // --- can_join_backdated ---

    fn event(opened: &str, closed: Option<&str>) -> Event {
        Event {
            id: EventId(1),
            owner_id: UserId(1),
            opened_at: t(opened),
            closed: closed.map(|at| Closure {
                at: t(at),
                reason: CloseReason::Manual,
            }),
            auto_close_notice_seen_at: None,
            recorded_at: t(opened),
        }
    }

    #[test]
    fn a_backdated_report_joins_events_that_had_started_and_ran_that_day() {
        let s = schedule();
        let evening = event("2026-01-14T18:00:00Z", Some("2026-01-14T19:00:00Z"));
        assert!(can_join_backdated(&evening, t("2026-01-14T18:30:00Z"), &s));
        assert!(
            can_join_backdated(&evening, t("2026-01-14T22:00:00Z"), &s),
            "later that day: the end will move"
        );
        assert!(!can_join_backdated(&evening, t("2026-01-14T17:59:00Z"), &s));
        assert!(
            !can_join_backdated(&evening, t("2026-01-15T10:00:00Z"), &s),
            "the next day"
        );
        let open = event("2026-01-14T18:00:00Z", None);
        assert!(can_join_backdated(&open, t("2026-01-15T10:00:00Z"), &s));
    }

    #[test]
    fn an_event_running_past_midnight_belongs_to_both_days() {
        let s = schedule();
        let late = event("2026-01-14T22:30:00Z", Some("2026-01-15T00:30:00Z"));
        assert!(can_join_backdated(&late, t("2026-01-15T00:10:00Z"), &s));
        assert!(can_join_backdated(&late, t("2026-01-15T09:00:00Z"), &s));
    }

    // --- auto_close_notice_is_for ---

    fn person(id: i64, role: Role, expires_at: Option<&str>) -> Actor {
        Actor {
            id: UserId(id),
            role,
            expires_at: expires_at.map(t),
        }
    }

    #[test]
    fn the_notice_goes_to_the_owner_and_to_residents_for_guests() {
        let resident = person(1, Role::Resident, None);
        let partner = person(2, Role::Resident, None);
        let guest = person(3, Role::Guest, Some("2026-02-01T00:00:00Z"));
        let viewer = person(4, Role::Viewer, None);
        assert!(auto_close_notice_is_for(&resident, &resident, now()));
        assert!(!auto_close_notice_is_for(&partner, &resident, now()));
        assert!(!auto_close_notice_is_for(&guest, &resident, now()));
        assert!(auto_close_notice_is_for(&guest, &guest, now()));
        assert!(auto_close_notice_is_for(&partner, &guest, now()));
        assert!(!auto_close_notice_is_for(&viewer, &guest, now()));
        let expired = person(3, Role::Guest, Some("2026-01-01T00:00:00Z"));
        assert!(!auto_close_notice_is_for(&expired, &expired, now()));
    }

    // --- large_gap_indices ---

    #[test]
    fn no_reports_or_one_report_has_no_gaps() {
        assert!(large_gap_indices(&[]).is_empty());
        assert!(large_gap_indices(&[now()]).is_empty());
    }

    #[test]
    fn gaps_over_three_hours_are_flagged_at_the_later_report() {
        let t0 = t("2026-01-15T09:00:00Z");
        let times = [
            t0,
            t0 + TimeDelta::hours(1),
            t0 + TimeDelta::hours(5),
            t0 + TimeDelta::minutes(330),
        ];
        assert_eq!(large_gap_indices(&times), vec![2]);
    }

    #[test]
    fn a_gap_of_exactly_three_hours_is_not_flagged() {
        let t0 = t("2026-01-15T09:00:00Z");
        assert!(large_gap_indices(&[t0, t0 + TimeDelta::hours(3)]).is_empty());
    }
}
