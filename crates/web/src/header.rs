//! What the page header shows for the signed-in person (spec §9 screen 8).

use chrono::{DateTime, Utc};
use smoke_domain::permissions::{Action, can};
use smoke_domain::records::User;

pub(crate) struct SignedIn {
    pub(crate) username: String,
    /// The menu links to People (residents).
    pub(crate) people: bool,
}

impl SignedIn {
    pub(crate) fn of(user: &User, now: DateTime<Utc>) -> Option<Self> {
        Some(Self {
            username: user.username.to_string(),
            people: can(&user.actor(), Action::ManageUsers, now),
        })
    }
}
