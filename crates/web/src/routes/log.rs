//! The Log screen (spec §9, screen 1): the event status, the month's summary card and the form.

use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::{Room, Symptom, User};
use smoke_domain::repos::Store;
use smoke_domain::starter::BEDROOM_KEY;
use smoke_domain::summary::{MonthSummary, summarise};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{Intensity, RoomArea, RoomId, SymptomId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::i18n::Tr;
use crate::routes::guide::{Level, levels};
use crate::routes::{can_open, notices};
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

impl Draft {
    /// The form as posted, to show it again after a validation error.
    pub(crate) fn from_form(form: &Pairs) -> Self {
        let text = |name| form.get(name).unwrap_or_default().to_owned();
        Self {
            intensity: form.get("intensity").and_then(|v| v.parse().ok()),
            rooms: form
                .ids("room")
                .unwrap_or_default()
                .into_iter()
                .map(RoomId)
                .collect(),
            symptoms: form
                .ids("symptom")
                .unwrap_or_default()
                .into_iter()
                .map(SymptomId)
                .collect(),
            pm25: text("pm25"),
            observed_at: text("observed_at"),
            note: text("note"),
        }
    }
}

/// A radio button, chip or checkbox.
pub(crate) struct Choice {
    pub(crate) value: i64,
    pub(crate) label: String,
    pub(crate) checked: bool,
}

/// The intensity, room and symptom fields shared by the Log form and the correction form
/// (`templates/report_fields.html`).
pub(crate) struct Fields {
    pub(crate) intensities: Vec<Choice>,
    pub(crate) inside: Vec<Choice>,
    pub(crate) common: Vec<Choice>,
    pub(crate) symptoms: Vec<Choice>,
}

/// The choices for `draft`. Hidden rooms and symptoms are offered only if listed in `keep`
/// (a correction keeps those its report already names).
pub(crate) fn fields(
    tr: Tr,
    rooms: &[Room],
    symptoms: &[Symptom],
    draft: &Draft,
    keep: (&[RoomId], &[SymptomId]),
) -> Fields {
    let room_choices = |area: RoomArea| {
        rooms
            .iter()
            .filter(|r| r.area == area && (r.hidden_at.is_none() || keep.0.contains(&r.id)))
            .map(|r| Choice {
                value: r.id.0,
                label: labels::room(tr, r),
                checked: draft.rooms.contains(&r.id),
            })
            .collect()
    };
    Fields {
        intensities: (Intensity::MIN..=Intensity::MAX)
            .map(|value| Choice {
                value: value.into(),
                label: value.to_string(),
                checked: draft.intensity == Some(value),
            })
            .collect(),
        inside: room_choices(RoomArea::Inside),
        common: room_choices(RoomArea::Common),
        symptoms: symptoms
            .iter()
            .filter(|s| s.hidden_at.is_none() || keep.1.contains(&s.id))
            .map(|s| Choice {
                value: s.id.0,
                label: labels::symptom(tr, s),
                checked: draft.symptoms.contains(&s.id),
            })
            .collect(),
    }
}

/// What the Log screen says about the open event.
pub(crate) enum Status {
    /// No event is open.
    None,
    /// The viewer may see the open event, which started at this time.
    Open(String),
    /// An event is open but the viewer may not know about it, so the screen says nothing.
    Hidden,
}

pub(crate) struct FormView {
    fields: Fields,
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
    status: Status,
    /// The open event, when the user may close it.
    close_event: Option<i64>,
    /// "The event from … was auto-closed", shown while another event is open (spec §7.4).
    notice: Option<String>,
    /// A translation key confirming what was just saved.
    saved: Option<&'static str>,
    summary: MonthSummary,
    levels: Vec<Level>,
    /// `None` for people who may not log (viewers, expired guests).
    form: Option<FormView>,
}

#[derive(Deserialize)]
pub(crate) struct ShowQuery {
    saved: Option<String>,
}

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Query(query): Query<ShowQuery>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    auto_close(&state.store, &state.config.schedule, now).await?;
    let saved = match query.saved.as_deref() {
        Some("report") => Some("log-saved-report"),
        Some("pm25") => Some("log-saved-pm25"),
        _ => None,
    };
    let log_page = log_page(&state, &user, tr, now, None, None, saved).await?;
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
    saved: Option<&'static str>,
) -> Result<LogPage, AppError> {
    let schedule = &state.config.schedule;
    let actor = user.actor();
    let open = state.store.open_event().await?;
    let open_reports = match &open {
        Some(event) => state.store.reports_for_event(event.id).await?,
        None => Vec::new(),
    };
    let shown = open
        .as_ref()
        .filter(|e| can_open(&actor, e, &open_reports, now));
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
    let notice = match &open {
        Some(_) => notices::pending(state, user, now)
            .await?
            .pop()
            .map(|closed| {
                let when = time::date_time(tr.0, closed.opened_at, schedule.tz());
                tr.t1("prompt-auto-closed", "when", &when)
            }),
        None => None,
    };

    let form = if can(&actor, Action::LogEntry, now) {
        let draft = match draft {
            Some(draft) => draft,
            None => {
                let mut blank = Draft::default();
                if let Some(last) = open_reports.iter().rev().find(|r| r.user_id == user.id) {
                    blank.rooms = last.values.rooms().to_vec();
                }
                blank
            }
        };
        let symptoms = state.store.symptoms().await?;
        Some(FormView {
            fields: fields(tr, &rooms, &symptoms, &draft, (&[], &[])),
            pm25: draft.pm25,
            observed_at: draft.observed_at,
            note: draft.note,
            update: shown.is_some(),
            error,
        })
    } else {
        None
    };

    Ok(LogPage {
        tr,
        signed_in: Some(user.username.to_string()),
        close_event: shown
            .filter(|e| can(&actor, Action::ManageEvent { owner: e.owner_id }, now))
            .map(|e| e.id.0),
        status: match (&open, shown) {
            (None, _) => Status::None,
            (Some(_), Some(e)) => Status::Open(time::since(tr.0, e.opened_at, now, schedule.tz())),
            (Some(_), None) => Status::Hidden,
        },
        notice,
        saved,
        summary,
        levels: levels(tr),
        form,
    })
}
