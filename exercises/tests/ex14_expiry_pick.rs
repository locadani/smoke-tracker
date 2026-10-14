use smoke_exercises::ex02_permissions::Role;
use smoke_exercises::ex14_expiry_pick::{
    DAY, ExpiryError, ExpiryPick, check_expiry, weekday_from_monday,
};

// 2026-10-14, a Wednesday, is day number 20740.
const WEDNESDAY: u64 = 20_740;
const SUNDAY: u64 = 20_744;
const MONDAY: u64 = 20_745;
const NEXT_MONDAY: u64 = 20_752;

#[test]
fn days_know_their_weekday() {
    assert_eq!(weekday_from_monday(0), 3, "1970-01-01 was a Thursday");
    assert_eq!(weekday_from_monday(WEDNESDAY), 2);
    assert_eq!(weekday_from_monday(SUNDAY), 6);
    assert_eq!(weekday_from_monday(MONDAY), 0);
}

#[test]
fn quick_picks_count_from_now() {
    let now = WEDNESDAY * DAY + 18 * 3600;
    assert_eq!(ExpiryPick::OneDay.resolve(now), Some(now + DAY));
    assert_eq!(ExpiryPick::OneWeek.resolve(now), Some(now + 7 * DAY));
    assert_eq!(ExpiryPick::TwoWeeks.resolve(now), Some(now + 14 * DAY));
}

#[test]
fn never_has_no_end() {
    assert_eq!(ExpiryPick::Never.resolve(123), None);
}

#[test]
fn the_weekend_lasts_until_the_coming_monday_morning() {
    let wednesday_evening = WEDNESDAY * DAY + 20 * 3600;
    assert_eq!(
        ExpiryPick::Weekend.resolve(wednesday_evening),
        Some(MONDAY * DAY)
    );
    let sunday_night = SUNDAY * DAY + 23 * 3600;
    assert_eq!(
        ExpiryPick::Weekend.resolve(sunday_night),
        Some(MONDAY * DAY)
    );
}

#[test]
fn on_a_monday_the_weekend_means_the_next_one() {
    let monday_morning = MONDAY * DAY + 8 * 3600;
    assert_eq!(
        ExpiryPick::Weekend.resolve(monday_morning),
        Some(NEXT_MONDAY * DAY)
    );
    assert_eq!(
        ExpiryPick::Weekend.resolve(MONDAY * DAY),
        Some(NEXT_MONDAY * DAY)
    );
}

#[test]
fn a_chosen_day_lasts_until_the_end_of_that_day() {
    assert_eq!(
        ExpiryPick::Until(WEDNESDAY).resolve(WEDNESDAY * DAY + 5),
        Some((WEDNESDAY + 1) * DAY)
    );
    assert_eq!(
        ExpiryPick::Until(NEXT_MONDAY).resolve(0),
        Some((NEXT_MONDAY + 1) * DAY)
    );
}

const NOW: u64 = 1_000;

#[test]
fn residents_never_expire() {
    assert_eq!(
        check_expiry(Role::Resident, Some(NOW + 1), NOW),
        Err(ExpiryError::ResidentsNeverExpire)
    );
    assert_eq!(check_expiry(Role::Resident, None, NOW), Ok(()));
}

#[test]
fn guests_always_expire() {
    assert_eq!(
        check_expiry(Role::Guest, None, NOW),
        Err(ExpiryError::GuestsMustExpire)
    );
    assert_eq!(check_expiry(Role::Guest, Some(NOW + 1), NOW), Ok(()));
}

#[test]
fn viewers_may_or_may_not_expire() {
    assert_eq!(check_expiry(Role::Viewer, None, NOW), Ok(()));
    assert_eq!(check_expiry(Role::Viewer, Some(NOW + 1), NOW), Ok(()));
}

#[test]
fn an_expiry_must_lie_strictly_in_the_future() {
    assert_eq!(
        check_expiry(Role::Guest, Some(NOW), NOW),
        Err(ExpiryError::ExpiryNotInFuture)
    );
    assert_eq!(
        check_expiry(Role::Viewer, Some(NOW - 1), NOW),
        Err(ExpiryError::ExpiryNotInFuture)
    );
}

#[test]
fn a_resident_with_a_past_expiry_is_refused_as_a_resident_first() {
    assert_eq!(
        check_expiry(Role::Resident, Some(NOW - 1), NOW),
        Err(ExpiryError::ResidentsNeverExpire)
    );
}
