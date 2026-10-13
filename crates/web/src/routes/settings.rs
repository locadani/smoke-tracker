//! Settings (spec §9 screen 7): language and password for everyone, and the room and symptom
//! lists for residents (edited in [`crate::routes::lists`]).

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use smoke_domain::people::change_password;
use smoke_domain::permissions::{Action, can};
use smoke_domain::records::User;
use smoke_domain::repos::{Commit, Store};
use smoke_domain::types::{Locale, NewPassword};
use tower_sessions::Session;

use crate::auth::client_ip::ClientIp;
use crate::auth::limiter::LoginLimiter;
use crate::auth::{self, CurrentUser, password};
use crate::error::{AppError, page};
use crate::i18n::Tr;
use crate::routes::lists::{self, ListsView};
use crate::routes::people_refusal;
use crate::state::AppState;

/// The languages, each named in itself.
pub(crate) const LANGUAGES: [(Locale, &str); 3] = [
    (Locale::En, "English"),
    (Locale::It, "Italiano"),
    (Locale::Tr, "Türkçe"),
];

pub(crate) struct LanguageChoice {
    pub(crate) value: &'static str,
    pub(crate) name: &'static str,
    pub(crate) current: bool,
}

/// The messages above the page's sections: what was just saved, or why a form was refused.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Notices {
    pub(crate) saved: Option<&'static str>,
    pub(crate) password_error: Option<&'static str>,
    pub(crate) list_error: Option<&'static str>,
}

#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsPage {
    tr: Tr,
    signed_in: Option<String>,
    /// For password managers, which file the new password under this name.
    username: String,
    languages: Vec<LanguageChoice>,
    notices: Notices,
    /// `None` for people who may not edit the lists.
    lists: Option<ListsView>,
}

pub(crate) async fn settings_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    status: StatusCode,
    notices: Notices,
) -> Result<Response, AppError> {
    let lists = if can(&user.actor(), Action::EditLists, state.clock.now()) {
        Some(lists::view(state, tr).await?)
    } else {
        None
    };
    page(
        status,
        &SettingsPage {
            tr,
            signed_in: Some(user.username.to_string()),
            username: user.username.to_string(),
            languages: LANGUAGES
                .iter()
                .map(|&(locale, name)| LanguageChoice {
                    value: locale.as_str(),
                    name,
                    current: locale == user.locale,
                })
                .collect(),
            notices,
            lists,
        },
    )
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
    let saved = match query.saved.as_deref() {
        Some("language") => Some("settings-saved-language"),
        Some("password") => Some("settings-saved-password"),
        Some("lists") => Some("settings-saved-lists"),
        _ => None,
    };
    let notices = Notices {
        saved,
        ..Notices::default()
    };
    settings_page(&state, &user, tr, StatusCode::OK, notices).await
}

#[derive(Deserialize)]
pub(crate) struct PasswordForm {
    current: String,
    new: String,
    repeat: String,
}

/// Why a password change was refused, as the message shown above the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refused {
    Mismatch,
    Length,
    WrongCurrent,
    Locked,
}

impl Refused {
    #[cfg(test)]
    const ALL: [Refused; 4] = [
        Refused::Mismatch,
        Refused::Length,
        Refused::WrongCurrent,
        Refused::Locked,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Mismatch => "settings-password-mismatch",
            Self::Length => "settings-password-length",
            Self::WrongCurrent => "settings-password-wrong",
            Self::Locked => "login-locked",
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::Locked => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

/// Changes the signed-in person's password (decision #116). The current password is checked
/// like a sign-in attempt, under the same limits. Every session of the account ends; this
/// device gets a new one, so only the other devices are signed out.
pub(crate) async fn change<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    session: Session,
    ClientIp(address): ClientIp,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Form(form): Form<PasswordForm>,
) -> Result<Response, AppError> {
    let refuse = async |refused: Refused| {
        let notices = Notices {
            password_error: Some(refused.key()),
            ..Notices::default()
        };
        settings_page(&state, &user, tr, refused.status(), notices).await
    };
    if form.new != form.repeat {
        return refuse(Refused::Mismatch).await;
    }
    let Ok(new_password) = NewPassword::new(form.new) else {
        return refuse(Refused::Length).await;
    };
    let now = state.clock.now();
    let key = LoginLimiter::key(user.username.as_str());
    if !state.limiter.begin(&key, address, now) {
        return refuse(Refused::Locked).await;
    }
    let stored = user.password_hash.clone();
    let current = form.current;
    let new_hash = password::run_limited(state.hash_permits.clone(), move || {
        password::verify(stored.as_ref(), &current).then(|| password::hash(&new_password))
    })
    .await?;
    let Some(new_hash) = new_hash else {
        return refuse(Refused::WrongCurrent).await;
    };
    state.limiter.succeeded(&key, address, now);
    let new_hash = new_hash.map_err(AppError::internal)?;
    let mut tx = state.store.begin(Some(user.id)).await?;
    let changed = change_password(&mut tx, &user.actor(), new_hash, now)
        .await
        .map_err(people_refusal)?;
    tx.commit().await?;
    auth::start_session(&session, &changed, now).await?;
    tracing::info!(user = user.id.0, "password changed");
    Ok(Redirect::to("/settings?saved=password").into_response())
}

/// Every key this module looks up through a variable, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<&'static str> {
    let mut keys: Vec<_> = Refused::ALL.iter().map(|r| r.key()).collect();
    keys.extend([
        "settings-saved-language",
        "settings-saved-password",
        "settings-saved-lists",
    ]);
    keys
}
