//! The account use cases (spec §4, §8.3, §8.4): inviting people, setup and password-reset
//! links, expiry, reactivation and password changes. Like the logbook, each one runs inside a
//! write transaction the caller begins and commits, and checks permissions against the data read
//! inside it.

use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc};
use thiserror::Error;

use crate::DomainError;
use crate::permissions::{Action, Actor, can, check_expiry};
use crate::records::{Invite, NewInvite, NewUser, PasswordHash, User};
use crate::repos::{Conflict, InviteWrites, StoreError, UserWrites};
use crate::schedule::DailySchedule;
use crate::types::{InvitePurpose, Locale, Role, TokenHash, UserId, Username};

#[derive(Debug, Error)]
pub enum PeopleError {
    #[error("not allowed")]
    Forbidden,
    /// No such user.
    #[error("not found")]
    NotFound,
    #[error("that username is taken")]
    UsernameTaken,
    /// Only an expired account can be reactivated.
    #[error("the account has not expired")]
    NotExpired,
    /// The link is unknown, already used, or older than five minutes.
    #[error("the link is no longer valid")]
    LinkInvalid,
    #[error(transparent)]
    Invalid(#[from] DomainError),
    #[error(transparent)]
    Store(StoreError),
}

impl From<StoreError> for PeopleError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Conflict(Conflict::UsernameTaken) => Self::UsernameTaken,
            StoreError::Conflict(Conflict::InviteUsed) => Self::LinkInvalid,
            StoreError::Invalid(invalid) => Self::Invalid(invalid),
            other => Self::Store(other),
        }
    }
}

/// When a new or reactivated account stops working (spec §8.3): the quick picks for guests, a
/// chosen day, or never.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryPick {
    OneDay,
    /// Until the start of the coming Monday.
    Weekend,
    OneWeek,
    TwoWeeks,
    /// Until the end of this local day.
    Until(NaiveDate),
    Never,
}

impl ExpiryPick {
    /// The instant the account expires, in the configured time zone; `None` for never.
    pub fn resolve(self, now: DateTime<Utc>, schedule: &DailySchedule) -> Option<DateTime<Utc>> {
        match self {
            Self::OneDay => Some(now + TimeDelta::days(1)),
            Self::OneWeek => Some(now + TimeDelta::days(7)),
            Self::TwoWeeks => Some(now + TimeDelta::days(14)),
            Self::Weekend => {
                let today = schedule.local_date(now);
                let days = 7 - i64::from(today.weekday().num_days_from_monday());
                let monday = today + TimeDelta::days(days);
                Some(schedule.day_span(monday).0)
            }
            Self::Until(date) => Some(schedule.day_span(date).1),
            Self::Never => None,
        }
    }
}

/// A person a resident invites: the account is created without a password, which the invitee
/// chooses through the setup link.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPerson {
    pub username: Username,
    pub role: Role,
    /// Shown first on the setup page; the invitee confirms or changes it there.
    pub locale: Locale,
    pub expires_at: Option<DateTime<Utc>>,
}

fn require_manager(actor: &Actor, now: DateTime<Utc>) -> Result<(), PeopleError> {
    if can(actor, Action::ManageUsers, now) {
        Ok(())
    } else {
        Err(PeopleError::Forbidden)
    }
}

async fn require_user<T: UserWrites>(tx: &mut T, id: UserId) -> Result<User, PeopleError> {
    tx.user(id).await?.ok_or(PeopleError::NotFound)
}

async fn new_link<T: InviteWrites>(
    tx: &mut T,
    actor: &Actor,
    user: &User,
    token: TokenHash,
    now: DateTime<Utc>,
) -> Result<Invite, PeopleError> {
    let purpose = if user.password_hash.is_some() {
        InvitePurpose::PasswordReset
    } else {
        InvitePurpose::Setup
    };
    let invite = tx
        .create_invite(NewInvite {
            user_id: user.id,
            purpose,
            token_hash: token,
            created_by: actor.id,
            expires_at: now + Invite::LIFETIME,
        })
        .await?;
    Ok(invite)
}

/// Creates the account and its setup link (spec §8.3). `token` is the hash of the link's
/// token, which only the QR code carries.
pub async fn invite<T: UserWrites + InviteWrites>(
    tx: &mut T,
    actor: &Actor,
    person: NewPerson,
    token: TokenHash,
    now: DateTime<Utc>,
) -> Result<(User, Invite), PeopleError> {
    require_manager(actor, now)?;
    check_expiry(person.role, person.expires_at, now)?;
    let user = tx
        .create_user(NewUser {
            username: person.username,
            password_hash: None,
            role: person.role,
            locale: person.locale,
            expires_at: person.expires_at,
        })
        .await?;
    let invite = new_link(tx, actor, &user, token, now).await?;
    Ok((user, invite))
}

/// A fresh link for an existing account: a password reset, or a new setup link if the person
/// never chose a password. Earlier unused links stop working.
pub async fn issue_link<T: UserWrites + InviteWrites>(
    tx: &mut T,
    actor: &Actor,
    user_id: UserId,
    token: TokenHash,
    now: DateTime<Utc>,
) -> Result<(User, Invite), PeopleError> {
    require_manager(actor, now)?;
    let user = require_user(tx, user_id).await?;
    let invite = new_link(tx, actor, &user, token, now).await?;
    Ok((user, invite))
}

/// The usable invite behind a link, or [`PeopleError::LinkInvalid`].
pub async fn usable_invite<T: InviteWrites>(
    tx: &mut T,
    token: &TokenHash,
    now: DateTime<Utc>,
) -> Result<Invite, PeopleError> {
    tx.invite_by_token(token)
        .await?
        .filter(|invite| invite.is_usable(now))
        .ok_or(PeopleError::LinkInvalid)
}

/// The invitee chose a password and a language through the link (spec §8.3). The link is used
/// up, and every session the account had ends with the password change.
pub async fn accept<T: UserWrites + InviteWrites>(
    tx: &mut T,
    token: &TokenHash,
    password: PasswordHash,
    locale: Locale,
    now: DateTime<Utc>,
) -> Result<User, PeopleError> {
    let invite = usable_invite(tx, token, now).await?;
    tx.use_invite(invite.id).await?;
    tx.set_password(invite.user_id, password).await?;
    Ok(tx.set_locale(invite.user_id, locale).await?)
}

/// Ends a guest's or viewer's access now (spec §4) and every session they have. Expiring an
/// account that has already expired changes nothing.
pub async fn expire<T: UserWrites>(
    tx: &mut T,
    actor: &Actor,
    user_id: UserId,
    now: DateTime<Utc>,
) -> Result<User, PeopleError> {
    require_manager(actor, now)?;
    let user = require_user(tx, user_id).await?;
    if user.actor().is_expired(now) {
        return Ok(user);
    }
    Ok(tx.expire_user(user_id).await?)
}

/// Gives an expired account a new expiry (or none, for a viewer); the old password still works
/// (spec §4).
pub async fn reactivate<T: UserWrites>(
    tx: &mut T,
    actor: &Actor,
    user_id: UserId,
    expires_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<User, PeopleError> {
    require_manager(actor, now)?;
    let user = require_user(tx, user_id).await?;
    if !user.actor().is_expired(now) {
        return Err(PeopleError::NotExpired);
    }
    Ok(tx.reactivate_user(user_id, expires_at).await?)
}

/// The signed-in person sets a new password (spec §9 screen 7). Every session of the account
/// ends; the caller signs the current device in again.
pub async fn change_password<T: UserWrites>(
    tx: &mut T,
    actor: &Actor,
    password: PasswordHash,
    now: DateTime<Utc>,
) -> Result<User, PeopleError> {
    if !actor.can_sign_in(now) {
        return Err(PeopleError::Forbidden);
    }
    Ok(tx.set_password(actor.id, password).await?)
}

#[cfg(test)]
mod tests {
    use chrono_tz::Europe::Rome;

    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn rome() -> DailySchedule {
        DailySchedule::new(Rome, 4).unwrap()
    }

    #[test]
    fn quick_picks_count_from_now() {
        let now = t("2026-01-15T18:30:00Z");
        assert_eq!(
            ExpiryPick::OneDay.resolve(now, &rome()),
            Some(t("2026-01-16T18:30:00Z"))
        );
        assert_eq!(
            ExpiryPick::OneWeek.resolve(now, &rome()),
            Some(t("2026-01-22T18:30:00Z"))
        );
        assert_eq!(
            ExpiryPick::TwoWeeks.resolve(now, &rome()),
            Some(t("2026-01-29T18:30:00Z"))
        );
        assert_eq!(ExpiryPick::Never.resolve(now, &rome()), None);
    }

    #[test]
    fn the_weekend_lasts_until_the_coming_monday_in_local_time() {
        let thursday = t("2026-01-15T18:30:00Z");
        let monday = Some(t("2026-01-18T23:00:00Z"));
        assert_eq!(ExpiryPick::Weekend.resolve(thursday, &rome()), monday);
        let sunday_night = t("2026-01-18T22:30:00Z");
        assert_eq!(ExpiryPick::Weekend.resolve(sunday_night, &rome()), monday);
        let monday_morning = t("2026-01-19T07:00:00Z");
        assert_eq!(
            ExpiryPick::Weekend.resolve(monday_morning, &rome()),
            Some(t("2026-01-25T23:00:00Z")),
            "on a Monday the weekend is the coming one"
        );
    }

    #[test]
    fn a_chosen_day_lasts_until_its_local_midnight() {
        let day = NaiveDate::from_ymd_opt(2026, 3, 28).unwrap();
        assert_eq!(
            ExpiryPick::Until(day).resolve(t("2026-01-15T18:30:00Z"), &rome()),
            Some(t("2026-03-28T23:00:00Z")),
            "winter time still holds at that midnight"
        );
    }
}
