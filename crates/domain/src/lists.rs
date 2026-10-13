//! Editing the room and symptom lists (spec §4: residents only). Items are hidden, never
//! deleted, so the reports that name them keep their names; custom items have no translation
//! key and are shown as typed.

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::permissions::{Action, Actor, can};
use crate::records::{NewRoom, NewSymptom, Room, Symptom};
use crate::repos::{ListWrites, StoreError};
use crate::types::{ListName, RoomArea, RoomId, SymptomId};

#[derive(Debug, Error)]
pub enum ListError {
    #[error("not allowed")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Store(StoreError),
}

impl From<StoreError> for ListError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::NotFound { .. } => Self::NotFound,
            other => Self::Store(other),
        }
    }
}

fn require_editor(actor: &Actor, now: DateTime<Utc>) -> Result<(), ListError> {
    if can(actor, Action::EditLists, now) {
        Ok(())
    } else {
        Err(ListError::Forbidden)
    }
}

pub async fn add_room<T: ListWrites>(
    tx: &mut T,
    actor: &Actor,
    name: ListName,
    area: RoomArea,
    now: DateTime<Utc>,
) -> Result<Room, ListError> {
    require_editor(actor, now)?;
    let new = NewRoom {
        name,
        i18n_key: None,
        area,
    };
    Ok(tx.create_room(new).await?)
}

/// Hiding stops offering the room on the forms; showing offers it again.
pub async fn set_room_hidden<T: ListWrites>(
    tx: &mut T,
    actor: &Actor,
    id: RoomId,
    hidden: bool,
    now: DateTime<Utc>,
) -> Result<Room, ListError> {
    require_editor(actor, now)?;
    Ok(tx.set_room_hidden(id, hidden).await?)
}

pub async fn add_symptom<T: ListWrites>(
    tx: &mut T,
    actor: &Actor,
    name: ListName,
    now: DateTime<Utc>,
) -> Result<Symptom, ListError> {
    require_editor(actor, now)?;
    let new = NewSymptom {
        name,
        i18n_key: None,
    };
    Ok(tx.create_symptom(new).await?)
}

/// Hiding stops offering the symptom on the forms; showing offers it again.
pub async fn set_symptom_hidden<T: ListWrites>(
    tx: &mut T,
    actor: &Actor,
    id: SymptomId,
    hidden: bool,
    now: DateTime<Utc>,
) -> Result<Symptom, ListError> {
    require_editor(actor, now)?;
    Ok(tx.set_symptom_hidden(id, hidden).await?)
}
