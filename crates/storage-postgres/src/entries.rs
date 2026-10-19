//! Log entries: one press of Log, with its smell report (rooms and symptoms in link tables) and
//! its PM2.5 reading, if it has them. Deleted softly, as a whole.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use smoke_domain::DomainError;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{
    LogEntry, NewEntry, Note, Pm25Reading, ReportValues, RoomLevel, SmellReport,
};
use smoke_domain::repos::{
    Commit, Conflict, EntryRepo, EntryWrites, StoreError, StoreResult, Transactional,
};
use smoke_domain::types::{
    EventId, Level, LogEntryId, Pm25, ReadingId, ReportId, RoomId, SymptomId, UserId,
};
use sqlx::PgConnection;

use crate::convert::{corrupt, instant, micros, opt_instant};
use crate::error::IntoStore;
use crate::{PostgresStore, PostgresTx};

struct EntryRow {
    id: i64,
    user_id: i64,
    event_id: Option<i64>,
    observed_at: i64,
    recorded_at: i64,
    note: Option<String>,
    deleted_at: Option<i64>,
}

/// A report's row and its `(room id, level)` pairs and symptom ids.
struct ReportParts {
    id: i64,
    rooms: Vec<(i64, i64)>,
    symptoms: Vec<i64>,
}

/// A reading's id and value.
struct ReadingParts {
    id: i64,
    value_ugm3: f64,
}

async fn load(conn: &mut PgConnection, row: EntryRow) -> StoreResult<LogEntry> {
    let report = sqlx::query_scalar!(
        "SELECT id FROM smell_reports WHERE log_entry_id = $1",
        row.id
    )
    .fetch_optional(&mut *conn)
    .await
    .db()?;
    let report = match report {
        Some(id) => {
            let rooms = sqlx::query!(
                "SELECT room_id, level FROM smell_report_rooms \
                 WHERE smell_report_id = $1 ORDER BY room_id",
                id,
            )
            .fetch_all(&mut *conn)
            .await
            .db()?
            .into_iter()
            .map(|l| (l.room_id, l.level))
            .collect();
            let symptoms = sqlx::query_scalar!(
                "SELECT symptom_id FROM smell_report_symptoms \
                 WHERE smell_report_id = $1 ORDER BY symptom_id",
                id,
            )
            .fetch_all(&mut *conn)
            .await
            .db()?;
            Some(ReportParts {
                id,
                rooms,
                symptoms,
            })
        }
        None => None,
    };
    let reading = sqlx::query!(
        "SELECT id, value_ugm3 FROM pm25_readings WHERE log_entry_id = $1",
        row.id
    )
    .fetch_optional(&mut *conn)
    .await
    .db()?
    .map(|r| ReadingParts {
        id: r.id,
        value_ugm3: r.value_ugm3,
    });
    assemble(row, report, reading)
}

/// An entry from its row and the parts of its children.
fn assemble(
    row: EntryRow,
    report: Option<ReportParts>,
    reading: Option<ReadingParts>,
) -> StoreResult<LogEntry> {
    let note = Note::parse(row.note.as_deref()).map_err(corrupt)?;
    if note.as_ref().map(Note::as_str) != row.note.as_deref() {
        return Err(corrupt("note is not stored in canonical form"));
    }
    let entry_id = LogEntryId(row.id);
    let report = report
        .map(|parts| {
            let rooms = parts
                .rooms
                .into_iter()
                .map(|(room, level)| {
                    let level = u8::try_from(level).map_err(corrupt)?;
                    Ok(RoomLevel {
                        room: RoomId(room),
                        level: Level::new(level).map_err(corrupt)?,
                    })
                })
                .collect::<StoreResult<_>>()?;
            let values =
                ReportValues::new(rooms, parts.symptoms.into_iter().map(SymptomId).collect())
                    .map_err(corrupt)?;
            Ok::<_, StoreError>(SmellReport {
                id: ReportId(parts.id),
                log_entry_id: entry_id,
                values,
            })
        })
        .transpose()?;
    let reading = reading
        .map(|parts| {
            Ok::<_, StoreError>(Pm25Reading {
                id: ReadingId(parts.id),
                log_entry_id: entry_id,
                value: Pm25::new(parts.value_ugm3).map_err(corrupt)?,
            })
        })
        .transpose()?;
    Ok(LogEntry {
        id: entry_id,
        user_id: UserId(row.user_id),
        event_id: row.event_id.map(EventId),
        observed_at: instant(row.observed_at)?,
        recorded_at: instant(row.recorded_at)?,
        note,
        deleted_at: opt_instant(row.deleted_at)?,
        report,
        reading,
    })
}

/// Groups `(parent id, child)` pairs by parent.
fn group<T>(links: impl IntoIterator<Item = (i64, T)>) -> HashMap<i64, Vec<T>> {
    let mut grouped: HashMap<i64, Vec<T>> = HashMap::new();
    for (parent, child) in links {
        grouped.entry(parent).or_default().push(child);
    }
    grouped
}

/// Entries from their rows and the children of all of them: `reports` are `(report id, entry
/// id)`, `rooms` and `symptoms` are keyed by report id, `readings` by entry id.
fn assemble_all(
    rows: Vec<EntryRow>,
    reports: impl IntoIterator<Item = (i64, i64)>,
    rooms: impl IntoIterator<Item = (i64, (i64, i64))>,
    symptoms: impl IntoIterator<Item = (i64, i64)>,
    readings: impl IntoIterator<Item = (i64, ReadingParts)>,
) -> StoreResult<Vec<LogEntry>> {
    let mut rooms = group(rooms);
    let mut symptoms = group(symptoms);
    let mut reports: HashMap<i64, ReportParts> = reports
        .into_iter()
        .map(|(id, entry)| {
            (
                entry,
                ReportParts {
                    id,
                    rooms: rooms.remove(&id).unwrap_or_default(),
                    symptoms: symptoms.remove(&id).unwrap_or_default(),
                },
            )
        })
        .collect();
    let mut readings: HashMap<i64, ReadingParts> = readings.into_iter().collect();
    rows.into_iter()
        .map(|row| {
            let id = row.id;
            assemble(row, reports.remove(&id), readings.remove(&id))
        })
        .collect()
}

async fn load_all(conn: &mut PgConnection, rows: Vec<EntryRow>) -> StoreResult<Vec<LogEntry>> {
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        entries.push(load(&mut *conn, row).await?);
    }
    Ok(entries)
}

async fn fetch(conn: &mut PgConnection, id: LogEntryId) -> StoreResult<Option<LogEntry>> {
    let row = sqlx::query_as!(
        EntryRow,
        "SELECT id, user_id, event_id, observed_at, recorded_at, note, deleted_at \
         FROM log_entries WHERE id = $1",
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

async fn fetch_of_report(conn: &mut PgConnection, id: ReportId) -> StoreResult<Option<LogEntry>> {
    let row = sqlx::query_as!(
        EntryRow,
        "SELECT e.id, e.user_id, e.event_id, e.observed_at, e.recorded_at, e.note, e.deleted_at \
         FROM log_entries e JOIN smell_reports r ON r.log_entry_id = e.id WHERE r.id = $1",
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

async fn fetch_of_reading(conn: &mut PgConnection, id: ReadingId) -> StoreResult<Option<LogEntry>> {
    let row = sqlx::query_as!(
        EntryRow,
        "SELECT e.id, e.user_id, e.event_id, e.observed_at, e.recorded_at, e.note, e.deleted_at \
         FROM log_entries e JOIN pm25_readings p ON p.log_entry_id = e.id WHERE p.id = $1",
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

async fn require(conn: &mut PgConnection, id: LogEntryId) -> StoreResult<LogEntry> {
    fetch(conn, id).await?.ok_or(StoreError::NotFound {
        entity: AuditEntity::LogEntry,
        id: id.0,
    })
}

/// Deleted entries cannot be changed.
async fn require_live(conn: &mut PgConnection, id: LogEntryId) -> StoreResult<LogEntry> {
    let entry = require(conn, id).await?;
    if entry.is_deleted() {
        Err(Conflict::AlreadyDeleted.into())
    } else {
        Ok(entry)
    }
}

async fn insert_links(conn: &mut PgConnection, id: i64, values: &ReportValues) -> StoreResult<()> {
    for room in values.rooms() {
        sqlx::query!(
            "INSERT INTO smell_report_rooms (smell_report_id, room_id, level) VALUES ($1, $2, $3)",
            id,
            room.room.0,
            i64::from(room.level.get()),
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

async fn delete_links(conn: &mut PgConnection, id: i64) -> StoreResult<()> {
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
    tx: &mut PostgresTx,
    action: AuditAction,
    before: LogEntry,
) -> StoreResult<LogEntry> {
    let after = require(tx.conn(), before.id).await?;
    tx.record(action, Some(&before), &after).await?;
    Ok(after)
}

impl EntryRepo for PostgresStore {
    async fn entry(&self, id: LogEntryId) -> StoreResult<Option<LogEntry>> {
        fetch(&mut *self.read().await?, id).await
    }

    async fn entry_of_report(&self, id: ReportId) -> StoreResult<Option<LogEntry>> {
        fetch_of_report(&mut *self.read().await?, id).await
    }

    async fn entry_of_reading(&self, id: ReadingId) -> StoreResult<Option<LogEntry>> {
        fetch_of_reading(&mut *self.read().await?, id).await
    }

    /// Five queries whatever the number of entries, like [`Self::entries_between`].
    async fn entries_for_event(&self, event: EventId) -> StoreResult<Vec<LogEntry>> {
        let mut conn = self.read().await?;
        let rows = sqlx::query_as!(
            EntryRow,
            "SELECT id, user_id, event_id, observed_at, recorded_at, note, deleted_at \
             FROM log_entries WHERE event_id = $1 AND deleted_at IS NULL \
             ORDER BY observed_at, id",
            event.0,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let reports = sqlx::query!(
            "SELECT r.id, r.log_entry_id FROM smell_reports r \
             JOIN log_entries e ON e.id = r.log_entry_id \
             WHERE e.deleted_at IS NULL AND e.event_id = $1",
            event.0,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let rooms = sqlx::query!(
            "SELECT l.smell_report_id, l.room_id, l.level FROM smell_report_rooms l \
             JOIN smell_reports r ON r.id = l.smell_report_id \
             JOIN log_entries e ON e.id = r.log_entry_id \
             WHERE e.deleted_at IS NULL AND e.event_id = $1 \
             ORDER BY l.smell_report_id, l.room_id",
            event.0,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let symptoms = sqlx::query!(
            "SELECT l.smell_report_id, l.symptom_id FROM smell_report_symptoms l \
             JOIN smell_reports r ON r.id = l.smell_report_id \
             JOIN log_entries e ON e.id = r.log_entry_id \
             WHERE e.deleted_at IS NULL AND e.event_id = $1 \
             ORDER BY l.smell_report_id, l.symptom_id",
            event.0,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let readings = sqlx::query!(
            "SELECT p.id, p.log_entry_id, p.value_ugm3 FROM pm25_readings p \
             JOIN log_entries e ON e.id = p.log_entry_id \
             WHERE e.deleted_at IS NULL AND e.event_id = $1",
            event.0,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        assemble_all(
            rows,
            reports.into_iter().map(|r| (r.id, r.log_entry_id)),
            rooms
                .into_iter()
                .map(|l| (l.smell_report_id, (l.room_id, l.level))),
            symptoms
                .into_iter()
                .map(|l| (l.smell_report_id, l.symptom_id)),
            readings.into_iter().map(|r| {
                (
                    r.log_entry_id,
                    ReadingParts {
                        id: r.id,
                        value_ugm3: r.value_ugm3,
                    },
                )
            }),
        )
    }

    /// Five queries whatever the number of entries: the rows, then all their reports, room
    /// links, symptom links and readings.
    async fn entries_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> StoreResult<Vec<LogEntry>> {
        let (from, to) = (micros(from), micros(to));
        let mut conn = self.read().await?;
        let rows = sqlx::query_as!(
            EntryRow,
            "SELECT id, user_id, event_id, observed_at, recorded_at, note, deleted_at \
             FROM log_entries WHERE deleted_at IS NULL AND observed_at >= $1 AND observed_at < $2 \
             ORDER BY observed_at, id",
            from,
            to,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let reports = sqlx::query!(
            "SELECT r.id, r.log_entry_id FROM smell_reports r \
             JOIN log_entries e ON e.id = r.log_entry_id \
             WHERE e.deleted_at IS NULL AND e.observed_at >= $1 AND e.observed_at < $2",
            from,
            to,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let rooms = sqlx::query!(
            "SELECT l.smell_report_id, l.room_id, l.level FROM smell_report_rooms l \
             JOIN smell_reports r ON r.id = l.smell_report_id \
             JOIN log_entries e ON e.id = r.log_entry_id \
             WHERE e.deleted_at IS NULL AND e.observed_at >= $1 AND e.observed_at < $2 \
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
             JOIN log_entries e ON e.id = r.log_entry_id \
             WHERE e.deleted_at IS NULL AND e.observed_at >= $1 AND e.observed_at < $2 \
             ORDER BY l.smell_report_id, l.symptom_id",
            from,
            to,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        let readings = sqlx::query!(
            "SELECT p.id, p.log_entry_id, p.value_ugm3 FROM pm25_readings p \
             JOIN log_entries e ON e.id = p.log_entry_id \
             WHERE e.deleted_at IS NULL AND e.observed_at >= $1 AND e.observed_at < $2",
            from,
            to,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        assemble_all(
            rows,
            reports.into_iter().map(|r| (r.id, r.log_entry_id)),
            rooms
                .into_iter()
                .map(|l| (l.smell_report_id, (l.room_id, l.level))),
            symptoms
                .into_iter()
                .map(|l| (l.smell_report_id, l.symptom_id)),
            readings.into_iter().map(|r| {
                (
                    r.log_entry_id,
                    ReadingParts {
                        id: r.id,
                        value_ugm3: r.value_ugm3,
                    },
                )
            }),
        )
    }
}

impl EntryWrites for PostgresTx {
    async fn entry(&mut self, id: LogEntryId) -> StoreResult<Option<LogEntry>> {
        fetch(self.conn(), id).await
    }

    async fn entry_of_report(&mut self, id: ReportId) -> StoreResult<Option<LogEntry>> {
        fetch_of_report(self.conn(), id).await
    }

    async fn create_entry(&mut self, new: NewEntry) -> StoreResult<LogEntry> {
        if new.report.is_none() && new.reading.is_none() {
            return Err(DomainError::EntryWithoutChildren.into());
        }
        let now = micros(self.now());
        let id = sqlx::query_scalar!(
            "INSERT INTO log_entries (user_id, event_id, observed_at, recorded_at, note) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id AS \"id!\"",
            new.user_id.0,
            new.event_id.map(|event| event.0),
            micros(new.observed_at),
            now,
            new.note.as_ref().map(Note::as_str),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        if let Some(values) = &new.report {
            let report_id = sqlx::query_scalar!(
                "INSERT INTO smell_reports (log_entry_id) VALUES ($1) RETURNING id AS \"id!\"",
                id,
            )
            .fetch_one(self.conn())
            .await
            .db()?;
            insert_links(self.conn(), report_id, values).await?;
        }
        if let Some(value) = new.reading {
            sqlx::query!(
                "INSERT INTO pm25_readings (log_entry_id, value_ugm3) VALUES ($1, $2)",
                id,
                value.ugm3(),
            )
            .execute(self.conn())
            .await
            .db()?;
        }
        let entry = require(self.conn(), LogEntryId(id)).await?;
        self.record(AuditAction::Create, None, &entry).await?;
        Ok(entry)
    }

    async fn correct_entry(
        &mut self,
        id: LogEntryId,
        observed_at: DateTime<Utc>,
        note: Option<Note>,
        report: Option<ReportValues>,
    ) -> StoreResult<LogEntry> {
        let before = require_live(self.conn(), id).await?;
        let report_id = match (&report, &before.report) {
            (Some(_), None) => return Err(Conflict::RuleViolated.into()),
            (Some(_), Some(stored)) => Some(stored.id.0),
            (None, _) => None,
        };
        sqlx::query!(
            "UPDATE log_entries SET observed_at = $1, note = $2 WHERE id = $3",
            micros(observed_at),
            note.as_ref().map(Note::as_str),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        if let (Some(values), Some(report_id)) = (&report, report_id) {
            delete_links(self.conn(), report_id).await?;
            insert_links(self.conn(), report_id, values).await?;
        }
        audited(self, AuditAction::Update, before).await
    }

    async fn move_entry(&mut self, id: LogEntryId, to: EventId) -> StoreResult<LogEntry> {
        let before = require_live(self.conn(), id).await?;
        if before.event_id == Some(to) {
            return Ok(before);
        }
        sqlx::query!(
            "UPDATE log_entries SET event_id = $1 WHERE id = $2",
            to.0,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Move, before).await
    }

    async fn delete_entry(&mut self, id: LogEntryId) -> StoreResult<LogEntry> {
        let before = require_live(self.conn(), id).await?;
        let now = micros(self.now());
        sqlx::query!(
            "UPDATE log_entries SET deleted_at = $1 WHERE id = $2",
            now,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Delete, before).await
    }
}

pub(crate) async fn fetch_all(conn: &mut PgConnection) -> StoreResult<Vec<LogEntry>> {
    let rows = sqlx::query_as!(
        EntryRow,
        "SELECT id, user_id, event_id, observed_at, recorded_at, note, deleted_at \
         FROM log_entries ORDER BY id"
    )
    .fetch_all(&mut *conn)
    .await
    .db()?;
    load_all(conn, rows).await
}

/// Whether the audit has any `log_entry` entry.
async fn has_log_entry_audit(conn: &mut PgConnection) -> StoreResult<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT id FROM audit_log WHERE entity = $1 LIMIT 1",
        AuditEntity::LogEntry.as_str(),
    )
    .fetch_optional(conn)
    .await
    .db()?
    .is_some())
}

/// Audits the entries migration 0005 created: one `create` entry each, whole-log snapshot, no
/// actor, in one transaction. It runs only while the audit has no `log_entry` entry at all,
/// checked again once the writer lock is held: the table lock makes a second instance wait, and
/// its re-check then finds the entries and returns `Ok`. Should it still fail on the audit id, it
/// rolls back and returns `Ok` once the entries are there.
pub(crate) async fn backfill_log_entry_audit(store: &PostgresStore) -> StoreResult<()> {
    match write_backfill(store).await {
        Err(StoreError::Conflict(Conflict::ChainForked)) => {
            if has_log_entry_audit(&mut *store.read().await?).await? {
                Ok(())
            } else {
                Err(Conflict::ChainForked.into())
            }
        }
        other => other,
    }
}

async fn write_backfill(store: &PostgresStore) -> StoreResult<()> {
    let mut tx = store.begin(None).await?;
    if has_log_entry_audit(tx.conn()).await? {
        return Ok(());
    }
    let entries = fetch_all(tx.conn()).await?;
    if entries.is_empty() {
        return Ok(());
    }
    for entry in &entries {
        tx.record(AuditAction::Create, None, entry).await?;
    }
    tx.commit().await
}
