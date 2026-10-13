//! Stored records and the inputs that create them. Each record serialises to the snapshot the
//! audit log keeps of it.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer};
use serde_json::Value;

use crate::DomainError;
use crate::audit::AuditEntity;
use crate::events::EventSummary;
use crate::permissions::Actor;
use crate::types::{
    CloseReason, EventId, Intensity, ListName, Locale, Pm25, ReadingId, ReportId, Role, RoomArea,
    RoomId, SymptomId, UserId, Username,
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

/// What a smell report says. Room and symptom ids are kept sorted and unique, and a blank note
/// is no note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportValues {
    intensity: Intensity,
    rooms: Vec<RoomId>,
    symptoms: Vec<SymptomId>,
    note: Option<String>,
}

impl ReportValues {
    pub const NOTE_MAX_CHARS: usize = 2000;

    pub fn new(
        intensity: Intensity,
        mut rooms: Vec<RoomId>,
        mut symptoms: Vec<SymptomId>,
        note: Option<&str>,
    ) -> Result<Self, DomainError> {
        rooms.sort_unstable();
        rooms.dedup();
        symptoms.sort_unstable();
        symptoms.dedup();
        let note = note.map(str::trim).filter(|n| !n.is_empty());
        if let Some(n) = note {
            let chars = n.chars().count();
            if chars > Self::NOTE_MAX_CHARS {
                return Err(DomainError::NoteTooLong(chars));
            }
        }
        Ok(Self {
            intensity,
            rooms,
            symptoms,
            note: note.map(str::to_owned),
        })
    }

    pub fn intensity(&self) -> Intensity {
        self.intensity
    }

    pub fn rooms(&self) -> &[RoomId] {
        &self.rooms
    }

    pub fn symptoms(&self) -> &[SymptomId] {
        &self.symptoms
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SmellReport {
    pub id: ReportId,
    pub event_id: EventId,
    pub user_id: UserId,
    pub observed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    #[serde(flatten)]
    pub values: ReportValues,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl SmellReport {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

impl Audited for SmellReport {
    const ENTITY: AuditEntity = AuditEntity::SmellReport;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pm25Reading {
    pub id: ReadingId,
    pub user_id: UserId,
    pub event_id: Option<EventId>,
    pub observed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub value: Pm25,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Pm25Reading {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

impl Audited for Pm25Reading {
    const ENTITY: AuditEntity = AuditEntity::Pm25Reading;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
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

#[derive(Debug, Clone, PartialEq)]
pub struct NewReport {
    pub event_id: EventId,
    pub user_id: UserId,
    pub observed_at: DateTime<Utc>,
    pub values: ReportValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewReading {
    pub user_id: UserId,
    pub event_id: Option<EventId>,
    pub observed_at: DateTime<Utc>,
    pub value: Pm25,
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

    fn values(rooms: Vec<i64>, note: Option<&str>) -> ReportValues {
        ReportValues::new(
            Intensity::new(6).unwrap(),
            rooms.into_iter().map(RoomId).collect(),
            vec![SymptomId(2), SymptomId(1), SymptomId(2)],
            note,
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
    fn report_values_sort_and_dedup_ids_and_trim_the_note() {
        let v = values(vec![3, 1, 3], Some("  smell from the balcony "));
        assert_eq!(v.rooms(), [RoomId(1), RoomId(3)]);
        assert_eq!(v.symptoms(), [SymptomId(1), SymptomId(2)]);
        assert_eq!(v.note(), Some("smell from the balcony"));
    }

    #[test]
    fn a_blank_note_becomes_none() {
        assert_eq!(values(vec![], Some("   ")).note(), None);
        assert_eq!(values(vec![], None).note(), None);
    }

    #[test]
    fn notes_are_limited_in_characters_not_bytes() {
        let at_limit = "ç".repeat(ReportValues::NOTE_MAX_CHARS);
        assert!(
            ReportValues::new(Intensity::new(1).unwrap(), vec![], vec![], Some(&at_limit)).is_ok()
        );
        let over = "x".repeat(ReportValues::NOTE_MAX_CHARS + 1);
        assert_eq!(
            ReportValues::new(Intensity::new(1).unwrap(), vec![], vec![], Some(&over)),
            Err(DomainError::NoteTooLong(2001))
        );
    }

    #[test]
    fn report_snapshot_is_flat() {
        let report = SmellReport {
            id: ReportId(9),
            event_id: EventId(4),
            user_id: UserId(1),
            observed_at: t("2026-01-15T19:00:00Z"),
            recorded_at: t("2026-01-15T19:00:00.123456Z"),
            values: values(vec![2], None),
            deleted_at: None,
        };
        assert_eq!(
            report.snapshot(),
            json!({
                "id": 9,
                "event_id": 4,
                "user_id": 1,
                "observed_at": "2026-01-15T19:00:00Z",
                "recorded_at": "2026-01-15T19:00:00.123456Z",
                "intensity": 6,
                "rooms": [2],
                "symptoms": [1, 2],
                "note": null,
                "deleted_at": null,
            })
        );
        assert_eq!(report.audit_id(), 9);
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
    fn reading_snapshot_has_exactly_the_audited_fields() {
        let mut reading = Pm25Reading {
            id: ReadingId(7),
            user_id: UserId(1),
            event_id: Some(EventId(4)),
            observed_at: t("2026-01-15T19:00:00Z"),
            recorded_at: t("2026-01-15T19:00:00.5Z"),
            value: Pm25::new(12.5).unwrap(),
            deleted_at: None,
        };
        assert_eq!(
            reading.snapshot(),
            json!({
                "id": 7,
                "user_id": 1,
                "event_id": 4,
                "observed_at": "2026-01-15T19:00:00Z",
                "recorded_at": "2026-01-15T19:00:00.500Z",
                "value": 12.5,
                "deleted_at": null,
            })
        );
        reading.event_id = None;
        assert_eq!(reading.snapshot()["event_id"], json!(null));
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
        assert_eq!(SmellReport::ENTITY, AuditEntity::SmellReport);
        assert_eq!(Pm25Reading::ENTITY, AuditEntity::Pm25Reading);
    }

    #[test]
    fn user_converts_to_its_permission_actor() {
        let actor = user(None).actor();
        assert_eq!(actor.id, UserId(1));
        assert_eq!(actor.role, Role::Resident);
    }
}
