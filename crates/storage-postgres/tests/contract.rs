mod common;

use std::sync::Arc;

use smoke_domain::clock::{FakeClock, SystemClock};
use smoke_storage_contract::fixture::{Fixture, raw_runner, start};
use smoke_storage_contract::migrations;
use smoke_storage_postgres::PostgresStore;
use sqlx::PgPool;

async fn fixture() -> Option<Fixture<PostgresStore>> {
    let url = common::fresh_database().await?;
    let clock = Arc::new(FakeClock::new(start()));
    let store = PostgresStore::connect(&url, clock.clone())
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");
    let pool = PgPool::connect(&url).await.expect("raw connection");
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
    Some(Fixture::new(store, clock, raw, drop_guard, ()))
}

smoke_storage_contract::contract_tests!(fixture);

#[test]
fn the_postgres_store_is_a_complete_store() {
    fn assert_store<S: smoke_domain::repos::Store>() {}
    assert_store::<PostgresStore>();
}

/// A fresh database migrated up to `version`, with `rows` written by hand.
async fn database_at(version: i64, rows: &'static str) -> Option<(String, PgPool)> {
    let url = common::fresh_database().await?;
    let pool = PgPool::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations")
        .run_to(version, &pool)
        .await
        .unwrap();
    sqlx::raw_sql(rows).execute(&pool).await.unwrap();
    Some((url, pool))
}

#[tokio::test]
async fn migration_0003_gives_each_room_its_reports_intensity() {
    let Some((url, _pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
        return;
    };
    let store = PostgresStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    migrations::each_room_took_its_reports_intensity(&store).await;
}

#[tokio::test]
async fn migration_0003_refuses_a_report_without_rooms_and_changes_nothing() {
    let Some((url, pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
        return;
    };
    sqlx::raw_sql(migrations::REPORT_WITHOUT_ROOMS)
        .execute(&pool)
        .await
        .unwrap();
    let store = PostgresStore::connect(&url, Arc::new(SystemClock))
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
