//! The Events screens (spec §9, screen 3): a month of events grouped by day, and one event's
//! timeline of reports and PM2.5 readings with the large-gap warning, Close and Reopen.
//! Guests see only events they own or reported in, and only their own entries.

use std::collections::{BTreeMap, HashMap};

use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Datelike, Months, NaiveDate, TimeDelta, Utc};
use smoke_domain::deleting::{AuthorCount, EntryAuthor, EventDeletion, event_deletion};
use smoke_domain::events::{FUTURE_TOLERANCE, large_gap_indices};
use smoke_domain::logbook::{close_event, reopen_event};
use smoke_domain::permissions::{Action, Actor, can, can_see};
use smoke_domain::records::{
    Event, LogEntry, ReadingView, ReportView, User, reading_views, report_views,
};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{EventId, Role, UserId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::{Pairs, QueryPairs};
use crate::header::{SignedIn, Tab};
use crate::i18n::Tr;
use crate::labels;
use crate::routes::entries::read_time;
use crate::routes::{can_open, refusal};
use crate::state::AppState;
use crate::time;

struct EventRow {
    id: i64,
    span: String,
    /// "3 reports · max 6".
    detail: String,
    /// The highest intensity among the reports, or a dash when there is none to show.
    badge: String,
}

struct ReadingRow {
    entry: i64,
    at: String,
    value: String,
}

/// A no-smell check made while no event was running.
struct CheckRow {
    entry: i64,
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
    QueryPairs(query): QueryPairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    auto_close(&state.store, &state.config.schedule, now).await?;
    let schedule = &state.config.schedule;
    let tz = schedule.tz();
    let actor = user.actor();
    let first = query
        .get("month")
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
    let mut by_event: HashMap<EventId, Vec<ReportView>> = HashMap::new();
    let mut checks = Vec::new();
    let entries = state
        .store
        .entries_between(window_start, window_end + TimeDelta::microseconds(1))
        .await?;
    for report in report_views(&entries) {
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
        let max = visible.iter().map(|r| r.values.intensity().get()).max();
        let detail = labels::reports_with(
            tr,
            visible.len(),
            max.map(|max| tr.t1("events-max-intensity", "level", &max.to_string())),
        );
        day(&mut days, tr, local).events.push(EventRow {
            id: event.id.0,
            span: span(tr, event, tz),
            detail,
            badge: max.map_or_else(|| "–".to_owned(), |max| max.to_string()),
        });
    }
    for check in checks.iter().rev() {
        let in_month = (from..to).contains(&check.observed_at);
        if in_month && can_see(&actor, check.user_id, now) {
            day(&mut days, tr, schedule.local_date(check.observed_at))
                .checks
                .push(CheckRow {
                    entry: check.entry_id.0,
                    at: time::clock(check.observed_at, tz),
                });
        }
    }
    let entries = state.store.entries_between(from, to).await?;
    for reading in reading_views(&entries).iter().rev() {
        if reading.event_id.is_none() && can_see(&actor, reading.user_id, now) {
            day(&mut days, tr, schedule.local_date(reading.observed_at))
                .readings
                .push(ReadingRow {
                    entry: reading.entry_id.0,
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
            signed_in: SignedIn::of(&user, now, Tab::Events),
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
    /// The log entry the line links to.
    entry: i64,
    /// A PM2.5 reading, as opposed to a smell report.
    reading: bool,
    text: String,
    /// Shown to people who see everyone's data.
    by: Option<String>,
    /// More than three hours after the previous report (spec §7.7).
    gap: Option<Gap>,
}

/// The large-gap warning before a report.
struct Gap {
    /// The report after the gap, when the viewer may move it to a new event (look-and-feel
    /// spec §6.4). A no-smell report cannot start an event, so it is never offered.
    movable: Option<i64>,
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
    deletion: DeletionView,
    error: Option<&'static str>,
}

/// What the page offers for deleting the event.
enum DeletionView {
    /// None of the entries is the viewer's to delete: no control.
    Hidden,
    /// The viewer may delete them all; `confirm` shows the question (`?delete`).
    Allowed { question: String, confirm: bool },
    /// Some entries are other people's: a greyed button with who logged what and why.
    Blocked { recap: String, reason: String },
}

impl DeletionView {
    /// Whether the buttons row has a delete control (the question form has its own buttons).
    fn shows_button(&self) -> bool {
        matches!(
            self,
            Self::Blocked { .. } | Self::Allowed { confirm: false, .. }
        )
    }
}

/// Who logged how many entries, the viewer first. People who see everyone's data get names;
/// everyone else sees the other authors collapsed into one count, so no name leaks.
fn recap(tr: Tr, actor: &Actor, lines: &[AuthorCount], names: &HashMap<UserId, String>) -> String {
    let total: usize = lines.iter().map(|line| line.entries).sum();
    let mut parts = Vec::new();
    let mut others = 0;
    for line in lines {
        let count = line.entries.to_string();
        if line.author == actor.id {
            parts.push(tr.t1("event-delete-you", "count", &count));
        } else if let Some(name) = names.get(&line.author) {
            parts.push(format!("{name} {count}"));
        } else {
            others += line.entries;
        }
    }
    if others > 0 {
        parts.push(tr.t1("event-delete-others", "count", &others.to_string()));
    }
    tr.t2(
        "event-delete-recap",
        ("count", &total.to_string()),
        ("people", &parts.join(" · ")),
    )
}

fn deletion_view(
    tr: Tr,
    actor: &Actor,
    entries: &[LogEntry],
    roles: &HashMap<UserId, Role>,
    names: &HashMap<UserId, String>,
    confirm: bool,
    now: DateTime<Utc>,
) -> DeletionView {
    let authors: Vec<EntryAuthor> = entries
        .iter()
        .filter_map(|entry| {
            roles.get(&entry.user_id).map(|&role| EntryAuthor {
                author: entry.user_id,
                role,
            })
        })
        .collect();
    match event_deletion(actor, &authors, now) {
        EventDeletion::Hidden => DeletionView::Hidden,
        EventDeletion::Allowed { entries } => DeletionView::Allowed {
            question: tr.n("event-delete-question", &entries),
            confirm,
        },
        EventDeletion::Blocked {
            recap: lines,
            blockers,
        } => {
            let one = match blockers.as_slice() {
                [only] => names.get(only),
                _ => None,
            };
            DeletionView::Blocked {
                recap: recap(tr, actor, &lines, names),
                reason: match one {
                    Some(name) => tr.t1("event-delete-blocked-one", "name", name),
                    None => tr.t("event-delete-blocked-many"),
                },
            }
        }
    }
}

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
    QueryPairs(query): QueryPairs,
) -> Result<Response, AppError> {
    let detail = detail(
        &state,
        &user,
        tr,
        EventId(id),
        query.contains("delete"),
        None,
    )
    .await?;
    page(StatusCode::OK, &detail)
}

/// The event and all its entries, or `NotFound` when it does not exist or `actor` may not open
/// it: answering the same either way hides that the record exists.
pub(crate) async fn openable<S: Store>(
    state: &AppState<S>,
    actor: &Actor,
    id: EventId,
    now: DateTime<Utc>,
) -> Result<(Event, Vec<LogEntry>), AppError> {
    let event = state.store.event(id).await?.ok_or(AppError::NotFound)?;
    let entries = state.store.entries_for_event(id).await?;
    if can_open(actor, &event, &report_views(&entries), now) {
        Ok((event, entries))
    } else {
        Err(AppError::NotFound)
    }
}

async fn detail<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    id: EventId,
    confirm_delete: bool,
    error: Option<&'static str>,
) -> Result<DetailPage, AppError> {
    let now = state.clock.now();
    let tz = state.config.schedule.tz();
    let actor = user.actor();
    let (event, entries) = openable(state, &actor, id, now).await?;
    let sees_all = can(&actor, Action::ViewAll, now);
    let users = state.store.users().await?;
    let roles: HashMap<UserId, Role> = users.iter().map(|u| (u.id, u.role)).collect();
    let names: HashMap<UserId, String> = if sees_all {
        users
            .iter()
            .map(|u| (u.id, u.username.to_string()))
            .collect()
    } else {
        HashMap::new()
    };
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let reports: Vec<ReportView> = report_views(&entries)
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let readings: Vec<ReadingView> = reading_views(&entries)
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
                entry: report.entry_id.0,
                reading: false,
                text: parts
                    .into_iter()
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
                by: names.get(&report.user_id).cloned(),
                gap: gaps.contains(&index).then(|| Gap {
                    movable: (!report.values.is_no_smell()
                        && can(
                            &actor,
                            Action::EditEntry {
                                owner: report.user_id,
                            },
                            now,
                        ))
                    .then_some(report.id.0),
                }),
            },
        ));
    }
    for reading in &readings {
        timeline.push((
            reading.observed_at,
            Entry {
                at: time::clock(reading.observed_at, tz),
                entry: reading.entry_id.0,
                reading: true,
                text: tr.t1("event-pm25", "value", &reading.value.ugm3().to_string()),
                by: names.get(&reading.user_id).cloned(),
                gap: None,
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
        signed_in: SignedIn::of(user, now, Tab::Events),
        id: id.0,
        date: time::date(tr.0, state.config.schedule.local_date(event.opened_at)),
        span: span(tr, &event, tz),
        reason: event.closed.map(|c| labels::close_reason(c.reason)),
        owner: names.get(&event.owner_id).cloned(),
        entries: timeline.into_iter().map(|(_, entry)| entry).collect(),
        close_at: (manages && event.is_open()).then(|| time::input_value(now, tz)),
        can_reopen: manages && !event.is_open() && !other_open,
        deletion: deletion_view(tr, &actor, &entries, &roles, &names, confirm_delete, now),
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
            let detail = detail(&state, &user, tr, id, false, Some(key)).await?;
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
            let detail = detail(&state, &user, tr, id, false, Some(key)).await?;
            page(StatusCode::UNPROCESSABLE_ENTITY, &detail)
        }
    }
}
