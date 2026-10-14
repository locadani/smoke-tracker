//! Exercise 4: a tamper-evident chain of entries.
//!
//! Practises: `docs/learning/01-foundation-and-domain.md`, section 10.
//! Mirrors: `smoke_domain::audit::{ChainedEntry, compute_hash, verify_chain}`.
//! Run: `cargo test --test ex04_audit_chain`
//!
//! Hint: every entry stores the hash of the entry before it (`prev_hash`) and its own `hash`,
//! which covers `prev_hash`, its id and its payload. Change or remove any entry and every
//! link after it stops matching. `link_hash` is given; use it for both building and checking.
//! Walk the slice once, remembering the hash the next entry must point back to (a `for` loop
//! with a variable, or `windows(2)` for the pairs). Check the link before the content, so an
//! attacker who recomputes one entry's hash is still caught at the next entry.
//!
//! The app hashes with SHA-256 (the `sha2` crate). The standard library has no SHA-256, so
//! `link_hash` uses `DefaultHasher`: fine for learning, but NOT tamper-proof.

use std::hash::{DefaultHasher, Hash, Hasher};

/// The `prev_hash` of the first entry.
pub const GENESIS: u64 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: u32,
    pub payload: String,
    pub prev_hash: u64,
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// `prev_hash` is not the hash of the entry before.
    BrokenLink { id: u32 },
    /// The content does not match the entry's own `hash`.
    HashMismatch { id: u32 },
}

/// Given: the hash of an entry's content after `prev_hash`.
pub fn link_hash(prev_hash: u64, id: u32, payload: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    prev_hash.hash(&mut hasher);
    id.hash(&mut hasher);
    payload.hash(&mut hasher);
    hasher.finish()
}

/// Adds an entry after the last one (ids count up from 1; the first entry follows `GENESIS`).
pub fn append(chain: &mut Vec<Entry>, payload: &str) {
    todo!()
}

/// A chain with one entry per payload, in order.
pub fn build_chain(payloads: &[&str]) -> Vec<Entry> {
    todo!("start empty and `append` each payload")
}

/// Checks the whole chain from `GENESIS` and reports the first problem found.
pub fn verify_chain(entries: &[Entry]) -> Result<(), ChainError> {
    todo!()
}
