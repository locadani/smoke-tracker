//! Data written before a migration, and what each backend must make of it. Backends load the
//! rows into a database migrated up to the version before, then run the remaining migrations.

use crate::fixture::{EntryExt, FIRST_ROOM, RawSql, report_values};
use crate::logbook::rome;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use smoke_domain::audit::verify_chain;
use smoke_domain::audit::{AuditAction, AuditEntity, AuditRecord, ChainedEntry, GENESIS_HASH};
use smoke_domain::history::{Change, HistoryEntry, entry_history};
use smoke_domain::logbook::{ReportCorrection, correct_report};
use smoke_domain::permissions::Actor;
use smoke_domain::records::{Audited, Closure, Event, LogEntry, Note, Room, Symptom, User};
use smoke_domain::repos::{
    AuditRepo, Commit, EntryRepo, SnapshotRepo, StoreError, StoreResult, Transactional, WriteTx,
};
use smoke_domain::types::{
    CloseReason, EventId, Level, ListName, Locale, LogEntryId, Pm25, ReadingId, ReportId, Role,
    RoomArea, RoomId, SymptomId, UserId, Username,
};
use smoke_domain::verify::{SnapshotProblem, verify_store};

/// Rows as the schema of migration 0002 stores them: two rooms and a symptom; report 1 at
/// intensity 6 in both rooms, with the symptom; report 2 at intensity 4 in the second room.
/// Each report has its `create` audit entry, which migration 0005 needs to accept the rows.
pub const ROWS_BEFORE_0003: &str = "\
    INSERT INTO users (id, username, role, locale, created_at) \
        VALUES (1, 'daniele', 'resident', 'en', 0); \
    INSERT INTO rooms (id, name, area) VALUES (1, 'Kitchen', 'inside'), (2, 'Bedroom', 'inside'); \
    INSERT INTO symptoms (id, name) VALUES (1, 'Cough'); \
    INSERT INTO events (id, owner_id, opened_at, recorded_at) VALUES (1, 1, 0, 0); \
    INSERT INTO smell_reports (id, event_id, user_id, observed_at, recorded_at, intensity) \
        VALUES (1, 1, 1, 0, 0, 6), (2, 1, 1, 60000000, 60000000, 4); \
    INSERT INTO smell_report_rooms (smell_report_id, room_id) VALUES (1, 1), (1, 2), (2, 2); \
    INSERT INTO smell_report_symptoms (smell_report_id, symptom_id) VALUES (1, 1); \
    INSERT INTO audit_log (id, at, actor_id, entity, entity_id, action, after_json, prev_hash, hash) \
        VALUES (1, 0, 1, 'smell_report', 1, 'create', '{}', 'prev-1', 'hash-1'), \
               (2, 60000000, 1, 'smell_report', 2, 'create', '{}', 'hash-1', 'hash-2');";

/// A report without rooms, which migration 0003 refuses to convert.
pub const REPORT_WITHOUT_ROOMS: &str = "\
    INSERT INTO smell_reports (id, event_id, user_id, observed_at, recorded_at, intensity) \
        VALUES (3, 1, 1, 120000000, 120000000, 5);";

/// The name the refusal carries in its error message.
pub const NEEDS_A_ROOM: &str = "every_smell_report_needs_a_room";

/// After the migrations, every room of [`ROWS_BEFORE_0003`] has its report's intensity, and the
/// reports keep their event and symptoms.
pub async fn each_room_took_its_reports_intensity<S: EntryRepo>(store: &S) {
    let first = store.entry_of_report(ReportId(1)).await.unwrap().unwrap();
    assert_eq!(first.event_id, Some(EventId(1)));
    assert_eq!(first.report.unwrap().values.symptoms(), [SymptomId(1)]);
    let levels = |id| async move {
        let entry = store.entry_of_report(ReportId(id)).await.unwrap().unwrap();
        entry
            .report
            .unwrap()
            .values
            .rooms()
            .iter()
            .map(|r| (r.room, r.level))
            .collect::<Vec<_>>()
    };
    let level = |value| Level::new(value).unwrap();
    assert_eq!(
        levels(1).await,
        [(RoomId(1), level(6)), (RoomId(2), level(6))]
    );
    assert_eq!(levels(2).await, [(RoomId(2), level(4))]);
}

fn at(spec: &str) -> DateTime<Utc> {
    spec.parse().expect("valid instant")
}

/// The version-4 database written by hand: rows and their audit entries, hashed into a real
/// chain, in the shapes the version-4 store wrote them.
struct Version4 {
    sql: String,
    prev_hash: String,
    next_audit_id: i64,
}

#[derive(Clone)]
struct OldReport {
    id: i64,
    user: i64,
    event: Option<i64>,
    observed: DateTime<Utc>,
    recorded: DateTime<Utc>,
    rooms: Vec<(i64, u8)>,
    symptoms: Vec<i64>,
    note: Option<&'static str>,
}

impl OldReport {
    fn snapshot(&self) -> Value {
        json!({
            "id": self.id,
            "event_id": self.event,
            "user_id": self.user,
            "observed_at": self.observed,
            "recorded_at": self.recorded,
            "rooms": self.rooms.iter().map(|&(room, level)| json!({ "room": room, "level": level })).collect::<Vec<_>>(),
            "symptoms": self.symptoms,
            "note": self.note,
            "deleted_at": null,
        })
    }
}

struct OldReading {
    id: i64,
    user: i64,
    event: Option<i64>,
    observed: DateTime<Utc>,
    recorded: DateTime<Utc>,
    value: f64,
}

impl OldReading {
    fn snapshot(&self) -> Value {
        json!({
            "id": self.id,
            "user_id": self.user,
            "event_id": self.event,
            "observed_at": self.observed,
            "recorded_at": self.recorded,
            "value": self.value,
            "deleted_at": null,
        })
    }
}

fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

fn nullable(value: Option<i64>) -> String {
    value.map_or_else(|| "NULL".to_owned(), |v| v.to_string())
}

impl Version4 {
    fn new() -> Self {
        Self {
            sql: String::new(),
            prev_hash: GENESIS_HASH.to_owned(),
            next_audit_id: 1,
        }
    }

    fn run(&mut self, sql: &str) {
        self.sql.push_str(sql);
        self.sql.push(' ');
    }

    #[allow(clippy::too_many_arguments)]
    fn audit(
        &mut self,
        when: DateTime<Utc>,
        actor: Option<i64>,
        entity: AuditEntity,
        entity_id: i64,
        action: AuditAction,
        before: Option<Value>,
        after: Value,
    ) {
        let entry = ChainedEntry::seal(
            AuditRecord {
                id: self.next_audit_id,
                at_micros: when.timestamp_micros(),
                actor_id: actor.map(UserId),
                entity,
                entity_id,
                action,
                before,
                after: Some(after),
            },
            &self.prev_hash,
        );
        let record = &entry.record;
        let json = |value: &Option<Value>| {
            value
                .as_ref()
                .map_or("NULL".to_owned(), |v| quoted(&v.to_string()))
        };
        self.run(&format!(
            "INSERT INTO audit_log (id, at, actor_id, entity, entity_id, action, before_json, after_json, prev_hash, hash) \
             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {});",
            record.id,
            record.at_micros,
            nullable(actor),
            quoted(record.entity.as_str()),
            record.entity_id,
            quoted(record.action.as_str()),
            json(&record.before),
            json(&record.after),
            quoted(&entry.prev_hash),
            quoted(&entry.hash),
        ));
        self.prev_hash = entry.hash;
        self.next_audit_id += 1;
    }

    fn user(&mut self, id: i64, name: &str) {
        let user = User {
            id: UserId(id),
            username: Username::new(name).unwrap(),
            password_hash: None,
            role: Role::Resident,
            locale: Locale::En,
            expires_at: None,
            created_at: at("2026-10-01T08:00:00Z"),
        };
        self.run(&format!(
            "INSERT INTO users (id, username, role, locale, created_at) VALUES ({id}, {}, 'resident', 'en', {});",
            quoted(name),
            user.created_at.timestamp_micros(),
        ));
        self.audit(
            user.created_at,
            None,
            AuditEntity::User,
            id,
            AuditAction::Create,
            None,
            user.snapshot(),
        );
    }

    fn room(&mut self, id: i64, name: &str) {
        let room = Room {
            id: RoomId(id),
            name: ListName::new(name).unwrap(),
            i18n_key: None,
            area: RoomArea::Inside,
            hidden_at: None,
        };
        self.run(&format!(
            "INSERT INTO rooms (id, name, area) VALUES ({id}, {}, 'inside');",
            quoted(name)
        ));
        self.audit(
            at("2026-10-01T08:01:00Z"),
            None,
            AuditEntity::Room,
            id,
            AuditAction::Create,
            None,
            room.snapshot(),
        );
    }

    fn symptom(&mut self, id: i64, name: &str) {
        let symptom = Symptom {
            id: SymptomId(id),
            name: ListName::new(name).unwrap(),
            i18n_key: None,
            hidden_at: None,
        };
        self.run(&format!(
            "INSERT INTO symptoms (id, name) VALUES ({id}, {});",
            quoted(name)
        ));
        self.audit(
            at("2026-10-01T08:02:00Z"),
            None,
            AuditEntity::Symptom,
            id,
            AuditAction::Create,
            None,
            symptom.snapshot(),
        );
    }

    fn event(&mut self, id: i64, owner: i64, opened: &str, closed: Option<&str>) {
        let opened = at(opened);
        let event = Event {
            id: EventId(id),
            owner_id: UserId(owner),
            opened_at: opened,
            closed: closed.map(|c| Closure {
                at: at(c),
                reason: CloseReason::Manual,
            }),
            auto_close_notice_seen_at: None,
            recorded_at: opened,
        };
        let (closed_at, reason) = match &event.closed {
            Some(c) => (c.at.timestamp_micros().to_string(), "'manual'"),
            None => ("NULL".to_owned(), "NULL"),
        };
        self.run(&format!(
            "INSERT INTO events (id, owner_id, opened_at, closed_at, close_reason, recorded_at) \
             VALUES ({id}, {owner}, {}, {closed_at}, {reason}, {});",
            opened.timestamp_micros(),
            opened.timestamp_micros(),
        ));
        self.audit(
            opened,
            Some(owner),
            AuditEntity::Event,
            id,
            AuditAction::Create,
            None,
            event.snapshot(),
        );
    }

    fn report(&mut self, report: &OldReport) {
        self.run(&format!(
            "INSERT INTO smell_reports (id, event_id, user_id, observed_at, recorded_at, note) \
             VALUES ({}, {}, {}, {}, {}, {});",
            report.id,
            nullable(report.event),
            report.user,
            report.observed.timestamp_micros(),
            report.recorded.timestamp_micros(),
            report.note.map_or("NULL".to_owned(), quoted),
        ));
        for &(room, level) in &report.rooms {
            self.run(&format!(
                "INSERT INTO smell_report_rooms (smell_report_id, room_id, level) VALUES ({}, {room}, {level});",
                report.id
            ));
        }
        for symptom in &report.symptoms {
            self.run(&format!(
                "INSERT INTO smell_report_symptoms (smell_report_id, symptom_id) VALUES ({}, {symptom});",
                report.id
            ));
        }
        self.audit(
            report.recorded,
            Some(report.user),
            AuditEntity::SmellReport,
            report.id,
            AuditAction::Create,
            None,
            report.snapshot(),
        );
    }

    fn reading(&mut self, reading: &OldReading) {
        self.run(&format!(
            "INSERT INTO pm25_readings (id, user_id, event_id, observed_at, recorded_at, value_ugm3) \
             VALUES ({}, {}, {}, {}, {}, {});",
            reading.id,
            reading.user,
            nullable(reading.event),
            reading.observed.timestamp_micros(),
            reading.recorded.timestamp_micros(),
            reading.value,
        ));
        self.audit(
            reading.recorded,
            Some(reading.user),
            AuditEntity::Pm25Reading,
            reading.id,
            AuditAction::Create,
            None,
            reading.snapshot(),
        );
    }

    fn move_report(&mut self, report: &mut OldReport, to: i64, when: DateTime<Utc>) {
        let before = report.snapshot();
        report.event = Some(to);
        self.run(&format!(
            "UPDATE smell_reports SET event_id = {to} WHERE id = {};",
            report.id
        ));
        self.audit(
            when,
            Some(report.user),
            AuditEntity::SmellReport,
            report.id,
            AuditAction::Move,
            Some(before),
            report.snapshot(),
        );
    }

    fn retime_report(
        &mut self,
        report: &mut OldReport,
        observed: DateTime<Utc>,
        when: DateTime<Utc>,
    ) {
        let before = report.snapshot();
        report.observed = observed;
        self.run(&format!(
            "UPDATE smell_reports SET observed_at = {} WHERE id = {};",
            observed.timestamp_micros(),
            report.id
        ));
        self.audit(
            when,
            Some(report.user),
            AuditEntity::SmellReport,
            report.id,
            AuditAction::Update,
            Some(before),
            report.snapshot(),
        );
    }
}

/// Where each kind of case sits in [`rows_before_0005`], by id.
pub mod before_0005 {
    pub const USERS: [i64; 2] = [1, 2];
    /// (report id, reading id): written by one `log_smell` call, audit entries side by side.
    pub const PAIRS: [(i64, i64); 6] = [(1, 1), (2, 2), (5, 3), (8, 8), (9, 9), (10, 10)];
    pub const REPORTS_ALONE: [i64; 6] = [3, 4, 6, 7, 11, 12];
    pub const READINGS_ALONE: [i64; 4] = [4, 5, 6, 7];
    pub const REPORTS: i64 = 12;
    pub const READINGS: i64 = 10;
    /// Reports that carry a note, with the text.
    pub const NOTES: [(i64, &str); 2] = [
        (2, "kitchen smells of smoke"),
        (11, "smell from the balcony"),
    ];
    /// Report 4 is by user 2 and directly followed by user 1's combined log (report 5, reading 3).
    pub const ALONE_BEFORE_ANOTHER_PERSONS_PAIR: (i64, i64) = (4, 5);
    /// Report 6 (user 2, alone) and reading 4 (user 1, alone) are adjacent in the audit log and
    /// share their observed time.
    pub const ADJACENT_ACROSS_PEOPLE: (i64, i64) = (6, 4);
    /// Report 7 and reading 5 are adjacent and by one person, but observed a minute apart.
    pub const ADJACENT_AT_OTHER_TIMES: (i64, i64) = (7, 5);
    /// Readings 6 and 7 are written one after the other with the same observed time.
    pub const TWO_READINGS_IN_A_ROW: (i64, i64) = (6, 7);
    /// Report 8 moved to event 2 and back to event 1; report 9 moved to event 2 and stays;
    /// report 10 was retimed ten minutes earlier.
    pub const MOVED_AND_BACK: i64 = 8;
    pub const MOVED_AND_STAYS: i64 = 9;
    pub const RETIMED: i64 = 10;
    pub const NO_SMELL_CHECK: i64 = 12;
}

/// Everything that happened on the production database up to migration 0004, in made-up data
/// that mirrors it (2026-10-19): rows and the audit entries the version-4 store wrote with
/// them, in one hash chain. Two people, a few events, and these cases:
///
/// - combined logs by one person (reports 1, 2, 5, 8, 9 and 10 with their readings), their
///   audit entries side by side;
/// - a combined log by user 2 followed by a report alone by user 2 (report 3);
/// - a report alone by user 2 (4) directly followed by user 1's combined log (5 and reading 3);
/// - a report alone by user 2 (6) followed by a reading alone by user 1 (4) observed at the same
///   time, and a report alone (7) followed by a reading alone by the same person (5) a minute
///   apart: neither pairs;
/// - two readings alone in a row (6 and 7), at the same observed time;
/// - a combined log whose report was moved to another event and back (8), one whose report
///   moved and stays moved (9), and one whose report time was corrected (10);
/// - a report with a note alone (11), a combined log whose report has one (2);
/// - a no-smell check without rooms (12).
pub fn rows_before_0005() -> String {
    let mut db = Version4::new();
    db.user(1, "daniele");
    db.user(2, "partner");
    db.room(1, "Kitchen");
    db.room(2, "Bedroom");
    db.symptom(1, "Cough");
    db.event(1, 1, "2026-10-10T10:00:00Z", Some("2026-10-10T14:00:00Z"));
    db.event(2, 1, "2026-10-10T08:00:00Z", Some("2026-10-10T09:00:00Z"));
    db.event(3, 2, "2026-10-12T10:00:00Z", None);

    let report = |id: i64, user: i64, event: Option<i64>, observed: &str, rooms: &[(i64, u8)]| {
        let observed = at(observed);
        OldReport {
            id,
            user,
            event,
            observed,
            recorded: observed + TimeDelta::milliseconds(5),
            rooms: rooms.to_vec(),
            symptoms: vec![],
            note: None,
        }
    };
    let reading = |id: i64, user: i64, event: Option<i64>, observed: &str, value: f64| {
        let observed = at(observed);
        OldReading {
            id,
            user,
            event,
            observed,
            recorded: observed + TimeDelta::milliseconds(40),
            value,
        }
    };

    let mut r1 = report(1, 1, Some(1), "2026-10-10T10:05:00Z", &[(1, 6)]);
    r1.symptoms = vec![1];
    db.report(&r1);
    db.reading(&reading(1, 1, Some(1), "2026-10-10T10:05:00Z", 12.5));

    let mut r2 = report(2, 2, Some(1), "2026-10-10T10:20:00Z", &[(1, 3), (2, 2)]);
    r2.note = Some("kitchen smells of smoke");
    db.report(&r2);
    db.reading(&reading(2, 2, Some(1), "2026-10-10T10:20:00Z", 30.0));
    db.report(&report(3, 2, Some(1), "2026-10-10T10:30:00Z", &[(2, 4)]));

    db.report(&report(4, 2, Some(1), "2026-10-10T10:40:00Z", &[(1, 1)]));
    db.report(&report(5, 1, Some(1), "2026-10-10T10:50:00Z", &[(1, 5)]));
    db.reading(&reading(3, 1, Some(1), "2026-10-10T10:50:00Z", 18.0));

    db.report(&report(6, 2, Some(1), "2026-10-10T11:00:00Z", &[(2, 2)]));
    db.reading(&reading(4, 1, Some(1), "2026-10-10T11:00:00Z", 9.0));
    db.report(&report(7, 1, Some(1), "2026-10-10T11:10:00Z", &[(1, 2)]));
    db.reading(&reading(5, 1, Some(1), "2026-10-10T11:11:00Z", 7.5));

    db.reading(&reading(6, 1, Some(1), "2026-10-10T11:20:00Z", 14.0));
    db.reading(&reading(7, 1, Some(1), "2026-10-10T11:20:00Z", 15.0));

    let mut r8 = report(8, 1, Some(1), "2026-10-10T11:30:00Z", &[(1, 4)]);
    db.report(&r8);
    db.reading(&reading(8, 1, Some(1), "2026-10-10T11:30:00Z", 21.0));
    db.move_report(&mut r8, 2, at("2026-10-10T12:00:00Z"));
    db.move_report(&mut r8, 1, at("2026-10-10T12:01:00Z"));

    let mut r9 = report(9, 1, Some(1), "2026-10-10T11:40:00Z", &[(2, 6)]);
    db.report(&r9);
    db.reading(&reading(9, 1, Some(1), "2026-10-10T11:40:00Z", 33.0));
    db.move_report(&mut r9, 2, at("2026-10-10T12:10:00Z"));

    let mut r10 = report(10, 1, Some(1), "2026-10-10T11:50:00Z", &[(1, 7)]);
    db.report(&r10);
    db.reading(&reading(10, 1, Some(1), "2026-10-10T11:50:00Z", 44.0));
    db.retime_report(
        &mut r10,
        at("2026-10-10T11:40:00Z"),
        at("2026-10-10T12:20:00Z"),
    );

    let mut r11 = report(11, 2, Some(1), "2026-10-10T12:30:00Z", &[(1, 2)]);
    r11.note = Some("smell from the balcony");
    db.report(&r11);
    db.report(&report(12, 1, None, "2026-10-11T09:00:00Z", &[]));
    db.sql
}

/// SQL that moves each identity sequence past the ids [`rows_before_0005`] wrote by hand, so
/// rows written later do not collide. Postgres only.
pub const POSTGRES_SEQUENCES_AFTER_0005_ROWS: &str = "\
    SELECT setval(pg_get_serial_sequence('users', 'id'), (SELECT max(id) FROM users)); \
    SELECT setval(pg_get_serial_sequence('rooms', 'id'), (SELECT max(id) FROM rooms)); \
    SELECT setval(pg_get_serial_sequence('symptoms', 'id'), (SELECT max(id) FROM symptoms)); \
    SELECT setval(pg_get_serial_sequence('events', 'id'), (SELECT max(id) FROM events)); \
    SELECT setval(pg_get_serial_sequence('smell_reports', 'id'), (SELECT max(id) FROM smell_reports)); \
    SELECT setval(pg_get_serial_sequence('pm25_readings', 'id'), (SELECT max(id) FROM pm25_readings));";

/// A report with no `create` audit entry, which migration 0005 refuses to pair or convert.
pub const REPORT_WITHOUT_CREATE_ENTRY: &str = "\
    INSERT INTO smell_reports (id, event_id, user_id, observed_at, recorded_at) \
        VALUES (99, 1, 1, 0, 0);";

/// A reading with no `create` audit entry.
pub const READING_WITHOUT_CREATE_ENTRY: &str = "\
    INSERT INTO pm25_readings (id, user_id, observed_at, recorded_at, value_ugm3) \
        VALUES (99, 1, 0, 0, 5.0);";

/// The name the refusal carries in its error message.
pub const NEEDS_A_CREATE_ENTRY: &str = "every_report_and_reading_has_a_create_entry";

async fn entry_of_report<S: EntryRepo>(store: &S, id: i64) -> LogEntry {
    store
        .entry_of_report(ReportId(id))
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("report {id} has no entry"))
}

async fn entry_of_reading<S: EntryRepo>(store: &S, id: i64) -> LogEntry {
    store
        .entry_of_reading(ReadingId(id))
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("reading {id} has no entry"))
}

/// Every entry the migration created, oldest id first.
async fn all_entries<S: EntryRepo>(store: &S) -> Vec<LogEntry> {
    let mut entries = Vec::new();
    let mut id = 1;
    loop {
        match store.entry(LogEntryId(id)).await {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => return entries,
            Err(StoreError::NotFound { .. }) => return entries,
            Err(other) => panic!("{other:?}"),
        }
        id += 1;
    }
}

/// One entry per report, plus one per reading that was not written with a report.
pub async fn migration_0005_creates_one_entry_per_report_and_per_unpaired_reading<S: EntryRepo>(
    store: &S,
) {
    let entries = all_entries(store).await;
    let expected = before_0005::REPORTS + before_0005::READINGS_ALONE.len() as i64;
    assert_eq!(entries.len() as i64, expected);
    let both = entries
        .iter()
        .filter(|e| e.report.is_some() && e.reading.is_some())
        .count();
    let only_report = entries
        .iter()
        .filter(|e| e.report.is_some() && e.reading.is_none())
        .count();
    let only_reading = entries
        .iter()
        .filter(|e| e.report.is_none() && e.reading.is_some())
        .count();
    assert_eq!(
        (both, only_report, only_reading),
        (
            before_0005::PAIRS.len(),
            before_0005::REPORTS_ALONE.len(),
            before_0005::READINGS_ALONE.len()
        )
    );
}

pub async fn pairs_by_adjacent_create_entries<S: EntryRepo>(store: &S) {
    for (report, reading) in before_0005::PAIRS {
        let entry = entry_of_report(store, report).await;
        assert_eq!(entry_of_reading(store, reading).await, entry);
        assert_eq!(entry.report.as_ref().unwrap().id, ReportId(report));
        assert_eq!(entry.reading.as_ref().unwrap().id, ReadingId(reading));
    }
    for report in before_0005::REPORTS_ALONE {
        assert_eq!(entry_of_report(store, report).await.reading, None);
    }
}

pub async fn never_pairs_across_people<S: EntryRepo>(store: &S) {
    let (alone, pair) = before_0005::ALONE_BEFORE_ANOTHER_PERSONS_PAIR;
    let alone = entry_of_report(store, alone).await;
    let pair = entry_of_report(store, pair).await;
    assert_eq!((alone.user_id, alone.reading), (UserId(2), None));
    assert_ne!(alone.id, pair.id);
    assert_eq!((pair.user_id, pair.reading.is_some()), (UserId(1), true));

    let (report, reading) = before_0005::ADJACENT_ACROSS_PEOPLE;
    let report = entry_of_report(store, report).await;
    let reading = entry_of_reading(store, reading).await;
    assert_ne!(report.id, reading.id);
    assert_eq!((report.user_id, report.reading), (UserId(2), None));
    assert_eq!((reading.user_id, reading.report), (UserId(1), None));
    assert_eq!(report.observed_at, reading.observed_at);
}

pub async fn keeps_a_reading_after_a_reading_alone<S: EntryRepo>(store: &S) {
    let (first, second) = before_0005::TWO_READINGS_IN_A_ROW;
    let first = entry_of_reading(store, first).await;
    let second = entry_of_reading(store, second).await;
    assert_ne!(first.id, second.id);
    for entry in [&first, &second] {
        assert_eq!((entry.report.is_none(), entry.note.is_none()), (true, true));
    }
    assert_eq!(first.observed_at, second.observed_at);

    let (report, reading) = before_0005::ADJACENT_AT_OTHER_TIMES;
    let report = entry_of_report(store, report).await;
    let reading = entry_of_reading(store, reading).await;
    assert_ne!(
        report.id, reading.id,
        "one person, adjacent, but another time"
    );
    assert_eq!(report.reading, None);
    assert_eq!(reading.report, None);
}

pub async fn pairs_a_report_moved_or_retimed_since<S: EntryRepo>(store: &S) {
    let back = entry_of_report(store, before_0005::MOVED_AND_BACK).await;
    assert_eq!(back.event_id, Some(EventId(1)));
    assert_eq!(back.reading.as_ref().unwrap().id, ReadingId(8));

    let stays = entry_of_report(store, before_0005::MOVED_AND_STAYS).await;
    assert_eq!(
        stays.event_id,
        Some(EventId(2)),
        "the shared entry takes the report's current event"
    );
    assert_eq!(stays.reading_view().unwrap().event_id, Some(EventId(2)));
    assert_eq!(stays.reading.as_ref().unwrap().id, ReadingId(9));

    let retimed = entry_of_report(store, before_0005::RETIMED).await;
    assert_eq!(retimed.observed_at, at("2026-10-10T11:40:00Z"));
    assert_eq!(
        retimed.reading_view().unwrap().observed_at,
        at("2026-10-10T11:40:00Z")
    );
    assert_eq!(retimed.reading.as_ref().unwrap().id, ReadingId(10));
    assert_eq!(
        retimed.recorded_at,
        at("2026-10-10T11:50:00Z") + TimeDelta::milliseconds(5),
        "recorded_at is the report's"
    );
}

pub async fn moves_each_report_note_to_its_entry<S: EntryRepo>(store: &S) {
    let mut with_note = Vec::new();
    for entry in all_entries(store).await {
        if let Some(note) = &entry.note {
            let report = entry.report.as_ref().expect("only reports had notes");
            with_note.push((report.id.0, note.as_str().to_owned()));
        }
    }
    let expected: Vec<_> = before_0005::NOTES
        .iter()
        .map(|&(id, text)| (id, text.to_owned()))
        .collect();
    assert_eq!(with_note, expected);
    assert_eq!(
        entry_of_report(store, 2).await.note,
        Note::parse(Some("kitchen smells of smoke")).unwrap()
    );
}

pub async fn keeps_report_and_reading_ids<S: EntryRepo>(store: &S) {
    for id in 1..=before_0005::REPORTS {
        let entry = entry_of_report(store, id).await;
        assert_eq!(entry.report.unwrap().id, ReportId(id));
    }
    for id in 1..=before_0005::READINGS {
        let entry = entry_of_reading(store, id).await;
        assert_eq!(entry.reading.unwrap().id, ReadingId(id));
    }
    let first = entry_of_report(store, 1).await;
    assert_eq!(first.values().symptoms(), [SymptomId(1)]);
    assert_eq!(first.values().room_ids(), [RoomId(1)]);
    assert_eq!(first.reading.unwrap().value, Pm25::new(12.5).unwrap());
    let check = entry_of_report(store, before_0005::NO_SMELL_CHECK).await;
    assert_eq!(check.event_id, None);
    assert!(check.values().rooms().is_empty());
    assert_eq!(
        entry_of_report(store, 9).await.reading.unwrap().value,
        Pm25::new(33.0).unwrap()
    );
}

/// The migration never rewrites the audit log: the chain written by version 4 still verifies,
/// and the entries the migration added are all `log_entry` entries.
pub async fn keeps_the_audit_chain_valid<S: AuditRepo>(store: &S, before: &[ChainedEntry]) {
    let log = store.audit_log().await.unwrap();
    assert!(!before.is_empty());
    assert_eq!(verify_chain(&log), Ok(()));
    assert_eq!(
        &log[..before.len()],
        before,
        "existing entries are untouched"
    );
    assert!(
        log[before.len()..]
            .iter()
            .all(|e| e.record.entity == AuditEntity::LogEntry)
    );
}

/// The backfill wrote exactly one `create` entry per log entry after the entries version 4
/// wrote (`before`, the audit log read before migrating): actor none, no `before`, the whole-log
/// snapshot, in entry id order, one instant, and the store verifies clean.
pub async fn backfill_writes_one_create_entry_per_log_entry_once<
    S: AuditRepo + EntryRepo + SnapshotRepo,
>(
    store: &S,
    before: &[ChainedEntry],
) {
    let entries = all_entries(store).await;
    let log = store.audit_log().await.unwrap();
    assert_eq!(
        &log[..before.len()],
        before,
        "existing entries are untouched"
    );
    let added = &log[before.len()..];
    assert_eq!(added.len(), entries.len(), "one entry per log entry");
    for (id, (row, entry)) in (before.len() as i64 + 1..).zip(added.iter().zip(&entries)) {
        let record = &row.record;
        assert_eq!(record.id, id, "ids continue the chain");
        assert_eq!(record.entity, AuditEntity::LogEntry);
        assert_eq!(record.entity_id, entry.id.0);
        assert_eq!(record.action, AuditAction::Create);
        assert_eq!(record.actor_id, None);
        assert_eq!(record.before, None);
        assert_eq!(record.after, Some(entry.snapshot()));
        assert_eq!(record.at_micros, added[0].record.at_micros, "one instant");
    }
    assert_eq!(
        added
            .iter()
            .filter(|e| e.record.entity != AuditEntity::LogEntry)
            .count(),
        0
    );
    assert_eq!(verify_chain(&log), Ok(()));
    let report = verify_store(store).await.unwrap();
    assert!(report.is_ok(), "{report:?}");
}

/// Tampering with any part of a migrated log is caught, and putting it back verifies clean.
/// Each pair is the edit and its inverse.
pub async fn a_direct_edit_of_any_part_of_a_log_is_caught<S: SnapshotRepo>(
    store: &S,
    raw: &RawSql,
) {
    let edits: [(&'static str, &'static str); 4] = [
        (
            "UPDATE log_entries SET note = 'edited' \
             WHERE id = (SELECT log_entry_id FROM smell_reports WHERE id = 2)",
            "UPDATE log_entries SET note = 'kitchen smells of smoke' \
             WHERE id = (SELECT log_entry_id FROM smell_reports WHERE id = 2)",
        ),
        (
            "UPDATE smell_report_rooms SET level = level + 1 WHERE smell_report_id = 1",
            "UPDATE smell_report_rooms SET level = level - 1 WHERE smell_report_id = 1",
        ),
        (
            "UPDATE pm25_readings SET value_ugm3 = value_ugm3 + 1 WHERE id = 1",
            "UPDATE pm25_readings SET value_ugm3 = value_ugm3 - 1 WHERE id = 1",
        ),
        (
            "UPDATE pm25_readings SET log_entry_id = \
             (SELECT log_entry_id FROM smell_reports WHERE id = 3) WHERE id = 1",
            "UPDATE pm25_readings SET log_entry_id = \
             (SELECT log_entry_id FROM smell_reports WHERE id = 1) WHERE id = 1",
        ),
    ];
    assert!(verify_store(store).await.unwrap().is_ok());
    for (edit, undo) in edits {
        raw(edit).await.unwrap();
        let verdict = verify_store(store).await.unwrap();
        assert!(
            !verdict.problems.is_empty()
                && verdict.problems.iter().all(|p| matches!(
                    p,
                    SnapshotProblem::Differs {
                        entity: AuditEntity::LogEntry,
                        ..
                    }
                )),
            "{edit}: {verdict:?}"
        );
        raw(undo).await.unwrap();
        assert!(verify_store(store).await.unwrap().is_ok(), "{undo}");
    }
}

/// An entry written directly after the backfill is not backfilled by a later `migrate`: the
/// audit has `log_entry` entries, so the guard holds, and the entry is reported as unaudited.
pub async fn an_entry_logged_after_the_backfill_is_not_backfilled<S, M, Fut>(
    store: &S,
    raw: &RawSql,
    migrate: M,
) where
    S: AuditRepo + SnapshotRepo,
    M: Fn() -> Fut,
    Fut: std::future::Future<Output = StoreResult<()>>,
{
    let audited = store.audit_log().await.unwrap();
    raw(
        "INSERT INTO log_entries (user_id, event_id, observed_at, recorded_at, note) \
         VALUES (1, NULL, 1000000, 1000000, NULL)",
    )
    .await
    .unwrap();
    raw("INSERT INTO pm25_readings (log_entry_id, value_ugm3) \
         VALUES ((SELECT max(id) FROM log_entries), 5.0)")
    .await
    .unwrap();
    migrate().await.unwrap();
    assert_eq!(
        store.audit_log().await.unwrap(),
        audited,
        "nothing was written"
    );
    let report = verify_store(store).await.unwrap();
    assert!(
        matches!(
            report.problems[..],
            [SnapshotProblem::Unaudited {
                entity: AuditEntity::LogEntry,
                ..
            }]
        ),
        "{report:?}"
    );
}

/// A database that never had reports or readings gets no `log_entry` entries.
pub async fn an_empty_database_migrates_without_backfill_entries<S: AuditRepo>(store: &S) {
    let log = store.audit_log().await.unwrap();
    assert!(log.iter().all(|e| e.record.entity != AuditEntity::LogEntry));
}

/// A migrated report's History as the report page builds it.
async fn history_of<S: AuditRepo + EntryRepo>(store: &S, report: i64) -> Vec<HistoryEntry> {
    let entry = entry_of_report(store, report).await;
    let old_rows = store
        .audit_history(AuditEntity::SmellReport, report)
        .await
        .unwrap();
    let entry_rows = store
        .audit_history(AuditEntity::LogEntry, entry.id.0)
        .await
        .unwrap();
    entry_history(&old_rows, &entry_rows)
}

fn actions(history: &[HistoryEntry]) -> Vec<AuditAction> {
    history.iter().map(|line| line.action).collect()
}

/// A migrated report's History reads as before the migration, with no backfill line: the
/// report moved twice shows `create, move, move`, the retimed one `create` and a time change.
/// A correction made afterwards adds exactly one `update` line.
pub async fn migrated_history_reads_as_before_and_takes_a_correction<S>(store: &S)
where
    S: AuditRepo + EntryRepo + Transactional<Tx: WriteTx>,
{
    let moved = history_of(store, before_0005::MOVED_AND_BACK).await;
    assert_eq!(
        actions(&moved),
        [AuditAction::Create, AuditAction::Move, AuditAction::Move]
    );
    assert!(moved[0].actor.is_some(), "no system backfill line");

    let retimed = history_of(store, before_0005::RETIMED).await;
    assert_eq!(
        actions(&retimed),
        [AuditAction::Create, AuditAction::Update]
    );
    assert!(matches!(
        retimed[1].changes.as_slice(),
        [Change::ObservedAt { .. }]
    ));

    let entry = entry_of_report(store, before_0005::MOVED_AND_BACK).await;
    let actor = Actor {
        id: entry.user_id,
        role: Role::Resident,
        expires_at: None,
    };
    let correction = ReportCorrection {
        observed_at: entry.observed_at,
        values: report_values(9, &[FIRST_ROOM], &[]),
        note: None,
    };
    let mut tx = store.begin(Some(entry.user_id)).await.unwrap();
    correct_report(
        &mut tx,
        &actor,
        ReportId(before_0005::MOVED_AND_BACK),
        correction,
        &rome(),
        at("2026-10-19T12:00:00Z"),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let corrected = history_of(store, before_0005::MOVED_AND_BACK).await;
    assert_eq!(corrected.len(), moved.len() + 1, "exactly one extra line");
    assert_eq!(corrected[..moved.len()], moved[..]);
    assert_eq!(corrected.last().unwrap().action, AuditAction::Update);
    assert_eq!(
        actions(&history_of(store, before_0005::RETIMED).await),
        actions(&retimed),
        "other reports' History is untouched"
    );
}
