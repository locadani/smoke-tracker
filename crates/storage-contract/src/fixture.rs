//! Per-test database plumbing that backends provide, and test data shared by the suites.

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::audit::ChainedEntry;
use smoke_domain::clock::FakeClock;
use smoke_domain::records::{
    Closure, Event, LogEntry, NewEntry, NewEvent, NewUser, Note, PasswordHash, ReportValues, Room,
    RoomLevel, Symptom, User,
};
use smoke_domain::repos::{
    AuditRepo, Commit, EntryWrites, EventWrites, ListRepo, ListWrites, Transactional, UserWrites,
};
use smoke_domain::starter::seed_starter_lists;
use smoke_domain::types::{
    Level, Locale, Pm25, ReportId, Role, RoomId, SymptomId, UserId, Username,
};

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

/// Seeded lists, a resident, and an event they opened an hour before [`start`].
pub struct Scene {
    pub user: User,
    pub event: Event,
    pub rooms: Vec<Room>,
    pub symptoms: Vec<Symptom>,
}

pub async fn scene<S>(fx: &Fixture<S>) -> Scene
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites> + ListRepo,
{
    seed_starter_lists(&fx.store).await.expect("seed");
    let user = add_user(fx, "daniele", Role::Resident).await;
    let event = add_event(fx, user.id, start() - TimeDelta::hours(1), None).await;
    Scene {
        user,
        event,
        rooms: fx.store.rooms().await.expect("rooms"),
        symptoms: fx.store.symptoms().await.expect("symptoms"),
    }
}

/// The first starter room. Suites that log seed the starter lists first.
pub const FIRST_ROOM: RoomId = RoomId(1);

/// Report values with every room in `rooms` at `level`.
pub fn report_values(level: u8, rooms: &[RoomId], symptoms: &[SymptomId]) -> ReportValues {
    let level = Level::new(level).expect("valid level");
    let rooms = rooms
        .iter()
        .map(|&room| RoomLevel { room, level })
        .collect();
    ReportValues::new(rooms, symptoms.to_vec()).expect("valid values")
}

/// What tests ask of an entry they know has a report.
pub trait EntryExt {
    fn report_id(&self) -> ReportId;
    fn values(&self) -> &ReportValues;
}

impl EntryExt for LogEntry {
    fn report_id(&self) -> ReportId {
        self.report.as_ref().expect("the entry has a report").id
    }

    fn values(&self) -> &ReportValues {
        &self.report.as_ref().expect("the entry has a report").values
    }
}

/// Writes an entry in the scene's event, with whichever children and note are given.
pub async fn add_entry<S>(
    fx: &Fixture<S>,
    scene: &Scene,
    observed_at: DateTime<Utc>,
    note: Option<&str>,
    report: Option<ReportValues>,
    reading: Option<Pm25>,
) -> LogEntry
where
    S: Transactional<Tx: EntryWrites>,
{
    let new = NewEntry::new(
        scene.user.id,
        Some(scene.event.id),
        observed_at,
        Note::parse(note).expect("valid note"),
        report,
        reading,
    )
    .expect("an entry with a child");
    let mut tx = fx.store.begin(Some(scene.user.id)).await.expect("begin");
    let entry = tx.create_entry(new).await.expect("create entry");
    tx.commit().await.expect("commit");
    entry
}

/// An entry holding only a smell report.
pub async fn add_report<S>(
    fx: &Fixture<S>,
    scene: &Scene,
    observed_at: DateTime<Utc>,
    values: ReportValues,
) -> LogEntry
where
    S: Transactional<Tx: EntryWrites>,
{
    add_entry(fx, scene, observed_at, None, Some(values), None).await
}

/// Fails if any log entry has neither a report nor a reading: the domain keeps this rule, the
/// schema cannot. The insert violates a NOT NULL constraint only when such an entry exists.
pub async fn assert_no_childless_entries<S>(fx: &Fixture<S>) {
    fx.raw(
        "INSERT INTO jobs (name, last_run_at) \
         SELECT 'childless entry', NULL WHERE EXISTS ( \
             SELECT 1 FROM log_entries e \
             WHERE NOT EXISTS (SELECT 1 FROM smell_reports r WHERE r.log_entry_id = e.id) \
               AND NOT EXISTS (SELECT 1 FROM pm25_readings p WHERE p.log_entry_id = e.id))",
    )
    .await
    .expect("every log entry has a report or a reading");
}
