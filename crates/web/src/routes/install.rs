//! Installing the app (spec §8.3 step 4, §9.1): the page with the install button and the iPhone
//! steps, and the page the service worker shows when there is no connection.

use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;
use smoke_domain::repos::Store;

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::header::{SignedIn, Tab};
use crate::i18n::Tr;
use crate::state::AppState;

#[derive(Template)]
#[template(path = "install.html")]
struct InstallPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    /// Shown right after an account was set up.
    welcome: bool,
}

#[derive(Deserialize)]
pub(crate) struct InstallQuery {
    welcome: Option<String>,
}

/// Open to everyone: someone may want to install before signing in.
pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    user: Option<CurrentUser>,
    tr: Tr,
    Query(query): Query<InstallQuery>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    page(
        StatusCode::OK,
        &InstallPage {
            tr,
            signed_in: user.and_then(|CurrentUser(user)| SignedIn::of(&user, now, Tab::Settings)),
            welcome: query.welcome.is_some(),
        },
    )
}

#[derive(Template)]
#[template(path = "offline.html")]
struct OfflinePage {
    tr: Tr,
    signed_in: Option<SignedIn>,
}

/// Cached by the service worker when it installs, in the language of that moment.
pub(crate) async fn offline(tr: Tr) -> Result<Response, AppError> {
    page(
        StatusCode::OK,
        &OfflinePage {
            tr,
            signed_in: None,
        },
    )
}
