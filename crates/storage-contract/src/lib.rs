//! The storage contract: behaviour every backend must share, written once against the
//! `smoke-domain` traits. A backend crate runs all of it with
//!
//! ```ignore
//! smoke_storage_contract::contract_tests!(fixture);
//! ```
//!
//! where `fixture` is an `async fn() -> Option<Fixture<MyStore>>` that gives each test a fresh,
//! migrated database, or `None` to skip.

pub mod events;
pub mod fixture;
pub mod lists;
pub mod readings;
pub mod reports;
pub mod transactions;
pub mod users;

pub use fixture::Fixture;

/// Generates one `#[tokio::test]` per contract test, each with its own fixture.
#[macro_export]
macro_rules! contract_tests {
    ($fixture:path) => {
        $crate::contract_tests!(@each $fixture;
            transactions::an_empty_transaction_commits,
            transactions::the_audit_log_starts_empty,
            users::creates_and_finds_a_user,
            users::usernames_are_unique_ignoring_case,
            users::creating_a_user_is_audited_without_the_password_hash,
            users::a_password_change_is_always_audited,
            users::a_locale_change_is_audited_but_a_no_op_is_not,
            users::guests_can_be_expired_and_reactivated,
            users::residents_cannot_expire,
            users::a_dropped_transaction_leaves_no_trace,
            users::instants_are_kept_to_the_microsecond,
            users::writing_an_unknown_user_is_not_found,
            lists::seeding_adds_the_starter_lists_once,
            lists::a_custom_room_has_no_translation_key,
            lists::hiding_a_room_is_audited_once,
            lists::hiding_an_unknown_symptom_is_not_found,
            events::a_new_event_can_be_the_open_event,
            events::only_one_event_can_be_open,
            events::closing_and_reopening_are_audited,
            events::reopening_is_refused_while_another_event_is_open,
            events::the_end_of_a_closed_event_can_move,
            events::the_auto_close_notice_is_marked_seen_once,
            events::events_between_returns_overlapping_events,
            reports::a_report_keeps_its_rooms_and_symptoms,
            reports::correcting_a_report_records_before_and_after,
            reports::moving_a_report_is_audited,
            reports::deleting_a_report_is_soft,
            reports::latest_report_time_ignores_deleted_reports,
            reports::an_unknown_room_rolls_back_the_report,
            readings::a_reading_can_stand_alone_or_join_an_event,
            readings::correcting_and_deleting_a_reading_are_audited,
            readings::pm25_values_round_trip_exactly,
            readings::job_runs_are_remembered_but_not_audited,
        );
    };
    (@each $fixture:path; $($module:ident::$test:ident),+ $(,)?) => {
        $(
            #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
            async fn $test() {
                if let Some(fixture) = $fixture().await {
                    $crate::$module::$test(&fixture).await;
                }
            }
        )+
    };
}
