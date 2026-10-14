//! The Events screens (spec §9, screen 3): a month of events grouped by day, and one event's
//! timeline of reports and PM2.5 readings with the large-gap warning, Close and Reopen.
//! Guests see only events they own or reported in, and only their own entries.

use std::collections::{BTreeMap, HashMap};

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Datelike, Months, NaiveDate, TimeDelta, Utc};
use serde::Deserialize;
use smoke_domain::events::{FUTURE_TOLERANCE, large_gap_indices};
use smoke_domain::logbook::{close_event, reopen_event};
use smoke_domain::permissions::{Action, Actor, can, can_see};
use smoke_domain::records::{Event, Pm25Reading, SmellReport, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{EventId, UserId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::header::SignedIn;
use crate::i18n::Tr;
use crate::labels;
use crate::routes::entries::read_time;
use crate::routes::{can_open, refusal};
use crate::state::AppState;
use crate::time;

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    /// `YYYY-MM`; the current month when absent or invalid.
    month: Option<String>,
}

struct EventRow {
    id: i64,
    span: String,
    reports: usize,
    max_intensity: Option<String>,
}

struct ReadingRow {
    at: String,
    value: String,
}

/// A no-smell check made while no event was running.
struct CheckRow {
    id: i64,
    at: String,
}

struct Day {
    date: String,
    events: Vec<EventRow>,
    checks: Vec<CheckRow>,
    readings: Vec<ReadingRow>,
}

#[derive(Template)]
#[template(path = "events.html")]
struct ListPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    month: String,
    previous: String,
    next: String,
    days: Vec<Day>,
}

/// `start–end`, `start–` while open; local clock times.
fn span(tr: Tr, event: &Event, tz: chrono_tz::Tz) -> String {
    let start = time::clock(event.opened_at, tz);
    match event.closed {
        Some(closed) => format!("{start}–{}", time::clock(closed.at, tz)),
        None => format!("{start}–{}", tr.t("events-in-progress")),
    }
}

/// The day's group, created on first use.
fn day(days: &mut BTreeMap<NaiveDate, Day>, tr: Tr, date: NaiveDate) -> &mut Day {
    days.entry(date).or_insert_with(|| Day {
        date: time::date(tr.0, date),
        events: Vec::new(),
        checks: Vec::new(),
        readings: Vec::new(),
    })
}

pub(crate) async fn list<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Query(query): Query<ListQuery>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    auto_close(&state.store, &state.config.schedule, now).await?;
    let schedule = &state.config.schedule;
    let tz = schedule.tz();
    let actor = user.actor();
    let first = query
        .month
        .as_deref()
        .and_then(time::parse_month)
        .unwrap_or_else(|| schedule.local_date(now));
    let (from, to) = schedule.month_span(first);
    let events = state.store.events_between(from, to).await?;
    // Reports lie within their event's span, so this window holds every report of `events`.
    let window_start = events.iter().map(|e| e.opened_at).min().unwrap_or(from);
    let window_end = events
        .iter()
        .map(|e| e.closed.map_or(now + FUTURE_TOLERANCE, |c| c.at))
        .max()
        .unwrap_or(to)
        .max(to);
    let mut by_event: HashMap<EventId, Vec<SmellReport>> = HashMap::new();
    let mut checks = Vec::new();
    for report in state
        .store
        .reports_between(window_start, window_end + TimeDelta::microseconds(1))
        .await?
    {
        match report.event_id {
            Some(event) => by_event.entry(event).or_default().push(report),
            None => checks.push(report),
        }
    }

    let mut days: BTreeMap<NaiveDate, Day> = BTreeMap::new();
    for event in events.iter().rev() {
        let reports = by_event.remove(&event.id).unwrap_or_default();
        if !can_open(&actor, event, &reports, now) {
            continue;
        }
        let visible: Vec<_> = reports
            .iter()
            .filter(|r| can_see(&actor, r.user_id, now))
            .collect();
        let local = schedule.local_date(event.opened_at);
        if local < schedule.local_date(from) {
            continue;
        }
        day(&mut days, tr, local).events.push(EventRow {
            id: event.id.0,
            span: span(tr, event, tz),
            reports: visible.len(),
            max_intensity: visible
                .iter()
                .map(|r| r.values.intensity().get())
                .max()
                .map(|max| max.to_string()),
        });
    }
    for check in checks.iter().rev() {
        let in_month = (from..to).contains(&check.observed_at);
        if in_month && can_see(&actor, check.user_id, now) {
            day(&mut days, tr, schedule.local_date(check.observed_at))
                .checks
                .push(CheckRow {
                    id: check.id.0,
                    at: time::clock(check.observed_at, tz),
                });
        }
    }
    for reading in state.store.readings_between(from, to).await?.iter().rev() {
        if reading.event_id.is_none() && can_see(&actor, reading.user_id, now) {
            day(&mut days, tr, schedule.local_date(reading.observed_at))
                .readings
                .push(ReadingRow {
                    at: time::clock(reading.observed_at, tz),
                    value: reading.value.ugm3().to_string(),
                });
        }
    }

    let month_param = |date: NaiveDate| date.format("%Y-%m").to_string();
    let first_of_month = first.with_day(1).expect("every month has a first day");
    page(
        StatusCode::OK,
        &ListPage {
            tr,
            signed_in: SignedIn::of(&user, now),
            month: first_of_month.format("%m/%Y").to_string(),
            previous: month_param(first_of_month - Months::new(1)),
            next: month_param(first_of_month + Months::new(1)),
            days: days.into_values().rev().collect(),
        },
    )
}

/// One line of an event's timeline.
struct Entry {
    at: String,
    /// `Some` for a report (its id), `None` for a PM2.5 reading.
    report: Option<i64>,
    text: String,
    /// Shown to people who see everyone's data.
    by: Option<String>,
    /// More than three hours after the previous report (spec §7.7).
    gap_before: bool,
}

#[derive(Template)]
#[template(path = "event.html")]
struct DetailPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    id: i64,
    date: String,
    span: String,
    reason: Option<&'static str>,
    owner: Option<String>,
    entries: Vec<Entry>,
    /// The close form's default end: now, as a `datetime-local` value.
    close_at: Option<String>,
    can_reopen: bool,
    error: Option<&'static str>,
}

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let detail = detail(&state, &user, tr, EventId(id), None).await?;
    page(StatusCode::OK, &detail)
}

/// The event and all its reports, or `NotFound` when it does not exist or `actor` may not open
/// it: answering the same either way hides that the record exists.
async fn openable<S: Store>(
    state: &AppState<S>,
    actor: &Actor,
    id: EventId,
    now: DateTime<Utc>,
) -> Result<(Event, Vec<SmellReport>), AppError> {
    let event = state.store.event(id).await?.ok_or(AppError::NotFound)?;
    let reports = state.store.reports_for_event(id).await?;
    if can_open(actor, &event, &reports, now) {
        Ok((event, reports))
    } else {
        Err(AppError::NotFound)
    }
}

async fn detail<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    id: EventId,
    error: Option<&'static str>,
) -> Result<DetailPage, AppError> {
    let now = state.clock.now();
    let tz = state.config.schedule.tz();
    let actor = user.actor();
    let (event, reports) = openable(state, &actor, id, now).await?;
    let sees_all = can(&actor, Action::ViewAll, now);
    let names: HashMap<UserId, String> = if sees_all {
        state
            .store
            .users()
            .await?
            .into_iter()
            .map(|u| (u.id, u.username.to_string()))
            .collect()
    } else {
        HashMap::new()
    };
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let reports: Vec<SmellReport> = reports
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let readings: Vec<Pm25Reading> = state
        .store
        .readings_for_event(id)
        .await?
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let gaps = large_gap_indices(&reports.iter().map(|r| r.observed_at).collect::<Vec<_>>());

    let mut timeline: Vec<(DateTime<Utc>, Entry)> = Vec::new();
    for (index, report) in reports.iter().enumerate() {
        let mut parts = vec![tr.t1(
            "event-intensity",
            "level",
            &report.values.intensity().get().to_string(),
        )];
        let room_names = labels::report_rooms(tr, &rooms, &report.values);
        let symptom_names: Vec<String> = symptoms
            .iter()
            .filter(|s| report.values.symptoms().contains(&s.id))
            .map(|s| labels::symptom(tr, s))
            .collect();
        parts.extend([room_names, symptom_names.join(", ")]);
        timeline.push((
            report.observed_at,
            Entry {
                at: time::clock(report.observed_at, tz),
                report: Some(report.id.0),
                text: parts
                    .into_iter()
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
                by: names.get(&report.user_id).cloned(),
                gap_before: gaps.contains(&index),
            },
        ));
    }
    for reading in &readings {
        timeline.push((
            reading.observed_at,
            Entry {
                at: time::clock(reading.observed_at, tz),
                report: None,
                text: tr.t1("event-pm25", "value", &reading.value.ugm3().to_string()),
                by: names.get(&reading.user_id).cloned(),
                gap_before: false,
            },
        ));
    }
    timeline.sort_by_key(|(at, _)| *at);

    let manages = can(
        &actor,
        Action::ManageEvent {
            owner: event.owner_id,
        },
        now,
    );
    let other_open = match state.store.open_event().await? {
        Some(open) => open.id != event.id,
        None => false,
    };
    Ok(DetailPage {
        tr,
        signed_in: SignedIn::of(user, now),
        id: id.0,
        date: time::date(tr.0, state.config.schedule.local_date(event.opened_at)),
        span: span(tr, &event, tz),
        reason: event.closed.map(|c| labels::close_reason(c.reason)),
        owner: names.get(&event.owner_id).cloned(),
        entries: timeline.into_iter().map(|(_, entry)| entry).collect(),
        close_at: (manages && event.is_open()).then(|| time::input_value(now, tz)),
        can_reopen: manages && !event.is_open() && !other_open,
        error,
    })
}

/// Closes the event by hand (spec §7.3). `closed_at` defaults to now; `return_to=log` comes
/// back to the Log screen.
pub(crate) async fn close<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    auto_close(&state.store, &state.config.schedule, now).await?;
    let id = EventId(id);
    openable(&state, &user.actor(), id, now).await?;
    let closed_at =
        read_time(&form, "closed_at", state.config.schedule.tz()).map(|at| at.unwrap_or(now));
    let result = match closed_at {
        Ok(at) => {
            let mut tx = state.store.begin(Some(user.id)).await?;
            match close_event(&mut tx, &user.actor(), id, at, now).await {
                Ok(_) => {
                    tx.commit().await?;
                    Ok(())
                }
                Err(error) => Err(refusal(error)?),
            }
        }
        Err(key) => Err(key),
    };
    match result {
        Ok(()) if form.get("return_to") == Some("log") => Ok(Redirect::to("/").into_response()),
        Ok(()) => Ok(Redirect::to(&format!("/events/{}", id.0)).into_response()),
        Err(key) => {
            let detail = detail(&state, &user, tr, id, Some(key)).await?;
            page(StatusCode::UNPROCESSABLE_ENTITY, &detail)
        }
    }
}

pub(crate) async fn reopen<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    auto_close(&state.store, &state.config.schedule, now).await?;
    let id = EventId(id);
    openable(&state, &user.actor(), id, now).await?;
    let mut tx = state.store.begin(Some(user.id)).await?;
    match reopen_event(&mut tx, &user.actor(), id, now).await {
        Ok(_) => {
            tx.commit().await?;
            Ok(Redirect::to(&format!("/events/{}", id.0)).into_response())
        }
        Err(error) => {
            drop(tx);
            let key = refusal(error)?;
            let detail = detail(&state, &user, tr, id, Some(key)).await?;
            page(StatusCode::UNPROCESSABLE_ENTITY, &detail)
        }
    }
}
