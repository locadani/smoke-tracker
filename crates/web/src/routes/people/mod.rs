//! People (spec §9 screen 6, residents only): everyone with their role and expiry, and the
//! buttons that end or restore their access.

pub(crate) mod expiry;

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use smoke_domain::DomainError;
use smoke_domain::people::{PeopleError, expire, reactivate};
use smoke_domain::permissions::{Action, can};
use smoke_domain::records::User;
use smoke_domain::repos::{Commit, Store};
use smoke_domain::types::{Role, UserId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::header::SignedIn;
use crate::i18n::Tr;
use crate::routes::people::expiry::{ExpiryDraft, ExpiryFields};
use crate::routes::people_refusal;
use crate::state::AppState;
use crate::time;

/// One person on the page.
pub(crate) struct PersonRow {
    pub(crate) id: i64,
    pub(crate) username: String,
    pub(crate) role: &'static str,
    /// "until 22/01/2026 19:05", "expired 15/01/2026 20:00" or "no end date".
    pub(crate) status: String,
    /// Invited but has not chosen a password yet.
    pub(crate) pending_setup: bool,
    pub(crate) can_expire: bool,
    /// The reactivation form, for expired accounts.
    pub(crate) reactivate: Option<ExpiryFields>,
}

#[derive(Template)]
#[template(path = "people.html")]
struct PeoplePage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    saved: Option<&'static str>,
    error: Option<&'static str>,
    people: Vec<PersonRow>,
}

pub(crate) fn role_key(role: Role) -> &'static str {
    match role {
        Role::Resident => "role-resident",
        Role::Guest => "role-guest",
        Role::Viewer => "role-viewer",
    }
}

fn status(tr: Tr, user: &User, now: DateTime<Utc>, tz: chrono_tz::Tz) -> String {
    match user.expires_at {
        None => tr.t("people-status-no-end"),
        Some(at) => {
            let when = time::date_time(tr.0, at, tz);
            let key = if at <= now {
                "people-status-expired"
            } else {
                "people-status-until"
            };
            tr.t1(key, "when", &when)
        }
    }
}

/// Answers 403 unless `user` manages people.
pub(crate) fn require_manager(user: &User, now: DateTime<Utc>) -> Result<(), AppError> {
    if can(&user.actor(), Action::ManageUsers, now) {
        Ok(())
    } else {
        Err(AppError::NotAllowed)
    }
}

/// What the page says after a refused change, or the error page it becomes.
fn refusal_key(error: PeopleError) -> Result<&'static str, AppError> {
    match error {
        PeopleError::NotExpired => Ok("people-error-not-expired"),
        PeopleError::Invalid(DomainError::ExpiryNotInFuture) => Ok("people-error-past"),
        PeopleError::Invalid(DomainError::GuestsMustExpire) => Ok("people-error-expiry"),
        PeopleError::Invalid(DomainError::ResidentsNeverExpire) => Ok("people-error-resident"),
        other => Err(people_refusal(other)),
    }
}

/// The page, with `draft` shown again in the reactivation form of the person it names.
async fn people_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    status_code: StatusCode,
    notices: (Option<&'static str>, Option<&'static str>),
    draft: Option<(UserId, ExpiryDraft)>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let tz = state.config.schedule.tz();
    let people = state
        .store
        .users()
        .await?
        .iter()
        .map(|person| {
            let expired = person.actor().is_expired(now);
            let typed = draft
                .as_ref()
                .filter(|(id, _)| *id == person.id)
                .map(|(_, typed)| typed.clone())
                .unwrap_or_default();
            PersonRow {
                id: person.id.0,
                username: person.username.to_string(),
                role: role_key(person.role),
                status: status(tr, person, now, tz),
                pending_setup: person.password_hash.is_none(),
                can_expire: person.role != Role::Resident && !expired,
                reactivate: expired
                    .then(|| expiry::fields(person.role, "", &typed))
                    .flatten(),
            }
        })
        .collect();
    let (saved, error) = notices;
    page(
        status_code,
        &PeoplePage {
            tr,
            signed_in: SignedIn::of(user, now),
            saved,
            error,
            people,
        },
    )
}

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    saved: Option<String>,
}

pub(crate) async fn list<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Query(query): Query<ListQuery>,
) -> Result<Response, AppError> {
    require_manager(&user, state.clock.now())?;
    let saved = match query.saved.as_deref() {
        Some("expired") => Some("people-saved-expired"),
        Some("reactivated") => Some("people-saved-reactivated"),
        _ => None,
    };
    people_page(&state, &user, tr, StatusCode::OK, (saved, None), None).await
}

/// Ends a guest's or viewer's access now; they are signed out everywhere.
pub(crate) async fn end_access<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let mut tx = state.store.begin(Some(user.id)).await?;
    match expire(&mut tx, &user.actor(), UserId(id), now).await {
        Ok(_) => {
            tx.commit().await?;
            Ok(Redirect::to("/people?saved=expired").into_response())
        }
        Err(error) => {
            drop(tx);
            let key = refusal_key(error)?;
            let code = StatusCode::UNPROCESSABLE_ENTITY;
            people_page(&state, &user, tr, code, (None, Some(key)), None).await
        }
    }
}

/// Gives an expired account a new expiry, read from the form's expiry fields.
pub(crate) async fn restore_access<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    require_manager(&user, now)?;
    let person = state
        .store
        .user(UserId(id))
        .await?
        .ok_or(AppError::NotFound)?;
    let draft = ExpiryDraft::from_form(&form, "");
    let refuse = async |key: &'static str| {
        let code = StatusCode::UNPROCESSABLE_ENTITY;
        let draft = Some((person.id, draft.clone()));
        people_page(&state, &user, tr, code, (None, Some(key)), draft).await
    };
    let pick = match expiry::read(person.role, &draft) {
        Ok(pick) => pick,
        Err(error) => return refuse(error.key()).await,
    };
    let expires_at = pick.resolve(now, &state.config.schedule);
    let mut tx = state.store.begin(Some(user.id)).await?;
    match reactivate(&mut tx, &user.actor(), person.id, expires_at, now).await {
        Ok(_) => {
            tx.commit().await?;
            Ok(Redirect::to("/people?saved=reactivated").into_response())
        }
        Err(error) => {
            drop(tx);
            refuse(refusal_key(error)?).await
        }
    }
}

/// Every key this module passes through a variable, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<&'static str> {
    let mut keys = vec![
        "people-status-no-end",
        "people-status-expired",
        "people-status-until",
        "people-error-not-expired",
        "people-error-past",
        "people-error-expiry",
        "people-error-resident",
        "people-saved-expired",
        "people-saved-reactivated",
    ];
    keys.extend(Role::ALL.iter().map(|&role| role_key(role)));
    keys.extend(expiry::keys());
    keys
}
