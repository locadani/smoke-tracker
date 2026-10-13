//! Rooms and symptoms.

use chrono::TimeDelta;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{NewRoom, NewSymptom};
use smoke_domain::repos::{AuditRepo, Commit, ListRepo, ListWrites, StoreError, Transactional};
use smoke_domain::starter::{STARTER_ROOMS, STARTER_SYMPTOMS, seed_starter_lists};
use smoke_domain::types::{ListName, RoomArea, SymptomId};

use crate::Fixture;
use crate::fixture::start;

pub async fn seeding_adds_the_starter_lists_once<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: ListWrites> + ListRepo + AuditRepo,
{
    seed_starter_lists(&fx.store).await.unwrap();
    seed_starter_lists(&fx.store).await.unwrap();

    let rooms = fx.store.rooms().await.unwrap();
    let keys: Vec<_> = rooms
        .iter()
        .map(|r| r.i18n_key.as_deref().unwrap())
        .collect();
    assert_eq!(keys, STARTER_ROOMS.map(|r| r.i18n_key));
    let stairwell = rooms.last().unwrap();
    assert_eq!(stairwell.area, RoomArea::Common);
    assert_eq!(stairwell.name.as_str(), "Stairwell (shared)");
    assert_eq!(
        fx.store.symptoms().await.unwrap().len(),
        STARTER_SYMPTOMS.len()
    );

    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(log.len(), 12);
    assert!(
        log.iter()
            .all(|e| e.record.actor_id.is_none() && e.record.action == AuditAction::Create)
    );
}

pub async fn a_custom_room_has_no_translation_key<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: ListWrites> + ListRepo,
{
    let mut tx = fx.store.begin(None).await.unwrap();
    let room = tx
        .create_room(NewRoom {
            name: ListName::new("Balcony").unwrap(),
            i18n_key: None,
            area: RoomArea::Common,
        })
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(room.i18n_key, None);
    assert_eq!(room.hidden_at, None);
    assert_eq!(fx.store.rooms().await.unwrap(), [room]);
}

pub async fn hiding_a_room_is_audited_once<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: ListWrites> + ListRepo + AuditRepo,
{
    seed_starter_lists(&fx.store).await.unwrap();
    let kitchen = fx.store.rooms().await.unwrap()[1].clone();
    assert_eq!(kitchen.i18n_key.as_deref(), Some("room-kitchen"));

    let mut tx = fx.store.begin(None).await.unwrap();
    let hidden = tx.set_room_hidden(kitchen.id, true).await.unwrap();
    assert_eq!(hidden.hidden_at, Some(start()));
    fx.clock.advance(TimeDelta::minutes(5));
    let still_hidden = tx.set_room_hidden(kitchen.id, true).await.unwrap();
    assert_eq!(still_hidden.hidden_at, Some(start()));
    let shown = tx.set_room_hidden(kitchen.id, false).await.unwrap();
    assert_eq!(shown.hidden_at, None);
    tx.commit().await.unwrap();

    assert_eq!(fx.store.rooms().await.unwrap().len(), 6);
    let updates = fx
        .store
        .audit_history(AuditEntity::Room, kitchen.id.0)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.record.action == AuditAction::Update)
        .count();
    assert_eq!(updates, 2);
}

pub async fn symptoms_can_be_created_and_hidden<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: ListWrites> + ListRepo + AuditRepo,
{
    let mut tx = fx.store.begin(None).await.unwrap();
    let symptom = tx
        .create_symptom(NewSymptom {
            name: ListName::new("Dizziness").unwrap(),
            i18n_key: None,
        })
        .await
        .unwrap();
    assert_eq!(
        (symptom.i18n_key.as_deref(), symptom.hidden_at),
        (None, None)
    );
    let hidden = tx.set_symptom_hidden(symptom.id, true).await.unwrap();
    assert_eq!(hidden.hidden_at, Some(start()));
    let shown = tx.set_symptom_hidden(symptom.id, false).await.unwrap();
    assert_eq!(shown.hidden_at, None);
    tx.commit().await.unwrap();

    assert_eq!(fx.store.symptoms().await.unwrap(), [shown]);
    let updates = fx
        .store
        .audit_history(AuditEntity::Symptom, symptom.id.0)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.record.action == AuditAction::Update)
        .count();
    assert_eq!(updates, 2);
}

pub async fn hiding_an_unknown_symptom_is_not_found<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: ListWrites>,
{
    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx
        .set_symptom_hidden(SymptomId(42), true)
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::NotFound {
                entity: AuditEntity::Symptom,
                id: 42
            }
        ),
        "{error:?}"
    );
}
