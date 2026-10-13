//! Events: one smoke incident each. At most one is open at a time.

use chrono::{DateTime, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{Closure, Event, NewEvent};
use smoke_domain::repos::{Conflict, EventRepo, EventWrites, StoreError, StoreResult};
use smoke_domain::types::{EventId, UserId};
use sqlx::PgConnection;

use crate::convert::{corrupt, instant, micros, opt_instant, parse};
use crate::error::IntoStore;
use crate::{PostgresStore, PostgresTx};

struct EventRow {
    id: i64,
    owner_id: i64,
    opened_at: i64,
    closed_at: Option<i64>,
    close_reason: Option<String>,
    auto_close_notice_seen_at: Option<i64>,
    recorded_at: i64,
}

impl EventRow {
    fn into_event(self) -> StoreResult<Event> {
        let closed = match (self.closed_at, self.close_reason) {
            (Some(at), Some(reason)) => Some(Closure {
                at: instant(at)?,
                reason: parse(&reason)?,
            }),
            (None, None) => None,
            _ => {
                return Err(corrupt(format!(
                    "event {}: closed_at and close_reason disagree",
                    self.id
                )));
            }
        };
        Ok(Event {
            id: EventId(self.id),
            owner_id: UserId(self.owner_id),
            opened_at: instant(self.opened_at)?,
            closed,
            auto_close_notice_seen_at: opt_instant(self.auto_close_notice_seen_at)?,
            recorded_at: instant(self.recorded_at)?,
        })
    }
}

async fn fetch(conn: &mut PgConnection, id: EventId) -> StoreResult<Option<Event>> {
    sqlx::query_as!(
        EventRow,
        "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
         FROM events WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .map(EventRow::into_event)
    .transpose()
}

async fn require(conn: &mut PgConnection, id: EventId) -> StoreResult<Event> {
    fetch(conn, id).await?.ok_or(StoreError::NotFound {
        entity: AuditEntity::Event,
        id: id.0,
    })
}

async fn fetch_open(conn: &mut PgConnection) -> StoreResult<Option<Event>> {
    sqlx::query_as!(
        EventRow,
        "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
         FROM events WHERE closed_at IS NULL"
    )
    .fetch_optional(conn)
    .await
    .db()?
    .map(EventRow::into_event)
    .transpose()
}

async fn audited(tx: &mut PostgresTx, action: AuditAction, before: Event) -> StoreResult<Event> {
    let after = require(tx.conn(), before.id).await?;
    tx.record(action, Some(&before), &after).await?;
    Ok(after)
}

impl EventRepo for PostgresStore {
    async fn event(&self, id: EventId) -> StoreResult<Option<Event>> {
        fetch(&mut *self.read().await?, id).await
    }

    async fn open_event(&self) -> StoreResult<Option<Event>> {
        fetch_open(&mut *self.read().await?).await
    }

    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> StoreResult<Vec<Event>> {
        sqlx::query_as!(
            EventRow,
            "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
             FROM events WHERE (closed_at IS NULL OR closed_at >= $1) AND opened_at < $2 \
             ORDER BY opened_at, id",
            micros(from),
            micros(to),
        )
        .fetch_all(&mut *self.read().await?)
        .await
        .db()?
        .into_iter()
        .map(EventRow::into_event)
        .collect()
    }

    async fn unseen_auto_closed(&self) -> StoreResult<Vec<Event>> {
        sqlx::query_as!(
            EventRow,
            "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
             FROM events WHERE close_reason = 'auto_0400' AND auto_close_notice_seen_at IS NULL \
             ORDER BY opened_at, id"
        )
        .fetch_all(&mut *self.read().await?)
        .await
        .db()?
        .into_iter()
        .map(EventRow::into_event)
        .collect()
    }
}

impl EventWrites for PostgresTx {
    async fn open_event(&mut self) -> StoreResult<Option<Event>> {
        fetch_open(self.conn()).await
    }

    async fn event(&mut self, id: EventId) -> StoreResult<Option<Event>> {
        fetch(self.conn(), id).await
    }

    async fn event_at(&mut self, at: DateTime<Utc>) -> StoreResult<Option<Event>> {
        sqlx::query_as!(
            EventRow,
            "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
             FROM events WHERE opened_at <= $1 AND (closed_at IS NULL OR closed_at >= $1) \
             ORDER BY opened_at DESC, id DESC LIMIT 1",
            micros(at),
        )
        .fetch_optional(self.conn())
        .await
        .db()?
        .map(EventRow::into_event)
        .transpose()
    }

    async fn latest_report_at(&mut self, event: EventId) -> StoreResult<Option<DateTime<Utc>>> {
        let latest = sqlx::query_scalar!(
            "SELECT MAX(observed_at) FROM smell_reports WHERE event_id = $1 AND deleted_at IS NULL",
            event.0,
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        opt_instant(latest)
    }

    async fn create_event(&mut self, new: NewEvent) -> StoreResult<Event> {
        let now = micros(self.now());
        let id = sqlx::query_scalar!(
            "INSERT INTO events (owner_id, opened_at, closed_at, close_reason, recorded_at) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id AS \"id!\"",
            new.owner_id.0,
            micros(new.opened_at),
            new.closed.map(|c| micros(c.at)),
            new.closed.map(|c| c.reason.as_str()),
            now,
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let event = require(self.conn(), EventId(id)).await?;
        self.record(AuditAction::Create, None, &event).await?;
        Ok(event)
    }

    async fn close_event(&mut self, id: EventId, closure: Closure) -> StoreResult<Event> {
        let before = require(self.conn(), id).await?;
        if !before.is_open() {
            return Err(Conflict::EventNotOpen.into());
        }
        sqlx::query!(
            "UPDATE events SET closed_at = $1, close_reason = $2 WHERE id = $3",
            micros(closure.at),
            closure.reason.as_str(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Close, before).await
    }

    async fn reopen_event(&mut self, id: EventId) -> StoreResult<Event> {
        let before = require(self.conn(), id).await?;
        if before.is_open() {
            return Err(Conflict::EventNotClosed.into());
        }
        sqlx::query!(
            "UPDATE events SET closed_at = NULL, close_reason = NULL WHERE id = $1",
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Reopen, before).await
    }

    async fn move_event_end(
        &mut self,
        id: EventId,
        closed_at: DateTime<Utc>,
    ) -> StoreResult<Event> {
        let before = require(self.conn(), id).await?;
        if before.is_open() {
            return Err(Conflict::EventNotClosed.into());
        }
        sqlx::query!(
            "UPDATE events SET closed_at = $1 WHERE id = $2",
            micros(closed_at),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Update, before).await
    }

    async fn mark_auto_close_notice_seen(&mut self, id: EventId) -> StoreResult<Event> {
        let before = require(self.conn(), id).await?;
        if before.auto_close_notice_seen_at.is_some() {
            return Ok(before);
        }
        let now = micros(self.now());
        sqlx::query!(
            "UPDATE events SET auto_close_notice_seen_at = $1 WHERE id = $2",
            now,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Update, before).await
    }
}

pub(crate) async fn fetch_all(conn: &mut PgConnection) -> StoreResult<Vec<Event>> {
    sqlx::query_as!(
        EventRow,
        "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
         FROM events ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(EventRow::into_event)
    .collect()
}
