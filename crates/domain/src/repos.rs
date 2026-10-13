//! The storage boundary: traits the database crates implement and the rest of the app uses.
//!
//! Reads go through the store. Writes go through a transaction from [`Transactional::begin`]:
//! every write method appends its own audit entry inside that transaction, so data cannot change
//! without being recorded. Dropping a transaction without [`Commit::commit`] rolls it back.

use std::error::Error as StdError;
use std::future::Future;

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::DomainError;
use crate::audit::{AuditEntity, ChainedEntry};
use crate::records::{
    Closure, Event, Invite, NewEvent, NewInvite, NewReading, NewReport, NewRoom, NewSymptom,
    NewUser, PasswordHash, Pm25Reading, ReportValues, Room, SmellReport, StoredSession, Symptom,
    User,
};
use crate::types::{
    EventId, InviteId, Locale, Pm25, ReadingId, ReportId, RoomId, SymptomId, TokenHash, UserId,
    Username,
};
use crate::verify::AuditState;

/// A write the current data does not allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Conflict {
    #[error("that username is taken")]
    UsernameTaken,
    #[error("another event is already open")]
    AnotherEventOpen,
    #[error("the event is not open")]
    EventNotOpen,
    #[error("the event is not closed")]
    EventNotClosed,
    #[error("the entry has been deleted")]
    AlreadyDeleted,
    #[error("a referenced user, event, room or symptom does not exist")]
    UnknownReference,
    #[error("an identical entry already exists")]
    Duplicate,
    #[error("another write extended the audit log at the same time")]
    ChainForked,
    #[error("the change breaks a data rule")]
    RuleViolated,
    #[error("the invite has already been used")]
    InviteUsed,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("{entity} {id} does not exist")]
    NotFound { entity: AuditEntity, id: i64 },
    #[error(transparent)]
    Conflict(#[from] Conflict),
    #[error(transparent)]
    Invalid(#[from] DomainError),
    #[error("stored data is corrupt: {0}")]
    Corrupt(String),
    #[error("database error")]
    Backend(#[source] Box<dyn StdError + Send + Sync>),
}

pub type StoreResult<T> = Result<T, StoreError>;

/// A store that can start write transactions.
pub trait Transactional: Send + Sync {
    type Tx: Commit;

    /// Starts a write transaction whose audit entries name `actor` (`None` for the system:
    /// the sweep, seeding, the CLI). Write transactions run one at a time, so do not call
    /// `begin` again while holding one on the same task: it waits for the first to finish.
    ///
    /// After any error from a write method, drop the transaction instead of committing it: a
    /// failed write may have applied part of its statements, and on Postgres the transaction is
    /// aborted.
    fn begin(&self, actor: Option<UserId>) -> impl Future<Output = StoreResult<Self::Tx>> + Send;
}

pub trait Commit: Send {
    fn commit(self) -> impl Future<Output = StoreResult<()>> + Send;
}

pub trait UserRepo: Send + Sync {
    fn user(&self, id: UserId) -> impl Future<Output = StoreResult<Option<User>>> + Send;
    fn user_by_username(
        &self,
        username: &Username,
    ) -> impl Future<Output = StoreResult<Option<User>>> + Send;
    /// Every user, ordered by username.
    fn users(&self) -> impl Future<Output = StoreResult<Vec<User>>> + Send;
}

pub trait InviteRepo: Send + Sync {
    /// The invite whose token has this hash, used or expired ones included.
    fn invite_by_token(
        &self,
        token: &TokenHash,
    ) -> impl Future<Output = StoreResult<Option<Invite>>> + Send;
    /// The daily cleanup (spec §11): removes every invite that expired by `now`, used or not,
    /// and returns how many there were. Not audited: invites are not evidence and have no
    /// snapshot (decision #112).
    fn delete_expired_invites(
        &self,
        now: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<u64>> + Send;
}

pub trait ListRepo: Send + Sync {
    /// Every room, hidden ones included, ordered by id.
    fn rooms(&self) -> impl Future<Output = StoreResult<Vec<Room>>> + Send;
    /// Every symptom, hidden ones included, ordered by id.
    fn symptoms(&self) -> impl Future<Output = StoreResult<Vec<Symptom>>> + Send;
}

pub trait EventRepo: Send + Sync {
    fn event(&self, id: EventId) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
    fn open_event(&self) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
    /// Events whose span overlaps `[from, to)`, an open event counting as still running;
    /// ordered by `opened_at`.
    fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<Vec<Event>>> + Send;
    /// Events closed by the 04:00 sweep whose notice nobody has seen yet, ordered by
    /// `opened_at`.
    fn unseen_auto_closed(&self) -> impl Future<Output = StoreResult<Vec<Event>>> + Send;
}

pub trait ReportRepo: Send + Sync {
    /// A report, deleted or not.
    fn report(&self, id: ReportId)
    -> impl Future<Output = StoreResult<Option<SmellReport>>> + Send;
    /// The event's reports that are not deleted, ordered by observed time.
    fn reports_for_event(
        &self,
        event: EventId,
    ) -> impl Future<Output = StoreResult<Vec<SmellReport>>> + Send;
    /// Reports that are not deleted, observed in `[from, to)`, ordered by observed time.
    fn reports_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<Vec<SmellReport>>> + Send;
}

pub trait ReadingRepo: Send + Sync {
    /// A reading, deleted or not.
    fn reading(
        &self,
        id: ReadingId,
    ) -> impl Future<Output = StoreResult<Option<Pm25Reading>>> + Send;
    /// The event's readings that are not deleted, ordered by observed time.
    fn readings_for_event(
        &self,
        event: EventId,
    ) -> impl Future<Output = StoreResult<Vec<Pm25Reading>>> + Send;
    /// Readings that are not deleted, observed in `[from, to)`, ordered by observed time.
    fn readings_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<Vec<Pm25Reading>>> + Send;
}

pub trait JobRepo: Send + Sync {
    fn job_last_run(
        &self,
        name: &str,
    ) -> impl Future<Output = StoreResult<Option<DateTime<Utc>>>> + Send;
}

pub trait AuditRepo: Send + Sync {
    /// The whole chain, oldest first.
    fn audit_log(&self) -> impl Future<Output = StoreResult<Vec<ChainedEntry>>> + Send;
    /// One record's entries, oldest first: its History view.
    fn audit_history(
        &self,
        entity: AuditEntity,
        entity_id: i64,
    ) -> impl Future<Output = StoreResult<Vec<ChainedEntry>>> + Send;
}

pub trait UserWrites: Send {
    /// Fails with [`DomainError`] if the expiry breaks the role rules
    /// ([`crate::permissions::check_expiry`]) and with [`Conflict::UsernameTaken`].
    fn create_user(&mut self, new: NewUser) -> impl Future<Output = StoreResult<User>> + Send;
    /// Always audited (as `password_change`), although the snapshot only shows `has_password`.
    /// Ends every session of the user in the same transaction (spec §8.4).
    fn set_password(
        &mut self,
        id: UserId,
        hash: PasswordHash,
    ) -> impl Future<Output = StoreResult<User>> + Send;
    fn set_locale(
        &mut self,
        id: UserId,
        locale: Locale,
    ) -> impl Future<Output = StoreResult<User>> + Send;
    /// Sets `expires_at` to now and ends every session of the user. Residents cannot expire.
    fn expire_user(&mut self, id: UserId) -> impl Future<Output = StoreResult<User>> + Send;
    fn reactivate_user(
        &mut self,
        id: UserId,
        expires_at: Option<DateTime<Utc>>,
    ) -> impl Future<Output = StoreResult<User>> + Send;
}

pub trait InviteWrites: Send {
    /// Audited as `invite`. Removes the user's earlier unused invites, so only the newest link
    /// works (decision #112). `created_at` is now.
    fn create_invite(&mut self, new: NewInvite)
    -> impl Future<Output = StoreResult<Invite>> + Send;
    /// Reads inside the transaction, so checking an invite and using it see the same data.
    fn invite_by_token(
        &mut self,
        token: &TokenHash,
    ) -> impl Future<Output = StoreResult<Option<Invite>>> + Send;
    /// Sets `used_at` to now. Fails with [`Conflict::InviteUsed`] if it is already set.
    fn use_invite(&mut self, id: InviteId) -> impl Future<Output = StoreResult<Invite>> + Send;
}

pub trait ListWrites: Send {
    fn create_room(&mut self, new: NewRoom) -> impl Future<Output = StoreResult<Room>> + Send;
    /// Hiding keeps the first `hidden_at`; unhiding clears it.
    fn set_room_hidden(
        &mut self,
        id: RoomId,
        hidden: bool,
    ) -> impl Future<Output = StoreResult<Room>> + Send;
    fn create_symptom(
        &mut self,
        new: NewSymptom,
    ) -> impl Future<Output = StoreResult<Symptom>> + Send;
    fn set_symptom_hidden(
        &mut self,
        id: SymptomId,
        hidden: bool,
    ) -> impl Future<Output = StoreResult<Symptom>> + Send;
}

pub trait EventWrites: Send {
    /// Reads inside the transaction, so a decision and the write that follows it see the same data.
    fn open_event(&mut self) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
    fn event(&mut self, id: EventId) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
    /// The most recently opened event whose span contains `at`; an open event has no end yet.
    fn event_at(
        &mut self,
        at: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
    /// The observed time of the event's latest report that is not deleted.
    fn latest_report_at(
        &mut self,
        event: EventId,
    ) -> impl Future<Output = StoreResult<Option<DateTime<Utc>>>> + Send;
    /// Fails with [`Conflict::AnotherEventOpen`] when creating an open event while one is open.
    fn create_event(&mut self, new: NewEvent) -> impl Future<Output = StoreResult<Event>> + Send;
    fn close_event(
        &mut self,
        id: EventId,
        closure: Closure,
    ) -> impl Future<Output = StoreResult<Event>> + Send;
    fn reopen_event(&mut self, id: EventId) -> impl Future<Output = StoreResult<Event>> + Send;
    /// Moves a closed event's end, keeping its close reason (e.g. a backdated report after it).
    fn move_event_end(
        &mut self,
        id: EventId,
        closed_at: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<Event>> + Send;
    /// Records that the auto-close notice was shown; later calls keep the first time.
    fn mark_auto_close_notice_seen(
        &mut self,
        id: EventId,
    ) -> impl Future<Output = StoreResult<Event>> + Send;
}

pub trait ReportWrites: Send {
    fn report(
        &mut self,
        id: ReportId,
    ) -> impl Future<Output = StoreResult<Option<SmellReport>>> + Send;
    /// `recorded_at` is set to now by the store.
    fn create_report(
        &mut self,
        new: NewReport,
    ) -> impl Future<Output = StoreResult<SmellReport>> + Send;
    /// "Correct a mistake". Deleted reports cannot be corrected.
    fn correct_report(
        &mut self,
        id: ReportId,
        observed_at: DateTime<Utc>,
        values: ReportValues,
    ) -> impl Future<Output = StoreResult<SmellReport>> + Send;
    fn move_report(
        &mut self,
        id: ReportId,
        to: EventId,
    ) -> impl Future<Output = StoreResult<SmellReport>> + Send;
    /// Soft delete: sets `deleted_at`.
    fn delete_report(
        &mut self,
        id: ReportId,
    ) -> impl Future<Output = StoreResult<SmellReport>> + Send;
}

pub trait ReadingWrites: Send {
    fn create_reading(
        &mut self,
        new: NewReading,
    ) -> impl Future<Output = StoreResult<Pm25Reading>> + Send;
    fn correct_reading(
        &mut self,
        id: ReadingId,
        observed_at: DateTime<Utc>,
        value: Pm25,
    ) -> impl Future<Output = StoreResult<Pm25Reading>> + Send;
    /// Soft delete: sets `deleted_at`.
    fn delete_reading(
        &mut self,
        id: ReadingId,
    ) -> impl Future<Output = StoreResult<Pm25Reading>> + Send;
}

/// Bookkeeping for background jobs. Not audited: it is not evidence and changes daily.
pub trait JobWrites: Send {
    fn record_job_run(
        &mut self,
        name: &str,
        at: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<()>> + Send;
}

/// Everything a write transaction can do.
pub trait WriteTx:
    Commit
    + UserWrites
    + InviteWrites
    + ListWrites
    + EventWrites
    + ReportWrites
    + ReadingWrites
    + JobWrites
{
}

impl<T> WriteTx for T where
    T: Commit
        + UserWrites
        + InviteWrites
        + ListWrites
        + EventWrites
        + ReportWrites
        + ReadingWrites
        + JobWrites
{
}

pub trait SnapshotRepo: Send + Sync {
    /// The audit log and every record with audit snapshots (deleted ones included), read in one
    /// read transaction.
    fn audit_state(&self) -> impl Future<Output = StoreResult<AuditState>> + Send;
}

/// Login sessions, for the web layer's session store. Not audited (decision #50).
///
/// Each method is its own small write or read, outside any [`Transactional::begin`]
/// transaction; on SQLite the writes share the single writer connection, so never call them
/// while holding a transaction.
pub trait SessionRepo: Send + Sync {
    /// The session, unless it has expired by `now`.
    fn session(
        &self,
        id_hash: &str,
        now: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<Option<StoredSession>>> + Send;
    /// Fails with [`Conflict::Duplicate`] if a session with that `id_hash` exists.
    fn create_session(
        &self,
        session: &StoredSession,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    /// Updates a stored session. A session that no longer exists (ended by sign-out, a
    /// password change or expiry) stays ended: a request that loaded it earlier and saves it
    /// late never brings it back.
    fn save_session(&self, session: &StoredSession)
    -> impl Future<Output = StoreResult<()>> + Send;
    fn delete_session(&self, id_hash: &str) -> impl Future<Output = StoreResult<()>> + Send;
    /// Removes every session that has expired by `now` and returns how many there were.
    fn delete_expired_sessions(
        &self,
        now: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<u64>> + Send;
}

/// A complete storage backend.
pub trait Store:
    Transactional<Tx: WriteTx>
    + UserRepo
    + InviteRepo
    + ListRepo
    + EventRepo
    + ReportRepo
    + ReadingRepo
    + JobRepo
    + AuditRepo
    + SnapshotRepo
    + SessionRepo
{
}

impl<T> Store for T where
    T: Transactional<Tx: WriteTx>
        + UserRepo
        + InviteRepo
        + ListRepo
        + EventRepo
        + ReportRepo
        + ReadingRepo
        + JobRepo
        + AuditRepo
        + SnapshotRepo
        + SessionRepo
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicts_and_rule_violations_convert_into_store_errors() {
        let conflict: StoreError = Conflict::AnotherEventOpen.into();
        assert!(matches!(
            conflict,
            StoreError::Conflict(Conflict::AnotherEventOpen)
        ));
        assert_eq!(conflict.to_string(), "another event is already open");

        let invalid: StoreError = DomainError::GuestsMustExpire.into();
        assert!(matches!(
            invalid,
            StoreError::Invalid(DomainError::GuestsMustExpire)
        ));
        assert_eq!(invalid.to_string(), "guests always need an expiry date");
    }

    #[test]
    fn not_found_names_the_entity_and_id() {
        let error = StoreError::NotFound {
            entity: AuditEntity::SmellReport,
            id: 4,
        };
        assert_eq!(error.to_string(), "smell_report 4 does not exist");
    }

    #[test]
    fn backend_errors_keep_their_source() {
        let error = StoreError::Backend("disk full".into());
        assert_eq!(error.to_string(), "database error");
        assert_eq!(
            std::error::Error::source(&error).unwrap().to_string(),
            "disk full"
        );
    }
}
