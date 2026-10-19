//! A smell report's History view (spec §6.1): what each audit entry changed, e.g.
//! "living room 6 → 7, added bedroom 3".

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::audit::{AuditAction, ChainedEntry};
use crate::records::{RoomLevel, SmellReport};
use crate::types::{EventId, Level, RoomId, SymptomId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    ObservedAt {
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    },
    /// A room listed before and after, at another level.
    LevelChanged {
        room: RoomId,
        from: Level,
        to: Level,
    },
    RoomAdded(RoomLevel),
    RoomRemoved(RoomId),
    SymptomAdded(SymptomId),
    SymptomRemoved(SymptomId),
    Note {
        from: Option<String>,
        to: Option<String>,
    },
    /// Into another event; `None` is no event (a no-smell check).
    Moved {
        from: Option<EventId>,
        to: Option<EventId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub at: DateTime<Utc>,
    pub actor: Option<UserId>,
    pub action: AuditAction,
    /// Empty for the creation and for a deletion.
    pub changes: Vec<Change>,
}

/// The audited fields of a smell report in one flat shape: what a report's own audit entries
/// held. A log entry's whole-log snapshot is read through [`flat_report`], older report
/// snapshots through [`SmellReport::upgrade_snapshot`].
#[derive(Deserialize)]
struct Snapshot {
    event_id: Option<EventId>,
    observed_at: DateTime<Utc>,
    rooms: Vec<RoomLevel>,
    symptoms: Vec<SymptomId>,
    note: Option<String>,
}

/// The report's fields of a snapshot, flat. A log entry's snapshot (`"report": {...}` next to
/// the entry's own fields) is flattened; a snapshot without a report has none.
fn flat_report(value: &serde_json::Value) -> Option<serde_json::Value> {
    let Some(report) = value.get("report") else {
        return Some(SmellReport::upgrade_snapshot(value).into_owned());
    };
    let report = report.as_object()?;
    Some(serde_json::json!({
        "event_id": value.get("event_id"),
        "observed_at": value.get("observed_at"),
        "rooms": report.get("rooms"),
        "symptoms": report.get("symptoms"),
        "note": value.get("note"),
    }))
}

/// One report's audit entries (oldest first, from `AuditRepo::audit_history`) as history.
pub fn report_history(entries: &[ChainedEntry]) -> Vec<HistoryEntry> {
    entries
        .iter()
        .map(|entry| {
            let record = &entry.record;
            let snapshot = |value: &Option<serde_json::Value>| -> Option<Snapshot> {
                serde_json::from_value(flat_report(value.as_ref()?)?).ok()
            };
            let changes = match (snapshot(&record.before), snapshot(&record.after)) {
                (Some(before), Some(after)) => changes(&before, &after),
                _ => Vec::new(),
            };
            HistoryEntry {
                at: DateTime::from_timestamp_micros(record.at_micros)
                    .expect("audit instants are in range"),
                actor: record.actor_id,
                action: record.action,
                changes,
            }
        })
        .collect()
}

/// Whether the report was ever corrected: what its "edited" badge shows.
pub fn was_corrected(history: &[HistoryEntry]) -> bool {
    history.iter().any(|e| e.action == AuditAction::Update)
}

/// What `new` has that `old` lacks.
fn added<T: PartialEq + Copy>(old: &[T], new: &[T]) -> Vec<T> {
    new.iter().filter(|id| !old.contains(id)).copied().collect()
}

fn changes(before: &Snapshot, after: &Snapshot) -> Vec<Change> {
    let mut changes = Vec::new();
    if before.event_id != after.event_id {
        changes.push(Change::Moved {
            from: before.event_id,
            to: after.event_id,
        });
    }
    if before.observed_at != after.observed_at {
        changes.push(Change::ObservedAt {
            from: before.observed_at,
            to: after.observed_at,
        });
    }
    let level_in =
        |rooms: &[RoomLevel], room: RoomId| rooms.iter().find(|r| r.room == room).map(|r| r.level);
    for now in &after.rooms {
        if let Some(was) = level_in(&before.rooms, now.room)
            && was != now.level
        {
            changes.push(Change::LevelChanged {
                room: now.room,
                from: was,
                to: now.level,
            });
        }
    }
    changes.extend(
        after
            .rooms
            .iter()
            .filter(|now| level_in(&before.rooms, now.room).is_none())
            .map(|now| Change::RoomAdded(*now)),
    );
    changes.extend(
        before
            .rooms
            .iter()
            .filter(|was| level_in(&after.rooms, was.room).is_none())
            .map(|was| Change::RoomRemoved(was.room)),
    );
    changes.extend(
        added(&before.symptoms, &after.symptoms)
            .into_iter()
            .map(Change::SymptomAdded),
    );
    changes.extend(
        added(&after.symptoms, &before.symptoms)
            .into_iter()
            .map(Change::SymptomRemoved),
    );
    if before.note != after.note {
        changes.push(Change::Note {
            from: before.note.clone(),
            to: after.note.clone(),
        });
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditEntity, AuditRecord, GENESIS_HASH};
    use crate::records::{Audited, LogEntry, Note, ReportValues};
    use crate::types::{LogEntryId, ReportId};

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn at(room: i64, level: u8) -> RoomLevel {
        RoomLevel {
            room: RoomId(room),
            level: Level::new(level).unwrap(),
        }
    }

    fn report(rooms: &[(i64, u8)]) -> LogEntry {
        LogEntry {
            id: LogEntryId(3),
            user_id: UserId(1),
            event_id: Some(EventId(4)),
            observed_at: t("2026-01-15T19:00:00Z"),
            recorded_at: t("2026-01-15T19:00:01Z"),
            note: None,
            deleted_at: None,
            report: Some(SmellReport {
                id: ReportId(9),
                log_entry_id: LogEntryId(3),
                values: ReportValues::new(
                    rooms.iter().map(|&(room, level)| at(room, level)).collect(),
                    vec![SymptomId(1)],
                )
                .unwrap(),
            }),
            reading: None,
        }
    }

    fn entry(action: AuditAction, before: Option<&LogEntry>, after: &LogEntry) -> ChainedEntry {
        ChainedEntry::seal(
            AuditRecord {
                id: 1,
                at_micros: t("2026-01-16T08:15:00Z").timestamp_micros(),
                actor_id: Some(UserId(2)),
                entity: AuditEntity::LogEntry,
                entity_id: 3,
                action,
                before: before.map(Audited::snapshot),
                after: Some(after.snapshot()),
            },
            GENESIS_HASH,
        )
    }

    #[test]
    fn a_correction_lists_each_changed_value() {
        let before = report(&[(2, 4), (3, 6)]);
        let mut after = before.clone();
        after.report.as_mut().unwrap().values =
            ReportValues::new(vec![at(1, 3), at(3, 7)], vec![]).unwrap();
        after.note = Note::parse(Some("from the balcony")).unwrap();
        after.observed_at = t("2026-01-15T19:30:00Z");
        let history = report_history(&[
            entry(AuditAction::Create, None, &before),
            entry(AuditAction::Update, Some(&before), &after),
        ]);
        assert_eq!(history[0].changes, []);
        assert_eq!(history[1].at, t("2026-01-16T08:15:00Z"));
        assert_eq!(history[1].actor, Some(UserId(2)));
        assert_eq!(
            history[1].changes,
            [
                Change::ObservedAt {
                    from: t("2026-01-15T19:00:00Z"),
                    to: t("2026-01-15T19:30:00Z"),
                },
                Change::LevelChanged {
                    room: RoomId(3),
                    from: Level::new(6).unwrap(),
                    to: Level::new(7).unwrap(),
                },
                Change::RoomAdded(at(1, 3)),
                Change::RoomRemoved(RoomId(2)),
                Change::SymptomRemoved(SymptomId(1)),
                Change::Note {
                    from: None,
                    to: Some("from the balcony".to_owned()),
                },
            ]
        );
        assert!(was_corrected(&history));
    }

    #[test]
    fn a_move_names_both_events_and_is_not_a_correction() {
        let before = report(&[(1, 6)]);
        let mut after = before.clone();
        after.event_id = Some(EventId(5));
        let history = report_history(&[
            entry(AuditAction::Create, None, &before),
            entry(AuditAction::Move, Some(&before), &after),
        ]);
        assert_eq!(
            history[1].changes,
            [Change::Moved {
                from: Some(EventId(4)),
                to: Some(EventId(5)),
            }]
        );
        assert!(!was_corrected(&history));
    }

    #[test]
    fn a_correction_audited_before_room_levels_compares_every_room_at_its_intensity() {
        let mut old = entry(
            AuditAction::Update,
            Some(&report(&[(1, 6)])),
            &report(&[(1, 6)]),
        );
        old.record.before = Some(serde_json::json!({
            "event_id": 4, "observed_at": "2026-01-15T19:00:00Z",
            "intensity": 6, "rooms": [1], "symptoms": [1], "note": null,
        }));
        old.record.after = Some(serde_json::json!({
            "event_id": 4, "observed_at": "2026-01-15T19:00:00Z",
            "intensity": 7, "rooms": [1, 3], "symptoms": [1], "note": null,
        }));
        assert_eq!(
            report_history(&[old])[0].changes,
            [
                Change::LevelChanged {
                    room: RoomId(1),
                    from: Level::new(6).unwrap(),
                    to: Level::new(7).unwrap(),
                },
                Change::RoomAdded(at(3, 7)),
            ]
        );
    }

    #[test]
    fn a_deletion_has_no_value_changes() {
        let before = report(&[(1, 6)]);
        let mut after = before.clone();
        after.deleted_at = Some(t("2026-01-16T08:15:00Z"));
        let history = report_history(&[entry(AuditAction::Delete, Some(&before), &after)]);
        assert_eq!(history[0].action, AuditAction::Delete);
        assert!(history[0].changes.is_empty());
    }
}
