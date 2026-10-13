//! `serve`: the web app plus its background tasks, until SIGTERM or Ctrl-C.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use smoke_domain::clock::Clock;
use smoke_domain::repos::{SessionRepo, Store};
use smoke_web::{AppState, DbSessions};
use tokio::net::TcpListener;
use tower_sessions::session_store::ExpiredDeletion;
use tracing::{info, warn};

use crate::config::ServeConfig;

/// How often expired sessions are deleted.
const SESSION_SWEEP_EVERY: Duration = Duration::from_secs(60 * 60);

pub async fn run<S: Store + Clone + 'static>(
    store: S,
    clock: Arc<dyn Clock>,
    config: ServeConfig,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("could not listen on {}", config.bind_addr))?;
    info!(
        address = %listener.local_addr()?,
        origin = config.web.base_url.origin(),
        timezone = %config.schedule.tz(),
        auto_close_hour = config.schedule.hour(),
        "listening"
    );
    let sweeper = tokio::spawn(sweep_sessions(DbSessions::new(
        store.clone(),
        clock.clone(),
    )));
    let app = smoke_web::router(AppState::new(store, clock, config.web));
    let served = smoke_web::serve(listener, app, shutdown_signal()).await;
    sweeper.abort();
    served.context("the server stopped with an error")
}

/// Deletes expired sessions at startup and then every hour, through tower-sessions'
/// [`ExpiredDeletion`]. A failed run is logged and the next one still happens.
async fn sweep_sessions<S: SessionRepo + Clone + 'static>(sessions: DbSessions<S>) {
    let mut ticks = tokio::time::interval(SESSION_SWEEP_EVERY);
    loop {
        ticks.tick().await;
        if let Err(error) = sessions.delete_expired().await {
            warn!(%error, "could not delete expired sessions");
        }
    }
}

/// Resolves on Ctrl-C, or on SIGTERM (what `docker stop` sends).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                warn!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    info!("shutting down");
}
