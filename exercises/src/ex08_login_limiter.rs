//! Exercise 8: a sign-in limiter with a `HashMap`, and the same limiter shared between threads.
//!
//! Practises: `docs/learning/03-web-and-auth.md`, section 4.
//! Mirrors: `smoke_web::auth::limiter::{LoginLimiter, Rule, PER_USERNAME}`.
//! Run: `cargo test --test ex08_login_limiter`
//!
//! The rule: after `max_failures` attempts for one key within `window` seconds, the key is
//! locked for `lockout` seconds. Every admitted attempt is counted as a failure at once, before
//! the password is checked (so ten parallel guesses cannot all slip past the check);
//! `succeeded` takes the count back by forgetting the key.
//!
//! Hint: `map.entry(key.to_owned()).or_insert_with(|| Strikes::new(now))` finds or creates the
//! entry in one lookup and gives you `&mut Strikes`. An entry whose window has passed (and which
//! is not locked) starts over. The attempt that reaches `max_failures` is still admitted; the
//! lock applies from the next one. For `SharedLimiter`, `Mutex::lock` returns a `Result`
//! because a thread that panicked while holding the lock poisons it; this limiter's state is
//! always consistent, so use `unwrap_or_else(PoisonError::into_inner)`.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub max_failures: u32,
    /// Seconds.
    pub window: u64,
    /// Seconds.
    pub lockout: u64,
}

/// 5 attempts within 15 minutes lock a username for 15 minutes.
pub const PER_USERNAME: Rule = Rule {
    max_failures: 5,
    window: 15 * 60,
    lockout: 15 * 60,
};

#[derive(Debug, Clone, Copy)]
struct Strikes {
    failures: u32,
    since: u64,
    locked_until: Option<u64>,
}

impl Strikes {
    fn new(now: u64) -> Self {
        Self {
            failures: 0,
            since: now,
            locked_until: None,
        }
    }
}

#[derive(Debug)]
pub struct Limiter {
    rule: Rule,
    strikes: HashMap<String, Strikes>,
}

impl Limiter {
    pub fn new(rule: Rule) -> Self {
        todo!()
    }

    /// Admits an attempt for `key` at second `now`, or refuses it while the key is locked.
    pub fn begin(&mut self, key: &str, now: u64) -> bool {
        todo!()
    }

    /// A correct password: forget the key's failures.
    pub fn succeeded(&mut self, key: &str) {
        todo!()
    }
}

/// The same limiter behind `&self`, so several threads (or requests) can share one.
#[derive(Debug)]
pub struct SharedLimiter {
    inner: Mutex<Limiter>,
}

impl SharedLimiter {
    pub fn new(rule: Rule) -> Self {
        todo!()
    }

    pub fn begin(&self, key: &str, now: u64) -> bool {
        todo!()
    }

    pub fn succeeded(&self, key: &str) {
        todo!()
    }

    fn lock(&self) -> MutexGuard<'_, Limiter> {
        todo!("lock, ignoring poisoning")
    }
}
