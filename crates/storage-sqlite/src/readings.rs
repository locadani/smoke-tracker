//! PM2.5 readings: hand-entered sensor values, optionally linked to an event. Deleted softly.

use chrono::{DateTime, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{NewReading, Pm25Reading};
use smoke_domain::repos::{Conflict, ReadingRepo, ReadingWrites, StoreError, StoreResult};
use smoke_domain::types::{EventId, Pm25, ReadingId, UserId};
use sqlx::SqliteConnection;

use crate::convert::{corrupt, instant, micros, opt_instant};
use crate::error::IntoStore;
use crate::{SqliteStore, SqliteTx};

struct ReadingRow {
    id: i64,
    user_id: i64,
    event_id: Option<i64>,
    observed_at: i64,
    recorded_at: i64,
    value_ugm3: f64,
    deleted_at: Option<i64>,
}

impl ReadingRow {
    fn into_reading(self) -> StoreResult<Pm25Reading> {
        Ok(Pm25Reading {
            id: ReadingId(self.id),
            user_id: UserId(self.user_id),
            event_id: self.event_id.map(EventId),
            observed_at: instant(self.observed_at)?,
            recorded_at: instant(self.recorded_at)?,
            value: Pm25::new(self.value_ugm3).map_err(corrupt)?,
            deleted_at: opt_instant(self.deleted_at)?,
        })
    }
}

async fn fetch(conn: &mut SqliteConnection, id: ReadingId) -> StoreResult<Option<Pm25Reading>> {
    sqlx::query_as!(
        ReadingRow,
        "SELECT id, user_id, event_id, observed_at, recorded_at, value_ugm3, deleted_at \
         FROM pm25_readings WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .map(ReadingRow::into_reading)
    .transpose()
}

async fn require(conn: &mut SqliteConnection, id: ReadingId) -> StoreResult<Pm25Reading> {
    fetch(conn, id).await?.ok_or(StoreError::NotFound {
        entity: AuditEntity::Pm25Reading,
        id: id.0,
    })
}

async fn require_live(conn: &mut SqliteConnection, id: ReadingId) -> StoreResult<Pm25Reading> {
    let reading = require(conn, id).await?;
    if reading.is_deleted() {
        Err(Conflict::AlreadyDeleted.into())
    } else {
        Ok(reading)
    }
}

async fn audited(
    tx: &mut SqliteTx,
    action: AuditAction,
    before: Pm25Reading,
) -> StoreResult<Pm25Reading> {
    let after = require(tx.conn(), before.id).await?;
    tx.record(action, Some(&before), &after).await?;
    Ok(after)
}

impl ReadingRepo for SqliteStore {
    async fn reading(&self, id: ReadingId) -> StoreResult<Option<Pm25Reading>> {
        fetch(&mut *self.read().await?, id).await
    }

    async fn readings_for_event(&self, event: EventId) -> StoreResult<Vec<Pm25Reading>> {
        sqlx::query_as!(
            ReadingRow,
            "SELECT id, user_id, event_id, observed_at, recorded_at, value_ugm3, deleted_at \
             FROM pm25_readings WHERE event_id = $1 AND deleted_at IS NULL \
             ORDER BY observed_at, id",
            event.0,
        )
        .fetch_all(&mut *self.read().await?)
        .await
        .db()?
        .into_iter()
        .map(ReadingRow::into_reading)
        .collect()
    }
}

impl ReadingWrites for SqliteTx {
    async fn create_reading(&mut self, new: NewReading) -> StoreResult<Pm25Reading> {
        let now = micros(self.now());
        let id = sqlx::query_scalar!(
            "INSERT INTO pm25_readings (user_id, event_id, observed_at, recorded_at, value_ugm3) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id AS \"id!\"",
            new.user_id.0,
            new.event_id.map(|e| e.0),
            micros(new.observed_at),
            now,
            new.value.ugm3(),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let reading = require(self.conn(), ReadingId(id)).await?;
        self.record(AuditAction::Create, None, &reading).await?;
        Ok(reading)
    }

    async fn correct_reading(
        &mut self,
        id: ReadingId,
        observed_at: DateTime<Utc>,
        value: Pm25,
    ) -> StoreResult<Pm25Reading> {
        let before = require_live(self.conn(), id).await?;
        sqlx::query!(
            "UPDATE pm25_readings SET observed_at = $1, value_ugm3 = $2 WHERE id = $3",
            micros(observed_at),
            value.ugm3(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Update, before).await
    }

    async fn delete_reading(&mut self, id: ReadingId) -> StoreResult<Pm25Reading> {
        let before = require_live(self.conn(), id).await?;
        let now = micros(self.now());
        sqlx::query!(
            "UPDATE pm25_readings SET deleted_at = $1 WHERE id = $2",
            now,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Delete, before).await
    }
}

pub(crate) async fn fetch_all(conn: &mut SqliteConnection) -> StoreResult<Vec<Pm25Reading>> {
    sqlx::query_as!(
        ReadingRow,
        "SELECT id, user_id, event_id, observed_at, recorded_at, value_ugm3, deleted_at \
         FROM pm25_readings ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(ReadingRow::into_reading)
    .collect()
}
