use std::sync::Arc;

use smoke_domain::audit::ChainedEntry;
use smoke_domain::clock::{FakeClock, SystemClock};
use smoke_domain::repos::AuditRepo;
use smoke_storage_contract::fixture::{Fixture, RawSql, raw_runner, start};
use smoke_storage_contract::migrations;
use smoke_storage_sqlite::SqliteStore;
use sqlx::AssertSqlSafe;
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
            "log_entries",
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
async fn database_at(version: i64, rows: &str) -> (TempDir, String) {
    let (dir, url) = database();
    let pool = SqlitePool::connect(&format!("{url}?mode=rwc"))
        .await
        .unwrap();
    sqlx::migrate!("./migrations")
        .run_to(version, &pool)
        .await
        .unwrap();
    sqlx::raw_sql(AssertSqlSafe(rows.to_owned()))
        .execute(&pool)
        .await
        .unwrap();
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

#[tokio::test]
async fn after_the_rebuild_the_link_tables_still_refer_to_the_reports() {
    let (_dir, url) = database();
    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let pool = SqlitePool::connect(&url).await.unwrap();
    for table in ["smell_report_rooms", "smell_report_symptoms"] {
        let targets: Vec<String> =
            sqlx::query_scalar("SELECT \"table\" FROM pragma_foreign_key_list($1)")
                .bind(table)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(
            targets.contains(&"smell_reports".to_owned()),
            "{table}: {targets:?}"
        );
    }
}

/// A version-4 database with [`migrations::rows_before_0005`], not yet migrated, and its audit
/// log as version 4 left it.
async fn unmigrated_0005() -> (TempDir, String, SqliteStore, Vec<ChainedEntry>) {
    let (dir, url) = database_at(4, &migrations::rows_before_0005()).await;
    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    let before = store.audit_log().await.unwrap();
    (dir, url, store, before)
}

async fn migrated_0005() -> (TempDir, SqliteStore) {
    let (dir, _, store, _) = unmigrated_0005().await;
    store.migrate().await.unwrap();
    (dir, store)
}

async fn raw_sql_on(url: &str) -> RawSql {
    let pool = SqlitePool::connect(url).await.unwrap();
    raw_runner(move |sql| {
        let pool = pool.clone();
        async move {
            sqlx::raw_sql(sql)
                .execute(&pool)
                .await
                .map(drop)
                .map_err(|e| e.to_string())
        }
    })
}

macro_rules! migration_0005_checks {
    ($($name:ident => $check:path),+ $(,)?) => {
        $(
            #[tokio::test]
            async fn $name() {
                let (_dir, store) = migrated_0005().await;
                $check(&store).await;
            }
        )+
    };
}

migration_0005_checks! {
    migration_0005_creates_one_entry_per_report_and_per_unpaired_reading =>
        migrations::migration_0005_creates_one_entry_per_report_and_per_unpaired_reading,
    migration_0005_pairs_by_adjacent_create_entries => migrations::pairs_by_adjacent_create_entries,
    migration_0005_never_pairs_across_people => migrations::never_pairs_across_people,
    migration_0005_keeps_a_reading_after_a_reading_alone =>
        migrations::keeps_a_reading_after_a_reading_alone,
    migration_0005_pairs_a_report_moved_or_retimed_since =>
        migrations::pairs_a_report_moved_or_retimed_since,
    migration_0005_moves_each_report_note_to_its_entry =>
        migrations::moves_each_report_note_to_its_entry,
    migration_0005_keeps_report_and_reading_ids => migrations::keeps_report_and_reading_ids,
}

#[tokio::test]
async fn migration_0005_keeps_the_audit_chain_valid() {
    let (_dir, _, store, before) = unmigrated_0005().await;
    store.migrate().await.unwrap();
    migrations::keeps_the_audit_chain_valid(&store, &before).await;
}

#[tokio::test]
async fn backfill_writes_one_create_entry_per_log_entry_once() {
    let (_dir, _, store, before) = unmigrated_0005().await;
    store.migrate().await.unwrap();
    migrations::backfill_writes_one_create_entry_per_log_entry_once(&store, &before).await;
}

#[tokio::test]
async fn a_direct_edit_of_any_part_of_a_log_is_caught() {
    let (_dir, url, store, _) = unmigrated_0005().await;
    store.migrate().await.unwrap();
    let raw = raw_sql_on(&url).await;
    migrations::a_direct_edit_of_any_part_of_a_log_is_caught(&store, &raw).await;
}

#[tokio::test]
async fn migrate_twice_adds_no_second_set_of_entries() {
    let (_dir, _, store, before) = unmigrated_0005().await;
    store.migrate().await.unwrap();
    store.migrate().await.unwrap();
    migrations::backfill_writes_one_create_entry_per_log_entry_once(&store, &before).await;
}

#[tokio::test]
async fn a_second_backfill_racing_the_first_changes_nothing() {
    let (_dir, _, store, before) = unmigrated_0005().await;
    let other = store.clone();
    let (first, second) = tokio::join!(store.migrate(), other.migrate());
    first.unwrap();
    second.unwrap();
    migrations::backfill_writes_one_create_entry_per_log_entry_once(&store, &before).await;
}

#[tokio::test]
async fn an_entry_logged_after_the_backfill_is_not_backfilled() {
    let (_dir, url, store, _) = unmigrated_0005().await;
    store.migrate().await.unwrap();
    let raw = raw_sql_on(&url).await;
    migrations::an_entry_logged_after_the_backfill_is_not_backfilled(&store, &raw, || {
        store.migrate()
    })
    .await;
}

#[tokio::test]
async fn an_empty_database_migrates_without_backfill_entries() {
    let (_dir, url) = database_at(
        4,
        "INSERT INTO users (id, username, role, locale, created_at) \
         VALUES (1, 'daniele', 'resident', 'en', 0);",
    )
    .await;
    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    migrations::an_empty_database_migrates_without_backfill_entries(&store).await;
    assert_eq!(store.audit_log().await.unwrap(), []);
}

async fn refused_and_unchanged(extra_row: &'static str) {
    let (_dir, url) = database_at(4, &migrations::rows_before_0005()).await;
    let pool = SqlitePool::connect(&url).await.unwrap();
    sqlx::raw_sql(extra_row).execute(&pool).await.unwrap();
    let counts = || {
        let pool = pool.clone();
        async move {
            let mut counts = Vec::new();
            for sql in [
                "SELECT count(*) FROM smell_reports",
                "SELECT count(*) FROM pm25_readings",
                "SELECT count(*) FROM audit_log",
            ] {
                counts.push(
                    sqlx::query_scalar::<_, i64>(sql)
                        .fetch_one(&pool)
                        .await
                        .unwrap(),
                );
            }
            counts
        }
    };
    let counts_before = counts().await;

    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    let error = store.migrate().await.unwrap_err();
    assert!(
        format!("{error:?}").contains(migrations::NEEDS_A_CREATE_ENTRY),
        "{error:?}"
    );
    let log_entries: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'log_entries'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(log_entries, 0, "nothing was created");
    let owner_columns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pragma_table_info('smell_reports') WHERE name = 'user_id'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(owner_columns, 1, "the reports table was not rebuilt");
    assert_eq!(counts().await, counts_before);
}

#[tokio::test]
async fn migration_0005_refuses_a_report_without_a_create_entry_and_changes_nothing() {
    refused_and_unchanged(migrations::REPORT_WITHOUT_CREATE_ENTRY).await;
}

#[tokio::test]
async fn migration_0005_refuses_a_reading_without_a_create_entry_and_changes_nothing() {
    refused_and_unchanged(migrations::READING_WITHOUT_CREATE_ENTRY).await;
}

#[tokio::test]
async fn after_0005_the_children_refer_to_log_entries() {
    let (_dir, url) = database();
    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let pool = SqlitePool::connect(&url).await.unwrap();
    for (table, target) in [
        ("smell_reports", "log_entries"),
        ("pm25_readings", "log_entries"),
        ("smell_report_rooms", "smell_reports"),
        ("smell_report_symptoms", "smell_reports"),
    ] {
        let targets: Vec<String> =
            sqlx::query_scalar("SELECT \"table\" FROM pragma_foreign_key_list($1)")
                .bind(table)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(targets.contains(&target.to_owned()), "{table}: {targets:?}");
    }
}
