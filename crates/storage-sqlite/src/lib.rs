//! SQLite implementation of the `smoke-domain` storage traits.
//!
//! SQLite allows one writer at a time, so all write transactions share a single connection: they
//! run one after another and every audit entry links to the one before it. Reads use a separate
//! pool of read-only connections, which WAL mode lets run alongside the writer.

mod audit;
mod convert;
mod error;
mod events;
mod jobs;
mod lists;
mod readings;
mod reports;
mod sessions;
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
    /// Opens (and creates if missing) the database file at `url`, e.g. `sqlite://data/smoke.db`.
    ///
    /// File URLs only: the writer and the readers are separate connections that must share one
    /// file, so in-memory databases are rejected.
    pub async fn connect(url: &str, clock: Arc<dyn Clock>) -> StoreResult<Self> {
        if url.contains(":memory:") || url.contains("mode=memory") {
            return Err(StoreError::Backend(
                "in-memory SQLite is not supported: the writer and the readers must share one file"
                    .into(),
            ));
        }
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

    /// The writer connection, for the unaudited single-statement writes (sessions).
    async fn write(&self) -> StoreResult<PoolConnection<Sqlite>> {
        self.writer.acquire().await.db()
    }
}

impl Transactional for SqliteStore {
    type Tx = SqliteTx;

    /// Write transactions start with `BEGIN IMMEDIATE`, taking the write lock up front: another
    /// process (e.g. the CLI) writing at the same time makes this wait out the busy timeout
    /// instead of failing later with `SQLITE_BUSY_SNAPSHOT`.
    async fn begin(&self, actor: Option<UserId>) -> StoreResult<SqliteTx> {
        let tx = self.writer.begin_with("BEGIN IMMEDIATE").await.db()?;
        Ok(SqliteTx::new(tx, actor, self.clock.clone()))
    }
}
