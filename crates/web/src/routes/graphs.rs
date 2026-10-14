//! The Graphs screen (spec §9 screen 5). Each chart is a `<canvas>` whose Chart.js
//! configuration travels as JSON in its `data-chart` attribute; `static/graphs.js` draws it, so
//! no page carries an inline script. The numbers come from `smoke_domain::graphs`, over what the
//! viewer may see: guests get graphs of their own data only (spec §4).

use std::collections::HashMap;

use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::{Value, json};
use smoke_domain::graphs::{Period, PersonLine, events_per_bucket, intensity_lines};
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::Pm25Reading;
use smoke_domain::repos::Store;
use smoke_domain::types::{Locale, UserId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::header::SignedIn;
use crate::i18n::Tr;
use crate::state::AppState;
use crate::time;

#[derive(Deserialize)]
pub(crate) struct GraphsQuery {
    /// `YYYY-MM`: that month, counted per day.
    month: Option<String>,
    /// `YYYY`: that year, counted per week. Wins over `month`.
    year: Option<String>,
}

/// The period an address asks for; the current local month when it asks for none we accept.
fn period(query: &GraphsQuery, today: NaiveDate) -> Period {
    let year = query
        .year
        .as_deref()
        .and_then(|y| y.parse::<i32>().ok())
        .filter(|y| time::SUPPORTED_YEARS.contains(y))
        .and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1));
    if let Some(first) = year {
        return Period::Year(first);
    }
    let month = query.month.as_deref().and_then(time::parse_month);
    Period::Month(month.unwrap_or(today))
}

/// The query string that asks for `period`.
fn address(period: Period) -> String {
    match period {
        Period::Month(_) => format!("month={}", period.first_day().format("%Y-%m")),
        Period::Year(_) => format!("year={}", period.first_day().year()),
    }
}

/// The other kind of period: a month's year, or a year's current month (January when the year
/// is not this one).
fn switched(period: Period, today: NaiveDate) -> Period {
    match period {
        Period::Month(date) => Period::Year(date),
        Period::Year(date) if date.year() == today.year() => Period::Month(today),
        Period::Year(date) => Period::Month(date),
    }
}

/// The language tag `Intl.DateTimeFormat` gets, chosen so that dates come out day first as on
/// every other page (decision #95).
fn intl_locale(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "en-GB",
        Locale::It => "it-IT",
        Locale::Tr => "tr-TR",
    }
}

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Intensity over time: one line per person on the 0–10 axis, PM2.5 readings on their own axis
/// on the right (spec §9). The x axis is milliseconds since the epoch.
fn intensity_chart(
    tr: Tr,
    lines: &[PersonLine],
    names: &HashMap<UserId, String>,
    readings: &[Pm25Reading],
    period: Period,
    (from, to): (DateTime<Utc>, DateTime<Utc>),
    tz: Tz,
) -> Value {
    let mut datasets: Vec<Value> = lines
        .iter()
        .map(|line| {
            let points: Vec<Value> = line
                .points
                .iter()
                .map(|(at, level)| json!({ "x": at.timestamp_millis(), "y": level.get() }))
                .collect();
            json!({
                "label": names.get(&line.user).map_or("", String::as_str),
                "data": points,
                "yAxisID": "level",
            })
        })
        .collect();
    if !readings.is_empty() {
        let points: Vec<Value> = readings
            .iter()
            .map(|r| json!({ "x": r.observed_at.timestamp_millis(), "y": r.value.ugm3() }))
            .collect();
        datasets.push(json!({
            "label": tr.t("graphs-pm25"),
            "data": points,
            "yAxisID": "pm25",
            "borderDash": [6, 4],
        }));
    }
    let step = match period {
        Period::Month(_) => json!({ "stepSize": DAY_MS }),
        Period::Year(_) => json!({}),
    };
    json!({
        "type": "line",
        "data": { "datasets": datasets },
        "options": {
            "scales": {
                "x": {
                    "type": "linear",
                    "min": from.timestamp_millis(),
                    "max": (to - TimeDelta::milliseconds(1)).timestamp_millis(),
                    "ticks": step,
                },
                "level": {
                    "type": "linear",
                    "position": "left",
                    "min": 0,
                    "max": 10,
                    "title": { "display": true, "text": tr.t("rating-level") },
                },
                "pm25": {
                    "type": "linear",
                    "position": "right",
                    "display": "auto",
                    "beginAtZero": true,
                    "grid": { "drawOnChartArea": false },
                    "title": { "display": true, "text": tr.t("graphs-pm25") },
                },
            },
        },
        "time": { "zone": tz.name(), "locale": intl_locale(tr.0) },
    })
}

/// The label under each bucket: the day of the month, or a week's Monday as day and month.
fn bucket_labels(tr: Tr, period: Period, buckets: &[NaiveDate]) -> Vec<String> {
    buckets
        .iter()
        .map(|&day| match period {
            Period::Month(_) => day.day().to_string(),
            Period::Year(_) => time::day_month(tr.0, day),
        })
        .collect()
}

/// A bar chart of whole numbers, one bar per bucket and no legend.
fn count_chart(tr: Tr, labels: Vec<String>, counts: Vec<usize>) -> Value {
    json!({
        "type": "bar",
        "data": {
            "labels": labels,
            "datasets": [{ "label": tr.t("graphs-events-label"), "data": counts }],
        },
        "options": {
            "scales": { "y": { "beginAtZero": true, "ticks": { "precision": 0 } } },
            "plugins": { "legend": { "display": false } },
        },
    })
}

#[derive(Template)]
#[template(path = "graphs.html")]
struct GraphsPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    label: String,
    previous: String,
    next: String,
    other: String,
    other_key: &'static str,
    /// Guests see only their own data; the page says so.
    own_only: bool,
    empty: bool,
    intensity: String,
    events_key: &'static str,
    events: String,
}

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Query(query): Query<GraphsQuery>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let schedule = &state.config.schedule;
    let today = schedule.local_date(now);
    let actor = user.actor();
    let period = period(&query, today);
    let span = period.span(schedule);

    let reports: Vec<_> = state
        .store
        .reports_between(span.0, span.1)
        .await?
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let readings: Vec<_> = state
        .store
        .readings_between(span.0, span.1)
        .await?
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let names: HashMap<UserId, String> = state
        .store
        .users()
        .await?
        .into_iter()
        .map(|u| (u.id, u.username.to_string()))
        .collect();

    let buckets = period.buckets();
    let labels = bucket_labels(tr, period, &buckets);
    let events = events_per_bucket(&reports, period, schedule)
        .into_iter()
        .map(|(_, count)| count)
        .collect();
    let lines = intensity_lines(&reports);
    let intensity = intensity_chart(tr, &lines, &names, &readings, period, span, schedule.tz());

    let (label, events_key, other_key) = match period {
        Period::Month(_) => (
            period.first_day().format("%m/%Y").to_string(),
            "graphs-events-day",
            "graphs-year",
        ),
        Period::Year(_) => (
            period.first_day().year().to_string(),
            "graphs-events-week",
            "graphs-month",
        ),
    };
    page(
        StatusCode::OK,
        &GraphsPage {
            tr,
            signed_in: SignedIn::of(&user, now),
            label,
            previous: address(period.previous()),
            next: address(period.next()),
            other: address(switched(period, today)),
            other_key,
            own_only: !can(&actor, Action::ViewAll, now),
            empty: reports.is_empty() && readings.is_empty(),
            intensity: intensity.to_string(),
            events_key,
            events: count_chart(tr, labels, events).to_string(),
        },
    )
}

/// Every key passed to the template through a variable, for the translation tests.
#[cfg(test)]
pub(crate) const KEYS: [&str; 4] = [
    "graphs-events-day",
    "graphs-events-week",
    "graphs-year",
    "graphs-month",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn query(month: Option<&str>, year: Option<&str>) -> GraphsQuery {
        GraphsQuery {
            month: month.map(str::to_owned),
            year: year.map(str::to_owned),
        }
    }

    #[test]
    fn the_address_picks_a_month_or_a_year_and_falls_back_to_this_month() {
        let today = date("2026-10-14");
        assert_eq!(
            period(&query(Some("2026-01"), None), today),
            Period::Month(date("2026-01-01"))
        );
        assert_eq!(
            period(&query(Some("2026-01"), Some("2025")), today),
            Period::Year(date("2025-01-01"))
        );
        for (month, year) in [(Some("2026-13"), None), (None, Some("99999")), (None, None)] {
            assert_eq!(
                period(&query(month, year), today),
                Period::Month(today),
                "{month:?} {year:?}"
            );
        }
    }

    #[test]
    fn switching_keeps_the_year_and_prefers_this_month() {
        let today = date("2026-10-14");
        assert_eq!(
            address(switched(Period::Month(date("2025-03-01")), today)),
            "year=2025"
        );
        assert_eq!(
            address(switched(Period::Year(date("2026-01-01")), today)),
            "month=2026-10"
        );
        assert_eq!(
            address(switched(Period::Year(date("2024-01-01")), today)),
            "month=2024-01"
        );
    }
}
