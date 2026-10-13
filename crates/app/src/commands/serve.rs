//! `serve`: the web app plus its background tasks, until SIGTERM or Ctrl-C.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use smoke_domain::clock::Clock;
use smoke_domain::repos::{SessionRepo, Store};
use smoke_domain::schedule::DailySchedule;
use smoke_domain::sweep::{SweepOutcome, auto_close};
use smoke_web::{AppState, DbSessions};
use tokio::net::TcpListener;
use tower_sessions::session_store::ExpiredDeletion;
use tracing::{info, warn};

use crate::config::ServeConfig;

/// How often expired sessions are deleted.
const SESSION_SWEEP_EVERY: Duration = Duration::from_secs(60 * 60);

/// How often the auto-close sweep checks whether a 04:00 boundary has passed. A check that
/// finds nothing to do is one small read.
const EVENT_SWEEP_EVERY: Duration = Duration::from_secs(5 * 60);

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
        timezone = %config.web.schedule.tz(),
        auto_close_hour = config.web.schedule.hour(),
        "listening"
    );
    let sweeper = tokio::spawn(sweep_sessions(DbSessions::new(
        store.clone(),
        clock.clone(),
    )));
    let closer = tokio::spawn(sweep_events(
        store.clone(),
        clock.clone(),
        config.web.schedule,
    ));
    let app = smoke_web::router(AppState::new(store, clock, config.web));
    let served = smoke_web::serve(listener, app, shutdown_signal()).await;
    sweeper.abort();
    closer.abort();
    served.context("the server stopped with an error")
}

/// Runs the auto-close sweep at startup and then every few minutes (spec §11). Logging
/// requests run it too, so a sleeping host catches up on its first request.
async fn sweep_events<S: Store>(store: S, clock: Arc<dyn Clock>, schedule: DailySchedule) {
    let mut ticks = tokio::time::interval(EVENT_SWEEP_EVERY);
    loop {
        ticks.tick().await;
        match auto_close(&store, &schedule, clock.now()).await {
            Ok(SweepOutcome::Ran { closed: Some(id) }) => info!(event = id.0, "auto-closed"),
            Ok(_) => {}
            Err(error) => warn!(%error, "the auto-close sweep failed"),
        }
    }
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

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeDelta, Utc};
    use smoke_domain::clock::FakeClock;
    use smoke_domain::records::{NewEvent, NewUser};
    use smoke_domain::repos::{Commit, EventRepo, EventWrites, Transactional, UserWrites};
    use smoke_domain::types::{CloseReason, Locale, Role, Username};
    use smoke_storage_sqlite::SqliteStore;

    use super::*;

    #[tokio::test]
    async fn the_event_sweep_runs_as_soon_as_the_server_starts() {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", dir.path().join("serve.db").display());
        let evening: DateTime<Utc> = "2026-01-15T20:00:00Z".parse().unwrap();
        let clock = Arc::new(FakeClock::new(evening));
        let store = SqliteStore::connect(&url, clock.clone()).await.unwrap();
        store.migrate().await.unwrap();
        let mut tx = store.begin(None).await.unwrap();
        let owner = tx
            .create_user(NewUser {
                username: Username::new("daniele").unwrap(),
                password_hash: None,
                role: Role::Resident,
                locale: Locale::En,
                expires_at: None,
            })
            .await
            .unwrap();
        let event = tx
            .create_event(NewEvent {
                owner_id: owner.id,
                opened_at: evening,
                closed: None,
            })
            .await
            .unwrap();
        tx.commit().await.unwrap();
        clock.advance(TimeDelta::hours(12));

        let schedule = DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap();
        let task = tokio::spawn(sweep_events(store.clone(), clock, schedule));
        let mut closed = None;
        for _ in 0..100 {
            closed = store.event(event.id).await.unwrap().unwrap().closed;
            if closed.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        task.abort();
        assert_eq!(closed.map(|c| c.reason), Some(CloseReason::Auto0400));
    }
}
