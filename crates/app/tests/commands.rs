//! The commands against a temporary SQLite database.

use std::sync::Arc;

use smoke_app::backend::Backend;
use smoke_app::commands::{create_resident, verify_audit};
use smoke_domain::clock::SystemClock;
use smoke_domain::repos::{AuditRepo, Conflict, ListRepo, StoreError, UserRepo};
use smoke_domain::types::{Locale, NewPassword, Role, Username};
use smoke_storage_sqlite::SqliteStore;
use smoke_web::password;
use sqlx::SqlitePool;
use tempfile::TempDir;

const PASSWORD: &str = "correct horse battery";

async fn prepared() -> (TempDir, String, SqliteStore) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("app.db").display());
    let backend = Backend::connect(&url, Arc::new(SystemClock)).await.unwrap();
    backend.prepare().await.unwrap();
    let Backend::Sqlite(store) = backend else {
        unreachable!("a sqlite URL")
    };
    (dir, url, store)
}

fn username(name: &str) -> Username {
    Username::new(name).unwrap()
}

fn password() -> NewPassword {
    NewPassword::new(PASSWORD.to_owned()).unwrap()
}

#[tokio::test]
async fn prepare_can_run_again_and_seeds_the_lists_once() {
    let (_dir, url, store) = prepared().await;
    let again = Backend::connect(&url, Arc::new(SystemClock)).await.unwrap();
    again.prepare().await.unwrap();
    assert_eq!(store.rooms().await.unwrap().len(), 6);
    assert_eq!(store.symptoms().await.unwrap().len(), 6);
    assert_eq!(store.audit_log().await.unwrap().len(), 12);
}

#[tokio::test]
async fn an_unknown_scheme_is_refused_before_connecting() {
    let error = Backend::connect("mysql://db/smoke", Arc::new(SystemClock))
        .await
        .err()
        .unwrap();
    assert_eq!(
        error.to_string(),
        "DATABASE_URL must start with sqlite: or postgres://"
    );
}

#[tokio::test]
async fn create_resident_stores_an_argon2_hash_and_audits_without_it() {
    let (_dir, _url, store) = prepared().await;
    let user = create_resident::create(&store, username(" Daniele "), password(), Locale::It)
        .await
        .unwrap();
    assert_eq!(user.username.as_str(), "daniele");
    assert_eq!(
        (user.role, user.locale, user.expires_at),
        (Role::Resident, Locale::It, None)
    );
    let stored = store.user(user.id).await.unwrap().unwrap();
    assert!(password::verify(stored.password_hash.as_ref(), PASSWORD));
    let log = format!("{:?}", store.audit_log().await.unwrap());
    assert!(!log.contains("argon2"), "the hash reached the audit log");
}

#[tokio::test]
async fn create_resident_refuses_a_taken_username() {
    let (_dir, _url, store) = prepared().await;
    create_resident::create(&store, username("daniele"), password(), Locale::En)
        .await
        .unwrap();
    let error = create_resident::create(&store, username("DANIELE"), password(), Locale::En)
        .await
        .unwrap_err();
    assert!(
        matches!(
            error.downcast_ref::<StoreError>(),
            Some(StoreError::Conflict(Conflict::UsernameTaken))
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn verify_audit_passes_on_honest_data() {
    let (_dir, _url, store) = prepared().await;
    create_resident::create(&store, username("daniele"), password(), Locale::En)
        .await
        .unwrap();
    let report = verify_audit::run(&store).await.unwrap();
    assert!(report.is_ok());
    assert_eq!(report.entries, 13);
    let text = verify_audit::describe(&report);
    assert!(text.contains("hash chain:    ok"), "{text}");
    assert!(text.contains(&report.head_hash), "{text}");
}

#[tokio::test]
async fn verify_audit_reports_a_direct_database_edit() {
    let (_dir, url, store) = prepared().await;
    let pool = SqlitePool::connect(&url).await.unwrap();
    sqlx::raw_sql("UPDATE rooms SET name = 'Garage' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let report = verify_audit::run(&store).await.unwrap();
    assert!(!report.is_ok());
    let text = verify_audit::describe(&report);
    assert!(
        text.contains("room 1: differs from audit entry 1 (edited outside the app)"),
        "{text}"
    );
}
