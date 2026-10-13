//! Transactions in general.

use smoke_domain::repos::{AuditRepo, Commit, Transactional};

use crate::Fixture;

pub async fn an_empty_transaction_commits<S: Transactional>(fx: &Fixture<S>) {
    let tx = fx.store.begin(None).await.expect("begin");
    tx.commit().await.expect("commit");
}

pub async fn the_audit_log_starts_empty<S: AuditRepo>(fx: &Fixture<S>) {
    assert!(fx.store.audit_log().await.expect("audit log").is_empty());
}
