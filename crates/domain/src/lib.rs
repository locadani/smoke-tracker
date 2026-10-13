//! Pure domain logic for Smoke Tracker: types, business rules and the audit chain.
//!
//! This crate performs no I/O. Time is passed in explicitly or through
//! [`clock::Clock`], so every rule is deterministic and fast to test.

pub mod audit;
pub mod clock;
pub mod error;
pub mod events;
pub mod history;
pub mod logbook;
pub mod permissions;
pub mod records;
pub mod repos;
pub mod schedule;
pub mod starter;
pub mod summary;
pub mod sweep;
pub mod types;
pub mod verify;

pub use error::DomainError;
