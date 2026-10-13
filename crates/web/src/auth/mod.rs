//! Who is signed in: the session's user, checked against expiry on every request.

pub mod password;

use std::convert::Infallible;

use axum::extract::{FromRequestParts, OptionalFromRequestParts, Request, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::records::User;
use smoke_domain::repos::Store;
use smoke_domain::types::UserId;
use tower_sessions::Session;

use crate::cookies;
use crate::error::AppError;
use crate::i18n::{LANG_COOKIE, Tr, negotiate};
use crate::sessions::{RENEWED_AT_KEY, USER_ID_KEY};
use crate::state::AppState;

/// How often an active session's 30-day lifetime is extended (one database write).
const RENEW_AFTER: TimeDelta = TimeDelta::days(1);

/// The signed-in user, put in the request by [`load_user`].
#[derive(Debug, Clone)]
pub(crate) struct CurrentUser(pub User);

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = Redirect;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .ok_or_else(|| Redirect::to("/login"))
    }
}

impl<S: Send + Sync> OptionalFromRequestParts<S> for CurrentUser {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts.extensions.get::<CurrentUser>().cloned())
    }
}

/// Loads the session's user, signs out anyone who may no longer sign in (an expired viewer, a
/// vanished account), renews the session at most daily, and picks the page language.
pub(crate) async fn load_user<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    session: Session,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let mut account_locale = None;
    if let Some(id) = session.get::<i64>(USER_ID_KEY).await? {
        match state.store.user(UserId(id)).await? {
            Some(user) if user.actor().can_sign_in(now) => {
                renew(&session, now).await?;
                account_locale = Some(user.locale);
                request.extensions_mut().insert(CurrentUser(user));
            }
            _ => session.flush().await?,
        }
    }
    let tr = Tr(negotiate(
        account_locale,
        cookies::get(request.headers(), LANG_COOKIE).as_deref(),
        cookies::accept_language(request.headers()),
    ));
    request.extensions_mut().insert(tr);
    Ok(next.run(request).await)
}

async fn renew(session: &Session, now: DateTime<Utc>) -> Result<(), AppError> {
    let renewed = session
        .get::<i64>(RENEWED_AT_KEY)
        .await?
        .and_then(DateTime::<Utc>::from_timestamp_micros);
    if renewed.is_none_or(|at| now - at >= RENEW_AFTER) {
        session
            .insert(RENEWED_AT_KEY, now.timestamp_micros())
            .await?;
    }
    Ok(())
}

/// Sends visitors who are not signed in to the login page; htmx requests get `HX-Redirect`
/// so the whole page navigates instead of swapping the login form into a fragment.
pub(crate) async fn require_user(request: Request, next: Next) -> Response {
    if request.extensions().get::<CurrentUser>().is_some() {
        next.run(request).await
    } else if request.headers().contains_key("hx-request") {
        (StatusCode::OK, [("hx-redirect", "/login")]).into_response()
    } else {
        Redirect::to("/login").into_response()
    }
}
