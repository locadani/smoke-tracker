//! One module per feature.

pub(crate) mod accept;
pub(crate) mod entries;
pub(crate) mod events;
pub(crate) mod guide;
pub(crate) mod lang;
pub(crate) mod lists;
pub(crate) mod log;
pub(crate) mod login;
pub(crate) mod notices;
pub(crate) mod people;
pub(crate) mod reports;
pub(crate) mod settings;

use chrono::{DateTime, Utc};
use smoke_domain::DomainError;
use smoke_domain::logbook::LogError;
use smoke_domain::people::PeopleError;
use smoke_domain::permissions::{Actor, can_see};
use smoke_domain::records::{Event, SmellReport};

use crate::error::AppError;

pub(crate) async fn not_found() -> AppError {
    AppError::NotFound
}

/// Whether `actor` may open this event: it is theirs to see, or they reported in it. Guests reach
/// no other event, so the same check decides what the Log screen may say about the open one.
pub(crate) fn can_open(
    actor: &Actor,
    event: &Event,
    reports: &[SmellReport],
    now: DateTime<Utc>,
) -> bool {
    can_see(actor, event.owner_id, now) || reports.iter().any(|r| can_see(actor, r.user_id, now))
}

/// What to tell the user when a use case refuses: the translation key of a message shown with
/// the form, or an error page.
pub(crate) fn refusal(error: LogError) -> Result<&'static str, AppError> {
    match error {
        LogError::Forbidden => Err(AppError::NotAllowed),
        LogError::NotFound => Err(AppError::NotFound),
        LogError::Stale => Ok("refused-stale"),
        LogError::AnotherEventOpen => Ok("refused-another-open"),
        LogError::OutsideEvent => Ok("refused-outside-event"),
        LogError::NoSmellCannotOpen => Ok("refused-no-smell-new-event"),
        LogError::CheckCannotSmell => Ok("refused-check-smell"),
        LogError::Invalid(DomainError::ObservedInFuture) => Ok("refused-future"),
        LogError::Invalid(DomainError::CloseBeforeLastReport) => {
            Ok("refused-close-before-last-report")
        }
        LogError::Invalid(DomainError::CloseInFuture) => Ok("refused-close-in-future"),
        LogError::Invalid(other) => Err(AppError::internal(other)),
        LogError::Store(error) => Err(error.into()),
    }
}

/// What an account use case's refusal becomes when the page has no message of its own for it.
pub(crate) fn people_refusal(error: PeopleError) -> AppError {
    match error {
        PeopleError::Forbidden => AppError::NotAllowed,
        PeopleError::NotFound => AppError::NotFound,
        other => AppError::internal(other),
    }
}

/// Every key [`refusal`] can return, for the translation tests.
#[cfg(test)]
pub(crate) const REFUSALS: [&str; 8] = [
    "refused-stale",
    "refused-another-open",
    "refused-outside-event",
    "refused-no-smell-new-event",
    "refused-check-smell",
    "refused-future",
    "refused-close-before-last-report",
    "refused-close-in-future",
];
