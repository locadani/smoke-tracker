//! Invites: stored by token hash, used once, replaced by a newer link, cleaned up daily.

use chrono::TimeDelta;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::Clock;
use smoke_domain::records::{Audited, Invite, NewInvite, User};
use smoke_domain::repos::{Commit, Conflict, InviteWrites, Store, StoreError, Transactional};
use smoke_domain::types::{InvitePurpose, Role, TokenHash, UserId};
use smoke_domain::verify::verify_store;

use crate::Fixture;
use crate::fixture::{add_user, last_entry, start};

fn new_invite(user: &User, by: &User, token: &str) -> NewInvite {
    NewInvite {
        user_id: user.id,
        purpose: InvitePurpose::Setup,
        token_hash: TokenHash::of(token),
        created_by: by.id,
        expires_at: start() + Invite::LIFETIME,
    }
}

async fn add_invite<S: Transactional<Tx: InviteWrites>>(fx: &Fixture<S>, new: NewInvite) -> Invite {
    let mut tx = fx.store.begin(Some(new.created_by)).await.unwrap();
    let invite = tx.create_invite(new).await.unwrap();
    tx.commit().await.unwrap();
    invite
}

pub async fn an_invite_is_found_by_its_token_hash_only<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let guest = add_user(fx, "guest", Role::Guest).await;
    let invite = add_invite(fx, new_invite(&guest, &resident, "token-a")).await;
    assert_eq!(
        (invite.user_id, invite.created_by, invite.purpose),
        (guest.id, resident.id, InvitePurpose::Setup)
    );
    assert_eq!(
        (invite.created_at, invite.expires_at, invite.used_at),
        (start(), start() + Invite::LIFETIME, None)
    );
    assert_eq!(
        fx.store
            .invite_by_token(&TokenHash::of("token-a"))
            .await
            .unwrap(),
        Some(invite.clone())
    );
    assert_eq!(
        fx.store
            .invite_by_token(&TokenHash::of("token-b"))
            .await
            .unwrap(),
        None
    );

    let entry = last_entry(fx).await.record;
    assert_eq!(
        (entry.entity, entry.entity_id, entry.action),
        (AuditEntity::Invite, invite.id.0, AuditAction::Invite)
    );
    assert_eq!(entry.after, Some(invite.snapshot()));
    let log = format!("{:?}", fx.store.audit_log().await.unwrap());
    assert!(!log.contains(TokenHash::of("token-a").as_str()));
}

pub async fn an_invite_is_used_once_and_that_is_audited<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let guest = add_user(fx, "guest", Role::Guest).await;
    let invite = add_invite(fx, new_invite(&guest, &resident, "token-a")).await;
    fx.clock.advance(TimeDelta::minutes(1));

    let mut tx = fx.store.begin(Some(guest.id)).await.unwrap();
    let found = tx.invite_by_token(&TokenHash::of("token-a")).await.unwrap();
    assert_eq!(found, Some(invite.clone()));
    let used = tx.use_invite(invite.id).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(used.used_at, Some(fx.clock.now()));
    let entry = last_entry(fx).await.record;
    assert_eq!(
        (entry.entity, entry.action, entry.actor_id),
        (AuditEntity::Invite, AuditAction::Update, Some(guest.id))
    );
    assert_eq!(entry.before, Some(invite.snapshot()));

    let mut tx = fx.store.begin(Some(guest.id)).await.unwrap();
    let error = tx.use_invite(invite.id).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::InviteUsed)),
        "{error:?}"
    );
}

pub async fn a_new_invite_replaces_the_users_unused_ones<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let guest = add_user(fx, "guest", Role::Guest).await;
    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
    let used = add_invite(fx, new_invite(&guest, &resident, "used")).await;
    let mut tx = fx.store.begin(Some(guest.id)).await.unwrap();
    tx.use_invite(used.id).await.unwrap();
    tx.commit().await.unwrap();
    add_invite(fx, new_invite(&guest, &resident, "first")).await;
    add_invite(fx, new_invite(&viewer, &resident, "other-user")).await;

    add_invite(fx, new_invite(&guest, &resident, "second")).await;
    for (token, kept) in [
        ("used", true),
        ("first", false),
        ("other-user", true),
        ("second", true),
    ] {
        let found = fx
            .store
            .invite_by_token(&TokenHash::of(token))
            .await
            .unwrap();
        assert_eq!(found.is_some(), kept, "{token}");
    }
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
}

pub async fn an_invite_for_an_unknown_user_is_refused<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let mut ghost = resident.clone();
    ghost.id = UserId(999);
    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
    let error = tx
        .create_invite(new_invite(&ghost, &resident, "token-a"))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
}

pub async fn expired_invites_are_removed_without_an_audit_entry<S: Store>(fx: &Fixture<S>) {
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let guest = add_user(fx, "guest", Role::Guest).await;
    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
    add_invite(fx, new_invite(&guest, &resident, "old")).await;
    fx.clock.advance(TimeDelta::minutes(1));
    let mut fresh = new_invite(&viewer, &resident, "fresh");
    fresh.expires_at = fx.clock.now() + Invite::LIFETIME;
    add_invite(fx, fresh).await;
    let entries = fx.store.audit_log().await.unwrap().len();

    let at_expiry = start() + Invite::LIFETIME;
    assert_eq!(fx.store.delete_expired_invites(at_expiry).await.unwrap(), 1);
    assert_eq!(fx.store.delete_expired_invites(at_expiry).await.unwrap(), 0);
    assert!(
        fx.store
            .invite_by_token(&TokenHash::of("fresh"))
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
}
