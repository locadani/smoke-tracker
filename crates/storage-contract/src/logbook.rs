//! Logging a smell or a PM2.5 reading (spec §7.1–§7.6), on a real store.

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::DomainError;
use smoke_domain::clock::Clock;
use smoke_domain::logbook::{EventChoice, LogError, Logged, SmellEntry, log_reading, log_smell};
use smoke_domain::records::{Closure, Event, Pm25Reading, ReportValues, User};
use smoke_domain::repos::{
    AuditRepo, Commit, EventRepo, ListRepo, ListWrites, ReadingRepo, ReportRepo, Transactional,
    UserWrites, WriteTx,
};
use smoke_domain::schedule::DailySchedule;
use smoke_domain::starter::seed_starter_lists;
use smoke_domain::types::{CloseReason, EventId, Pm25, Role, RoomId};

use crate::Fixture;
use crate::fixture::{FIRST_ROOM, add_event, add_user, report_values, start};

/// Everyone who might log, and the seeded lists.
pub(crate) struct People {
    pub resident: User,
    pub partner: User,
    pub guest: User,
    pub viewer: User,
}

pub(crate) async fn people<S>(fx: &Fixture<S>) -> People
where
    S: Transactional<Tx: UserWrites + ListWrites> + ListRepo,
{
    seed_starter_lists(&fx.store).await.unwrap();
    People {
        resident: add_user(fx, "daniele", Role::Resident).await,
        partner: add_user(fx, "partner", Role::Resident).await,
        guest: add_user(fx, "guest", Role::Guest).await,
        viewer: add_user(fx, "lawyer", Role::Viewer).await,
    }
}

pub(crate) fn rome() -> DailySchedule {
    DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap()
}

pub(crate) fn smell(at: DateTime<Utc>, intensity: u8) -> SmellEntry {
    SmellEntry {
        observed_at: at,
        values: report_values(intensity, &[FIRST_ROOM], &[]),
        pm25: None,
    }
}

/// "No smell" with no rooms: nothing anywhere in the home.
pub(crate) fn no_smell(at: DateTime<Utc>) -> SmellEntry {
    SmellEntry {
        observed_at: at,
        values: ReportValues::new(vec![], vec![], None).unwrap(),
        pm25: None,
    }
}

/// Runs [`log_smell`] in its own transaction, committed only on success.
pub(crate) async fn log<S>(
    fx: &Fixture<S>,
    who: &User,
    entry: SmellEntry,
    choice: EventChoice,
) -> Result<Logged, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let logged = log_smell(
        &mut tx,
        &who.actor(),
        entry,
        choice,
        &rome(),
        fx.clock.now(),
    )
    .await?;
    tx.commit().await.unwrap();
    Ok(logged)
}

async fn reading<S>(fx: &Fixture<S>, who: &User, at: DateTime<Utc>) -> Pm25Reading
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let value = Pm25::new(42.0).unwrap();
    let reading = log_reading(&mut tx, &who.actor(), at, value, fx.clock.now())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    reading
}

async fn event<S: EventRepo>(fx: &Fixture<S>, id: EventId) -> Event {
    fx.store.event(id).await.unwrap().unwrap()
}

pub async fn logging_now_with_nothing_open_starts_an_open_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let logged = log(
        fx,
        &p.guest,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    let open = fx.store.open_event().await.unwrap().unwrap();
    assert_eq!(logged.report.event_id, Some(open.id));
    assert_eq!((open.owner_id, open.opened_at), (p.guest.id, start()));
    assert_eq!(logged.reading, None);
}

pub async fn same_smell_joins_the_open_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + ReportRepo,
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
    fx.clock.advance(TimeDelta::minutes(20));
    let event_id = first.report.event_id.unwrap();
    let second = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 6),
        EventChoice::Join(event_id),
    )
    .await
    .unwrap();
    assert_eq!(second.report.event_id, Some(event_id));
    assert_eq!(event(fx, event_id).await.owner_id, p.resident.id);
    assert_eq!(fx.store.reports_for_event(event_id).await.unwrap().len(), 2);
}

pub async fn a_new_event_closes_the_open_one_at_its_last_report<S>(fx: &Fixture<S>)
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
    fx.clock.advance(TimeDelta::minutes(30));
    let last = log(
        fx,
        &p.resident,
        smell(fx.clock.now(), 2),
        EventChoice::Join(first.report.event_id.unwrap()),
    )
    .await
    .unwrap();
    fx.clock.advance(TimeDelta::hours(2));
    let next = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 5),
        EventChoice::New {
            replacing: first.report.event_id,
        },
    )
    .await
    .unwrap();

    let old = event(fx, first.report.event_id.unwrap()).await;
    assert_eq!(
        old.closed,
        Some(Closure {
            at: last.report.observed_at,
            reason: CloseReason::NewEvent
        })
    );
    let open = fx.store.open_event().await.unwrap().unwrap();
    assert_eq!(Some(open.id), next.report.event_id);
    assert_eq!(open.owner_id, p.guest.id);
}

pub async fn a_stale_answer_is_refused_and_changes_nothing<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + AuditRepo,
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
    fx.clock.advance(TimeDelta::minutes(5));
    // The partner's phone answered "No, it had stopped" first.
    log(
        fx,
        &p.partner,
        smell(fx.clock.now(), 5),
        EventChoice::New {
            replacing: first.report.event_id,
        },
    )
    .await
    .unwrap();
    let entries = fx.store.audit_log().await.unwrap().len();

    let late_yes = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 6),
        EventChoice::Join(first.report.event_id.unwrap()),
    )
    .await;
    assert!(matches!(late_yes, Err(LogError::Stale)), "{late_yes:?}");
    let late_continue = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 6),
        EventChoice::Continue(first.report.event_id.unwrap()),
    )
    .await;
    assert!(
        matches!(late_continue, Err(LogError::Stale)),
        "{late_continue:?}"
    );
    let unknown = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 6),
        EventChoice::Join(EventId(999)),
    )
    .await;
    assert!(matches!(unknown, Err(LogError::Stale)), "{unknown:?}");
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
}

pub async fn a_new_event_answer_about_an_event_that_is_no_longer_open_is_stale<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + AuditRepo,
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
    let first_id = first.report.event_id.unwrap();
    fx.clock.advance(TimeDelta::minutes(5));
    let second = log(
        fx,
        &p.partner,
        smell(fx.clock.now(), 5),
        EventChoice::New {
            replacing: Some(first_id),
        },
    )
    .await
    .unwrap();
    let entries = fx.store.audit_log().await.unwrap().len();

    let late = log(
        fx,
        &p.resident,
        smell(fx.clock.now(), 6),
        EventChoice::New {
            replacing: Some(first_id),
        },
    )
    .await;
    assert!(matches!(late, Err(LogError::Stale)), "{late:?}");
    let unaware = log(
        fx,
        &p.resident,
        smell(fx.clock.now(), 6),
        EventChoice::New { replacing: None },
    )
    .await;
    assert!(matches!(unaware, Err(LogError::Stale)), "{unaware:?}");
    assert_eq!(
        fx.store.open_event().await.unwrap().map(|e| e.id),
        second.report.event_id
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
}

pub async fn a_backdated_new_event_is_open_only_if_nothing_is_and_no_four_am_passed<S>(
    fx: &Fixture<S>,
) where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    // 21:00 in Rome: an hour ago is the same evening.
    let earlier_tonight = log(
        fx,
        &p.resident,
        smell(start() - TimeDelta::hours(1), 3),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    assert!(
        event(fx, earlier_tonight.report.event_id.unwrap())
            .await
            .is_open()
    );

    let earlier_still = start() - TimeDelta::hours(2);
    let while_open = log(
        fx,
        &p.resident,
        smell(earlier_still, 3),
        EventChoice::New {
            replacing: earlier_tonight.report.event_id,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        event(fx, while_open.report.event_id.unwrap()).await.closed,
        Some(Closure {
            at: earlier_still,
            reason: CloseReason::Backdated
        }),
        "another event is open"
    );

    let yesterday = start() - TimeDelta::days(1);
    let past_four = log(
        fx,
        &p.resident,
        smell(yesterday, 3),
        EventChoice::New {
            replacing: earlier_tonight.report.event_id,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        event(fx, past_four.report.event_id.unwrap())
            .await
            .closed
            .map(|c| c.reason),
        Some(CloseReason::Backdated)
    );
    assert!(
        event(fx, earlier_tonight.report.event_id.unwrap())
            .await
            .is_open(),
        "a past report never closes the open event"
    );
}

pub async fn a_backdated_report_after_the_end_moves_the_end<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + AuditRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(5);
    let closed = Closure {
        at: opened + TimeDelta::hours(1),
        reason: CloseReason::Manual,
    };
    let past = add_event(fx, p.resident.id, opened, Some(closed)).await;

    let inside = log(
        fx,
        &p.guest,
        smell(opened + TimeDelta::minutes(30), 3),
        EventChoice::Join(past.id),
    )
    .await
    .unwrap();
    assert_eq!(inside.report.event_id, Some(past.id));
    assert_eq!(event(fx, past.id).await.closed, Some(closed));

    let after = opened + TimeDelta::hours(2);
    log(fx, &p.guest, smell(after, 3), EventChoice::Join(past.id))
        .await
        .unwrap();
    assert_eq!(
        event(fx, past.id).await.closed,
        Some(Closure {
            at: after,
            reason: CloseReason::Manual
        })
    );
}

pub async fn a_backdated_report_cannot_join_an_event_that_had_not_started<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let open = add_event(fx, p.resident.id, start() - TimeDelta::hours(1), None).await;
    let before = log(
        fx,
        &p.resident,
        smell(start() - TimeDelta::hours(2), 3),
        EventChoice::Join(open.id),
    )
    .await;
    assert!(matches!(before, Err(LogError::OutsideEvent)), "{before:?}");

    let yesterday = start() - TimeDelta::days(1);
    let old = add_event(
        fx,
        p.resident.id,
        yesterday,
        Some(Closure {
            at: yesterday,
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let next_day = log(
        fx,
        &p.resident,
        smell(start() - TimeDelta::hours(2), 3),
        EventChoice::Join(old.id),
    )
    .await;
    assert!(
        matches!(next_day, Err(LogError::OutsideEvent)),
        "{next_day:?}"
    );
}

pub async fn continuing_an_auto_closed_event_reopens_it<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(20);
    let auto = Closure {
        at: opened + TimeDelta::hours(1),
        reason: CloseReason::Auto0400,
    };
    let guests = add_event(fx, p.guest.id, opened, Some(auto)).await;

    let logged = log(
        fx,
        &p.partner,
        smell(start(), 5),
        EventChoice::Continue(guests.id),
    )
    .await
    .unwrap();
    assert_eq!(logged.report.event_id, Some(guests.id));
    assert!(event(fx, guests.id).await.is_open());

    let manual = add_event(
        fx,
        p.resident.id,
        opened - TimeDelta::hours(1),
        Some(Closure {
            at: opened,
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let not_auto = log(
        fx,
        &p.resident,
        smell(start(), 5),
        EventChoice::Continue(manual.id),
    )
    .await;
    assert!(matches!(not_auto, Err(LogError::Stale)), "{not_auto:?}");
}

pub async fn a_guest_cannot_continue_someone_elses_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(20);
    let residents = add_event(
        fx,
        p.resident.id,
        opened,
        Some(Closure {
            at: opened,
            reason: CloseReason::Auto0400,
        }),
    )
    .await;
    let refused = log(
        fx,
        &p.guest,
        smell(start(), 5),
        EventChoice::Continue(residents.id),
    )
    .await;
    assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
    assert!(!event(fx, residents.id).await.is_open());
}

pub async fn a_reading_with_a_report_joins_the_reports_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + ReadingRepo,
{
    let p = people(fx).await;
    let mut entry = smell(start(), 4);
    entry.pm25 = Some(Pm25::new(35.5).unwrap());
    let logged = log(fx, &p.resident, entry, EventChoice::New { replacing: None })
        .await
        .unwrap();
    let reading = logged.reading.unwrap();
    assert_eq!(reading.event_id, logged.report.event_id);
    assert_eq!(
        (reading.observed_at, reading.user_id),
        (start(), p.resident.id)
    );
    assert_eq!(
        fx.store
            .readings_for_event(logged.report.event_id.unwrap())
            .await
            .unwrap(),
        [reading]
    );
}

pub async fn a_reading_alone_joins_the_event_containing_its_time<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(5);
    let past = add_event(
        fx,
        p.resident.id,
        opened,
        Some(Closure {
            at: opened + TimeDelta::hours(1),
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let open = add_event(fx, p.resident.id, start() - TimeDelta::minutes(30), None).await;

    let inside = reading(fx, &p.guest, opened + TimeDelta::minutes(10)).await;
    assert_eq!(inside.event_id, Some(past.id));
    let between = reading(fx, &p.guest, opened + TimeDelta::hours(2)).await;
    assert_eq!(between.event_id, None);
    let now = reading(fx, &p.guest, start()).await;
    assert_eq!(now.event_id, Some(open.id));

    assert_eq!(
        event(fx, past.id).await.closed.map(|c| c.at),
        Some(opened + TimeDelta::hours(1)),
        "a reading never moves an end"
    );
    assert_eq!(
        fx.store.open_event().await.unwrap().map(|e| e.id),
        Some(open.id)
    );
}

pub async fn a_failed_report_rolls_back_the_new_event_close<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + AuditRepo,
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
    let entries = fx.store.audit_log().await.unwrap().len();
    fx.clock.advance(TimeDelta::minutes(30));

    let mut tx = fx.store.begin(Some(p.guest.id)).await.unwrap();
    let mut entry = smell(fx.clock.now(), 5);
    entry.values = report_values(5, &[RoomId(999)], &[]);
    let failed = log_smell(
        &mut tx,
        &p.guest.actor(),
        entry,
        EventChoice::New {
            replacing: first.report.event_id,
        },
        &rome(),
        fx.clock.now(),
    )
    .await;
    assert!(matches!(failed, Err(LogError::Store(_))), "{failed:?}");
    drop(tx);

    assert_eq!(
        fx.store.open_event().await.unwrap().map(|e| e.id),
        first.report.event_id,
        "the close and the new event were rolled back with the report"
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
}

pub async fn only_people_who_may_log_can_log<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx + UserWrites> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let viewer = log(
        fx,
        &p.viewer,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await;
    assert!(matches!(viewer, Err(LogError::Forbidden)), "{viewer:?}");

    let mut tx = fx.store.begin(Some(p.resident.id)).await.unwrap();
    let expired = tx.expire_user(p.guest.id).await.unwrap();
    tx.commit().await.unwrap();
    let guest = log(
        fx,
        &expired,
        smell(start(), 4),
        EventChoice::New { replacing: None },
    )
    .await;
    assert!(matches!(guest, Err(LogError::Forbidden)), "{guest:?}");

    let mut tx = fx.store.begin(Some(p.viewer.id)).await.unwrap();
    let alone = log_reading(
        &mut tx,
        &p.viewer.actor(),
        start(),
        Pm25::new(1.0).unwrap(),
        start(),
    )
    .await;
    assert!(matches!(alone, Err(LogError::Forbidden)), "{alone:?}");
    drop(tx);
    assert_eq!(fx.store.open_event().await.unwrap(), None);
}

pub async fn a_time_in_the_future_is_refused<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo,
{
    let p = people(fx).await;
    let future = log(
        fx,
        &p.resident,
        smell(start() + TimeDelta::minutes(5), 4),
        EventChoice::New { replacing: None },
    )
    .await;
    assert!(
        matches!(
            future,
            Err(LogError::Invalid(DomainError::ObservedInFuture))
        ),
        "{future:?}"
    );
}

pub async fn a_no_smell_report_with_nothing_open_is_a_check_outside_any_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + ReportRepo,
{
    let p = people(fx).await;
    let logged = log(
        fx,
        &p.guest,
        no_smell(start()),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    assert_eq!(logged.report.event_id, None);
    assert!(logged.report.values.rooms().is_empty(), "the whole home");
    assert_eq!(fx.store.open_event().await.unwrap(), None);
    assert_eq!(fx.store.event(EventId(1)).await.unwrap(), None);
    let listed = fx
        .store
        .reports_between(start(), start() + TimeDelta::minutes(1))
        .await
        .unwrap();
    assert_eq!(listed, [logged.report]);
}

pub async fn a_no_smell_report_joins_the_open_event_without_closing_it<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let first = log(
        fx,
        &p.resident,
        smell(start(), 6),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    fx.clock.advance(TimeDelta::minutes(30));
    let clean = SmellEntry {
        values: report_values(0, &[FIRST_ROOM], &[]),
        ..smell(fx.clock.now(), 0)
    };
    let answer = EventChoice::New {
        replacing: first.report.event_id,
    };
    let joined = log(fx, &p.guest, clean, answer).await.unwrap();
    assert_eq!(joined.report.event_id, first.report.event_id);
    let open = fx.store.open_event().await.unwrap().unwrap();
    assert_eq!(Some(open.id), first.report.event_id);
}

pub async fn a_backdated_no_smell_report_joins_the_event_picked_or_none<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let past = add_event(fx, p.resident.id, start() - TimeDelta::hours(3), None).await;
    let earlier = start() - TimeDelta::hours(2);
    let joined = log(
        fx,
        &p.resident,
        no_smell(earlier),
        EventChoice::Join(past.id),
    )
    .await
    .unwrap();
    assert_eq!(joined.report.event_id, Some(past.id));

    let before = start() - TimeDelta::hours(4);
    let check = log(
        fx,
        &p.resident,
        no_smell(before),
        EventChoice::New { replacing: None },
    )
    .await
    .unwrap();
    assert_eq!(check.report.event_id, None);
    assert_eq!(
        fx.store.events_between(before, start()).await.unwrap(),
        [event(fx, past.id).await],
        "no event was created"
    );
}

pub async fn a_no_smell_report_answered_continue_now_is_a_check_and_reopens_nothing<S>(
    fx: &Fixture<S>,
) where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(20);
    let auto = Closure {
        at: opened + TimeDelta::hours(1),
        reason: CloseReason::Auto0400,
    };
    let closed = add_event(fx, p.resident.id, opened, Some(auto)).await;

    let logged = log(
        fx,
        &p.resident,
        no_smell(start()),
        EventChoice::Continue(closed.id),
    )
    .await
    .unwrap();
    assert_eq!(logged.report.event_id, None, "a check");
    assert_eq!(
        event(fx, closed.id).await.closed,
        Some(auto),
        "still closed"
    );
    assert_eq!(fx.store.open_event().await.unwrap(), None);
}

pub async fn a_backdated_no_smell_report_cannot_continue_an_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(20);
    let auto = Closure {
        at: opened + TimeDelta::hours(1),
        reason: CloseReason::Auto0400,
    };
    let closed = add_event(fx, p.resident.id, opened, Some(auto)).await;

    let refused = log(
        fx,
        &p.resident,
        no_smell(start() - TimeDelta::hours(2)),
        EventChoice::Continue(closed.id),
    )
    .await;
    assert!(matches!(refused, Err(LogError::Stale)), "{refused:?}");
    assert_eq!(event(fx, closed.id).await.closed, Some(auto));
}

pub async fn a_backdated_no_smell_report_after_the_end_moves_the_end<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(5);
    let closed = Closure {
        at: opened + TimeDelta::hours(1),
        reason: CloseReason::Manual,
    };
    let past = add_event(fx, p.resident.id, opened, Some(closed)).await;

    let inside = log(
        fx,
        &p.resident,
        no_smell(opened + TimeDelta::minutes(30)),
        EventChoice::Join(past.id),
    )
    .await
    .unwrap();
    assert_eq!(inside.report.event_id, Some(past.id));
    assert_eq!(event(fx, past.id).await.closed, Some(closed), "end kept");

    let after = opened + TimeDelta::hours(2);
    log(fx, &p.resident, no_smell(after), EventChoice::Join(past.id))
        .await
        .unwrap();
    assert_eq!(
        event(fx, past.id).await.closed,
        Some(Closure {
            at: after,
            reason: CloseReason::Manual
        }),
        "spec 7.5: the end moves to the report time"
    );
}
