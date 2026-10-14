//! The web layer of Smoke Tracker: Axum routes, Askama templates, Fluent translations and the
//! embedded static assets. It reaches the database only through the `smoke-domain` traits, so
//! every route works the same on SQLite and Postgres.

mod assets;
mod auth;
pub mod config;
mod cookies;
mod error;
mod forms;
mod header;
mod i18n;
mod labels;
mod qr;
mod request_id;
mod routes;
mod security;
mod sessions;
mod state;
mod time;

use std::future::Future;
use std::net::SocketAddr;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request};
use axum::middleware;
use axum::routing::{get, post};
use smoke_domain::repos::Store;
use tokio::net::TcpListener;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

pub use crate::auth::password;
pub use crate::sessions::DbSessions;
pub use crate::state::AppState;

/// Form posts are small; anything larger is refused before a handler runs.
const BODY_LIMIT_BYTES: usize = 16 * 1024;

/// The whole application. Static assets and the health check skip sessions and the database.
pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
    auth::password::prepare_dummy_hash();
    let signed_in = Router::new()
        .route("/", get(routes::log::show::<S>))
        .route("/guide/rating", get(routes::guide::rating::<S>))
        .route("/log", post(routes::entries::submit::<S>))
        .route("/notices/dismiss", post(routes::notices::dismiss::<S>))
        .route("/events", get(routes::events::list::<S>))
        .route("/events/{id}", get(routes::events::show::<S>))
        .route("/graphs", get(routes::graphs::show::<S>))
        .route("/events/{id}/close", post(routes::events::close::<S>))
        .route("/events/{id}/reopen", post(routes::events::reopen::<S>))
        .route("/reports/{id}", get(routes::reports::show::<S>))
        .route(
            "/reports/{id}/correct",
            get(routes::reports::correct_form::<S>).post(routes::reports::correct::<S>),
        )
        .route("/reports/{id}/move", post(routes::reports::relocate::<S>))
        .route("/people", get(routes::people::list::<S>))
        .route(
            "/people/invite",
            get(routes::people::links::form::<S>).post(routes::people::links::create::<S>),
        )
        .route("/people/{id}/link", post(routes::people::links::renew::<S>))
        .route("/people/{id}/expire", post(routes::people::end_access::<S>))
        .route(
            "/people/{id}/reactivate",
            post(routes::people::restore_access::<S>),
        )
        .route("/settings", get(routes::settings::show::<S>))
        .route("/settings/password", post(routes::settings::change::<S>))
        .route("/settings/rooms", post(routes::lists::create_room::<S>))
        .route(
            "/settings/rooms/{id}/hidden",
            post(routes::lists::hide_room::<S>),
        )
        .route(
            "/settings/symptoms",
            post(routes::lists::create_symptom::<S>),
        )
        .route(
            "/settings/symptoms/{id}/hidden",
            post(routes::lists::hide_symptom::<S>),
        )
        .route_layer(middleware::from_fn(auth::require_user));
    let pages = Router::new()
        .merge(signed_in)
        .route(
            "/login",
            get(routes::login::form).post(routes::login::submit::<S>),
        )
        .route("/logout", post(routes::login::logout))
        .route(
            "/invite/{token}",
            get(routes::accept::form::<S>).post(routes::accept::submit::<S>),
        )
        .route("/lang", post(routes::lang::choose::<S>))
        .route("/install", get(routes::install::show::<S>))
        .route("/offline", get(routes::install::offline))
        .fallback(routes::not_found)
        .layer(CatchPanicLayer::custom(error::panic_response))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::load_user::<S>,
        ))
        .layer(sessions::layer(
            state.store.clone(),
            state.clock.clone(),
            &state.config.base_url,
        ))
        .layer(middleware::from_fn_with_state(
            state.config.clone(),
            security::check_origin,
        ))
        .layer(middleware::from_fn(error::render_errors))
        .with_state(state.clone());

    Router::new()
        .route("/static/{*path}", get(assets::serve))
        .route("/sw.js", get(assets::service_worker))
        .route("/healthz", get(assets::healthz))
        .merge(pages)
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .layer(middleware::from_fn_with_state(
            state.config.base_url.is_https(),
            security::headers,
        ))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(request_span)
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(SetRequestIdLayer::x_request_id(request_id::RandomRequestId))
}

fn request_span(request: &Request) -> tracing::Span {
    let id = request_id::of(request.headers());
    let path = logged_path(request.uri().path());
    tracing::info_span!("request", id, method = %request.method(), path)
}

/// The path as logs show it. An invite token works like a password until it is used, so it
/// never reaches the logs (decision #119).
pub(crate) fn logged_path(path: &str) -> &str {
    if path.starts_with("/invite/") {
        "/invite/…"
    } else {
        path
    }
}

/// Serves `app` until `shutdown` completes, then finishes the requests in flight.
pub async fn serve(
    listener: TcpListener,
    app: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_tokens_are_cut_from_logged_paths() {
        assert_eq!(logged_path("/invite/0123abcd"), "/invite/…");
        assert_eq!(logged_path("/events/3"), "/events/3");
        assert_eq!(logged_path("/people/invite"), "/people/invite");
    }
}
