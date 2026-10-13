//! When each background job last ran. Bookkeeping, not evidence: not audited.

use chrono::{DateTime, Utc};
use smoke_domain::repos::{JobRepo, JobWrites, StoreResult};

use crate::convert::{micros, opt_instant};
use crate::error::IntoStore;
use crate::{PostgresStore, PostgresTx};

impl JobRepo for PostgresStore {
    async fn job_last_run(&self, name: &str) -> StoreResult<Option<DateTime<Utc>>> {
        let last = sqlx::query_scalar!("SELECT last_run_at FROM jobs WHERE name = $1", name)
            .fetch_optional(&mut *self.read().await?)
            .await
            .db()?;
        opt_instant(last)
    }
}

impl JobWrites for PostgresTx {
    async fn record_job_run(&mut self, name: &str, at: DateTime<Utc>) -> StoreResult<()> {
        sqlx::query!(
            "INSERT INTO jobs (name, last_run_at) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET last_run_at = excluded.last_run_at",
            name,
            micros(at),
        )
        .execute(self.conn())
        .await
        .db()?;
        Ok(())
    }
}
