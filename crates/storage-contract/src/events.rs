//! Events: opening, closing, reopening, and the one-open-event rule.

use chrono::TimeDelta;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{Closure, Event, NewEvent};
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, EventRepo, EventWrites, StoreError, Transactional, UserWrites,
};
use smoke_domain::types::{CloseReason, EventId, Role};

use crate::Fixture;
use crate::fixture::{add_event, add_user, last_entry, start};

fn closure(at: chrono::DateTime<chrono::Utc>, reason: CloseReason) -> Option<Closure> {
    Some(Closure { at, reason })
}

pub async fn a_new_event_can_be_the_open_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + EventRepo + AuditRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let opened_at = start() - TimeDelta::minutes(5);
    let event = add_event(fx, owner.id, opened_at, None).await;
    assert!(event.is_open());
    assert_eq!(
        (event.owner_id, event.opened_at, event.recorded_at),
        (owner.id, opened_at, start())
    );
    assert_eq!(fx.store.open_event().await.unwrap(), Some(event.clone()));
    assert_eq!(fx.store.event(event.id).await.unwrap(), Some(event.clone()));

    let mut tx = fx.store.begin(None).await.unwrap();
    assert_eq!(tx.open_event().await.unwrap(), Some(event.clone()));
    assert_eq!(tx.event(event.id).await.unwrap(), Some(event));
    drop(tx);

    let entry = last_entry(fx).await.record;
    assert_eq!(
        (entry.entity, entry.action, entry.actor_id),
        (AuditEntity::Event, AuditAction::Create, Some(owner.id))
    );
}

pub async fn only_one_event_can_be_open<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites>,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    add_event(fx, owner.id, start(), None).await;

    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx
        .create_event(NewEvent {
            owner_id: owner.id,
            opened_at: start(),
            closed: None,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::AnotherEventOpen)),
        "{error:?}"
    );
    drop(tx);

    let yesterday = start() - TimeDelta::days(1);
    let backdated = add_event(
        fx,
        owner.id,
        yesterday,
        closure(yesterday, CloseReason::Backdated),
    )
    .await;
    assert!(!backdated.is_open());
}

pub async fn closing_and_reopening_are_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + EventRepo + AuditRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let event = add_event(fx, owner.id, start() - TimeDelta::hours(1), None).await;
    let manual = Closure {
        at: start(),
        reason: CloseReason::Manual,
    };

    let mut tx = fx.store.begin(Some(owner.id)).await.unwrap();
    let closed = tx.close_event(event.id, manual).await.unwrap();
    assert_eq!(closed.closed, Some(manual));
    let error = tx.close_event(event.id, manual).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::EventNotOpen)),
        "{error:?}"
    );
    let reopened = tx.reopen_event(event.id).await.unwrap();
    assert!(reopened.is_open());
    let error = tx.reopen_event(event.id).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::EventNotClosed)),
        "{error:?}"
    );
    tx.commit().await.unwrap();

    assert_eq!(fx.store.open_event().await.unwrap(), Some(reopened));
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::Event, event.id.0)
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(
        actions,
        [AuditAction::Create, AuditAction::Close, AuditAction::Reopen]
    );
}

pub async fn closing_an_event_before_it_opened_breaks_a_data_rule<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites>,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let event = add_event(fx, owner.id, start(), None).await;
    let mut tx = fx.store.begin(Some(owner.id)).await.unwrap();
    let error = tx
        .close_event(
            event.id,
            Closure {
                at: start() - TimeDelta::hours(1),
                reason: CloseReason::Manual,
            },
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::RuleViolated)),
        "{error:?}"
    );
}

pub async fn reopening_is_refused_while_another_event_is_open<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + EventRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let earlier = start() - TimeDelta::hours(3);
    let first = add_event(
        fx,
        owner.id,
        earlier,
        closure(earlier + TimeDelta::hours(1), CloseReason::Manual),
    )
    .await;
    add_event(fx, owner.id, start() - TimeDelta::minutes(30), None).await;

    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx.reopen_event(first.id).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::AnotherEventOpen)),
        "{error:?}"
    );
    drop(tx);
    assert!(!fx.store.event(first.id).await.unwrap().unwrap().is_open());
}

pub async fn the_end_of_a_closed_event_can_move<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites>,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let at = start() - TimeDelta::days(1);
    let past = add_event(fx, owner.id, at, closure(at, CloseReason::Backdated)).await;
    let open = add_event(fx, owner.id, start(), None).await;

    let mut tx = fx.store.begin(None).await.unwrap();
    let moved = tx
        .move_event_end(past.id, at + TimeDelta::hours(1))
        .await
        .unwrap();
    assert_eq!(
        moved.closed,
        closure(at + TimeDelta::hours(1), CloseReason::Backdated)
    );
    let error = tx.move_event_end(open.id, start()).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::EventNotClosed)),
        "{error:?}"
    );
}

pub async fn the_auto_close_notice_is_marked_seen_once<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + AuditRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let opened = start() - TimeDelta::hours(6);
    let event = add_event(
        fx,
        owner.id,
        opened,
        closure(opened + TimeDelta::hours(1), CloseReason::Auto0400),
    )
    .await;

    let mut tx = fx.store.begin(Some(owner.id)).await.unwrap();
    let seen = tx.mark_auto_close_notice_seen(event.id).await.unwrap();
    assert_eq!(seen.auto_close_notice_seen_at, Some(start()));
    fx.clock.advance(TimeDelta::minutes(10));
    let again = tx.mark_auto_close_notice_seen(event.id).await.unwrap();
    assert_eq!(again.auto_close_notice_seen_at, Some(start()));
    tx.commit().await.unwrap();

    let history = fx
        .store
        .audit_history(AuditEntity::Event, event.id.0)
        .await
        .unwrap();
    assert_eq!(history.len(), 2);
}

pub async fn events_between_returns_overlapping_events<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + EventRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let morning = start() - TimeDelta::hours(12);
    let at = |hours: i64| morning + TimeDelta::hours(hours);
    let first = add_event(fx, owner.id, at(0), closure(at(1), CloseReason::Manual)).await;
    let second = add_event(fx, owner.id, at(2), closure(at(3), CloseReason::Manual)).await;
    let open = add_event(fx, owner.id, at(8), None).await;

    let between = |from, to| async move {
        fx.store
            .events_between(from, to)
            .await
            .unwrap()
            .into_iter()
            .map(|e: Event| e.id)
            .collect::<Vec<EventId>>()
    };
    let half = TimeDelta::minutes(30);
    assert_eq!(
        between(at(0) + half, at(2) + half).await,
        [first.id, second.id]
    );
    assert_eq!(between(at(1), at(2)).await, [first.id]);
    assert!(between(at(4), at(5)).await.is_empty());
    assert_eq!(between(at(10), at(11)).await, [open.id]);
}
