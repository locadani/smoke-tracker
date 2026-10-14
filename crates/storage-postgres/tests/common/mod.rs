//! A fresh database per test on the server named by `TEST_POSTGRES_URL`, dropped again when the
//! test is done with it.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A database that lives as long as this value. Dropping it drops the database and closes any
/// connection still open to it, so the server does not keep one database per test ever run.
pub struct TestDatabase {
    pub url: String,
    admin_url: String,
    name: String,
}

impl Drop for TestDatabase {
    /// Runs on a thread with its own runtime: the test's runtime may be the one dropping this
    /// value and cannot wait on itself. A failure is reported, never a panic inside `drop`.
    fn drop(&mut self) {
        let admin_url = self.admin_url.clone();
        let sql = format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name);
        let dropped = std::thread::spawn(move || -> Result<(), String> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            runtime.block_on(async {
                let mut admin = PgConnection::connect(&admin_url)
                    .await
                    .map_err(|e| e.to_string())?;
                sqlx::raw_sql(AssertSqlSafe(sql))
                    .execute(&mut admin)
                    .await
                    .map_err(|e| e.to_string())?;
                admin.close().await.map_err(|e| e.to_string())
            })
        })
        .join()
        .unwrap_or_else(|_| Err("the thread dropping it panicked".to_owned()));
        if let Err(error) = dropped {
            eprintln!("could not drop test database {}: {error}", self.name);
        }
    }
}

/// A new, empty database, or `None` outside CI when no server is configured.
/// `TEST_POSTGRES_URL` must end with a database name (e.g. `/postgres`) and have no query string.
pub async fn fresh_database() -> Option<TestDatabase> {
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
        "smoke_test_{}_{stamp}_{}",
        std::process::id(),
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
    Some(TestDatabase {
        url: format!("{server}/{name}"),
        admin_url,
        name,
    })
}
