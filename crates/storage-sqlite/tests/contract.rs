use std::sync::Arc;

use smoke_domain::clock::{FakeClock, SystemClock};
use smoke_storage_contract::fixture::{Fixture, raw_runner, start};
use smoke_storage_sqlite::SqliteStore;
use sqlx::SqlitePool;
use tempfile::TempDir;

fn database() -> (TempDir, String) {
    let dir = tempfile::tempdir().expect("temp dir");
    let url = format!("sqlite://{}", dir.path().join("test.db").display());
    (dir, url)
}

async fn fixture() -> Option<Fixture<SqliteStore>> {
    let (dir, url) = database();
    let clock = Arc::new(FakeClock::new(start()));
    let store = SqliteStore::connect(&url, clock.clone())
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");
    let pool = SqlitePool::connect(&url).await.expect("raw connection");
    let raw = raw_runner(move |sql| {
        let pool = pool.clone();
        async move {
            sqlx::raw_sql(sql)
                .execute(&pool)
                .await
                .map(drop)
                .map_err(|e| e.to_string())
        }
    });
    let drop_guard = "DROP TRIGGER audit_log_no_update; DROP TRIGGER audit_log_no_delete;";
    Some(Fixture::new(store, clock, raw, drop_guard, dir))
}

smoke_storage_contract::contract_tests!(fixture);

#[tokio::test]
async fn migrations_create_every_table_and_can_run_again() {
    let (_dir, url) = database();
    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store.migrate().await.unwrap();

    let pool = SqlitePool::connect(&url).await.unwrap();
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' \
         AND name NOT LIKE 'sqlite_%' AND name <> '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        tables,
        [
            "audit_log",
            "events",
            "invites",
            "jobs",
            "pm25_readings",
            "rooms",
            "smell_report_rooms",
            "smell_report_symptoms",
            "smell_reports",
            "symptoms",
            "users",
        ]
    );
}

#[tokio::test]
async fn the_database_uses_write_ahead_logging() {
    let (_dir, url) = database();
    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let pool = SqlitePool::connect(&url).await.unwrap();
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(mode, "wal");
}
