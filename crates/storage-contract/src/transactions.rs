//! Transactions in general.

use chrono::TimeDelta;
use smoke_domain::repos::{AuditRepo, Commit, JobRepo, JobWrites, Transactional};

use crate::Fixture;
use crate::fixture::start;

pub async fn an_empty_transaction_commits<S: Transactional>(fx: &Fixture<S>) {
    let tx = fx.store.begin(None).await.expect("begin");
    tx.commit().await.expect("commit");
}

pub async fn the_audit_log_starts_empty<S: AuditRepo>(fx: &Fixture<S>) {
    assert!(fx.store.audit_log().await.expect("audit log").is_empty());
}

pub async fn job_runs_are_remembered_but_not_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: JobWrites> + JobRepo + AuditRepo,
{
    const SWEEP: &str = "auto_close_sweep";
    assert_eq!(fx.store.job_last_run(SWEEP).await.unwrap(), None);
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.record_job_run(SWEEP, start()).await.unwrap();
    tx.record_job_run(SWEEP, start() + TimeDelta::days(1))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        fx.store.job_last_run(SWEEP).await.unwrap(),
        Some(start() + TimeDelta::days(1))
    );
    assert!(fx.store.audit_log().await.unwrap().is_empty());
}
