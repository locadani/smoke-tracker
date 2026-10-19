//! Closing, reopening, correcting and moving (spec §7.3, §7.7), on a real store.

use chrono::TimeDelta;
use smoke_domain::DomainError;
use smoke_domain::clock::Clock;
use smoke_domain::logbook::{
    EventChoice, LogError, MoveTarget, close_event, correct_report, move_report, reopen_event,
};
use smoke_domain::records::{Closure, Event, LogEntry, ReportValues, User};
use smoke_domain::repos::{
    AuditRepo, Commit, EntryWrites, EventRepo, ListRepo, Transactional, WriteTx,
};
use smoke_domain::types::{CloseReason, EventId, ReportId};

use crate::Fixture;
use crate::fixture::{EntryExt, FIRST_ROOM, add_event, report_values, start};
use crate::logbook::{log, no_smell, people, rome, smell};

async fn close<S>(
    fx: &Fixture<S>,
    who: &User,
    id: EventId,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<Event, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let event = close_event(&mut tx, &who.actor(), id, at, fx.clock.now()).await?;
    tx.commit().await.unwrap();
    Ok(event)
}

async fn reopen<S>(fx: &Fixture<S>, who: &User, id: EventId) -> Result<Event, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let event = reopen_event(&mut tx, &who.actor(), id, fx.clock.now()).await?;
    tx.commit().await.unwrap();
    Ok(event)
}

async fn correct<S>(
    fx: &Fixture<S>,
    who: &User,
    id: ReportId,
    at: chrono::DateTime<chrono::Utc>,
    level: u8,
) -> Result<LogEntry, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let values = report_values(level, &[FIRST_ROOM], &[]);
    correct_values(fx, who, id, at, values).await
}

async fn correct_values<S>(
    fx: &Fixture<S>,
    who: &User,
    id: ReportId,
    at: chrono::DateTime<chrono::Utc>,
    values: ReportValues,
) -> Result<LogEntry, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let now = fx.clock.now();
    let report = correct_report(&mut tx, &who.actor(), id, at, values, None, &rome(), now).await?;
    tx.commit().await.unwrap();
    Ok(report)
}

async fn relocate<S>(
    fx: &Fixture<S>,
    who: &User,
    id: ReportId,
    to: MoveTarget,
) -> Result<LogEntry, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let report = move_report(&mut tx, &who.actor(), id, to, &rome(), fx.clock.now()).await?;
    tx.commit().await.unwrap();
    Ok(report)
}

async fn event<S: EventRepo>(fx: &Fixture<S>, id: EventId) -> Event {
    fx.store.event(id).await.unwrap().unwrap()
}

pub async fn a_manual_close_needs_a_time_between_the_last_report_and_now<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let logged = log(
        fx,
        &p.resident,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    let id = logged.entry.event_id.unwrap();
    fx.clock.advance(TimeDelta::hours(1));

    let early = close(fx, &p.resident, id, start() - TimeDelta::minutes(1)).await;
    assert!(
        matches!(
            early,
            Err(LogError::Invalid(DomainError::CloseBeforeLastReport))
        ),
        "{early:?}"
    );
    let future = close(fx, &p.resident, id, fx.clock.now() + TimeDelta::hours(1)).await;
    assert!(
        matches!(future, Err(LogError::Invalid(DomainError::CloseInFuture))),
        "{future:?}"
    );
    let at = start() + TimeDelta::minutes(40);
    let closed = close(fx, &p.resident, id, at).await.unwrap();
    assert_eq!(
        closed.closed,
        Some(Closure {
            at,
            reason: CloseReason::Manual
        })
    );
    let again = close(fx, &p.resident, id, at).await;
    assert!(matches!(again, Err(LogError::Stale)), "{again:?}");
}

pub async fn only_residents_and_owning_guests_can_close_or_reopen<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let residents = log(
        fx,
        &p.resident,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap()
    .entry
    .event_id
    .unwrap();
    for who in [&p.guest, &p.viewer] {
        let refused = close(fx, who, residents, start()).await;
        assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
    }
    close(fx, &p.partner, residents, start()).await.unwrap();
    let refused = reopen(fx, &p.guest, residents).await;
    assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");

    fx.clock.advance(TimeDelta::minutes(5));
    let guests = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap()
    .entry
    .event_id
    .unwrap();
    close(fx, &p.guest, guests, fx.clock.now()).await.unwrap();
    reopen(fx, &p.guest, guests).await.unwrap();
    assert!(event(fx, guests).await.is_open());

    let missing = close(fx, &p.resident, EventId(999), start()).await;
    assert!(matches!(missing, Err(LogError::NotFound)), "{missing:?}");
}

pub async fn reopening_waits_until_no_other_event_is_open<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(3);
    let past = add_event(
        fx,
        p.resident.id,
        opened,
        Some(Closure {
            at: opened,
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let open = add_event(fx, p.resident.id, start(), None).await;

    let blocked = reopen(fx, &p.resident, past.id).await;
    assert!(
        matches!(blocked, Err(LogError::AnotherEventOpen)),
        "{blocked:?}"
    );
    let already = reopen(fx, &p.resident, open.id).await;
    assert!(matches!(already, Err(LogError::Stale)), "{already:?}");
    close(fx, &p.resident, open.id, start()).await.unwrap();
    reopen(fx, &p.resident, past.id).await.unwrap();
    assert_eq!(
        fx.store.open_event().await.unwrap().map(|e| e.id),
        Some(past.id)
    );
}

pub async fn correcting_after_the_end_moves_the_end_and_before_the_start_is_refused<S>(
    fx: &Fixture<S>,
) where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + AuditRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(4);
    let logged = log(
        fx,
        &p.resident,
        smell(opened, 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    let id = logged.entry.event_id.unwrap();
    close(fx, &p.resident, id, opened + TimeDelta::minutes(30))
        .await
        .unwrap();

    let corrected = correct(fx, &p.resident, logged.entry.report_id(), opened, 7)
        .await
        .unwrap();
    assert_eq!(corrected.values().intensity().get(), 7);

    let later = opened + TimeDelta::hours(1);
    let moved = correct(fx, &p.resident, logged.entry.report_id(), later, 7)
        .await
        .unwrap();
    assert_eq!(moved.observed_at, later);
    assert_eq!(event(fx, id).await.closed.map(|c| c.at), Some(later));

    let before = correct(
        fx,
        &p.resident,
        logged.entry.report_id(),
        opened - TimeDelta::minutes(1),
        7,
    )
    .await;
    assert!(matches!(before, Err(LogError::OutsideEvent)), "{before:?}");
}

pub async fn guests_can_correct_and_move_only_their_own_reports<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let residents = log(
        fx,
        &p.resident,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    let event_id = residents.entry.event_id.unwrap();
    let guests = log(fx, &p.guest, smell(start(), 5), EventChoice::Join(event_id))
        .await
        .unwrap();

    for who in [&p.guest, &p.viewer] {
        let refused = correct(fx, who, residents.entry.report_id(), start(), 9).await;
        assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
        let refused = relocate(fx, who, residents.entry.report_id(), MoveTarget::New).await;
        assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
    }
    correct(fx, &p.guest, guests.entry.report_id(), start(), 6)
        .await
        .unwrap();
    correct(fx, &p.partner, residents.entry.report_id(), start(), 3)
        .await
        .unwrap();

    let mut tx = fx.store.begin(Some(p.guest.id)).await.unwrap();
    tx.delete_entry(guests.entry.id).await.unwrap();
    tx.commit().await.unwrap();
    let deleted = correct(fx, &p.guest, guests.entry.report_id(), start(), 6).await;
    assert!(matches!(deleted, Err(LogError::NotFound)), "{deleted:?}");
}

pub async fn moving_to_a_new_event_creates_it_for_the_reports_author<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let first = log(
        fx,
        &p.resident,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    let joined = log(
        fx,
        &p.guest,
        smell(start(), 5),
        EventChoice::Join(first.entry.event_id.unwrap()),
    )
    .await
    .unwrap();

    let moved = relocate(fx, &p.partner, joined.entry.report_id(), MoveTarget::New)
        .await
        .unwrap();
    assert_ne!(moved.event_id, first.entry.event_id);
    let new = event(fx, moved.event_id.unwrap()).await;
    assert_eq!((new.owner_id, new.opened_at), (p.guest.id, start()));
    assert_eq!(
        new.closed,
        Some(Closure {
            at: start(),
            reason: CloseReason::Backdated
        }),
        "the first event is still open"
    );
    let same = relocate(
        fx,
        &p.partner,
        joined.entry.report_id(),
        MoveTarget::Event(moved.event_id.unwrap()),
    )
    .await
    .unwrap();
    assert_eq!(same, moved);
}

pub async fn moving_after_the_end_of_the_target_moves_its_end<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(4);
    let earlier = add_event(
        fx,
        p.resident.id,
        opened,
        Some(Closure {
            at: opened + TimeDelta::minutes(10),
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let later_at = opened + TimeDelta::hours(1);
    let later = log(
        fx,
        &p.resident,
        smell(later_at, 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();

    let moved = relocate(
        fx,
        &p.resident,
        later.entry.report_id(),
        MoveTarget::Event(earlier.id),
    )
    .await
    .unwrap();
    assert_eq!(moved.event_id, Some(earlier.id));
    assert_eq!(
        event(fx, earlier.id).await.closed.map(|c| c.at),
        Some(later_at)
    );

    let first = log(
        fx,
        &p.resident,
        smell(opened - TimeDelta::hours(1), 4),
        EventChoice::New {
            replacing: later.entry.event_id,
        },
    )
    .await
    .unwrap();
    let not_started = relocate(
        fx,
        &p.resident,
        first.entry.report_id(),
        MoveTarget::Event(earlier.id),
    )
    .await;
    assert!(
        matches!(not_started, Err(LogError::OutsideEvent)),
        "{not_started:?}"
    );
}

pub async fn a_report_cannot_move_into_another_days_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let logged = log(
        fx,
        &p.resident,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    let opened = start() - TimeDelta::days(2);
    let other_day = add_event(
        fx,
        p.resident.id,
        opened,
        Some(Closure {
            at: opened + TimeDelta::hours(1),
            reason: CloseReason::Manual,
        }),
    )
    .await;

    let refused = relocate(
        fx,
        &p.resident,
        logged.entry.report_id(),
        MoveTarget::Event(other_day.id),
    )
    .await;
    assert!(
        matches!(refused, Err(LogError::OutsideEvent)),
        "{refused:?}"
    );
}

pub async fn a_deleted_report_or_a_missing_target_event_is_not_found<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let logged = log(
        fx,
        &p.resident,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    let missing = relocate(
        fx,
        &p.resident,
        logged.entry.report_id(),
        MoveTarget::Event(EventId(999)),
    )
    .await;
    assert!(matches!(missing, Err(LogError::NotFound)), "{missing:?}");

    let mut tx = fx.store.begin(Some(p.resident.id)).await.unwrap();
    tx.delete_entry(logged.entry.id).await.unwrap();
    tx.commit().await.unwrap();
    let deleted = relocate(fx, &p.resident, logged.entry.report_id(), MoveTarget::New).await;
    assert!(matches!(deleted, Err(LogError::NotFound)), "{deleted:?}");
}

pub async fn a_check_can_be_corrected_but_never_to_a_smell<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let check = log(
        fx,
        &p.guest,
        no_smell(start()),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap()
    .entry;
    let earlier = start() - TimeDelta::hours(5);
    let corrected = correct_values(
        fx,
        &p.guest,
        check.report_id(),
        earlier,
        report_values(0, &[FIRST_ROOM], &[]),
    )
    .await
    .unwrap();
    assert_eq!((corrected.event_id, corrected.observed_at), (None, earlier));
    let refused = correct_values(
        fx,
        &p.guest,
        check.report_id(),
        earlier,
        report_values(3, &[FIRST_ROOM], &[]),
    )
    .await;
    assert!(
        matches!(refused, Err(LogError::CheckCannotSmell)),
        "{refused:?}"
    );
}

pub async fn a_check_can_move_into_an_event_but_never_opens_one<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let check = log(
        fx,
        &p.resident,
        no_smell(start()),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap()
    .entry;
    let refused = relocate(fx, &p.resident, check.report_id(), MoveTarget::New).await;
    assert!(
        matches!(refused, Err(LogError::NoSmellCannotOpen)),
        "{refused:?}"
    );
    let earlier = add_event(fx, p.resident.id, start() - TimeDelta::hours(1), None).await;
    let moved = relocate(
        fx,
        &p.resident,
        check.report_id(),
        MoveTarget::Event(earlier.id),
    )
    .await
    .unwrap();
    assert_eq!(moved.event_id, Some(earlier.id));
}
