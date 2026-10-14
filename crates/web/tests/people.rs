//! People (spec §9 screen 6): residents see everyone and end or restore their access.

mod common;

use axum::http::StatusCode;
use common::page::Page;
use common::*;
use smoke_domain::clock::Clock;
use smoke_domain::repos::UserRepo;
use smoke_domain::types::{Locale, Role};

#[tokio::test]
async fn only_residents_reach_people_and_see_it_in_the_menu() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    let home = text(app.send(get_with("/", &resident)).await).await;
    assert_eq!(
        Page::parse(&home).text(r#"nav.tabbar a[href="/people"]"#),
        "People"
    );
    for name in ["guest", "lawyer"] {
        let cookie = app.sign_in(name).await;
        let home = text(app.send(get_with("/", &cookie)).await).await;
        assert!(!home.contains(r#"href="/people""#), "{name}: {home}");
        let page = app.send(get_with("/people", &cookie)).await;
        assert_eq!(page.status(), StatusCode::FORBIDDEN, "{name}");
        for path in ["/people/1/expire", "/people/2/reactivate"] {
            let response = app.send(post_form(path, "expiry=1w", Some(&cookie))).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{name} {path}");
        }
    }
}

#[tokio::test]
async fn people_lists_everyone_with_their_role_and_expiry() {
    let app = TestApp::new().await;
    app.at("2026-01-15T19:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with("/people", &cookie)).await).await;
    assert!(
        html.contains("<strong>daniele</strong> · resident · no end date"),
        "{html}"
    );
    assert!(
        html.contains("<strong>guest</strong> · guest · until 22/01/2026 20:00"),
        "{html}"
    );
    assert!(html.contains(r#"action="/people/2/expire""#), "{html}");
    assert!(!html.contains(r#"action="/people/1/expire""#), "{html}");
}

#[tokio::test]
async fn ending_a_guests_access_signs_them_out_and_leaves_them_read_only() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    let guest_cookie = app.sign_in("guest").await;

    let path = format!("/people/{}/expire", guest.id);
    let response = app.send(post_form(&path, "", Some(&resident))).await;
    assert_eq!(location(&response), Some("/people?saved=expired"));
    let expired = app.store.user(guest.id).await.unwrap().unwrap();
    assert!(expired.actor().is_expired(app.clock.now()));

    let page = app.send(get_with("/", &guest_cookie)).await;
    assert_eq!(location(&page), Some("/login"), "signed out");
    let again = app.sign_in("guest").await;
    let home = text(app.send(get_with("/", &again)).await).await;
    assert!(
        home.contains("You can look at the data but not log."),
        "{home}"
    );

    let html = text(app.send(get_with("/people?saved=expired", &resident)).await).await;
    assert!(html.contains("Access ended."), "{html}");
    assert!(html.contains(r#"action="/people/2/reactivate""#), "{html}");
}

#[tokio::test]
async fn an_expired_viewer_can_be_reactivated_without_an_end() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let viewer = app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    let path = format!("/people/{}/expire", viewer.id);
    app.send(post_form(&path, "", Some(&resident))).await;
    assert_eq!(
        app.attempt("lawyer", PASSWORD).await.status(),
        StatusCode::FORBIDDEN
    );

    let path = format!("/people/{}/reactivate", viewer.id);
    let response = app
        .send(post_form(&path, "expiry=never", Some(&resident)))
        .await;
    assert_eq!(location(&response), Some("/people?saved=reactivated"));
    let back = app.store.user(viewer.id).await.unwrap().unwrap();
    assert_eq!(back.expires_at, None);
    assert_eq!(
        app.attempt("lawyer", PASSWORD).await.status(),
        StatusCode::SEE_OTHER,
        "the old password still works"
    );
}

#[tokio::test]
async fn a_guest_is_reactivated_only_with_a_future_end() {
    let app = TestApp::new().await;
    app.at("2026-01-15T19:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    let reactivate = format!("/people/{}/reactivate", guest.id);

    let response = app
        .send(post_form(&reactivate, "expiry=1w", Some(&resident)))
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        text(response)
            .await
            .contains("This account has not expired.")
    );

    let expire = format!("/people/{}/expire", guest.id);
    app.send(post_form(&expire, "", Some(&resident))).await;
    for (body, message) in [
        ("expiry=never", "Pick how long a guest may stay."),
        (
            "expiry=date&expiry_date=2026-01-14",
            "The end must be in the future.",
        ),
        ("expiry=date&expiry_date=soon", "Enter a valid date."),
    ] {
        let response = app
            .send(post_form(&reactivate, body, Some(&resident)))
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
            &reactivate,
            "expiry=date&expiry_date=2026-01-31",
            Some(&resident),
        ))
        .await;
    assert_eq!(location(&response), Some("/people?saved=reactivated"));
    let back = app.store.user(guest.id).await.unwrap().unwrap();
    assert_eq!(back.expires_at, Some(t("2026-01-31T23:00:00Z")));
}

#[tokio::test]
async fn a_resident_cannot_be_expired() {
    let app = TestApp::new().await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let path = format!("/people/{}/expire", partner.id);
    let response = app.send(post_form(&path, "", Some(&cookie))).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text(response).await.contains("Residents never expire."));
    let missing = app
        .send(post_form("/people/99/expire", "", Some(&cookie)))
        .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn reactivating_with_a_typed_date_but_another_pick_is_refused() {
    let app = TestApp::new().await;
    app.at("2026-01-15T19:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let viewer = app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    app.send(post_form(
        &format!("/people/{}/expire", viewer.id),
        "",
        Some(&resident),
    ))
    .await;
    let expired = app.store.user(viewer.id).await.unwrap().unwrap();
    let path = format!("/people/{}/reactivate", viewer.id);
    for body in [
        "expiry=never&expiry_date=2026-02-01",
        "expiry_date=2026-02-01",
    ] {
        let response = app.send(post_form(&path, body, Some(&resident))).await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{body}"
        );
        let html = text(response).await;
        assert!(html.contains("Choose “at the end of this day”"), "{html}");
        assert!(html.contains("2026-02-01"), "{html}");
    }
    let after = app.store.user(viewer.id).await.unwrap().unwrap();
    assert_eq!(after.expires_at, expired.expires_at, "nothing written");
}
