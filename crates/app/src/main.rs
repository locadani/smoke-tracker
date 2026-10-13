use std::process::ExitCode;

use clap::Parser;
use smoke_app::cli::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    smoke_app::init_logging();
    match smoke_app::run(Cli::parse()).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}
