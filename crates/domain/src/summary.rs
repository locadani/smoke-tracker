//! The summary card on the Log screen (spec §9, screen 2): "This month: 14 events · 41 reports
//! · 9 days with bedroom smoke · 6 clean checks."

use std::collections::BTreeSet;

use crate::records::ReportView;
use crate::schedule::DailySchedule;
use crate::types::RoomId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MonthSummary {
    /// Events with at least one of the reports.
    pub events: usize,
    /// Reports in an event.
    pub reports: usize,
    /// No-smell checks made while no event was running.
    pub checks: usize,
    /// Local days with a report giving the bedroom a level of 1 or more; `None` when there is no
    /// bedroom room.
    pub bedroom_days: Option<usize>,
}

/// Summarises `reports`, which the caller has already limited to one month and to what the
/// viewer may see. `bedroom` is the starter bedroom room, if it exists.
pub fn summarise(
    reports: &[ReportView],
    bedroom: Option<RoomId>,
    schedule: &DailySchedule,
) -> MonthSummary {
    let events: BTreeSet<_> = reports.iter().filter_map(|r| r.event_id).collect();
    let checks = reports.iter().filter(|r| r.event_id.is_none()).count();
    let bedroom_days = bedroom.map(|room| {
        reports
            .iter()
            .filter(|r| {
                r.values
                    .level_in(room)
                    .is_some_and(|level| !level.is_clean())
            })
            .map(|r| schedule.local_date(r.observed_at))
            .collect::<BTreeSet<_>>()
            .len()
    });
    MonthSummary {
        events: events.len(),
        reports: reports.len() - checks,
        checks,
        bedroom_days,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::records::{ReportValues, RoomLevel};
    use crate::types::{EventId, Level, LogEntryId, ReportId, UserId};

    /// Event 0 stands for no event: a no-smell check.
    fn report(id: i64, event: i64, at: &str, rooms: &[(i64, u8)]) -> ReportView {
        let at: DateTime<Utc> = at.parse().unwrap();
        ReportView {
            id: ReportId(id),
            entry_id: LogEntryId(id),
            event_id: (event != 0).then_some(EventId(event)),
            user_id: UserId(1),
            observed_at: at,
            recorded_at: at,
            values: ReportValues::new(
                rooms
                    .iter()
                    .map(|&(room, level)| RoomLevel {
                        room: RoomId(room),
                        level: Level::new(level).unwrap(),
                    })
                    .collect(),
                vec![],
            )
            .unwrap(),
            note: None,
            deleted_at: None,
        }
    }

    fn rome() -> DailySchedule {
        DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap()
    }

    #[test]
    fn counts_events_reports_checks_and_local_days_with_bedroom_smoke() {
        let bedroom = RoomId(3);
        let reports = [
            report(1, 1, "2026-01-10T20:00:00Z", &[(3, 4)]),
            report(2, 1, "2026-01-10T22:30:00Z", &[(1, 2), (3, 5)]),
            // 00:30 on the 11th in Rome: a second bedroom day.
            report(3, 1, "2026-01-10T23:30:00Z", &[(3, 1)]),
            report(4, 2, "2026-01-12T08:00:00Z", &[(1, 6)]),
            // The bedroom checked and clean: not a day with bedroom smoke.
            report(5, 2, "2026-01-12T09:00:00Z", &[(1, 6), (3, 0)]),
            report(6, 0, "2026-01-13T09:00:00Z", &[(3, 0)]),
            report(7, 0, "2026-01-14T09:00:00Z", &[]),
        ];
        assert_eq!(
            summarise(&reports, Some(bedroom), &rome()),
            MonthSummary {
                events: 2,
                reports: 5,
                checks: 2,
                bedroom_days: Some(2),
            }
        );
    }

    #[test]
    fn without_a_bedroom_or_reports_the_counts_are_empty() {
        assert_eq!(
            summarise(
                &[report(1, 1, "2026-01-10T20:00:00Z", &[(3, 4)])],
                None,
                &rome()
            )
            .bedroom_days,
            None
        );
        assert_eq!(
            summarise(&[], Some(RoomId(3)), &rome()),
            MonthSummary {
                events: 0,
                reports: 0,
                checks: 0,
                bedroom_days: Some(0),
            }
        );
    }
}
