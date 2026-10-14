//! Settings → Guides: *How to rate* and *How to log* (spec §5).

mod common;

use axum::http::StatusCode;
use common::page::Page;
use common::*;
use smoke_domain::types::{Locale, Role};

#[tokio::test]
async fn the_logging_guide_has_the_table_and_rules_of_thumb() {
    let app = TestApp::new().await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let cookie = app.sign_in("guest").await;
    let response = app.send(get_with("/guide/logging", &cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let page = Page::parse(&text(response).await);
    assert_eq!(page.text("h1"), "How to log");
    assert!(page.texts(".guide tbody tr").contains(
        &"The smell went away completely and came back later Start a new event".to_owned()
    ));
    assert_eq!(page.count(".rules > li"), 5);
    assert!(
        page.text(".rules")
            .contains("Forgetting is fine (04:00 auto-close)"),
        "the configured hour"
    );
    assert_eq!(page.text(r#"a.row[href="/guide/rating"]"#), "How to rate");
    assert_eq!(page.attr("a.back", "href").as_deref(), Some("/settings"));
}

#[tokio::test]
async fn settings_and_the_rating_guide_lead_to_both_guides() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let settings = Page::parse(&text(app.send(get_with("/settings", &cookie)).await).await);
    assert_eq!(
        settings.attrs("section.rows a.row", "href"),
        ["/guide/rating", "/guide/logging"]
    );
    assert_eq!(
        settings.texts("section.rows a.row"),
        ["How to rate", "How to log"]
    );
    let rating = Page::parse(&text(app.send(get_with("/guide/rating", &cookie)).await).await);
    assert_eq!(rating.text(r#"a.row[href="/guide/logging"]"#), "How to log");
    assert_eq!(rating.count(".scale tbody tr"), 11);
}

#[tokio::test]
async fn the_guides_are_translated() {
    let app = TestApp::new().await;
    app.add_user("partner", Role::Resident, Locale::It).await;
    let cookie = app.sign_in("partner").await;
    let html = text(app.send(get_with("/guide/logging", &cookie)).await).await;
    assert!(html.contains("<h1>Come registrare</h1>"), "{html}");
    assert!(html.contains("chiusura automatica alle 04:00"), "{html}");
}
