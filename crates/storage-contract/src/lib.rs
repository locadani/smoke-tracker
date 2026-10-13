//! The storage contract: behaviour every backend must share, written once against the
//! `smoke-domain` traits. A backend crate runs all of it with
//!
//! ```ignore
//! smoke_storage_contract::contract_tests!(fixture);
//! ```
//!
//! where `fixture` is an `async fn() -> Option<Fixture<MyStore>>` that gives each test a fresh,
//! migrated database, or `None` to skip.

pub mod fixture;
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
