//! Smell reports, with their rooms and symptoms in link tables. Deleted softly.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{NewReport, ReportValues, SmellReport};
use smoke_domain::repos::{Conflict, ReportRepo, ReportWrites, StoreError, StoreResult};
use smoke_domain::types::{EventId, Intensity, ReportId, RoomId, SymptomId, UserId};
use sqlx::SqliteConnection;

use crate::convert::{corrupt, instant, micros, opt_instant};
use crate::error::IntoStore;
use crate::{SqliteStore, SqliteTx};

struct ReportRow {
    id: i64,
    event_id: i64,
    user_id: i64,
    observed_at: i64,
    recorded_at: i64,
    intensity: i64,
    note: Option<String>,
    deleted_at: Option<i64>,
}

async fn load(conn: &mut SqliteConnection, row: ReportRow) -> StoreResult<SmellReport> {
    let rooms = sqlx::query_scalar!(
        "SELECT room_id FROM smell_report_rooms WHERE smell_report_id = $1 ORDER BY room_id",
        row.id,
    )
    .fetch_all(&mut *conn)
    .await
    .db()?;
    let symptoms = sqlx::query_scalar!(
        "SELECT symptom_id FROM smell_report_symptoms WHERE smell_report_id = $1 ORDER BY symptom_id",
        row.id,
    )
    .fetch_all(&mut *conn)
    .await
    .db()?;
    assemble(row, rooms, symptoms)
}

/// A report from its row and the ids in its link tables.
fn assemble(row: ReportRow, rooms: Vec<i64>, symptoms: Vec<i64>) -> StoreResult<SmellReport> {
    let intensity = u8::try_from(row.intensity)
        .map_err(corrupt)
        .and_then(|value| Intensity::new(value).map_err(corrupt))?;
    let values = ReportValues::new(
        intensity,
        rooms.into_iter().map(RoomId).collect(),
        symptoms.into_iter().map(SymptomId).collect(),
        row.note.as_deref(),
    )
    .map_err(corrupt)?;
    if row.note.as_deref() != values.note() {
        return Err(corrupt("note is not stored in canonical form"));
    }
    Ok(SmellReport {
        id: ReportId(row.id),
        event_id: EventId(row.event_id),
        user_id: UserId(row.user_id),
        observed_at: instant(row.observed_at)?,
        recorded_at: instant(row.recorded_at)?,
        values,
        deleted_at: opt_instant(row.deleted_at)?,
    })
}

/// Groups `(report id, linked id)` pairs by report.
fn group(links: impl IntoIterator<Item = (i64, i64)>) -> HashMap<i64, Vec<i64>> {
    let mut grouped: HashMap<i64, Vec<i64>> = HashMap::new();
    for (report, linked) in links {
        grouped.entry(report).or_default().push(linked);
    }
    grouped
}

async fn load_all(
    conn: &mut SqliteConnection,
    rows: Vec<ReportRow>,
) -> StoreResult<Vec<SmellReport>> {
    let mut reports = Vec::with_capacity(rows.len());
    for row in rows {
        reports.push(load(&mut *conn, row).await?);
    }
    Ok(reports)
}

async fn fetch(conn: &mut SqliteConnection, id: ReportId) -> StoreResult<Option<SmellReport>> {
    let row = sqlx::query_as!(
        ReportRow,
        "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
         FROM smell_reports WHERE id = $1",
        id.0,
    )
    .fetch_optional(&mut *conn)
    .await
    .db()?;
    match row {
        Some(row) => load(conn, row).await.map(Some),
        None => Ok(None),
    }
}

async fn require(conn: &mut SqliteConnection, id: ReportId) -> StoreResult<SmellReport> {
    fetch(conn, id).await?.ok_or(StoreError::NotFound {
        entity: AuditEntity::SmellReport,
        id: id.0,
    })
}

/// Deleted reports cannot be changed.
async fn require_live(conn: &mut SqliteConnection, id: ReportId) -> StoreResult<SmellReport> {
    let report = require(conn, id).await?;
    if report.is_deleted() {
        Err(Conflict::AlreadyDeleted.into())
    } else {
        Ok(report)
    }
}

async fn insert_links(
    conn: &mut SqliteConnection,
    id: i64,
    values: &ReportValues,
) -> StoreResult<()> {
    for room in values.rooms() {
        sqlx::query!(
            "INSERT INTO smell_report_rooms (smell_report_id, room_id) VALUES ($1, $2)",
            id,
            room.0,
        )
        .execute(&mut *conn)
        .await
        .db()?;
    }
    for symptom in values.symptoms() {
        sqlx::query!(
            "INSERT INTO smell_report_symptoms (smell_report_id, symptom_id) VALUES ($1, $2)",
            id,
            symptom.0,
        )
        .execute(&mut *conn)
        .await
        .db()?;
    }
    Ok(())
}

async fn delete_links(conn: &mut SqliteConnection, id: i64) -> StoreResult<()> {
    sqlx::query!(
        "DELETE FROM smell_report_rooms WHERE smell_report_id = $1",
        id
    )
    .execute(&mut *conn)
    .await
    .db()?;
    sqlx::query!(
        "DELETE FROM smell_report_symptoms WHERE smell_report_id = $1",
        id
    )
    .execute(&mut *conn)
    .await
    .db()?;
    Ok(())
}

async fn audited(
    tx: &mut SqliteTx,
    action: AuditAction,
    before: SmellReport,
) -> StoreResult<SmellReport> {
    let after = require(tx.conn(), before.id).await?;
    tx.record(action, Some(&before), &after).await?;
    Ok(after)
}

impl ReportRepo for SqliteStore {
    async fn report(&self, id: ReportId) -> StoreResult<Option<SmellReport>> {
        fetch(&mut *self.read().await?, id).await
    }

    async fn reports_for_event(&self, event: EventId) -> StoreResult<Vec<SmellReport>> {
        let mut conn = self.read().await?;
        let rows = sqlx::query_as!(
            ReportRow,
            "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
             FROM smell_reports WHERE event_id = $1 AND deleted_at IS NULL \
             ORDER BY observed_at, id",
            event.0,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        load_all(&mut conn, rows).await
    }

    /// Three queries whatever the number of reports: the rows, then all their room links and
    /// all their symptom links.
    async fn reports_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> StoreResult<Vec<SmellReport>> {
        let (from, to) = (micros(from), micros(to));
        let mut conn = self.read().await?;
        let rows = sqlx::query_as!(
            ReportRow,
            "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
             FROM smell_reports WHERE deleted_at IS NULL AND observed_at >= $1 AND observed_at < $2 \
             ORDER BY observed_at, id",
            from,
            to,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let rooms = sqlx::query!(
            "SELECT l.smell_report_id, l.room_id FROM smell_report_rooms l \
             JOIN smell_reports r ON r.id = l.smell_report_id \
             WHERE r.deleted_at IS NULL AND r.observed_at >= $1 AND r.observed_at < $2 \
             ORDER BY l.smell_report_id, l.room_id",
            from,
            to,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let symptoms = sqlx::query!(
            "SELECT l.smell_report_id, l.symptom_id FROM smell_report_symptoms l \
             JOIN smell_reports r ON r.id = l.smell_report_id \
             WHERE r.deleted_at IS NULL AND r.observed_at >= $1 AND r.observed_at < $2 \
             ORDER BY l.smell_report_id, l.symptom_id",
            from,
            to,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let mut rooms = group(rooms.into_iter().map(|l| (l.smell_report_id, l.room_id)));
        let mut symptoms = group(
            symptoms
                .into_iter()
                .map(|l| (l.smell_report_id, l.symptom_id)),
        );
        rows.into_iter()
            .map(|row| {
                let id = row.id;
                assemble(
                    row,
                    rooms.remove(&id).unwrap_or_default(),
                    symptoms.remove(&id).unwrap_or_default(),
                )
            })
            .collect()
    }
}

impl ReportWrites for SqliteTx {
    async fn report(&mut self, id: ReportId) -> StoreResult<Option<SmellReport>> {
        fetch(self.conn(), id).await
    }

    async fn create_report(&mut self, new: NewReport) -> StoreResult<SmellReport> {
        let now = micros(self.now());
        let id = sqlx::query_scalar!(
            "INSERT INTO smell_reports (event_id, user_id, observed_at, recorded_at, intensity, note) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id AS \"id!\"",
            new.event_id.0,
            new.user_id.0,
            micros(new.observed_at),
            now,
            i64::from(new.values.intensity().get()),
            new.values.note(),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        insert_links(self.conn(), id, &new.values).await?;
        let report = require(self.conn(), ReportId(id)).await?;
        self.record(AuditAction::Create, None, &report).await?;
        Ok(report)
    }

    async fn correct_report(
        &mut self,
        id: ReportId,
        observed_at: DateTime<Utc>,
        values: ReportValues,
    ) -> StoreResult<SmellReport> {
        let before = require_live(self.conn(), id).await?;
        sqlx::query!(
            "UPDATE smell_reports SET observed_at = $1, intensity = $2, note = $3 WHERE id = $4",
            micros(observed_at),
            i64::from(values.intensity().get()),
            values.note(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        delete_links(self.conn(), id.0).await?;
        insert_links(self.conn(), id.0, &values).await?;
        audited(self, AuditAction::Update, before).await
    }

    async fn move_report(&mut self, id: ReportId, to: EventId) -> StoreResult<SmellReport> {
        let before = require_live(self.conn(), id).await?;
        if before.event_id == to {
            return Ok(before);
        }
        sqlx::query!(
            "UPDATE smell_reports SET event_id = $1 WHERE id = $2",
            to.0,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Move, before).await
    }

    async fn delete_report(&mut self, id: ReportId) -> StoreResult<SmellReport> {
        let before = require_live(self.conn(), id).await?;
        let now = micros(self.now());
        sqlx::query!(
            "UPDATE smell_reports SET deleted_at = $1 WHERE id = $2",
            now,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Delete, before).await
    }
}

pub(crate) async fn fetch_all(conn: &mut SqliteConnection) -> StoreResult<Vec<SmellReport>> {
    let rows = sqlx::query_as!(
        ReportRow,
        "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
         FROM smell_reports ORDER BY id"
    )
    .fetch_all(&mut *conn)
    .await
    .db()?;
    load_all(conn, rows).await
}
