//! The installable app (spec §9.1): manifest, service worker, offline page and install page.

mod common;

use axum::http::StatusCode;
use common::page::Page;
use common::*;
use smoke_domain::types::{Locale, Role};

#[tokio::test]
async fn every_page_links_the_manifest() {
    let app = TestApp::new().await;
    for html in [
        text(app.send(get("/login")).await).await,
        text(app.send(get("/missing")).await).await,
    ] {
        assert!(
            html.contains(r#"<link rel="manifest" href="/static/manifest.webmanifest">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<link rel="apple-touch-icon" href="/static/icon-180.png">"#),
            "{html}"
        );
    }
    let manifest = app.send(get("/static/manifest.webmanifest")).await;
    assert_eq!(
        manifest.headers()["content-type"],
        "application/manifest+json"
    );
}

#[tokio::test]
async fn settings_offers_the_install_card_instead_of_the_header() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let home = Page::parse(&text(app.send(get_with("/", &cookie)).await).await);
    assert!(
        !home.exists("[data-install]"),
        "no install link outside Settings"
    );
    let settings = Page::parse(&text(app.send(get_with("/settings", &cookie)).await).await);
    assert_eq!(settings.text(".install strong"), "Install app");
    assert_eq!(
        settings
            .attr(".install a.button[data-install]", "href")
            .as_deref(),
        Some("/install")
    );
    let first_card = settings.attrs("main > section", "class");
    assert_eq!(first_card.first().map(String::as_str), Some("card install"));
}

#[tokio::test]
async fn the_service_worker_is_served_at_the_root() {
    let app = TestApp::new().await;
    let response = app.send(get("/sw.js")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/javascript; charset=utf-8"
    );
    assert_eq!(response.headers()["cache-control"], "no-cache");
    let script = text(response).await;
    assert!(script.contains("caches.match(\"/offline\")"), "{script}");
}

/// The service worker's install fails as a whole if one precached URL does not answer 200.
#[tokio::test]
async fn everything_the_service_worker_precaches_exists() {
    let app = TestApp::new().await;
    let script = text(app.send(get("/sw.js")).await).await;
    let list = &script[script.find("PRECACHE = [").unwrap()..];
    let list = &list[..list.find("];").unwrap()];
    let paths: Vec<&str> = list.split('"').skip(1).step_by(2).collect();
    assert!(paths.contains(&"/offline"), "{paths:?}");
    for path in paths {
        assert_eq!(app.send(get(path)).await.status(), StatusCode::OK, "{path}");
    }
}

#[tokio::test]
async fn the_offline_page_needs_no_session() {
    let app = TestApp::new().await;
    let html = text(app.send(get("/offline")).await).await;
    assert!(html.contains("<h1>No connection</h1>"), "{html}");
}

#[tokio::test]
async fn the_install_page_offers_the_button_and_the_iphone_steps() {
    let app = TestApp::new().await;
    let html = text(app.send(get("/install")).await).await;
    assert!(
        html.contains(r#"<button type="button" data-install hidden>Install</button>"#),
        "{html}"
    );
    assert!(html.contains("Tap the Share button"), "{html}");
    assert_eq!(html.matches("<svg").count(), 2, "{html}");
    assert!(!html.contains("Your account is ready."), "{html}");

    app.add_user("guest", Role::Guest, Locale::Tr).await;
    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/install?welcome", &cookie)).await).await;
    assert!(html.contains("Hesabınız hazır."), "{html}");
    assert!(
        Page::parse(&html).exists(r#"nav.tabbar a[href="/settings"][aria-current=page]"#),
        "signed in: {html}"
    );
}
