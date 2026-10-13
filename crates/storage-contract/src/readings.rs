//! PM2.5 readings, and job bookkeeping.

use chrono::TimeDelta;
use serde_json::json;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::Clock;
use smoke_domain::records::NewReading;
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, EventWrites, JobRepo, JobWrites, ListRepo, ListWrites,
    ReadingRepo, ReadingWrites, SnapshotRepo, StoreError, Transactional, UserWrites,
};
use smoke_domain::types::{EventId, Pm25};
use smoke_domain::verify::verify_store;

use crate::Fixture;
use crate::fixture::{last_entry, scene, start};

fn pm25(value: f64) -> Pm25 {
    Pm25::new(value).expect("valid PM2.5")
}

pub async fn a_reading_can_stand_alone_or_join_an_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReadingWrites>
        + ListRepo
        + ReadingRepo,
{
    let scene = scene(fx).await;
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let alone = tx
        .create_reading(NewReading {
            user_id: scene.user.id,
            event_id: None,
            observed_at: start() - TimeDelta::days(1),
            value: pm25(8.0),
        })
        .await
        .unwrap();
    let linked = tx
        .create_reading(NewReading {
            user_id: scene.user.id,
            event_id: Some(scene.event.id),
            observed_at: start(),
            value: pm25(35.5),
        })
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(alone.event_id, None);
    assert_eq!(linked.recorded_at, start());
    assert_eq!(fx.store.reading(alone.id).await.unwrap(), Some(alone));
    assert_eq!(
        fx.store.readings_for_event(scene.event.id).await.unwrap(),
        [linked]
    );
}

pub async fn correcting_and_deleting_a_reading_are_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReadingWrites>
        + ListRepo
        + ReadingRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let reading = tx
        .create_reading(NewReading {
            user_id: scene.user.id,
            event_id: Some(scene.event.id),
            observed_at: start(),
            value: pm25(12.5),
        })
        .await
        .unwrap();
    let corrected = tx
        .correct_reading(reading.id, reading.observed_at, pm25(14.0))
        .await
        .unwrap();
    assert_eq!(corrected.value, pm25(14.0));
    fx.clock.advance(TimeDelta::minutes(1));
    let deleted = tx.delete_reading(reading.id).await.unwrap();
    assert_eq!(deleted.deleted_at, Some(fx.clock.now()));
    let error = tx
        .correct_reading(reading.id, start(), pm25(1.0))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::AlreadyDeleted)),
        "{error:?}"
    );
    let error = tx.delete_reading(reading.id).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::AlreadyDeleted)),
        "{error:?}"
    );
    tx.commit().await.unwrap();

    assert!(
        fx.store
            .readings_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.reading(reading.id).await.unwrap(), Some(deleted));
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::Pm25Reading, reading.id.0)
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(
        actions,
        [
            AuditAction::Create,
            AuditAction::Update,
            AuditAction::Delete
        ]
    );
}

pub async fn a_reading_for_an_unknown_event_is_rejected<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReadingWrites>
        + ListRepo
        + ReadingRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx
        .create_reading(NewReading {
            user_id: scene.user.id,
            event_id: Some(EventId(999)),
            observed_at: start(),
            value: pm25(10.0),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
    drop(tx);

    assert!(
        fx.store
            .readings_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
}

pub async fn pm25_values_round_trip_exactly<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReadingWrites>
        + ListRepo
        + ReadingRepo
        + AuditRepo
        + SnapshotRepo,
{
    let scene = scene(fx).await;
    for value in [
        0.0,
        0.1,
        12.3,
        999.99,
        1000.0,
        985.690_694_632_869_5,
        0.300_000_000_000_000_04,
    ] {
        let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
        let reading = tx
            .create_reading(NewReading {
                user_id: scene.user.id,
                event_id: None,
                observed_at: start(),
                value: pm25(value),
            })
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let stored = fx.store.reading(reading.id).await.unwrap().unwrap();
        assert_eq!(stored.value, pm25(value));
        assert_eq!(
            last_entry(fx).await.record.after.unwrap()["value"],
            json!(value)
        );
    }
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
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
