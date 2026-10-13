//! Smell reports: creating, correcting, moving and soft-deleting.

use chrono::TimeDelta;
use serde_json::json;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::Clock;
use smoke_domain::records::{Closure, NewReport};
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, EventWrites, ListRepo, ListWrites, ReportRepo, ReportWrites,
    StoreError, Transactional, UserWrites,
};
use smoke_domain::types::{CloseReason, EventId, RoomId, SymptomId};

use crate::Fixture;
use crate::fixture::{add_event, add_report, last_entry, report_values, scene, start};

pub async fn a_report_keeps_its_rooms_and_symptoms<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo,
{
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let observed_at = start() - TimeDelta::minutes(2);
    let report = add_report(
        fx,
        &scene,
        observed_at,
        report_values(6, &[r2, r0, r2], &[scene.symptoms[0].id]),
    )
    .await;

    assert_eq!(report.values.rooms(), [r0, r2]);
    assert_eq!(report.values.symptoms(), [scene.symptoms[0].id]);
    assert_eq!(
        (
            report.observed_at,
            report.recorded_at,
            report.event_id,
            report.user_id
        ),
        (observed_at, start(), scene.event.id, scene.user.id)
    );
    assert_eq!(
        fx.store.report(report.id).await.unwrap(),
        Some(report.clone())
    );
    assert_eq!(
        fx.store.reports_for_event(scene.event.id).await.unwrap(),
        [report]
    );
}

pub async fn correcting_a_report_records_before_and_after<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let report = add_report(fx, &scene, start(), report_values(6, &[r0], &[])).await;
    fx.clock.advance(TimeDelta::hours(13));

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let corrected = tx
        .correct_report(
            report.id,
            report.observed_at,
            report_values(7, &[r0, r2], &[]),
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(corrected.values.intensity().get(), 7);
    assert_eq!(corrected.recorded_at, report.recorded_at);
    let entry = last_entry(fx).await.record;
    assert_eq!(entry.action, AuditAction::Update);
    assert_eq!(entry.at_micros, fx.clock.now().timestamp_micros());
    let before = entry.before.unwrap();
    let after = entry.after.unwrap();
    assert_eq!(before["intensity"], 6);
    assert_eq!(after["intensity"], 7);
    assert_eq!(after["rooms"], json!([r0.0, r2.0]));
}

pub async fn moving_a_report_is_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let yesterday = start() - TimeDelta::days(1);
    let other = add_event(
        fx,
        scene.user.id,
        yesterday,
        Some(Closure {
            at: yesterday,
            reason: CloseReason::Backdated,
        }),
    )
    .await;
    let report = add_report(fx, &scene, start(), report_values(4, &[], &[])).await;

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let moved = tx.move_report(report.id, other.id).await.unwrap();
    assert_eq!(moved.event_id, other.id);
    tx.move_report(report.id, other.id).await.unwrap();
    tx.commit().await.unwrap();

    assert!(
        fx.store
            .reports_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.reports_for_event(other.id).await.unwrap(), [moved]);
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::SmellReport, report.id.0)
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(actions, [AuditAction::Create, AuditAction::Move]);
}

pub async fn deleting_a_report_is_soft<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let report = add_report(fx, &scene, start(), report_values(5, &[], &[])).await;
    fx.clock.advance(TimeDelta::minutes(1));

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let deleted = tx.delete_report(report.id).await.unwrap();
    assert_eq!(deleted.deleted_at, Some(fx.clock.now()));
    for error in [
        tx.delete_report(report.id).await.unwrap_err(),
        tx.correct_report(report.id, start(), report_values(6, &[], &[]))
            .await
            .unwrap_err(),
        tx.move_report(report.id, scene.event.id).await.unwrap_err(),
    ] {
        assert!(
            matches!(error, StoreError::Conflict(Conflict::AlreadyDeleted)),
            "{error:?}"
        );
    }
    tx.commit().await.unwrap();

    assert_eq!(fx.store.report(report.id).await.unwrap(), Some(deleted));
    assert!(
        fx.store
            .reports_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(last_entry(fx).await.record.action, AuditAction::Delete);
}

pub async fn latest_report_time_ignores_deleted_reports<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites> + ListRepo,
{
    let scene = scene(fx).await;
    let earlier = start() - TimeDelta::minutes(30);
    let later = start() - TimeDelta::minutes(10);
    add_report(fx, &scene, earlier, report_values(3, &[], &[])).await;
    let last = add_report(fx, &scene, later, report_values(5, &[], &[])).await;

    let mut tx = fx.store.begin(None).await.unwrap();
    assert_eq!(
        tx.latest_report_at(scene.event.id).await.unwrap(),
        Some(later)
    );
    tx.delete_report(last.id).await.unwrap();
    assert_eq!(
        tx.latest_report_at(scene.event.id).await.unwrap(),
        Some(earlier)
    );
    let empty = tx
        .create_event(smoke_domain::records::NewEvent {
            owner_id: scene.user.id,
            opened_at: start() - TimeDelta::days(2),
            closed: Some(Closure {
                at: start() - TimeDelta::days(2),
                reason: CloseReason::Backdated,
            }),
        })
        .await
        .unwrap();
    assert_eq!(tx.latest_report_at(empty.id).await.unwrap(), None);
}

pub async fn an_unknown_room_rolls_back_the_report<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx
        .create_report(NewReport {
            event_id: scene.event.id,
            user_id: scene.user.id,
            observed_at: start(),
            values: report_values(5, &[RoomId(999)], &[]),
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
            .reports_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
}

pub async fn correcting_a_report_can_remove_rooms_and_symptoms<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let report = add_report(
        fx,
        &scene,
        start(),
        report_values(6, &[r0, r2], &[scene.symptoms[0].id]),
    )
    .await;

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    tx.correct_report(report.id, report.observed_at, report_values(6, &[r2], &[]))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let stored = fx.store.report(report.id).await.unwrap().unwrap();
    assert_eq!(stored.values.rooms(), [r2]);
    assert!(stored.values.symptoms().is_empty());
    let after = last_entry(fx).await.record.after.unwrap();
    assert_eq!(after["rooms"], json!([r2.0]));
    assert_eq!(after["symptoms"], json!([]));
}

pub async fn an_unknown_room_on_correction_keeps_the_original_links<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let report = add_report(fx, &scene, start(), report_values(6, &[r0, r2], &[])).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx
        .correct_report(
            report.id,
            report.observed_at,
            report_values(8, &[r0, RoomId(999)], &[]),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
    drop(tx);

    assert_eq!(fx.store.report(report.id).await.unwrap(), Some(report));
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
}

pub async fn moving_a_report_to_an_unknown_event_is_rejected<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let report = add_report(fx, &scene, start(), report_values(4, &[], &[])).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx.move_report(report.id, EventId(999)).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
    drop(tx);

    assert_eq!(fx.store.report(report.id).await.unwrap(), Some(report));
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
}

pub async fn an_unknown_symptom_rolls_back_the_report<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx
        .create_report(NewReport {
            event_id: scene.event.id,
            user_id: scene.user.id,
            observed_at: start(),
            values: report_values(5, &[], &[SymptomId(999)]),
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
            .reports_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
}

pub async fn reports_between_returns_live_reports_with_their_links<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo,
{
    let scene = scene(fx).await;
    let (r0, r1, s0) = (scene.rooms[0].id, scene.rooms[1].id, scene.symptoms[0].id);
    let at = |minutes: i64| start() - TimeDelta::minutes(minutes);
    let before = add_report(fx, &scene, at(50), report_values(2, &[r0], &[])).await;
    let first = add_report(fx, &scene, at(40), report_values(5, &[r1, r0], &[s0])).await;
    let second = add_report(fx, &scene, at(30), report_values(6, &[], &[])).await;
    let deleted = add_report(fx, &scene, at(20), report_values(7, &[r0], &[s0])).await;
    let at_end = add_report(fx, &scene, at(10), report_values(8, &[r1], &[])).await;
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.delete_report(deleted.id).await.unwrap();
    tx.commit().await.unwrap();

    let found = fx.store.reports_between(at(40), at(10)).await.unwrap();
    assert_eq!(found, [first.clone(), second]);
    assert_eq!(found[0].values.rooms(), [r0, r1]);
    assert_eq!(found[0].values.symptoms(), [s0]);
    assert!(!found.contains(&before) && !found.contains(&at_end));
    assert!(
        fx.store
            .reports_between(at(5), start())
            .await
            .unwrap()
            .is_empty()
    );
}
