//! A fresh database per test on the server named by `TEST_POSTGRES_URL`.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::{AssertSqlSafe, PgPool};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// The URL of a new, empty database, or `None` outside CI when no server is configured.
/// `TEST_POSTGRES_URL` must end with a database name (e.g. `/postgres`) and have no query string.
pub async fn fresh_database() -> Option<String> {
    let Ok(admin_url) = std::env::var("TEST_POSTGRES_URL") else {
        assert!(
            std::env::var_os("CI").is_none(),
            "TEST_POSTGRES_URL must be set in CI"
        );
        eprintln!("skipped: set TEST_POSTGRES_URL to run the Postgres tests");
        return None;
    };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_micros();
    let name = format!(
        "smoke_test_{stamp}_{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let admin = PgPool::connect(&admin_url)
        .await
        .expect("connect to TEST_POSTGRES_URL");
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&admin)
        .await
        .expect("create a test database");
    admin.close().await;
    let (server, _) = admin_url
        .rsplit_once('/')
        .expect("TEST_POSTGRES_URL ends with /<database>");
    Some(format!("{server}/{name}"))
}
