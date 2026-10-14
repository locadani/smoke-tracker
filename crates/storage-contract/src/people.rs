//! The account use cases of `smoke_domain::people`, run against each backend: invitations,
//! setup and reset links, expiry, reactivation and password changes.

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::DomainError;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::Clock;
use smoke_domain::people::{
    NewPerson, PeopleError, accept, change_password, expire, invite, issue_link, reactivate,
};
use smoke_domain::records::{Invite, PasswordHash, StoredSession, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::types::{InvitePurpose, Locale, Role, TokenHash, Username};

use crate::Fixture;
use crate::fixture::{add_user, start};

fn person(name: &str, role: Role, expires_at: Option<DateTime<Utc>>) -> NewPerson {
    NewPerson {
        username: Username::new(name).unwrap(),
        role,
        locale: Locale::It,
        expires_at,
    }
}

fn next_week() -> Option<DateTime<Utc>> {
    Some(start() + TimeDelta::days(7))
}

fn hash(text: &str) -> PasswordHash {
    PasswordHash::new(text.to_owned())
}

async fn invite_guest<S: Store>(fx: &Fixture<S>, by: &User, token: &str) -> (User, Invite) {
    let mut tx = fx.store.begin(Some(by.id)).await.unwrap();
    let invited = invite(
        &mut tx,
        &by.actor(),
        person("guest", Role::Guest, next_week()),
        TokenHash::of(token),
        fx.clock.now(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    invited
}

async fn accept_link<S: Store>(
    fx: &Fixture<S>,
    user: &User,
    token: &str,
) -> Result<User, PeopleError> {
    let mut tx = fx.store.begin(Some(user.id)).await.unwrap();
    let accepted = accept(
        &mut tx,
        &TokenHash::of(token),
        hash("chosen-hash"),
        Locale::Tr,
        fx.clock.now(),
    )
    .await?;
    tx.commit().await.unwrap();
    Ok(accepted)
}

async fn actions<S: Store>(fx: &Fixture<S>) -> Vec<(AuditEntity, AuditAction)> {
    fx.store
        .audit_log()
        .await
        .unwrap()
        .iter()
        .map(|e| (e.record.entity, e.record.action))
        .collect()
}

pub async fn an_invited_guest_sets_up_the_account_once<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let (guest, link) = invite_guest(fx, &resident, "setup-token").await;
    assert_eq!(guest.password_hash, None);
    assert_eq!((guest.role, guest.expires_at), (Role::Guest, next_week()));
    assert_eq!(link.purpose, InvitePurpose::Setup);
    assert_eq!(link.expires_at, start() + Invite::LIFETIME);

    fx.clock.advance(TimeDelta::minutes(4));
    let set_up = accept_link(fx, &guest, "setup-token").await.unwrap();
    assert_eq!(set_up.password_hash, Some(hash("chosen-hash")));
    assert_eq!(set_up.locale, Locale::Tr);
    assert!(matches!(
        accept_link(fx, &guest, "setup-token").await,
        Err(PeopleError::LinkInvalid)
    ));
    assert_eq!(
        actions(fx).await,
        [
            (AuditEntity::User, AuditAction::Create),
            (AuditEntity::User, AuditAction::Create),
            (AuditEntity::Invite, AuditAction::Invite),
            (AuditEntity::Invite, AuditAction::Update),
            (AuditEntity::User, AuditAction::PasswordChange),
            (AuditEntity::User, AuditAction::Update),
        ]
    );
}

pub async fn a_link_stops_working_after_five_minutes<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let (guest, _) = invite_guest(fx, &resident, "setup-token").await;
    fx.clock.advance(Invite::LIFETIME);
    assert!(matches!(
        accept_link(fx, &guest, "setup-token").await,
        Err(PeopleError::LinkInvalid)
    ));
    assert!(matches!(
        accept_link(fx, &guest, "never-issued").await,
        Err(PeopleError::LinkInvalid)
    ));
    let stored = fx.store.user(guest.id).await.unwrap().unwrap();
    assert_eq!(stored.password_hash, None);
}

pub async fn a_reset_link_replaces_the_password_and_ends_every_session<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let partner = add_user(fx, "partner", Role::Resident).await;
    fx.store
        .create_session(&StoredSession {
            id_hash: "partner-phone".to_owned(),
            user_id: Some(partner.id),
            data: "{}".to_owned(),
            expires_at: start() + TimeDelta::days(30),
        })
        .await
        .unwrap();
    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
    let (_, first) = issue_link(
        &mut tx,
        &resident.actor(),
        partner.id,
        TokenHash::of("first"),
        start(),
    )
    .await
    .unwrap();
    let (_, second) = issue_link(
        &mut tx,
        &resident.actor(),
        partner.id,
        TokenHash::of("second"),
        start(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(first.purpose, InvitePurpose::PasswordReset);
    assert_eq!(second.purpose, InvitePurpose::PasswordReset);

    assert!(matches!(
        accept_link(fx, &partner, "first").await,
        Err(PeopleError::LinkInvalid)
    ));
    let reset = accept_link(fx, &partner, "second").await.unwrap();
    assert_eq!(reset.password_hash, Some(hash("chosen-hash")));
    assert_eq!(
        fx.store.session("partner-phone", start()).await.unwrap(),
        None
    );
}

pub async fn a_new_link_for_someone_who_never_set_up_is_a_setup_link<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let (guest, _) = invite_guest(fx, &resident, "lost").await;
    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
    let (_, again) = issue_link(
        &mut tx,
        &resident.actor(),
        guest.id,
        TokenHash::of("again"),
        start(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(again.purpose, InvitePurpose::Setup);
    assert!(accept_link(fx, &guest, "lost").await.is_err());
    assert!(accept_link(fx, &guest, "again").await.is_ok());
}

pub async fn only_residents_manage_people<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let guest = add_user(fx, "guest", Role::Guest).await;
    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
    let before = actions(fx).await;
    for outsider in [&guest, &viewer] {
        let actor = outsider.actor();
        let mut tx = fx.store.begin(Some(outsider.id)).await.unwrap();
        let refused = [
            invite(
                &mut tx,
                &actor,
                person("friend", Role::Guest, next_week()),
                TokenHash::of("x"),
                start(),
            )
            .await
            .map(|_| ()),
            issue_link(&mut tx, &actor, resident.id, TokenHash::of("y"), start())
                .await
                .map(|_| ()),
            expire(&mut tx, &actor, resident.id, start())
                .await
                .map(|_| ()),
            reactivate(&mut tx, &actor, guest.id, next_week(), start())
                .await
                .map(|_| ()),
        ];
        for result in refused {
            assert!(
                matches!(result, Err(PeopleError::Forbidden)),
                "{:?}: {result:?}",
                outsider.role
            );
        }
        tx.commit().await.unwrap();
    }
    assert_eq!(actions(fx).await, before);
}

pub async fn a_taken_username_is_refused_and_nothing_is_written<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let before = actions(fx).await;
    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
    let result = invite(
        &mut tx,
        &resident.actor(),
        person("Daniele", Role::Guest, next_week()),
        TokenHash::of("x"),
        start(),
    )
    .await;
    assert!(
        matches!(result, Err(PeopleError::UsernameTaken)),
        "{result:?}"
    );
    drop(tx);
    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
    let result = invite(
        &mut tx,
        &resident.actor(),
        person("friend", Role::Guest, None),
        TokenHash::of("x"),
        start(),
    )
    .await;
    assert!(
        matches!(
            result,
            Err(PeopleError::Invalid(DomainError::GuestsMustExpire))
        ),
        "{result:?}"
    );
    drop(tx);
    assert_eq!(actions(fx).await, before);
    assert!(
        fx.store
            .invite_by_token(&TokenHash::of("x"))
            .await
            .unwrap()
            .is_none()
    );
}

pub async fn expiry_and_reactivation_follow_the_role_rules<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let guest = add_user(fx, "guest", Role::Guest).await;
    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
    let actor = resident.actor();
    let now = start() + TimeDelta::hours(1);
    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();

    let result = reactivate(&mut tx, &actor, guest.id, next_week(), now).await;
    assert!(matches!(result, Err(PeopleError::NotExpired)), "{result:?}");
    let result = expire(&mut tx, &actor, resident.id, now).await;
    assert!(
        matches!(
            result,
            Err(PeopleError::Invalid(DomainError::ResidentsNeverExpire))
        ),
        "{result:?}"
    );

    fx.clock.set(now);
    let expired = expire(&mut tx, &actor, guest.id, now).await.unwrap();
    assert_eq!(expired.expires_at, Some(now));
    let again = expire(&mut tx, &actor, guest.id, now + TimeDelta::hours(1))
        .await
        .unwrap();
    assert_eq!(
        again.expires_at,
        Some(now),
        "an expired account stays as it was"
    );

    let later = now + TimeDelta::hours(1);
    let result = reactivate(&mut tx, &actor, guest.id, None, later).await;
    assert!(
        matches!(
            result,
            Err(PeopleError::Invalid(DomainError::GuestsMustExpire))
        ),
        "{result:?}"
    );
    let back = reactivate(&mut tx, &actor, guest.id, next_week(), later)
        .await
        .unwrap();
    assert_eq!(back.expires_at, next_week());
    assert_eq!(
        back.password_hash, guest.password_hash,
        "the old password still works"
    );

    expire(&mut tx, &actor, viewer.id, now).await.unwrap();
    let unlimited = reactivate(&mut tx, &actor, viewer.id, None, later)
        .await
        .unwrap();
    assert_eq!(unlimited.expires_at, None);
    tx.commit().await.unwrap();

    let user_actions: Vec<_> = actions(fx)
        .await
        .into_iter()
        .skip(3)
        .map(|(_, action)| action)
        .collect();
    assert_eq!(
        user_actions,
        [
            AuditAction::Expire,
            AuditAction::Reactivate,
            AuditAction::Expire,
            AuditAction::Reactivate
        ]
    );
}

pub async fn a_password_change_needs_an_account_that_can_sign_in<S: Store>(fx: &Fixture<S>) {
    let guest = add_user(fx, "guest", Role::Guest).await;
    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
    let after_expiry = start() + TimeDelta::days(8);
    assert!(guest.actor().is_expired(after_expiry));
    let mut expired_viewer = viewer.actor();
    expired_viewer.expires_at = Some(start());

    let mut tx = fx.store.begin(Some(guest.id)).await.unwrap();
    let changed = change_password(&mut tx, &guest.actor(), hash("new"), after_expiry)
        .await
        .unwrap();
    assert_eq!(
        changed.password_hash,
        Some(hash("new")),
        "an expired guest still signs in"
    );
    let refused = change_password(&mut tx, &expired_viewer, hash("new"), after_expiry).await;
    assert!(
        matches!(refused, Err(PeopleError::Forbidden)),
        "{refused:?}"
    );
    tx.commit().await.unwrap();
}
