mod common;

use std::sync::Arc;

use smoke_domain::clock::FakeClock;
use smoke_storage_contract::fixture::{Fixture, raw_runner, start};
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
    let drop_guard = "DROP TRIGGER audit_log_append_only ON audit_log";
    Some(Fixture::new(store, clock, raw, drop_guard, ()))
}

smoke_storage_contract::contract_tests!(fixture);

#[test]
fn the_postgres_store_is_a_complete_store() {
    fn assert_store<S: smoke_domain::repos::Store>() {}
    assert_store::<PostgresStore>();
}
