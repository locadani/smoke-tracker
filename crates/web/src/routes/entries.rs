//! Saving the Log form (spec §7): read it, ask the event question if there is one, then log
//! everything in one transaction. The question travels with the answer (`asked`), so an answer
//! to a question that no longer applies — another phone opened or closed an event meanwhile —
//! gets the current question instead of being applied to the wrong event.

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use smoke_domain::events::{
    LogPrompt, can_join_backdated, classify_observed, log_prompt, no_smell_prompt,
};
use smoke_domain::logbook::{
    EventChoice, LogError, ReadingEntry, SmellEntry, log_reading, log_smell,
};
use smoke_domain::permissions::{Action, can};
use smoke_domain::records::{Event, Note, ReportValues, Room, RoomLevel, Symptom, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{EventId, Level, Pm25, Role, RoomId, SymptomId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::header::{SignedIn, Tab};
use crate::i18n::Tr;
use crate::routes::log::{Draft, log_page, offered};
use crate::routes::{notices, refusal};
use crate::state::AppState;
use crate::time::{self, TimeInputError};

/// The translation keys [`read`] can return, for the translation tests.
#[cfg(test)]
pub(crate) const FORM_ERRORS: [&str; 9] = [
    "log-error-time",
    "log-error-time-skipped",
    "log-error-pm25",
    "log-error-empty",
    "log-error-rooms",
    "log-error-level",
    "log-error-no-smell-level",
    "log-error-form",
    "log-error-note",
];

/// What the form asks to log.
enum Entry {
    Smell(SmellEntry),
    Pm25Only {
        observed_at: DateTime<Utc>,
        value: Pm25,
    },
}

/// Reads an optional `datetime-local` field; `None` when left blank.
pub(crate) fn read_time(
    form: &Pairs,
    name: &str,
    tz: Tz,
) -> Result<Option<DateTime<Utc>>, &'static str> {
    form.get(name)
        .map(|value| time::parse_input(value, tz).map_err(TimeInputError::key))
        .transpose()
}

/// The level picked for `room` (`level-<id>`), if any.
pub(crate) fn level_field(form: &Pairs, room: RoomId) -> Option<u8> {
    form.get(&format!("level-{room}"))
        .and_then(|value| value.parse().ok())
}

/// How a form's rooms and levels are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Smell {
    /// "Log" with rooms: each room with a level; all at 0 is a clean check.
    Levels,
    /// "No smell": the selected rooms at 0, or the whole home when none is selected.
    None,
    /// "Correct a mistake": each selected room with a level. No room is accepted only when the
    /// report being corrected already covered the whole home, so a correction never turns a
    /// report into a whole-home "no smell" by an unticked chip.
    Correction { whole_home: bool },
}

/// Reads the rooms with their levels, the symptoms and the note, which belongs to the log entry.
/// Only the offered rooms and symptoms are accepted.
pub(crate) fn read_values(
    form: &Pairs,
    rooms: &[RoomId],
    symptoms: &[SymptomId],
    smell: Smell,
) -> Result<(ReportValues, Option<Note>), &'static str> {
    let mut room_ids: Vec<RoomId> = form
        .ids("room")
        .ok_or("log-error-form")?
        .into_iter()
        .map(RoomId)
        .collect();
    room_ids.sort_unstable();
    room_ids.dedup();
    let symptom_ids: Vec<SymptomId> = form
        .ids("symptom")
        .ok_or("log-error-form")?
        .into_iter()
        .map(SymptomId)
        .collect();
    if !room_ids.iter().all(|id| rooms.contains(id))
        || !symptom_ids.iter().all(|id| symptoms.contains(id))
    {
        return Err("log-error-form");
    }
    let rooms_required = matches!(
        smell,
        Smell::Levels | Smell::Correction { whole_home: false }
    );
    if room_ids.is_empty() && rooms_required {
        return Err("log-error-rooms");
    }
    let rooms = room_ids
        .into_iter()
        .map(|room| {
            let picked = level_field(form, room);
            let level = match (smell, picked) {
                (Smell::None, None | Some(0)) => Some(Level::CLEAN),
                (Smell::None, Some(_)) => return Err("log-error-no-smell-level"),
                (Smell::Levels | Smell::Correction { .. }, picked) => {
                    picked.and_then(|value| Level::new(value).ok())
                }
            };
            Ok(RoomLevel {
                room,
                level: level.ok_or("log-error-level")?,
            })
        })
        .collect::<Result<_, &'static str>>()?;
    let values = ReportValues::new(rooms, symptom_ids).map_err(|_| "log-error-note")?;
    let note = Note::parse(form.get("note")).map_err(|_| "log-error-note")?;
    Ok((values, note))
}

/// Reads and validates the Log form. `Err` is the translation key of what to fix. Only rooms
/// and symptoms the form offers (not hidden) are accepted.
///
/// "No smell" has its own button. The one Log button decides from what is filled in (look-and-
/// feel spec §6.1): rooms make a smell report, or a clean check when every level is 0, with the
/// PM2.5 value saved alongside; without rooms a PM2.5 value alone is a reading, while symptoms
/// or a note still need a room. Rooms ticked without any level count as no rooms next to a
/// PM2.5 value: the Log screen ticks the rooms of the person's last report in the open event.
fn read(
    form: &Pairs,
    rooms: &[Room],
    symptoms: &[Symptom],
    tz: Tz,
    now: DateTime<Utc>,
) -> Result<Entry, &'static str> {
    let observed_at = read_time(form, "observed_at", tz)?.unwrap_or(now);
    if classify_observed(observed_at, now).is_err() {
        return Err("refused-future");
    }
    let pm25 = form
        .get("pm25")
        .map(|value| {
            value
                .replace(',', ".")
                .parse()
                .ok()
                .and_then(|value| Pm25::new(value).ok())
                .ok_or("log-error-pm25")
        })
        .transpose()?;
    let no_smell = form.get("action") == Some("no-smell");
    let picked = form.ids("room").ok_or("log-error-form")?;
    let unrated = picked
        .iter()
        .all(|&room| level_field(form, RoomId(room)).is_none());
    if unrated && !no_smell {
        let symptoms = form.ids("symptom").ok_or("log-error-form")?;
        let described = form.get("note").is_some() || !symptoms.is_empty();
        match pm25 {
            Some(value) if !described => return Ok(Entry::Pm25Only { observed_at, value }),
            _ if !picked.is_empty() => {}
            _ if described => return Err("log-error-rooms"),
            _ => return Err("log-error-empty"),
        }
    }
    let smell = if no_smell { Smell::None } else { Smell::Levels };
    let (offered_rooms, offered_symptoms) = offered(rooms, symptoms, (&[], &[]));
    let (values, note) = read_values(form, &offered_rooms, &offered_symptoms, smell)?;
    Ok(Entry::Smell(SmellEntry {
        observed_at,
        values,
        note,
        pm25,
    }))
}

/// The answer to a prompt, as the buttons post it: `new`, `join-<id>` or `continue-<id>`.
fn parse_choice(value: &str) -> Option<EventChoice> {
    let id = |rest: &str| rest.parse().ok().map(EventId);
    if value == "new" {
        Some(EventChoice::New { replacing: None })
    } else if let Some(rest) = value.strip_prefix("join-") {
        id(rest).map(EventChoice::Join)
    } else {
        value
            .strip_prefix("continue-")
            .and_then(id)
            .map(EventChoice::Continue)
    }
}

/// Names a question, so an answer can be checked against the question that applies now.
fn question_key(prompt: &LogPrompt) -> Option<String> {
    match prompt {
        LogPrompt::Direct => None,
        LogPrompt::SameSmell(open) => Some(format!("same-smell-{}", open.id)),
        LogPrompt::WhichEvent => Some("which-event".to_owned()),
        LogPrompt::AutoClosed(closed) => Some(format!("auto-closed-{}", closed.id)),
    }
}

/// The question that applies now, the auto-close notices pending for the user, and the event
/// that is open (a "new event" answer closes exactly that one).
async fn current_prompt<S: Store>(
    state: &AppState<S>,
    user: &User,
    entry: &SmellEntry,
    now: DateTime<Utc>,
) -> Result<(LogPrompt, Vec<Event>, Option<EventId>), AppError> {
    let observed = classify_observed(entry.observed_at, now).map_err(AppError::internal)?;
    let open = state.store.open_event().await?;
    let notices = notices::pending(state, user, now).await?;
    let prompt = if entry.values.is_no_smell() {
        no_smell_prompt(observed)
    } else {
        log_prompt(
            observed,
            open.as_ref().map(Event::summary),
            notices.last().map(Event::summary),
        )
    };
    Ok((prompt, notices, open.map(|e| e.id)))
}

pub(crate) async fn submit<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let schedule = state.config.schedule;
    let actor = user.actor();
    if !can(&actor, Action::LogEntry, now) {
        return Err(AppError::NotAllowed);
    }
    auto_close(&state.store, &schedule, now).await?;
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let entry = match read(&form, &rooms, &symptoms, schedule.tz(), now) {
        Ok(entry) => entry,
        Err(key) => return back_to_form(&state, &user, tr, now, &form, key).await,
    };
    let entry = match entry {
        Entry::Pm25Only { observed_at, value } => {
            let mut tx = state.store.begin(Some(user.id)).await?;
            let reading = ReadingEntry {
                observed_at,
                value,
                note: None,
            };
            if let Err(error) = log_reading(&mut tx, &actor, reading, now).await {
                drop(tx);
                return back_to_form(&state, &user, tr, now, &form, refusal(error)?).await;
            }
            tx.commit().await?;
            return Ok(Redirect::to("/?saved=pm25").into_response());
        }
        Entry::Smell(entry) => entry,
    };

    let (prompt, notices, open) = current_prompt(&state, &user, &entry, now).await?;
    let asked = question_key(&prompt);
    let answer = form.get("choice").and_then(parse_choice);
    let choice = match (&asked, form.get("asked"), answer) {
        (None, _, _) => EventChoice::New { replacing: open },
        (Some(asked), Some(answered), Some(choice)) if asked == answered => match choice {
            EventChoice::New { .. } => EventChoice::New { replacing: open },
            other => other,
        },
        (Some(_), _, _) => {
            return ask(&state, &user, tr, now, &form, prompt, &entry).await;
        }
    };
    let saved = if entry.values.is_no_smell() {
        "/?saved=no-smell"
    } else {
        "/?saved=report"
    };
    let mut tx = state.store.begin(Some(user.id)).await?;
    match log_smell(&mut tx, &actor, entry.clone(), choice, &schedule, now).await {
        Ok(_) => {}
        Err(LogError::Stale) => {
            drop(tx);
            let (fresh, ..) = current_prompt(&state, &user, &entry, now).await?;
            if fresh == LogPrompt::Direct {
                return back_to_form(&state, &user, tr, now, &form, "refused-stale").await;
            }
            return ask(&state, &user, tr, now, &form, fresh, &entry).await;
        }
        Err(error) => {
            drop(tx);
            return back_to_form(&state, &user, tr, now, &form, refusal(error)?).await;
        }
    }
    if matches!(prompt, LogPrompt::AutoClosed(_)) {
        notices::mark_seen(&mut tx, &notices).await?;
    }
    tx.commit().await?;
    Ok(Redirect::to(saved).into_response())
}

/// The Log screen again, with what was typed and what to fix.
async fn back_to_form<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    form: &Pairs,
    error: &'static str,
) -> Result<Response, AppError> {
    let draft = Draft::from_form(form);
    let log_page = log_page(state, user, tr, now, Some(draft), Some(error), None).await?;
    page(StatusCode::UNPROCESSABLE_ENTITY, &log_page)
}

struct Answer {
    choice: String,
    label: String,
    primary: bool,
}

#[derive(Template)]
#[template(path = "prompt.html")]
struct PromptPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    title: String,
    detail: Option<String>,
    /// The form as posted, re-sent with the answer.
    hidden: Vec<(String, String)>,
    asked: String,
    answers: Vec<Answer>,
}

/// The events a report observed at `observed_at` may join, as `(id, label)` pairs, skipping
/// `except`. The label gives when the event started and, unless the user is a guest, when it
/// ended: guests learn only the start of other people's events (spec §7.2, §7.5).
pub(crate) async fn joinable_events<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    observed_at: DateTime<Utc>,
    except: Option<EventId>,
) -> Result<Vec<(EventId, String)>, AppError> {
    let schedule = &state.config.schedule;
    let tz = schedule.tz();
    let (day_start, day_end) = schedule.day_span(schedule.local_date(observed_at));
    let events = state.store.events_between(day_start, day_end).await?;
    Ok(events
        .into_iter()
        .filter(|event| {
            Some(event.id) != except && can_join_backdated(event, observed_at, schedule)
        })
        .map(|event| {
            let mut label = tr.t1(
                "prompt-started-at",
                "time",
                &time::clock(event.opened_at, tz),
            );
            if user.role != Role::Guest
                && let Some(closed) = event.closed
            {
                label.push_str(", ");
                label.push_str(&tr.t1("prompt-ended-at", "time", &time::clock(closed.at, tz)));
            }
            (event.id, label)
        })
        .collect())
}

/// Asks the event question. Guests see only start times of events (spec §7.2, §7.5). A
/// no-smell report is offered "no event" instead of a new one (spec §7.1).
async fn ask<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    form: &Pairs,
    prompt: LogPrompt,
    entry: &SmellEntry,
) -> Result<Response, AppError> {
    let observed_at = entry.observed_at;
    let schedule = &state.config.schedule;
    let tz = schedule.tz();
    let answer = |choice: String, key: &str, primary: bool| Answer {
        choice,
        label: tr.t(key),
        primary,
    };
    let new_event = |key: &str| answer("new".to_owned(), key, false);
    let (title, detail, answers) = match prompt {
        LogPrompt::Direct => unreachable!("Direct needs no question"),
        LogPrompt::SameSmell(open) => (
            tr.t("prompt-same-smell"),
            Some(tr.t1(
                "prompt-ongoing",
                "time",
                &time::since(tr.0, open.opened_at, now, tz),
            )),
            vec![
                answer(format!("join-{}", open.id), "prompt-yes-update", true),
                new_event("prompt-no-new-event"),
            ],
        ),
        LogPrompt::WhichEvent => {
            let mut answers: Vec<Answer> = joinable_events(state, user, tr, observed_at, None)
                .await?
                .into_iter()
                .map(|(id, label)| Answer {
                    choice: format!("join-{id}"),
                    label,
                    primary: false,
                })
                .collect();
            answers.push(new_event(if entry.values.is_no_smell() {
                "prompt-no-event"
            } else {
                "prompt-new-event"
            }));
            (tr.t("prompt-which-event"), None, answers)
        }
        LogPrompt::AutoClosed(closed) => (
            tr.t1(
                "prompt-auto-closed",
                "when",
                &time::date_time(tr.0, closed.opened_at, tz),
            ),
            None,
            vec![
                answer(format!("continue-{}", closed.id), "prompt-continue", true),
                new_event("prompt-new-event"),
            ],
        ),
    };
    page(
        StatusCode::OK,
        &PromptPage {
            tr,
            signed_in: SignedIn::of(user, now, Tab::Log),
            title,
            detail,
            hidden: form.without(&["choice", "asked"]),
            asked: question_key(&prompt).unwrap_or_default(),
            answers,
        },
    )
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use smoke_domain::types::{ListName, RoomArea};

    use super::*;

    fn now() -> DateTime<Utc> {
        "2026-01-15T20:00:00Z".parse().unwrap()
    }

    fn rooms() -> Vec<Room> {
        let room = |id, hidden: bool| Room {
            id: RoomId(id),
            name: ListName::new("Room").unwrap(),
            i18n_key: None,
            area: RoomArea::Inside,
            hidden_at: hidden.then(now),
        };
        vec![room(1, false), room(2, true)]
    }

    fn read_form(pairs: &[(&str, &str)]) -> Result<Entry, &'static str> {
        read(
            &Pairs::of(pairs),
            &rooms(),
            &[],
            chrono_tz::Europe::Rome,
            now(),
        )
    }

    #[test]
    fn a_smell_needs_offered_rooms_each_with_a_level() {
        let Ok(Entry::Smell(entry)) = read_form(&[("room", "1"), ("level-1", "6")]) else {
            panic!("a valid smell");
        };
        assert_eq!(entry.observed_at, now());
        assert_eq!(
            entry.values.rooms(),
            [RoomLevel {
                room: RoomId(1),
                level: Level::new(6).unwrap(),
            }]
        );
        assert_eq!(entry.pm25, None);
        let Ok(Entry::Smell(clean)) =
            read_form(&[("room", "1"), ("level-1", "0"), ("level-3", "9")])
        else {
            panic!("a valid clean report");
        };
        assert!(
            clean.values.intensity().is_clean(),
            "unselected rooms are ignored"
        );
        for (pairs, key) in [
            (&[("level-1", "6")][..], "log-error-empty"),
            (&[("room", "1")][..], "log-error-level"),
            (&[("room", "1"), ("level-1", "11")][..], "log-error-level"),
            (&[("room", "2"), ("level-2", "6")][..], "log-error-form"),
            (&[("room", "x")][..], "log-error-form"),
            (
                &[("room", "1"), ("level-1", "6"), ("symptom", "1")][..],
                "log-error-form",
            ),
            (
                &[("room", "1"), ("level-1", "6"), ("pm25", "-1")][..],
                "log-error-pm25",
            ),
            (
                &[
                    ("room", "1"),
                    ("level-1", "6"),
                    ("observed_at", "2026-01-15T22:00"),
                ][..],
                "refused-future",
            ),
        ] {
            assert_eq!(read_form(pairs).err(), Some(key), "{pairs:?}");
        }
    }

    #[test]
    fn no_smell_needs_no_room_and_puts_selected_rooms_at_zero() {
        let Ok(Entry::Smell(home)) = read_form(&[("action", "no-smell")]) else {
            panic!("the whole home");
        };
        assert!(home.values.rooms().is_empty() && home.values.is_no_smell());
        let Ok(Entry::Smell(room)) = read_form(&[("action", "no-smell"), ("room", "1")]) else {
            panic!("one room at 0");
        };
        assert_eq!(room.values.level_in(RoomId(1)), Some(Level::CLEAN));
        assert_eq!(
            read_form(&[("action", "no-smell"), ("room", "1"), ("level-1", "2")]).err(),
            Some("log-error-no-smell-level")
        );
    }

    /// The Log button's decision table (look-and-feel spec §6.1), row by row.
    #[test]
    fn the_log_button_decides_from_what_is_filled_in() {
        let Ok(Entry::Smell(smell)) = read_form(&[
            ("action", "log"),
            ("room", "1"),
            ("level-1", "6"),
            ("pm25", "40"),
        ]) else {
            panic!("rooms with a level above 0: a smell report");
        };
        assert_eq!(smell.values.intensity().get(), 6);
        assert_eq!(smell.pm25.map(Pm25::ugm3), Some(40.0), "PM2.5 alongside");

        let Ok(Entry::Smell(check)) =
            read_form(&[("action", "log"), ("room", "1"), ("level-1", "0")])
        else {
            panic!("rooms all at 0: a clean check");
        };
        assert!(check.values.is_no_smell());
        assert_eq!(check.values.room_ids(), [RoomId(1)]);

        let Ok(Entry::Pm25Only { observed_at, value }) = read_form(&[
            ("action", "log"),
            ("pm25", "12,5"),
            ("observed_at", "2026-01-15T20:30"),
        ]) else {
            panic!("no rooms, only PM2.5: a reading");
        };
        assert_eq!(observed_at, now() - TimeDelta::minutes(30));
        assert_eq!(value.ugm3(), 12.5);

        for pairs in [
            &[("action", "log"), ("symptom", "1")][..],
            &[("action", "log"), ("note", "acrid")][..],
            &[("action", "log"), ("note", "acrid"), ("pm25", "12")][..],
        ] {
            assert_eq!(read_form(pairs).err(), Some("log-error-rooms"), "{pairs:?}");
        }
        assert_eq!(
            read_form(&[("action", "log"), ("note", "  ")]).err(),
            Some("log-error-empty"),
            "a blank note is no note"
        );
        assert_eq!(
            read_form(&[("action", "log")]).err(),
            Some("log-error-empty")
        );
        assert_eq!(
            read_form(&[]).err(),
            Some("log-error-empty"),
            "no action: Log"
        );
        let Ok(Entry::Pm25Only { .. }) =
            read_form(&[("action", "log"), ("room", "1"), ("pm25", "18")])
        else {
            panic!("a ticked room without a level beside PM2.5: a reading");
        };
        for pairs in [
            &[("action", "log"), ("room", "1")][..],
            &[
                ("action", "log"),
                ("room", "1"),
                ("pm25", "18"),
                ("note", "acrid"),
            ][..],
        ] {
            assert_eq!(read_form(pairs).err(), Some("log-error-level"), "{pairs:?}");
        }
    }

    #[test]
    fn answers_name_the_choice_and_the_event() {
        assert_eq!(
            parse_choice("new"),
            Some(EventChoice::New { replacing: None })
        );
        assert_eq!(
            parse_choice("join-12"),
            Some(EventChoice::Join(EventId(12)))
        );
        assert_eq!(
            parse_choice("continue-3"),
            Some(EventChoice::Continue(EventId(3)))
        );
        for bad in ["", "join-", "join-x", "continue", "skip-1"] {
            assert_eq!(parse_choice(bad), None, "{bad}");
        }
    }
}
