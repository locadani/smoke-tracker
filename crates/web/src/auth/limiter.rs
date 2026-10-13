//! Failed sign-in limits, kept in memory (one instance; spec §8.1).
//!
//! - A username is locked for 15 minutes after 5 failures within 15 minutes.
//! - An address is blocked for 15 minutes after 20 failures within 15 minutes, whatever the
//!   usernames, so one client cannot try every account.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::net::IpAddr;
use std::sync::Mutex;

use chrono::{DateTime, TimeDelta, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rule {
    pub(crate) max_failures: u32,
    pub(crate) window: TimeDelta,
    pub(crate) lockout: TimeDelta,
}

pub(crate) const PER_USERNAME: Rule = Rule {
    max_failures: 5,
    window: TimeDelta::minutes(15),
    lockout: TimeDelta::minutes(15),
};

pub(crate) const PER_ADDRESS: Rule = Rule {
    max_failures: 20,
    window: TimeDelta::minutes(15),
    lockout: TimeDelta::minutes(15),
};

/// Above this many tracked keys, finished entries are dropped on the next failure, so random
/// usernames cannot grow memory without bound.
const PRUNE_ABOVE: usize = 1_000;
/// Usernames longer than this are cut for the key; real ones have at most 32 characters.
const MAX_KEY_CHARS: usize = 64;

#[derive(Debug, Clone, Copy)]
struct Strikes {
    failures: u32,
    since: DateTime<Utc>,
    locked_until: Option<DateTime<Utc>>,
}

impl Strikes {
    fn new(now: DateTime<Utc>) -> Self {
        Self {
            failures: 0,
            since: now,
            locked_until: None,
        }
    }

    fn is_locked(&self, now: DateTime<Utc>) -> bool {
        self.locked_until.is_some_and(|until| now < until)
    }

    fn is_finished(&self, rule: Rule, now: DateTime<Utc>) -> bool {
        !self.is_locked(now) && now >= self.since + rule.window
    }
}

#[derive(Debug)]
struct Counter<K> {
    rule: Rule,
    entries: Mutex<HashMap<K, Strikes>>,
}

impl<K: Eq + Hash> Counter<K> {
    fn new(rule: Rule) -> Self {
        Self {
            rule,
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn is_locked<Q>(&self, key: &Q, now: DateTime<Utc>) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.lock().get(key).is_some_and(|s| s.is_locked(now))
    }

    fn strike(&self, key: K, now: DateTime<Utc>) {
        let rule = self.rule;
        let mut entries = self.lock();
        if entries.len() > PRUNE_ABOVE {
            entries.retain(|_, s| !s.is_finished(rule, now));
        }
        let strikes = entries.entry(key).or_insert_with(|| Strikes::new(now));
        if strikes.is_finished(rule, now) {
            *strikes = Strikes::new(now);
        }
        strikes.failures += 1;
        if strikes.failures >= rule.max_failures {
            *strikes = Strikes {
                locked_until: Some(now + rule.lockout),
                ..Strikes::new(now)
            };
        }
    }

    fn forget<Q>(&self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.lock().remove(key);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<K, Strikes>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[derive(Debug)]
pub(crate) struct LoginLimiter {
    usernames: Counter<String>,
    addresses: Counter<IpAddr>,
}

impl Default for LoginLimiter {
    fn default() -> Self {
        Self {
            usernames: Counter::new(PER_USERNAME),
            addresses: Counter::new(PER_ADDRESS),
        }
    }
}

impl LoginLimiter {
    /// The username as typed, trimmed and lowercased like `Username::new`, so "Daniele " and
    /// "daniele" share one counter, and so do names that are not valid usernames at all.
    pub(crate) fn key(raw: &str) -> String {
        raw.trim()
            .to_lowercase()
            .chars()
            .take(MAX_KEY_CHARS)
            .collect()
    }

    pub(crate) fn is_blocked(&self, key: &str, address: IpAddr, now: DateTime<Utc>) -> bool {
        self.usernames.is_locked(key, now) || self.addresses.is_locked(&address, now)
    }

    pub(crate) fn record_failure(&self, key: String, address: IpAddr, now: DateTime<Utc>) {
        self.usernames.strike(key, now);
        self.addresses.strike(address, now);
    }

    /// A successful sign-in clears the username's failures; the address keeps its count.
    pub(crate) fn record_success(&self, key: &str) {
        self.usernames.forget(key);
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    fn t0() -> DateTime<Utc> {
        "2026-01-15T20:00:00Z".parse().unwrap()
    }

    fn ip(last: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(192, 0, 2, last))
    }

    #[test]
    fn the_fifth_failure_locks_the_username_for_fifteen_minutes() {
        let limiter = LoginLimiter::default();
        for _ in 0..4 {
            limiter.record_failure("daniele".into(), ip(1), t0());
        }
        assert!(!limiter.is_blocked("daniele", ip(2), t0()));
        limiter.record_failure("daniele".into(), ip(1), t0());
        assert!(limiter.is_blocked("daniele", ip(2), t0()));
        assert!(limiter.is_blocked(
            "daniele",
            ip(2),
            t0() + TimeDelta::minutes(15) - TimeDelta::seconds(1)
        ));
        assert!(!limiter.is_blocked("daniele", ip(2), t0() + TimeDelta::minutes(15)));
        assert!(!limiter.is_blocked("partner", ip(2), t0()));
    }

    #[test]
    fn failures_older_than_the_window_are_forgotten() {
        let limiter = LoginLimiter::default();
        for _ in 0..4 {
            limiter.record_failure("daniele".into(), ip(1), t0());
        }
        let later = t0() + TimeDelta::minutes(15);
        limiter.record_failure("daniele".into(), ip(1), later);
        assert!(!limiter.is_blocked("daniele", ip(1), later));
    }

    #[test]
    fn a_success_clears_the_username_count() {
        let limiter = LoginLimiter::default();
        for _ in 0..4 {
            limiter.record_failure("daniele".into(), ip(1), t0());
        }
        limiter.record_success("daniele");
        limiter.record_failure("daniele".into(), ip(1), t0());
        assert!(!limiter.is_blocked("daniele", ip(1), t0()));
    }

    #[test]
    fn twenty_failures_from_one_address_block_it_for_every_username() {
        let limiter = LoginLimiter::default();
        for n in 0..19 {
            limiter.record_failure(format!("user{n}"), ip(1), t0());
        }
        assert!(!limiter.is_blocked("anyone", ip(1), t0()));
        limiter.record_failure("user19".into(), ip(1), t0());
        assert!(limiter.is_blocked("anyone", ip(1), t0()));
        assert!(!limiter.is_blocked("anyone", ip(2), t0()));
    }

    #[test]
    fn keys_ignore_case_and_surrounding_spaces() {
        assert_eq!(LoginLimiter::key("  Daniele "), "daniele");
        assert_eq!(LoginLimiter::key("DANIELE"), LoginLimiter::key("daniele"));
        assert_eq!(LoginLimiter::key(&"x".repeat(500)).len(), MAX_KEY_CHARS);
    }

    #[test]
    fn finished_entries_are_pruned_once_there_are_many() {
        let limiter = LoginLimiter::default();
        for n in 0..=PRUNE_ABOVE {
            limiter.record_failure(format!("user{n}"), ip(1), t0());
        }
        let later = t0() + TimeDelta::hours(1);
        limiter.record_failure("one-more".into(), ip(3), later);
        assert_eq!(limiter.usernames.lock().len(), 1);
    }
}
