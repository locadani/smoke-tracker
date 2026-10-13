//! Per-test database plumbing that backends provide, and test data shared by the suites.

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::audit::ChainedEntry;
use smoke_domain::clock::FakeClock;
use smoke_domain::records::{Closure, Event, NewEvent, NewUser, PasswordHash, User};
use smoke_domain::repos::{AuditRepo, Commit, EventWrites, Transactional, UserWrites};
use smoke_domain::types::{Locale, Role, UserId, Username};

/// Runs SQL directly against the test database, bypassing the store: how tests tamper.
pub type RawSql = Box<
    dyn Fn(&'static str) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> + Send + Sync,
>;

pub fn raw_runner<F, Fut>(run: F) -> RawSql
where
    F: Fn(&'static str) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    Box::new(move |sql| Box::pin(run(sql)))
}

/// A migrated, empty database behind a store, with a fake clock set to [`start`].
pub struct Fixture<S> {
    pub store: S,
    pub clock: Arc<FakeClock>,
    raw: RawSql,
    drop_audit_guard: &'static str,
    _keep_alive: Box<dyn Any + Send + Sync>,
}

impl<S> Fixture<S> {
    /// `drop_audit_guard` is the backend's SQL that removes the append-only triggers on
    /// `audit_log`, as someone with database access could. `keep_alive` lives as long as the
    /// fixture (e.g. the temporary directory of a SQLite file).
    pub fn new(
        store: S,
        clock: Arc<FakeClock>,
        raw: RawSql,
        drop_audit_guard: &'static str,
        keep_alive: impl Any + Send + Sync,
    ) -> Self {
        Self {
            store,
            clock,
            raw,
            drop_audit_guard,
            _keep_alive: Box::new(keep_alive),
        }
    }

    pub async fn raw(&self, sql: &'static str) -> Result<(), String> {
        (self.raw)(sql).await
    }

    pub async fn drop_audit_guard(&self) {
        self.raw(self.drop_audit_guard)
            .await
            .expect("drop the audit_log triggers");
    }
}

/// Where the fake clock starts in every contract test.
pub fn start() -> DateTime<Utc> {
    "2026-01-15T20:00:00Z".parse().expect("valid instant")
}

/// The password hash every test user gets; it must never appear in the audit log.
pub const PASSWORD: &str = "test-password-hash";

/// Guests expire a week after [`start`]; residents and viewers never.
pub fn new_user(name: &str, role: Role) -> NewUser {
    NewUser {
        username: Username::new(name).expect("valid username"),
        password_hash: Some(PasswordHash::new(PASSWORD.to_owned())),
        role,
        locale: Locale::En,
        expires_at: (role == Role::Guest).then(|| start() + TimeDelta::days(7)),
    }
}

pub async fn add_user<S>(fx: &Fixture<S>, name: &str, role: Role) -> User
where
    S: Transactional<Tx: UserWrites>,
{
    let mut tx = fx.store.begin(None).await.expect("begin");
    let user = tx
        .create_user(new_user(name, role))
        .await
        .expect("create user");
    tx.commit().await.expect("commit");
    user
}

pub async fn last_entry<S: AuditRepo>(fx: &Fixture<S>) -> ChainedEntry {
    fx.store
        .audit_log()
        .await
        .expect("audit log")
        .pop()
        .expect("at least one audit entry")
}

pub async fn add_event<S>(
    fx: &Fixture<S>,
    owner: UserId,
    opened_at: DateTime<Utc>,
    closed: Option<Closure>,
) -> Event
where
    S: Transactional<Tx: EventWrites>,
{
    let mut tx = fx.store.begin(Some(owner)).await.expect("begin");
    let event = tx
        .create_event(NewEvent {
            owner_id: owner,
            opened_at,
            closed,
        })
        .await
        .expect("create event");
    tx.commit().await.expect("commit");
    event
}
