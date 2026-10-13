//! The storage contract: behaviour every backend must share, written once against the
//! `smoke-domain` traits. A backend crate runs all of it with
//!
//! ```ignore
//! smoke_storage_contract::contract_tests!(fixture);
//! ```
//!
//! where `fixture` is an `async fn() -> Option<Fixture<MyStore>>` that gives each test a fresh,
//! migrated database, or `None` to skip.

pub mod audit;
pub mod events;
pub mod fixture;
pub mod lists;
pub mod readings;
pub mod reports;
pub mod sessions;
pub mod sweep;
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
            lists::symptoms_can_be_created_and_hidden,
            lists::hiding_an_unknown_symptom_is_not_found,
            events::a_new_event_can_be_the_open_event,
            events::only_one_event_can_be_open,
            events::closing_and_reopening_are_audited,
            events::closing_an_event_before_it_opened_breaks_a_data_rule,
            events::reopening_is_refused_while_another_event_is_open,
            events::the_end_of_a_closed_event_can_move,
            events::moving_an_event_end_is_audited_and_checked,
            events::the_auto_close_notice_is_marked_seen_once,
            events::events_between_returns_overlapping_events,
            events::unseen_auto_closed_events_are_listed_until_seen,
            events::event_at_finds_the_event_whose_span_contains_the_time,
            reports::a_report_keeps_its_rooms_and_symptoms,
            reports::correcting_a_report_records_before_and_after,
            reports::moving_a_report_is_audited,
            reports::deleting_a_report_is_soft,
            reports::latest_report_time_ignores_deleted_reports,
            reports::an_unknown_room_rolls_back_the_report,
            reports::correcting_a_report_can_remove_rooms_and_symptoms,
            reports::an_unknown_room_on_correction_keeps_the_original_links,
            reports::moving_a_report_to_an_unknown_event_is_rejected,
            reports::an_unknown_symptom_rolls_back_the_report,
            reports::reports_between_returns_live_reports_with_their_links,
            readings::a_reading_can_stand_alone_or_join_an_event,
            readings::correcting_and_deleting_a_reading_are_audited,
            readings::a_reading_for_an_unknown_event_is_rejected,
            readings::pm25_values_round_trip_exactly,
            readings::job_runs_are_remembered_but_not_audited,
            readings::readings_between_returns_live_readings_in_the_window,
            audit::every_write_extends_one_unbroken_chain,
            audit::the_history_of_a_record_lists_its_entries,
            audit::a_direct_record_edit_is_detected,
            audit::a_directly_inserted_record_is_detected,
            audit::a_directly_deleted_record_is_detected,
            audit::soft_deletes_still_verify,
            audit::a_direct_link_table_edit_is_detected,
            audit::the_audit_log_refuses_updates_and_deletes,
            audit::an_edited_audit_entry_breaks_the_chain,
            audit::a_directly_edited_username_is_corrupt_not_normalised,
            audit::a_directly_edited_list_name_is_corrupt_not_normalised,
            audit::a_directly_edited_note_is_corrupt_not_normalised,
            audit::concurrent_writers_never_fork_the_chain,
            sessions::a_session_round_trips_until_it_expires,
            sessions::creating_a_session_twice_is_a_duplicate,
            sessions::a_session_for_an_unknown_user_is_refused,
            sessions::saving_a_session_inserts_or_replaces_it,
            sessions::deleting_a_session_removes_only_that_one,
            sessions::expired_sessions_are_swept,
            sessions::a_password_change_ends_the_users_sessions,
            sessions::expiring_a_user_ends_their_sessions,
            sessions::a_rolled_back_password_change_keeps_the_sessions,
            sessions::sessions_are_not_audited,
            sweep::the_first_sweep_runs_and_is_remembered,
            sweep::a_sweep_at_four_closes_the_event_at_its_last_report,
            sweep::a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four,
            sweep::an_event_opened_after_the_boundary_stays_open,
            sweep::an_event_without_reports_closes_where_it_opened,
            sweep::on_dst_days_the_sweep_still_runs_at_four_local,
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
