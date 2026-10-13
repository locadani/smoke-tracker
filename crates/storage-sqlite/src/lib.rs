//! SQLite implementation of the `smoke-domain` storage traits.
//!
//! SQLite allows one writer at a time, so all write transactions share a single connection: they
//! run one after another and every audit entry links to the one before it. Reads use a separate
//! pool of read-only connections, which WAL mode lets run alongside the writer.

mod audit;
mod convert;
mod error;
mod lists;
mod tx;
mod users;

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use smoke_domain::clock::Clock;
use smoke_domain::repos::{StoreError, StoreResult, Transactional};
use smoke_domain::types::UserId;
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Sqlite, SqlitePool};

use crate::error::IntoStore;
pub use crate::tx::SqliteTx;

#[derive(Clone)]
pub struct SqliteStore {
    writer: SqlitePool,
    reader: SqlitePool,
    clock: Arc<dyn Clock>,
}

impl SqliteStore {
    /// Opens (and creates if missing) the database at `url`, e.g. `sqlite://data/smoke.db`.
    pub async fn connect(url: &str, clock: Arc<dyn Clock>) -> StoreResult<Self> {
        let base = SqliteConnectOptions::from_str(url)
            .db()?
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let writer = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                base.clone()
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Normal),
            )
            .await
            .db()?;
        let reader = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_lazy_with(base.read_only(true));
        Ok(Self {
            writer,
            reader,
            clock,
        })
    }

    /// Applies the embedded migrations. Safe to run on every start.
    pub async fn migrate(&self) -> StoreResult<()> {
        sqlx::migrate!("./migrations")
            .run(&self.writer)
            .await
            .map_err(|e| StoreError::Backend(Box::new(e)))
    }

    async fn read(&self) -> StoreResult<PoolConnection<Sqlite>> {
        self.reader.acquire().await.db()
    }
}

impl Transactional for SqliteStore {
    type Tx = SqliteTx;

    async fn begin(&self, actor: Option<UserId>) -> StoreResult<SqliteTx> {
        let tx = self.writer.begin().await.db()?;
        Ok(SqliteTx::new(tx, actor, self.clock.clone()))
    }
}
