//! Login sessions: stored by hash, unaudited, and ended with a password change or expiry.

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::clock::Clock;
use smoke_domain::records::{PasswordHash, StoredSession};
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, SessionRepo, StoreError, Transactional, UserWrites,
};
use smoke_domain::types::{Role, UserId};

use crate::Fixture;
use crate::fixture::{add_user, start};

fn session(id_hash: &str, user_id: Option<UserId>, expires_at: DateTime<Utc>) -> StoredSession {
    StoredSession {
        id_hash: id_hash.to_owned(),
        user_id,
        data: format!(r#"{{"user_id":{}}}"#, user_id.map_or(0, |u| u.0)),
        expires_at,
    }
}

fn in_30_days() -> DateTime<Utc> {
    start() + TimeDelta::days(30)
}

pub async fn a_session_round_trips_until_it_expires<
    S: SessionRepo + Transactional<Tx: UserWrites>,
>(
    fx: &Fixture<S>,
) {
    let user = add_user(fx, "daniele", Role::Resident).await;
    let stored = session("hash-a", Some(user.id), in_30_days());
    fx.store.create_session(&stored).await.unwrap();
    assert_eq!(
        fx.store.session("hash-a", start()).await.unwrap(),
        Some(stored)
    );
    assert_eq!(
        fx.store.session("hash-a", in_30_days()).await.unwrap(),
        None
    );
    assert_eq!(fx.store.session("unknown", start()).await.unwrap(), None);
}

pub async fn creating_a_session_twice_is_a_duplicate<S: SessionRepo>(fx: &Fixture<S>) {
    fx.store
        .create_session(&session("hash-a", None, in_30_days()))
        .await
        .unwrap();
    let error = fx
        .store
        .create_session(&session("hash-a", None, in_30_days()))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::Duplicate)),
        "{error:?}"
    );
}

pub async fn a_session_for_an_unknown_user_is_refused<S: SessionRepo>(fx: &Fixture<S>) {
    let error = fx
        .store
        .create_session(&session("hash-a", Some(UserId(999)), in_30_days()))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
}

pub async fn saving_a_session_inserts_or_replaces_it<
    S: SessionRepo + Transactional<Tx: UserWrites>,
>(
    fx: &Fixture<S>,
) {
    let user = add_user(fx, "daniele", Role::Resident).await;
    let anonymous = session("hash-a", None, start() + TimeDelta::days(1));
    fx.store.save_session(&anonymous).await.unwrap();
    assert_eq!(
        fx.store.session("hash-a", start()).await.unwrap(),
        Some(anonymous)
    );
    let signed_in = session("hash-a", Some(user.id), in_30_days());
    fx.store.save_session(&signed_in).await.unwrap();
    assert_eq!(
        fx.store.session("hash-a", start()).await.unwrap(),
        Some(signed_in)
    );
}

pub async fn deleting_a_session_removes_only_that_one<S: SessionRepo>(fx: &Fixture<S>) {
    for id_hash in ["hash-a", "hash-b"] {
        fx.store
            .create_session(&session(id_hash, None, in_30_days()))
            .await
            .unwrap();
    }
    fx.store.delete_session("hash-a").await.unwrap();
    fx.store.delete_session("never-existed").await.unwrap();
    assert_eq!(fx.store.session("hash-a", start()).await.unwrap(), None);
    assert!(fx.store.session("hash-b", start()).await.unwrap().is_some());
}

pub async fn expired_sessions_are_swept<S: SessionRepo>(fx: &Fixture<S>) {
    let now = fx.clock.now();
    fx.store
        .create_session(&session("old", None, now - TimeDelta::seconds(1)))
        .await
        .unwrap();
    fx.store
        .create_session(&session("ends-now", None, now))
        .await
        .unwrap();
    fx.store
        .create_session(&session("fresh", None, now + TimeDelta::seconds(1)))
        .await
        .unwrap();
    assert_eq!(fx.store.delete_expired_sessions(now).await.unwrap(), 2);
    assert!(fx.store.session("fresh", now).await.unwrap().is_some());
    assert_eq!(fx.store.delete_expired_sessions(now).await.unwrap(), 0);
}

pub async fn a_password_change_ends_the_users_sessions<
    S: SessionRepo + Transactional<Tx: UserWrites>,
>(
    fx: &Fixture<S>,
) {
    let daniele = add_user(fx, "daniele", Role::Resident).await;
    let other = add_user(fx, "partner", Role::Resident).await;
    for (id_hash, user) in [
        ("d1", Some(daniele.id)),
        ("d2", Some(daniele.id)),
        ("p1", Some(other.id)),
        ("anon", None),
    ] {
        fx.store
            .create_session(&session(id_hash, user, in_30_days()))
            .await
            .unwrap();
    }
    let mut tx = fx.store.begin(Some(daniele.id)).await.unwrap();
    tx.set_password(daniele.id, PasswordHash::new("new-hash".to_owned()))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    for (id_hash, survives) in [("d1", false), ("d2", false), ("p1", true), ("anon", true)] {
        assert_eq!(
            fx.store.session(id_hash, start()).await.unwrap().is_some(),
            survives,
            "{id_hash}"
        );
    }
}

pub async fn expiring_a_user_ends_their_sessions<S: SessionRepo + Transactional<Tx: UserWrites>>(
    fx: &Fixture<S>,
) {
    let guest = add_user(fx, "guest", Role::Guest).await;
    fx.store
        .create_session(&session("g1", Some(guest.id), in_30_days()))
        .await
        .unwrap();
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.expire_user(guest.id).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(fx.store.session("g1", start()).await.unwrap(), None);
}

pub async fn a_rolled_back_password_change_keeps_the_sessions<
    S: SessionRepo + Transactional<Tx: UserWrites>,
>(
    fx: &Fixture<S>,
) {
    let user = add_user(fx, "daniele", Role::Resident).await;
    fx.store
        .create_session(&session("d1", Some(user.id), in_30_days()))
        .await
        .unwrap();
    let mut tx = fx.store.begin(Some(user.id)).await.unwrap();
    tx.set_password(user.id, PasswordHash::new("new-hash".to_owned()))
        .await
        .unwrap();
    drop(tx);
    assert!(fx.store.session("d1", start()).await.unwrap().is_some());
}

pub async fn sessions_are_not_audited<S: SessionRepo + AuditRepo>(fx: &Fixture<S>) {
    let stored = session("hash-a", None, in_30_days());
    fx.store.create_session(&stored).await.unwrap();
    fx.store.save_session(&stored).await.unwrap();
    fx.store.delete_session("hash-a").await.unwrap();
    assert!(fx.store.audit_log().await.unwrap().is_empty());
}
