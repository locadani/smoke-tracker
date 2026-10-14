use std::sync::Arc;
use std::thread;

use smoke_exercises::ex08_login_limiter::{Limiter, PER_USERNAME, SharedLimiter};

const T: u64 = 1_000;

fn attempts(limiter: &mut Limiter, key: &str, count: usize, now: u64) -> Vec<bool> {
    (0..count).map(|_| limiter.begin(key, now)).collect()
}

#[test]
fn the_fifth_attempt_is_admitted_and_the_sixth_refused() {
    let mut limiter = Limiter::new(PER_USERNAME);
    assert_eq!(
        attempts(&mut limiter, "alice", 7, T),
        [true, true, true, true, true, false, false]
    );
}

#[test]
fn the_lock_lasts_exactly_the_lockout() {
    let mut limiter = Limiter::new(PER_USERNAME);
    attempts(&mut limiter, "alice", 5, T);
    assert!(!limiter.begin("alice", T + 899));
    assert!(limiter.begin("alice", T + 900));
}

#[test]
fn failures_older_than_the_window_are_forgotten() {
    let mut limiter = Limiter::new(PER_USERNAME);
    assert_eq!(attempts(&mut limiter, "alice", 4, T), [true; 4]);
    assert_eq!(
        attempts(&mut limiter, "alice", 6, T + 901),
        [true, true, true, true, true, false]
    );
}

#[test]
fn failures_inside_the_window_add_up() {
    let mut limiter = Limiter::new(PER_USERNAME);
    attempts(&mut limiter, "alice", 3, T);
    assert_eq!(
        attempts(&mut limiter, "alice", 3, T + 600),
        [true, true, false]
    );
}

#[test]
fn keys_are_counted_separately() {
    let mut limiter = Limiter::new(PER_USERNAME);
    attempts(&mut limiter, "alice", 5, T);
    assert!(!limiter.begin("alice", T));
    assert!(limiter.begin("bob", T));
}

#[test]
fn a_correct_password_clears_the_count() {
    let mut limiter = Limiter::new(PER_USERNAME);
    attempts(&mut limiter, "alice", 4, T);
    limiter.succeeded("alice");
    assert_eq!(
        attempts(&mut limiter, "alice", 6, T),
        [true, true, true, true, true, false]
    );
}

#[test]
fn succeeding_for_an_unknown_key_changes_nothing() {
    let mut limiter = Limiter::new(PER_USERNAME);
    limiter.succeeded("nobody");
    assert!(limiter.begin("nobody", T));
}

#[test]
fn a_shared_limiter_admits_exactly_five_of_many_parallel_attempts() {
    let limiter = Arc::new(SharedLimiter::new(PER_USERNAME));
    let handles: Vec<_> = (0..16)
        .map(|_| {
            let limiter = Arc::clone(&limiter);
            thread::spawn(move || limiter.begin("alice", T))
        })
        .collect();
    let admitted = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|&admitted| admitted)
        .count();
    assert_eq!(admitted, 5);
}

#[test]
fn a_shared_limiter_forgets_a_key_after_a_success() {
    let limiter = SharedLimiter::new(PER_USERNAME);
    for _ in 0..4 {
        assert!(limiter.begin("alice", T));
    }
    limiter.succeeded("alice");
    for _ in 0..5 {
        assert!(limiter.begin("alice", T));
    }
    assert!(!limiter.begin("alice", T));
}
