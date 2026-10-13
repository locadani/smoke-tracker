//! The Log screen (spec §9, screen 1): the event status, the month's summary card and the form.

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, Utc};
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::User;
use smoke_domain::repos::Store;
use smoke_domain::starter::BEDROOM_KEY;
use smoke_domain::summary::{MonthSummary, summarise};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{Intensity, RoomArea, RoomId, SymptomId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::i18n::Tr;
use crate::routes::guide::{Level, levels};
use crate::state::AppState;
use crate::{labels, time};

/// What the form holds: blank, pre-filled, or as typed before a validation error.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Draft {
    pub(crate) intensity: Option<u8>,
    pub(crate) rooms: Vec<RoomId>,
    pub(crate) symptoms: Vec<SymptomId>,
    pub(crate) pm25: String,
    pub(crate) observed_at: String,
    pub(crate) note: String,
}

/// A radio button, chip or checkbox.
pub(crate) struct Choice {
    value: i64,
    label: String,
    checked: bool,
}

pub(crate) struct FormView {
    intensities: Vec<Choice>,
    inside: Vec<Choice>,
    common: Vec<Choice>,
    symptoms: Vec<Choice>,
    pm25: String,
    observed_at: String,
    note: String,
    /// An event is open, so the main button reads "Update: situation changed".
    update: bool,
    /// A translation key explaining what to fix.
    error: Option<&'static str>,
}

#[derive(Template)]
#[template(path = "log.html")]
pub(crate) struct LogPage {
    tr: Tr,
    signed_in: Option<String>,
    open_since: Option<String>,
    summary: MonthSummary,
    levels: Vec<Level>,
    /// `None` for people who may not log (viewers, expired guests).
    form: Option<FormView>,
}

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    auto_close(&state.store, &state.config.schedule, now).await?;
    let log_page = log_page(&state, &user, tr, now, None, None).await?;
    page(StatusCode::OK, &log_page)
}

/// Builds the Log screen. Without a `draft`, the rooms of the user's last report in the open
/// event are pre-selected (spec §9).
pub(crate) async fn log_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    draft: Option<Draft>,
    error: Option<&'static str>,
) -> Result<LogPage, AppError> {
    let schedule = &state.config.schedule;
    let actor = user.actor();
    let open = state.store.open_event().await?;
    let rooms = state.store.rooms().await?;

    let (month_start, month_end) = schedule.month_span(schedule.local_date(now));
    let visible: Vec<_> = state
        .store
        .reports_between(month_start, month_end)
        .await?
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let bedroom = rooms
        .iter()
        .find(|r| r.i18n_key.as_deref() == Some(BEDROOM_KEY))
        .map(|r| r.id);
    let summary = summarise(&visible, bedroom, schedule);

    let form = if can(&actor, Action::LogEntry, now) {
        let draft = match draft {
            Some(draft) => draft,
            None => {
                let mut blank = Draft::default();
                if let Some(open) = &open {
                    let reports = state.store.reports_for_event(open.id).await?;
                    if let Some(last) = reports.iter().rev().find(|r| r.user_id == user.id) {
                        blank.rooms = last.values.rooms().to_vec();
                    }
                }
                blank
            }
        };
        let room_choices = |area: RoomArea| {
            rooms
                .iter()
                .filter(|r| r.area == area && r.hidden_at.is_none())
                .map(|r| Choice {
                    value: r.id.0,
                    label: labels::room(tr, r),
                    checked: draft.rooms.contains(&r.id),
                })
                .collect()
        };
        let symptoms = state
            .store
            .symptoms()
            .await?
            .iter()
            .filter(|s| s.hidden_at.is_none())
            .map(|s| Choice {
                value: s.id.0,
                label: labels::symptom(tr, s),
                checked: draft.symptoms.contains(&s.id),
            })
            .collect();
        Some(FormView {
            intensities: (Intensity::MIN..=Intensity::MAX)
                .map(|value| Choice {
                    value: value.into(),
                    label: value.to_string(),
                    checked: draft.intensity == Some(value),
                })
                .collect(),
            inside: room_choices(RoomArea::Inside),
            common: room_choices(RoomArea::Common),
            symptoms,
            pm25: draft.pm25,
            observed_at: draft.observed_at,
            note: draft.note,
            update: open.is_some(),
            error,
        })
    } else {
        None
    };

    Ok(LogPage {
        tr,
        signed_in: Some(user.username.to_string()),
        open_since: open.map(|e| time::since(tr.0, e.opened_at, now, schedule.tz())),
        summary,
        levels: levels(tr),
        form,
    })
}
