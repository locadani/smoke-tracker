//! Invites and password resets with a QR code (spec §8.3).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::records::Invite;
use smoke_domain::repos::UserRepo;
use smoke_domain::types::{Locale, Role, Username};

const CHOSEN: &str = "my own long passphrase";

/// The token in the link a QR page shows.
fn token(page: &str) -> String {
    let start = page.find("/invite/").expect("a link") + "/invite/".len();
    page[start..start + 64].to_owned()
}

fn choose(password: &str, repeat: &str, lang: &str) -> String {
    format!("password={password}&repeat={repeat}&lang={lang}").replace(' ', "+")
}

async fn invite_guest(app: &TestApp, cookie: &str, name: &str) -> String {
    let body = format!("username={name}&role=guest&guest_expiry=1w");
    let response = app
        .send(post_form("/people/invite", &body, Some(cookie)))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    text(response).await
}

#[tokio::test]
async fn an_invited_guest_scans_sets_a_password_and_is_signed_in() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    let form = text(app.send(get_with("/people/invite", &resident)).await).await;
    assert!(
        form.contains(r#"name="guest_expiry" value="1w" checked"#),
        "{form}"
    );

    let page = invite_guest(&app, &resident, "Friend").await;
    assert!(page.contains("<h1>Setup link for friend</h1>"), "{page}");
    assert!(page.contains(r#"<svg class="qr""#), "{page}");
    let token = token(&page);
    assert!(token.bytes().all(|b| b.is_ascii_hexdigit()), "{token}");
    assert!(page.contains(&format!("{BASE}/invite/{token}")), "{page}");
    let friend = Username::new("friend").unwrap();
    let invited = app.store.user_by_username(&friend).await.unwrap().unwrap();
    assert_eq!(invited.password_hash, None);
    assert_eq!(invited.role, Role::Guest);

    let link = format!("/invite/{token}");
    let welcome = text(app.send(get(&link)).await).await;
    assert!(welcome.contains("<h1>Welcome, friend</h1>"), "{welcome}");
    let response = app
        .send(post_form(&link, &choose(CHOSEN, CHOSEN, "it"), None))
        .await;
    assert_eq!(location(&response), Some("/"));
    let cookie = session_cookie(&response).expect("signed in");
    let home = text(app.send(get_with("/", &cookie)).await).await;
    assert!(home.contains("Accesso effettuato come friend."), "{home}");

    assert_eq!(app.send(get(&link)).await.status(), StatusCode::GONE);
    let again = app
        .send(post_form(&link, &choose(CHOSEN, CHOSEN, "en"), None))
        .await;
    assert_eq!(again.status(), StatusCode::GONE);
    assert!(text(again).await.contains("This link no longer works"));
    let response = app.send(login_request("friend", CHOSEN)).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn a_link_works_for_five_minutes_only() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    let link = format!(
        "/invite/{}",
        token(&invite_guest(&app, &resident, "friend").await)
    );
    app.clock.advance(Invite::LIFETIME);
    assert_eq!(app.send(get(&link)).await.status(), StatusCode::GONE);
    let response = app
        .send(post_form(&link, &choose(CHOSEN, CHOSEN, "en"), None))
        .await;
    assert_eq!(response.status(), StatusCode::GONE);
    assert_eq!(
        app.send(get("/invite/not-a-token")).await.status(),
        StatusCode::GONE
    );
}

#[tokio::test]
async fn a_reset_link_replaces_the_password_and_signs_out_every_device() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    let partner_phone = app.sign_in("partner").await;

    let path = format!("/people/{}/link", partner.id);
    let page = text(app.send(post_form(&path, "", Some(&resident))).await).await;
    assert!(
        page.contains("<h1>Password reset for partner</h1>"),
        "{page}"
    );
    let link = format!("/invite/{}", token(&page));
    let form = text(app.send(get(&link)).await).await;
    assert!(form.contains("<h1>Choose a new password</h1>"), "{form}");

    let response = app
        .send(post_form(&link, &choose(CHOSEN, CHOSEN, "en"), None))
        .await;
    assert_eq!(location(&response), Some("/"));
    assert_eq!(
        location(&app.send(get_with("/", &partner_phone)).await),
        Some("/login")
    );
    assert_eq!(
        app.attempt("partner", PASSWORD).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let response = app.send(login_request("partner", CHOSEN)).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn a_newer_link_replaces_the_one_shown_before() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    let first = token(&invite_guest(&app, &resident, "friend").await);
    let friend = Username::new("friend").unwrap();
    let invited = app.store.user_by_username(&friend).await.unwrap().unwrap();
    let path = format!("/people/{}/link", invited.id);
    let page = text(app.send(post_form(&path, "", Some(&resident))).await).await;
    assert!(page.contains("<h1>Setup link for friend</h1>"), "{page}");
    assert_eq!(
        app.send(get(&format!("/invite/{first}"))).await.status(),
        StatusCode::GONE
    );
    let second = format!("/invite/{}", token(&page));
    assert_eq!(app.send(get(&second)).await.status(), StatusCode::OK);
}

#[tokio::test]
async fn the_invite_form_says_what_to_fix_and_keeps_what_was_typed() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    for (body, message) in [
        (
            "username=no+spaces&role=guest&guest_expiry=1w",
            "A username needs 3 to 32 characters",
        ),
        ("username=daniele&role=viewer", "That username is taken."),
        (
            "username=friend&role=guest&guest_expiry=date",
            "Enter a valid date.",
        ),
    ] {
        let response = app
            .send(post_form("/people/invite", body, Some(&cookie)))
            .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{body}"
        );
        let html = text(response).await;
        assert!(html.contains(message), "{body}: {html}");
    }
    let response = app
        .send(post_form(
            "/people/invite",
            "username=friend&role=viewer",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let friend = Username::new("friend").unwrap();
    let viewer = app.store.user_by_username(&friend).await.unwrap().unwrap();
    assert_eq!((viewer.role, viewer.expires_at), (Role::Viewer, None));
    assert_eq!(app.store.users().await.unwrap().len(), 2);
}

#[tokio::test]
async fn the_chosen_password_must_be_valid_and_repeated() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    let link = format!(
        "/invite/{}",
        token(&invite_guest(&app, &resident, "friend").await)
    );
    for (body, message) in [
        (
            choose(CHOSEN, "something else", "en"),
            "The two new passwords do not match.",
        ),
        (
            choose("short", "short", "en"),
            "The new password needs 10 to 128 characters.",
        ),
    ] {
        let response = app.send(post_form(&link, &body, None)).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(text(response).await.contains(message), "{message}");
    }
    assert_eq!(
        app.send(get(&link)).await.status(),
        StatusCode::OK,
        "still usable"
    );
}

#[tokio::test]
async fn only_residents_create_links() {
    let app = TestApp::new().await;
    let resident = app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let cookie = app.sign_in("guest").await;
    let form = app.send(get_with("/people/invite", &cookie)).await;
    assert_eq!(form.status(), StatusCode::FORBIDDEN);
    let body = "username=friend&role=guest&guest_expiry=1w";
    let create = app
        .send(post_form("/people/invite", body, Some(&cookie)))
        .await;
    assert_eq!(create.status(), StatusCode::FORBIDDEN);
    let path = format!("/people/{}/link", resident.id);
    let reset = app.send(post_form(&path, "", Some(&cookie))).await;
    assert_eq!(reset.status(), StatusCode::FORBIDDEN);
    assert_eq!(app.store.users().await.unwrap().len(), 2);
}
