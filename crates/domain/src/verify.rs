//! What `verify-audit` checks: the hash chain is intact, and every record equals its latest
//! audited snapshot, which catches edits made directly in the database (spec §6.1).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::audit::{AuditEntity, ChainError, ChainedEntry, GENESIS_HASH, verify_chain};
use crate::records::{Audited, SmellReport};
use crate::repos::{SnapshotRepo, StoreResult};

/// The audit log and every audited record, read at one moment.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditState {
    pub entries: Vec<ChainedEntry>,
    pub snapshots: Vec<Snapshot>,
}

/// A record as it is stored now.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub entity: AuditEntity,
    pub entity_id: i64,
    pub value: Value,
}

impl Snapshot {
    pub fn of<T: Audited>(record: &T) -> Self {
        Self {
            entity: T::ENTITY,
            entity_id: record.audit_id(),
            value: record.snapshot(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotProblem {
    /// A record exists that no audit entry describes: it was inserted directly.
    Unaudited { entity: AuditEntity, entity_id: i64 },
    /// The audit log describes a record that no longer exists: it was deleted directly.
    Vanished { entity: AuditEntity, entity_id: i64 },
    /// The record differs from its latest audited snapshot: it was edited directly.
    Differs {
        entity: AuditEntity,
        entity_id: i64,
        audit_id: i64,
    },
}

impl SnapshotProblem {
    pub fn key(&self) -> (&'static str, i64) {
        match *self {
            Self::Unaudited { entity, entity_id }
            | Self::Vanished { entity, entity_id }
            | Self::Differs {
                entity, entity_id, ..
            } => (entity.as_str(), entity_id),
        }
    }
}

/// The outcome of a full verification.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditReport {
    pub entries: usize,
    /// The latest hash, to compare with the one pinned in backups.
    pub head_hash: String,
    pub chain: Result<(), ChainError>,
    pub problems: Vec<SnapshotProblem>,
}

impl AuditReport {
    pub fn is_ok(&self) -> bool {
        self.chain.is_ok() && self.problems.is_empty()
    }
}

/// Compares the current records with the latest audit entry of each.
pub fn verify_snapshots(entries: &[ChainedEntry], current: &[Snapshot]) -> Vec<SnapshotProblem> {
    let mut latest: BTreeMap<(&'static str, i64), &ChainedEntry> = BTreeMap::new();
    for entry in entries.iter().filter(|e| e.record.entity.has_snapshot()) {
        latest.insert(
            (entry.record.entity.as_str(), entry.record.entity_id),
            entry,
        );
    }

    let mut problems = Vec::new();
    let mut present = BTreeSet::new();
    for snapshot in current.iter().filter(|s| s.entity.has_snapshot()) {
        let key = (snapshot.entity.as_str(), snapshot.entity_id);
        present.insert(key);
        match latest.get(&key) {
            None => problems.push(SnapshotProblem::Unaudited {
                entity: snapshot.entity,
                entity_id: snapshot.entity_id,
            }),
            Some(entry) if !matches_audited(entry, &snapshot.value) => {
                problems.push(SnapshotProblem::Differs {
                    entity: snapshot.entity,
                    entity_id: snapshot.entity_id,
                    audit_id: entry.record.id,
                })
            }
            Some(_) => {}
        }
    }
    for (key, entry) in &latest {
        if !present.contains(key) && entry.record.after.is_some() {
            problems.push(SnapshotProblem::Vanished {
                entity: entry.record.entity,
                entity_id: entry.record.entity_id,
            });
        }
    }
    problems.sort_by_key(SnapshotProblem::key);
    problems
}

/// Whether `current` equals the entry's `after` snapshot. Report snapshots audited before rooms
/// had their own levels are compared in today's shape (decision #104).
fn matches_audited(entry: &ChainedEntry, current: &Value) -> bool {
    match &entry.record.after {
        Some(after) if entry.record.entity == AuditEntity::SmellReport => {
            *SmellReport::upgrade_snapshot(after) == *current
        }
        Some(after) => after == current,
        None => false,
    }
}

/// Runs both checks against a store.
pub async fn verify_store<S: SnapshotRepo>(store: &S) -> StoreResult<AuditReport> {
    let AuditState {
        entries,
        snapshots: current,
    } = store.audit_state().await?;
    Ok(AuditReport {
        entries: entries.len(),
        head_hash: entries
            .last()
            .map_or(GENESIS_HASH, |e| e.hash.as_str())
            .to_owned(),
        chain: verify_chain(&entries),
        problems: verify_snapshots(&entries, &current),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::audit::{AuditAction, AuditRecord, ChainError};

    /// A chain built from `(entity, entity_id, action, after)` steps.
    fn chain(steps: &[(AuditEntity, i64, AuditAction, Value)]) -> Vec<ChainedEntry> {
        let mut entries: Vec<ChainedEntry> = Vec::new();
        for (i, (entity, entity_id, action, after)) in steps.iter().enumerate() {
            let prev = entries
                .last()
                .map_or(GENESIS_HASH, |e| e.hash.as_str())
                .to_owned();
            let record = AuditRecord {
                id: i as i64 + 1,
                at_micros: 1_768_500_000_000_000,
                actor_id: None,
                entity: *entity,
                entity_id: *entity_id,
                action: *action,
                before: None,
                after: Some(after.clone()),
            };
            entries.push(ChainedEntry::seal(record, &prev));
        }
        entries
    }

    fn snap(entity: AuditEntity, entity_id: i64, value: Value) -> Snapshot {
        Snapshot {
            entity,
            entity_id,
            value,
        }
    }

    #[test]
    fn records_matching_their_latest_entry_have_no_problems() {
        let entries = chain(&[
            (
                AuditEntity::Room,
                1,
                AuditAction::Create,
                json!({ "name": "Kitchen" }),
            ),
            (
                AuditEntity::Room,
                1,
                AuditAction::Update,
                json!({ "name": "Kitchenette" }),
            ),
        ]);
        let current = [snap(AuditEntity::Room, 1, json!({ "name": "Kitchenette" }))];
        assert_eq!(verify_snapshots(&entries, &current), []);
    }

    #[test]
    fn a_record_edited_outside_the_app_differs() {
        let entries = chain(&[(
            AuditEntity::SmellReport,
            7,
            AuditAction::Create,
            json!({ "rooms": [{ "room": 1, "level": 6 }] }),
        )]);
        let current = [snap(
            AuditEntity::SmellReport,
            7,
            json!({ "rooms": [{ "room": 1, "level": 9 }] }),
        )];
        assert_eq!(
            verify_snapshots(&entries, &current),
            [SnapshotProblem::Differs {
                entity: AuditEntity::SmellReport,
                entity_id: 7,
                audit_id: 1
            }]
        );
    }

    #[test]
    fn a_report_audited_before_room_levels_still_matches_its_converted_record() {
        let entries = chain(&[(
            AuditEntity::SmellReport,
            7,
            AuditAction::Create,
            json!({ "intensity": 6, "rooms": [1, 3] }),
        )]);
        let converted = json!({ "rooms": [{ "room": 1, "level": 6 }, { "room": 3, "level": 6 }] });
        let current = [snap(AuditEntity::SmellReport, 7, converted)];
        assert_eq!(verify_snapshots(&entries, &current), []);

        let edited = json!({ "rooms": [{ "room": 1, "level": 6 }, { "room": 3, "level": 9 }] });
        let current = [snap(AuditEntity::SmellReport, 7, edited)];
        assert_eq!(verify_snapshots(&entries, &current).len(), 1);
    }

    #[test]
    fn a_record_without_any_entry_is_unaudited() {
        let current = [snap(AuditEntity::Symptom, 3, json!({}))];
        assert_eq!(
            verify_snapshots(&[], &current),
            [SnapshotProblem::Unaudited {
                entity: AuditEntity::Symptom,
                entity_id: 3
            }]
        );
    }

    #[test]
    fn an_audited_record_that_is_gone_has_vanished() {
        let entries = chain(&[(AuditEntity::Pm25Reading, 2, AuditAction::Create, json!({}))]);
        assert_eq!(
            verify_snapshots(&entries, &[]),
            [SnapshotProblem::Vanished {
                entity: AuditEntity::Pm25Reading,
                entity_id: 2
            }]
        );
    }

    #[test]
    fn invites_are_not_compared() {
        let entries = chain(&[(AuditEntity::Invite, 1, AuditAction::Invite, json!({}))]);
        assert_eq!(verify_snapshots(&entries, &[]), []);
    }

    #[test]
    fn problems_are_sorted_by_entity_then_id() {
        let current = [
            snap(AuditEntity::User, 2, json!({})),
            snap(AuditEntity::Event, 9, json!({})),
            snap(AuditEntity::User, 1, json!({})),
        ];
        let ids: Vec<_> = verify_snapshots(&[], &current)
            .iter()
            .map(SnapshotProblem::key)
            .collect();
        assert_eq!(ids, [("event", 9), ("user", 1), ("user", 2)]);
    }

    #[test]
    fn a_report_is_ok_only_with_an_intact_chain_and_no_problems() {
        let ok = AuditReport {
            entries: 0,
            head_hash: GENESIS_HASH.to_owned(),
            chain: Ok(()),
            problems: vec![],
        };
        assert!(ok.is_ok());
        let broken = AuditReport {
            chain: Err(ChainError::BrokenLink { id: 2 }),
            ..ok.clone()
        };
        assert!(!broken.is_ok());
        let tampered = AuditReport {
            problems: vec![SnapshotProblem::Unaudited {
                entity: AuditEntity::Room,
                entity_id: 1,
            }],
            ..ok
        };
        assert!(!tampered.is_ok());
    }
}
