//! Log entries: creating, correcting, moving and soft-deleting a report, a reading or both.

use chrono::TimeDelta;
use serde_json::json;
use smoke_domain::DomainError;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::Clock;
use smoke_domain::records::{Closure, NewEntry, NewEvent, Note};
use smoke_domain::repos::{Commit, Conflict, EntryWrites, EventWrites, Store, StoreError};
use smoke_domain::types::{CloseReason, EventId, Pm25, RoomId, SymptomId};
use smoke_domain::verify::verify_store;

use crate::Fixture;
use crate::fixture::{
    EntryExt, FIRST_ROOM, add_entry, add_event, add_report, assert_no_childless_entries,
    last_entry, report_values, scene, start,
};

fn pm25(value: f64) -> Pm25 {
    Pm25::new(value).expect("valid PM2.5")
}

pub async fn a_report_keeps_its_rooms_and_symptoms<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let observed_at = start() - TimeDelta::minutes(2);
    let entry = add_report(
        fx,
        &scene,
        observed_at,
        report_values(6, &[r2, r0], &[scene.symptoms[0].id]),
    )
    .await;

    assert_eq!(entry.values().room_ids(), [r0, r2]);
    assert_eq!(entry.values().symptoms(), [scene.symptoms[0].id]);
    assert_eq!(
        (
            entry.observed_at,
            entry.recorded_at,
            entry.event_id,
            entry.user_id
        ),
        (observed_at, start(), Some(scene.event.id), scene.user.id)
    );
    assert_eq!(entry.reading, None);
    assert_eq!(fx.store.entry(entry.id).await.unwrap(), Some(entry.clone()));
    assert_eq!(
        fx.store.entries_for_event(scene.event.id).await.unwrap(),
        [entry]
    );
}

pub async fn entry_with_report_and_reading_round_trips<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let note = "from the balcony";
    let entry = add_entry(
        fx,
        &scene,
        start() - TimeDelta::minutes(5),
        Some(note),
        Some(report_values(4, &[FIRST_ROOM], &[scene.symptoms[0].id])),
        Some(pm25(35.5)),
    )
    .await;

    let report = entry.report.clone().unwrap();
    let reading = entry.reading.clone().unwrap();
    assert_eq!(report.log_entry_id, entry.id);
    assert_eq!(reading.log_entry_id, entry.id);
    assert_eq!(reading.value, pm25(35.5));
    assert_eq!(entry.note, Note::parse(Some(note)).unwrap());
    assert_eq!(fx.store.entry(entry.id).await.unwrap(), Some(entry.clone()));
    assert_eq!(
        fx.store.entry_of_report(report.id).await.unwrap(),
        Some(entry.clone())
    );
    assert_eq!(
        fx.store.entry_of_reading(reading.id).await.unwrap(),
        Some(entry.clone())
    );
    assert_eq!(
        fx.store.entries_for_event(scene.event.id).await.unwrap(),
        [entry]
    );
    assert_no_childless_entries(fx).await;
}

pub async fn an_entry_cannot_be_created_without_a_child<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    assert_eq!(
        NewEntry::new(scene.user.id, None, start(), None, None, None),
        Err(DomainError::EntryWithoutChildren)
    );
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let childless = NewEntry {
        user_id: scene.user.id,
        event_id: Some(scene.event.id),
        observed_at: start(),
        note: Note::parse(Some("only a note")).unwrap(),
        report: None,
        reading: None,
    };
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx.create_entry(childless).await.unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::Invalid(DomainError::EntryWithoutChildren)
        ),
        "{error:?}"
    );
    drop(tx);

    assert!(
        fx.store
            .entries_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
    assert_no_childless_entries(fx).await;
}

pub async fn creating_a_combined_log_writes_one_log_entry_audit_row<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();
    let entry = add_entry(
        fx,
        &scene,
        start(),
        Some("smoky"),
        Some(report_values(5, &[FIRST_ROOM], &[scene.symptoms[0].id])),
        Some(pm25(20.0)),
    )
    .await;

    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(log.len(), entries_before + 1, "exactly one new audit row");
    let row = &last_entry(fx).await.record;
    assert_eq!(
        (row.entity, row.action, row.entity_id),
        (AuditEntity::LogEntry, AuditAction::Create, entry.id.0)
    );
    assert_eq!(row.before, None);
    let after = row.after.clone().unwrap();
    let report = entry.report.as_ref().unwrap();
    let reading = entry.reading.as_ref().unwrap();
    assert_eq!(after["note"], json!("smoky"));
    assert_eq!(
        after["report"],
        json!({
            "id": report.id.0,
            "rooms": [{ "room": FIRST_ROOM.0, "level": 5 }],
            "symptoms": [scene.symptoms[0].id.0],
        })
    );
    assert_eq!(
        after["reading"],
        json!({ "id": reading.id.0, "value": 20.0 })
    );
    assert!(
        log.iter().all(|e| !matches!(
            e.record.entity,
            AuditEntity::SmellReport | AuditEntity::Pm25Reading
        )),
        "no audit rows for the halves"
    );
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
}

pub async fn correcting_an_entry_records_before_and_after<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let entry = add_entry(
        fx,
        &scene,
        start(),
        None,
        Some(report_values(6, &[r0], &[])),
        Some(pm25(12.0)),
    )
    .await;
    fx.clock.advance(TimeDelta::hours(13));
    let earlier = start() - TimeDelta::minutes(7);

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let corrected = tx
        .correct_entry(
            entry.id,
            earlier,
            Note::parse(Some("corrected")).unwrap(),
            Some(report_values(7, &[r0, r2], &[])),
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(corrected.values().intensity().get(), 7);
    assert_eq!(corrected.recorded_at, entry.recorded_at);
    assert_eq!(corrected.observed_at, earlier);
    assert_eq!(
        corrected.reading, entry.reading,
        "the reading keeps its value and follows the entry's time"
    );
    let row = last_entry(fx).await.record;
    assert_eq!(row.action, AuditAction::Update);
    assert_eq!(row.entity, AuditEntity::LogEntry);
    assert_eq!(row.at_micros, fx.clock.now().timestamp_micros());
    let before = row.before.unwrap();
    let after = row.after.unwrap();
    assert_eq!(
        before["report"]["rooms"],
        json!([{ "room": r0.0, "level": 6 }])
    );
    assert_eq!(
        after["report"]["rooms"],
        json!([{ "room": r0.0, "level": 7 }, { "room": r2.0, "level": 7 }])
    );
    assert_eq!(before["note"], json!(null));
    assert_eq!(after["note"], json!("corrected"));
    assert_eq!(after["reading"], before["reading"]);
}

pub async fn correcting_a_reading_entry_changes_its_time_and_note<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let entry = add_entry(fx, &scene, start(), None, None, Some(pm25(12.5))).await;
    let earlier = start() - TimeDelta::minutes(3);

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let corrected = tx
        .correct_entry(
            entry.id,
            earlier,
            Note::parse(Some("sensor moved")).unwrap(),
            None,
        )
        .await
        .unwrap();
    let refused = tx
        .correct_entry(
            entry.id,
            earlier,
            None,
            Some(report_values(3, &[FIRST_ROOM], &[])),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(refused, StoreError::Conflict(Conflict::RuleViolated)),
        "a reading-only entry has no report to correct: {refused:?}"
    );
    drop(tx);

    assert_eq!(corrected.observed_at, earlier);
    assert_eq!(corrected.reading, entry.reading);
    assert_eq!(corrected.report, None);
    assert_eq!(corrected.note, Note::parse(Some("sensor moved")).unwrap());
}

pub async fn moving_an_entry_moves_both_children<S: Store>(fx: &Fixture<S>) {
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
    let entry = add_entry(
        fx,
        &scene,
        start(),
        None,
        Some(report_values(4, &[FIRST_ROOM], &[])),
        Some(pm25(9.0)),
    )
    .await;

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let moved = tx.move_entry(entry.id, other.id).await.unwrap();
    assert_eq!(moved.event_id, Some(other.id));
    let again = tx.move_entry(entry.id, other.id).await.unwrap();
    assert_eq!(again, moved);
    tx.commit().await.unwrap();

    assert!(
        fx.store
            .entries_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    let listed = fx.store.entries_for_event(other.id).await.unwrap();
    assert_eq!(listed, std::slice::from_ref(&moved));
    assert_eq!(moved.report_view().unwrap().event_id, Some(other.id));
    assert_eq!(moved.reading_view().unwrap().event_id, Some(other.id));
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::LogEntry, entry.id.0)
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(
        actions,
        [AuditAction::Create, AuditAction::Move],
        "moving twice to the same event records once"
    );
}

pub async fn deleting_an_entry_is_soft<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let entry = add_report(fx, &scene, start(), report_values(5, &[FIRST_ROOM], &[])).await;
    fx.clock.advance(TimeDelta::minutes(1));

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let deleted = tx.delete_entry(entry.id).await.unwrap();
    assert_eq!(deleted.deleted_at, Some(fx.clock.now()));
    for error in [
        tx.delete_entry(entry.id).await.unwrap_err(),
        tx.correct_entry(
            entry.id,
            start(),
            None,
            Some(report_values(6, &[FIRST_ROOM], &[])),
        )
        .await
        .unwrap_err(),
        tx.move_entry(entry.id, scene.event.id).await.unwrap_err(),
    ] {
        assert!(
            matches!(error, StoreError::Conflict(Conflict::AlreadyDeleted)),
            "{error:?}"
        );
    }
    tx.commit().await.unwrap();

    assert_eq!(fx.store.entry(entry.id).await.unwrap(), Some(deleted));
    assert!(
        fx.store
            .entries_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(last_entry(fx).await.record.action, AuditAction::Delete);
}

pub async fn delete_entry_hides_both_children<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let entry = add_entry(
        fx,
        &scene,
        start() - TimeDelta::minutes(1),
        None,
        Some(report_values(5, &[FIRST_ROOM], &[])),
        Some(pm25(30.0)),
    )
    .await;
    let kept = add_entry(
        fx,
        &scene,
        start() - TimeDelta::minutes(2),
        None,
        None,
        Some(pm25(8.0)),
    )
    .await;
    fx.clock.advance(TimeDelta::minutes(1));

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    tx.delete_entry(entry.id).await.unwrap();
    tx.commit().await.unwrap();

    let window = (start() - TimeDelta::hours(1), start() + TimeDelta::hours(1));
    assert_eq!(
        fx.store.entries_between(window.0, window.1).await.unwrap(),
        std::slice::from_ref(&kept)
    );
    assert_eq!(
        fx.store.entries_for_event(scene.event.id).await.unwrap(),
        [kept]
    );
    let stored = fx.store.entry(entry.id).await.unwrap().unwrap();
    assert!(stored.is_deleted());
    assert_eq!(
        (stored.report.is_some(), stored.reading.is_some()),
        (true, true),
        "the children stay with their deleted entry"
    );
    assert_no_childless_entries(fx).await;
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
}

pub async fn latest_report_time_ignores_deleted_reports_and_readings<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let earlier = start() - TimeDelta::minutes(30);
    let later = start() - TimeDelta::minutes(10);
    add_report(fx, &scene, earlier, report_values(3, &[FIRST_ROOM], &[])).await;
    let last = add_report(fx, &scene, later, report_values(5, &[FIRST_ROOM], &[])).await;
    add_entry(fx, &scene, start(), None, None, Some(pm25(40.0))).await;

    let mut tx = fx.store.begin(None).await.unwrap();
    assert_eq!(
        tx.latest_report_at(scene.event.id).await.unwrap(),
        Some(later),
        "a reading never counts"
    );
    tx.delete_entry(last.id).await.unwrap();
    assert_eq!(
        tx.latest_report_at(scene.event.id).await.unwrap(),
        Some(earlier)
    );
    let empty = tx
        .create_event(NewEvent {
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

pub async fn an_unknown_room_rolls_back_the_entry<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let new = NewEntry::new(
        scene.user.id,
        Some(scene.event.id),
        start(),
        None,
        Some(report_values(5, &[RoomId(999)], &[])),
        Some(pm25(10.0)),
    )
    .unwrap();
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx.create_entry(new).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
    drop(tx);

    assert!(
        fx.store
            .entries_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
    assert_no_childless_entries(fx).await;
}

pub async fn correcting_a_report_can_remove_rooms_and_symptoms<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let entry = add_report(
        fx,
        &scene,
        start(),
        report_values(6, &[r0, r2], &[scene.symptoms[0].id]),
    )
    .await;

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    tx.correct_entry(
        entry.id,
        entry.observed_at,
        None,
        Some(report_values(6, &[r2], &[])),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let stored = fx.store.entry(entry.id).await.unwrap().unwrap();
    assert_eq!(stored.values().room_ids(), [r2]);
    assert!(stored.values().symptoms().is_empty());
    let after = last_entry(fx).await.record.after.unwrap();
    assert_eq!(
        after["report"]["rooms"],
        json!([{ "room": r2.0, "level": 6 }])
    );
    assert_eq!(after["report"]["symptoms"], json!([]));
}

pub async fn an_unknown_room_on_correction_keeps_the_original_links<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let entry = add_report(fx, &scene, start(), report_values(6, &[r0, r2], &[])).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx
        .correct_entry(
            entry.id,
            entry.observed_at,
            None,
            Some(report_values(8, &[r0, RoomId(999)], &[])),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
    drop(tx);

    assert_eq!(fx.store.entry(entry.id).await.unwrap(), Some(entry));
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
}

pub async fn moving_an_entry_to_an_unknown_event_is_rejected<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let entry = add_report(fx, &scene, start(), report_values(4, &[FIRST_ROOM], &[])).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx.move_entry(entry.id, EventId(999)).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
    drop(tx);

    assert_eq!(fx.store.entry(entry.id).await.unwrap(), Some(entry));
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
}

pub async fn an_unknown_symptom_rolls_back_the_entry<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let new = NewEntry::new(
        scene.user.id,
        Some(scene.event.id),
        start(),
        None,
        Some(report_values(5, &[FIRST_ROOM], &[SymptomId(999)])),
        None,
    )
    .unwrap();
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx.create_entry(new).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
    drop(tx);

    assert!(
        fx.store
            .entries_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
    assert_no_childless_entries(fx).await;
}

pub async fn entries_between_returns_live_entries_with_their_children<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let (r0, r1, s0) = (scene.rooms[0].id, scene.rooms[1].id, scene.symptoms[0].id);
    let at = |minutes: i64| start() - TimeDelta::minutes(minutes);
    let before = add_report(fx, &scene, at(50), report_values(2, &[r0], &[])).await;
    let first = add_report(fx, &scene, at(40), report_values(5, &[r1, r0], &[s0])).await;
    let reading = add_entry(fx, &scene, at(35), Some("alone"), None, Some(pm25(10.0))).await;
    let second = add_entry(
        fx,
        &scene,
        at(30),
        None,
        Some(report_values(6, &[FIRST_ROOM], &[])),
        Some(pm25(22.0)),
    )
    .await;
    let deleted = add_report(fx, &scene, at(20), report_values(7, &[r0], &[s0])).await;
    let at_end = add_report(fx, &scene, at(10), report_values(8, &[r1], &[])).await;
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.delete_entry(deleted.id).await.unwrap();
    tx.commit().await.unwrap();

    let found = fx.store.entries_between(at(40), at(10)).await.unwrap();
    assert_eq!(found, [first.clone(), reading, second]);
    assert_eq!(found[0].values().room_ids(), [r0, r1]);
    assert_eq!(found[0].values().symptoms(), [s0]);
    assert!(!found.contains(&before) && !found.contains(&at_end));
    assert!(
        fx.store
            .entries_between(at(5), start())
            .await
            .unwrap()
            .is_empty()
    );
}

pub async fn a_reading_can_stand_alone_or_join_an_event<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let new = |event_id: Option<EventId>, observed_at, value| {
        NewEntry::new(
            scene.user.id,
            event_id,
            observed_at,
            None,
            None,
            Some(pm25(value)),
        )
        .unwrap()
    };
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let alone = tx
        .create_entry(new(None, start() - TimeDelta::days(1), 8.0))
        .await
        .unwrap();
    let linked = tx
        .create_entry(new(Some(scene.event.id), start(), 35.5))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(alone.event_id, None);
    assert_eq!(alone.report, None);
    assert_eq!(linked.recorded_at, start());
    assert_eq!(fx.store.entry(alone.id).await.unwrap(), Some(alone.clone()));
    assert_eq!(
        fx.store.entries_for_event(scene.event.id).await.unwrap(),
        [linked]
    );
    assert_no_childless_entries(fx).await;
}

pub async fn correcting_and_deleting_a_reading_entry_are_audited<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let new = NewEntry::new(
        scene.user.id,
        Some(scene.event.id),
        start(),
        None,
        None,
        Some(pm25(12.5)),
    )
    .unwrap();
    let entry = tx.create_entry(new).await.unwrap();
    let earlier = start() - TimeDelta::minutes(1);
    let corrected = tx
        .correct_entry(
            entry.id,
            earlier,
            Note::parse(Some("recheck")).unwrap(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(corrected.observed_at, earlier);
    tx.commit().await.unwrap();
    fx.clock.advance(TimeDelta::minutes(1));

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let deleted = tx.delete_entry(entry.id).await.unwrap();
    assert_eq!(deleted.deleted_at, Some(fx.clock.now()));
    for error in [
        tx.correct_entry(entry.id, start(), None, None)
            .await
            .unwrap_err(),
        tx.delete_entry(entry.id).await.unwrap_err(),
    ] {
        assert!(
            matches!(error, StoreError::Conflict(Conflict::AlreadyDeleted)),
            "{error:?}"
        );
    }
    tx.commit().await.unwrap();

    assert!(
        fx.store
            .entries_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.entry(entry.id).await.unwrap(), Some(deleted));
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::LogEntry, entry.id.0)
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

pub async fn a_reading_for_an_unknown_event_is_rejected<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let new = NewEntry::new(
        scene.user.id,
        Some(EventId(999)),
        start(),
        None,
        None,
        Some(pm25(10.0)),
    )
    .unwrap();
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx.create_entry(new).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
    drop(tx);

    assert!(
        fx.store
            .entries_for_event(scene.event.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
    assert_no_childless_entries(fx).await;
}

pub async fn pm25_values_round_trip_exactly<S: Store>(fx: &Fixture<S>) {
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
        let entry = add_entry(fx, &scene, start(), None, None, Some(pm25(value))).await;
        let stored = fx.store.entry(entry.id).await.unwrap().unwrap();
        assert_eq!(stored.reading.unwrap().value, pm25(value));
        assert_eq!(
            last_entry(fx).await.record.after.unwrap()["reading"]["value"],
            json!(value)
        );
    }
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
}

pub async fn one_write_transaction_uses_one_instant<S: Store>(fx: &Fixture<S>) {
    let scene = scene(fx).await;
    let audit_before = fx.store.audit_log().await.unwrap().len();
    fx.clock.tick_on_read(TimeDelta::milliseconds(1));

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let event = tx
        .create_event(NewEvent {
            owner_id: scene.user.id,
            opened_at: start() - TimeDelta::days(1),
            closed: Some(Closure {
                at: start() - TimeDelta::days(1),
                reason: CloseReason::Backdated,
            }),
        })
        .await
        .unwrap();
    let new = NewEntry::new(
        scene.user.id,
        Some(event.id),
        start() - TimeDelta::days(1),
        None,
        Some(report_values(5, &[FIRST_ROOM], &[])),
        Some(pm25(15.0)),
    )
    .unwrap();
    let entry = tx.create_entry(new).await.unwrap();
    let deleted = tx.delete_entry(entry.id).await.unwrap();
    tx.commit().await.unwrap();

    let new_rows: Vec<_> = fx
        .store
        .audit_log()
        .await
        .unwrap()
        .into_iter()
        .skip(audit_before)
        .collect();
    assert_eq!(new_rows.len(), 3, "event, entry, delete");
    let instant = entry.recorded_at;
    assert_eq!(event.recorded_at, instant);
    assert_eq!(deleted.deleted_at, Some(instant));
    assert!(
        new_rows
            .iter()
            .all(|row| row.record.at_micros == instant.timestamp_micros()),
        "every row and audit entry of one transaction shares its instant"
    );
}
