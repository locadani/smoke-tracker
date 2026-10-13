//! The Smoke Tracker binary as a library: configuration, backend selection, the commands and
//! the background tasks. `main.rs` only parses the command line and calls [`run`].

pub mod backend;
pub mod cli;
pub mod commands;
pub mod config;

use std::io::IsTerminal;
use std::process::ExitCode;
use std::sync::Arc;

use smoke_domain::clock::{Clock, SystemClock};
use smoke_domain::types::Username;
use tracing_subscriber::EnvFilter;

use crate::backend::{Backend, with_store};
use crate::cli::{Cli, Command};
use crate::commands::{create_resident, serve, verify_audit};
use crate::config::{ServeConfig, env};

/// Logs go to stderr, filtered by `RUST_LOG` (default `info`), so command output on stdout
/// stays clean. Colours only on a terminal, not in container logs.
pub fn init_logging() {
    let stderr = std::io::stderr();
    tracing_subscriber::fmt()
        .with_ansi(stderr.is_terminal())
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
}

/// Runs one command. `Ok` carries the exit code: 0, or 1 when `verify-audit` finds problems.
/// Errors exit with 2.
pub async fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let database_url = config::database_url(env)?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    match cli.command {
        Command::Serve => {
            let config = ServeConfig::from_lookup(env)?;
            let backend = Backend::connect(&database_url, clock.clone()).await?;
            backend.prepare().await?;
            with_store!(backend, |store| serve::run(store, clock, config).await)?;
        }
        Command::Migrate => {
            Backend::connect(&database_url, clock)
                .await?
                .prepare()
                .await?;
            println!("The database is up to date.");
        }
        Command::CreateResident { username, locale } => {
            let username = Username::new(&username)?;
            let password = create_resident::prompt_password()?;
            let backend = Backend::connect(&database_url, clock).await?;
            backend.prepare().await?;
            let user = with_store!(&backend, |store| {
                create_resident::create(store, username, password, locale).await
            })?;
            println!("Created resident {}.", user.username);
        }
        Command::VerifyAudit => {
            let backend = Backend::connect(&database_url, clock).await?;
            let report = with_store!(&backend, |store| verify_audit::run(store).await)?;
            print!("{}", verify_audit::describe(&report));
            if !report.is_ok() {
                return Ok(ExitCode::from(1));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
