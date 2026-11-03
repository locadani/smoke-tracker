//! Who may delete the entries of an event, and what the confirmation screen has to say about it.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::permissions::{Action, Actor, can};
use crate::types::{Role, UserId};

/// Who logged one live entry of an event, and the role they hold today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryAuthor {
    pub author: UserId,
    pub role: Role,
}

/// How many live entries one author has in an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorCount {
    pub author: UserId,
    pub entries: usize,
}

/// What the actor may do with the live entries of an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventDeletion {
    /// The actor may delete none of the entries (or there are none): no delete control is shown.
    Hidden,
    /// The actor may delete every one of the `entries`.
    Allowed { entries: usize },
    /// The actor may delete some entries but not all. `recap` lists every author (the actor
    /// first, then by entry count descending, then by id); `blockers` are the authors with an
    /// entry the actor may not delete, in the same order.
    Blocked {
        recap: Vec<AuthorCount>,
        blockers: Vec<UserId>,
    },
}

/// Decides whether `actor` may delete a whole event holding `entries`, applying
/// [`Action::DeleteEntry`] to each one.
pub fn event_deletion(actor: &Actor, entries: &[EntryAuthor], now: DateTime<Utc>) -> EventDeletion {
    let deletable = entries
        .iter()
        .filter(|entry| may_delete(actor, entry, now))
        .count();
    if deletable == 0 {
        return EventDeletion::Hidden;
    }
    if deletable == entries.len() {
        return EventDeletion::Allowed { entries: deletable };
    }

    let mut per_author: BTreeMap<UserId, usize> = BTreeMap::new();
    for entry in entries {
        *per_author.entry(entry.author).or_default() += 1;
    }
    let mut recap: Vec<AuthorCount> = per_author
        .into_iter()
        .map(|(author, entries)| AuthorCount { author, entries })
        .collect();
    recap.sort_by_key(|line| (line.author != actor.id, Reverse(line.entries), line.author));

    let blockers = recap
        .iter()
        .map(|line| line.author)
        .filter(|author| {
            entries
                .iter()
                .any(|entry| entry.author == *author && !may_delete(actor, entry, now))
        })
        .collect();
    EventDeletion::Blocked { recap, blockers }
}

fn may_delete(actor: &Actor, entry: &EntryAuthor, now: DateTime<Utc>) -> bool {
    let action = Action::DeleteEntry {
        author: entry.author,
        author_role: entry.role,
    };
    can(actor, action, now)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeDelta, Utc};

    use super::*;
    use crate::permissions::Actor;
    use crate::types::{Role, UserId};

    const ME: UserId = UserId(1);
    const DUYGU: UserId = UserId(2);
    const MARCO: UserId = UserId(3);

    fn now() -> DateTime<Utc> {
        "2026-06-01T12:00:00Z".parse().unwrap()
    }

    fn actor(role: Role, expires_at: Option<DateTime<Utc>>) -> Actor {
        Actor {
            id: ME,
            role,
            expires_at,
        }
    }

    fn resident() -> Actor {
        actor(Role::Resident, None)
    }

    fn by(author: UserId, role: Role, n: usize) -> Vec<EntryAuthor> {
        vec![EntryAuthor { author, role }; n]
    }

    fn count(author: UserId, entries: usize) -> AuthorCount {
        AuthorCount { author, entries }
    }

    #[test]
    fn only_my_entries_is_allowed() {
        let entries = by(ME, Role::Resident, 3);
        assert_eq!(
            event_deletion(&resident(), &entries, now()),
            EventDeletion::Allowed { entries: 3 }
        );
    }

    #[test]
    fn a_resident_may_delete_a_guests_entries() {
        let mut entries = by(ME, Role::Resident, 2);
        entries.extend(by(MARCO, Role::Guest, 1));
        assert_eq!(
            event_deletion(&resident(), &entries, now()),
            EventDeletion::Allowed { entries: 3 }
        );
    }

    #[test]
    fn another_resident_blocks() {
        let mut entries = by(DUYGU, Role::Resident, 1);
        entries.extend(by(ME, Role::Resident, 3));
        assert_eq!(
            event_deletion(&resident(), &entries, now()),
            EventDeletion::Blocked {
                recap: vec![count(ME, 3), count(DUYGU, 1)],
                blockers: vec![DUYGU],
            }
        );
    }

    #[test]
    fn guests_never_block_a_resident() {
        let mut entries = by(MARCO, Role::Guest, 1);
        entries.extend(by(DUYGU, Role::Resident, 2));
        entries.extend(by(ME, Role::Resident, 2));
        assert_eq!(
            event_deletion(&resident(), &entries, now()),
            EventDeletion::Blocked {
                recap: vec![count(ME, 2), count(DUYGU, 2), count(MARCO, 1)],
                blockers: vec![DUYGU],
            }
        );
    }

    #[test]
    fn a_viewer_or_expired_guest_sees_nothing() {
        let entries = by(ME, Role::Guest, 2);
        let expired = Some(now() - TimeDelta::days(1));
        for who in [
            actor(Role::Viewer, None),
            actor(Role::Viewer, expired),
            actor(Role::Guest, expired),
        ] {
            assert_eq!(
                event_deletion(&who, &entries, now()),
                EventDeletion::Hidden,
                "{who:?}"
            );
        }
    }

    #[test]
    fn none_of_mine_is_hidden() {
        let entries = by(DUYGU, Role::Resident, 2);
        assert_eq!(
            event_deletion(&resident(), &entries, now()),
            EventDeletion::Hidden
        );
    }

    #[test]
    fn no_entries_is_hidden() {
        assert_eq!(
            event_deletion(&resident(), &[], now()),
            EventDeletion::Hidden
        );
    }

    #[test]
    fn a_guest_is_blocked_by_anyone_else() {
        let guest = actor(Role::Guest, Some(now() + TimeDelta::days(1)));
        let mut entries = by(ME, Role::Guest, 1);
        entries.extend(by(DUYGU, Role::Resident, 1));
        entries.extend(by(MARCO, Role::Guest, 2));
        assert_eq!(
            event_deletion(&guest, &entries, now()),
            EventDeletion::Blocked {
                recap: vec![count(ME, 1), count(MARCO, 2), count(DUYGU, 1)],
                blockers: vec![MARCO, DUYGU],
            }
        );
    }

    #[test]
    fn the_recap_leaves_out_an_actor_without_entries() {
        let mut entries = by(MARCO, Role::Guest, 1);
        entries.extend(by(DUYGU, Role::Resident, 1));
        let EventDeletion::Blocked { recap, blockers } =
            event_deletion(&resident(), &entries, now())
        else {
            panic!("expected Blocked");
        };
        assert_eq!(recap, vec![count(DUYGU, 1), count(MARCO, 1)]);
        assert_eq!(blockers, vec![DUYGU]);
    }
}
