use std::sync::Arc;

use smoke_domain::clock::{FakeClock, SystemClock};
use smoke_storage_contract::fixture::{Fixture, raw_runner, start};
use smoke_storage_contract::migrations;
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
            "sessions",
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

#[test]
fn the_sqlite_store_is_a_complete_store() {
    fn assert_store<S: smoke_domain::repos::Store>() {}
    assert_store::<SqliteStore>();
}

#[tokio::test]
async fn in_memory_databases_are_rejected() {
    for url in [
        "sqlite::memory:",
        "sqlite://file:smoke?mode=memory&cache=shared",
    ] {
        let error = SqliteStore::connect(url, Arc::new(SystemClock))
            .await
            .err()
            .unwrap_or_else(|| panic!("{url} was accepted"));
        assert!(
            error.to_string().contains("database error"),
            "{url}: {error:?}"
        );
        assert!(
            format!("{error:?}").contains("in-memory SQLite is not supported"),
            "{url}: {error:?}"
        );
    }
}

#[tokio::test]
async fn a_second_writer_waits_at_begin_instead_of_failing_at_commit() {
    use smoke_domain::repos::Transactional;

    let (_dir, url) = database();
    let first = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    first.migrate().await.unwrap();
    let second = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();

    let held = first.begin(None).await.unwrap();
    assert!(
        second.begin(None).await.is_err(),
        "the write lock is taken when the transaction starts"
    );
    drop(held);
    second
        .begin(None)
        .await
        .expect("the lock is free again once the first transaction ends");
}

/// A database migrated up to `version`, with `rows` written by hand.
async fn database_at(version: i64, rows: &'static str) -> (TempDir, String) {
    let (dir, url) = database();
    let pool = SqlitePool::connect(&format!("{url}?mode=rwc"))
        .await
        .unwrap();
    sqlx::migrate!("./migrations")
        .run_to(version, &pool)
        .await
        .unwrap();
    sqlx::raw_sql(rows).execute(&pool).await.unwrap();
    pool.close().await;
    (dir, url)
}

#[tokio::test]
async fn migration_0003_gives_each_room_its_reports_intensity() {
    let (_dir, url) = database_at(2, migrations::ROWS_BEFORE_0003).await;
    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    migrations::each_room_took_its_reports_intensity(&store).await;
}

#[tokio::test]
async fn migration_0003_refuses_a_report_without_rooms_and_changes_nothing() {
    let (_dir, url) = database_at(2, migrations::ROWS_BEFORE_0003).await;
    let pool = SqlitePool::connect(&url).await.unwrap();
    sqlx::raw_sql(migrations::REPORT_WITHOUT_ROOMS)
        .execute(&pool)
        .await
        .unwrap();
    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    let error = store.migrate().await.unwrap_err();
    assert!(
        format!("{error:?}").contains(migrations::NEEDS_A_ROOM),
        "{error:?}"
    );
    let intensities: Vec<i64> =
        sqlx::query_scalar("SELECT intensity FROM smell_reports ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(intensities, [6, 4, 5]);
}
