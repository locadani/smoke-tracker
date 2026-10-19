//! A log entry's page (spec 2026-10-19 §7): time, author, PM2.5 value, rooms, symptoms and the
//! note, then, for an entry with a smell report, the "edited" badge, History, "Correct a
//! mistake" and "Move to another event". `/reports/{id}` redirects here.

use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::history::{Change, entry_history, was_corrected};
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::{LogEntry, Symptom, User};
use smoke_domain::repos::Store;
use smoke_domain::types::{EventId, LogEntryId, ReportId, SymptomId, UserId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::header::{SignedIn, Tab};
use crate::i18n::Tr;
use crate::routes::entries::joinable_events;
use crate::state::AppState;
use crate::{labels, time};

/// The entry, if it exists, is not deleted and `user` may see it; otherwise "not found", so
/// nobody learns that someone else's entry exists.
async fn visible_entry<S: Store>(
    state: &AppState<S>,
    user: &User,
    id: LogEntryId,
) -> Result<LogEntry, AppError> {
    let now = state.clock.now();
    state
        .store
        .entry(id)
        .await?
        .filter(|entry| !entry.is_deleted() && can_see(&user.actor(), entry.user_id, now))
        .ok_or(AppError::NotFound)
}

fn symptom_names(tr: Tr, symptoms: &[Symptom], ids: &[SymptomId]) -> Vec<String> {
    symptoms
        .iter()
        .filter(|s| ids.contains(&s.id))
        .map(|s| labels::symptom(tr, s))
        .collect()
}

struct HistoryLine {
    at: String,
    by: Option<String>,
    action: &'static str,
    changes: Vec<String>,
}

struct EventLink {
    id: i64,
    date: String,
}

struct MoveOption {
    value: String,
    label: String,
}

/// What the smell report of an entry says.
struct SmellFacts {
    id: i64,
    intensity: String,
    rooms: String,
    symptoms: String,
}

#[derive(Template)]
#[template(path = "entry.html")]
struct EntryPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    /// `None` for a no-smell check outside any event, and for a reading outside every event.
    event: Option<EventLink>,
    observed: String,
    recorded: String,
    pm25: Option<String>,
    smell: Option<SmellFacts>,
    note: Option<String>,
    by: Option<String>,
    edited: bool,
    history: Vec<HistoryLine>,
    /// Shown to those who may correct and move the entry's report.
    editable: bool,
    move_options: Vec<MoveOption>,
    error: Option<&'static str>,
}

fn action_key(action: AuditAction) -> &'static str {
    match action {
        AuditAction::Create => "history-create",
        AuditAction::Move => "history-move",
        AuditAction::Delete => "history-delete",
        _ => "history-update",
    }
}

/// Every key [`action_key`] can return, for the translation tests.
#[cfg(test)]
pub(crate) const HISTORY_KEYS: [&str; 4] = [
    "history-create",
    "history-move",
    "history-delete",
    "history-update",
];

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let entry_page = entry_page(&state, &user, tr, LogEntryId(id), None).await?;
    page(StatusCode::OK, &entry_page)
}

/// `/reports/{id}`: the page of the entry that owns the report, with the same 404 for a report
/// that is missing, deleted or not the viewer's to see.
pub(crate) async fn from_report<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let entry = state
        .store
        .entry_of_report(ReportId(id))
        .await?
        .ok_or(AppError::NotFound)?;
    let entry = visible_entry(&state, &user, entry.id).await?;
    Ok(Redirect::to(&format!("/entries/{}", entry.id)).into_response())
}

/// The entry's page again, with the key of what refused the move above it.
pub(crate) async fn refused<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    id: LogEntryId,
    error: &'static str,
) -> Result<Response, AppError> {
    let entry_page = entry_page(state, user, tr, id, Some(error)).await?;
    page(StatusCode::UNPROCESSABLE_ENTITY, &entry_page)
}

async fn entry_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    id: LogEntryId,
    error: Option<&'static str>,
) -> Result<EntryPage, AppError> {
    let now = state.clock.now();
    let schedule = &state.config.schedule;
    let tz = schedule.tz();
    let actor = user.actor();
    let entry = visible_entry(state, user, id).await?;
    let event = match entry.event_id {
        Some(id) => Some(state.store.event(id).await?.ok_or(AppError::NotFound)?),
        None => None,
    };
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let sees_all = can(&actor, Action::ViewAll, now);
    let name = |id: Option<UserId>, users: &[User]| -> Option<String> {
        users
            .iter()
            .find(|u| Some(u.id) == id)
            .map(|u| u.username.to_string())
    };
    let users = if sees_all {
        state.store.users().await?
    } else {
        Vec::new()
    };

    let report = entry.report_view();
    let mut lines = Vec::new();
    let mut edited = false;
    if let Some(report) = &report {
        let old_rows = state
            .store
            .audit_history(AuditEntity::SmellReport, report.id.0)
            .await?;
        let entry_rows = state
            .store
            .audit_history(AuditEntity::LogEntry, entry.id.0)
            .await?;
        let history = entry_history(&old_rows, &entry_rows);
        edited = was_corrected(&history);
        lines = history
            .iter()
            .map(|entry| HistoryLine {
                at: time::date_time(tr.0, entry.at, tz),
                by: name(entry.actor, &users),
                action: action_key(entry.action),
                changes: entry
                    .changes
                    .iter()
                    .map(|change| match change {
                        Change::ObservedAt { from, to } => tr.t1(
                            "history-time",
                            "change",
                            &format!(
                                "{} → {}",
                                time::date_time(tr.0, *from, tz),
                                time::date_time(tr.0, *to, tz)
                            ),
                        ),
                        Change::LevelChanged { room, from, to } => format!(
                            "{} {} → {}",
                            labels::room_by_id(tr, &rooms, *room),
                            from.get(),
                            to.get()
                        ),
                        Change::RoomAdded(at) => tr.t1(
                            "history-added",
                            "name",
                            &labels::room_level(tr, &rooms, *at),
                        ),
                        Change::RoomRemoved(room) => tr.t1(
                            "history-removed",
                            "name",
                            &labels::room_by_id(tr, &rooms, *room),
                        ),
                        Change::SymptomAdded(symptom) => tr.t1(
                            "history-added",
                            "name",
                            &symptom_names(tr, &symptoms, &[*symptom]).concat(),
                        ),
                        Change::SymptomRemoved(symptom) => tr.t1(
                            "history-removed",
                            "name",
                            &symptom_names(tr, &symptoms, &[*symptom]).concat(),
                        ),
                        Change::Note { .. } => tr.t("history-note"),
                        Change::Moved { from, to } => {
                            let event = |id: &Option<EventId>| {
                                id.map_or("—".to_owned(), |id| format!("#{id}"))
                            };
                            tr.t1(
                                "history-moved",
                                "change",
                                &format!("{} → {}", event(from), event(to)),
                            )
                        }
                    })
                    .collect(),
            })
            .collect();
    }

    let editable = report.is_some()
        && can(
            &actor,
            Action::EditEntry {
                owner: entry.user_id,
            },
            now,
        );
    let mut move_options = Vec::new();
    if let (true, Some(report)) = (editable, &report) {
        let joinable = joinable_events(state, user, tr, entry.observed_at, entry.event_id);
        for (id, label) in joinable.await? {
            move_options.push(MoveOption {
                value: format!("event-{id}"),
                label,
            });
        }
        if !report.values.is_no_smell() {
            move_options.push(MoveOption {
                value: "new".to_owned(),
                label: tr.t("prompt-new-event"),
            });
        }
    }

    let none = || tr.t("report-none");
    let join = |names: Vec<String>| {
        if names.is_empty() {
            none()
        } else {
            names.join(", ")
        }
    };
    let smell = report.as_ref().map(|report| {
        let intensity = report.values.intensity().get();
        SmellFacts {
            id: report.id.0,
            intensity: format!(
                "{intensity} · {}",
                tr.t(&format!("intensity-{intensity}-name"))
            ),
            rooms: labels::report_rooms(tr, &rooms, &report.values),
            symptoms: join(symptom_names(tr, &symptoms, report.values.symptoms())),
        }
    });
    Ok(EntryPage {
        tr,
        signed_in: SignedIn::of(user, now, Tab::Events),
        event: event.map(|event| EventLink {
            id: event.id.0,
            date: time::date(tr.0, schedule.local_date(event.opened_at)),
        }),
        observed: time::date_time(tr.0, entry.observed_at, tz),
        recorded: time::date_time(tr.0, entry.recorded_at, tz),
        pm25: entry.reading.as_ref().map(|reading| {
            tr.t1(
                "entry-pm25-value",
                "value",
                &reading.value.ugm3().to_string(),
            )
        }),
        smell,
        note: entry.note.as_ref().map(|n| n.as_str().to_owned()),
        by: name(Some(entry.user_id), &users),
        edited,
        history: lines,
        editable,
        move_options,
        error,
    })
}
