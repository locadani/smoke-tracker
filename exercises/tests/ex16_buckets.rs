use smoke_exercises::ex16_buckets::{Period, events_per_bucket};

// 2026-10-19 is a Monday: day number 20745.
const MON: i64 = 20_745;
const TUE: i64 = MON + 1;
const SUN: i64 = MON + 6;
const NEXT_MON: i64 = MON + 7;

#[test]
fn a_day_period_has_one_bucket_per_day() {
    let period = Period::Days { first: MON, len: 7 };
    assert_eq!(period.end(), MON + 7);
    assert_eq!(period.bucket_of(TUE), TUE);
    assert_eq!(
        period.buckets(),
        [MON, MON + 1, MON + 2, MON + 3, MON + 4, MON + 5, SUN]
    );
}

#[test]
fn a_week_bucket_is_named_by_its_monday() {
    let period = Period::Weeks {
        first: MON,
        len: 14,
    };
    assert_eq!(period.bucket_of(MON), MON);
    assert_eq!(period.bucket_of(SUN), MON);
    assert_eq!(period.bucket_of(NEXT_MON), NEXT_MON);
}

#[test]
fn weeks_start_on_monday_even_when_the_period_starts_midweek() {
    // Wednesday 2026-10-14 for 14 days: the weeks of Mon 12th, 19th and 26th touch it.
    let period = Period::Weeks {
        first: MON - 5,
        len: 14,
    };
    assert_eq!(period.buckets(), [MON - 7, MON, NEXT_MON]);
}

#[test]
fn no_bucket_starts_at_or_after_the_end() {
    let period = Period::Weeks {
        first: MON,
        len: 14,
    };
    assert_eq!(period.buckets(), [MON, NEXT_MON]);
    let one_day = Period::Days { first: MON, len: 1 };
    assert_eq!(one_day.buckets(), [MON]);
}

#[test]
fn an_empty_period_has_no_buckets() {
    assert!(Period::Days { first: MON, len: 0 }.buckets().is_empty());
}

#[test]
fn every_bucket_is_listed_even_without_events() {
    let period = Period::Days { first: MON, len: 3 };
    assert_eq!(
        events_per_bucket(period, &[]),
        [(MON, 0), (MON + 1, 0), (MON + 2, 0)]
    );
}

#[test]
fn an_event_counts_once_on_the_day_of_its_earliest_report() {
    let period = Period::Days { first: MON, len: 4 };
    let reports = [
        (Some(1), MON + 2),
        (Some(1), MON),
        (Some(1), MON + 1),
        (Some(2), MON + 1),
    ];
    assert_eq!(
        events_per_bucket(period, &reports),
        [(MON, 1), (MON + 1, 1), (MON + 2, 0), (MON + 3, 0)]
    );
}

#[test]
fn clean_checks_and_events_outside_the_period_count_for_nothing() {
    let period = Period::Days { first: MON, len: 2 };
    let reports = [
        (None, MON),
        (None, MON + 1),
        (Some(5), MON - 1),
        (Some(6), MON + 9),
    ];
    assert_eq!(
        events_per_bucket(period, &reports),
        [(MON, 0), (MON + 1, 0)]
    );
}

#[test]
fn a_year_style_period_counts_events_per_week() {
    let period = Period::Weeks {
        first: MON,
        len: 14,
    };
    let reports = [
        (Some(1), TUE),
        (Some(2), SUN),
        (Some(3), NEXT_MON),
        (Some(3), NEXT_MON + 3),
    ];
    assert_eq!(
        events_per_bucket(period, &reports),
        [(MON, 2), (NEXT_MON, 1)]
    );
}

#[test]
fn an_event_that_began_before_the_period_is_not_counted_in_it() {
    let period = Period::Days { first: MON, len: 3 };
    let reports = [(Some(1), MON - 1), (Some(1), MON + 1)];
    assert_eq!(
        events_per_bucket(period, &reports),
        [(MON, 0), (MON + 1, 0), (MON + 2, 0)]
    );
}
