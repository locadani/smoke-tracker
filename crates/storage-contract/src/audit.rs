//! The hash chain and `verify-audit`, including tampering done directly in the database.

use chrono::TimeDelta;
use smoke_domain::audit::{AuditAction, AuditEntity, ChainError, verify_chain};
use smoke_domain::records::{Closure, LogEntry, NewRoom};
use smoke_domain::repos::{Commit, EntryWrites, ListWrites, Store, StoreError};
use smoke_domain::types::{CloseReason, ListName, Pm25, RoomArea};
use smoke_domain::verify::{SnapshotProblem, verify_store};

use crate::Fixture;
use crate::fixture::{
    SEED_ENTRY, add_entry, add_event, add_photo_reading, add_report, photo, report_values, scene,
    start,
};

struct Populated {
    report: LogEntry,
    reading: LogEntry,
}

/// Seeding, a user, an event, a report entry (one room at level 6), a reading entry and a
/// correction of the report.
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
    let reading = add_entry(
        fx,
        &scene,
        start(),
        None,
        None,
        Some(Pm25::new(42.0).unwrap()),
    )
    .await;
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    tx.correct_entry(
        report.id,
        report.observed_at,
        None,
        Some(report_values(6, &rooms, &[])),
    )
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
        .audit_history(AuditEntity::LogEntry, p.report.id.0)
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
        .audit_history(AuditEntity::LogEntry, p.report.id.0)
        .await
        .unwrap()
        .pop()
        .unwrap();
    fx.raw("UPDATE smell_report_rooms SET level = 9")
        .await
        .unwrap();

    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(report.chain, Ok(()));
    assert_eq!(
        report.problems,
        [SnapshotProblem::Differs {
            entity: AuditEntity::LogEntry,
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
    fx.raw(
        "DELETE FROM pm25_readings WHERE log_entry_id <> 1; \
         DELETE FROM log_entries WHERE id <> 1 \
           AND id NOT IN (SELECT log_entry_id FROM smell_reports)",
    )
    .await
    .unwrap();
    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(
        report.problems,
        [SnapshotProblem::Vanished {
            entity: AuditEntity::LogEntry,
            entity_id: p.reading.id.0,
        }]
    );
}

pub async fn soft_deletes_still_verify<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.delete_entry(p.report.id).await.unwrap();
    tx.delete_entry(p.reading.id).await.unwrap();
    tx.commit().await.unwrap();

    let report = verify_store(&fx.store).await.unwrap();
    assert!(report.is_ok(), "{report:?}");
}

pub async fn a_direct_link_table_edit_is_detected<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    let latest = fx
        .store
        .audit_history(AuditEntity::LogEntry, p.report.id.0)
        .await
        .unwrap()
        .pop()
        .unwrap();
    let stored = fx.store.entry(p.report.id).await.unwrap().unwrap();
    let in_event = fx
        .store
        .entries_for_event(stored.event_id.unwrap())
        .await
        .unwrap();
    assert_eq!(in_event.iter().filter(|e| e.id != SEED_ENTRY).count(), 2);
    let rooms = fx.store.rooms().await.unwrap();
    let target = rooms.last().unwrap();
    assert_eq!(target.i18n_key.as_deref(), Some("room-stairwell"));
    assert!(
        !stored
            .report
            .as_ref()
            .unwrap()
            .values
            .room_ids()
            .contains(&target.id)
    );
    // `populate` makes the only report, and the stairwell (the last room) is not linked to it.
    fx.raw(
        "INSERT INTO smell_report_rooms (smell_report_id, room_id, level) \
         SELECT (SELECT MIN(id) FROM smell_reports), (SELECT MAX(id) FROM rooms), 3",
    )
    .await
    .unwrap();

    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(report.chain, Ok(()));
    assert_eq!(
        report.problems,
        [SnapshotProblem::Differs {
            entity: AuditEntity::LogEntry,
            entity_id: p.report.id.0,
            audit_id: latest.record.id,
        }]
    );
}

pub async fn the_audit_log_refuses_updates_and_deletes<S: Store>(fx: &Fixture<S>) {
    populate(fx).await;
    let before = fx.store.audit_log().await.unwrap();
    assert!(fx.raw("UPDATE audit_log SET hash = 'x'").await.is_err());
    assert!(fx.raw("DELETE FROM audit_log").await.is_err());
    assert!(fx.raw("TRUNCATE audit_log").await.is_err());
    let after = fx.store.audit_log().await.unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(
        after.last().map(|e| &e.hash),
        before.last().map(|e| &e.hash)
    );
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
}

pub async fn an_edited_audit_entry_breaks_the_chain<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    let first = fx
        .store
        .audit_history(AuditEntity::LogEntry, p.report.id.0)
        .await
        .unwrap()
        .remove(0);
    fx.drop_audit_guard().await;
    fx.raw(
        r#"UPDATE audit_log SET after_json = replace(after_json, '"level":6', '"level":9') WHERE entity = 'log_entry'"#,
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
    // On SQLite the single writer connection already serialises callers, so this mainly
    // exercises Postgres's lock.
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
    fx.raw("UPDATE log_entries SET note = '  padded  '")
        .await
        .unwrap();

    let error = fx.store.entry(p.report.id).await.unwrap_err();
    assert!(matches!(error, StoreError::Corrupt(_)), "{error:?}");
}

/// A combined entry (report and reading) and a report-only entry, and a second event.
async fn combined_and_report_only<S: Store>(fx: &Fixture<S>) -> (LogEntry, LogEntry) {
    let scene = scene(fx).await;
    let combined = add_entry(
        fx,
        &scene,
        start() - TimeDelta::minutes(2),
        None,
        Some(report_values(5, &[scene.rooms[0].id], &[])),
        Some(Pm25::new(20.0).unwrap()),
    )
    .await;
    let report_only = add_report(
        fx,
        &scene,
        start() - TimeDelta::minutes(1),
        report_values(3, &[scene.rooms[1].id], &[]),
    )
    .await;
    let yesterday = start() - TimeDelta::days(1);
    add_event(
        fx,
        scene.user.id,
        yesterday,
        Some(Closure {
            at: yesterday,
            reason: CloseReason::Backdated,
        }),
    )
    .await;
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
    (combined, report_only)
}

async fn assert_differs<S: Store>(fx: &Fixture<S>, entries: &[&LogEntry]) {
    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(report.chain, Ok(()));
    let mut expected: Vec<_> = entries.iter().map(|e| e.id.0).collect();
    expected.sort_unstable();
    let mut found: Vec<_> = report
        .problems
        .iter()
        .map(|p| match p {
            SnapshotProblem::Differs {
                entity: AuditEntity::LogEntry,
                entity_id,
                ..
            } => *entity_id,
            other => panic!("unexpected problem {other:?}"),
        })
        .collect();
    found.sort_unstable();
    assert_eq!(found, expected, "{report:?}");
}

pub async fn a_direct_observed_time_edit_is_detected<S: Store>(fx: &Fixture<S>) {
    let (combined, report_only) = combined_and_report_only(fx).await;
    fx.raw("UPDATE log_entries SET observed_at = observed_at + 1000000 WHERE id = (SELECT MIN(id) FROM log_entries WHERE id <> 1)")
        .await
        .unwrap();
    assert!(combined.id.0 < report_only.id.0);
    assert_differs(fx, &[&combined]).await;
}

pub async fn a_direct_event_edit_is_detected<S: Store>(fx: &Fixture<S>) {
    let (combined, _) = combined_and_report_only(fx).await;
    fx.raw("UPDATE log_entries SET event_id = (SELECT MAX(id) FROM events) WHERE id = (SELECT MIN(id) FROM log_entries WHERE id <> 1)")
        .await
        .unwrap();
    assert_differs(fx, &[&combined]).await;
}

pub async fn a_direct_reading_value_edit_is_detected<S: Store>(fx: &Fixture<S>) {
    let (combined, _) = combined_and_report_only(fx).await;
    fx.raw("UPDATE pm25_readings SET value_ugm3 = value_ugm3 + 1 WHERE log_entry_id <> 1")
        .await
        .unwrap();
    assert_differs(fx, &[&combined]).await;
}

pub async fn relinking_a_reading_to_another_entry_is_detected<S: Store>(fx: &Fixture<S>) {
    let (combined, report_only) = combined_and_report_only(fx).await;
    fx.raw(
        "UPDATE pm25_readings SET log_entry_id = \
         (SELECT id FROM log_entries WHERE id NOT IN (SELECT log_entry_id FROM pm25_readings)) \
         WHERE log_entry_id <> 1",
    )
    .await
    .unwrap();
    assert_differs(fx, &[&combined, &report_only]).await;
}

pub async fn deleting_only_the_reading_of_a_combined_entry_is_detected<S: Store>(fx: &Fixture<S>) {
    let (combined, _) = combined_and_report_only(fx).await;
    fx.raw(
        "DELETE FROM pm25_readings WHERE log_entry_id IN (SELECT log_entry_id FROM smell_reports)",
    )
    .await
    .unwrap();
    assert_differs(fx, &[&combined]).await;
}

/// An entry whose reading has a photo, and an entry whose reading has none.
async fn photographed_and_plain<S: Store>(fx: &Fixture<S>) -> (LogEntry, LogEntry) {
    let scene = scene(fx).await;
    let photographed = add_photo_reading(
        fx,
        &scene,
        start() - TimeDelta::minutes(2),
        Pm25::new(20.0).unwrap(),
        photo(&"ab".repeat(32), start() - TimeDelta::minutes(3)),
    )
    .await;
    let plain = add_entry(
        fx,
        &scene,
        start() - TimeDelta::minutes(1),
        None,
        None,
        Some(Pm25::new(8.0).unwrap()),
    )
    .await;
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
    (photographed, plain)
}

pub async fn a_changed_photo_fingerprint_is_detected<S: Store>(fx: &Fixture<S>) {
    let (photographed, _) = photographed_and_plain(fx).await;
    fx.raw(
        "UPDATE pm25_photos SET sha256 = \
         'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff'",
    )
    .await
    .unwrap();
    assert_differs(fx, &[&photographed]).await;
}

pub async fn a_relinked_photo_is_detected<S: Store>(fx: &Fixture<S>) {
    let (photographed, plain) = photographed_and_plain(fx).await;
    fx.raw(
        "UPDATE pm25_photos SET pm25_reading_id = \
         (SELECT id FROM pm25_readings WHERE id NOT IN (SELECT pm25_reading_id FROM pm25_photos) \
            AND log_entry_id <> 1)",
    )
    .await
    .unwrap();
    assert_differs(fx, &[&photographed, &plain]).await;
}

pub async fn a_deleted_photo_row_is_detected<S: Store>(fx: &Fixture<S>) {
    let (photographed, _) = photographed_and_plain(fx).await;
    fx.raw("DELETE FROM pm25_photos").await.unwrap();
    assert_differs(fx, &[&photographed]).await;
}
