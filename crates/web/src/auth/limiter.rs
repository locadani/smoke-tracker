//! Failed sign-in limits, kept in memory (one instance; spec §8.1).
//!
//! - A username is locked for 15 minutes after 5 attempts within 15 minutes.
//! - An address is blocked for 15 minutes after 20 attempts within 15 minutes, whatever the
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

fn is_locked<K, Q>(map: &HashMap<K, Strikes>, key: &Q, now: DateTime<Utc>) -> bool
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
{
    map.get(key).is_some_and(|s| s.is_locked(now))
}

fn strike<K: Eq + Hash>(map: &mut HashMap<K, Strikes>, rule: Rule, key: K, now: DateTime<Utc>) {
    if map.len() > PRUNE_ABOVE {
        map.retain(|_, s| !s.is_finished(rule, now));
    }
    let strikes = map.entry(key).or_insert_with(|| Strikes::new(now));
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

#[derive(Debug, Default)]
struct Tables {
    usernames: HashMap<String, Strikes>,
    addresses: HashMap<IpAddr, Strikes>,
}

/// Both tables sit behind one lock, so checking and counting an attempt is a single step.
#[derive(Debug, Default)]
pub(crate) struct LoginLimiter {
    tables: Mutex<Tables>,
}

impl LoginLimiter {
    /// The username as typed, trimmed and lowercased like `Username::new`, so "Daniele " and
    /// "daniele" share one counter, and so do names that are not valid usernames at all.
    pub(crate) fn key(raw: &str) -> String {
        raw.trim()
            .to_ascii_lowercase()
            .chars()
            .take(MAX_KEY_CHARS)
            .collect()
    }

    /// Admits a sign-in attempt, or refuses it while the username or the address is locked.
    /// An admitted attempt is counted as a failure at once, before any password work, so
    /// parallel guesses cannot all slip past the check; `succeeded` takes the count back.
    pub(crate) fn begin(&self, key: &str, address: IpAddr, now: DateTime<Utc>) -> bool {
        let mut tables = self.lock();
        if is_locked(&tables.usernames, key, now) || is_locked(&tables.addresses, &address, now) {
            return false;
        }
        strike(&mut tables.usernames, PER_USERNAME, key.to_owned(), now);
        strike(&mut tables.addresses, PER_ADDRESS, address, now);
        true
    }

    /// A correct password clears the username's failures and refunds the address's strike, so
    /// legitimate sign-ins do not use up the address budget. A lock already in force is kept.
    pub(crate) fn succeeded(&self, key: &str, address: IpAddr, now: DateTime<Utc>) {
        let mut tables = self.lock();
        tables.usernames.remove(key);
        if let Some(strikes) = tables.addresses.get_mut(&address)
            && !strikes.is_locked(now)
        {
            strikes.failures = strikes.failures.saturating_sub(1);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Tables> {
        self.tables
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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

    fn nth_ip(n: usize) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(198, 51, (n / 250) as u8, (n % 250) as u8))
    }

    #[test]
    fn the_sixth_attempt_for_a_username_is_refused_for_fifteen_minutes() {
        let limiter = LoginLimiter::default();
        for _ in 0..5 {
            assert!(limiter.begin("daniele", ip(1), t0()));
        }
        assert!(!limiter.begin("daniele", ip(2), t0()));
        assert!(!limiter.begin(
            "daniele",
            ip(2),
            t0() + TimeDelta::minutes(15) - TimeDelta::seconds(1)
        ));
        assert!(limiter.begin("daniele", ip(2), t0() + TimeDelta::minutes(15)));
        assert!(limiter.begin("partner", ip(2), t0()));
    }

    #[test]
    fn attempts_older_than_the_window_are_forgotten() {
        let limiter = LoginLimiter::default();
        for _ in 0..4 {
            assert!(limiter.begin("daniele", ip(1), t0()));
        }
        let later = t0() + TimeDelta::minutes(15);
        assert!(limiter.begin("daniele", ip(1), later));
        assert!(limiter.begin("daniele", ip(1), later));
    }

    #[test]
    fn a_success_clears_the_username_count_and_refunds_the_address() {
        let limiter = LoginLimiter::default();
        for _ in 0..4 {
            assert!(limiter.begin("daniele", ip(1), t0()));
        }
        limiter.succeeded("daniele", ip(1), t0());
        for _ in 0..4 {
            assert!(limiter.begin("daniele", ip(1), t0()));
        }
        assert!(limiter.begin("daniele", ip(1), t0()));
        assert!(!limiter.begin("daniele", ip(1), t0()));

        let limiter = LoginLimiter::default();
        for n in 0..19 {
            assert!(limiter.begin(&format!("user{n}"), ip(1), t0()));
        }
        limiter.succeeded("user0", ip(1), t0());
        assert!(limiter.begin("one", ip(1), t0()));
        assert!(limiter.begin("two", ip(1), t0()));
        assert!(!limiter.begin("three", ip(1), t0()));
    }

    #[test]
    fn twenty_attempts_from_one_address_block_it_for_every_username() {
        let limiter = LoginLimiter::default();
        for n in 0..20 {
            assert!(limiter.begin(&format!("user{n}"), ip(1), t0()));
        }
        assert!(!limiter.begin("anyone", ip(1), t0()));
        assert!(limiter.begin("anyone", ip(2), t0()));
    }

    #[test]
    fn attempts_counted_before_any_outcome_cannot_exceed_the_limits() {
        let limiter = LoginLimiter::default();
        let same_username = (0..50)
            .filter(|&n| limiter.begin("daniele", nth_ip(n), t0()))
            .count();
        assert_eq!(same_username, 5);

        let same_address = (0..50)
            .filter(|n| limiter.begin(&format!("user{n}"), ip(1), t0()))
            .count();
        assert_eq!(same_address, 20);
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
            assert!(limiter.begin(&format!("user{n}"), nth_ip(n), t0()));
        }
        let later = t0() + TimeDelta::hours(1);
        assert!(limiter.begin("one-more", ip(3), later));
        let tables = limiter.lock();
        assert_eq!(tables.usernames.len(), 1);
        assert_eq!(tables.addresses.len(), 1);
    }
}
