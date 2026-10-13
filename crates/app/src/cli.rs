//! The command line (spec §10.4).

use clap::{Parser, Subcommand};
use smoke_domain::types::Locale;

#[derive(Debug, Parser)]
#[command(
    name = "smoke-tracker",
    version,
    about = "Log cigarette smoke drifting in from the neighbours"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Apply migrations, then serve the web app on BIND_ADDR.
    Serve,
    /// Apply the database migrations and add the starter rooms and symptoms.
    Migrate,
    /// Create a resident account. Prompts for the password; nothing is echoed.
    CreateResident {
        /// 3–32 characters: letters, digits, '.', '_' or '-'. Stored in lowercase.
        username: String,
        /// Interface language: en, it or tr.
        #[arg(long, default_value = "en", value_parser = parse_locale)]
        locale: Locale,
    },
    /// Check the audit hash chain and that every record matches its latest audit entry.
    /// Exits with 1 when anything is wrong.
    VerifyAudit,
}

fn parse_locale(text: &str) -> Result<Locale, String> {
    text.parse()
        .map_err(|_| format!("expected en, it or tr, got {text:?}"))
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(args: &[&str]) -> Result<Command, clap::Error> {
        Cli::try_parse_from(std::iter::once("smoke-tracker").chain(args.iter().copied()))
            .map(|cli| cli.command)
    }

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_command_parses() {
        assert_eq!(parse(&["serve"]).unwrap(), Command::Serve);
        assert_eq!(parse(&["migrate"]).unwrap(), Command::Migrate);
        assert_eq!(parse(&["verify-audit"]).unwrap(), Command::VerifyAudit);
        assert_eq!(
            parse(&["create-resident", "daniele"]).unwrap(),
            Command::CreateResident {
                username: "daniele".into(),
                locale: Locale::En
            }
        );
        assert_eq!(
            parse(&["create-resident", "daniele", "--locale", "it"]).unwrap(),
            Command::CreateResident {
                username: "daniele".into(),
                locale: Locale::It
            }
        );
    }

    #[test]
    fn a_password_cannot_be_passed_as_an_argument() {
        assert!(parse(&["create-resident", "daniele", "--password", "x"]).is_err());
        assert!(parse(&["create-resident", "daniele", "secret-password"]).is_err());
        assert!(parse(&["create-resident", "daniele", "--locale", "fr"]).is_err());
    }
}
