//! Settings: language and password for everyone (spec §9 screen 7).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::repos::UserRepo;
use smoke_domain::types::{Locale, Role};

const NEW_PASSWORD: &str = "a brand new passphrase";

fn change(current: &str, new: &str, repeat: &str) -> String {
    format!("current={current}&new={new}&repeat={repeat}").replace(' ', "+")
}

#[tokio::test]
async fn the_settings_page_shows_the_language_and_password_forms() {
    let app = TestApp::new().await;
    app.add_user("guest", Role::Guest, Locale::It).await;
    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/settings", &cookie)).await).await;
    assert!(html.contains("<h1>Impostazioni</h1>"), "{html}");
    assert!(
        html.contains(
            r#"value="it" lang="it" class="current" aria-pressed="true">Italiano</button>"#
        ),
        "{html}"
    );
    assert!(html.contains(r#"action="/settings/password""#), "{html}");
    assert!(
        html.contains(r#"<a href="/settings">Impostazioni</a>"#),
        "{html}"
    );
}

#[tokio::test]
async fn choosing_a_language_in_settings_saves_it_and_comes_back() {
    let app = TestApp::new().await;
    let user = app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let response = app
        .send(post_form(
            "/lang",
            "lang=tr&return_to=settings",
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some("/settings?saved=language"));
    let stored = app.store.user(user.id).await.unwrap().unwrap();
    assert_eq!(stored.locale, Locale::Tr);
    let html = text(
        app.send(get_with("/settings?saved=language", &cookie))
            .await,
    )
    .await;
    assert!(html.contains("Dil kaydedildi."), "{html}");
}

#[tokio::test]
async fn a_password_change_keeps_this_device_and_signs_out_the_others() {
    let app = TestApp::new().await;
    app.add_user("partner", Role::Resident, Locale::En).await;
    let other_device = app.sign_in("partner").await;
    let this_device = app.sign_in("partner").await;

    let response = app
        .send(post_form(
            "/settings/password",
            &change(PASSWORD, NEW_PASSWORD, NEW_PASSWORD),
            Some(&this_device),
        ))
        .await;
    assert_eq!(location(&response), Some("/settings?saved=password"));
    let renewed = session_cookie(&response).expect("a new session for this device");
    assert_ne!(renewed, this_device);

    let page = app
        .send(get_with("/settings?saved=password", &renewed))
        .await;
    assert_eq!(page.status(), StatusCode::OK);
    assert!(
        text(page)
            .await
            .contains("Your other devices are signed out.")
    );
    for ended in [&other_device, &this_device] {
        let page = app.send(get_with("/", ended)).await;
        assert_eq!(location(&page), Some("/login"), "{ended}");
    }
    assert_eq!(
        app.attempt("partner", PASSWORD).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let response = app.send(login_request("partner", NEW_PASSWORD)).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn a_refused_password_change_says_why_and_changes_nothing() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let other_device = app.sign_in("daniele").await;
    for (body, message) in [
        (
            change(PASSWORD, NEW_PASSWORD, "something else entirely"),
            "The two new passwords do not match.",
        ),
        (
            change(PASSWORD, "too short", "too short"),
            "The new password needs 10 to 128 characters.",
        ),
        (
            change("not the password", NEW_PASSWORD, NEW_PASSWORD),
            "The current password is not correct.",
        ),
    ] {
        let response = app
            .send(post_form("/settings/password", &body, Some(&cookie)))
            .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let html = text(response).await;
        assert!(html.contains(message), "{message}: {html}");
    }
    assert_eq!(
        location(&app.send(get_with("/", &other_device)).await),
        None,
        "still signed in"
    );
    assert_eq!(
        app.attempt("daniele", PASSWORD).await.status(),
        StatusCode::SEE_OTHER
    );
}

#[tokio::test]
async fn guessing_the_current_password_locks_like_signing_in() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let wrong = change("not the password", NEW_PASSWORD, NEW_PASSWORD);
    for _ in 0..5 {
        let response = app
            .send(post_form("/settings/password", &wrong, Some(&cookie)))
            .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    let right = change(PASSWORD, NEW_PASSWORD, NEW_PASSWORD);
    let response = app
        .send(post_form("/settings/password", &right, Some(&cookie)))
        .await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(text(response).await.contains("Too many attempts."));
}
