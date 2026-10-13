//! Expired sessions are removed through tower-sessions' `ExpiredDeletion` (spec §11), and a
//! session ended by a password change stays ended.

mod common;

use chrono::{DateTime, TimeDelta, Utc};
use common::*;
use smoke_domain::clock::Clock;
use smoke_domain::records::{PasswordHash, StoredSession};
use smoke_domain::repos::{Commit, SessionRepo, Transactional, UserWrites};
use smoke_domain::types::{Locale, Role};
use smoke_web::DbSessions;
use sqlx::SqlitePool;
use tower_sessions::session::Id;
use tower_sessions::session_store::{ExpiredDeletion, SessionStore};

fn session(id_hash: &str, expires_at: DateTime<Utc>) -> StoredSession {
    StoredSession {
        id_hash: id_hash.to_owned(),
        user_id: None,
        data: "{}".to_owned(),
        expires_at,
    }
}

async fn stored_ids(app: &TestApp) -> Vec<String> {
    let pool = SqlitePool::connect(&app.db_url).await.unwrap();
    sqlx::query_scalar("SELECT id_hash FROM sessions ORDER BY id_hash")
        .fetch_all(&pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn delete_expired_removes_only_expired_sessions() {
    let app = TestApp::new().await;
    let now = app.clock.now();
    for (id_hash, expires_at) in [
        ("ended", now - TimeDelta::seconds(1)),
        ("ends-now", now),
        ("fresh", now + TimeDelta::days(30)),
    ] {
        app.store
            .create_session(&session(id_hash, expires_at))
            .await
            .unwrap();
    }

    let sessions = DbSessions::new(app.store.clone(), app.clock.clone());
    sessions.delete_expired().await.unwrap();
    assert_eq!(stored_ids(&app).await, ["fresh"]);

    sessions.delete_expired().await.unwrap();
    assert_eq!(
        stored_ids(&app).await,
        ["fresh"],
        "running again is harmless"
    );
}

#[tokio::test]
async fn a_late_save_cannot_revive_a_session_a_password_change_ended() {
    let app = TestApp::new().await;
    let user = app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let id: Id = cookie.split_once('=').unwrap().1.parse().unwrap();
    let sessions = DbSessions::new(app.store.clone(), app.clock.clone());
    let loaded = sessions.load(&id).await.unwrap().expect("signed in");

    let mut tx = app.store.begin(Some(user.id)).await.unwrap();
    tx.set_password(user.id, PasswordHash::new("new-hash".to_owned()))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    sessions.save(&loaded).await.unwrap();
    assert_eq!(sessions.load(&id).await.unwrap(), None);
    let page = app.send(get_with("/", &cookie)).await;
    assert_eq!(location(&page), Some("/login"));
}
