//! Which language a page is in (spec §9.2).

mod common;

use axum::body::Body;
use axum::http::Request;
use common::*;
use smoke_domain::types::{Locale, Role};

async fn heading(app: &TestApp, request: Request<Body>) -> String {
    let html = text(app.send(request).await).await;
    let start = html.find("<h1>").unwrap() + 4;
    html[start..start + html[start..].find("</h1>").unwrap()].to_owned()
}

#[tokio::test]
async fn before_sign_in_the_lang_cookie_then_the_browser_decide() {
    let app = TestApp::new().await;
    assert_eq!(heading(&app, get("/login")).await, "Sign in");
    assert_eq!(heading(&app, get_with("/login", "lang=it")).await, "Accedi");
    let browser = Request::get("/login")
        .header("accept-language", "tr-TR,tr;q=0.9,en;q=0.8")
        .body(Body::empty())
        .unwrap();
    assert_eq!(heading(&app, browser).await, "Giriş yap");
    let unknown_cookie = Request::get("/login")
        .header("cookie", "lang=fr")
        .header("accept-language", "it-IT")
        .body(Body::empty())
        .unwrap();
    assert_eq!(heading(&app, unknown_cookie).await, "Accedi");
}

#[tokio::test]
async fn once_signed_in_the_account_language_wins_over_the_cookie() {
    let app = TestApp::new().await;
    app.add_user("partner", Role::Resident, Locale::It).await;
    let response = app.attempt("partner", PASSWORD).await;
    assert!(
        set_cookies(&response)
            .iter()
            .any(|c| c.starts_with("lang=it;")),
        "sign-in aligns the cookie with the account"
    );
    let session = session_cookie(&response).unwrap();
    let cookies = format!("{session}; lang=tr");
    assert_eq!(heading(&app, get_with("/", &cookies)).await, "Home");
    let html = text(app.send(get_with("/", &cookies)).await).await;
    assert!(html.contains(r#"<html lang="it">"#), "{html}");
    assert!(html.contains("Accesso effettuato come partner."), "{html}");
}
