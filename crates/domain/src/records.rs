//! Stored records and the inputs that create them. Each record serialises to the snapshot the
//! audit log keeps of it.

use std::borrow::Cow;
use std::fmt;

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::DomainError;
use crate::audit::AuditEntity;
use crate::events::EventSummary;
use crate::permissions::Actor;
use crate::types::{
    CloseReason, EventId, InviteId, InvitePurpose, Level, ListName, Locale, LogEntryId, Pm25,
    ReadingId, ReportId, Role, RoomArea, RoomId, SymptomId, TokenHash, UserId, Username,
};

/// A record the audit log keeps snapshots of.
pub trait Audited: Serialize {
    const ENTITY: AuditEntity;

    fn audit_id(&self) -> i64;

    fn snapshot(&self) -> Value {
        serde_json::to_value(self).expect("records always serialise")
    }
}

/// An Argon2id hash in PHC string format. Never printed and never audited.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    pub fn new(phc: String) -> Self {
        Self(phc)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PasswordHash(<redacted>)")
    }
}

fn serialize_is_some<T, S: Serializer>(
    value: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_bool(value.is_some())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct User {
    pub id: UserId,
    pub username: Username,
    /// Audited only as whether one is set.
    #[serde(rename = "has_password", serialize_with = "serialize_is_some")]
    pub password_hash: Option<PasswordHash>,
    pub role: Role,
    pub locale: Locale,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn actor(&self) -> Actor {
        Actor {
            id: self.id,
            role: self.role,
            expires_at: self.expires_at,
        }
    }
}

impl Audited for User {
    const ENTITY: AuditEntity = AuditEntity::User;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

/// A one-time link to set up an account or reset its password (spec §8.3). The token's hash is
/// stored but never part of the record or its audit snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Invite {
    pub id: InviteId,
    pub user_id: UserId,
    pub purpose: InvitePurpose,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl Invite {
    /// How long a link works after it is created.
    pub const LIFETIME: TimeDelta = TimeDelta::minutes(5);

    /// Unused and not yet expired: from `expires_at` on, the link is dead.
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.used_at.is_none() && now < self.expires_at
    }
}

impl Audited for Invite {
    const ENTITY: AuditEntity = AuditEntity::Invite;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewInvite {
    pub user_id: UserId,
    pub purpose: InvitePurpose,
    pub token_hash: TokenHash,
    pub created_by: UserId,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Room {
    pub id: RoomId,
    pub name: ListName,
    /// Set for starter rooms, which are shown translated; custom rooms are shown as typed.
    pub i18n_key: Option<String>,
    pub area: RoomArea,
    pub hidden_at: Option<DateTime<Utc>>,
}

impl Audited for Room {
    const ENTITY: AuditEntity = AuditEntity::Room;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Symptom {
    pub id: SymptomId,
    pub name: ListName,
    pub i18n_key: Option<String>,
    pub hidden_at: Option<DateTime<Utc>>,
}

impl Audited for Symptom {
    const ENTITY: AuditEntity = AuditEntity::Symptom;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

/// How and when an event ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Closure {
    pub at: DateTime<Utc>,
    pub reason: CloseReason,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    pub id: EventId,
    pub owner_id: UserId,
    pub opened_at: DateTime<Utc>,
    pub closed: Option<Closure>,
    pub auto_close_notice_seen_at: Option<DateTime<Utc>>,
    pub recorded_at: DateTime<Utc>,
}

impl Event {
    pub fn is_open(&self) -> bool {
        self.closed.is_none()
    }

    pub fn summary(&self) -> EventSummary {
        EventSummary {
            id: self.id,
            opened_at: self.opened_at,
        }
    }
}

impl Audited for Event {
    const ENTITY: AuditEntity = AuditEntity::Event;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

/// A room the person checked, and how strong the smell was there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomLevel {
    pub room: RoomId,
    pub level: Level,
}

/// A free-text note: trimmed, never blank, at most [`Note::MAX_CHARS`] characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Note(String);

impl Note {
    pub const MAX_CHARS: usize = 2000;

    /// A blank or missing note is no note.
    pub fn parse(raw: Option<&str>) -> Result<Option<Self>, DomainError> {
        let Some(text) = raw.map(str::trim).filter(|n| !n.is_empty()) else {
            return Ok(None);
        };
        let chars = text.chars().count();
        if chars > Self::MAX_CHARS {
            return Err(DomainError::NoteTooLong(chars));
        }
        Ok(Some(Self(text.to_owned())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a smell report says: the rooms checked, each with its level, sorted by room, and the
/// symptoms, sorted and unique. No rooms at all means "no smell anywhere in the home" (spec
/// §7.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportValues {
    rooms: Vec<RoomLevel>,
    symptoms: Vec<SymptomId>,
}

impl ReportValues {
    pub fn new(
        mut rooms: Vec<RoomLevel>,
        mut symptoms: Vec<SymptomId>,
    ) -> Result<Self, DomainError> {
        rooms.sort_unstable_by_key(|r| r.room);
        if let Some(pair) = rooms.windows(2).find(|pair| pair[0].room == pair[1].room) {
            return Err(DomainError::RoomListedTwice(pair[0].room.0));
        }
        symptoms.sort_unstable();
        symptoms.dedup();
        Ok(Self { rooms, symptoms })
    }

    pub fn rooms(&self) -> &[RoomLevel] {
        &self.rooms
    }

    pub fn room_ids(&self) -> Vec<RoomId> {
        self.rooms.iter().map(|r| r.room).collect()
    }

    /// The level reported for `room`; `None` when the room was not checked.
    pub fn level_in(&self, room: RoomId) -> Option<Level> {
        self.rooms.iter().find(|r| r.room == room).map(|r| r.level)
    }

    /// Whether this is a no-smell report: every room listed is at 0, or none is listed (spec
    /// §7.1). Such a report never opens an event.
    pub fn is_no_smell(&self) -> bool {
        self.intensity().is_clean()
    }

    /// The report's intensity: its highest room level (spec §6); 0 without rooms.
    pub fn intensity(&self) -> Level {
        self.rooms
            .iter()
            .map(|r| r.level)
            .max()
            .unwrap_or(Level::CLEAN)
    }

    pub fn symptoms(&self) -> &[SymptomId] {
        &self.symptoms
    }
}

/// The smell half of a log entry. Audited as part of its [`LogEntry`], so the entry it belongs
/// to is not part of its own snapshot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SmellReport {
    pub id: ReportId,
    #[serde(skip)]
    pub log_entry_id: LogEntryId,
    #[serde(flatten)]
    pub values: ReportValues,
}

impl SmellReport {
    /// A snapshot audited before rooms had their own levels (`"intensity": 6, "rooms": [2]`),
    /// in today's shape: every room at the report's intensity, as migration 0003 converted the
    /// stored report. Any other snapshot is returned unchanged.
    pub fn upgrade_snapshot(snapshot: &Value) -> Cow<'_, Value> {
        let Value::Object(fields) = snapshot else {
            return Cow::Borrowed(snapshot);
        };
        let (Some(intensity), Some(Value::Array(rooms))) =
            (fields.get("intensity"), fields.get("rooms"))
        else {
            return Cow::Borrowed(snapshot);
        };
        let mut upgraded = fields.clone();
        upgraded.remove("intensity");
        let rooms = rooms
            .iter()
            .map(|room| serde_json::json!({ "room": room, "level": intensity }))
            .collect();
        upgraded.insert("rooms".to_owned(), Value::Array(rooms));
        Cow::Owned(Value::Object(upgraded))
    }
}

/// The PM2.5 half of a log entry; audited as part of its [`LogEntry`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pm25Reading {
    pub id: ReadingId,
    #[serde(skip)]
    pub log_entry_id: LogEntryId,
    pub value: Pm25,
}

/// One press of Log: who, when, which event and the note, owning a smell report, a PM2.5
/// reading or both. Its snapshot is the whole log (spec §6), children included.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LogEntry {
    pub id: LogEntryId,
    pub user_id: UserId,
    /// `None` for a no-smell check made while no event was running (spec §7.1) and for a
    /// reading outside every event.
    pub event_id: Option<EventId>,
    pub observed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub note: Option<Note>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub report: Option<SmellReport>,
    pub reading: Option<Pm25Reading>,
}

impl LogEntry {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn report_view(&self) -> Option<ReportView> {
        let report = self.report.as_ref()?;
        Some(ReportView {
            id: report.id,
            entry_id: self.id,
            event_id: self.event_id,
            user_id: self.user_id,
            observed_at: self.observed_at,
            recorded_at: self.recorded_at,
            values: report.values.clone(),
            note: self.note.clone(),
            deleted_at: self.deleted_at,
        })
    }

    pub fn reading_view(&self) -> Option<ReadingView> {
        let reading = self.reading.as_ref()?;
        Some(ReadingView {
            id: reading.id,
            entry_id: self.id,
            event_id: self.event_id,
            user_id: self.user_id,
            observed_at: self.observed_at,
            recorded_at: self.recorded_at,
            value: reading.value,
            deleted_at: self.deleted_at,
        })
    }
}

impl Audited for LogEntry {
    const ENTITY: AuditEntity = AuditEntity::LogEntry;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

/// The smell reports of `entries`, in their order.
pub fn report_views(entries: &[LogEntry]) -> Vec<ReportView> {
    entries.iter().filter_map(LogEntry::report_view).collect()
}

/// The PM2.5 readings of `entries`, in their order.
pub fn reading_views(entries: &[LogEntry]) -> Vec<ReadingView> {
    entries.iter().filter_map(LogEntry::reading_view).collect()
}

/// A smell report together with what its entry says about it: the shape lists, graphs and
/// summaries read.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportView {
    pub id: ReportId,
    pub entry_id: LogEntryId,
    pub event_id: Option<EventId>,
    pub user_id: UserId,
    pub observed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub values: ReportValues,
    pub note: Option<Note>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ReportView {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

/// A PM2.5 reading together with what its entry says about it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadingView {
    pub id: ReadingId,
    pub entry_id: LogEntryId,
    pub event_id: Option<EventId>,
    pub user_id: UserId,
    pub observed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub value: Pm25,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewUser {
    pub username: Username,
    /// `None` until the invitee sets a password.
    pub password_hash: Option<PasswordHash>,
    pub role: Role,
    pub locale: Locale,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewRoom {
    pub name: ListName,
    pub i18n_key: Option<String>,
    pub area: RoomArea,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewSymptom {
    pub name: ListName,
    pub i18n_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewEvent {
    pub owner_id: UserId,
    pub opened_at: DateTime<Utc>,
    /// `Some` for an event created already over (a backdated report).
    pub closed: Option<Closure>,
}

/// What creates a log entry: at least one of a report and a reading.
#[derive(Debug, Clone, PartialEq)]
pub struct NewEntry {
    pub user_id: UserId,
    pub event_id: Option<EventId>,
    pub observed_at: DateTime<Utc>,
    pub note: Option<Note>,
    pub report: Option<ReportValues>,
    pub reading: Option<Pm25>,
}

impl NewEntry {
    pub fn new(
        user_id: UserId,
        event_id: Option<EventId>,
        observed_at: DateTime<Utc>,
        note: Option<Note>,
        report: Option<ReportValues>,
        reading: Option<Pm25>,
    ) -> Result<Self, DomainError> {
        if report.is_none() && reading.is_none() {
            return Err(DomainError::EntryWithoutChildren);
        }
        Ok(Self {
            user_id,
            event_id,
            observed_at,
            note,
            report,
            reading,
        })
    }
}

/// A login session as stored (spec §8.4). Not audited: sessions are not evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSession {
    /// SHA-256 of the session id in the cookie, so a copy of the database cannot be used to
    /// take over a session.
    pub id_hash: String,
    /// The signed-in user; lets a password change or expiry end all of that user's sessions.
    pub user_id: Option<UserId>,
    /// The session's key/value data as JSON.
    pub data: String,
    pub expires_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use serde_json::json;

    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn user(password: Option<&str>) -> User {
        User {
            id: UserId(1),
            username: Username::new("daniele").unwrap(),
            password_hash: password.map(|p| PasswordHash::new(p.to_owned())),
            role: Role::Resident,
            locale: Locale::It,
            expires_at: None,
            created_at: t("2026-01-15T20:00:00Z"),
        }
    }

    fn at(room: i64, level: u8) -> RoomLevel {
        RoomLevel {
            room: RoomId(room),
            level: Level::new(level).unwrap(),
        }
    }

    fn values(rooms: &[(i64, u8)]) -> ReportValues {
        ReportValues::new(
            rooms.iter().map(|&(room, level)| at(room, level)).collect(),
            vec![SymptomId(2), SymptomId(1), SymptomId(2)],
        )
        .unwrap()
    }

    #[test]
    fn user_snapshot_hides_the_password_hash() {
        let snapshot = user(Some("argon-hash-value")).snapshot();
        assert_eq!(snapshot["has_password"], true);
        assert!(!snapshot.to_string().contains("argon-hash-value"));
        assert_eq!(user(None).snapshot()["has_password"], false);
    }

    #[test]
    fn password_hash_debug_is_redacted() {
        let printed = format!("{:?}", user(Some("argon-hash-value")));
        assert!(!printed.contains("argon-hash-value"), "{printed}");
        assert!(printed.contains("<redacted>"));
    }

    #[test]
    fn user_snapshot_has_exactly_the_audited_fields() {
        assert_eq!(
            user(Some("x")).snapshot(),
            json!({
                "id": 1,
                "username": "daniele",
                "has_password": true,
                "role": "resident",
                "locale": "it",
                "expires_at": null,
                "created_at": "2026-01-15T20:00:00Z",
            })
        );
    }

    #[test]
    fn report_values_sort_rooms_and_symptoms() {
        let v = values(&[(3, 0), (1, 6)]);
        assert_eq!(v.rooms(), [at(1, 6), at(3, 0)]);
        assert_eq!(v.room_ids(), [RoomId(1), RoomId(3)]);
        assert_eq!(v.symptoms(), [SymptomId(1), SymptomId(2)]);
    }

    #[test]
    fn a_report_lists_each_room_once() {
        assert_eq!(
            ReportValues::new(vec![at(3, 2), at(1, 1), at(3, 2)], vec![]),
            Err(DomainError::RoomListedTwice(3))
        );
    }

    #[test]
    fn the_intensity_is_the_highest_room_level() {
        let v = values(&[(1, 2), (2, 7), (3, 0)]);
        assert_eq!(v.intensity(), Level::new(7).unwrap());
        assert_eq!(v.level_in(RoomId(3)), Some(Level::CLEAN));
        assert_eq!(v.level_in(RoomId(4)), None);
        assert_eq!(values(&[(1, 0)]).intensity(), Level::CLEAN);
    }

    #[test]
    fn only_clean_rooms_or_no_rooms_make_a_no_smell_report() {
        assert!(values(&[(1, 0), (2, 0)]).is_no_smell());
        assert!(values(&[]).is_no_smell(), "no rooms: the whole home");
        assert!(!values(&[(1, 0), (2, 1)]).is_no_smell());
    }

    #[test]
    fn note_rules_are_unchanged() {
        assert_eq!(Note::parse(None), Ok(None));
        assert_eq!(Note::parse(Some("  \n ")), Ok(None));
        let trimmed = Note::parse(Some("  smoky  ")).unwrap().unwrap();
        assert_eq!(trimmed.as_str(), "smoky");
        assert_eq!(serde_json::to_value(&trimmed).unwrap(), json!("smoky"));
        let at_limit = "ç".repeat(Note::MAX_CHARS);
        assert!(Note::parse(Some(&at_limit)).unwrap().is_some());
        let over = "x".repeat(Note::MAX_CHARS + 1);
        assert_eq!(
            Note::parse(Some(&over)),
            Err(DomainError::NoteTooLong(2001))
        );
    }

    fn entry(report: Option<SmellReport>, reading: Option<Pm25Reading>) -> LogEntry {
        LogEntry {
            id: LogEntryId(3),
            user_id: UserId(1),
            event_id: Some(EventId(4)),
            observed_at: t("2026-01-15T19:00:00Z"),
            recorded_at: t("2026-01-15T19:00:00.123456Z"),
            note: Note::parse(Some("from the balcony")).unwrap(),
            deleted_at: None,
            report,
            reading,
        }
    }

    fn report(rooms: &[(i64, u8)]) -> SmellReport {
        SmellReport {
            id: ReportId(9),
            log_entry_id: LogEntryId(3),
            values: values(rooms),
        }
    }

    fn reading() -> Pm25Reading {
        Pm25Reading {
            id: ReadingId(7),
            log_entry_id: LogEntryId(3),
            value: Pm25::new(12.5).unwrap(),
        }
    }

    #[test]
    fn a_log_entry_snapshot_is_the_whole_log() {
        let both = entry(Some(report(&[(2, 6), (5, 0)])), Some(reading()));
        assert_eq!(
            both.snapshot(),
            json!({
                "id": 3,
                "user_id": 1,
                "event_id": 4,
                "observed_at": "2026-01-15T19:00:00Z",
                "recorded_at": "2026-01-15T19:00:00.123456Z",
                "note": "from the balcony",
                "deleted_at": null,
                "report": {
                    "id": 9,
                    "rooms": [{ "room": 2, "level": 6 }, { "room": 5, "level": 0 }],
                    "symptoms": [1, 2],
                },
                "reading": { "id": 7, "value": 12.5 },
            })
        );
        assert_eq!(both.audit_id(), 3);
        let reading_only = entry(None, Some(reading()));
        assert_eq!(reading_only.snapshot()["report"], json!(null));
        assert_eq!(
            reading_only.snapshot()["reading"],
            json!({ "id": 7, "value": 12.5 })
        );
        let check = LogEntry {
            event_id: None,
            ..entry(Some(report(&[])), None)
        };
        assert_eq!(check.snapshot()["event_id"], json!(null));
        assert_eq!(check.snapshot()["report"]["rooms"], json!([]));
        assert_eq!(check.snapshot()["reading"], json!(null));
    }

    #[test]
    fn views_copy_the_entry_fields_next_to_each_child() {
        let both = entry(Some(report(&[(2, 6)])), Some(reading()));
        let report_view = both.report_view().unwrap();
        assert_eq!(
            (report_view.id, report_view.entry_id, report_view.event_id),
            (ReportId(9), LogEntryId(3), Some(EventId(4)))
        );
        assert_eq!(
            report_view.note.as_ref().map(Note::as_str),
            Some("from the balcony")
        );
        assert_eq!(report_view.values, values(&[(2, 6)]));
        let reading_view = both.reading_view().unwrap();
        assert_eq!(
            (
                reading_view.id,
                reading_view.entry_id,
                reading_view.observed_at
            ),
            (ReadingId(7), LogEntryId(3), t("2026-01-15T19:00:00Z"))
        );
        assert_eq!(reading_view.value, Pm25::new(12.5).unwrap());
        assert!(entry(None, Some(reading())).report_view().is_none());
        assert!(entry(Some(report(&[])), None).reading_view().is_none());
    }

    #[test]
    fn an_entry_needs_a_report_or_a_reading() {
        let new = |report, reading| {
            NewEntry::new(
                UserId(1),
                None,
                t("2026-01-15T19:00:00Z"),
                None,
                report,
                reading,
            )
        };
        assert_eq!(new(None, None), Err(DomainError::EntryWithoutChildren));
        assert!(new(Some(values(&[(1, 1)])), None).is_ok());
        assert!(new(None, Some(Pm25::new(3.0).unwrap())).is_ok());
    }

    #[test]
    fn a_snapshot_from_before_room_levels_reads_as_every_room_at_the_intensity() {
        let old = json!({
            "id": 9,
            "event_id": 4,
            "intensity": 6,
            "rooms": [2, 5],
            "symptoms": [1],
        });
        assert_eq!(
            SmellReport::upgrade_snapshot(&old).into_owned(),
            json!({
                "id": 9,
                "event_id": 4,
                "rooms": [{ "room": 2, "level": 6 }, { "room": 5, "level": 6 }],
                "symptoms": [1],
            })
        );
        let current = json!({ "id": 9, "rooms": [{ "room": 2, "level": 6 }] });
        assert!(matches!(
            SmellReport::upgrade_snapshot(&current),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn room_snapshot_has_exactly_the_audited_fields() {
        let room = Room {
            id: RoomId(3),
            name: ListName::new("Kitchen").unwrap(),
            i18n_key: Some("room-kitchen".to_owned()),
            area: RoomArea::Inside,
            hidden_at: Some(t("2026-01-15T20:00:00Z")),
        };
        assert_eq!(
            room.snapshot(),
            json!({
                "id": 3,
                "name": "Kitchen",
                "i18n_key": "room-kitchen",
                "area": "inside",
                "hidden_at": "2026-01-15T20:00:00Z",
            })
        );
    }

    #[test]
    fn symptom_snapshot_has_exactly_the_audited_fields() {
        let symptom = Symptom {
            id: SymptomId(5),
            name: ListName::new("Headache").unwrap(),
            i18n_key: None,
            hidden_at: None,
        };
        assert_eq!(
            symptom.snapshot(),
            json!({
                "id": 5,
                "name": "Headache",
                "i18n_key": null,
                "hidden_at": null,
            })
        );
    }

    #[test]
    fn an_event_is_open_until_it_has_a_closure() {
        let mut event = Event {
            id: EventId(4),
            owner_id: UserId(1),
            opened_at: t("2026-01-15T18:05:00Z"),
            closed: None,
            auto_close_notice_seen_at: None,
            recorded_at: t("2026-01-15T18:05:01Z"),
        };
        assert!(event.is_open());
        assert_eq!(event.summary().opened_at, t("2026-01-15T18:05:00Z"));
        event.closed = Some(Closure {
            at: event.opened_at + TimeDelta::hours(1),
            reason: CloseReason::Manual,
        });
        assert!(!event.is_open());
        assert_eq!(
            event.snapshot()["closed"],
            json!({ "at": "2026-01-15T19:05:00Z", "reason": "manual" })
        );
    }

    #[test]
    fn each_record_names_its_entity() {
        assert_eq!(User::ENTITY, AuditEntity::User);
        assert_eq!(Room::ENTITY, AuditEntity::Room);
        assert_eq!(Symptom::ENTITY, AuditEntity::Symptom);
        assert_eq!(Event::ENTITY, AuditEntity::Event);
        assert_eq!(LogEntry::ENTITY, AuditEntity::LogEntry);
        assert_eq!(Invite::ENTITY, AuditEntity::Invite);
    }

    #[test]
    fn an_invite_works_once_and_for_five_minutes() {
        let created_at = t("2026-01-15T20:00:00Z");
        let invite = Invite {
            id: InviteId(1),
            user_id: UserId(2),
            purpose: InvitePurpose::Setup,
            created_by: UserId(1),
            created_at,
            expires_at: created_at + Invite::LIFETIME,
            used_at: None,
        };
        assert!(invite.is_usable(created_at));
        assert!(invite.is_usable(created_at + TimeDelta::seconds(299)));
        assert!(!invite.is_usable(created_at + Invite::LIFETIME));
        let used = Invite {
            used_at: Some(created_at),
            ..invite.clone()
        };
        assert!(!used.is_usable(created_at));
        assert_eq!(
            invite.snapshot(),
            json!({
                "id": 1, "user_id": 2, "purpose": "setup", "created_by": 1,
                "created_at": "2026-01-15T20:00:00Z", "expires_at": "2026-01-15T20:05:00Z",
                "used_at": null
            })
        );
    }

    #[test]
    fn user_converts_to_its_permission_actor() {
        let actor = user(None).actor();
        assert_eq!(actor.id, UserId(1));
        assert_eq!(actor.role, Role::Resident);
    }
}
