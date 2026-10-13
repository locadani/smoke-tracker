//! The source of "now". Production uses [`SystemClock`]; tests use [`FakeClock`].

use std::sync::Mutex;

use chrono::{DateTime, DurationRound, TimeDelta, Utc};

/// Drops sub-microsecond precision: Postgres stores no finer, so every stored instant is cut
/// here to keep SQLite and Postgres data identical.
pub fn truncate_to_micros(t: DateTime<Utc>) -> DateTime<Utc> {
    t.duration_trunc(TimeDelta::microseconds(1))
        .expect("a microsecond always divides a valid instant")
}

/// Anything that can tell the current time.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// The real wall clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        truncate_to_micros(Utc::now())
    }
}

/// A clock that only moves when told to.
#[derive(Debug)]
pub struct FakeClock {
    now: Mutex<DateTime<Utc>>,
}

impl FakeClock {
    pub fn new(start: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(truncate_to_micros(start)),
        }
    }

    pub fn set(&self, to: DateTime<Utc>) {
        *self.now.lock().expect("clock mutex poisoned") = truncate_to_micros(to);
    }

    pub fn advance(&self, by: TimeDelta) {
        let mut now = self.now.lock().expect("clock mutex poisoned");
        *now = truncate_to_micros(*now + by);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().expect("clock mutex poisoned")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn fake_clock_returns_the_time_it_was_given() {
        let clock = FakeClock::new(t("2026-01-15T10:00:00Z"));
        assert_eq!(clock.now(), t("2026-01-15T10:00:00Z"));
    }

    #[test]
    fn fake_clock_can_be_set_and_advanced() {
        let clock = FakeClock::new(t("2026-01-15T10:00:00Z"));
        clock.advance(TimeDelta::minutes(90));
        assert_eq!(clock.now(), t("2026-01-15T11:30:00Z"));
        clock.set(t("2026-03-29T01:59:00Z"));
        assert_eq!(clock.now(), t("2026-03-29T01:59:00Z"));
    }

    #[test]
    fn truncation_drops_everything_below_a_microsecond() {
        assert_eq!(
            truncate_to_micros(t("2026-01-15T20:00:00.123456789Z")),
            t("2026-01-15T20:00:00.123456Z")
        );
    }

    #[test]
    fn clocks_never_report_sub_microsecond_precision() {
        let fake = FakeClock::new(t("2026-01-15T20:00:00.123456789Z"));
        assert_eq!(fake.now(), t("2026-01-15T20:00:00.123456Z"));
        fake.set(t("2026-01-15T20:00:00.000000999Z"));
        assert_eq!(fake.now(), t("2026-01-15T20:00:00Z"));
        for _ in 0..100 {
            assert_eq!(SystemClock.now().timestamp_subsec_nanos() % 1_000, 0);
        }
    }

    #[test]
    fn clocks_are_usable_as_shared_trait_objects() {
        let fake = Arc::new(FakeClock::new(t("2026-01-15T10:00:00Z")));
        let shared: Arc<dyn Clock> = fake.clone();
        fake.advance(TimeDelta::seconds(1));
        assert_eq!(shared.now(), t("2026-01-15T10:00:01Z"));

        let system: Arc<dyn Clock> = Arc::new(SystemClock);
        assert!(system.now() > t("2026-01-01T00:00:00Z"));
    }
}
