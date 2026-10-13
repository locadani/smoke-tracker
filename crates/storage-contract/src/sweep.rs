//! The 04:00 auto-close sweep on a real store (spec §11, §13.3: "sweep at 04:00 vs catch-up at
//! 08:00; DST days").

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::Clock;
use smoke_domain::records::{Event, NewReport};
use smoke_domain::repos::{
    AuditRepo, Commit, EventRepo, EventWrites, JobRepo, JobWrites, ListRepo, ListWrites,
    ReportWrites, Transactional, UserWrites,
};
use smoke_domain::schedule::DailySchedule;
use smoke_domain::starter::seed_starter_lists;
use smoke_domain::sweep::{AUTO_CLOSE_JOB, SweepOutcome, auto_close};
use smoke_domain::types::{CloseReason, Role, UserId};

use crate::Fixture;
use crate::fixture::{FIRST_ROOM, add_event, add_user, last_entry, report_values};

fn t(s: &str) -> DateTime<Utc> {
    s.parse().expect("valid instant")
}

fn rome() -> DailySchedule {
    DailySchedule::new(chrono_tz::Europe::Rome, 4).expect("valid schedule")
}

async fn report_at<S>(fx: &Fixture<S>, user: UserId, event: &Event, at: DateTime<Utc>)
where
    S: Transactional<Tx: ReportWrites>,
{
    let mut tx = fx.store.begin(Some(user)).await.unwrap();
    tx.create_report(NewReport {
        event_id: Some(event.id),
        user_id: user,
        observed_at: at,
        values: report_values(5, &[FIRST_ROOM], &[]),
    })
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

async fn sweep_at<S>(fx: &Fixture<S>, now: DateTime<Utc>) -> SweepOutcome
where
    S: Transactional<Tx: EventWrites + JobWrites> + JobRepo,
{
    fx.clock.set(now);
    auto_close(&fx.store, &rome(), fx.clock.now())
        .await
        .unwrap()
}

/// An evening event (20:00–21:30 UTC) on the night before `date`, with no sweep since.
async fn evening_event<S>(fx: &Fixture<S>, user: UserId, date: &str) -> Event
where
    S: Transactional<Tx: EventWrites + ReportWrites + JobWrites + ListWrites> + JobRepo + ListRepo,
{
    seed_starter_lists(&fx.store).await.unwrap();
    let opened = t(&format!("{date}T20:00:00Z")) - TimeDelta::days(1);
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.record_job_run(AUTO_CLOSE_JOB, opened).await.unwrap();
    tx.commit().await.unwrap();
    let event = add_event(fx, user, opened, None).await;
    report_at(fx, user, &event, opened).await;
    report_at(fx, user, &event, opened + TimeDelta::minutes(90)).await;
    event
}

pub async fn the_first_sweep_runs_and_is_remembered<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: EventWrites + JobWrites> + JobRepo,
{
    let now = t("2026-01-15T12:00:00Z");
    assert_eq!(sweep_at(fx, now).await, SweepOutcome::Ran { closed: None });
    assert_eq!(
        fx.store.job_last_run(AUTO_CLOSE_JOB).await.unwrap(),
        Some(now)
    );
    assert_eq!(
        sweep_at(fx, now + TimeDelta::hours(10)).await,
        SweepOutcome::NotDue
    );
}

pub async fn a_sweep_at_four_closes_the_event_at_its_last_report<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites + ListWrites>
        + JobRepo
        + ListRepo
        + EventRepo
        + AuditRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let event = evening_event(fx, user.id, "2026-01-16").await;
    assert_eq!(
        sweep_at(fx, t("2026-01-16T02:59:59Z")).await,
        SweepOutcome::NotDue,
        "03:59:59 in Rome"
    );
    assert_eq!(
        sweep_at(fx, t("2026-01-16T03:00:00Z")).await,
        SweepOutcome::Ran {
            closed: Some(event.id)
        }
    );
    let closed = fx
        .store
        .event(event.id)
        .await
        .unwrap()
        .unwrap()
        .closed
        .unwrap();
    assert_eq!(closed.at, event.opened_at + TimeDelta::minutes(90));
    assert_eq!(closed.reason, CloseReason::Auto0400);
    let entry = last_entry(fx).await.record;
    assert_eq!(
        (entry.entity, entry.action, entry.actor_id),
        (AuditEntity::Event, AuditAction::Close, None)
    );
}

pub async fn a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites + ListWrites>
        + JobRepo
        + ListRepo
        + EventRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let first = evening_event(fx, user.id, "2026-01-16").await;
    sweep_at(fx, t("2026-01-16T03:00:00Z")).await;
    let second = evening_event(fx, user.id, "2026-01-18").await;
    assert_eq!(
        sweep_at(fx, t("2026-01-18T07:00:00Z")).await,
        SweepOutcome::Ran {
            closed: Some(second.id)
        },
        "08:00 in Rome, after the host slept through 04:00"
    );
    let span = |event: Event| {
        let closed = event.closed.unwrap();
        (closed.at - event.opened_at, closed.reason)
    };
    let at_four = span(fx.store.event(first.id).await.unwrap().unwrap());
    let at_eight = span(fx.store.event(second.id).await.unwrap().unwrap());
    assert_eq!(at_four, at_eight);
    assert_eq!(at_eight, (TimeDelta::minutes(90), CloseReason::Auto0400));
}

pub async fn an_event_opened_after_the_boundary_stays_open<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + JobWrites> + JobRepo + EventRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.record_job_run(AUTO_CLOSE_JOB, t("2026-01-15T12:00:00Z"))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let early_morning = add_event(fx, user.id, t("2026-01-16T04:00:00Z"), None).await;
    assert_eq!(
        sweep_at(fx, t("2026-01-16T06:00:00Z")).await,
        SweepOutcome::Ran { closed: None }
    );
    assert!(
        fx.store
            .event(early_morning.id)
            .await
            .unwrap()
            .unwrap()
            .is_open()
    );
}

pub async fn an_event_without_reports_closes_where_it_opened<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + JobWrites> + JobRepo + EventRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let opened = t("2026-01-15T20:00:00Z");
    let event = add_event(fx, user.id, opened, None).await;
    sweep_at(fx, t("2026-01-16T09:00:00Z")).await;
    let closed = fx.store.event(event.id).await.unwrap().unwrap().closed;
    assert_eq!(closed.map(|c| c.at), Some(opened));
}

pub async fn on_dst_days_the_sweep_still_runs_at_four_local<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites + ListWrites>
        + JobRepo
        + ListRepo
        + EventRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    // Spring forward: 04:00 CEST is 02:00 UTC. Fall back: 04:00 CET is 03:00 UTC.
    for (date, before, at_four) in [
        ("2026-03-29", "2026-03-29T01:59:00Z", "2026-03-29T02:00:00Z"),
        ("2026-10-25", "2026-10-25T02:59:00Z", "2026-10-25T03:00:00Z"),
    ] {
        let event = evening_event(fx, user.id, date).await;
        assert_eq!(
            sweep_at(fx, t(before)).await,
            SweepOutcome::NotDue,
            "{date}"
        );
        assert_eq!(
            sweep_at(fx, t(at_four)).await,
            SweepOutcome::Ran {
                closed: Some(event.id)
            },
            "{date}"
        );
    }
}
