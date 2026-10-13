//! Signing in and out, the failure limits and session lifetime (spec §8.1, §8.4).

mod common;

use axum::http::StatusCode;
use chrono::TimeDelta;
use common::*;
use smoke_domain::records::PasswordHash;
use smoke_domain::repos::{Commit, Transactional, UserWrites};
use smoke_domain::types::{Locale, Role};

#[tokio::test]
async fn anonymous_visitors_are_sent_to_the_login_page() {
    let app = TestApp::new().await;
    let response = app.send(get("/")).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));

    let login = app.send(get("/login")).await;
    assert_eq!(login.status(), StatusCode::OK);
    let html = text(login).await;
    assert!(html.contains("<h1>Sign in</h1>"), "{html}");
}

#[tokio::test]
async fn htmx_requests_are_redirected_with_hx_redirect() {
    let app = TestApp::new().await;
    let mut request = get("/");
    request
        .headers_mut()
        .insert("hx-request", "true".parse().unwrap());
    let response = app.send(request).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["hx-redirect"], "/login");
}

#[tokio::test]
async fn signing_in_sets_a_host_prefixed_session_cookie() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let response = app.attempt("daniele", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/"));
    let cookie = set_cookies(&response)
        .into_iter()
        .find(|c| c.starts_with("__Host-session="))
        .expect("a __Host- session cookie");
    for flag in [
        "HttpOnly",
        "Secure",
        "SameSite=Lax",
        "Path=/",
        "Max-Age=2592000",
    ] {
        assert!(cookie.contains(flag), "{flag} missing: {cookie}");
    }
    assert!(!cookie.contains("Domain"), "{cookie}");
}

#[tokio::test]
async fn a_signed_in_user_sees_the_home_page() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let response = app.send(get_with("/", &cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert!(html.contains("Signed in as daniele."), "{html}");

    let login = app.send(get_with("/login", &cookie)).await;
    assert_eq!(location(&login), Some("/"));
}

#[tokio::test]
async fn usernames_are_matched_ignoring_case_and_spaces() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let response = app.attempt("  Daniele ", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn unknown_users_and_wrong_passwords_get_the_same_answer() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let wrong_password = app.attempt("daniele", "not the password").await;
    let unknown_user = app.attempt("nobody", "not the password").await;
    let invalid_name = app.attempt("no such user!", "not the password").await;
    assert_eq!(wrong_password.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(unknown_user.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(invalid_name.status(), StatusCode::UNAUTHORIZED);
    let wrong_password = text(wrong_password).await.replace("daniele", "NAME");
    let unknown_user = text(unknown_user).await.replace("nobody", "NAME");
    assert!(wrong_password.contains("Invalid username or password."));
    assert_eq!(wrong_password, unknown_user);
    assert!(
        text(invalid_name)
            .await
            .contains("Invalid username or password.")
    );
}

#[tokio::test]
async fn an_expired_viewer_cannot_sign_in() {
    let app = TestApp::new().await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    app.clock.advance(TimeDelta::days(7));
    let response = app.attempt("lawyer", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(session_cookie(&response).is_none());
    assert!(text(response).await.contains("This account has expired."));

    let wrong = app.attempt("lawyer", "not the password").await;
    assert!(text(wrong).await.contains("Invalid username or password."));
}

#[tokio::test]
async fn an_expired_guest_can_still_sign_in() {
    let app = TestApp::new().await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    app.clock.advance(TimeDelta::days(8));
    let cookie = app.sign_in("guest").await;
    assert_eq!(
        app.send(get_with("/", &cookie)).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn a_viewer_who_expires_mid_session_is_signed_out() {
    let app = TestApp::new().await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let cookie = app.sign_in("lawyer").await;
    assert_eq!(
        app.send(get_with("/", &cookie)).await.status(),
        StatusCode::OK
    );
    app.clock.advance(TimeDelta::days(7));
    let response = app.send(get_with("/", &cookie)).await;
    assert_eq!(location(&response), Some("/login"));
}

#[tokio::test]
async fn signing_out_ends_the_session() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let response = app.send(post_form("/logout", "", Some(&cookie))).await;
    assert_eq!(location(&response), Some("/login"));
    let cleared = set_cookies(&response)
        .into_iter()
        .find(|c| c.starts_with("__Host-session="))
        .expect("the cookie is cleared");
    assert!(cleared.contains("Max-Age=0"), "{cleared}");
    assert_eq!(
        location(&app.send(get_with("/", &cookie)).await),
        Some("/login")
    );
}

#[tokio::test]
async fn every_sign_in_gets_a_fresh_session_id() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let first = app.sign_in("daniele").await;
    let mut again = login_request("daniele", PASSWORD);
    again.headers_mut().insert("cookie", first.parse().unwrap());
    let second = session_cookie(&app.send(again).await).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        location(&app.send(get_with("/", &first)).await),
        Some("/login")
    );
    assert_eq!(
        app.send(get_with("/", &second)).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn an_idle_session_ends_after_thirty_days() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    app.clock
        .advance(TimeDelta::days(30) + TimeDelta::minutes(1));
    assert_eq!(
        location(&app.send(get_with("/", &cookie)).await),
        Some("/login")
    );
}

#[tokio::test]
async fn an_active_session_is_renewed_at_most_daily() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let same_day = app.send(get_with("/", &cookie)).await;
    assert!(session_cookie(&same_day).is_none(), "no write within a day");
    app.clock.advance(TimeDelta::days(1));
    let next_day = app.send(get_with("/", &cookie)).await;
    assert_eq!(session_cookie(&next_day), Some(cookie));
}

#[tokio::test]
async fn a_password_change_signs_the_user_out() {
    let app = TestApp::new().await;
    let user = app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let mut tx = app.store.begin(Some(user.id)).await.unwrap();
    tx.set_password(user.id, PasswordHash::new("$argon2id$other".to_owned()))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        location(&app.send(get_with("/", &cookie)).await),
        Some("/login")
    );
}

#[tokio::test]
async fn five_failures_lock_the_username_for_fifteen_minutes() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    for _ in 0..5 {
        app.attempt("daniele", "not the password").await;
    }
    let locked = app.attempt("daniele", PASSWORD).await;
    assert_eq!(locked.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(text(locked).await.contains("Too many attempts."));

    app.clock.advance(TimeDelta::minutes(15));
    let after = app.attempt("daniele", PASSWORD).await;
    assert_eq!(after.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn changing_the_case_of_the_username_does_not_bypass_the_lock() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    for name in ["daniele", "Daniele", "DANIELE", " daniele", "daniele "] {
        app.attempt(name, "not the password").await;
    }
    for name in ["daniele", "DaNiElE", "  DANIELE  "] {
        let response = app.attempt(name, PASSWORD).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS, "{name:?}");
    }
}

#[tokio::test]
async fn a_success_resets_the_username_failures() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    for _ in 0..4 {
        app.attempt("daniele", "not the password").await;
    }
    app.sign_in("daniele").await;
    for _ in 0..4 {
        app.attempt("daniele", "not the password").await;
    }
    assert_eq!(
        app.attempt("daniele", PASSWORD).await.status(),
        StatusCode::SEE_OTHER
    );
}

#[tokio::test]
async fn one_address_cannot_try_many_usernames() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    for n in 0..20 {
        app.attempt(&format!("guess{n}"), "not the password").await;
    }
    let blocked = app.attempt("daniele", PASSWORD).await;
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);

    let mut elsewhere = login_request("daniele", PASSWORD);
    from(&mut elsewhere, "198.51.100.1".parse().unwrap());
    assert_eq!(app.send(elsewhere).await.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn behind_a_proxy_the_configured_header_names_the_client() {
    let app = TestApp::with_config(BASE, Some("x-forwarded-for")).await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let attempt = |client: &'static str, username: &str, password: &str| {
        let mut request = login_request(username, password);
        request
            .headers_mut()
            .insert("x-forwarded-for", client.parse().unwrap());
        request
    };
    for n in 0..20 {
        app.send(attempt(
            "203.0.113.1",
            &format!("guess{n}"),
            "not the password",
        ))
        .await;
    }
    let blocked = app.send(attempt("203.0.113.1", "daniele", PASSWORD)).await;
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    let other_client = app.send(attempt("203.0.113.2", "daniele", PASSWORD)).await;
    assert_eq!(other_client.status(), StatusCode::SEE_OTHER);
}
