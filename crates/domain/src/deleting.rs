//! Who may delete the entries of an event, and what the confirmation screen has to say about it.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::logbook::{LogError, follow_remaining_entries, is_earliest_entry};
use crate::permissions::{Action, Actor, can, can_see};
use crate::repos::{EntryWrites, EventWrites, UserWrites};
use crate::types::{EventId, LogEntryId, Role, UserId};

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
        return EventDeletion::Allowed {
            entries: entries.len(),
        };
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

/// Deletes one live entry `actor` may delete. An entry the actor may not even see is
/// [`LogError::NotFound`], so nobody learns it exists. Returns the entry's event if it still exists
/// afterwards; when the entry was the event's earliest, the event's start and owner follow the
/// next one.
pub async fn delete_entry<T: EventWrites + EntryWrites + UserWrites>(
    tx: &mut T,
    actor: &Actor,
    id: LogEntryId,
    now: DateTime<Utc>,
) -> Result<Option<EventId>, LogError> {
    let entry = tx
        .entry(id)
        .await?
        .filter(|entry| !entry.is_deleted())
        .ok_or(LogError::NotFound)?;
    if !can_see(actor, entry.user_id, now) {
        return Err(LogError::NotFound);
    }
    let author = tx.user(entry.user_id).await?.ok_or(LogError::NotFound)?;
    let allowed = can(
        actor,
        Action::DeleteEntry {
            author: author.id,
            author_role: author.role,
        },
        now,
    );
    if !allowed {
        return Err(LogError::Forbidden);
    }
    let Some(event) = entry.event_id else {
        tx.delete_entry(id).await?;
        return Ok(None);
    };
    let earliest = is_earliest_entry(tx, event, id).await?;
    tx.delete_entry(id).await?;
    if earliest {
        follow_remaining_entries(tx, event).await?;
    }
    Ok(tx.event(event).await?.map(|event| event.id))
}

/// Deletes every live entry of an event, all or none: refused with [`LogError::OthersLogged`]
/// when `actor` may not delete some of them. Returns how many entries were deleted.
pub async fn delete_event<T: EventWrites + EntryWrites + UserWrites>(
    tx: &mut T,
    actor: &Actor,
    id: EventId,
    now: DateTime<Utc>,
) -> Result<usize, LogError> {
    tx.event(id).await?.ok_or(LogError::NotFound)?;
    let entries = tx.entries_for_event(id).await?;
    let mut authors = Vec::with_capacity(entries.len());
    for entry in &entries {
        let author = tx.user(entry.user_id).await?.ok_or(LogError::NotFound)?;
        authors.push(EntryAuthor {
            author: author.id,
            role: author.role,
        });
    }
    match event_deletion(actor, &authors, now) {
        EventDeletion::Hidden => Err(LogError::Forbidden),
        EventDeletion::Blocked { .. } => Err(LogError::OthersLogged),
        EventDeletion::Allowed { entries: count } => {
            for entry in &entries {
                tx.delete_entry(entry.id).await?;
            }
            Ok(count)
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::*;

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

    #[test]
    fn equal_counts_are_ordered_by_id() {
        let mut entries = by(MARCO, Role::Guest, 1);
        entries.extend(by(DUYGU, Role::Resident, 1));
        entries.extend(by(ME, Role::Resident, 1));
        let guest = actor(Role::Guest, Some(now() + TimeDelta::days(1)));
        let EventDeletion::Blocked { recap, blockers } = event_deletion(&guest, &entries, now())
        else {
            panic!("expected Blocked");
        };
        assert_eq!(recap, vec![count(ME, 1), count(DUYGU, 1), count(MARCO, 1)]);
        assert_eq!(blockers, vec![DUYGU, MARCO]);
    }
}
