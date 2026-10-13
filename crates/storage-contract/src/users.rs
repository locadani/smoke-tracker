//! Users: accounts, passwords, languages and expiry.

use chrono::TimeDelta;
use smoke_domain::DomainError;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::{Clock, truncate_to_micros};
use smoke_domain::records::{Audited, NewUser, PasswordHash};
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, StoreError, Transactional, UserRepo, UserWrites,
};
use smoke_domain::types::{Locale, Role, UserId, Username};

use crate::Fixture;
use crate::fixture::{PASSWORD, add_user, last_entry, new_user, start};

pub async fn creates_and_finds_a_user<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + UserRepo,
{
    let user = add_user(fx, " Daniele ", Role::Resident).await;
    assert_eq!(user.username.as_str(), "daniele");
    assert_eq!(user.created_at, start());
    assert_eq!(user.expires_at, None);
    assert_eq!(fx.store.user(user.id).await.unwrap(), Some(user.clone()));
    let by_name = Username::new("DANIELE").unwrap();
    assert_eq!(
        fx.store.user_by_username(&by_name).await.unwrap(),
        Some(user.clone())
    );
    assert_eq!(fx.store.users().await.unwrap(), [user]);
}

pub async fn usernames_are_unique_ignoring_case<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites>,
{
    add_user(fx, "daniele", Role::Resident).await;
    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx
        .create_user(new_user("Daniele", Role::Guest))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UsernameTaken)),
        "{error:?}"
    );
}

pub async fn creating_a_user_is_audited_without_the_password_hash<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + AuditRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(log.len(), 1);
    let entry = &log[0].record;
    assert_eq!(
        (entry.entity, entry.entity_id, entry.action),
        (AuditEntity::User, user.id.0, AuditAction::Create)
    );
    assert_eq!(entry.actor_id, None);
    assert_eq!(entry.at_micros, start().timestamp_micros());
    assert_eq!(entry.before, None);
    assert_eq!(entry.after, Some(user.snapshot()));
    assert_eq!(entry.after.as_ref().unwrap()["has_password"], true);
    assert!(
        !format!("{log:?}").contains(PASSWORD),
        "the password hash reached the audit log"
    );
}

pub async fn a_password_change_is_always_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + UserRepo + AuditRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let new_hash = PasswordHash::new("another-hash".to_owned());
    let mut tx = fx.store.begin(Some(user.id)).await.unwrap();
    tx.set_password(user.id, new_hash.clone()).await.unwrap();
    tx.commit().await.unwrap();

    let stored = fx.store.user(user.id).await.unwrap().unwrap();
    assert_eq!(stored.password_hash, Some(new_hash));
    let entry = last_entry(fx).await.record;
    assert_eq!(entry.action, AuditAction::PasswordChange);
    assert_eq!(entry.actor_id, Some(user.id));
    assert_eq!(entry.before, entry.after);
    let log = fx.store.audit_log().await.unwrap();
    assert!(!format!("{log:?}").contains("another-hash"));
}

pub async fn a_locale_change_is_audited_but_a_no_op_is_not<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + AuditRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let mut tx = fx.store.begin(Some(user.id)).await.unwrap();
    let changed = tx.set_locale(user.id, Locale::Tr).await.unwrap();
    assert_eq!(changed.locale, Locale::Tr);
    tx.set_locale(user.id, Locale::Tr).await.unwrap();
    tx.commit().await.unwrap();

    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(log.len(), 2);
    let update = &log[1].record;
    assert_eq!(update.action, AuditAction::Update);
    assert_eq!(update.before.as_ref().unwrap()["locale"], "en");
    assert_eq!(update.after.as_ref().unwrap()["locale"], "tr");
}

pub async fn guests_can_be_expired_and_reactivated<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + AuditRepo,
{
    let guest = add_user(fx, "guest", Role::Guest).await;
    fx.clock.advance(TimeDelta::hours(1));
    let now = fx.clock.now();

    let mut tx = fx.store.begin(None).await.unwrap();
    let expired = tx.expire_user(guest.id).await.unwrap();
    assert_eq!(expired.expires_at, Some(now));
    assert!(expired.actor().is_expired(now));

    let next_week = Some(now + TimeDelta::days(7));
    let reactivated = tx.reactivate_user(guest.id, next_week).await.unwrap();
    assert_eq!(reactivated.expires_at, next_week);

    let error = tx.reactivate_user(guest.id, None).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Invalid(DomainError::GuestsMustExpire)),
        "{error:?}"
    );
    let past = Some(now - TimeDelta::seconds(1));
    let error = tx.reactivate_user(guest.id, past).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Invalid(DomainError::ExpiryNotInFuture)),
        "{error:?}"
    );
    tx.commit().await.unwrap();

    let actions: Vec<_> = fx
        .store
        .audit_log()
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(
        actions,
        [
            AuditAction::Create,
            AuditAction::Expire,
            AuditAction::Reactivate
        ]
    );
}

pub async fn residents_cannot_expire<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites>,
{
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx.expire_user(resident.id).await.unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::Invalid(DomainError::ResidentsNeverExpire)
        ),
        "{error:?}"
    );
    let with_expiry = NewUser {
        expires_at: Some(start() + TimeDelta::days(1)),
        ..new_user("other", Role::Resident)
    };
    let error = tx.create_user(with_expiry).await.unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::Invalid(DomainError::ResidentsNeverExpire)
        ),
        "{error:?}"
    );
}

pub async fn a_dropped_transaction_leaves_no_trace<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + UserRepo + AuditRepo,
{
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.create_user(new_user("daniele", Role::Resident))
        .await
        .unwrap();
    drop(tx);
    assert!(fx.store.users().await.unwrap().is_empty());
    assert!(fx.store.audit_log().await.unwrap().is_empty());
}

pub async fn instants_are_kept_to_the_microsecond<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + UserRepo + AuditRepo,
{
    let precise = start() + TimeDelta::days(7) + TimeDelta::nanoseconds(123_456_789);
    let mut tx = fx.store.begin(None).await.unwrap();
    let guest = tx
        .create_user(NewUser {
            expires_at: Some(precise),
            ..new_user("guest", Role::Guest)
        })
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(guest.expires_at, Some(truncate_to_micros(precise)));
    assert_eq!(fx.store.user(guest.id).await.unwrap(), Some(guest.clone()));
    assert_eq!(last_entry(fx).await.record.after, Some(guest.snapshot()));
}

pub async fn writing_an_unknown_user_is_not_found<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites>,
{
    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx.set_locale(UserId(999), Locale::It).await.unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::NotFound {
                entity: AuditEntity::User,
                id: 999
            }
        ),
        "{error:?}"
    );
}
