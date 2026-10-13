//! Reading the audit log.

use serde_json::Value;
use smoke_domain::audit::{AuditEntity, AuditRecord, ChainedEntry};
use smoke_domain::repos::{AuditRepo, StoreResult};
use smoke_domain::types::UserId;
use sqlx::SqliteConnection;

use crate::SqliteStore;
use crate::convert::{corrupt, parse};
use crate::error::IntoStore;

struct AuditRow {
    id: i64,
    at: i64,
    actor_id: Option<i64>,
    entity: String,
    entity_id: i64,
    action: String,
    before_json: Option<String>,
    after_json: Option<String>,
    prev_hash: String,
    hash: String,
}

impl AuditRow {
    fn into_entry(self) -> StoreResult<ChainedEntry> {
        Ok(ChainedEntry {
            record: AuditRecord {
                id: self.id,
                at_micros: self.at,
                actor_id: self.actor_id.map(UserId),
                entity: parse(&self.entity)?,
                entity_id: self.entity_id,
                action: parse(&self.action)?,
                before: json(self.before_json)?,
                after: json(self.after_json)?,
            },
            prev_hash: self.prev_hash,
            hash: self.hash,
        })
    }
}

fn json(text: Option<String>) -> StoreResult<Option<Value>> {
    text.map(|t| serde_json::from_str(&t).map_err(corrupt))
        .transpose()
}

pub(crate) async fn fetch_entries(conn: &mut SqliteConnection) -> StoreResult<Vec<ChainedEntry>> {
    sqlx::query_as!(
        AuditRow,
        "SELECT id, at, actor_id, entity, entity_id, action, before_json, after_json, prev_hash, hash \
         FROM audit_log ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(AuditRow::into_entry)
    .collect()
}

impl AuditRepo for SqliteStore {
    async fn audit_log(&self) -> StoreResult<Vec<ChainedEntry>> {
        fetch_entries(&mut *self.read().await?).await
    }

    async fn audit_history(
        &self,
        entity: AuditEntity,
        entity_id: i64,
    ) -> StoreResult<Vec<ChainedEntry>> {
        sqlx::query_as!(
            AuditRow,
            "SELECT id, at, actor_id, entity, entity_id, action, before_json, after_json, prev_hash, hash \
             FROM audit_log WHERE entity = $1 AND entity_id = $2 ORDER BY id",
            entity.as_str(),
            entity_id,
        )
        .fetch_all(&mut *self.read().await?)
        .await
        .db()?
        .into_iter()
        .map(AuditRow::into_entry)
        .collect()
    }
}
