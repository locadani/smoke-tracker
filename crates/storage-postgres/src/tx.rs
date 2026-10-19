//! The write transaction, and the audit entry every write appends to it.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;
use smoke_domain::audit::{AuditAction, AuditRecord, ChainedEntry, GENESIS_HASH};
use smoke_domain::clock::Clock;
use smoke_domain::records::Audited;
use smoke_domain::repos::{Commit, StoreResult};
use smoke_domain::types::UserId;
use sqlx::{PgConnection, Postgres, Transaction};

use crate::error::IntoStore;

pub struct PostgresTx {
    tx: Transaction<'static, Postgres>,
    actor: Option<UserId>,
    now: DateTime<Utc>,
}

impl PostgresTx {
    /// Takes the transaction's instant from the clock, once: every row and audit entry the
    /// transaction writes carries it. `begin` calls this after it holds the writer lock.
    pub(crate) fn new(
        tx: Transaction<'static, Postgres>,
        actor: Option<UserId>,
        clock: &Arc<dyn Clock>,
    ) -> Self {
        Self {
            tx,
            actor,
            now: clock.now(),
        }
    }

    pub(crate) fn conn(&mut self) -> &mut PgConnection {
        &mut self.tx
    }

    pub(crate) fn now(&self) -> DateTime<Utc> {
        self.now
    }

    /// Appends the audit entry for one write, linked to the latest entry. An `update` that
    /// changed nothing is not recorded.
    pub(crate) async fn record<T: Audited>(
        &mut self,
        action: AuditAction,
        before: Option<&T>,
        after: &T,
    ) -> StoreResult<()> {
        let before = before.map(Audited::snapshot);
        let after_value = after.snapshot();
        if action == AuditAction::Update && before.as_ref() == Some(&after_value) {
            return Ok(());
        }
        let head = sqlx::query!("SELECT id, hash FROM audit_log ORDER BY id DESC LIMIT 1")
            .fetch_optional(self.conn())
            .await
            .db()?;
        let (id, prev_hash) = head.map_or((1, GENESIS_HASH.to_owned()), |h| (h.id + 1, h.hash));
        let entry = ChainedEntry::seal(
            AuditRecord {
                id,
                at_micros: self.now().timestamp_micros(),
                actor_id: self.actor,
                entity: T::ENTITY,
                entity_id: after.audit_id(),
                action,
                before,
                after: Some(after_value),
            },
            &prev_hash,
        );
        let record = &entry.record;
        sqlx::query!(
            "INSERT INTO audit_log \
             (id, at, actor_id, entity, entity_id, action, before_json, after_json, prev_hash, hash) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            record.id,
            record.at_micros,
            record.actor_id.map(|a| a.0),
            record.entity.as_str(),
            record.entity_id,
            record.action.as_str(),
            record.before.as_ref().map(Value::to_string),
            record.after.as_ref().map(Value::to_string),
            entry.prev_hash,
            entry.hash,
        )
        .execute(&mut *self.tx)
        .await
        .db()?;
        Ok(())
    }
}

impl Commit for PostgresTx {
    async fn commit(self) -> StoreResult<()> {
        self.tx.commit().await.db()
    }
}
