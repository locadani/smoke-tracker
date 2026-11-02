//! `verify-audit` (spec §6.1): recomputes the hash chain and compares every record with its
//! latest audit entry, then (when the photo store can be read) re-fingerprints every photo
//! file. Read-only.

use std::collections::BTreeSet;
use std::fmt::Write;

use sha2::{Digest, Sha256};
use smoke_domain::audit::AuditEntity;
use smoke_domain::records::PhotoKey;
use smoke_domain::repos::SnapshotRepo;
use smoke_domain::verify::{AuditReport, SnapshotProblem, verify_store};
use smoke_photos::{PhotoError, PhotoStore};

use crate::backend::BackendKind;
use crate::config::PhotoLocation;

pub async fn run<S: SnapshotRepo>(store: &S) -> anyhow::Result<AuditReport> {
    Ok(verify_store(store).await?)
}

/// What the photo files check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoCheck {
    NotChecked(String),
    Checked(Vec<PhotoProblem>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoProblem {
    /// The entry's file is gone, or its length or SHA-256 no longer match the audited record.
    ChangedOrMissing { entry_id: i64 },
    /// A file in the store that no entry (deleted ones included) points to.
    WithoutEntry { key: String },
    /// The store failed to read the entry's file (permissions, I/O).
    Unreadable { entry_id: i64, reason: String },
    /// The entry's stored photo key is not a valid key.
    InvalidReference { entry_id: i64 },
}

impl PhotoCheck {
    pub fn is_ok(&self) -> bool {
        match self {
            Self::NotChecked(_) => true,
            Self::Checked(problems) => problems.is_empty(),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Self::NotChecked(reason) => format!("photo files not checked ({reason})\n"),
            Self::Checked(problems) if problems.is_empty() => "photo files:   ok\n".to_owned(),
            Self::Checked(problems) => {
                let mut out = format!("photo files:   {} problem(s)\n", problems.len());
                for problem in problems {
                    let _ = match problem {
                        PhotoProblem::ChangedOrMissing { entry_id } => {
                            writeln!(out, "  photo file changed or missing: entry {entry_id}")
                        }
                        PhotoProblem::WithoutEntry { key } => {
                            writeln!(out, "  photo file without entry: {key}")
                        }
                        PhotoProblem::Unreadable { entry_id, reason } => {
                            writeln!(out, "  photo file unreadable: entry {entry_id} ({reason})")
                        }
                        PhotoProblem::InvalidReference { entry_id } => {
                            writeln!(out, "  photo reference invalid: entry {entry_id}")
                        }
                    };
                }
                out
            }
        }
    }
}

/// The store to check, or why there is none. Uses the same variables as `serve`. The
/// production database without `PHOTO_BUCKET` is not checked: an empty local folder would make
/// every photo look missing.
pub fn photo_store(kind: BackendKind, location: &PhotoLocation) -> Result<PhotoStore, String> {
    if kind == BackendKind::Postgres && !matches!(location, PhotoLocation::Gcs { .. }) {
        return Err("PHOTO_BUCKET is not set".to_owned());
    }
    location.build_store().map_err(|e| e.to_string())
}

/// Re-fingerprints the photo of every entry, deleted ones included, and looks for files no
/// entry points to. An unreachable store is "not checked", not a failure.
pub async fn check_photos<S: SnapshotRepo>(
    store: &S,
    photos: &PhotoStore,
) -> anyhow::Result<PhotoCheck> {
    let listed = match photos.list_photos().await {
        Ok(keys) => keys,
        Err(PhotoError::Unavailable(reason)) => return Ok(PhotoCheck::NotChecked(reason)),
        Err(error) => return Err(error.into()),
    };
    let state = store.audit_state().await?;
    let mut problems = Vec::new();
    let mut referenced = BTreeSet::new();
    for snapshot in state
        .snapshots
        .iter()
        .filter(|s| s.entity == AuditEntity::LogEntry)
    {
        let photo = &snapshot.value["reading"]["photo"];
        let Some(raw_key) = photo["object_key"].as_str() else {
            continue;
        };
        referenced.insert(raw_key.to_owned());
        let entry_id = snapshot.entity_id;
        let Ok(key) = PhotoKey::parse(raw_key) else {
            problems.push(PhotoProblem::InvalidReference { entry_id });
            continue;
        };
        match photos.get(&key).await {
            Ok(Some(file)) => {
                let intact = photo["bytes"].as_u64() == Some(file.len() as u64)
                    && photo["sha256"].as_str()
                        == Some(hex::encode(Sha256::digest(&file)).as_str());
                if !intact {
                    problems.push(PhotoProblem::ChangedOrMissing { entry_id });
                }
            }
            Ok(None) | Err(PhotoError::Missing | PhotoError::Mismatch) => {
                problems.push(PhotoProblem::ChangedOrMissing { entry_id });
            }
            Err(error) => problems.push(PhotoProblem::Unreadable {
                entry_id,
                reason: error.to_string(),
            }),
        }
    }
    for key in listed {
        if !referenced.contains(key.as_str()) {
            problems.push(PhotoProblem::WithoutEntry {
                key: key.as_str().to_owned(),
            });
        }
    }
    Ok(PhotoCheck::Checked(problems))
}

/// The report as printed: the head hash to compare with the one pinned in backups, then each
/// check's outcome.
pub fn describe(report: &AuditReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "audit entries: {}", report.entries);
    let _ = writeln!(out, "head hash:     {}", report.head_hash);
    match &report.chain {
        Ok(()) => out.push_str("hash chain:    ok\n"),
        Err(error) => {
            let _ = writeln!(out, "hash chain:    BROKEN: {error}");
        }
    }
    if report.problems.is_empty() {
        out.push_str("records:       ok\n");
    } else {
        let _ = writeln!(out, "records:       {} problem(s)", report.problems.len());
        for problem in &report.problems {
            let _ = writeln!(out, "  {}", explain(problem));
        }
    }
    out
}

fn explain(problem: &SnapshotProblem) -> String {
    match problem {
        SnapshotProblem::Unaudited { entity, entity_id } => {
            format!("{entity} {entity_id}: has no audit entry (inserted outside the app)")
        }
        SnapshotProblem::Vanished { entity, entity_id } => {
            format!("{entity} {entity_id}: is audited but missing (deleted outside the app)")
        }
        SnapshotProblem::Differs {
            entity,
            entity_id,
            audit_id,
        } => format!(
            "{entity} {entity_id}: differs from audit entry {audit_id} (edited outside the app)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use smoke_domain::audit::{AuditEntity, ChainError, GENESIS_HASH};

    use super::*;

    fn report(chain: Result<(), ChainError>, problems: Vec<SnapshotProblem>) -> AuditReport {
        AuditReport {
            entries: 3,
            head_hash: "ab12".into(),
            chain,
            problems,
        }
    }

    #[test]
    fn a_clean_report_says_ok_twice() {
        assert_eq!(
            describe(&report(Ok(()), vec![])),
            "audit entries: 3\nhead hash:     ab12\nhash chain:    ok\nrecords:       ok\n"
        );
        let empty = AuditReport {
            entries: 0,
            head_hash: GENESIS_HASH.into(),
            chain: Ok(()),
            problems: vec![],
        };
        assert!(describe(&empty).contains(GENESIS_HASH));
    }

    #[test]
    fn every_problem_is_listed_with_its_likely_cause() {
        let text = describe(&report(
            Err(ChainError::HashMismatch { id: 2 }),
            vec![
                SnapshotProblem::Unaudited {
                    entity: AuditEntity::User,
                    entity_id: 3,
                },
                SnapshotProblem::Vanished {
                    entity: AuditEntity::Room,
                    entity_id: 2,
                },
                SnapshotProblem::Differs {
                    entity: AuditEntity::SmellReport,
                    entity_id: 4,
                    audit_id: 9,
                },
            ],
        ));
        assert!(
            text.contains(
                "hash chain:    BROKEN: audit entry 2: its content does not match its hash"
            )
        );
        assert!(text.contains("records:       3 problem(s)"));
        assert!(text.contains("  user 3: has no audit entry (inserted outside the app)"));
        assert!(text.contains("  room 2: is audited but missing (deleted outside the app)"));
        assert!(
            text.contains("  smell_report 4: differs from audit entry 9 (edited outside the app)")
        );
    }
}
