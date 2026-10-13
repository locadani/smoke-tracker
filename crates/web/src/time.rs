//! Times as people read and type them: in the configured time zone (`APP_TIMEZONE`), never the
//! browser's, so every phone shows the same times.

use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use smoke_domain::types::Locale;
use thiserror::Error;

/// The `value` format of `<input type="datetime-local">`, to the minute.
const INPUT_FORMAT: &str = "%Y-%m-%dT%H:%M";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum TimeInputError {
    #[error("not a date and time")]
    Malformed,
    #[error("that local time does not exist: the clocks skipped it")]
    Skipped,
}

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

/// The value that pre-fills a `datetime-local` input.
pub(crate) fn input_value(at: DateTime<Utc>, tz: Tz) -> String {
    at.with_timezone(&tz).format(INPUT_FORMAT).to_string()
}

/// Reads a `datetime-local` value (seconds optional) as local time in `tz`. A time skipped by a
/// DST change is refused; a repeated one means its first occurrence.
pub(crate) fn parse_input(value: &str, tz: Tz) -> Result<DateTime<Utc>, TimeInputError> {
    let value = value.trim();
    let naive = NaiveDateTime::parse_from_str(value, INPUT_FORMAT)
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| TimeInputError::Malformed)?;
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(at) | LocalResult::Ambiguous(at, _) => Ok(at.with_timezone(&Utc)),
        LocalResult::None => Err(TimeInputError::Skipped),
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

    #[test]
    fn inputs_are_prefilled_with_local_time_to_the_minute() {
        assert_eq!(
            input_value(t("2026-01-15T18:05:59Z"), Rome),
            "2026-01-15T19:05"
        );
    }

    #[test]
    fn typed_times_are_read_as_local_time() {
        assert_eq!(
            parse_input("2026-01-15T19:05", Rome),
            Ok(t("2026-01-15T18:05:00Z"))
        );
        assert_eq!(
            parse_input(" 2026-07-01T19:05:30 ", Rome),
            Ok(t("2026-07-01T17:05:30Z"))
        );
        for bad in ["", "now", "2026-13-01T10:00", "2026-01-15 19:05"] {
            assert_eq!(
                parse_input(bad, Rome),
                Err(TimeInputError::Malformed),
                "{bad}"
            );
        }
    }

    #[test]
    fn dst_gaps_are_refused_and_repeats_take_the_first() {
        assert_eq!(
            parse_input("2026-03-29T02:30", Rome),
            Err(TimeInputError::Skipped)
        );
        assert_eq!(
            parse_input("2026-10-25T02:30", Rome),
            Ok(t("2026-10-25T00:30:00Z"))
        );
    }
}
