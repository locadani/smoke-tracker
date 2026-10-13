//! The hash chain and `verify-audit`, including tampering done directly in the database.

use chrono::TimeDelta;
use smoke_domain::audit::{AuditAction, AuditEntity, ChainError, verify_chain};
use smoke_domain::records::{NewReading, NewRoom, Pm25Reading, SmellReport};
use smoke_domain::repos::{Commit, ListWrites, ReadingWrites, ReportWrites, Store, StoreError};
use smoke_domain::types::{ListName, Pm25, RoomArea};
use smoke_domain::verify::{SnapshotProblem, verify_store};

use crate::Fixture;
use crate::fixture::{add_report, report_values, scene, start};

struct Populated {
    report: SmellReport,
    reading: Pm25Reading,
}

/// Seeding, a user, an event, a report (intensity 6), a reading and a correction.
async fn populate<S: Store>(fx: &Fixture<S>) -> Populated {
    let scene = scene(fx).await;
    let rooms = [scene.rooms[0].id, scene.rooms[1].id];
    let report = add_report(
        fx,
        &scene,
        start() - TimeDelta::minutes(1),
        report_values(6, &rooms[..1], &[scene.symptoms[0].id]),
    )
    .await;
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let reading = tx
        .create_reading(NewReading {
            user_id: scene.user.id,
            event_id: Some(scene.event.id),
            observed_at: start(),
            value: Pm25::new(42.0).unwrap(),
        })
        .await
        .unwrap();
    tx.correct_report(report.id, report.observed_at, report_values(6, &rooms, &[]))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    Populated { report, reading }
}

pub async fn every_write_extends_one_unbroken_chain<S: Store>(fx: &Fixture<S>) {
    populate(fx).await;
    let log = fx.store.audit_log().await.unwrap();
    let ids: Vec<i64> = log.iter().map(|e| e.record.id).collect();
    assert_eq!(ids, (1..=log.len() as i64).collect::<Vec<_>>());
    assert_eq!(verify_chain(&log), Ok(()));

    let report = verify_store(&fx.store).await.unwrap();
    assert!(report.is_ok(), "{report:?}");
    assert_eq!(report.entries, log.len());
    assert_eq!(report.head_hash, log.last().unwrap().hash);
}

pub async fn the_history_of_a_record_lists_its_entries<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::SmellReport, p.report.id.0)
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(actions, [AuditAction::Create, AuditAction::Update]);
}

pub async fn a_direct_record_edit_is_detected<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    let latest = fx
        .store
        .audit_history(AuditEntity::SmellReport, p.report.id.0)
        .await
        .unwrap()
        .pop()
        .unwrap();
    fx.raw("UPDATE smell_reports SET intensity = 9")
        .await
        .unwrap();

    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(report.chain, Ok(()));
    assert_eq!(
        report.problems,
        [SnapshotProblem::Differs {
            entity: AuditEntity::SmellReport,
            entity_id: p.report.id.0,
            audit_id: latest.record.id,
        }]
    );
}

pub async fn a_directly_inserted_record_is_detected<S: Store>(fx: &Fixture<S>) {
    populate(fx).await;
    fx.raw("INSERT INTO symptoms (name) VALUES ('Dizziness')")
        .await
        .unwrap();
    let inserted = fx
        .store
        .symptoms()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.name.as_str() == "Dizziness")
        .unwrap();

    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(
        report.problems,
        [SnapshotProblem::Unaudited {
            entity: AuditEntity::Symptom,
            entity_id: inserted.id.0,
        }]
    );
}

pub async fn a_directly_deleted_record_is_detected<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    fx.raw("DELETE FROM pm25_readings").await.unwrap();
    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(
        report.problems,
        [SnapshotProblem::Vanished {
            entity: AuditEntity::Pm25Reading,
            entity_id: p.reading.id.0,
        }]
    );
}

pub async fn the_audit_log_refuses_updates_and_deletes<S: Store>(fx: &Fixture<S>) {
    populate(fx).await;
    assert!(fx.raw("UPDATE audit_log SET hash = 'x'").await.is_err());
    assert!(fx.raw("DELETE FROM audit_log").await.is_err());
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
}

pub async fn an_edited_audit_entry_breaks_the_chain<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    let first = fx
        .store
        .audit_history(AuditEntity::SmellReport, p.report.id.0)
        .await
        .unwrap()
        .remove(0);
    fx.drop_audit_guard().await;
    fx.raw(
        r#"UPDATE audit_log SET after_json = replace(after_json, '"intensity":6', '"intensity":9') WHERE entity = 'smell_report'"#,
    )
    .await
    .unwrap();

    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(
        report.chain,
        Err(ChainError::HashMismatch {
            id: first.record.id
        })
    );
}

pub async fn concurrent_writers_never_fork_the_chain<S>(fx: &Fixture<S>)
where
    S: Store + Clone + 'static,
{
    let tasks: Vec<_> = (0..8)
        .map(|i| {
            let store = fx.store.clone();
            tokio::spawn(async move {
                let mut tx = store.begin(None).await?;
                tx.create_room(NewRoom {
                    name: ListName::new(&format!("Room {i}")).expect("valid name"),
                    i18n_key: None,
                    area: RoomArea::Inside,
                })
                .await?;
                tx.commit().await
            })
        })
        .collect();
    for task in tasks {
        task.await.expect("task panicked").expect("write");
    }
    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(log.len(), 8);
    assert_eq!(verify_chain(&log), Ok(()));
    assert_eq!(fx.store.rooms().await.unwrap().len(), 8);
}

pub async fn a_directly_edited_username_is_corrupt_not_normalised<S: Store>(fx: &Fixture<S>) {
    populate(fx).await;
    fx.raw("UPDATE users SET username = 'DANIELE'")
        .await
        .unwrap();

    let error = fx.store.users().await.unwrap_err();
    assert!(matches!(error, StoreError::Corrupt(_)), "{error:?}");
    let error = verify_store(&fx.store).await.unwrap_err();
    assert!(matches!(error, StoreError::Corrupt(_)), "{error:?}");
}

pub async fn a_directly_edited_list_name_is_corrupt_not_normalised<S: Store>(fx: &Fixture<S>) {
    populate(fx).await;
    fx.raw("UPDATE rooms SET name = '  Kitchen '")
        .await
        .unwrap();

    let error = fx.store.rooms().await.unwrap_err();
    assert!(matches!(error, StoreError::Corrupt(_)), "{error:?}");
}

pub async fn a_directly_edited_note_is_corrupt_not_normalised<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    fx.raw("UPDATE smell_reports SET note = '  padded  '")
        .await
        .unwrap();

    let error = fx.store.report(p.report.id).await.unwrap_err();
    assert!(matches!(error, StoreError::Corrupt(_)), "{error:?}");
}
