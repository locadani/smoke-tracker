//! The expiry fields of the invite and reactivation forms (spec §8.3): quick picks for guests,
//! an optional date for viewers, nothing for residents.

use chrono::NaiveDate;
use smoke_domain::people::ExpiryPick;
use smoke_domain::types::Role;

use crate::forms::Pairs;
use crate::i18n::Tr;

/// The guest quick picks: (form value, translation key).
const GUEST_PICKS: [(&str, &str); 5] = [
    ("1d", "expiry-1-day"),
    ("weekend", "expiry-weekend"),
    ("1w", "expiry-1-week"),
    ("2w", "expiry-2-weeks"),
    ("date", "expiry-date"),
];

/// A viewer may have no end, or end on a chosen day.
const VIEWER_PICKS: [(&str, &str); 2] = [("never", "expiry-never"), ("date", "expiry-date")];

/// One radio button of the expiry fields.
pub(crate) struct PickChoice {
    pub(crate) value: &'static str,
    pub(crate) label: String,
    pub(crate) checked: bool,
}

/// The expiry fields for one role, named `<prefix>expiry` and `<prefix>expiry_date` (the
/// `expiry` macro of `templates/components.html`).
pub(crate) struct ExpiryFields {
    pub(crate) role: &'static str,
    pub(crate) pick_name: String,
    pub(crate) date_name: String,
    pub(crate) picks: Vec<PickChoice>,
    pub(crate) date: String,
}

/// The fields as typed, to show them again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExpiryDraft {
    pub(crate) pick: Option<String>,
    pub(crate) date: String,
}

impl ExpiryDraft {
    pub(crate) fn from_form(form: &Pairs, prefix: &str) -> Self {
        Self {
            pick: form.get(&format!("{prefix}expiry")).map(str::to_owned),
            date: form
                .get(&format!("{prefix}expiry_date"))
                .unwrap_or_default()
                .to_owned(),
        }
    }
}

/// The fields for `role`, with `draft` picked (the role's default when nothing is); `None` for
/// residents, who never expire.
pub(crate) fn fields(
    tr: Tr,
    role: Role,
    prefix: &str,
    draft: &ExpiryDraft,
) -> Option<ExpiryFields> {
    let (picks, default): (&[(&'static str, &'static str)], &str) = match role {
        Role::Resident => return None,
        Role::Guest => (&GUEST_PICKS, "1w"),
        Role::Viewer => (&VIEWER_PICKS, "never"),
    };
    let picked = draft.pick.as_deref().unwrap_or(default);
    Some(ExpiryFields {
        role: role.as_str(),
        pick_name: format!("{prefix}expiry"),
        date_name: format!("{prefix}expiry_date"),
        picks: picks
            .iter()
            .map(|&(value, key)| PickChoice {
                value,
                label: tr.t(key),
                checked: value == picked,
            })
            .collect(),
        date: draft.date.clone(),
    })
}

/// Why the expiry fields cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpiryInputError {
    /// A guest needs one of the picks.
    Missing,
    /// "Until a day" without a valid date.
    Date,
    /// A date was typed but another pick is chosen: it would be dropped silently.
    DatePick,
}

impl ExpiryInputError {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Missing => "people-error-expiry",
            Self::Date => "people-error-date",
            Self::DatePick => "people-error-date-pick",
        }
    }
}

/// Reads the pick for `role`. Residents never expire, whatever the form says. A typed date next to
/// any pick but "date" is refused rather than ignored: the person meant it to count.
pub(crate) fn read(role: Role, draft: &ExpiryDraft) -> Result<ExpiryPick, ExpiryInputError> {
    let date = || {
        NaiveDate::parse_from_str(draft.date.trim(), "%Y-%m-%d")
            .map(ExpiryPick::Until)
            .map_err(|_| ExpiryInputError::Date)
    };
    match (role, draft.pick.as_deref()) {
        (Role::Resident, _) => Ok(ExpiryPick::Never),
        (_, pick) if pick != Some("date") && !draft.date.trim().is_empty() => {
            Err(ExpiryInputError::DatePick)
        }
        (Role::Guest, Some("1d")) => Ok(ExpiryPick::OneDay),
        (Role::Guest, Some("weekend")) => Ok(ExpiryPick::Weekend),
        (Role::Guest, Some("1w")) => Ok(ExpiryPick::OneWeek),
        (Role::Guest, Some("2w")) => Ok(ExpiryPick::TwoWeeks),
        (Role::Guest | Role::Viewer, Some("date")) => date(),
        (Role::Viewer, None | Some("never")) => Ok(ExpiryPick::Never),
        _ => Err(ExpiryInputError::Missing),
    }
}

/// Every key this module returns, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<&'static str> {
    GUEST_PICKS
        .iter()
        .chain(&VIEWER_PICKS)
        .map(|&(_, key)| key)
        .chain([
            ExpiryInputError::Missing.key(),
            ExpiryInputError::Date.key(),
            ExpiryInputError::DatePick.key(),
        ])
        .collect()
}

#[cfg(test)]
mod tests {
    use smoke_domain::types::Locale;

    use super::*;

    const EN: Tr = Tr(Locale::En);

    fn draft(pick: Option<&str>, date: &str) -> ExpiryDraft {
        ExpiryDraft {
            pick: pick.map(str::to_owned),
            date: date.to_owned(),
        }
    }

    #[test]
    fn guests_need_a_pick_and_viewers_default_to_no_end() {
        assert_eq!(
            read(Role::Guest, &draft(Some("weekend"), "")),
            Ok(ExpiryPick::Weekend)
        );
        assert_eq!(
            read(Role::Guest, &draft(None, "")),
            Err(ExpiryInputError::Missing)
        );
        assert_eq!(
            read(Role::Guest, &draft(Some("never"), "")),
            Err(ExpiryInputError::Missing)
        );
        assert_eq!(read(Role::Viewer, &draft(None, "")), Ok(ExpiryPick::Never));
        assert_eq!(
            read(Role::Viewer, &draft(Some("1w"), "")),
            Err(ExpiryInputError::Missing),
            "the quick picks are for guests"
        );
        assert_eq!(
            read(Role::Resident, &draft(Some("1d"), "")),
            Ok(ExpiryPick::Never)
        );
    }

    #[test]
    fn a_chosen_day_needs_a_valid_date() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        assert_eq!(
            read(Role::Viewer, &draft(Some("date"), " 2026-02-01 ")),
            Ok(ExpiryPick::Until(day))
        );
        for bad in ["", "2026-02-30", "01/02/2026"] {
            assert_eq!(
                read(Role::Guest, &draft(Some("date"), bad)),
                Err(ExpiryInputError::Date),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_typed_date_without_the_date_pick_is_refused() {
        for (role, pick) in [
            (Role::Viewer, None),
            (Role::Viewer, Some("never")),
            (Role::Guest, Some("1w")),
            (Role::Guest, None),
        ] {
            assert_eq!(
                read(role, &draft(pick, "2026-02-01")),
                Err(ExpiryInputError::DatePick),
                "{role:?} {pick:?}"
            );
        }
        assert_eq!(
            read(Role::Viewer, &draft(Some("never"), "  ")),
            Ok(ExpiryPick::Never)
        );
        assert_eq!(
            read(Role::Resident, &draft(Some("1d"), "2026-02-01")),
            Ok(ExpiryPick::Never)
        );
    }

    #[test]
    fn the_fields_preselect_the_default_or_the_draft() {
        let guest = fields(EN, Role::Guest, "guest_", &ExpiryDraft::default()).unwrap();
        assert_eq!(guest.pick_name, "guest_expiry");
        let checked: Vec<_> = guest.picks.iter().filter(|p| p.checked).collect();
        assert_eq!(checked.len(), 1);
        assert_eq!(checked[0].value, "1w");
        assert_eq!(checked[0].label, "in 1 week");
        let viewer = fields(EN, Role::Viewer, "", &draft(Some("date"), "2026-02-01")).unwrap();
        assert_eq!(viewer.date, "2026-02-01");
        assert!(viewer.picks.iter().any(|p| p.value == "date" && p.checked));
        assert!(fields(EN, Role::Resident, "", &ExpiryDraft::default()).is_none());
    }
}
