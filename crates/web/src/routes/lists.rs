//! Editing the room and symptom lists from Settings (spec §9 screen 7, residents only). Items
//! are added or hidden, never renamed or deleted, so old reports keep reading the same.

use axum::Form;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use smoke_domain::lists::{ListError, add_room, add_symptom, set_room_hidden, set_symptom_hidden};
use smoke_domain::records::User;
use smoke_domain::repos::{Commit, Store};
use smoke_domain::types::{ListName, RoomArea, RoomId, SymptomId};

use crate::auth::CurrentUser;
use crate::error::AppError;
use crate::i18n::Tr;
use crate::labels;
use crate::routes::settings::{Notices, settings_page};
use crate::state::AppState;

/// A room or symptom as Settings lists it.
pub(crate) struct ListItem {
    pub(crate) label: String,
    /// Where its Hide / Show button posts.
    pub(crate) action: String,
    /// A room's area; empty for symptoms.
    pub(crate) detail: String,
    pub(crate) hidden: bool,
    /// "Hidden" for screen readers beside a struck-through label; empty while shown.
    pub(crate) gone: String,
    /// The Hide or Show button's text.
    pub(crate) toggle: String,
}

impl ListItem {
    fn new(tr: Tr, action: String, label: String, detail: String, hidden: bool) -> Self {
        let (gone, toggle) = if hidden {
            (tr.t("settings-list-hidden"), tr.t("settings-list-show"))
        } else {
            (String::new(), tr.t("settings-list-hide"))
        };
        Self {
            label,
            action,
            detail,
            hidden,
            gone,
            toggle,
        }
    }
}

pub(crate) struct ListsView {
    pub(crate) rooms: Vec<ListItem>,
    pub(crate) symptoms: Vec<ListItem>,
}

fn area_key(area: RoomArea) -> &'static str {
    match area {
        RoomArea::Inside => "log-inside",
        RoomArea::Common => "log-common",
    }
}

/// Every room and symptom, hidden ones included, in the order the forms show them.
pub(crate) async fn view<S: Store>(state: &AppState<S>, tr: Tr) -> Result<ListsView, AppError> {
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    Ok(ListsView {
        rooms: rooms
            .iter()
            .map(|room| {
                ListItem::new(
                    tr,
                    format!("/settings/rooms/{}/hidden", room.id),
                    labels::room(tr, room),
                    tr.t(area_key(room.area)),
                    room.hidden_at.is_some(),
                )
            })
            .collect(),
        symptoms: symptoms
            .iter()
            .map(|symptom| {
                ListItem::new(
                    tr,
                    format!("/settings/symptoms/{}/hidden", symptom.id),
                    labels::symptom(tr, symptom),
                    String::new(),
                    symptom.hidden_at.is_some(),
                )
            })
            .collect(),
    })
}

fn refusal(error: ListError) -> AppError {
    match error {
        ListError::Forbidden => AppError::NotAllowed,
        ListError::NotFound => AppError::NotFound,
        ListError::Store(error) => error.into(),
    }
}

fn saved() -> Response {
    Redirect::to("/settings?saved=lists").into_response()
}

async fn refuse<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    key: &'static str,
) -> Result<Response, AppError> {
    let notices = Notices {
        list_error: Some(key),
        ..Notices::default()
    };
    settings_page(state, user, tr, StatusCode::UNPROCESSABLE_ENTITY, notices).await
}

#[derive(Deserialize)]
pub(crate) struct NewItemForm {
    name: String,
    /// Rooms only: `inside` or `common`.
    area: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct HiddenForm {
    hidden: bool,
}

pub(crate) async fn create_room<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Form(form): Form<NewItemForm>,
) -> Result<Response, AppError> {
    let Ok(name) = ListName::new(&form.name) else {
        return refuse(&state, &user, tr, "settings-list-name-invalid").await;
    };
    let Some(area) = form.area.and_then(|area| area.parse().ok()) else {
        return refuse(&state, &user, tr, "log-error-form").await;
    };
    let now = state.clock.now();
    let mut tx = state.store.begin(Some(user.id)).await?;
    add_room(&mut tx, &user.actor(), name, area, now)
        .await
        .map_err(refusal)?;
    tx.commit().await?;
    Ok(saved())
}

pub(crate) async fn hide_room<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Form(form): Form<HiddenForm>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let mut tx = state.store.begin(Some(user.id)).await?;
    set_room_hidden(&mut tx, &user.actor(), RoomId(id), form.hidden, now)
        .await
        .map_err(refusal)?;
    tx.commit().await?;
    Ok(saved())
}

pub(crate) async fn create_symptom<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Form(form): Form<NewItemForm>,
) -> Result<Response, AppError> {
    let Ok(name) = ListName::new(&form.name) else {
        return refuse(&state, &user, tr, "settings-list-name-invalid").await;
    };
    let now = state.clock.now();
    let mut tx = state.store.begin(Some(user.id)).await?;
    add_symptom(&mut tx, &user.actor(), name, now)
        .await
        .map_err(refusal)?;
    tx.commit().await?;
    Ok(saved())
}

pub(crate) async fn hide_symptom<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Form(form): Form<HiddenForm>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let mut tx = state.store.begin(Some(user.id)).await?;
    set_symptom_hidden(&mut tx, &user.actor(), SymptomId(id), form.hidden, now)
        .await
        .map_err(refusal)?;
    tx.commit().await?;
    Ok(saved())
}

/// Every key this module passes through a variable, for the translation tests.
#[cfg(test)]
pub(crate) const KEYS: [&str; 4] = [
    "settings-list-name-invalid",
    "log-error-form",
    "log-inside",
    "log-common",
];
