//! Postgres implementation of the `smoke-domain` storage traits, for a later move to Neon
//! (spec §10.2).
//!
//! Every write transaction first locks `audit_log` against other writers (plain reads are not
//! blocked), so transactions append to the hash chain one after another.

mod audit;
mod convert;
mod entries;
mod error;
mod events;
mod invites;
mod jobs;
mod lists;
mod sessions;
mod tx;
mod users;

use std::sync::Arc;

use smoke_domain::clock::Clock;
use smoke_domain::repos::{StoreError, StoreResult, Transactional};
use smoke_domain::types::UserId;
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres};

use crate::error::IntoStore;
pub use crate::tx::PostgresTx;

#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
    clock: Arc<dyn Clock>,
}

impl PostgresStore {
    /// Connects to `url`, e.g. `postgres://user@host/db?sslmode=require`.
    pub async fn connect(url: &str, clock: Arc<dyn Clock>) -> StoreResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .db()?;
        Ok(Self { pool, clock })
    }

    /// Applies the embedded migrations, then audits the log entries migration 0005 created
    /// (once; see `entries::backfill_log_entry_audit`). Safe to run on every start.
    pub async fn migrate(&self) -> StoreResult<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| StoreError::Backend(Box::new(e)))?;
        entries::backfill_log_entry_audit(self).await
    }

    async fn read(&self) -> StoreResult<PoolConnection<Postgres>> {
        self.pool.acquire().await.db()
    }

    /// A pooled connection for the unaudited single-statement writes (sessions).
    async fn write(&self) -> StoreResult<PoolConnection<Postgres>> {
        self.pool.acquire().await.db()
    }
}

impl Transactional for PostgresStore {
    type Tx = PostgresTx;

    /// Waiting for the writer lock is capped at ten seconds (`lock_timeout`), so a stuck writer
    /// fails the next one instead of hanging it.
    async fn begin(&self, actor: Option<UserId>) -> StoreResult<PostgresTx> {
        let mut tx = self.pool.begin().await.db()?;
        sqlx::query!("SET LOCAL lock_timeout = '10s'")
            .execute(&mut *tx)
            .await
            .db()?;
        sqlx::query!("LOCK TABLE audit_log IN EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await
            .db()?;
        Ok(PostgresTx::new(tx, actor, &self.clock))
    }
}
