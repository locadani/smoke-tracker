//! Rooms and symptoms: the lists a report picks from. Hidden, never deleted.

use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{NewRoom, NewSymptom, Room, Symptom};
use smoke_domain::repos::{ListRepo, ListWrites, StoreError, StoreResult};
use smoke_domain::types::{ListName, RoomId, SymptomId};
use sqlx::PgConnection;

use crate::convert::{corrupt, micros, opt_instant, parse};
use crate::error::IntoStore;
use crate::{PostgresStore, PostgresTx};

struct RoomRow {
    id: i64,
    name: String,
    i18n_key: Option<String>,
    area: String,
    hidden_at: Option<i64>,
}

impl RoomRow {
    fn into_room(self) -> StoreResult<Room> {
        Ok(Room {
            id: RoomId(self.id),
            name: ListName::new(&self.name).map_err(corrupt)?,
            i18n_key: self.i18n_key,
            area: parse(&self.area)?,
            hidden_at: opt_instant(self.hidden_at)?,
        })
    }
}

struct SymptomRow {
    id: i64,
    name: String,
    i18n_key: Option<String>,
    hidden_at: Option<i64>,
}

impl SymptomRow {
    fn into_symptom(self) -> StoreResult<Symptom> {
        Ok(Symptom {
            id: SymptomId(self.id),
            name: ListName::new(&self.name).map_err(corrupt)?,
            i18n_key: self.i18n_key,
            hidden_at: opt_instant(self.hidden_at)?,
        })
    }
}

async fn require_room(conn: &mut PgConnection, id: RoomId) -> StoreResult<Room> {
    sqlx::query_as!(
        RoomRow,
        "SELECT id, name, i18n_key, area, hidden_at FROM rooms WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .ok_or(StoreError::NotFound {
        entity: AuditEntity::Room,
        id: id.0,
    })?
    .into_room()
}

async fn require_symptom(conn: &mut PgConnection, id: SymptomId) -> StoreResult<Symptom> {
    sqlx::query_as!(
        SymptomRow,
        "SELECT id, name, i18n_key, hidden_at FROM symptoms WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .ok_or(StoreError::NotFound {
        entity: AuditEntity::Symptom,
        id: id.0,
    })?
    .into_symptom()
}

pub(crate) async fn fetch_rooms(conn: &mut PgConnection) -> StoreResult<Vec<Room>> {
    sqlx::query_as!(
        RoomRow,
        "SELECT id, name, i18n_key, area, hidden_at FROM rooms ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(RoomRow::into_room)
    .collect()
}

pub(crate) async fn fetch_symptoms(conn: &mut PgConnection) -> StoreResult<Vec<Symptom>> {
    sqlx::query_as!(
        SymptomRow,
        "SELECT id, name, i18n_key, hidden_at FROM symptoms ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(SymptomRow::into_symptom)
    .collect()
}

impl ListRepo for PostgresStore {
    async fn rooms(&self) -> StoreResult<Vec<Room>> {
        fetch_rooms(&mut *self.read().await?).await
    }

    async fn symptoms(&self) -> StoreResult<Vec<Symptom>> {
        fetch_symptoms(&mut *self.read().await?).await
    }
}

impl ListWrites for PostgresTx {
    async fn create_room(&mut self, new: NewRoom) -> StoreResult<Room> {
        let id = sqlx::query_scalar!(
            "INSERT INTO rooms (name, i18n_key, area) VALUES ($1, $2, $3) RETURNING id",
            new.name.as_str(),
            new.i18n_key,
            new.area.as_str(),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let room = require_room(self.conn(), RoomId(id)).await?;
        self.record(AuditAction::Create, None, &room).await?;
        Ok(room)
    }

    async fn set_room_hidden(&mut self, id: RoomId, hidden: bool) -> StoreResult<Room> {
        let before = require_room(self.conn(), id).await?;
        let hidden_at = hidden.then(|| before.hidden_at.unwrap_or_else(|| self.now()));
        sqlx::query!(
            "UPDATE rooms SET hidden_at = $1 WHERE id = $2",
            hidden_at.map(micros),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        let after = require_room(self.conn(), id).await?;
        self.record(AuditAction::Update, Some(&before), &after)
            .await?;
        Ok(after)
    }

    async fn create_symptom(&mut self, new: NewSymptom) -> StoreResult<Symptom> {
        let id = sqlx::query_scalar!(
            "INSERT INTO symptoms (name, i18n_key) VALUES ($1, $2) RETURNING id",
            new.name.as_str(),
            new.i18n_key,
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let symptom = require_symptom(self.conn(), SymptomId(id)).await?;
        self.record(AuditAction::Create, None, &symptom).await?;
        Ok(symptom)
    }

    async fn set_symptom_hidden(&mut self, id: SymptomId, hidden: bool) -> StoreResult<Symptom> {
        let before = require_symptom(self.conn(), id).await?;
        let hidden_at = hidden.then(|| before.hidden_at.unwrap_or_else(|| self.now()));
        sqlx::query!(
            "UPDATE symptoms SET hidden_at = $1 WHERE id = $2",
            hidden_at.map(micros),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        let after = require_symptom(self.conn(), id).await?;
        self.record(AuditAction::Update, Some(&before), &after)
            .await?;
        Ok(after)
    }
}
