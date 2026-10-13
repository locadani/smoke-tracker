//! Pure domain logic for Smoke Tracker: types, business rules and the audit chain.
//!
//! This crate performs no I/O. Time is passed in explicitly or through
//! [`clock::Clock`], so every rule is deterministic and fast to test.

pub mod clock;
pub mod error;
pub mod permissions;
pub mod schedule;
pub mod types;

pub use error::DomainError;
