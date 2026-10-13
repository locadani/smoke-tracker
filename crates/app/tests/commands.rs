//! The commands against a temporary SQLite database.

use std::sync::Arc;

use smoke_app::backend::Backend;
use smoke_domain::clock::SystemClock;
use smoke_domain::repos::{AuditRepo, ListRepo};
use smoke_storage_sqlite::SqliteStore;
use tempfile::TempDir;

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
