//! A smell report's History view (spec §6.1): what each audit entry changed, e.g.
//! "intensity 6 → 7, added bedroom".

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::audit::{AuditAction, ChainedEntry};
use crate::types::{EventId, RoomId, SymptomId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    ObservedAt {
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    },
    Intensity {
        from: u8,
        to: u8,
    },
    RoomAdded(RoomId),
    RoomRemoved(RoomId),
    SymptomAdded(SymptomId),
    SymptomRemoved(SymptomId),
    Note {
        from: Option<String>,
        to: Option<String>,
    },
    Moved {
        from: EventId,
        to: EventId,
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

/// The audited fields of a smell report, as `SmellReport` serialises them.
#[derive(Deserialize)]
struct Snapshot {
    event_id: EventId,
    observed_at: DateTime<Utc>,
    intensity: u8,
    rooms: Vec<RoomId>,
    symptoms: Vec<SymptomId>,
    note: Option<String>,
}

/// One report's audit entries (oldest first, from `AuditRepo::audit_history`) as history.
pub fn report_history(entries: &[ChainedEntry]) -> Vec<HistoryEntry> {
    entries
        .iter()
        .map(|entry| {
            let record = &entry.record;
            let snapshot = |value: &Option<serde_json::Value>| -> Option<Snapshot> {
                serde_json::from_value(value.clone()?).ok()
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
    if before.intensity != after.intensity {
        changes.push(Change::Intensity {
            from: before.intensity,
            to: after.intensity,
        });
    }
    changes.extend(
        added(&before.rooms, &after.rooms)
            .into_iter()
            .map(Change::RoomAdded),
    );
    changes.extend(
        added(&after.rooms, &before.rooms)
            .into_iter()
            .map(Change::RoomRemoved),
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
    use crate::records::{Audited, ReportValues, SmellReport};
    use crate::types::{Intensity, ReportId};

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn report(intensity: u8, rooms: &[i64]) -> SmellReport {
        SmellReport {
            id: ReportId(9),
            event_id: EventId(4),
            user_id: UserId(1),
            observed_at: t("2026-01-15T19:00:00Z"),
            recorded_at: t("2026-01-15T19:00:01Z"),
            values: ReportValues::new(
                Intensity::new(intensity).unwrap(),
                rooms.iter().copied().map(RoomId).collect(),
                vec![SymptomId(1)],
                None,
            )
            .unwrap(),
            deleted_at: None,
        }
    }

    fn entry(
        action: AuditAction,
        before: Option<&SmellReport>,
        after: &SmellReport,
    ) -> ChainedEntry {
        ChainedEntry::seal(
            AuditRecord {
                id: 1,
                at_micros: t("2026-01-16T08:15:00Z").timestamp_micros(),
                actor_id: Some(UserId(2)),
                entity: AuditEntity::SmellReport,
                entity_id: 9,
                action,
                before: before.map(Audited::snapshot),
                after: Some(after.snapshot()),
            },
            GENESIS_HASH,
        )
    }

    #[test]
    fn a_correction_lists_each_changed_value() {
        let before = report(6, &[1]);
        let mut after = before.clone();
        after.values = ReportValues::new(
            Intensity::new(7).unwrap(),
            vec![RoomId(1), RoomId(3)],
            vec![],
            Some("from the balcony"),
        )
        .unwrap();
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
                Change::Intensity { from: 6, to: 7 },
                Change::RoomAdded(RoomId(3)),
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
        let before = report(6, &[1]);
        let mut after = before.clone();
        after.event_id = EventId(5);
        let history = report_history(&[
            entry(AuditAction::Create, None, &before),
            entry(AuditAction::Move, Some(&before), &after),
        ]);
        assert_eq!(
            history[1].changes,
            [Change::Moved {
                from: EventId(4),
                to: EventId(5),
            }]
        );
        assert!(!was_corrected(&history));
    }

    #[test]
    fn a_deletion_has_no_value_changes() {
        let before = report(6, &[1]);
        let mut after = before.clone();
        after.deleted_at = Some(t("2026-01-16T08:15:00Z"));
        let history = report_history(&[entry(AuditAction::Delete, Some(&before), &after)]);
        assert_eq!(history[0].action, AuditAction::Delete);
        assert!(history[0].changes.is_empty());
    }
}
