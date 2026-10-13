//! Errors a handler can return, and the translated error pages they become.

use std::any::Any;
use std::error::Error as StdError;

use askama::Template;
use axum::body::Body;
use axum::extract::Request;
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use smoke_domain::repos::StoreError;

use crate::cookies;
use crate::i18n::{LANG_COOKIE, Tr, negotiate};
use crate::request_id;

#[derive(Debug)]
pub(crate) enum AppError {
    NotFound,
    /// A cross-site request (the Origin check).
    Forbidden,
    /// The signed-in user's role does not allow this (spec §4).
    NotAllowed,
    /// Anything unexpected. Logged with the request id; the user sees only the id.
    Internal(Box<dyn StdError + Send + Sync>),
}

impl AppError {
    pub(crate) fn internal(error: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::Internal(error.into())
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Forbidden | Self::NotAllowed => StatusCode::FORBIDDEN,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// The title and message keys of the page this error becomes.
    fn keys(&self) -> (&'static str, &'static str) {
        match self {
            Self::NotFound => ("error-not-found-title", "error-not-found"),
            Self::Forbidden => ("error-forbidden-title", "error-forbidden"),
            Self::NotAllowed => ("error-not-allowed-title", "error-not-allowed"),
            Self::Internal(_) => ("error-internal-title", "error-internal"),
        }
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        Self::internal(error)
    }
}

impl From<askama::Error> for AppError {
    fn from(error: askama::Error) -> Self {
        Self::internal(error)
    }
}

impl From<tower_sessions::session::Error> for AppError {
    fn from(error: tower_sessions::session::Error) -> Self {
        Self::internal(error)
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::internal(error)
    }
}

/// Marks a response whose body [`render_errors`] should replace with a translated page.
#[derive(Debug, Clone, Copy)]
struct ErrorPage {
    title_key: &'static str,
    message_key: &'static str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        if let Self::Internal(error) = &self {
            tracing::error!(error = %chain(error.as_ref()), "request failed");
        }
        let (title_key, message_key) = self.keys();
        let mut response = status.into_response();
        response.extensions_mut().insert(ErrorPage {
            title_key,
            message_key,
        });
        response
    }
}

/// The error and all its sources, e.g. `database error: no such table: users`.
fn chain(error: &(dyn StdError + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

/// For `CatchPanicLayer`: a panicking handler becomes an ordinary internal error.
pub(crate) fn panic_response(panic: Box<dyn Any + Send + 'static>) -> Response {
    let message = panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a handler panicked".to_owned());
    AppError::internal(message).into_response()
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    tr: Tr,
    signed_in: Option<String>,
    title_key: &'static str,
    message_key: &'static str,
    request_id: String,
    internal: bool,
}

/// Replaces the body of every [`AppError`] response with a translated page. The language comes
/// from the `lang` cookie or the browser; sign-in sets the cookie to the account's language.
pub(crate) async fn render_errors(request: Request, next: Next) -> Response {
    let tr = Tr(negotiate(
        None,
        cookies::get(request.headers(), LANG_COOKIE).as_deref(),
        cookies::accept_language(request.headers()),
    ));
    let request_id = request_id::of(request.headers()).to_owned();
    let response = next.run(request).await;
    let Some(ErrorPage {
        title_key,
        message_key,
    }) = response.extensions().get::<ErrorPage>().copied()
    else {
        return response;
    };
    let status = response.status();
    let page = ErrorTemplate {
        tr,
        signed_in: None,
        title_key,
        message_key,
        request_id,
        internal: status.is_server_error(),
    };
    let html = match page.render() {
        Ok(html) => html,
        Err(error) => {
            tracing::error!(%error, "the error page failed to render");
            return (status, "Something went wrong").into_response();
        }
    };
    let (mut parts, _) = response.into_parts();
    parts.headers.remove(CONTENT_LENGTH);
    parts.headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    Response::from_parts(parts, Body::from(html))
}

/// Renders a page template with the given status.
pub(crate) fn page(status: StatusCode, template: &impl Template) -> Result<Response, AppError> {
    Ok((status, Html(template.render()?)).into_response())
}

/// Every message key the error pages use, for the translation tests.
#[cfg(test)]
pub(crate) const KEYS: [&str; 8] = [
    "error-not-found-title",
    "error-not-found",
    "error-forbidden-title",
    "error-forbidden",
    "error-not-allowed-title",
    "error-not-allowed",
    "error-internal-title",
    "error-internal",
];
