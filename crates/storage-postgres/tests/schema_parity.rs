//! The SQLite and Postgres migrations must produce the same tables and columns (spec §10.1).

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use smoke_domain::clock::SystemClock;
use smoke_storage_postgres::PostgresStore;
use smoke_storage_sqlite::SqliteStore;
use sqlx::{PgPool, SqlitePool};

/// (table, column, type family, nullable)
type Columns = BTreeSet<(String, String, &'static str, bool)>;

fn family(sql_type: &str) -> &'static str {
    match sql_type.to_ascii_lowercase().as_str() {
        "integer" | "bigint" => "integer",
        "text" => "text",
        "real" | "double precision" => "real",
        other => panic!("no parity rule for column type {other:?}"),
    }
}

async fn sqlite_columns() -> Columns {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("parity.db").display());
    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let pool = SqlitePool::connect(&url).await.unwrap();
    let rows: Vec<(String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT m.name, p.name, p.type, p.\"notnull\", p.pk \
         FROM sqlite_master m JOIN pragma_table_info(m.name) p \
         WHERE m.type = 'table' AND m.name NOT LIKE 'sqlite_%' AND m.name <> '_sqlx_migrations'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    rows.into_iter()
        .map(|(table, column, ty, not_null, pk)| {
            (table, column, family(&ty), not_null == 0 && pk == 0)
        })
        .collect()
}

async fn postgres_columns(url: &str) -> Columns {
    let store = PostgresStore::connect(url, Arc::new(SystemClock))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let pool = PgPool::connect(url).await.unwrap();
    let rows: Vec<(String, String, String, bool)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text, data_type::text, is_nullable = 'YES' \
         FROM information_schema.columns \
         WHERE table_schema = current_schema() AND table_name <> '_sqlx_migrations'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    rows.into_iter()
        .map(|(table, column, ty, nullable)| (table, column, family(&ty), nullable))
        .collect()
}

#[tokio::test]
async fn sqlite_and_postgres_schemas_have_the_same_columns() {
    let Some(url) = common::fresh_database().await else {
        return;
    };
    let sqlite = sqlite_columns().await;
    let postgres = postgres_columns(&url).await;
    let only_sqlite: Vec<_> = sqlite.difference(&postgres).collect();
    let only_postgres: Vec<_> = postgres.difference(&sqlite).collect();
    assert!(
        only_sqlite.is_empty() && only_postgres.is_empty(),
        "only in SQLite: {only_sqlite:#?}\nonly in Postgres: {only_postgres:#?}"
    );
    assert_eq!(sqlite.len(), 66, "a table went missing from the comparison");
}
