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
        if value <= Self::MAX {
            Ok(Level(value))
        }
        else {
            Err(RuleError::InvalidLevel(value))
        }
    }

    pub fn get(self) -> u8 {
        self.0
    }

    pub fn is_clean(self) -> bool {
        self == Self::CLEAN
    }
}

impl TryFrom<u8> for Level {
    type Error = RuleError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Level> for u8 {
    fn from(level: Level) -> Self {
        level.get()
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
        let username = raw.to_string().trim().to_lowercase();
        if username.len() >= Self::MIN_LEN && username.len() <= Self::MAX_LEN && username.chars().all(|c| Self::is_allowed(c)) {
            Ok(Username(username))
        }
        else {
            Err(RuleError::InvalidUsername(raw.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0 // == &(self.0) Username owns the string, and self only would return the value and not the reference. In this way we borrow the inner String
    }

    fn is_allowed(c: char) -> bool {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-'
    }
}

impl fmt::Display for Username {
    // formatter is where we write the string to
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A password being chosen: 10 to 128 characters, kept exactly as typed.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    pub const MIN_CHARS: usize = 10;
    pub const MAX_CHARS: usize = 128;

    pub fn new(raw: String) -> Result<Self, RuleError> {
        let pass_length = raw.chars().count();
        if  pass_length>= Self::MIN_CHARS && pass_length <= Self::MAX_CHARS {
            Ok(Password(raw))
        }else {
            Err(RuleError::InvalidPasswordLength(pass_length))
        }
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(<redacted>)")
    }
}
