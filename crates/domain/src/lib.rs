//! Pure domain logic for Smoke Tracker: types, business rules and the audit chain.
//!
//! This crate performs no I/O. Time is passed in explicitly or through
//! [`clock::Clock`], so every rule is deterministic and fast to test.
