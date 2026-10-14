//! The page frame for the signed-in person (look-and-feel spec §3.3): the avatar menu in the
//! header and the bottom tab bar, with the current tab marked.

use chrono::{DateTime, Utc};
use smoke_domain::permissions::{Action, can};
use smoke_domain::records::User;

use crate::i18n::Tr;

/// The screens of the tab bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Log,
    Events,
    Graphs,
    People,
    Settings,
}

impl Tab {
    pub(crate) const ALL: [Tab; 5] = [
        Tab::Log,
        Tab::Events,
        Tab::Graphs,
        Tab::People,
        Tab::Settings,
    ];

    fn href(self) -> &'static str {
        match self {
            Self::Log => "/",
            Self::Events => "/events",
            Self::Graphs => "/graphs",
            Self::People => "/people",
            Self::Settings => "/settings",
        }
    }

    /// The symbol in `static/icons.svg`.
    pub(crate) fn icon(self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Events => "events",
            Self::Graphs => "graphs",
            Self::People => "people",
            Self::Settings => "settings",
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::Log => "nav-log",
            Self::Events => "nav-events",
            Self::Graphs => "nav-graphs",
            Self::People => "nav-people",
            Self::Settings => "nav-settings",
        }
    }
}

/// One link of the tab bar.
pub(crate) struct TabLink {
    pub(crate) href: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) label: String,
    pub(crate) current: bool,
}

pub(crate) struct SignedIn {
    pub(crate) username: String,
    /// The avatar: the username's first letter, upper case.
    pub(crate) initial: String,
    /// The tab bar links to People (residents).
    people: bool,
    /// The tab this page belongs to.
    tab: Tab,
}

impl SignedIn {
    pub(crate) fn of(user: &User, now: DateTime<Utc>, tab: Tab) -> Option<Self> {
        let username = user.username.to_string();
        Some(Self {
            initial: username
                .chars()
                .next()
                .into_iter()
                .flat_map(char::to_uppercase)
                .collect(),
            username,
            people: can(&user.actor(), Action::ManageUsers, now),
            tab,
        })
    }

    pub(crate) fn tabs(&self, tr: &Tr) -> Vec<TabLink> {
        Tab::ALL
            .into_iter()
            .filter(|&tab| tab != Tab::People || self.people)
            .map(|tab| TabLink {
                href: tab.href(),
                icon: tab.icon(),
                label: tr.t(tab.label_key()),
                current: tab == self.tab,
            })
            .collect()
    }
}

/// Every key the tab bar looks up, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<&'static str> {
    Tab::ALL.iter().map(|tab| tab.label_key()).collect()
}
