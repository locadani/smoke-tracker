//! Choosing the interface language (spec §9.2): a cookie before sign-in, the account after.

use axum::Form;
use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use smoke_domain::repos::{Commit, Store, UserWrites};
use smoke_domain::types::Locale;

use crate::auth::CurrentUser;
use crate::cookies;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub(crate) struct LangForm {
    lang: String,
    /// `settings` when posted from the Settings page, which it then shows again.
    return_to: Option<String>,
}

/// Sets the `lang` cookie and, when signed in, saves the language to the account (audited).
/// An unknown language changes nothing.
pub(crate) async fn choose<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    user: Option<CurrentUser>,
    Form(form): Form<LangForm>,
) -> Result<Response, AppError> {
    let target = match (&user, form.return_to.as_deref()) {
        (None, _) => "/login",
        (Some(_), Some("settings")) => "/settings?saved=language",
        (Some(_), _) => "/",
    };
    let Ok(locale) = form.lang.parse::<Locale>() else {
        return Ok(Redirect::to(target).into_response());
    };
    if let Some(CurrentUser(user)) = user
        && user.locale != locale
    {
        let mut tx = state.store.begin(Some(user.id)).await?;
        tx.set_locale(user.id, locale).await?;
        tx.commit().await?;
    }
    let mut response = Redirect::to(target).into_response();
    response.headers_mut().append(
        SET_COOKIE,
        cookies::lang(locale, state.config.base_url.is_https()),
    );
    Ok(response)
}
