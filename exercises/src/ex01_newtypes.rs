//! Exercise 1: validated newtypes.
//!
//! Practises: `docs/learning/01-foundation-and-domain.md`, section 3.
//! Mirrors: `smoke_domain::types::{Level, Username, NewPassword}`.
//! Run: `cargo test --test ex01_newtypes`
//!
//! Hint: the only way to build each type is its `new`, which checks the rule, so a value that
//! exists is a valid value. Keep the field private. For `Password`, count characters with
//! `chars().count()`, not bytes with `len()`, and never print the text in `Debug`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleError {
    InvalidLevel(u8),
    InvalidUsername(String),
    InvalidPasswordLength(usize),
}

/// A smell level from 0 (checked and clean) to 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Level(u8);

impl Level {
    pub const MAX: u8 = 10;
    pub const CLEAN: Level = Level(0);

    pub fn new(value: u8) -> Result<Self, RuleError> {
        todo!("accept 0..=MAX, otherwise RuleError::InvalidLevel(value)")
    }

    pub fn get(self) -> u8 {
        todo!()
    }

    pub fn is_clean(self) -> bool {
        todo!()
    }
}

impl TryFrom<u8> for Level {
    type Error = RuleError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        todo!("reuse Level::new")
    }
}

impl From<Level> for u8 {
    fn from(level: Level) -> Self {
        todo!()
    }
}

/// A login name: trimmed and lowercased, 3 to 32 characters, only a-z, 0-9, '.', '_' and '-'.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username(String);

impl Username {
    pub const MIN_LEN: usize = 3;
    pub const MAX_LEN: usize = 32;

    /// The error carries the text as it was typed, not the cleaned-up version.
    pub fn new(raw: &str) -> Result<Self, RuleError> {
        todo!()
    }

    pub fn as_str(&self) -> &str {
        todo!()
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// A password being chosen: 10 to 128 characters, kept exactly as typed.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    pub const MIN_CHARS: usize = 10;
    pub const MAX_CHARS: usize = 128;

    pub fn new(raw: String) -> Result<Self, RuleError> {
        todo!("the error carries the number of characters")
    }

    pub fn expose(&self) -> &str {
        todo!()
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("print exactly: Password(<redacted>)")
    }
}
