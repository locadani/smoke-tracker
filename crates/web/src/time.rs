//! Times as people read and type them: in the configured time zone (`APP_TIMEZONE`), never the
//! browser's, so every phone shows the same times.

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use smoke_domain::types::Locale;

/// A calendar date as the language writes it with digits.
pub(crate) fn date(locale: Locale, date: NaiveDate) -> String {
    let pattern = match locale {
        Locale::En | Locale::It => "%d/%m/%Y",
        Locale::Tr => "%d.%m.%Y",
    };
    date.format(pattern).to_string()
}

/// `19:05`, in the local time zone.
pub(crate) fn clock(at: DateTime<Utc>, tz: Tz) -> String {
    at.with_timezone(&tz).format("%H:%M").to_string()
}

/// `15/01/2026 19:05`, in the local time zone.
pub(crate) fn date_time(locale: Locale, at: DateTime<Utc>, tz: Tz) -> String {
    let local_date = at.with_timezone(&tz).date_naive();
    format!("{} {}", date(locale, local_date), clock(at, tz))
}

/// Just the time when `at` is on the same local day as `now`, else the date and time.
pub(crate) fn since(locale: Locale, at: DateTime<Utc>, now: DateTime<Utc>, tz: Tz) -> String {
    if at.with_timezone(&tz).date_naive() == now.with_timezone(&tz).date_naive() {
        clock(at, tz)
    } else {
        date_time(locale, at, tz)
    }
}

#[cfg(test)]
mod tests {
    use chrono_tz::Europe::Rome;

    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn times_are_shown_in_the_configured_zone_and_language() {
        let at = t("2026-01-15T18:05:00Z");
        assert_eq!(clock(at, Rome), "19:05");
        assert_eq!(date_time(Locale::En, at, Rome), "15/01/2026 19:05");
        assert_eq!(date_time(Locale::Tr, at, Rome), "15.01.2026 19:05");
        assert_eq!(
            date_time(Locale::It, t("2026-07-01T22:30:00Z"), Rome),
            "02/07/2026 00:30"
        );
    }

    #[test]
    fn today_shows_only_the_time() {
        let at = t("2026-01-15T18:05:00Z");
        assert_eq!(
            since(Locale::En, at, t("2026-01-15T22:59:00Z"), Rome),
            "19:05"
        );
        assert_eq!(
            since(Locale::En, at, t("2026-01-15T23:00:00Z"), Rome),
            "15/01/2026 19:05",
            "midnight in Rome"
        );
    }
}
