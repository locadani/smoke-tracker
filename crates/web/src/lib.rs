//! The web layer of Smoke Tracker: Axum routes, Askama templates, Fluent translations and the
//! embedded static assets. It reaches the database only through the `smoke-domain` traits, so
//! every route works the same on SQLite and Postgres.

mod assets;
mod auth;
pub mod config;
mod cookies;
mod error;
mod forms;
mod i18n;
mod labels;
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
        .route("/guide/rating", get(routes::guide::rating))
        .route("/log", post(routes::entries::submit::<S>))
        .route("/notices/dismiss", post(routes::notices::dismiss::<S>))
        .route_layer(middleware::from_fn(auth::require_user));
    let pages = Router::new()
        .merge(signed_in)
        .route(
            "/login",
            get(routes::login::form).post(routes::login::submit::<S>),
        )
        .route("/logout", post(routes::login::logout))
        .route("/lang", post(routes::lang::choose::<S>))
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
    tracing::info_span!("request", id, method = %request.method(), path = request.uri().path())
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
