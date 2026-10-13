//! One module per feature.

pub(crate) mod entries;
pub(crate) mod guide;
pub(crate) mod lang;
pub(crate) mod log;
pub(crate) mod login;
pub(crate) mod notices;

use smoke_domain::DomainError;
use smoke_domain::logbook::LogError;

use crate::error::AppError;

pub(crate) async fn not_found() -> AppError {
    AppError::NotFound
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
        LogError::Invalid(DomainError::ObservedInFuture) => Ok("refused-future"),
        LogError::Invalid(DomainError::CloseBeforeLastReport) => {
            Ok("refused-close-before-last-report")
        }
        LogError::Invalid(DomainError::CloseInFuture) => Ok("refused-close-in-future"),
        LogError::Invalid(other) => Err(AppError::internal(other)),
        LogError::Store(error) => Err(error.into()),
    }
}

/// Every key [`refusal`] can return, for the translation tests.
#[cfg(test)]
pub(crate) const REFUSALS: [&str; 6] = [
    "refused-stale",
    "refused-another-open",
    "refused-outside-event",
    "refused-future",
    "refused-close-before-last-report",
    "refused-close-in-future",
];
