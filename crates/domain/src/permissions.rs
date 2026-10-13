//! Who may do what. This is the spec §4 table, expressed as one function.

use chrono::{DateTime, Utc};

use crate::DomainError;
use crate::types::{Role, UserId};

/// The signed-in user, as far as permissions are concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub id: UserId,
    pub role: Role,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Actor {
    /// Expired once `now` reaches `expires_at`. Residents have no expiry.
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.is_some_and(|expires_at| expires_at <= now)
    }

    /// Expired guests keep read access to their own data; expired viewers lose all access.
    pub fn can_sign_in(&self, now: DateTime<Utc>) -> bool {
        !(self.role == Role::Viewer && self.is_expired(now))
    }
}

/// Residents never expire; guests always do; viewers may. A new expiry must lie in the future.
pub fn check_expiry(
    role: Role,
    expires_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<(), DomainError> {
    match (role, expires_at) {
        (Role::Resident, Some(_)) => Err(DomainError::ResidentsNeverExpire),
        (Role::Guest, None) => Err(DomainError::GuestsMustExpire),
        (_, Some(at)) if at <= now => Err(DomainError::ExpiryNotInFuture),
        _ => Ok(()),
    }
}

/// Something a user might try to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Log a smell report or PM2.5 reading.
    LogEntry,
    /// See everyone's data.
    ViewAll,
    /// See one's own data.
    ViewOwn,
    /// Correct or move an entry owned by `owner`.
    EditEntry { owner: UserId },
    /// Close or reopen an event owned by `owner`.
    ManageEvent { owner: UserId },
    /// Invite, expire, reactivate users or reset their passwords.
    ManageUsers,
    /// Edit the room and symptom lists.
    EditLists,
}

pub fn can(actor: &Actor, action: Action, now: DateTime<Utc>) -> bool {
    let active = !actor.is_expired(now);
    match actor.role {
        Role::Resident => true,
        Role::Guest => match action {
            Action::ViewOwn => true,
            Action::LogEntry => active,
            Action::EditEntry { owner } | Action::ManageEvent { owner } => {
                active && owner == actor.id
            }
            Action::ViewAll | Action::ManageUsers | Action::EditLists => false,
        },
        Role::Viewer => active && matches!(action, Action::ViewAll | Action::ViewOwn),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::*;

    const ME: UserId = UserId(1);
    const OTHER: UserId = UserId(2);

    const ACTIONS: [Action; 9] = [
        Action::LogEntry,
        Action::ViewAll,
        Action::ViewOwn,
        Action::EditEntry { owner: ME },
        Action::EditEntry { owner: OTHER },
        Action::ManageEvent { owner: ME },
        Action::ManageEvent { owner: OTHER },
        Action::ManageUsers,
        Action::EditLists,
    ];

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

    fn active() -> Option<DateTime<Utc>> {
        Some(now() + TimeDelta::days(1))
    }

    fn expired() -> Option<DateTime<Utc>> {
        Some(now() - TimeDelta::days(1))
    }

    /// The spec §4 permissions table, cell by cell. Columns follow `ACTIONS`:
    /// log, view all, view own, edit own, edit other, manage own event, manage other event,
    /// manage users, edit lists.
    #[test]
    fn permission_matrix_matches_the_spec() {
        const T: bool = true;
        const F: bool = false;
        let rows = [
            (
                "resident",
                actor(Role::Resident, None),
                [T, T, T, T, T, T, T, T, T],
            ),
            (
                "active guest",
                actor(Role::Guest, active()),
                [T, F, T, T, F, T, F, F, F],
            ),
            (
                "expired guest",
                actor(Role::Guest, expired()),
                [F, F, T, F, F, F, F, F, F],
            ),
            (
                "active viewer",
                actor(Role::Viewer, active()),
                [F, T, T, F, F, F, F, F, F],
            ),
            (
                "viewer without expiry",
                actor(Role::Viewer, None),
                [F, T, T, F, F, F, F, F, F],
            ),
            (
                "expired viewer",
                actor(Role::Viewer, expired()),
                [F, F, F, F, F, F, F, F, F],
            ),
        ];
        for (label, actor, expected) in rows {
            for (action, want) in ACTIONS.iter().zip(expected) {
                assert_eq!(can(&actor, *action, now()), want, "{label} / {action:?}");
            }
        }
    }

    #[test]
    fn expiry_at_the_exact_instant_counts_as_expired() {
        let guest = actor(Role::Guest, Some(now()));
        assert!(guest.is_expired(now()));
        assert!(!can(&guest, Action::LogEntry, now()));
    }

    #[test]
    fn residents_never_expire() {
        assert!(!actor(Role::Resident, None).is_expired(now()));
    }

    #[test]
    fn expired_guests_can_still_sign_in_but_expired_viewers_cannot() {
        assert!(actor(Role::Guest, expired()).can_sign_in(now()));
        assert!(!actor(Role::Viewer, expired()).can_sign_in(now()));
        assert!(actor(Role::Viewer, active()).can_sign_in(now()));
        assert!(actor(Role::Resident, None).can_sign_in(now()));
    }

    #[test]
    fn expiry_rules_follow_the_role() {
        let later = Some(now() + TimeDelta::days(7));
        assert_eq!(check_expiry(Role::Resident, None, now()), Ok(()));
        assert_eq!(
            check_expiry(Role::Resident, later, now()),
            Err(DomainError::ResidentsNeverExpire)
        );
        assert_eq!(check_expiry(Role::Guest, later, now()), Ok(()));
        assert_eq!(
            check_expiry(Role::Guest, None, now()),
            Err(DomainError::GuestsMustExpire)
        );
        assert_eq!(check_expiry(Role::Viewer, None, now()), Ok(()));
        assert_eq!(check_expiry(Role::Viewer, later, now()), Ok(()));
    }

    #[test]
    fn a_new_expiry_must_be_in_the_future() {
        assert_eq!(
            check_expiry(Role::Guest, Some(now()), now()),
            Err(DomainError::ExpiryNotInFuture)
        );
        assert_eq!(
            check_expiry(Role::Viewer, Some(now() - TimeDelta::seconds(1)), now()),
            Err(DomainError::ExpiryNotInFuture)
        );
    }
}
