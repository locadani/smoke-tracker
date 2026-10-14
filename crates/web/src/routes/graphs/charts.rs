//! The Chart.js configurations of the Graphs screen, as JSON, and the rows of the room × hour
//! table. Each takes series from `smoke_domain::graphs` and adds only labels and axes.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Months, NaiveDate, TimeDelta, Utc};
use serde_json::{Value, json};
use smoke_domain::graphs::{
    AreaLevels, Granularity, Period, PersonLine, RoomHours, symptom_counts,
};
use smoke_domain::records::{Pm25Reading, Room, SmellReport, Symptom};
use smoke_domain::schedule::DailySchedule;
use smoke_domain::types::{Locale, UserId};

use crate::i18n::Tr;
use crate::{labels, time};

/// The language tag `Intl.DateTimeFormat` gets, chosen so that dates come out day first as on
/// every other page (decision #95).
fn intl_locale(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "en-GB",
        Locale::It => "it-IT",
        Locale::Tr => "tr-TR",
    }
}

/// Where the time axis puts its ticks, as milliseconds since the epoch: the local midnight of
/// each day of a period counted per day, or of each month's first day in one counted per week. Fixed steps of 24 hours would
/// drift an hour off midnight after a clock change.
fn tick_values(period: Period, schedule: &DailySchedule) -> Vec<i64> {
    let days: Vec<NaiveDate> = match period.granularity() {
        Granularity::Day => period.buckets(),
        Granularity::Week => {
            let month = period
                .first_day()
                .with_day(1)
                .expect("every month has a first day");
            std::iter::successors(Some(month), |&first| Some(first + Months::new(1)))
                .take_while(|&first| first <= period.last_day())
                .filter(|&first| first >= period.first_day())
                .collect()
        }
    };
    days.into_iter()
        .map(|day| schedule.day_span(day).0.timestamp_millis())
        .collect()
}

/// Intensity over time: one line per person on the 0–10 axis, PM2.5 readings on their own axis
/// on the right (spec §9). The x axis is milliseconds since the epoch.
pub(crate) fn intensity_chart(
    tr: Tr,
    lines: &[PersonLine],
    names: &HashMap<UserId, String>,
    readings: &[Pm25Reading],
    period: Period,
    (from, to): (DateTime<Utc>, DateTime<Utc>),
    schedule: &DailySchedule,
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
    json!({
        "type": "line",
        "data": { "datasets": datasets },
        "options": {
            "scales": {
                "x": {
                    "type": "linear",
                    "min": from.timestamp_millis(),
                    "max": (to - TimeDelta::milliseconds(1)).timestamp_millis(),
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
        "time": {
            "zone": schedule.tz().name(),
            "locale": intl_locale(tr.0),
            "ticks": tick_values(period, schedule),
        },
    })
}

/// The label under each bucket: its day, or a week's Monday, as day and month.
pub(crate) fn bucket_labels(tr: Tr, buckets: &[NaiveDate]) -> Vec<String> {
    buckets
        .iter()
        .map(|&day| time::day_month(tr.0, day))
        .collect()
}

/// A bar chart of whole numbers with no legend; `across` lays the bars along the y axis, for
/// long labels such as symptom names.
pub(crate) fn count_chart(
    label: String,
    labels: Vec<String>,
    counts: Vec<usize>,
    across: bool,
) -> Value {
    let (index, value) = if across { ("y", "x") } else { ("x", "y") };
    json!({
        "type": "bar",
        "data": {
            "labels": labels,
            "datasets": [{ "label": label, "data": counts }],
        },
        "options": {
            "indexAxis": index,
            "scales": { value: { "beginAtZero": true, "ticks": { "precision": 0 } } },
            "plugins": { "legend": { "display": false } },
        },
    })
}

/// How often each symptom was reported, most frequent first, by its shown name.
pub(crate) fn symptom_chart(
    tr: Tr,
    reports: &[SmellReport],
    symptoms: &[Symptom],
) -> Option<Value> {
    let counts = symptom_counts(reports);
    if counts.is_empty() {
        return None;
    }
    let (names, counts) = counts
        .into_iter()
        .filter_map(|(id, count)| {
            let symptom = symptoms.iter().find(|s| s.id == id)?;
            Some((labels::symptom(tr, symptom), count))
        })
        .unzip();
    Some(count_chart(
        tr.t("graphs-symptoms-label"),
        names,
        counts,
        true,
    ))
}

/// Inside the home against common areas: each area's highest level per bucket, side by side.
pub(crate) fn area_chart(tr: Tr, labels: Vec<String>, levels: &[(NaiveDate, AreaLevels)]) -> Value {
    let series = |pick: fn(&AreaLevels) -> Option<u8>| -> Vec<Option<u8>> {
        levels.iter().map(|(_, l)| pick(l)).collect()
    };
    json!({
        "type": "bar",
        "data": {
            "labels": labels,
            "datasets": [
                { "label": tr.t("log-inside"), "data": series(|l| l.inside.map(|v| v.get())) },
                { "label": tr.t("log-common"), "data": series(|l| l.common.map(|v| v.get())) },
            ],
        },
        "options": {
            "scales": {
                "y": {
                    "min": 0,
                    "max": 10,
                    "title": { "display": true, "text": tr.t("rating-level") },
                },
            },
        },
    })
}

/// One cell of the heatmap: the room's average level at that hour, rounded, which also picks
/// the colour (`heat-0` to `heat-10`). `None` where nobody checked the room.
pub(crate) type HeatCell = Option<u8>;

pub(crate) struct HeatRow {
    pub(crate) room: String,
    pub(crate) cells: Vec<HeatCell>,
}

pub(crate) fn heat_rows(tr: Tr, rows: Vec<RoomHours>, rooms: &[Room]) -> Vec<HeatRow> {
    rows.into_iter()
        .filter_map(|row| {
            let room = rooms.iter().find(|r| r.id == row.room)?;
            Some(HeatRow {
                room: labels::room(tr, room),
                cells: row
                    .hours
                    .iter()
                    .map(|tally| tally.mean().map(|mean| mean.round() as u8))
                    .collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn ticks_sit_on_local_midnights_across_a_clock_change() {
        let schedule = DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap();
        let ticks = tick_values(Period::month(date("2026-10-01")), &schedule);
        assert_eq!(ticks.len(), 31);
        let hour = 3_600_000;
        assert_eq!(ticks[24] - ticks[23], 24 * hour, "24 to 25 October");
        assert_eq!(
            ticks[25] - ticks[24],
            25 * hour,
            "the night of the 25th is an hour longer"
        );
        assert_eq!(ticks[26] - ticks[25], 24 * hour);
        let year = tick_values(Period::year(date("2026-01-01")), &schedule);
        assert_eq!(year.len(), 12);
        assert_eq!(
            year[0],
            schedule.year_span(date("2026-01-01")).0.timestamp_millis()
        );
    }

    #[test]
    fn weekly_ticks_sit_on_the_first_of_each_month_inside_the_period() {
        let schedule = DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap();
        let period = Period::new(date("2026-01-15"), date("2026-06-30")).unwrap();
        let ticks = tick_values(period, &schedule);
        assert_eq!(ticks.len(), 5, "February to June");
        assert_eq!(
            ticks[0],
            schedule.day_span(date("2026-02-01")).0.timestamp_millis()
        );
    }
}
