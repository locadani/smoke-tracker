//! Sign in and sign out (spec §8.1).

use askama::Template;
use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use smoke_domain::repos::Store;
use smoke_domain::types::Username;
use tower_sessions::Session;

use crate::auth::client_ip::ClientIp;
use crate::auth::limiter::LoginLimiter;
use crate::auth::{CurrentUser, password};
use crate::cookies;
use crate::error::{AppError, page};
use crate::i18n::Tr;
use crate::sessions::{RENEWED_AT_KEY, USER_ID_KEY};
use crate::state::AppState;

/// What the login page tells the user after a failed attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Notice {
    /// Wrong password or unknown username: deliberately the same message.
    Failed,
    /// Too many failures for this username or from this address.
    Locked,
    /// Correct password, but a viewer account past its expiry.
    Expired,
}

impl Notice {
    #[cfg(test)]
    pub(crate) const ALL: [Notice; 3] = [Notice::Failed, Notice::Locked, Notice::Expired];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Notice::Failed => "login-failed",
            Notice::Locked => "login-locked",
            Notice::Expired => "login-expired",
        }
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage {
    tr: Tr,
    signed_in: Option<String>,
    username: String,
    notice: Option<Notice>,
}

fn login_page(
    status: StatusCode,
    tr: Tr,
    username: String,
    notice: Option<Notice>,
) -> Result<Response, AppError> {
    page(
        status,
        &LoginPage {
            tr,
            signed_in: None,
            username,
            notice,
        },
    )
}

pub(crate) async fn form(user: Option<CurrentUser>, tr: Tr) -> Result<Response, AppError> {
    if user.is_some() {
        return Ok(Redirect::to("/").into_response());
    }
    login_page(StatusCode::OK, tr, String::new(), None)
}

#[derive(Deserialize)]
pub(crate) struct LoginForm {
    username: String,
    password: String,
}

/// Unknown usernames and wrong passwords get the same answer after the same work (one Argon2
/// verification, under the same permit), so neither the message nor the timing tells which
/// accounts exist. The limiter runs first: a refused attempt never waits for a permit.
pub(crate) async fn submit<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    session: Session,
    ClientIp(address): ClientIp,
    tr: Tr,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    tracing::debug!(client = %address, "sign-in attempt");
    let now = state.clock.now();
    let key = LoginLimiter::key(&form.username);
    if !state.limiter.begin(&key, address, now) {
        return login_page(
            StatusCode::TOO_MANY_REQUESTS,
            tr,
            form.username,
            Some(Notice::Locked),
        );
    }
    let user = match Username::new(&form.username) {
        Ok(username) => state.store.user_by_username(&username).await?,
        Err(_) => None,
    };
    let stored = user.as_ref().and_then(|u| u.password_hash.clone());
    let attempt = form.password;
    let valid = password::run_limited(state.hash_permits.clone(), move || {
        password::verify(stored.as_ref(), &attempt)
    })
    .await?;
    let Some(user) = user.filter(|_| valid) else {
        return login_page(
            StatusCode::UNAUTHORIZED,
            tr,
            form.username,
            Some(Notice::Failed),
        );
    };
    state.limiter.succeeded(&key, address, now);
    if !user.actor().can_sign_in(now) {
        return login_page(
            StatusCode::FORBIDDEN,
            tr,
            form.username,
            Some(Notice::Expired),
        );
    }
    session.cycle_id().await?;
    session.insert(USER_ID_KEY, user.id.0).await?;
    session
        .insert(RENEWED_AT_KEY, now.timestamp_micros())
        .await?;
    tracing::info!(user = user.id.0, "signed in");
    let mut response = Redirect::to("/").into_response();
    response.headers_mut().append(
        SET_COOKIE,
        cookies::lang(user.locale, state.config.base_url.is_https()),
    );
    Ok(response)
}

/// Deletes the session from the database and clears the cookie.
pub(crate) async fn logout(session: Session) -> Result<Redirect, AppError> {
    session.flush().await?;
    Ok(Redirect::to("/login"))
}
