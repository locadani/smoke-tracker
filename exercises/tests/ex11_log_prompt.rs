use smoke_exercises::ex11_log_prompt::{
    BACKDATE_THRESHOLD, DAY, DailyBoundary, EventSummary, FUTURE_TOLERANCE, LARGE_GAP, LogPrompt,
    NewEventState, ObservedTime, TimeError, classify_observed, large_gap_indices, log_prompt,
    new_event_state, no_smell_prompt,
};

const NOW: u64 = 100 * DAY + 20 * 3600;
const FOUR: DailyBoundary = DailyBoundary { offset: 4 * 3600 };

fn event(id: u32) -> EventSummary {
    EventSummary {
        id,
        opened_at: NOW - 3600,
    }
}

#[test]
fn a_time_close_to_now_is_now() {
    assert_eq!(classify_observed(NOW, NOW), Ok(ObservedTime::Now));
    assert_eq!(
        classify_observed(NOW - BACKDATE_THRESHOLD, NOW),
        Ok(ObservedTime::Now)
    );
    assert_eq!(
        classify_observed(NOW + FUTURE_TOLERANCE, NOW),
        Ok(ObservedTime::Now)
    );
}

#[test]
fn a_time_further_back_is_backdated() {
    assert_eq!(
        classify_observed(NOW - BACKDATE_THRESHOLD - 1, NOW),
        Ok(ObservedTime::Backdated)
    );
    assert_eq!(classify_observed(0, NOW), Ok(ObservedTime::Backdated));
}

#[test]
fn a_time_further_ahead_than_the_tolerance_is_refused() {
    assert_eq!(
        classify_observed(NOW + FUTURE_TOLERANCE + 1, NOW),
        Err(TimeError::ObservedInFuture)
    );
}

#[test]
fn near_the_start_of_time_nothing_overflows() {
    assert_eq!(classify_observed(0, 5), Ok(ObservedTime::Now));
}

#[test]
fn a_backdated_report_always_asks_which_event() {
    let open = Some(event(1));
    let closed = Some(event(2));
    assert_eq!(
        log_prompt(ObservedTime::Backdated, open, closed),
        LogPrompt::WhichEvent
    );
    assert_eq!(
        log_prompt(ObservedTime::Backdated, None, None),
        LogPrompt::WhichEvent
    );
}

#[test]
fn an_open_event_is_asked_about_before_an_auto_closed_one() {
    assert_eq!(
        log_prompt(ObservedTime::Now, Some(event(1)), Some(event(2))),
        LogPrompt::SameSmell(event(1))
    );
    assert_eq!(
        log_prompt(ObservedTime::Now, None, Some(event(2))),
        LogPrompt::AutoClosed(event(2))
    );
    assert_eq!(log_prompt(ObservedTime::Now, None, None), LogPrompt::Direct);
}

#[test]
fn a_no_smell_report_asks_only_when_backdated() {
    assert_eq!(no_smell_prompt(ObservedTime::Now), LogPrompt::Direct);
    assert_eq!(
        no_smell_prompt(ObservedTime::Backdated),
        LogPrompt::WhichEvent
    );
}

#[test]
fn the_last_boundary_is_todays_once_it_has_passed() {
    assert_eq!(
        FOUR.last_before(100 * DAY + 10 * 3600),
        100 * DAY + 4 * 3600
    );
    assert_eq!(FOUR.last_before(100 * DAY + 4 * 3600), 100 * DAY + 4 * 3600);
}

#[test]
fn before_todays_boundary_it_is_yesterdays() {
    assert_eq!(FOUR.last_before(100 * DAY + 3 * 3600), 99 * DAY + 4 * 3600);
    assert_eq!(FOUR.last_before(100 * DAY), 99 * DAY + 4 * 3600);
}

#[test]
fn a_boundary_passed_since_means_one_lies_after_that_instant_up_to_now() {
    let before = 100 * DAY + 3 * 3600;
    let after = 100 * DAY + 5 * 3600;
    assert!(FOUR.passed_since(before, after));
    assert!(!FOUR.passed_since(100 * DAY + 4 * 3600, after));
    assert!(!FOUR.passed_since(after, after + 3600));
}

#[test]
fn a_new_event_is_open_when_nothing_else_is() {
    assert_eq!(
        new_event_state(NOW - 60, NOW, &FOUR, false),
        NewEventState::Open
    );
}

#[test]
fn a_new_event_is_born_closed_when_another_is_open() {
    assert_eq!(
        new_event_state(NOW - 60, NOW, &FOUR, true),
        NewEventState::ClosedBackdated
    );
}

#[test]
fn a_new_event_is_born_closed_when_a_boundary_has_passed() {
    let yesterday_evening = 99 * DAY + 22 * 3600;
    assert_eq!(
        new_event_state(yesterday_evening, 100 * DAY + 8 * 3600, &FOUR, false),
        NewEventState::ClosedBackdated
    );
}

#[test]
fn only_gaps_longer_than_three_hours_are_flagged() {
    assert_eq!(large_gap_indices(&[0, 100, 100 + LARGE_GAP + 1]), [2]);
    assert!(large_gap_indices(&[0, LARGE_GAP]).is_empty());
    assert_eq!(
        large_gap_indices(&[0, LARGE_GAP + 1, LARGE_GAP + 2, 3 * LARGE_GAP]),
        [1, 3]
    );
    assert!(large_gap_indices(&[]).is_empty());
    assert!(large_gap_indices(&[5]).is_empty());
}
