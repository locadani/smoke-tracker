//! The numbers behind the Graphs screen (spec §9 screen 5). Every function takes the reports the
//! caller has already limited to one [`Period`] and to what the viewer may see, so guests get
//! graphs of their own data only (spec §4).

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Days, Months, NaiveDate, Timelike, Utc};

use crate::records::{Room, SmellReport};
use crate::schedule::DailySchedule;
use crate::types::{EventId, Level, RoomArea, RoomId, SymptomId, UserId};

/// The stretch of local calendar time the graphs cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// The local calendar month that contains this date, counted per day.
    Month(NaiveDate),
    /// The local calendar year that contains this date, counted per week (Monday to Sunday).
    Year(NaiveDate),
}

impl Period {
    /// `[start, end)` in UTC.
    pub fn span(&self, schedule: &DailySchedule) -> (DateTime<Utc>, DateTime<Utc>) {
        match *self {
            Self::Month(date) => schedule.month_span(date),
            Self::Year(date) => schedule.year_span(date),
        }
    }

    /// The first local day of the period.
    pub fn first_day(&self) -> NaiveDate {
        match *self {
            Self::Month(date) => date.with_day(1).expect("every month has a first day"),
            Self::Year(date) => date.with_ordinal(1).expect("every year has a first day"),
        }
    }

    /// The period before this one.
    pub fn previous(&self) -> Self {
        let first = self.first_day();
        match self {
            Self::Month(_) => Self::Month(first - Months::new(1)),
            Self::Year(_) => Self::Year(first - Months::new(12)),
        }
    }

    /// The period after this one.
    pub fn next(&self) -> Self {
        self.after(self.first_day())
    }

    fn after(&self, first: NaiveDate) -> Self {
        match self {
            Self::Month(_) => Self::Month(first + Months::new(1)),
            Self::Year(_) => Self::Year(first + Months::new(12)),
        }
    }

    /// The first day of every bucket, in order: each day of a month; each week touching a year,
    /// named by its Monday (the first may fall in December of the year before).
    pub fn buckets(&self) -> Vec<NaiveDate> {
        let first = self.first_day();
        let end = self.after(first).first_day();
        let step = match self {
            Self::Month(_) => Days::new(1),
            Self::Year(_) => Days::new(7),
        };
        std::iter::successors(Some(self.bucket_of(first)), |&day| Some(day + step))
            .take_while(|&day| day < end)
            .collect()
    }

    /// The first day of the bucket that holds `date`.
    pub fn bucket_of(&self, date: NaiveDate) -> NaiveDate {
        match self {
            Self::Month(_) => date,
            Self::Year(_) => date - Days::new(u64::from(date.weekday().num_days_from_monday())),
        }
    }
}

/// One person's reports as (observed time, intensity), in time order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonLine {
    pub user: UserId,
    pub points: Vec<(DateTime<Utc>, Level)>,
}

/// Intensity over time with one line per person (spec §9: "a report's highest room level, one
/// line per person"), ordered by user id. A no-smell report is a point at 0.
pub fn intensity_lines(reports: &[SmellReport]) -> Vec<PersonLine> {
    let mut lines: BTreeMap<UserId, Vec<(DateTime<Utc>, Level)>> = BTreeMap::new();
    for report in reports {
        lines
            .entry(report.user_id)
            .or_default()
            .push((report.observed_at, report.values.intensity()));
    }
    lines
        .into_iter()
        .map(|(user, mut points)| {
            points.sort_by_key(|&(at, _)| at);
            PersonLine { user, points }
        })
        .collect()
}

/// The number of events in each bucket of `period`. An event counts once, in the bucket of
/// its earliest report among `reports`, the rule of the summary card (decision #94); a no-smell
/// check outside any event is not an event.
pub fn events_per_bucket(
    reports: &[SmellReport],
    period: Period,
    schedule: &DailySchedule,
) -> Vec<(NaiveDate, usize)> {
    let mut first_seen: BTreeMap<EventId, DateTime<Utc>> = BTreeMap::new();
    for report in reports {
        if let Some(event) = report.event_id {
            first_seen
                .entry(event)
                .and_modify(|at| *at = (*at).min(report.observed_at))
                .or_insert(report.observed_at);
        }
    }
    let mut counts = zeroed(period, 0);
    for at in first_seen.into_values() {
        if let Some(count) = counts.get_mut(&period.bucket_of(schedule.local_date(at))) {
            *count += 1;
        }
    }
    counts.into_iter().collect()
}

/// The levels one room was given at one hour of the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LevelTally {
    /// Reports that checked the room at that hour, a clean check (0) included.
    pub checks: u32,
    /// The sum of their levels.
    pub total: u32,
}

impl LevelTally {
    /// The average level, or `None` when nobody checked the room at that hour.
    pub fn mean(&self) -> Option<f64> {
        (self.checks > 0).then(|| f64::from(self.total) / f64::from(self.checks))
    }
}

/// One row of the room × hour-of-day heatmap: the room's tallies at local hours 0–23.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomHours {
    pub room: RoomId,
    pub hours: [LevelTally; 24],
}

/// The room × hour-of-day heatmap of room levels, where 0 counts as checked and clean (spec
/// §9). Rooms that no report names are left out; the rest are ordered by id. A no-smell
/// report that names no room adds nothing: it says nothing about any one room.
pub fn room_hours(reports: &[SmellReport], schedule: &DailySchedule) -> Vec<RoomHours> {
    let mut rows: BTreeMap<RoomId, [LevelTally; 24]> = BTreeMap::new();
    for report in reports {
        let hour = report.observed_at.with_timezone(&schedule.tz()).hour() as usize;
        for room in report.values.rooms() {
            let tally = &mut rows.entry(room.room).or_default()[hour];
            tally.checks += 1;
            tally.total += u32::from(room.level.get());
        }
    }
    rows.into_iter()
        .map(|(room, hours)| RoomHours { room, hours })
        .collect()
}

/// How many reports name each symptom, most frequent first, then by id. Symptoms nobody
/// reported are left out.
pub fn symptom_counts(reports: &[SmellReport]) -> Vec<(SymptomId, usize)> {
    let mut counts: BTreeMap<SymptomId, usize> = BTreeMap::new();
    for report in reports {
        for &symptom in report.values.symptoms() {
            *counts.entry(symptom).or_default() += 1;
        }
    }
    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    counts
}

/// The highest level reported inside the home and in common areas within one bucket; `None`
/// where no room of that area was checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AreaLevels {
    pub inside: Option<Level>,
    pub common: Option<Level>,
}

/// Inside vs common areas, bucket by bucket: each area's highest room level, the same rule as
/// a report's intensity (spec §6). `rooms` gives each room's area; a room missing from it is
/// skipped.
pub fn area_levels(
    reports: &[SmellReport],
    rooms: &[Room],
    period: Period,
    schedule: &DailySchedule,
) -> Vec<(NaiveDate, AreaLevels)> {
    let areas: BTreeMap<RoomId, RoomArea> = rooms.iter().map(|r| (r.id, r.area)).collect();
    let mut buckets = zeroed(period, AreaLevels::default());
    for report in reports {
        let bucket = period.bucket_of(schedule.local_date(report.observed_at));
        let Some(levels) = buckets.get_mut(&bucket) else {
            continue;
        };
        for room in report.values.rooms() {
            let highest = match areas.get(&room.room) {
                Some(RoomArea::Inside) => &mut levels.inside,
                Some(RoomArea::Common) => &mut levels.common,
                None => continue,
            };
            *highest = (*highest).max(Some(room.level));
        }
    }
    buckets.into_iter().collect()
}

/// Every bucket of `period`, each holding `empty`.
fn zeroed<T: Copy>(period: Period, empty: T) -> BTreeMap<NaiveDate, T> {
    period.buckets().into_iter().map(|b| (b, empty)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::{ReportValues, RoomLevel};
    use crate::types::{ListName, ReportId};

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn date(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn rome() -> DailySchedule {
        DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap()
    }

    /// Event 0 stands for no event: a no-smell check.
    fn report(
        event: i64,
        user: i64,
        at: &str,
        rooms: &[(i64, u8)],
        symptoms: &[i64],
    ) -> SmellReport {
        let at = t(at);
        SmellReport {
            id: ReportId(1),
            event_id: (event != 0).then_some(EventId(event)),
            user_id: UserId(user),
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
                symptoms.iter().map(|&s| SymptomId(s)).collect(),
                None,
            )
            .unwrap(),
            deleted_at: None,
        }
    }

    fn room(id: i64, area: RoomArea) -> Room {
        Room {
            id: RoomId(id),
            name: ListName::new("Room").unwrap(),
            i18n_key: None,
            area,
            hidden_at: None,
        }
    }

    fn level(value: u8) -> Level {
        Level::new(value).unwrap()
    }

    #[test]
    fn a_month_is_counted_per_day() {
        let february = Period::Month(date("2026-02-14"));
        let days = february.buckets();
        assert_eq!(days.len(), 28);
        assert_eq!(days[0], date("2026-02-01"));
        assert_eq!(days[27], date("2026-02-28"));
        assert_eq!(february.bucket_of(date("2026-02-14")), date("2026-02-14"));
        assert_eq!(
            february.span(&rome()),
            (t("2026-01-31T23:00:00Z"), t("2026-02-28T23:00:00Z"))
        );
    }

    #[test]
    fn a_year_is_counted_per_week_from_monday() {
        // 1 January 2026 is a Thursday: the first week starts on Monday 29 December 2025.
        let year = Period::Year(date("2026-06-01"));
        let weeks = year.buckets();
        assert_eq!(weeks[0], date("2025-12-29"));
        assert_eq!(weeks[1], date("2026-01-05"));
        assert_eq!(*weeks.last().unwrap(), date("2026-12-28"));
        assert_eq!(weeks.len(), 53);
        assert_eq!(year.bucket_of(date("2026-01-01")), date("2025-12-29"));
        assert_eq!(year.bucket_of(date("2026-01-11")), date("2026-01-05"));
    }

    #[test]
    fn periods_step_back_and_forward_from_their_first_day() {
        let january = Period::Month(date("2026-01-31"));
        assert_eq!(january.previous(), Period::Month(date("2025-12-01")));
        assert_eq!(january.next(), Period::Month(date("2026-02-01")));
        let year = Period::Year(date("2026-07-15"));
        assert_eq!(year.previous(), Period::Year(date("2025-01-01")));
        assert_eq!(year.next().first_day(), date("2027-01-01"));
    }

    #[test]
    fn each_person_gets_a_line_in_time_order() {
        let reports = [
            report(1, 2, "2026-01-10T21:00:00Z", &[(1, 6)], &[]),
            report(1, 1, "2026-01-10T20:00:00Z", &[(1, 4), (3, 7)], &[]),
            report(1, 2, "2026-01-10T20:30:00Z", &[(1, 5)], &[]),
            report(0, 1, "2026-01-11T09:00:00Z", &[], &[]),
        ];
        assert_eq!(
            intensity_lines(&reports),
            [
                PersonLine {
                    user: UserId(1),
                    points: vec![
                        (t("2026-01-10T20:00:00Z"), level(7)),
                        (t("2026-01-11T09:00:00Z"), level(0)),
                    ],
                },
                PersonLine {
                    user: UserId(2),
                    points: vec![
                        (t("2026-01-10T20:30:00Z"), level(5)),
                        (t("2026-01-10T21:00:00Z"), level(6)),
                    ],
                },
            ]
        );
    }

    #[test]
    fn an_event_counts_once_on_the_local_day_of_its_first_report() {
        let january = Period::Month(date("2026-01-01"));
        let reports = [
            // 00:30 on 11 January in Rome.
            report(1, 1, "2026-01-10T23:30:00Z", &[(1, 4)], &[]),
            report(1, 2, "2026-01-11T08:00:00Z", &[(1, 6)], &[]),
            report(2, 1, "2026-01-11T20:00:00Z", &[(1, 3)], &[]),
            report(0, 1, "2026-01-12T09:00:00Z", &[], &[]),
        ];
        let counts: BTreeMap<_, _> = events_per_bucket(&reports, january, &rome())
            .into_iter()
            .collect();
        assert_eq!(counts.len(), 31);
        assert_eq!(counts[&date("2026-01-10")], 0);
        assert_eq!(counts[&date("2026-01-11")], 2);
        assert_eq!(counts[&date("2026-01-12")], 0);
        assert_eq!(counts.values().sum::<usize>(), 2);
    }

    #[test]
    fn a_year_counts_events_per_week() {
        let year = Period::Year(date("2026-01-01"));
        let reports = [
            report(1, 1, "2026-01-05T20:00:00Z", &[(1, 4)], &[]),
            report(2, 1, "2026-01-11T20:00:00Z", &[(1, 4)], &[]),
            report(3, 1, "2026-01-12T20:00:00Z", &[(1, 4)], &[]),
        ];
        let counts = events_per_bucket(&reports, year, &rome());
        assert_eq!(counts[1], (date("2026-01-05"), 2));
        assert_eq!(counts[2], (date("2026-01-12"), 1));
    }

    #[test]
    fn the_heatmap_averages_levels_per_room_and_local_hour_counting_clean_checks() {
        let reports = [
            // 21:xx in Rome (UTC+1).
            report(1, 1, "2026-01-10T20:05:00Z", &[(1, 6), (3, 0)], &[]),
            report(1, 2, "2026-01-11T20:50:00Z", &[(1, 3)], &[]),
            // In summer Rome is UTC+2: still 21:xx local.
            report(2, 1, "2026-07-01T19:10:00Z", &[(1, 0)], &[]),
            // No rooms named: nothing to place.
            report(0, 1, "2026-01-12T09:00:00Z", &[], &[]),
        ];
        let rows = room_hours(&reports, &rome());
        assert_eq!(
            rows.iter().map(|r| r.room).collect::<Vec<_>>(),
            [RoomId(1), RoomId(3)]
        );
        assert_eq!(
            rows[0].hours[21],
            LevelTally {
                checks: 3,
                total: 9
            }
        );
        assert_eq!(rows[0].hours[21].mean(), Some(3.0));
        assert_eq!(rows[1].hours[21].mean(), Some(0.0));
        assert_eq!(rows[0].hours[20].mean(), None);
        assert_eq!(rows[0].hours.iter().map(|h| h.checks).sum::<u32>(), 3);
    }

    #[test]
    fn symptoms_are_counted_per_report_most_frequent_first() {
        let reports = [
            report(1, 1, "2026-01-10T20:00:00Z", &[(1, 4)], &[2, 5]),
            report(1, 1, "2026-01-10T21:00:00Z", &[(1, 4)], &[5]),
            report(1, 2, "2026-01-10T22:00:00Z", &[(1, 4)], &[1]),
            report(1, 2, "2026-01-10T23:00:00Z", &[(1, 4)], &[]),
        ];
        assert_eq!(
            symptom_counts(&reports),
            [(SymptomId(5), 2), (SymptomId(1), 1), (SymptomId(2), 1)]
        );
    }

    #[test]
    fn each_area_keeps_its_highest_level_per_bucket() {
        let rooms = [room(1, RoomArea::Inside), room(6, RoomArea::Common)];
        let january = Period::Month(date("2026-01-01"));
        let reports = [
            report(1, 1, "2026-01-10T20:00:00Z", &[(1, 4), (6, 2)], &[]),
            report(1, 2, "2026-01-10T21:00:00Z", &[(1, 7)], &[]),
            report(0, 1, "2026-01-11T09:00:00Z", &[(6, 0)], &[]),
            // A room the list does not know is skipped.
            report(2, 1, "2026-01-12T09:00:00Z", &[(9, 8)], &[]),
        ];
        let levels: BTreeMap<_, _> = area_levels(&reports, &rooms, january, &rome())
            .into_iter()
            .collect();
        assert_eq!(
            levels[&date("2026-01-10")],
            AreaLevels {
                inside: Some(level(7)),
                common: Some(level(2)),
            }
        );
        assert_eq!(
            levels[&date("2026-01-11")],
            AreaLevels {
                inside: None,
                common: Some(level(0)),
            }
        );
        assert_eq!(levels[&date("2026-01-12")], AreaLevels::default());
    }
}
