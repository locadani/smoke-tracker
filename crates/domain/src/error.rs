use thiserror::Error;

/// Every way a domain rule can reject input.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DomainError {
    #[error("intensity must be between 1 and 10, got {0}")]
    InvalidIntensity(u8),

    #[error("PM2.5 must be a number between 0 and 1000 µg/m³, got {0}")]
    InvalidPm25(f64),

    #[error("unknown {kind}: {value:?}")]
    UnknownValue { kind: &'static str, value: String },

    #[error("hour must be between 0 and 23, got {0}")]
    InvalidHour(u32),

    #[error("the observed time is in the future")]
    ObservedInFuture,

    #[error("an event cannot close before its last report")]
    CloseBeforeLastReport,

    #[error("the close time is in the future")]
    CloseInFuture,
}
