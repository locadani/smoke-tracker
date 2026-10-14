//! The Graphs filters (look-and-feel spec §7): read from the address, so a view can be
//! bookmarked or shared, and shown again as the Filters card with a one-line summary.

use chrono::{Datelike, NaiveDate};
use smoke_domain::graphs::{DayPart, Filter, Period};
use smoke_domain::records::{Room, User};
use smoke_domain::types::{Level, RoomArea, RoomId, UserId};

use crate::forms::Pairs;
use crate::i18n::Tr;
use crate::routes::log::Choice;
use crate::routes::people::role_key;
use crate::{labels, time};

/// The quick picks of the period: (address value, translation key).
const RANGES: [(&str, &str); 4] = [
    ("month", "graphs-range-month"),
    ("7d", "graphs-range-7d"),
    ("30d", "graphs-range-30d"),
    ("year", "graphs-range-year"),
];

/// The areas: (address value, area, translation key); "all" is no area.
const AREAS: [(&str, Option<RoomArea>, &str); 3] = [
    ("all", None, "graphs-area-all"),
    ("inside", Some(RoomArea::Inside), "log-inside"),
    ("common", Some(RoomArea::Common), "log-common"),
];

/// The translation key beside `value` in a (value, key) table; empty when `value` is not in it.
fn key_of(table: &[(&str, &'static str)], value: &str) -> &'static str {
    table
        .iter()
        .find(|(candidate, _)| *candidate == value)
        .map_or("", |&(_, key)| key)
}

fn range_period(value: &str, today: NaiveDate) -> Option<Period> {
    match value {
        "month" => Some(Period::month(today)),
        "7d" => Some(Period::last_days(today, 7)),
        "30d" => Some(Period::last_days(today, 30)),
        "year" => Some(Period::year(today)),
        _ => None,
    }
}

fn day_part_value(part: DayPart) -> &'static str {
    match part {
        DayPart::Morning => "morning",
        DayPart::Afternoon => "afternoon",
        DayPart::Evening => "evening",
        DayPart::Night => "night",
    }
}

fn day_part_key(part: DayPart) -> &'static str {
    match part {
        DayPart::Morning => "graphs-time-morning",
        DayPart::Afternoon => "graphs-time-afternoon",
        DayPart::Evening => "graphs-time-evening",
        DayPart::Night => "graphs-time-night",
    }
}

/// What the address asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selection {
    /// The quick pick, or `None` for dates typed in From and To.
    pub(crate) range: Option<&'static str>,
    pub(crate) period: Period,
    pub(crate) filter: Filter,
}

/// Reads the filters from the query string. Anything missing or unusable falls back to its
/// default: this month, all rooms, everyone, any time, any level. Typed dates win over a quick
/// pick when both are valid and form a period. A person is kept only when `people` may be
/// chosen from (never for guests) and is one of them.
pub(crate) fn read(query: &Pairs, today: NaiveDate, people: &[User]) -> Selection {
    let date = |name| {
        query
            .get(name)
            .and_then(|value| value.parse::<NaiveDate>().ok())
    };
    let typed = date("from")
        .zip(date("to"))
        .and_then(|(first, last)| Period::new(first, last))
        .filter(|period| {
            [period.first_day(), period.last_day()]
                .iter()
                .all(|day| time::SUPPORTED_YEARS.contains(&day.year()))
        });
    let (range, period) = match typed {
        Some(period) => (None, period),
        None => RANGES
            .iter()
            .find(|(value, _)| query.get("range") == Some(*value))
            .and_then(|&(value, _)| range_period(value, today).map(|p| (Some(value), p)))
            .unwrap_or((Some("month"), Period::month(today))),
    };
    let area = AREAS
        .iter()
        .find(|(value, ..)| query.get("area") == Some(*value))
        .and_then(|&(_, area, _)| area);
    let person = query
        .get("by")
        .and_then(|value| value.parse().ok())
        .map(UserId)
        .filter(|id| people.iter().any(|user| user.id == *id));
    Selection {
        range,
        period,
        filter: Filter {
            rooms: query
                .ids("room")
                .unwrap_or_default()
                .into_iter()
                .map(RoomId)
                .collect(),
            area,
            person,
            day_part: DayPart::ALL
                .into_iter()
                .find(|&part| query.get("time") == Some(day_part_value(part))),
            min_level: query
                .get("min")
                .and_then(|value| value.parse().ok())
                .filter(|level| *level <= Level::MAX)
                .unwrap_or(0),
            hide_clean: query.get("clean") == Some("hide"),
        },
    }
}

/// A person the graphs can be narrowed to.
pub(crate) struct PersonOption {
    pub(crate) value: i64,
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) selected: bool,
}

/// The Filters card.
pub(crate) struct FiltersView {
    /// "This month · Bedroom · daniele · 22–06 · level 5+".
    pub(crate) summary: String,
    pub(crate) ranges: Vec<Choice<&'static str>>,
    /// The typed dates, empty while a quick pick is chosen.
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) areas: Vec<Choice<&'static str>>,
    pub(crate) rooms: Vec<Choice>,
    /// `None` for guests, who see only their own data.
    pub(crate) people: Option<Vec<PersonOption>>,
    pub(crate) times: Vec<Choice<&'static str>>,
    pub(crate) min_level: u8,
    /// What the minimum level reads as: "any" for 0, else "5+".
    pub(crate) min_label: String,
    pub(crate) hide_clean: bool,
}

/// "22–06": the hours of a part of the day.
fn hours(part: DayPart) -> String {
    let (start, end) = part.hours();
    format!("{start:02}–{end:02}")
}

fn min_label(tr: Tr, level: u8) -> String {
    if level == 0 {
        tr.t("graphs-level-any")
    } else {
        format!("{level}+")
    }
}

/// The Filters card for `selection`. `people` is `None` for guests.
pub(crate) fn view(
    tr: Tr,
    selection: &Selection,
    rooms: &[Room],
    people: Option<&[User]>,
) -> FiltersView {
    let filter = &selection.filter;
    let period = selection.period;
    let typed = selection.range.is_none();
    let date_value = |date: NaiveDate| {
        if typed {
            date.to_string()
        } else {
            String::new()
        }
    };
    let mut summary = vec![match selection.range {
        Some(range) => tr.t(key_of(&RANGES, range)),
        None => format!(
            "{}–{}",
            time::date(tr.0, period.first_day()),
            time::date(tr.0, period.last_day())
        ),
    }];
    let room_names: Vec<String> = filter
        .rooms
        .iter()
        .map(|&id| labels::room_by_id(tr, rooms, id))
        .filter(|name| !name.is_empty())
        .collect();
    let (area_value, _, area_key) = AREAS
        .iter()
        .copied()
        .find(|&(_, area, _)| area == filter.area)
        .expect("AREAS lists every area and no area");
    if !room_names.is_empty() {
        summary.push(room_names.join(", "));
    } else if filter.area.is_some() {
        summary.push(tr.t(area_key));
    }
    if let Some(person) = filter
        .person
        .and_then(|id| people?.iter().find(|u| u.id == id))
    {
        summary.push(person.username.to_string());
    }
    if let Some(part) = filter.day_part {
        summary.push(hours(part));
    }
    if filter.min_level > 0 {
        summary.push(tr.t1("graphs-level-from", "level", &filter.min_level.to_string()));
    }
    if filter.hide_clean {
        summary.push(tr.t("graphs-no-clean"));
    }
    FiltersView {
        summary: summary.join(" · "),
        ranges: RANGES
            .iter()
            .map(|&(value, key)| Choice {
                value,
                label: tr.t(key),
                checked: selection.range == Some(value),
            })
            .collect(),
        from: date_value(period.first_day()),
        to: date_value(period.last_day()),
        areas: AREAS
            .iter()
            .map(|&(value, _, key)| Choice {
                value,
                label: tr.t(key),
                checked: value == area_value,
            })
            .collect(),
        rooms: rooms
            .iter()
            .filter(|room| room.hidden_at.is_none() || filter.rooms.contains(&room.id))
            .map(|room| Choice {
                value: room.id.0,
                label: labels::room(tr, room),
                checked: filter.rooms.contains(&room.id),
            })
            .collect(),
        people: people.map(|people| {
            people
                .iter()
                .map(|user| PersonOption {
                    value: user.id.0,
                    name: user.username.to_string(),
                    role: tr.t(role_key(user.role)),
                    selected: filter.person == Some(user.id),
                })
                .collect()
        }),
        times: std::iter::once(Choice {
            value: "",
            label: tr.t("graphs-time-any"),
            checked: filter.day_part.is_none(),
        })
        .chain(DayPart::ALL.into_iter().map(|part| Choice {
            value: day_part_value(part),
            label: format!("{} {}", tr.t(day_part_key(part)), hours(part)),
            checked: filter.day_part == Some(part),
        }))
        .collect(),
        min_level: filter.min_level,
        min_label: min_label(tr, filter.min_level),
        hide_clean: filter.hide_clean,
    }
}

/// Every key this module looks up through a variable, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<&'static str> {
    RANGES
        .iter()
        .map(|&(_, key)| key)
        .chain(AREAS.iter().map(|&(_, _, key)| key))
        .chain(DayPart::ALL.into_iter().map(day_part_key))
        .collect()
}

#[cfg(test)]
mod tests {
    use smoke_domain::records::User;
    use smoke_domain::types::{Locale, Role, Username};

    use super::*;

    fn date(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn user(id: i64, name: &str) -> User {
        User {
            id: UserId(id),
            username: Username::new(name).unwrap(),
            password_hash: None,
            role: Role::Resident,
            locale: Locale::En,
            expires_at: None,
            created_at: "2026-01-01T00:00:00Z".parse().unwrap(),
        }
    }

    fn read_query(pairs: &[(&str, &str)]) -> Selection {
        read(
            &Pairs::of(pairs),
            date("2026-10-14"),
            &[user(1, "daniele"), user(2, "partner")],
        )
    }

    #[test]
    fn without_filters_the_graphs_show_this_month_and_everything() {
        let selection = read_query(&[]);
        assert_eq!(selection.range, Some("month"));
        assert_eq!(selection.period, Period::month(date("2026-10-01")));
        assert_eq!(selection.filter, Filter::default());
    }

    #[test]
    fn a_quick_pick_or_typed_dates_choose_the_period() {
        assert_eq!(
            read_query(&[("range", "7d")]).period,
            Period::last_days(date("2026-10-14"), 7)
        );
        assert_eq!(
            read_query(&[("range", "year")]).period,
            Period::year(date("2026-10-14"))
        );
        let typed = read_query(&[
            ("range", "7d"),
            ("from", "2026-01-01"),
            ("to", "2026-03-31"),
        ]);
        assert_eq!(typed.range, None, "typed dates win");
        assert_eq!(
            typed.period,
            Period::new(date("2026-01-01"), date("2026-03-31")).unwrap()
        );
        for bad in [
            &[("range", "decade")][..],
            &[("from", "2026-03-31"), ("to", "2026-01-01")][..],
            &[("from", "2020-01-01"), ("to", "2026-01-01")][..],
            &[("from", "1999-01-01"), ("to", "1999-01-02")][..],
            &[("from", "2026-01-01")][..],
            &[("from", "junk"), ("to", "2026-01-01")][..],
        ] {
            assert_eq!(
                read_query(bad).period,
                Period::month(date("2026-10-14")),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn each_filter_is_read_and_unusable_values_are_ignored() {
        let selection = read_query(&[
            ("area", "common"),
            ("room", "3"),
            ("room", "5"),
            ("by", "2"),
            ("time", "night"),
            ("min", "5"),
            ("clean", "hide"),
        ]);
        assert_eq!(
            selection.filter,
            Filter {
                rooms: vec![RoomId(3), RoomId(5)],
                area: Some(RoomArea::Common),
                person: Some(UserId(2)),
                day_part: Some(DayPart::Night),
                min_level: 5,
                hide_clean: true,
            }
        );
        let ignored = read_query(&[
            ("area", "garden"),
            ("room", "x"),
            ("by", "9"),
            ("time", "noon"),
            ("min", "11"),
            ("clean", "yes"),
        ]);
        assert_eq!(ignored.filter, Filter::default());
    }

    #[test]
    fn the_summary_names_every_filter_in_use() {
        let tr = Tr(Locale::En);
        let people = [user(1, "daniele")];
        let selection = read(
            &Pairs::of(&[
                ("by", "1"),
                ("time", "night"),
                ("min", "5"),
                ("area", "inside"),
            ]),
            date("2026-10-14"),
            &people,
        );
        let view = view(tr, &selection, &[], Some(&people));
        assert_eq!(
            view.summary,
            "This month · Inside · daniele · 22–06 · level 5+"
        );
        assert_eq!(view.from, "", "no typed dates while a quick pick is on");
        let typed = read(
            &Pairs::of(&[
                ("from", "2026-01-01"),
                ("to", "2026-01-31"),
                ("clean", "hide"),
            ]),
            date("2026-10-14"),
            &people,
        );
        let view = super::view(tr, &typed, &[], None);
        assert_eq!(view.summary, "01/01/2026–31/01/2026 · no clean checks");
        assert_eq!(
            (view.from.as_str(), view.to.as_str()),
            ("2026-01-01", "2026-01-31")
        );
        assert!(view.people.is_none());
        assert_eq!(view.min_label, "any");
    }
}
