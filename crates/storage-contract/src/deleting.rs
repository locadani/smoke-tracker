//! Deleting entries and events, and an event following its remaining entries, on a real store.

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity, verify_chain};
use smoke_domain::clock::Clock;
use smoke_domain::deleting::{delete_entry, delete_event};
use smoke_domain::logbook::{EventChoice, LogError, MoveTarget, move_report};
use smoke_domain::records::{Closure, Event, LogEntry, NewEntry, NewReading, User};
use smoke_domain::repos::{Commit, EntryWrites, Store};
use smoke_domain::types::{CloseReason, EventId, LogEntryId, Pm25};
use smoke_domain::verify::verify_store;

use crate::Fixture;
use crate::fixture::{FIRST_ROOM, add_event, add_live_event, report_values, start};
use crate::logbook::{log, no_smell, people, rome, smell};

fn t0() -> DateTime<Utc> {
    start() - TimeDelta::hours(1)
}

async fn audit_len<S: Store>(fx: &Fixture<S>) -> usize {
    fx.store.audit_log().await.unwrap().len()
}

/// The audit rows written after the first `before`, as (entity, action) pairs.
async fn audit_since<S: Store>(fx: &Fixture<S>, before: usize) -> Vec<(AuditEntity, AuditAction)> {
    let log = fx.store.audit_log().await.unwrap();
    log[before..]
        .iter()
        .map(|row| (row.record.entity, row.record.action))
        .collect()
}

async fn event<S: Store>(fx: &Fixture<S>, id: EventId) -> Option<Event> {
    fx.store.event(id).await.unwrap()
}

async fn entry_ids<S: Store>(fx: &Fixture<S>, id: EventId) -> Vec<LogEntryId> {
    let entries = fx.store.entries_for_event(id).await.unwrap();
    entries.iter().map(|entry| entry.id).collect()
}

async fn delete<S: Store>(
    fx: &Fixture<S>,
    who: &User,
    id: LogEntryId,
) -> Result<Option<EventId>, LogError> {
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let left = delete_entry(&mut tx, &who.actor(), id, fx.clock.now()).await?;
    tx.commit().await.unwrap();
    Ok(left)
}

async fn delete_whole<S: Store>(
    fx: &Fixture<S>,
    who: &User,
    id: EventId,
) -> Result<usize, LogError> {
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let deleted = delete_event(&mut tx, &who.actor(), id, fx.clock.now()).await?;
    tx.commit().await.unwrap();
    Ok(deleted)
}

/// Logs a smell by `who` at `at`, into `event`.
async fn join<S: Store>(
    fx: &Fixture<S>,
    who: &User,
    event: EventId,
    at: DateTime<Utc>,
) -> LogEntryId {
    let logged = log(fx, who, smell(at, 3), EventChoice::Join(event))
        .await
        .unwrap();
    logged.entry.id
}

/// A new open event with its one entry, a smell `who` logs now.
async fn open_event_of<S: Store>(fx: &Fixture<S>, who: &User) -> (EventId, LogEntryId) {
    let logged = log(
        fx,
        who,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    (logged.entry.event_id.unwrap(), logged.entry.id)
}

pub async fn deleting_the_only_entry_removes_the_event<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, entry) = open_event_of(fx, &p.resident).await;
    let before = audit_len(fx).await;

    assert_eq!(delete(fx, &p.resident, entry).await.unwrap(), None);

    assert_eq!(event(fx, event_id).await, None);
    assert_eq!(
        audit_since(fx, before).await,
        [(AuditEntity::LogEntry, AuditAction::Delete)]
    );
}

pub async fn deleting_a_later_entry_leaves_the_event_alone<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, _) = open_event_of(fx, &p.resident).await;
    fx.clock.advance(TimeDelta::minutes(30));
    let later = join(fx, &p.partner, event_id, fx.clock.now()).await;
    let unchanged = event(fx, event_id).await;
    let before = audit_len(fx).await;

    assert_eq!(delete(fx, &p.partner, later).await.unwrap(), Some(event_id));

    assert_eq!(event(fx, event_id).await, unchanged);
    assert_eq!(
        audit_since(fx, before).await,
        [(AuditEntity::LogEntry, AuditAction::Delete)]
    );
}

pub async fn deleting_the_earliest_entry_moves_the_start_and_owner<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let open = add_live_event(fx, p.resident.id, t0(), None).await;
    let duygu_at = t0() + TimeDelta::minutes(30);
    join(fx, &p.partner, open.id, duygu_at).await;
    let first = entry_ids(fx, open.id).await[0];
    let before = audit_len(fx).await;

    assert_eq!(delete(fx, &p.resident, first).await.unwrap(), Some(open.id));

    let after = event(fx, open.id).await.unwrap();
    assert_eq!((after.opened_at, after.owner_id), (duygu_at, p.partner.id));
    assert_eq!(after.closed, open.closed);
    assert_eq!(
        audit_since(fx, before).await,
        [
            (AuditEntity::LogEntry, AuditAction::Delete),
            (AuditEntity::Event, AuditAction::Update)
        ]
    );
}

pub async fn the_start_never_passes_the_end<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let end = Closure {
        at: t0() + TimeDelta::minutes(40),
        reason: CloseReason::Manual,
    };
    let closed = add_live_event(fx, p.resident.id, t0(), Some(end)).await;
    let first = entry_ids(fx, closed.id).await[0];
    let late = NewEntry::new(
        p.resident.id,
        Some(closed.id),
        t0() + TimeDelta::minutes(50),
        None,
        Some(report_values(3, &[FIRST_ROOM], &[])),
        None,
    )
    .unwrap();
    let mut tx = fx.store.begin(Some(p.resident.id)).await.unwrap();
    tx.create_entry(late).await.unwrap();
    tx.commit().await.unwrap();

    delete(fx, &p.resident, first).await.unwrap();

    let after = event(fx, closed.id).await.unwrap();
    assert_eq!(after.opened_at, end.at);
    assert_eq!(after.closed, Some(end));
}

pub async fn a_check_with_no_event_is_deleted<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let logged = log(
        fx,
        &p.resident,
        no_smell(start()),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    assert_eq!(logged.entry.event_id, None);
    let before = audit_len(fx).await;

    assert_eq!(
        delete(fx, &p.resident, logged.entry.id).await.unwrap(),
        None
    );

    assert_eq!(
        audit_since(fx, before).await,
        [(AuditEntity::LogEntry, AuditAction::Delete)]
    );
}

pub async fn deleting_twice_is_not_found<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (_, entry) = open_event_of(fx, &p.resident).await;
    delete(fx, &p.resident, entry).await.unwrap();
    let before = audit_len(fx).await;

    let again = delete(fx, &p.resident, entry).await;
    assert!(matches!(again, Err(LogError::NotFound)), "{again:?}");
    let unknown = delete(fx, &p.resident, LogEntryId(9_999)).await;
    assert!(matches!(unknown, Err(LogError::NotFound)), "{unknown:?}");
    assert_eq!(audit_len(fx).await, before);
}

pub async fn a_resident_deletes_a_guests_entry_but_not_another_residents<S: Store>(
    fx: &Fixture<S>,
) {
    let p = people(fx).await;
    let (event_id, _) = open_event_of(fx, &p.resident).await;
    let partners = join(fx, &p.partner, event_id, start()).await;
    let guests = join(fx, &p.guest, event_id, start()).await;
    let expired_guests = join(fx, &p.guest, event_id, start()).await;

    let refused = delete(fx, &p.resident, partners).await;
    assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
    assert_eq!(
        delete(fx, &p.resident, guests).await.unwrap(),
        Some(event_id)
    );

    fx.clock.advance(TimeDelta::days(8));
    assert_eq!(
        delete(fx, &p.resident, expired_guests).await.unwrap(),
        Some(event_id)
    );
}

pub async fn a_guest_cannot_see_others_entries_and_a_viewer_cannot_delete_them<S: Store>(
    fx: &Fixture<S>,
) {
    let p = people(fx).await;
    let (event_id, residents) = open_event_of(fx, &p.resident).await;
    let guests = join(fx, &p.guest, event_id, start()).await;
    let before = audit_len(fx).await;

    let hidden = delete(fx, &p.guest, residents).await;
    assert!(matches!(hidden, Err(LogError::NotFound)), "{hidden:?}");
    for entry in [guests, residents] {
        let refused = delete(fx, &p.viewer, entry).await;
        assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
    }
    assert_eq!(audit_len(fx).await, before);

    assert_eq!(delete(fx, &p.guest, guests).await.unwrap(), Some(event_id));
}

pub async fn deleting_an_event_deletes_every_entry_in_one_transaction<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, _) = open_event_of(fx, &p.resident).await;
    join(fx, &p.resident, event_id, start()).await;
    join(fx, &p.guest, event_id, start()).await;
    let before = audit_len(fx).await;

    assert_eq!(delete_whole(fx, &p.resident, event_id).await.unwrap(), 3);

    assert_eq!(event(fx, event_id).await, None);
    assert_eq!(
        audit_since(fx, before).await,
        [(AuditEntity::LogEntry, AuditAction::Delete); 3]
    );
}

pub async fn deleting_an_event_with_another_residents_entry_is_refused<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, _) = open_event_of(fx, &p.resident).await;
    join(fx, &p.partner, event_id, start()).await;
    let before = audit_len(fx).await;

    let refused = delete_whole(fx, &p.resident, event_id).await;
    assert!(
        matches!(refused, Err(LogError::OthersLogged)),
        "{refused:?}"
    );

    assert_eq!(audit_len(fx).await, before);
    assert_eq!(entry_ids(fx, event_id).await.len(), 2);
}

pub async fn deleting_an_event_none_of_mine_is_forbidden<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, _) = open_event_of(fx, &p.partner).await;
    let before = audit_len(fx).await;

    let refused = delete_whole(fx, &p.resident, event_id).await;
    assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
    assert_eq!(audit_len(fx).await, before);
}

pub async fn deleting_an_emptied_event_is_not_found<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, entry) = open_event_of(fx, &p.resident).await;
    delete(fx, &p.resident, entry).await.unwrap();
    let before = audit_len(fx).await;

    let gone = delete_whole(fx, &p.resident, event_id).await;
    assert!(matches!(gone, Err(LogError::NotFound)), "{gone:?}");
    assert_eq!(audit_len(fx).await, before);
}

async fn relocate_to_new<S: Store>(fx: &Fixture<S>, who: &User, entry: LogEntryId) -> EventId {
    let entry = fx.store.entry(entry).await.unwrap().unwrap();
    let report = entry.report.unwrap().id;
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let moved = move_report(
        &mut tx,
        &who.actor(),
        report,
        MoveTarget::New,
        &rome(),
        fx.clock.now(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    moved.event_id.unwrap()
}

pub async fn moving_the_earliest_entry_out_moves_the_start<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, first) = open_event_of(fx, &p.resident).await;
    fx.clock.advance(TimeDelta::minutes(30));
    let partner_at = fx.clock.now();
    join(fx, &p.partner, event_id, partner_at).await;

    let new_id = relocate_to_new(fx, &p.resident, first).await;

    assert_ne!(new_id, event_id);
    let old = event(fx, event_id).await.unwrap();
    assert_eq!((old.opened_at, old.owner_id), (partner_at, p.partner.id));
    assert_eq!(old.closed, None);
    assert_eq!(entry_ids(fx, new_id).await, [first]);
}

pub async fn moving_the_only_entry_out_keeps_its_open_event<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, entry) = open_event_of(fx, &p.partner).await;
    let window = (t0(), start() + TimeDelta::days(1));
    let events_before = fx.store.events_between(window.0, window.1).await.unwrap();

    let same = relocate_to_new(fx, &p.resident, entry).await;

    assert_eq!(same, event_id);
    let kept = event(fx, event_id).await.unwrap();
    assert_eq!((kept.opened_at, kept.owner_id), (start(), p.partner.id));
    assert!(kept.is_open());
    let events = fx.store.events_between(window.0, window.1).await.unwrap();
    assert_eq!(events, events_before);
}

pub async fn moving_the_only_entry_out_re_centres_its_closed_event<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let end = Closure {
        at: t0() + TimeDelta::minutes(20),
        reason: CloseReason::Manual,
    };
    let closed = add_event(fx, p.resident.id, t0(), Some(end)).await;
    let at = t0() + TimeDelta::minutes(10);
    let new = NewEntry::new(
        p.partner.id,
        Some(closed.id),
        at,
        None,
        Some(report_values(3, &[FIRST_ROOM], &[])),
        None,
    )
    .unwrap();
    let mut tx = fx.store.begin(Some(p.partner.id)).await.unwrap();
    let only = tx.create_entry(new).await.unwrap().id;
    tx.commit().await.unwrap();
    let before = audit_len(fx).await;

    let same = relocate_to_new(fx, &p.partner, only).await;

    assert_eq!(same, closed.id);
    let kept = event(fx, closed.id).await.unwrap();
    assert_eq!(kept.closed, Some(end));
    assert_eq!((kept.opened_at, kept.owner_id), (at, p.partner.id));
    assert_eq!(
        audit_since(fx, before).await,
        [(AuditEntity::Event, AuditAction::Update)]
    );
}

pub async fn moving_an_entry_while_another_event_is_open_creates_a_closed_event<S: Store>(
    fx: &Fixture<S>,
) {
    let p = people(fx).await;
    let (open_id, first) = open_event_of(fx, &p.resident).await;
    join(fx, &p.partner, open_id, start()).await;

    let new_id = relocate_to_new(fx, &p.resident, first).await;

    assert_ne!(new_id, open_id);
    let new = event(fx, new_id).await.unwrap();
    assert_eq!(
        new.closed,
        Some(Closure {
            at: start(),
            reason: CloseReason::Backdated
        })
    );
    assert!(event(fx, open_id).await.unwrap().is_open());
}

pub async fn verify_audit_is_clean_after_deletions<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let open = add_live_event(fx, p.resident.id, t0(), None).await;
    let second = join(fx, &p.resident, open.id, t0() + TimeDelta::minutes(10)).await;
    join(fx, &p.resident, open.id, t0() + TimeDelta::minutes(20)).await;
    let first = entry_ids(fx, open.id).await[0];

    delete(fx, &p.resident, second).await.unwrap();
    delete(fx, &p.resident, first).await.unwrap();
    assert_eq!(delete_whole(fx, &p.resident, open.id).await.unwrap(), 1);

    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(verify_chain(&log), Ok(()));
    let report = verify_store(&fx.store).await.unwrap();
    assert!(report.is_ok(), "{report:?}");
}

/// Logs a PM2.5 reading by `who` at `at` into `event`, with no report.
async fn add_reading<S: Store>(
    fx: &Fixture<S>,
    who: &User,
    event: EventId,
    at: DateTime<Utc>,
) -> LogEntryId {
    let entry = NewEntry::new(
        who.id,
        Some(event),
        at,
        None,
        None,
        Some(NewReading::without_photo(Pm25::new(12.0).unwrap())),
    )
    .unwrap();
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let id = tx.create_entry(entry).await.unwrap().id;
    tx.commit().await.unwrap();
    id
}

async fn live_entry<S: Store>(fx: &Fixture<S>, id: LogEntryId) -> LogEntry {
    fx.store.entry(id).await.unwrap().unwrap()
}

/// An event whose only smell report is the resident's, with one reading logged after it.
async fn event_with_a_reading<S: Store>(
    fx: &Fixture<S>,
    p: &crate::logbook::People,
) -> (EventId, LogEntryId, LogEntryId) {
    let (event_id, report) = open_event_of(fx, &p.resident).await;
    let reading = add_reading(fx, &p.resident, event_id, start() + TimeDelta::minutes(5)).await;
    (event_id, report, reading)
}

fn log_entry_audit(rows: &[(AuditEntity, AuditAction)]) -> bool {
    rows == [
        (AuditEntity::LogEntry, AuditAction::Delete),
        (AuditEntity::LogEntry, AuditAction::Update),
    ]
}

pub async fn deleting_the_last_report_detaches_the_readings<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, report, reading) = event_with_a_reading(fx, &p).await;
    let before = audit_len(fx).await;

    assert_eq!(delete(fx, &p.resident, report).await.unwrap(), None);

    assert_eq!(event(fx, event_id).await, None);
    let kept = live_entry(fx, reading).await;
    assert_eq!((kept.event_id, kept.deleted_at), (None, None));
    assert!(kept.reading.is_some());
    let rows = audit_since(fx, before).await;
    assert!(log_entry_audit(&rows), "{rows:?}");
}

pub async fn moving_the_last_report_to_another_event_detaches_the_readings<S: Store>(
    fx: &Fixture<S>,
) {
    let p = people(fx).await;
    let (event_id, report, reading) = event_with_a_reading(fx, &p).await;
    let end = Closure {
        at: start() + TimeDelta::minutes(10),
        reason: CloseReason::Manual,
    };
    let other = add_live_event(fx, p.partner.id, t0(), Some(end)).await;
    let before = audit_len(fx).await;
    let report_id = live_entry(fx, report).await.report.unwrap().id;

    let mut tx = fx.store.begin(Some(p.resident.id)).await.unwrap();
    move_report(
        &mut tx,
        &p.resident.actor(),
        report_id,
        MoveTarget::Event(other.id),
        &rome(),
        fx.clock.now(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(event(fx, event_id).await, None);
    assert_eq!(live_entry(fx, reading).await.event_id, None);
    assert_eq!(entry_ids(fx, other.id).await.len(), 2);
    let rows = audit_since(fx, before).await;
    assert_eq!(
        rows,
        [
            (AuditEntity::LogEntry, AuditAction::Move),
            (AuditEntity::LogEntry, AuditAction::Update)
        ]
    );
}

pub async fn moving_the_last_report_to_a_new_event_detaches_the_readings<S: Store>(
    fx: &Fixture<S>,
) {
    let p = people(fx).await;
    let (event_id, report, reading) = event_with_a_reading(fx, &p).await;

    let new_id = relocate_to_new(fx, &p.resident, report).await;

    assert_ne!(new_id, event_id);
    assert_eq!(event(fx, event_id).await, None);
    assert_eq!(live_entry(fx, reading).await.event_id, None);
    assert_eq!(entry_ids(fx, new_id).await, [report]);
    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(verify_chain(&log), Ok(()));
}

pub async fn an_earlier_reading_sets_the_start_and_the_next_report_the_owner<S: Store>(
    fx: &Fixture<S>,
) {
    let p = people(fx).await;
    let open = add_live_event(fx, p.resident.id, t0(), None).await;
    let reading_at = t0() + TimeDelta::minutes(10);
    let reading = add_reading(fx, &p.resident, open.id, reading_at).await;
    let first_report = join(fx, &p.resident, open.id, t0() + TimeDelta::minutes(15)).await;
    let next_at = t0() + TimeDelta::minutes(20);
    join(fx, &p.partner, open.id, next_at).await;
    let first = entry_ids(fx, open.id).await[0];
    delete(fx, &p.resident, first).await.unwrap();

    assert_eq!(
        delete(fx, &p.resident, first_report).await.unwrap(),
        Some(open.id)
    );

    let after = event(fx, open.id).await.unwrap();
    assert_eq!(
        (after.opened_at, after.owner_id),
        (reading_at, p.partner.id)
    );
    assert_eq!(live_entry(fx, reading).await.event_id, Some(open.id));
    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(verify_chain(&log), Ok(()));
    let report = verify_store(&fx.store).await.unwrap();
    assert!(report.is_ok(), "{report:?}");
}

pub async fn detaching_an_entry_is_audited_once<S: Store>(fx: &Fixture<S>) {
    let p = people(fx).await;
    let (event_id, _, reading) = event_with_a_reading(fx, &p).await;
    let before = audit_len(fx).await;

    let mut tx = fx.store.begin(Some(p.resident.id)).await.unwrap();
    let detached = tx.detach_entry(reading).await.unwrap();
    let again = tx.detach_entry(reading).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(detached.event_id, None);
    assert_eq!(again, detached);
    assert_eq!(
        audit_since(fx, before).await,
        [(AuditEntity::LogEntry, AuditAction::Update)]
    );
    assert_eq!(entry_ids(fx, event_id).await.len(), 1);
}
