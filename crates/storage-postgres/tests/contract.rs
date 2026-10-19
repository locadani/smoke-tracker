mod common;

use std::sync::Arc;

use smoke_domain::audit::ChainedEntry;
use smoke_domain::clock::{FakeClock, SystemClock};
use smoke_domain::repos::AuditRepo;
use smoke_storage_contract::fixture::{Fixture, RawSql, raw_runner, start};
use smoke_storage_contract::migrations;
use smoke_storage_postgres::PostgresStore;
use sqlx::AssertSqlSafe;
use sqlx::PgPool;

async fn fixture() -> Option<Fixture<PostgresStore>> {
    let database = common::fresh_database().await?;
    let clock = Arc::new(FakeClock::new(start()));
    let store = PostgresStore::connect(&database.url, clock.clone())
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");
    let pool = PgPool::connect(&database.url)
        .await
        .expect("raw connection");
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
    let drop_guard = "DROP TRIGGER audit_log_append_only ON audit_log; \
         DROP TRIGGER audit_log_no_truncate ON audit_log;";
    Some(Fixture::new(store, clock, raw, drop_guard, database))
}

smoke_storage_contract::contract_tests!(fixture);

#[test]
fn the_postgres_store_is_a_complete_store() {
    fn assert_store<S: smoke_domain::repos::Store>() {}
    assert_store::<PostgresStore>();
}

#[tokio::test]
async fn a_test_database_is_dropped_with_its_handle() {
    let Some(database) = common::fresh_database().await else {
        return;
    };
    let still_connected = PgPool::connect(&database.url).await.unwrap();
    let (_, name) = database.url.rsplit_once('/').unwrap();
    let name = name.to_owned();
    drop(database);
    let admin = PgPool::connect(&std::env::var("TEST_POSTGRES_URL").unwrap())
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_database WHERE datname = $1")
        .bind(&name)
        .fetch_one(&admin)
        .await
        .unwrap();
    assert_eq!(left, 0, "{name} is still on the server");
    drop(still_connected);
}

/// A fresh database migrated up to `version`, with `rows` written by hand. The database lives
/// as long as the returned handle.
async fn database_at(version: i64, rows: &str) -> Option<(common::TestDatabase, PgPool)> {
    let database = common::fresh_database().await?;
    let pool = PgPool::connect(&database.url).await.unwrap();
    sqlx::migrate!("./migrations")
        .run_to(version, &pool)
        .await
        .unwrap();
    sqlx::raw_sql(AssertSqlSafe(rows.to_owned()))
        .execute(&pool)
        .await
        .unwrap();
    Some((database, pool))
}

#[tokio::test]
async fn migration_0003_gives_each_room_its_reports_intensity() {
    let Some((database, _pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
        return;
    };
    let store = PostgresStore::connect(&database.url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    migrations::each_room_took_its_reports_intensity(&store).await;
}

#[tokio::test]
async fn migration_0003_refuses_a_report_without_rooms_and_changes_nothing() {
    let Some((database, pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
        return;
    };
    sqlx::raw_sql(migrations::REPORT_WITHOUT_ROOMS)
        .execute(&pool)
        .await
        .unwrap();
    let store = PostgresStore::connect(&database.url, Arc::new(SystemClock))
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

/// A version-4 database with [`migrations::rows_before_0005`], not yet migrated, and its audit
/// log as version 4 left it.
async fn unmigrated_0005() -> Option<(common::TestDatabase, PostgresStore, Vec<ChainedEntry>)> {
    let (database, pool) = database_at(4, &migrations::rows_before_0005()).await?;
    sqlx::raw_sql(migrations::POSTGRES_SEQUENCES_AFTER_0005_ROWS)
        .execute(&pool)
        .await
        .unwrap();
    let store = PostgresStore::connect(&database.url, Arc::new(SystemClock))
        .await
        .unwrap();
    let before = store.audit_log().await.unwrap();
    Some((database, store, before))
}

async fn migrated_0005() -> Option<(common::TestDatabase, PostgresStore)> {
    let (database, store, _) = unmigrated_0005().await?;
    store.migrate().await.unwrap();
    Some((database, store))
}

fn raw_sql_on(pool: PgPool) -> RawSql {
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
                let Some((_database, store)) = migrated_0005().await else {
                    return;
                };
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
    let Some((_database, store, before)) = unmigrated_0005().await else {
        return;
    };
    store.migrate().await.unwrap();
    migrations::keeps_the_audit_chain_valid(&store, &before).await;
}

#[tokio::test]
async fn backfill_writes_one_create_entry_per_log_entry_once() {
    let Some((_database, store, before)) = unmigrated_0005().await else {
        return;
    };
    store.migrate().await.unwrap();
    migrations::backfill_writes_one_create_entry_per_log_entry_once(&store, &before).await;
}

#[tokio::test]
async fn a_direct_edit_of_any_part_of_a_log_is_caught() {
    let Some((database, store)) = migrated_0005().await else {
        return;
    };
    let raw = raw_sql_on(PgPool::connect(&database.url).await.unwrap());
    migrations::a_direct_edit_of_any_part_of_a_log_is_caught(&store, &raw).await;
}

#[tokio::test]
async fn migrate_twice_adds_no_second_set_of_entries() {
    let Some((_database, store, before)) = unmigrated_0005().await else {
        return;
    };
    store.migrate().await.unwrap();
    store.migrate().await.unwrap();
    migrations::backfill_writes_one_create_entry_per_log_entry_once(&store, &before).await;
}

#[tokio::test]
async fn a_second_backfill_racing_the_first_changes_nothing() {
    let Some((database, store, before)) = unmigrated_0005().await else {
        return;
    };
    let other = PostgresStore::connect(&database.url, Arc::new(SystemClock))
        .await
        .unwrap();
    let (first, second) = tokio::join!(store.migrate(), other.migrate());
    first.unwrap();
    second.unwrap();
    migrations::backfill_writes_one_create_entry_per_log_entry_once(&store, &before).await;
}

#[tokio::test]
async fn an_entry_logged_after_the_backfill_is_not_backfilled() {
    let Some((database, store)) = migrated_0005().await else {
        return;
    };
    let raw = raw_sql_on(PgPool::connect(&database.url).await.unwrap());
    migrations::an_entry_logged_after_the_backfill_is_not_backfilled(&store, &raw, || {
        store.migrate()
    })
    .await;
}

#[tokio::test]
async fn an_empty_database_migrates_without_backfill_entries() {
    let Some((database, pool)) = database_at(
        4,
        "INSERT INTO users (id, username, role, locale, created_at) \
         VALUES (1, 'daniele', 'resident', 'en', 0);",
    )
    .await
    else {
        return;
    };
    drop(pool);
    let store = PostgresStore::connect(&database.url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    migrations::an_empty_database_migrates_without_backfill_entries(&store).await;
    assert_eq!(store.audit_log().await.unwrap(), []);
}

async fn refused_and_unchanged(extra_row: &'static str) {
    let Some((database, pool)) = database_at(4, &migrations::rows_before_0005()).await else {
        return;
    };
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

    let store = PostgresStore::connect(&database.url, Arc::new(SystemClock))
        .await
        .unwrap();
    let error = store.migrate().await.unwrap_err();
    assert!(
        format!("{error:?}").contains(migrations::NEEDS_A_CREATE_ENTRY),
        "{error:?}"
    );
    let log_entries: bool = sqlx::query_scalar("SELECT to_regclass('public.log_entries') IS NULL")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(log_entries, "nothing was created");
    let owner_columns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns \
         WHERE table_schema = current_schema() AND table_name = 'smell_reports' \
           AND column_name = 'user_id'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(owner_columns, 1, "the reports table was not changed");
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

const PREFLIGHT: &str = include_str!("../../../scripts/neon-preflight-0005.sql");

#[derive(sqlx::FromRow, Debug)]
struct Preflight {
    must_be_4_last_migration: i64,
    must_be_0_failed_migrations: i64,
    must_be_true_no_log_entries_table: bool,
    must_be_0_log_entry_audit_rows: i64,
    must_be_0_reports_without_create: i64,
    must_be_0_readings_without_create: i64,
    must_be_0_bad_notes: i64,
    info_pairs_one_entry_each: i64,
    info_reports_alone: i64,
    info_readings_alone: i64,
    info_reports_with_note: i64,
    info_audit_rows: i64,
}

#[tokio::test]
async fn preflight_counts_match_what_0005_creates() {
    let Some((database, pool)) = database_at(4, &migrations::rows_before_0005()).await else {
        return;
    };
    let preflight: Preflight = sqlx::query_as(PREFLIGHT).fetch_one(&pool).await.unwrap();
    assert_eq!(
        (
            preflight.must_be_4_last_migration,
            preflight.must_be_0_failed_migrations,
            preflight.must_be_true_no_log_entries_table,
            preflight.must_be_0_log_entry_audit_rows,
            preflight.must_be_0_reports_without_create,
            preflight.must_be_0_readings_without_create,
            preflight.must_be_0_bad_notes,
        ),
        (4, 0, true, 0, 0, 0, 0),
        "{preflight:?}"
    );

    let store = PostgresStore::connect(&database.url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let count = |sql: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>(sql)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    let both = count(
        "SELECT count(*) FROM smell_reports r JOIN pm25_readings p ON p.log_entry_id = r.log_entry_id",
    )
    .await;
    let reports_alone = count(
        "SELECT count(*) FROM smell_reports r \
         WHERE NOT EXISTS (SELECT 1 FROM pm25_readings p WHERE p.log_entry_id = r.log_entry_id)",
    )
    .await;
    let readings_alone = count(
        "SELECT count(*) FROM pm25_readings p \
         WHERE NOT EXISTS (SELECT 1 FROM smell_reports r WHERE r.log_entry_id = p.log_entry_id)",
    )
    .await;
    let with_note = count("SELECT count(*) FROM log_entries WHERE note IS NOT NULL").await;
    assert_eq!(
        (
            preflight.info_pairs_one_entry_each,
            preflight.info_reports_alone,
            preflight.info_readings_alone,
            preflight.info_reports_with_note,
        ),
        (both, reports_alone, readings_alone, with_note)
    );
    assert_eq!(
        preflight.info_audit_rows,
        count("SELECT count(*) FROM audit_log WHERE entity <> 'log_entry'").await,
        "the migration changes no existing audit row and adds only log_entry rows"
    );
}

#[tokio::test]
async fn the_children_are_tied_to_their_entry_and_the_indexes_are_right() {
    let Some(database) = common::fresh_database().await else {
        return;
    };
    let store = PostgresStore::connect(&database.url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let pool = PgPool::connect(&database.url).await.unwrap();
    for table in ["smell_reports", "pm25_readings"] {
        let not_null: bool = sqlx::query_scalar(
            "SELECT is_nullable = 'NO' FROM information_schema.columns \
             WHERE table_schema = current_schema() AND table_name = $1 \
               AND column_name = 'log_entry_id'",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(not_null, "{table}.log_entry_id is NOT NULL");
        let unique: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_constraint c \
             WHERE c.conrelid = $1::regclass AND c.contype = 'u' \
               AND (SELECT array_agg(a.attname::text) FROM pg_attribute a \
                    WHERE a.attrelid = c.conrelid AND a.attnum = ANY (c.conkey)) = ARRAY['log_entry_id']",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(unique, 1, "{table}.log_entry_id is UNIQUE");
        let references: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_constraint c \
             WHERE c.conrelid = $1::regclass AND c.contype = 'f' \
               AND c.confrelid = 'log_entries'::regclass \
               AND (SELECT array_agg(a.attname::text) FROM pg_attribute a \
                    WHERE a.attrelid = c.conrelid AND a.attnum = ANY (c.conkey)) = ARRAY['log_entry_id'] \
               AND (SELECT array_agg(a.attname::text) FROM pg_attribute a \
                    WHERE a.attrelid = c.confrelid AND a.attnum = ANY (c.confkey)) = ARRAY['id']",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            references, 1,
            "{table}.log_entry_id references log_entries(id)"
        );
    }
    let indexes: Vec<String> = sqlx::query_scalar(
        "SELECT indexname::text FROM pg_indexes WHERE schemaname = current_schema() \
         AND indexname IN ('log_entries_event', 'smell_reports_event', 'pm25_readings_event') \
         ORDER BY indexname",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(indexes, ["log_entries_event"]);
    let columns: String =
        sqlx::query_scalar("SELECT indexdef FROM pg_indexes WHERE indexname = 'log_entries_event'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(columns.contains("(event_id, observed_at)"), "{columns}");
}
