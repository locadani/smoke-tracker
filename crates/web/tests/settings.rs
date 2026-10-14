//! Settings: language and password for everyone (spec §9 screen 7).

mod common;

use axum::http::StatusCode;
use common::page::Page;
use common::*;
use smoke_domain::repos::{ListRepo, UserRepo};
use smoke_domain::types::{Locale, Role, RoomArea};

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
    let page = Page::parse(&html);
    assert_eq!(page.text("h1"), "Impostazioni");
    assert_eq!(
        page.text(r#"form[action="/lang"] button[aria-current=true]"#),
        "Italiano"
    );
    assert_eq!(page.count(r#"form[action="/lang"] button[name=lang]"#), 3);
    assert!(page.exists(r#"form[action="/settings/password"]"#));
    assert_eq!(
        page.text(r#"nav.tabbar a[aria-current=page]"#),
        "Impostazioni"
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

#[tokio::test]
async fn residents_add_rooms_and_symptoms_and_hide_them_from_the_forms() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;

    for (path, body) in [
        ("/settings/rooms", "name=Balcony&area=common"),
        ("/settings/symptoms", "name=Dizziness"),
    ] {
        let response = app.send(post_form(path, body, Some(&cookie))).await;
        assert_eq!(location(&response), Some("/settings?saved=lists"), "{path}");
    }
    let rooms = app.store.rooms().await.unwrap();
    let balcony = rooms.last().unwrap();
    assert_eq!(
        (
            balcony.name.as_str(),
            balcony.area,
            balcony.i18n_key.as_deref()
        ),
        ("Balcony", RoomArea::Common, None)
    );
    let log = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        Page::parse(&log)
            .texts(".room-check")
            .contains(&"Balcony".to_owned()),
        "{log}"
    );
    assert!(log.contains("Dizziness"), "{log}");

    let hide = format!("/settings/rooms/{}/hidden", balcony.id);
    let response = app
        .send(post_form(&hide, "hidden=true", Some(&cookie)))
        .await;
    assert_eq!(location(&response), Some("/settings?saved=lists"));
    let log = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!log.contains("Balcony"), "{log}");
    let settings = text(app.send(get_with("/settings?saved=lists", &cookie)).await).await;
    let page = Page::parse(&settings);
    assert_eq!(page.text(".notice[role=status]"), "Lists saved.");
    let balcony_row = format!(r#".items form[action="{hide}"]"#);
    assert_eq!(page.text(&format!("{balcony_row} s")), "Balcony");
    assert_eq!(
        page.text(&format!("{balcony_row} .row-text")),
        "Balcony hidden Common areas"
    );
    assert_eq!(page.text(&format!("{balcony_row} button")), "Show again");
    assert!(page.exists(&format!("{balcony_row} input[name=hidden][value=false]")));

    app.send(post_form(&hide, "hidden=false", Some(&cookie)))
        .await;
    let log = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        Page::parse(&log)
            .texts(".room-check")
            .contains(&"Balcony".to_owned()),
        "{log}"
    );
}

#[tokio::test]
async fn a_name_must_have_one_to_sixty_characters() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let long = format!("name={}", "x".repeat(61));
    for (path, body) in [
        ("/settings/rooms", "name=+++&area=inside"),
        ("/settings/symptoms", long.as_str()),
    ] {
        let response = app.send(post_form(path, body, Some(&cookie))).await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{path}"
        );
        let html = text(response).await;
        assert!(html.contains("A name needs 1 to 60 characters."), "{html}");
    }
    assert_eq!(app.store.rooms().await.unwrap().len(), 6);
    assert_eq!(app.store.symptoms().await.unwrap().len(), 6);
}

#[tokio::test]
async fn only_residents_see_and_edit_the_lists() {
    let app = TestApp::new().await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    for name in ["guest", "lawyer"] {
        let cookie = app.sign_in(name).await;
        let html = text(app.send(get_with("/settings", &cookie)).await).await;
        assert!(!html.contains("/settings/rooms"), "{name}: {html}");
        for (path, body) in [
            ("/settings/rooms", "name=Balcony&area=common"),
            ("/settings/rooms/1/hidden", "hidden=true"),
            ("/settings/symptoms", "name=Dizziness"),
            ("/settings/symptoms/1/hidden", "hidden=true"),
        ] {
            let response = app.send(post_form(path, body, Some(&cookie))).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{name} {path}");
        }
    }
    assert_eq!(app.store.rooms().await.unwrap().len(), 6);
    assert!(app.store.rooms().await.unwrap()[0].hidden_at.is_none());
}

#[tokio::test]
async fn hiding_an_unknown_item_is_not_found() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let response = app
        .send(post_form(
            "/settings/rooms/99/hidden",
            "hidden=true",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
