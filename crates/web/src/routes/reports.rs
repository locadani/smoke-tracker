//! A report's page (spec §9, screen 4): its values, observed and recorded times, the "edited"
//! badge and History; "Correct a mistake" and "Move to another event" for those allowed to.

use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::events::can_join_backdated;
use smoke_domain::history::{Change, report_history, was_corrected};
use smoke_domain::logbook::{MoveTarget, correct_report, move_report};
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::{Room, SmellReport, Symptom, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::types::{EventId, ReportId, Role, RoomId, SymptomId, UserId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::i18n::Tr;
use crate::routes::entries::{read_time, read_values};
use crate::routes::guide::{Level, levels};
use crate::routes::log::{Draft, Fields, fields};
use crate::routes::refusal;
use crate::state::AppState;
use crate::{labels, time};

/// The report, if it exists and `user` may see it; otherwise "not found", so nobody learns
/// that someone else's report exists.
async fn visible_report<S: Store>(
    state: &AppState<S>,
    user: &User,
    id: ReportId,
) -> Result<SmellReport, AppError> {
    let now = state.clock.now();
    state
        .store
        .report(id)
        .await?
        .filter(|r| can_see(&user.actor(), r.user_id, now))
        .ok_or(AppError::NotFound)
}

fn room_names(tr: Tr, rooms: &[Room], ids: &[RoomId]) -> Vec<String> {
    rooms
        .iter()
        .filter(|r| ids.contains(&r.id))
        .map(|r| labels::room(tr, r))
        .collect()
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

struct MoveOption {
    value: String,
    label: String,
}

#[derive(Template)]
#[template(path = "report.html")]
struct ReportPage {
    tr: Tr,
    signed_in: Option<String>,
    id: i64,
    event_id: i64,
    event_date: String,
    observed: String,
    recorded: String,
    intensity: String,
    rooms: String,
    symptoms: String,
    note: Option<String>,
    by: Option<String>,
    edited: bool,
    deleted: bool,
    history: Vec<HistoryLine>,
    /// Shown to those who may correct and move the report.
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
    let report_page = report_page(&state, &user, tr, ReportId(id), None).await?;
    page(StatusCode::OK, &report_page)
}

async fn report_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    id: ReportId,
    error: Option<&'static str>,
) -> Result<ReportPage, AppError> {
    let now = state.clock.now();
    let schedule = &state.config.schedule;
    let tz = schedule.tz();
    let actor = user.actor();
    let report = visible_report(state, user, id).await?;
    let event = state
        .store
        .event(report.event_id)
        .await?
        .ok_or(AppError::NotFound)?;
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

    let entries = state
        .store
        .audit_history(AuditEntity::SmellReport, id.0)
        .await?;
    let history = report_history(&entries);
    let lines = history
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
                    Change::Intensity { from, to } => {
                        tr.t1("history-intensity", "change", &format!("{from} → {to}"))
                    }
                    Change::RoomAdded(room) => tr.t1(
                        "history-added",
                        "name",
                        &room_names(tr, &rooms, &[*room]).concat(),
                    ),
                    Change::RoomRemoved(room) => tr.t1(
                        "history-removed",
                        "name",
                        &room_names(tr, &rooms, &[*room]).concat(),
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
                        tr.t1("history-moved", "change", &format!("#{from} → #{to}"))
                    }
                })
                .collect(),
        })
        .collect();

    let editable = !report.is_deleted()
        && can(
            &actor,
            Action::EditEntry {
                owner: report.user_id,
            },
            now,
        );
    let mut move_options = Vec::new();
    if editable {
        let (day_start, day_end) = schedule.day_span(schedule.local_date(report.observed_at));
        for target in state.store.events_between(day_start, day_end).await? {
            if target.id != report.event_id
                && can_join_backdated(&target, report.observed_at, schedule)
            {
                let mut label = tr.t1(
                    "prompt-started-at",
                    "time",
                    &time::clock(target.opened_at, tz),
                );
                if user.role != Role::Guest
                    && let Some(closed) = target.closed
                {
                    label.push_str(", ");
                    label.push_str(&tr.t1("prompt-ended-at", "time", &time::clock(closed.at, tz)));
                }
                move_options.push(MoveOption {
                    value: format!("event-{}", target.id),
                    label,
                });
            }
        }
        move_options.push(MoveOption {
            value: "new".to_owned(),
            label: tr.t("prompt-new-event"),
        });
    }

    let none = || tr.t("report-none");
    let join = |names: Vec<String>| {
        if names.is_empty() {
            none()
        } else {
            names.join(", ")
        }
    };
    let intensity = report.values.intensity().get();
    Ok(ReportPage {
        tr,
        signed_in: Some(user.username.to_string()),
        id: id.0,
        event_id: report.event_id.0,
        event_date: time::date(tr.0, schedule.local_date(event.opened_at)),
        observed: time::date_time(tr.0, report.observed_at, tz),
        recorded: time::date_time(tr.0, report.recorded_at, tz),
        intensity: format!(
            "{intensity} · {}",
            tr.t(&format!("intensity-{intensity}-name"))
        ),
        rooms: join(room_names(tr, &rooms, report.values.rooms())),
        symptoms: join(symptom_names(tr, &symptoms, report.values.symptoms())),
        note: report.values.note().map(str::to_owned),
        by: name(Some(report.user_id), &users),
        edited: was_corrected(&history),
        deleted: report.is_deleted(),
        history: lines,
        editable,
        move_options,
        error,
    })
}

pub(crate) struct CorrectForm {
    fields: Fields,
    observed_at: String,
    note: String,
    error: Option<&'static str>,
}

#[derive(Template)]
#[template(path = "correct.html")]
struct CorrectPage {
    tr: Tr,
    signed_in: Option<String>,
    id: i64,
    levels: Vec<Level>,
    form: CorrectForm,
}

/// The report, if `user` may correct and move it.
async fn editable_report<S: Store>(
    state: &AppState<S>,
    user: &User,
    id: ReportId,
) -> Result<SmellReport, AppError> {
    let report = visible_report(state, user, id).await?;
    let allowed = can(
        &user.actor(),
        Action::EditEntry {
            owner: report.user_id,
        },
        state.clock.now(),
    );
    if report.is_deleted() {
        Err(AppError::NotFound)
    } else if allowed {
        Ok(report)
    } else {
        Err(AppError::NotAllowed)
    }
}

async fn correct_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    report: &SmellReport,
    draft: Draft,
    error: Option<&'static str>,
) -> Result<CorrectPage, AppError> {
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let keep = (report.values.rooms(), report.values.symptoms());
    Ok(CorrectPage {
        tr,
        signed_in: Some(user.username.to_string()),
        id: report.id.0,
        levels: levels(tr),
        form: CorrectForm {
            fields: fields(tr, &rooms, &symptoms, &draft, keep),
            observed_at: draft.observed_at,
            note: draft.note,
            error,
        },
    })
}

/// "Correct a mistake": the form, pre-filled with the report as it is.
pub(crate) async fn correct_form<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let report = editable_report(&state, &user, ReportId(id)).await?;
    let draft = Draft {
        intensity: Some(report.values.intensity().get()),
        rooms: report.values.rooms().to_vec(),
        symptoms: report.values.symptoms().to_vec(),
        pm25: String::new(),
        observed_at: time::input_value(report.observed_at, state.config.schedule.tz()),
        note: report.values.note().unwrap_or_default().to_owned(),
    };
    let correct_page = correct_page(&state, &user, tr, &report, draft, None).await?;
    page(StatusCode::OK, &correct_page)
}

pub(crate) async fn correct<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let schedule = state.config.schedule;
    let report = editable_report(&state, &user, ReportId(id)).await?;
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let offered_rooms: Vec<RoomId> = rooms
        .iter()
        .filter(|r| r.hidden_at.is_none() || report.values.rooms().contains(&r.id))
        .map(|r| r.id)
        .collect();
    let offered_symptoms: Vec<SymptomId> = symptoms
        .iter()
        .filter(|s| s.hidden_at.is_none() || report.values.symptoms().contains(&s.id))
        .map(|s| s.id)
        .collect();
    let parsed = read_time(&form, "observed_at", schedule.tz()).and_then(|at| {
        let values = read_values(&form, &offered_rooms, &offered_symptoms)?;
        Ok((at.unwrap_or(report.observed_at), values))
    });
    let error = match parsed {
        Ok((observed_at, values)) => {
            let mut tx = state.store.begin(Some(user.id)).await?;
            match correct_report(
                &mut tx,
                &user.actor(),
                report.id,
                observed_at,
                values,
                &schedule,
                now,
            )
            .await
            {
                Ok(_) => {
                    tx.commit().await?;
                    return Ok(Redirect::to(&format!("/reports/{id}")).into_response());
                }
                Err(error) => refusal(error)?,
            }
        }
        Err(key) => key,
    };
    let draft = Draft::from_form(&form);
    let correct_page = correct_page(&state, &user, tr, &report, draft, Some(error)).await?;
    page(StatusCode::UNPROCESSABLE_ENTITY, &correct_page)
}

/// "Move to another event": `target` is `event-<id>` or `new`.
pub(crate) async fn relocate<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    visible_report(&state, &user, ReportId(id)).await?;
    let target = match form.get("target") {
        Some("new") => MoveTarget::New,
        Some(value) => match value.strip_prefix("event-").and_then(|v| v.parse().ok()) {
            Some(event) => MoveTarget::Event(EventId(event)),
            None => return Err(AppError::NotFound),
        },
        None => return Err(AppError::NotFound),
    };
    let mut tx = state.store.begin(Some(user.id)).await?;
    let schedule = state.config.schedule;
    match move_report(&mut tx, &user.actor(), ReportId(id), target, &schedule, now).await {
        Ok(_) => {
            tx.commit().await?;
            Ok(Redirect::to(&format!("/reports/{id}")).into_response())
        }
        Err(error) => {
            drop(tx);
            let key = refusal(error)?;
            let report_page = report_page(&state, &user, tr, ReportId(id), Some(key)).await?;
            page(StatusCode::UNPROCESSABLE_ENTITY, &report_page)
        }
    }
}
