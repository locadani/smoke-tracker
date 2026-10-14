//! The numbers behind the Graphs screen (spec §9 screen 5). [`Filter`] narrows what the viewer
//! may see to what the filters ask for; every other function takes the reports the caller has
//! already limited to one [`Period`] and to what the viewer may see, so guests get graphs of
//! their own data only (spec §4).

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Days, Months, NaiveDate, Timelike, Utc};

use crate::records::{Pm25Reading, ReportValues, Room, SmellReport};
use crate::schedule::DailySchedule;
use crate::types::{EventId, Level, RoomArea, RoomId, SymptomId, UserId};

/// The stretch of local calendar days the graphs cover, both ends included (look-and-feel spec
/// §7). Up to about two months it is counted per day, beyond that per week (Monday to Sunday).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    first: NaiveDate,
    last: NaiveDate,
}

/// How a [`Period`] is cut into buckets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    Day,
    Week,
}

impl Period {
    /// The longest period counted per day: about two months.
    pub const DAILY_UP_TO_DAYS: i64 = 62;
    /// The longest period the graphs show, so that every query stays bounded.
    pub const MAX_DAYS: i64 = 731;

    /// From `first` to `last`, both included; `None` when `last` comes first or the period is
    /// longer than [`Self::MAX_DAYS`].
    pub fn new(first: NaiveDate, last: NaiveDate) -> Option<Self> {
        let days = (last - first).num_days() + 1;
        (1..=Self::MAX_DAYS)
            .contains(&days)
            .then_some(Self { first, last })
    }

    /// The local calendar month that contains `date`.
    pub fn month(date: NaiveDate) -> Self {
        let first = date.with_day(1).expect("every month has a first day");
        let last = first + Months::new(1) - Days::new(1);
        Self { first, last }
    }

    /// The local calendar year that contains `date`.
    pub fn year(date: NaiveDate) -> Self {
        let first = date.with_ordinal(1).expect("every year has a first day");
        let last = first + Months::new(12) - Days::new(1);
        Self { first, last }
    }

    /// The `days` days that end with `today`, kept between one day and [`Self::MAX_DAYS`].
    pub fn last_days(today: NaiveDate, days: u64) -> Self {
        let days = days.clamp(1, Self::MAX_DAYS.unsigned_abs());
        Self {
            first: today - Days::new(days - 1),
            last: today,
        }
    }

    pub fn first_day(&self) -> NaiveDate {
        self.first
    }

    pub fn last_day(&self) -> NaiveDate {
        self.last
    }

    /// The number of days, both ends included.
    pub fn days(&self) -> i64 {
        (self.last - self.first).num_days() + 1
    }

    pub fn granularity(&self) -> Granularity {
        if self.days() <= Self::DAILY_UP_TO_DAYS {
            Granularity::Day
        } else {
            Granularity::Week
        }
    }

    /// `[start, end)` in UTC: from the local midnight that starts the first day to the one that
    /// ends the last.
    pub fn span(&self, schedule: &DailySchedule) -> (DateTime<Utc>, DateTime<Utc>) {
        (
            schedule.day_span(self.first).0,
            schedule.day_span(self.last).1,
        )
    }

    /// The first day of every bucket, in order: each day, or each week touching the period,
    /// named by its Monday (the first may fall before the period starts).
    pub fn buckets(&self) -> Vec<NaiveDate> {
        let step = match self.granularity() {
            Granularity::Day => Days::new(1),
            Granularity::Week => Days::new(7),
        };
        std::iter::successors(Some(self.bucket_of(self.first)), |&day| Some(day + step))
            .take_while(|&day| day <= self.last)
            .collect()
    }

    /// The first day of the bucket that holds `date`.
    pub fn bucket_of(&self, date: NaiveDate) -> NaiveDate {
        match self.granularity() {
            Granularity::Day => date,
            Granularity::Week => date - Days::new(u64::from(date.weekday().num_days_from_monday())),
        }
    }
}

/// A part of the day, by local observed time (look-and-feel spec §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayPart {
    /// 06:00–12:00.
    Morning,
    /// 12:00–18:00.
    Afternoon,
    /// 18:00–22:00.
    Evening,
    /// 22:00–06:00, across midnight.
    Night,
}

impl DayPart {
    pub const ALL: [DayPart; 4] = [
        DayPart::Morning,
        DayPart::Afternoon,
        DayPart::Evening,
        DayPart::Night,
    ];

    /// The local hours it starts and ends at; the end is excluded.
    pub fn hours(self) -> (u32, u32) {
        match self {
            Self::Morning => (6, 12),
            Self::Afternoon => (12, 18),
            Self::Evening => (18, 22),
            Self::Night => (22, 6),
        }
    }

    /// Whether the local `hour` (0–23) falls in this part.
    pub fn contains(self, hour: u32) -> bool {
        let (start, end) = self.hours();
        if start < end {
            (start..end).contains(&hour)
        } else {
            hour >= start || hour < end
        }
    }
}

/// What the graphs are narrowed to, besides the period (look-and-feel spec §7). The rooms, the
/// area and the minimum level concern smell reports only; PM2.5 readings follow the person and
/// the time of day. The default keeps everything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// Only these rooms; when any is chosen, `area` is ignored.
    pub rooms: Vec<RoomId>,
    /// Only the rooms of this area.
    pub area: Option<RoomArea>,
    /// Only what this person logged.
    pub person: Option<UserId>,
    /// Only what was observed in this part of the day.
    pub day_part: Option<DayPart>,
    /// Leaves out reports below this level; 0 keeps them all.
    pub min_level: u8,
    /// Leaves out reports at 0.
    pub hide_clean: bool,
}

impl Filter {
    /// The reports the graphs count. Each report keeps only its rooms among those chosen (by
    /// room, else by area) and is left out when none is left, a whole-home check included; its
    /// intensity is then the highest level among the rooms kept, which the minimum level and
    /// "hide clean checks" judge. `rooms` gives each room's area.
    pub fn reports(
        &self,
        reports: Vec<SmellReport>,
        rooms: &[Room],
        schedule: &DailySchedule,
    ) -> Vec<SmellReport> {
        let wanted = |room: RoomId| {
            if !self.rooms.is_empty() {
                self.rooms.contains(&room)
            } else {
                self.area
                    .is_none_or(|area| rooms.iter().any(|r| r.id == room && r.area == area))
            }
        };
        let narrowing = !self.rooms.is_empty() || self.area.is_some();
        reports
            .into_iter()
            .filter(|report| self.keeps(report.user_id, report.observed_at, schedule))
            .filter_map(|mut report| {
                if narrowing {
                    let kept: Vec<_> = report
                        .values
                        .rooms()
                        .iter()
                        .copied()
                        .filter(|room| wanted(room.room))
                        .collect();
                    if kept.is_empty() {
                        return None;
                    }
                    report.values = ReportValues::new(
                        kept,
                        report.values.symptoms().to_vec(),
                        report.values.note(),
                    )
                    .expect("some rooms of valid values are valid values");
                }
                Some(report)
            })
            .filter(|report| {
                let intensity = report.values.intensity().get();
                intensity >= self.min_level && !(self.hide_clean && intensity == 0)
            })
            .collect()
    }

    /// The PM2.5 readings the graphs show: those of the person, in the part of the day.
    pub fn readings(
        &self,
        readings: Vec<Pm25Reading>,
        schedule: &DailySchedule,
    ) -> Vec<Pm25Reading> {
        readings
            .into_iter()
            .filter(|reading| self.keeps(reading.user_id, reading.observed_at, schedule))
            .collect()
    }

    /// Whether the person and the part of the day let a record through.
    fn keeps(&self, user: UserId, observed_at: DateTime<Utc>, schedule: &DailySchedule) -> bool {
        let hour = observed_at.with_timezone(&schedule.tz()).hour();
        self.person.is_none_or(|person| person == user)
            && self.day_part.is_none_or(|part| part.contains(hour))
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
        let february = Period::month(date("2026-02-14"));
        assert_eq!(february.granularity(), Granularity::Day);
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
        let year = Period::year(date("2026-06-01"));
        assert_eq!(
            (year.first_day(), year.last_day()),
            (date("2026-01-01"), date("2026-12-31"))
        );
        assert_eq!(year.granularity(), Granularity::Week);
        let weeks = year.buckets();
        assert_eq!(weeks[0], date("2025-12-29"));
        assert_eq!(weeks[1], date("2026-01-05"));
        assert_eq!(*weeks.last().unwrap(), date("2026-12-28"));
        assert_eq!(weeks.len(), 53);
        assert_eq!(year.bucket_of(date("2026-01-01")), date("2025-12-29"));
        assert_eq!(year.bucket_of(date("2026-01-11")), date("2026-01-05"));
    }

    #[test]
    fn the_last_days_stay_between_one_day_and_the_longest_period() {
        let today = date("2026-10-14");
        assert_eq!(Period::last_days(today, 0).days(), 1);
        assert_eq!(Period::last_days(today, u64::MAX).days(), Period::MAX_DAYS);
    }

    #[test]
    fn up_to_about_two_months_is_counted_per_day() {
        let today = date("2026-10-14");
        let week = Period::last_days(today, 7);
        assert_eq!((week.first_day(), week.days()), (date("2026-10-08"), 7));
        assert_eq!(Period::last_days(today, 30).granularity(), Granularity::Day);
        let two_months = Period::new(date("2026-01-01"), date("2026-03-03")).unwrap();
        assert_eq!(
            (two_months.days(), two_months.granularity()),
            (62, Granularity::Day)
        );
        let longer = Period::new(date("2026-01-01"), date("2026-03-04")).unwrap();
        assert_eq!(longer.granularity(), Granularity::Week);
    }

    #[test]
    fn a_period_runs_forwards_and_at_most_two_years() {
        let day = date("2026-10-14");
        assert_eq!(Period::new(day, day).map(|p| p.days()), Some(1));
        assert_eq!(Period::new(day, date("2026-10-13")), None, "backwards");
        assert_eq!(
            Period::new(date("2025-01-01"), date("2027-01-01")).map(|p| p.days()),
            Some(731)
        );
        assert_eq!(
            Period::new(date("2025-01-01"), date("2027-01-02")),
            None,
            "732 days"
        );
    }

    #[test]
    fn the_night_runs_across_midnight() {
        let parts = |hour| {
            DayPart::ALL
                .into_iter()
                .filter(|part| part.contains(hour))
                .collect::<Vec<_>>()
        };
        assert_eq!(parts(5), [DayPart::Night]);
        assert_eq!(parts(6), [DayPart::Morning]);
        assert_eq!(parts(11), [DayPart::Morning]);
        assert_eq!(parts(12), [DayPart::Afternoon]);
        assert_eq!(parts(18), [DayPart::Evening]);
        assert_eq!(parts(21), [DayPart::Evening]);
        assert_eq!(parts(22), [DayPart::Night]);
        assert_eq!(parts(0), [DayPart::Night]);
        assert!(
            (0..24).all(|hour| parts(hour).len() == 1),
            "every hour in one part"
        );
    }

    fn levels_of(reports: &[SmellReport]) -> Vec<Vec<(i64, u8)>> {
        reports
            .iter()
            .map(|r| {
                r.values
                    .rooms()
                    .iter()
                    .map(|room| (room.room.0, room.level.get()))
                    .collect()
            })
            .collect()
    }

    fn sample() -> Vec<SmellReport> {
        vec![
            // 21:00 in Rome, by user 1: bedroom (1) 6, stairwell (6) 2.
            report(1, 1, "2026-01-10T20:00:00Z", &[(1, 6), (6, 2)], &[]),
            // 08:00 in Rome, by user 2: kitchen (2) 3.
            report(1, 2, "2026-01-11T07:00:00Z", &[(2, 3)], &[]),
            // 23:30 in Rome, by user 1: a whole-home check.
            report(0, 1, "2026-01-11T22:30:00Z", &[], &[]),
            // 14:00 in Rome, by user 2: bedroom 0, stairwell 4.
            report(2, 2, "2026-01-12T13:00:00Z", &[(1, 0), (6, 4)], &[]),
        ]
    }

    fn rooms() -> Vec<Room> {
        vec![
            room(1, RoomArea::Inside),
            room(2, RoomArea::Inside),
            room(6, RoomArea::Common),
        ]
    }

    #[test]
    fn no_filter_keeps_everything() {
        let kept = Filter::default().reports(sample(), &rooms(), &rome());
        assert_eq!(kept, sample());
    }

    #[test]
    fn chosen_rooms_narrow_each_report_and_win_over_the_area() {
        let filter = Filter {
            rooms: vec![RoomId(1)],
            area: Some(RoomArea::Common),
            ..Filter::default()
        };
        let kept = filter.reports(sample(), &rooms(), &rome());
        assert_eq!(
            levels_of(&kept),
            [vec![(1, 6)], vec![(1, 0)]],
            "the whole-home check names no room"
        );
        assert_eq!(
            kept[1].values.intensity().get(),
            0,
            "the intensity of the rooms kept"
        );
    }

    #[test]
    fn an_area_keeps_its_rooms() {
        let filter = Filter {
            area: Some(RoomArea::Common),
            ..Filter::default()
        };
        let kept = filter.reports(sample(), &rooms(), &rome());
        assert_eq!(levels_of(&kept), [vec![(6, 2)], vec![(6, 4)]]);
    }

    #[test]
    fn the_person_and_the_part_of_the_day_apply_to_reports_and_readings() {
        let filter = Filter {
            person: Some(UserId(1)),
            day_part: Some(DayPart::Night),
            ..Filter::default()
        };
        let kept = filter.reports(sample(), &rooms(), &rome());
        assert_eq!(levels_of(&kept), [Vec::<(i64, u8)>::new()], "23:30, user 1");
        let reading = |user, at: &str| Pm25Reading {
            id: crate::types::ReadingId(1),
            event_id: None,
            user_id: UserId(user),
            observed_at: t(at),
            recorded_at: t(at),
            value: crate::types::Pm25::new(10.0).unwrap(),
            deleted_at: None,
        };
        let readings = vec![
            reading(1, "2026-01-10T22:00:00Z"),
            reading(2, "2026-01-10T22:00:00Z"),
            reading(1, "2026-01-10T12:00:00Z"),
        ];
        let kept = filter.readings(readings, &rome());
        assert_eq!(kept.len(), 1);
        assert_eq!(
            (kept[0].user_id, kept[0].observed_at),
            (UserId(1), t("2026-01-10T22:00:00Z"))
        );
        let by_room = Filter {
            rooms: vec![RoomId(1)],
            min_level: 9,
            ..Filter::default()
        };
        assert_eq!(
            by_room
                .readings(vec![reading(2, "2026-01-10T12:00:00Z")], &rome())
                .len(),
            1,
            "rooms and level skip readings"
        );
    }

    #[test]
    fn the_minimum_level_and_hiding_clean_checks_judge_the_rooms_kept() {
        let at_least_four = Filter {
            min_level: 4,
            ..Filter::default()
        };
        assert_eq!(
            levels_of(&at_least_four.reports(sample(), &rooms(), &rome())),
            [vec![(1, 6), (6, 2)], vec![(1, 0), (6, 4)]]
        );
        let bedroom_from_four = Filter {
            rooms: vec![RoomId(1)],
            min_level: 4,
            ..Filter::default()
        };
        assert_eq!(
            levels_of(&bedroom_from_four.reports(sample(), &rooms(), &rome())),
            [vec![(1, 6)]]
        );
        let no_clean = Filter {
            hide_clean: true,
            ..Filter::default()
        };
        let kept = no_clean.reports(sample(), &rooms(), &rome());
        assert_eq!(kept.len(), 3, "the whole-home check goes");
        let bedroom_no_clean = Filter {
            rooms: vec![RoomId(1)],
            hide_clean: true,
            ..Filter::default()
        };
        assert_eq!(
            levels_of(&bedroom_no_clean.reports(sample(), &rooms(), &rome())),
            [vec![(1, 6)]]
        );
    }

    #[test]
    fn an_event_counts_when_any_of_its_reports_is_kept() {
        let filter = Filter {
            area: Some(RoomArea::Common),
            min_level: 3,
            ..Filter::default()
        };
        let kept = filter.reports(sample(), &rooms(), &rome());
        let counts = events_per_bucket(&kept, Period::month(date("2026-01-01")), &rome());
        let events: usize = counts.iter().map(|(_, n)| n).sum();
        assert_eq!(
            events, 1,
            "event 2 through its stairwell 4; event 1's stairwell 2 is too low"
        );
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
        let january = Period::month(date("2026-01-01"));
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
        let year = Period::year(date("2026-01-01"));
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
        let january = Period::month(date("2026-01-01"));
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
