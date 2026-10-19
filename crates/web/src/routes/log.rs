//! The Log screen (spec §9, screen 1): the event status, the month's summary card and the form.

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::{Event, ReportView, Room, RoomLevel, Symptom, User, report_views};
use smoke_domain::repos::Store;
use smoke_domain::starter::BEDROOM_KEY;
use smoke_domain::summary::{MonthSummary, summarise};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{RoomArea, RoomId, SymptomId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::{Pairs, QueryPairs};
use crate::header::{SignedIn, Tab};
use crate::i18n::Tr;
use crate::routes::entries::level_field;
use crate::routes::guide::{GuideRow, levels};
use crate::routes::{can_open, notices};
use crate::state::AppState;
use crate::{labels, time};

/// What the form holds: blank, pre-filled, or as typed before a validation error.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Draft {
    pub(crate) rooms: Vec<RoomId>,
    /// The level picked for each room, selected or not.
    pub(crate) levels: Vec<(RoomId, u8)>,
    pub(crate) symptoms: Vec<SymptomId>,
    pub(crate) pm25: String,
    pub(crate) observed_at: String,
    pub(crate) note: String,
}

impl Draft {
    /// The form as posted, to show it again after a validation error.
    pub(crate) fn from_form(form: &Pairs) -> Self {
        let text = |name| form.get(name).unwrap_or_default().to_owned();
        Self {
            rooms: form
                .ids("room")
                .unwrap_or_default()
                .into_iter()
                .map(RoomId)
                .collect(),
            levels: form
                .names_with_prefix("level-")
                .filter_map(|id| id.parse().ok().map(RoomId))
                .filter_map(|room| level_field(form, room).map(|level| (room, level)))
                .collect(),
            symptoms: form
                .ids("symptom")
                .unwrap_or_default()
                .into_iter()
                .map(SymptomId)
                .collect(),
            pm25: text("pm25"),
            observed_at: text("observed_at"),
            note: text("note"),
        }
    }
}

/// A radio button, chip or checkbox; `value` is an id or a word of the address.
pub(crate) struct Choice<V = i64> {
    pub(crate) value: V,
    pub(crate) label: String,
    pub(crate) checked: bool,
}

/// A room's line and the level grid it opens while selected.
pub(crate) struct RoomChoice {
    pub(crate) room: Choice,
    /// "Level in Bedroom", for screen readers.
    pub(crate) legend: String,
    pub(crate) picked: Option<u8>,
}

impl RoomChoice {
    pub(crate) fn is_picked(&self, level: &u8) -> bool {
        self.picked == Some(*level)
    }
}

/// The translated headings and hints of the shared report fields, so the two forms and the
/// rating sheet read them from one place.
pub(crate) struct FieldTexts {
    pub(crate) rooms: String,
    pub(crate) how_to_rate: String,
    pub(crate) inside: String,
    pub(crate) common: String,
    pub(crate) level_hint: String,
    pub(crate) symptoms: String,
    pub(crate) note: String,
    pub(crate) optional: String,
    pub(crate) add: String,
    pub(crate) rating_title: String,
    pub(crate) close: String,
    pub(crate) rating_tip: String,
    pub(crate) full_guide: String,
}

impl FieldTexts {
    fn new(tr: Tr) -> Self {
        Self {
            rooms: tr.t("log-rooms"),
            how_to_rate: tr.t("log-how-to-rate"),
            inside: tr.t("log-inside"),
            common: tr.t("log-common"),
            level_hint: tr.t("log-level-hint"),
            symptoms: tr.t("log-symptoms"),
            note: tr.t("log-note"),
            optional: tr.t("optional"),
            add: tr.t("action-add"),
            rating_title: tr.t("rating-title"),
            close: tr.t("close"),
            rating_tip: tr.t("rating-tip"),
            full_guide: tr.t("rating-full-guide"),
        }
    }
}

/// The room, level and symptom fields shared by the Log form and the correction form
/// (`templates/report_fields.html`).
pub(crate) struct Fields {
    pub(crate) texts: FieldTexts,
    pub(crate) inside: Vec<RoomChoice>,
    pub(crate) common: Vec<RoomChoice>,
    pub(crate) symptoms: Vec<Choice>,
    /// The levels 0–10 with their names and descriptions, offered for each room.
    pub(crate) levels: Vec<GuideRow>,
}

/// The choices for `draft`. Hidden rooms and symptoms are offered only if listed in `keep`
/// (a correction keeps those its report already names).
pub(crate) fn fields(
    tr: Tr,
    rooms: &[Room],
    symptoms: &[Symptom],
    draft: &Draft,
    keep: (&[RoomId], &[SymptomId]),
) -> Fields {
    let room_choices = |area: RoomArea| {
        rooms
            .iter()
            .filter(|r| r.area == area && (r.hidden_at.is_none() || keep.0.contains(&r.id)))
            .map(|r| {
                let label = labels::room(tr, r);
                RoomChoice {
                    legend: tr.t1("log-level-in", "room", &label),
                    picked: draft
                        .levels
                        .iter()
                        .find(|(room, _)| *room == r.id)
                        .map(|&(_, level)| level),
                    room: Choice {
                        value: r.id.0,
                        label,
                        checked: draft.rooms.contains(&r.id),
                    },
                }
            })
            .collect()
    };
    Fields {
        texts: FieldTexts::new(tr),
        inside: room_choices(RoomArea::Inside),
        common: room_choices(RoomArea::Common),
        symptoms: symptoms
            .iter()
            .filter(|s| s.hidden_at.is_none() || keep.1.contains(&s.id))
            .map(|s| Choice {
                value: s.id.0,
                label: labels::symptom(tr, s),
                checked: draft.symptoms.contains(&s.id),
            })
            .collect(),
        levels: levels(tr),
    }
}

/// The banner of the open event (look-and-feel spec §5, §6.2), for those who may open it.
pub(crate) struct Banner {
    /// When the event started.
    since: String,
    /// "3 reports · highest level 6 (Bedroom)", over the reports the viewer may see.
    detail: String,
}

/// The highest level any room reached in `reports`, with that room. On a tie the room of the
/// most recent report wins (by observed time, then recorded time), and within one report the
/// later room. `None` while every level is 0.
pub(crate) fn peak<'a>(reports: impl IntoIterator<Item = &'a ReportView>) -> Option<RoomLevel> {
    reports
        .into_iter()
        .flat_map(|report| {
            let when = (report.observed_at, report.recorded_at);
            report.values.rooms().iter().map(move |&room| (room, when))
        })
        .filter(|(room, _)| !room.level.is_clean())
        .max_by_key(|&(room, when)| (room.level, when))
        .map(|(room, _)| room)
}

fn banner(
    tr: Tr,
    event: &Event,
    visible: &[&ReportView],
    rooms: &[Room],
    now: DateTime<Utc>,
    tz: Tz,
) -> Banner {
    let top_room = peak(visible.iter().copied()).map(|top| {
        let room = labels::room_by_id(tr, rooms, top.room);
        let level = top.level.get().to_string();
        tr.t2("event-banner-peak", ("level", &level), ("room", &room))
    });
    Banner {
        since: time::since(tr.0, event.opened_at, now, tz),
        detail: labels::reports_with(tr, visible.len(), top_room),
    }
}

/// The ids a submitted form may name: every visible room and symptom, plus those in `keep` (a
/// correction keeps what its report already names).
pub(crate) fn offered(
    rooms: &[Room],
    symptoms: &[Symptom],
    keep: (&[RoomId], &[SymptomId]),
) -> (Vec<RoomId>, Vec<SymptomId>) {
    let rooms = rooms
        .iter()
        .filter(|r| r.hidden_at.is_none() || keep.0.contains(&r.id))
        .map(|r| r.id)
        .collect();
    let symptoms = symptoms
        .iter()
        .filter(|s| s.hidden_at.is_none() || keep.1.contains(&s.id))
        .map(|s| s.id)
        .collect();
    (rooms, symptoms)
}

pub(crate) struct FormView {
    fields: Fields,
    pm25: String,
    observed_at: String,
    note: String,
    /// An event is open, so the main button reads "Update: situation changed".
    update: bool,
    /// A translation key explaining what to fix.
    error: Option<&'static str>,
}

impl FormView {
    /// "now" beside "Time" while no time is typed.
    fn time_hint(&self, tr: &Tr) -> String {
        if self.observed_at.is_empty() {
            tr.t("log-time-now")
        } else {
            String::new()
        }
    }
}

#[derive(Template)]
#[template(path = "log.html")]
pub(crate) struct LogPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    /// `None` when no event is open, or the viewer may not know about it.
    banner: Option<Banner>,
    /// The open event, when the user may close it.
    close_event: Option<i64>,
    /// "The event from … was auto-closed", shown while another event is open (spec §7.4).
    notice: Option<String>,
    /// A translation key confirming what was just saved.
    saved: Option<&'static str>,
    summary: MonthSummary,
    /// `None` for people who may not log (viewers, expired guests).
    form: Option<FormView>,
}

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    QueryPairs(query): QueryPairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    auto_close(&state.store, &state.config.schedule, now).await?;
    let saved = match query.get("saved") {
        Some("report") => Some("log-saved-report"),
        Some("pm25") => Some("log-saved-pm25"),
        Some("no-smell") => Some("log-saved-no-smell"),
        _ => None,
    };
    let log_page = log_page(&state, &user, tr, now, None, None, saved).await?;
    page(StatusCode::OK, &log_page)
}

/// Builds the Log screen. Without a `draft`, the rooms of the user's last report in the open
/// event are pre-selected (spec §9).
pub(crate) async fn log_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    draft: Option<Draft>,
    error: Option<&'static str>,
    saved: Option<&'static str>,
) -> Result<LogPage, AppError> {
    let schedule = &state.config.schedule;
    let actor = user.actor();
    let open = state.store.open_event().await?;
    let open_reports = match &open {
        Some(event) => report_views(&state.store.entries_for_event(event.id).await?),
        None => Vec::new(),
    };
    let shown = open
        .as_ref()
        .filter(|e| can_open(&actor, e, &open_reports, now));
    let rooms = state.store.rooms().await?;

    let (month_start, month_end) = schedule.month_span(schedule.local_date(now));
    let visible: Vec<_> = report_views(&state.store.entries_between(month_start, month_end).await?)
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let bedroom = rooms
        .iter()
        .find(|r| r.i18n_key.as_deref() == Some(BEDROOM_KEY))
        .map(|r| r.id);
    let summary = summarise(&visible, bedroom, schedule);
    let notice = match &open {
        Some(_) => notices::pending(state, user, now)
            .await?
            .pop()
            .map(|closed| {
                let when = time::date_time(tr.0, closed.opened_at, schedule.tz());
                tr.t1("prompt-auto-closed", "when", &when)
            }),
        None => None,
    };

    let form = if can(&actor, Action::LogEntry, now) {
        let draft = match draft {
            Some(draft) => draft,
            None => {
                let mut blank = Draft::default();
                if let Some(last) = open_reports.iter().rev().find(|r| r.user_id == user.id) {
                    blank.rooms = last.values.room_ids();
                }
                blank
            }
        };
        let symptoms = state.store.symptoms().await?;
        Some(FormView {
            fields: fields(tr, &rooms, &symptoms, &draft, (&[], &[])),
            pm25: draft.pm25,
            observed_at: draft.observed_at,
            note: draft.note,
            update: shown.is_some(),
            error,
        })
    } else {
        None
    };

    Ok(LogPage {
        tr,
        signed_in: SignedIn::of(user, now, Tab::Log),
        close_event: shown
            .filter(|e| can(&actor, Action::ManageEvent { owner: e.owner_id }, now))
            .map(|e| e.id.0),
        banner: shown.map(|event| {
            let visible: Vec<&ReportView> = open_reports
                .iter()
                .filter(|r| can_see(&actor, r.user_id, now))
                .collect();
            banner(tr, event, &visible, &rooms, now, schedule.tz())
        }),
        notice,
        saved,
        summary,
        form,
    })
}

#[cfg(test)]
mod tests {
    use smoke_domain::records::ReportValues;
    use smoke_domain::types::{EventId, Level, LogEntryId, ReportId, UserId};

    use super::*;

    fn report(observed: &str, recorded: &str, rooms: &[(i64, u8)]) -> ReportView {
        ReportView {
            id: ReportId(1),
            entry_id: LogEntryId(1),
            event_id: Some(EventId(1)),
            user_id: UserId(1),
            observed_at: observed.parse().unwrap(),
            recorded_at: recorded.parse().unwrap(),
            values: ReportValues::new(
                rooms
                    .iter()
                    .map(|&(room, level)| RoomLevel {
                        room: RoomId(room),
                        level: Level::new(level).unwrap(),
                    })
                    .collect(),
                vec![],
            )
            .unwrap(),
            note: None,
            deleted_at: None,
        }
    }

    fn top(reports: &[ReportView]) -> Option<(i64, u8)> {
        peak(reports).map(|room| (room.room.0, room.level.get()))
    }

    #[test]
    fn the_peak_is_the_highest_room_level_of_the_event() {
        let reports = [
            report(
                "2026-01-15T18:00:00Z",
                "2026-01-15T18:00:00Z",
                &[(1, 3), (2, 6)],
            ),
            report("2026-01-15T19:00:00Z", "2026-01-15T19:00:00Z", &[(3, 4)]),
        ];
        assert_eq!(top(&reports), Some((2, 6)));
    }

    #[test]
    fn on_a_tie_the_most_recently_reported_room_wins() {
        let reports = [
            report("2026-01-15T19:00:00Z", "2026-01-15T19:00:00Z", &[(2, 6)]),
            report("2026-01-15T18:00:00Z", "2026-01-15T20:00:00Z", &[(1, 6)]),
        ];
        assert_eq!(
            top(&reports),
            Some((2, 6)),
            "by observed time, not list order"
        );
        let same_time = [
            report("2026-01-15T19:00:00Z", "2026-01-15T19:00:00Z", &[(1, 6)]),
            report("2026-01-15T19:00:00Z", "2026-01-15T19:30:00Z", &[(3, 6)]),
        ];
        assert_eq!(top(&same_time), Some((3, 6)), "then by recorded time");
        let one_report = [report(
            "2026-01-15T19:00:00Z",
            "2026-01-15T19:00:00Z",
            &[(1, 6), (4, 6)],
        )];
        assert_eq!(top(&one_report), Some((4, 6)), "then the later room");
    }

    #[test]
    fn an_event_at_zero_or_without_rooms_has_no_peak() {
        let reports = [
            report("2026-01-15T19:00:00Z", "2026-01-15T19:00:00Z", &[(1, 0)]),
            report("2026-01-15T19:10:00Z", "2026-01-15T19:10:00Z", &[]),
        ];
        assert_eq!(top(&reports), None);
        assert_eq!(top(&[]), None);
    }
}
