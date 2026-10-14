//! Settings → Guides: *How to rate* and *How to log* (spec §5).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::types::{Locale, Role};

#[tokio::test]
async fn the_logging_guide_has_the_table_and_rules_of_thumb() {
    let app = TestApp::new().await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let cookie = app.sign_in("guest").await;
    let response = app.send(get_with("/guide/logging", &cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert!(html.contains("<h1>How to log</h1>"), "{html}");
    assert!(
        html.contains(
            "<tr><td>The smell went away completely and came back later</td>\
             <td>Start a new event</td></tr>"
        ),
        "{html}"
    );
    assert_eq!(html.matches("<li>").count(), 5, "{html}");
    assert!(
        html.contains("Forgetting is fine (04:00 auto-close)"),
        "the configured hour: {html}"
    );
    assert!(html.contains(r#"<a href="/guide/rating">How to rate</a>"#));
}

#[tokio::test]
async fn settings_and_the_rating_guide_lead_to_both_guides() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let settings = text(app.send(get_with("/settings", &cookie)).await).await;
    for link in [
        r#"<a href="/guide/rating">How to rate</a>"#,
        r#"<a href="/guide/logging">How to log</a>"#,
    ] {
        assert!(settings.contains(link), "{link}: {settings}");
    }
    let rating = text(app.send(get_with("/guide/rating", &cookie)).await).await;
    assert!(
        rating.contains(r#"<a href="/guide/logging">How to log</a>"#),
        "{rating}"
    );
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
