//! The routes that change a smell report: "Correct a mistake" and "Move to another event".
//! The report's page itself is its entry's (`routes::entry_page`).

use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use smoke_domain::logbook::{MoveTarget, ReportCorrection, correct_report, move_report};
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::{ReportView, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{EventId, ReportId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::header::{SignedIn, Tab};
use crate::i18n::Tr;
use crate::routes::entries::{Smell, read_time, read_values};
use crate::routes::entry_page;
use crate::routes::log::{Draft, Fields, fields, offered};
use crate::routes::refusal;
use crate::state::AppState;
use crate::time;

/// The report, if it exists and `user` may see it; otherwise "not found", so nobody learns
/// that someone else's report exists.
async fn visible_report<S: Store>(
    state: &AppState<S>,
    user: &User,
    id: ReportId,
) -> Result<ReportView, AppError> {
    let now = state.clock.now();
    state
        .store
        .entry_of_report(id)
        .await?
        .and_then(|entry| entry.report_view())
        .filter(|r| can_see(&user.actor(), r.user_id, now))
        .ok_or(AppError::NotFound)
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
    signed_in: Option<SignedIn>,
    id: i64,
    entry_id: i64,
    /// A no-smell check: its levels stay at 0.
    check: bool,
    form: CorrectForm,
}

/// The report, if `user` may correct and move it.
async fn editable_report<S: Store>(
    state: &AppState<S>,
    user: &User,
    id: ReportId,
) -> Result<ReportView, AppError> {
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
    report: &ReportView,
    draft: Draft,
    error: Option<&'static str>,
) -> Result<CorrectPage, AppError> {
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let room_ids = report.values.room_ids();
    let keep = (room_ids.as_slice(), report.values.symptoms());
    Ok(CorrectPage {
        tr,
        signed_in: SignedIn::of(user, state.clock.now(), Tab::Events),
        id: report.id.0,
        entry_id: report.entry_id.0,
        check: report.event_id.is_none(),
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
        rooms: report.values.room_ids(),
        levels: report
            .values
            .rooms()
            .iter()
            .map(|r| (r.room, r.level.get()))
            .collect(),
        symptoms: report.values.symptoms().to_vec(),
        pm25: String::new(),
        observed_at: time::input_value(report.observed_at, state.config.schedule.tz()),
        note: report
            .note
            .as_ref()
            .map(|n| n.as_str().to_owned())
            .unwrap_or_default(),
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
    auto_close(&state.store, &state.config.schedule, now).await?;
    let schedule = state.config.schedule;
    let report = editable_report(&state, &user, ReportId(id)).await?;
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let room_ids = report.values.room_ids();
    let keep = (room_ids.as_slice(), report.values.symptoms());
    let (offered_rooms, offered_symptoms) = offered(&rooms, &symptoms, keep);
    let parsed = read_time(&form, "observed_at", schedule.tz()).and_then(|at| {
        let whole_home = report.values.rooms().is_empty();
        let smell = Smell::Correction { whole_home };
        let (values, note) = read_values(&form, &offered_rooms, &offered_symptoms, smell)?;
        Ok((at.unwrap_or(report.observed_at), values, note))
    });
    let error = match parsed {
        Ok((observed_at, values, note)) => {
            let mut tx = state.store.begin(Some(user.id)).await?;
            let correction = ReportCorrection {
                observed_at,
                values,
                note,
            };
            match correct_report(
                &mut tx,
                &user.actor(),
                report.id,
                correction,
                &schedule,
                now,
            )
            .await
            {
                Ok(_) => {
                    tx.commit().await?;
                    let to = format!("/entries/{}", report.entry_id);
                    return Ok(Redirect::to(&to).into_response());
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
    auto_close(&state.store, &state.config.schedule, now).await?;
    let report = visible_report(&state, &user, ReportId(id)).await?;
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
            Ok(Redirect::to(&format!("/entries/{}", report.entry_id)).into_response())
        }
        Err(error) => {
            drop(tx);
            let key = refusal(error)?;
            entry_page::refused(&state, &user, tr, report.entry_id, key).await
        }
    }
}
