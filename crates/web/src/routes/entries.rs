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
use smoke_domain::events::{LogPrompt, can_join_backdated, classify_observed, log_prompt};
use smoke_domain::logbook::{EventChoice, LogError, SmellEntry, log_reading, log_smell};
use smoke_domain::permissions::{Action, can};
use smoke_domain::records::{Event, ReportValues, Room, Symptom, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{EventId, Intensity, Pm25, Role, RoomId, SymptomId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::i18n::Tr;
use crate::routes::log::{Draft, log_page};
use crate::routes::{notices, refusal};
use crate::state::AppState;
use crate::time::{self, TimeInputError};

/// The translation keys [`read`] can return, for the translation tests.
#[cfg(test)]
pub(crate) const FORM_ERRORS: [&str; 7] = [
    "log-error-time",
    "log-error-time-skipped",
    "log-error-pm25",
    "log-error-pm25-missing",
    "log-error-intensity",
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
        .map(|value| {
            time::parse_input(value, tz).map_err(|error| match error {
                TimeInputError::Malformed => "log-error-time",
                TimeInputError::Skipped => "log-error-time-skipped",
            })
        })
        .transpose()
}

/// Reads intensity, rooms, symptoms and note. Only the offered rooms and symptoms are accepted.
pub(crate) fn read_values(
    form: &Pairs,
    rooms: &[RoomId],
    symptoms: &[SymptomId],
) -> Result<ReportValues, &'static str> {
    let intensity = form
        .get("intensity")
        .and_then(|value| value.parse().ok())
        .and_then(|value| Intensity::new(value).ok())
        .ok_or("log-error-intensity")?;
    let room_ids: Vec<RoomId> = form
        .ids("room")
        .ok_or("log-error-form")?
        .into_iter()
        .map(RoomId)
        .collect();
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
    ReportValues::new(intensity, room_ids, symptom_ids, form.get("note"))
        .map_err(|_| "log-error-note")
}

/// Reads and validates the Log form. `Err` is the translation key of what to fix. Only rooms
/// and symptoms the form offers (not hidden) are accepted.
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
    if form.get("action") == Some("pm25") {
        let value = pm25.ok_or("log-error-pm25-missing")?;
        return Ok(Entry::Pm25Only { observed_at, value });
    }
    let offered_rooms: Vec<RoomId> = rooms
        .iter()
        .filter(|r| r.hidden_at.is_none())
        .map(|r| r.id)
        .collect();
    let offered_symptoms: Vec<SymptomId> = symptoms
        .iter()
        .filter(|s| s.hidden_at.is_none())
        .map(|s| s.id)
        .collect();
    let values = read_values(form, &offered_rooms, &offered_symptoms)?;
    Ok(Entry::Smell(SmellEntry {
        observed_at,
        values,
        pm25,
    }))
}

/// The answer to a prompt, as the buttons post it: `new`, `join-<id>` or `continue-<id>`.
fn parse_choice(value: &str) -> Option<EventChoice> {
    let id = |rest: &str| rest.parse().ok().map(EventId);
    if value == "new" {
        Some(EventChoice::New)
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

/// The question that applies now, and the auto-close notices pending for the user.
async fn current_prompt<S: Store>(
    state: &AppState<S>,
    user: &User,
    observed_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(LogPrompt, Vec<Event>), AppError> {
    let observed = classify_observed(observed_at, now).map_err(AppError::internal)?;
    let open = state.store.open_event().await?;
    let notices = notices::pending(state, user, now).await?;
    let prompt = log_prompt(
        observed,
        open.as_ref().map(Event::summary),
        notices.last().map(Event::summary),
    );
    Ok((prompt, notices))
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
            if let Err(error) = log_reading(&mut tx, &actor, observed_at, value, now).await {
                drop(tx);
                return back_to_form(&state, &user, tr, now, &form, refusal(error)?).await;
            }
            tx.commit().await?;
            return Ok(Redirect::to("/?saved=pm25").into_response());
        }
        Entry::Smell(entry) => entry,
    };

    let (prompt, notices) = current_prompt(&state, &user, entry.observed_at, now).await?;
    let asked = question_key(&prompt);
    let answer = form.get("choice").and_then(parse_choice);
    let choice = match (&asked, form.get("asked"), answer) {
        (None, _, _) => EventChoice::New,
        (Some(asked), Some(answered), Some(choice)) if asked == answered => choice,
        (Some(_), _, _) => {
            return ask(&state, &user, tr, now, &form, prompt, entry.observed_at).await;
        }
    };
    let observed_at = entry.observed_at;
    let mut tx = state.store.begin(Some(user.id)).await?;
    match log_smell(&mut tx, &actor, entry, choice, &schedule, now).await {
        Ok(_) => {}
        Err(LogError::Stale) => {
            drop(tx);
            let (fresh, _) = current_prompt(&state, &user, observed_at, now).await?;
            if fresh == LogPrompt::Direct {
                return back_to_form(&state, &user, tr, now, &form, "refused-stale").await;
            }
            return ask(&state, &user, tr, now, &form, fresh, observed_at).await;
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
    Ok(Redirect::to("/?saved=report").into_response())
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
    signed_in: Option<String>,
    title: String,
    detail: Option<String>,
    /// The form as posted, re-sent with the answer.
    hidden: Vec<(String, String)>,
    asked: String,
    answers: Vec<Answer>,
}

/// Asks the event question. Guests see only start times of events (spec §7.2, §7.5).
async fn ask<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    form: &Pairs,
    prompt: LogPrompt,
    observed_at: DateTime<Utc>,
) -> Result<Response, AppError> {
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
            let (day_start, day_end) = schedule.day_span(schedule.local_date(observed_at));
            let mut answers: Vec<Answer> = state
                .store
                .events_between(day_start, day_end)
                .await?
                .into_iter()
                .filter(|event| can_join_backdated(event, observed_at, schedule))
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
                        label.push_str(&tr.t1(
                            "prompt-ended-at",
                            "time",
                            &time::clock(closed.at, tz),
                        ));
                    }
                    Answer {
                        choice: format!("join-{}", event.id),
                        label,
                        primary: false,
                    }
                })
                .collect();
            answers.push(new_event("prompt-new-event"));
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
            signed_in: Some(user.username.to_string()),
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
    fn a_smell_needs_an_intensity_and_offered_rooms_only() {
        let Ok(Entry::Smell(entry)) = read_form(&[("intensity", "6"), ("room", "1")]) else {
            panic!("a valid smell");
        };
        assert_eq!(entry.observed_at, now());
        assert_eq!(entry.values.rooms(), [RoomId(1)]);
        assert_eq!(entry.pm25, None);
        for (pairs, key) in [
            (&[("room", "1")][..], "log-error-intensity"),
            (&[("intensity", "11")][..], "log-error-intensity"),
            (&[("intensity", "6"), ("room", "2")][..], "log-error-form"),
            (&[("intensity", "6"), ("room", "x")][..], "log-error-form"),
            (
                &[("intensity", "6"), ("symptom", "1")][..],
                "log-error-form",
            ),
            (&[("intensity", "6"), ("pm25", "-1")][..], "log-error-pm25"),
            (
                &[("intensity", "6"), ("observed_at", "2026-01-15T22:00")][..],
                "refused-future",
            ),
        ] {
            assert_eq!(read_form(pairs).err(), Some(key), "{pairs:?}");
        }
    }

    #[test]
    fn pm25_only_ignores_the_smell_fields_but_needs_a_value() {
        let Ok(Entry::Pm25Only { observed_at, value }) = read_form(&[
            ("action", "pm25"),
            ("pm25", "12,5"),
            ("observed_at", "2026-01-15T20:30"),
        ]) else {
            panic!("a valid reading");
        };
        assert_eq!(observed_at, now() - TimeDelta::minutes(30));
        assert_eq!(value.ugm3(), 12.5);
        assert_eq!(
            read_form(&[("action", "pm25")]).err(),
            Some("log-error-pm25-missing")
        );
    }

    #[test]
    fn answers_name_the_choice_and_the_event() {
        assert_eq!(parse_choice("new"), Some(EventChoice::New));
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
