//! Opening an invite or reset link (spec §8.3 step 3): the invitee chooses a password and a
//! language and is signed in. Works without an account; the link itself is the credential, for
//! five minutes and once.

use askama::Template;
use axum::Form;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use smoke_domain::people::{PeopleError, accept};
use smoke_domain::records::Invite;
use smoke_domain::repos::{Commit, Store};
use smoke_domain::types::{InvitePurpose, Locale, NewPassword, TokenHash};
use tower_sessions::Session;

use crate::auth::{self, password};
use crate::cookies;
use crate::error::{AppError, page};
use crate::header::SignedIn;
use crate::i18n::Tr;
use crate::routes::people_refusal;
use crate::state::AppState;

#[derive(Template)]
#[template(path = "accept.html")]
struct AcceptPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    username: String,
    setup: bool,
    error: Option<&'static str>,
}

/// The usable invite behind `token` and its person's username, or the "link no longer works"
/// page.
async fn usable<S: Store>(
    state: &AppState<S>,
    token: &TokenHash,
) -> Result<(Invite, String), AppError> {
    let now = state.clock.now();
    let invite = state
        .store
        .invite_by_token(token)
        .await?
        .filter(|invite| invite.is_usable(now))
        .ok_or(AppError::LinkInvalid)?;
    let person = state
        .store
        .user(invite.user_id)
        .await?
        .ok_or(AppError::LinkInvalid)?;
    Ok((invite, person.username.to_string()))
}

fn accept_page(
    status: StatusCode,
    tr: Tr,
    invite: &Invite,
    username: String,
    error: Option<&'static str>,
) -> Result<Response, AppError> {
    page(
        status,
        &AcceptPage {
            tr,
            signed_in: None,
            username,
            setup: invite.purpose == InvitePurpose::Setup,
            error,
        },
    )
}

pub(crate) async fn form<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    tr: Tr,
    Path(token): Path<String>,
) -> Result<Response, AppError> {
    let (invite, username) = usable(&state, &TokenHash::of(&token)).await?;
    accept_page(StatusCode::OK, tr, &invite, username, None)
}

#[derive(Deserialize)]
pub(crate) struct AcceptForm {
    password: String,
    repeat: String,
    lang: Option<String>,
}

/// Sets the password and language, uses up the link, and signs the person in on this device.
/// After a first setup the next page offers to install the app (spec §8.3 step 4).
pub(crate) async fn submit<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    session: Session,
    tr: Tr,
    Path(token): Path<String>,
    Form(form): Form<AcceptForm>,
) -> Result<Response, AppError> {
    let token = TokenHash::of(&token);
    let (invite, username) = usable(&state, &token).await?;
    let locale = form
        .lang
        .and_then(|lang| lang.parse::<Locale>().ok())
        .unwrap_or(tr.0);
    if form.password != form.repeat {
        let error = Some("settings-password-mismatch");
        return accept_page(
            StatusCode::UNPROCESSABLE_ENTITY,
            tr,
            &invite,
            username,
            error,
        );
    }
    let Ok(new_password) = NewPassword::new(form.password) else {
        let error = Some("settings-password-length");
        return accept_page(
            StatusCode::UNPROCESSABLE_ENTITY,
            tr,
            &invite,
            username,
            error,
        );
    };
    let hash = password::run_limited(state.hash_permits.clone(), move || {
        password::hash(&new_password)
    })
    .await?
    .map_err(AppError::internal)?;
    let now = state.clock.now();
    let mut tx = state.store.begin(Some(invite.user_id)).await?;
    let person = match accept(&mut tx, &token, hash, locale, now).await {
        Ok(person) => person,
        Err(PeopleError::LinkInvalid) => return Err(AppError::LinkInvalid),
        Err(other) => return Err(people_refusal(other)),
    };
    tx.commit().await?;
    tracing::info!(user = person.id.0, purpose = %invite.purpose, "link used");
    let target = if !person.actor().can_sign_in(now) {
        "/login"
    } else {
        auth::start_session(&session, &person, now).await?;
        match invite.purpose {
            InvitePurpose::Setup => "/install?welcome",
            InvitePurpose::PasswordReset => "/",
        }
    };
    let mut response = Redirect::to(target).into_response();
    response.headers_mut().append(
        SET_COOKIE,
        cookies::lang(person.locale, state.config.base_url.is_https()),
    );
    Ok(response)
}
