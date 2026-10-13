//! A time of day that recurs in a local time zone, such as the 04:00 auto-close.

use chrono::{DateTime, LocalResult, NaiveDate, TimeDelta, TimeZone, Utc};
use chrono_tz::Tz;

use crate::DomainError;

/// "Every day at `hour`:00 local time in `tz`."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailySchedule {
    tz: Tz,
    hour: u32,
}

impl DailySchedule {
    pub fn new(tz: Tz, hour: u32) -> Result<Self, DomainError> {
        if hour < 24 {
            Ok(Self { tz, hour })
        } else {
            Err(DomainError::InvalidHour(hour))
        }
    }

    pub fn tz(&self) -> Tz {
        self.tz
    }

    pub fn hour(&self) -> u32 {
        self.hour
    }

    /// The most recent boundary at or before `now`.
    pub fn last_boundary(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        let today = now.with_timezone(&self.tz).date_naive();
        let candidate = self.boundary_on(today);
        if candidate <= now {
            candidate
        } else {
            let yesterday = today.pred_opt().expect("date within chrono's range");
            self.boundary_on(yesterday)
        }
    }

    /// Whether at least one boundary lies in `(since, now]`.
    pub fn boundary_passed_since(&self, since: DateTime<Utc>, now: DateTime<Utc>) -> bool {
        self.last_boundary(now) > since
    }

    /// The boundary instant on a local calendar date. A local time skipped by DST resolves to
    /// the end of the gap; a repeated one resolves to its first occurrence.
    fn boundary_on(&self, date: NaiveDate) -> DateTime<Utc> {
        let local = date
            .and_hms_opt(self.hour, 0, 0)
            .expect("hour validated in new()");
        let instant = match self.tz.from_local_datetime(&local) {
            LocalResult::Single(t) => t,
            LocalResult::Ambiguous(earliest, _) => earliest,
            LocalResult::None => (1..=96)
                .find_map(|step| {
                    let later = local + TimeDelta::minutes(15 * step);
                    self.tz.from_local_datetime(&later).earliest()
                })
                .expect("no time-zone transition skips a whole day"),
        };
        instant.with_timezone(&Utc)
    }
}

#[cfg(test)]
mod tests {
    use chrono_tz::Europe::Rome;

    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn rome_at(hour: u32) -> DailySchedule {
        DailySchedule::new(Rome, hour).unwrap()
    }

    #[test]
    fn rejects_hours_outside_the_day() {
        assert_eq!(
            DailySchedule::new(Rome, 24),
            Err(DomainError::InvalidHour(24))
        );
        assert!(DailySchedule::new(Rome, 23).is_ok());
    }

    #[test]
    fn winter_boundary_is_four_local_three_utc() {
        assert_eq!(
            rome_at(4).last_boundary(t("2026-01-15T10:00:00Z")),
            t("2026-01-15T03:00:00Z")
        );
    }

    #[test]
    fn before_todays_boundary_returns_yesterdays() {
        assert_eq!(
            rome_at(4).last_boundary(t("2026-01-15T02:59:00Z")),
            t("2026-01-14T03:00:00Z")
        );
    }

    #[test]
    fn exactly_at_the_boundary_returns_it() {
        assert_eq!(
            rome_at(4).last_boundary(t("2026-01-15T03:00:00Z")),
            t("2026-01-15T03:00:00Z")
        );
    }

    #[test]
    fn summer_boundary_is_four_local_two_utc() {
        assert_eq!(
            rome_at(4).last_boundary(t("2026-07-01T12:00:00Z")),
            t("2026-07-01T02:00:00Z")
        );
    }

    #[test]
    fn spring_forward_day_still_closes_at_four_local() {
        assert_eq!(
            rome_at(4).last_boundary(t("2026-03-29T08:00:00Z")),
            t("2026-03-29T02:00:00Z")
        );
    }

    #[test]
    fn fall_back_day_still_closes_at_four_local() {
        assert_eq!(
            rome_at(4).last_boundary(t("2026-10-25T12:00:00Z")),
            t("2026-10-25T03:00:00Z")
        );
    }

    #[test]
    fn a_skipped_local_hour_resolves_to_the_end_of_the_gap() {
        // 02:00 does not exist in Rome on 2026-03-29; the boundary becomes 03:00 CEST.
        assert_eq!(
            rome_at(2).last_boundary(t("2026-03-29T08:00:00Z")),
            t("2026-03-29T01:00:00Z")
        );
    }

    #[test]
    fn a_repeated_local_hour_resolves_to_its_first_occurrence() {
        // 02:00 happens twice in Rome on 2026-10-25 (CEST, then CET); take the first.
        assert_eq!(
            rome_at(2).last_boundary(t("2026-10-25T12:00:00Z")),
            t("2026-10-25T00:00:00Z")
        );
    }

    #[test]
    fn a_dst_gap_longer_than_an_hour_still_resolves() {
        // Antarctica/Troll jumps from 01:00 (UTC+0) to 03:00 (UTC+2) on 2026-03-29.
        let troll = DailySchedule::new(chrono_tz::Antarctica::Troll, 1).unwrap();
        assert_eq!(
            troll.last_boundary(t("2026-03-29T12:00:00Z")),
            t("2026-03-29T01:00:00Z")
        );
    }

    #[test]
    fn boundary_passed_since_detects_a_crossing() {
        let s = rome_at(4);
        assert!(s.boundary_passed_since(t("2026-01-14T23:00:00Z"), t("2026-01-15T08:00:00Z")));
        assert!(s.boundary_passed_since(t("2026-01-14T23:00:00Z"), t("2026-01-15T03:00:00Z")));
    }

    #[test]
    fn boundary_passed_since_is_false_without_a_crossing() {
        let s = rome_at(4);
        // The sweep ran exactly at the boundary: not due again until tomorrow.
        assert!(!s.boundary_passed_since(t("2026-01-15T03:00:00Z"), t("2026-01-15T20:00:00Z")));
        assert!(!s.boundary_passed_since(t("2026-01-15T04:00:00Z"), t("2026-01-15T05:00:00Z")));
    }

    #[test]
    fn catch_up_after_sleeping_through_several_nights_is_still_one_sweep() {
        let s = rome_at(4);
        assert!(s.boundary_passed_since(t("2026-01-10T12:00:00Z"), t("2026-01-15T08:00:00Z")));
    }
}
