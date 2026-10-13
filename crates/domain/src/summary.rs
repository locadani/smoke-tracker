//! The summary card on the Log screen (spec §9, screen 2): "This month: 14 events · 41 reports
//! · 9 days with bedroom smoke."

use std::collections::BTreeSet;

use crate::records::SmellReport;
use crate::schedule::DailySchedule;
use crate::types::RoomId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MonthSummary {
    /// Events with at least one of the reports.
    pub events: usize,
    pub reports: usize,
    /// Local days with a report naming the bedroom; `None` when there is no bedroom room.
    pub bedroom_days: Option<usize>,
}

/// Summarises `reports`, which the caller has already limited to one month and to what the
/// viewer may see. `bedroom` is the starter bedroom room, if it exists.
pub fn summarise(
    reports: &[SmellReport],
    bedroom: Option<RoomId>,
    schedule: &DailySchedule,
) -> MonthSummary {
    let events: BTreeSet<_> = reports.iter().map(|r| r.event_id).collect();
    let bedroom_days = bedroom.map(|room| {
        reports
            .iter()
            .filter(|r| r.values.rooms().contains(&room))
            .map(|r| schedule.local_date(r.observed_at))
            .collect::<BTreeSet<_>>()
            .len()
    });
    MonthSummary {
        events: events.len(),
        reports: reports.len(),
        bedroom_days,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::records::ReportValues;
    use crate::types::{EventId, Intensity, ReportId, UserId};

    fn report(id: i64, event: i64, at: &str, rooms: &[i64]) -> SmellReport {
        let at: DateTime<Utc> = at.parse().unwrap();
        SmellReport {
            id: ReportId(id),
            event_id: EventId(event),
            user_id: UserId(1),
            observed_at: at,
            recorded_at: at,
            values: ReportValues::new(
                Intensity::new(5).unwrap(),
                rooms.iter().copied().map(RoomId).collect(),
                vec![],
                None,
            )
            .unwrap(),
            deleted_at: None,
        }
    }

    fn rome() -> DailySchedule {
        DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap()
    }

    #[test]
    fn counts_events_reports_and_local_days_with_bedroom_smoke() {
        let bedroom = RoomId(3);
        let reports = [
            report(1, 1, "2026-01-10T20:00:00Z", &[3]),
            report(2, 1, "2026-01-10T22:30:00Z", &[1, 3]),
            // 00:30 on the 11th in Rome: a second bedroom day.
            report(3, 1, "2026-01-10T23:30:00Z", &[3]),
            report(4, 2, "2026-01-12T08:00:00Z", &[1]),
        ];
        assert_eq!(
            summarise(&reports, Some(bedroom), &rome()),
            MonthSummary {
                events: 2,
                reports: 4,
                bedroom_days: Some(2),
            }
        );
    }

    #[test]
    fn without_a_bedroom_or_reports_the_counts_are_empty() {
        assert_eq!(
            summarise(&[report(1, 1, "2026-01-10T20:00:00Z", &[3])], None, &rome()).bedroom_days,
            None
        );
        assert_eq!(
            summarise(&[], Some(RoomId(3)), &rome()),
            MonthSummary {
                events: 0,
                reports: 0,
                bedroom_days: Some(0),
            }
        );
    }
}
