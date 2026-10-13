//! `verify-audit` (spec §6.1): recomputes the hash chain and compares every record with its
//! latest audit entry. Read-only.

use std::fmt::Write;

use smoke_domain::repos::SnapshotRepo;
use smoke_domain::verify::{AuditReport, SnapshotProblem, verify_store};

pub async fn run<S: SnapshotRepo>(store: &S) -> anyhow::Result<AuditReport> {
    Ok(verify_store(store).await?)
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
