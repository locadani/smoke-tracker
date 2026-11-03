//! Deleting an entry from its page (spec 2026-11-03 "Screens", "After deleting").

mod common;

use axum::http::StatusCode;
use common::page::Page;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::types::{Locale, Role};

const NEW: EventChoice = EventChoice::New { replacing: None };

#[tokio::test]
async fn the_author_deletes_an_entry_and_lands_on_its_event() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let first = app.log(&me, "2026-01-15T18:00:00Z", 4, &[], NEW).await;
    let second = app
        .log(
            &me,
            "2026-01-15T18:30:00Z",
            5,
            &[],
            EventChoice::Join(first.report.event_id.unwrap()),
        )
        .await;
    let cookie = app.sign_in("daniele").await;

    let response = app
        .send(post_form(
            &format!("/entries/{}/delete", second.report.entry_id),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        location(&response),
        Some(format!("/events/{}", first.report.event_id.unwrap()).as_str())
    );
    let gone = app
        .send(get_with(
            &format!("/entries/{}", second.report.entry_id),
            &cookie,
        ))
        .await;
    assert_eq!(gone.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn deleting_the_last_entry_lands_on_history() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let only = app.log(&me, "2026-01-15T18:00:00Z", 4, &[], NEW).await;
    let cookie = app.sign_in("daniele").await;

    let response = app
        .send(post_form(
            &format!("/entries/{}/delete", only.report.entry_id),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/events"));
}

#[tokio::test]
async fn a_reading_only_entry_offers_delete() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let reading = app.log_pm25(&me, "2026-01-15T18:00:00Z", 12.0).await;
    let cookie = app.sign_in("daniele").await;

    let html = text(
        app.send(get_with(&format!("/entries/{}", reading.entry_id), &cookie))
            .await,
    )
    .await;
    let page = Page::parse(&html);
    assert_eq!(page.attrs("a[data-variant=danger]", "href"), ["?delete"]);
    assert_eq!(page.texts("a[data-variant=danger]"), ["Elimina"]);
}

#[tokio::test]
async fn the_confirm_step_shows_question_and_cancel() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let logged = app.log(&me, "2026-01-15T18:00:00Z", 4, &[], NEW).await;
    let id = logged.report.entry_id;
    let cookie = app.sign_in("daniele").await;

    let html = text(
        app.send(get_with(&format!("/entries/{id}?delete"), &cookie))
            .await,
    )
    .await;
    let page = Page::parse(&html);
    assert!(html.contains("Eliminare questa registrazione?"), "{html}");
    assert!(html.contains("Non si può annullare."), "{html}");
    assert_eq!(
        page.attrs("form[action$='/delete']", "action"),
        [format!("/entries/{id}/delete")]
    );
    assert_eq!(
        page.attrs(
            "a[data-variant=secondary][href='/entries/{id}']"
                .replace("{id}", &id.to_string())
                .as_str(),
            "href"
        ),
        [format!("/entries/{id}")]
    );
    assert_eq!(page.texts("button[data-variant=danger]"), ["Elimina"]);

    let plain = text(app.send(get_with(&format!("/entries/{id}"), &cookie)).await).await;
    assert!(!plain.contains("Non si può annullare."), "{plain}");
}

#[tokio::test]
async fn a_resident_may_delete_a_guests_entry_but_sees_no_button_on_another_residents() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::It).await;
    let other = app.add_user("marta", Role::Resident, Locale::It).await;
    let guest = app.add_user("ospite", Role::Guest, Locale::It).await;
    let theirs = app.log(&other, "2026-01-15T17:00:00Z", 4, &[], NEW).await;
    let replacing = theirs.report.event_id;
    let guests = app
        .log(
            &guest,
            "2026-01-15T18:00:00Z",
            4,
            &[],
            EventChoice::New { replacing },
        )
        .await;
    let cookie = app.sign_in("daniele").await;

    let html = text(
        app.send(get_with(
            &format!("/entries/{}", theirs.report.entry_id),
            &cookie,
        ))
        .await,
    )
    .await;
    assert!(
        !Page::parse(&html).exists("a[data-variant=danger]"),
        "{html}"
    );
    let refused = app
        .send(post_form(
            &format!("/entries/{}/delete", theirs.report.entry_id),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);

    let html = text(
        app.send(get_with(
            &format!("/entries/{}", guests.report.entry_id),
            &cookie,
        ))
        .await,
    )
    .await;
    assert!(
        Page::parse(&html).exists("a[data-variant=danger]"),
        "{html}"
    );
}

#[tokio::test]
async fn a_viewer_and_an_expired_guest_see_no_button_and_are_refused() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    app.add_user("vedo", Role::Viewer, Locale::It).await;
    let guest = app.add_user("ospite", Role::Guest, Locale::It).await;
    let mine = app.log(&me, "2026-01-15T18:00:00Z", 4, &[], NEW).await;
    let theirs = app
        .log(
            &guest,
            "2026-01-15T19:00:00Z",
            4,
            &[],
            EventChoice::New {
                replacing: mine.report.event_id,
            },
        )
        .await;
    let viewer = app.sign_in("vedo").await;
    let guest_cookie = app.sign_in("ospite").await;

    let html = text(
        app.send(get_with(
            &format!("/entries/{}", mine.report.entry_id),
            &viewer,
        ))
        .await,
    )
    .await;
    assert!(
        !Page::parse(&html).exists("a[data-variant=danger]"),
        "{html}"
    );
    let refused = app
        .send(post_form(
            &format!("/entries/{}/delete", mine.report.entry_id),
            "",
            Some(&viewer),
        ))
        .await;
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);

    app.at("2026-02-15T20:00:00Z");
    let refused = app
        .send(post_form(
            &format!("/entries/{}/delete", theirs.report.entry_id),
            "",
            Some(&guest_cookie),
        ))
        .await;
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_guest_deleting_in_a_residents_event_lands_on_history() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let guest = app.add_user("ospite", Role::Guest, Locale::It).await;
    let first = app.log(&me, "2026-01-15T18:00:00Z", 4, &[], NEW).await;
    let theirs = app
        .log(
            &guest,
            "2026-01-15T18:30:00Z",
            5,
            &[],
            EventChoice::Join(first.report.event_id.unwrap()),
        )
        .await;
    let cookie = app.sign_in("ospite").await;

    let response = app
        .send(post_form(
            &format!("/entries/{}/delete", theirs.report.entry_id),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/events"));
}

#[tokio::test]
async fn deleting_twice_is_not_found() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let only = app.log(&me, "2026-01-15T18:00:00Z", 4, &[], NEW).await;
    let cookie = app.sign_in("daniele").await;
    let path = format!("/entries/{}/delete", only.report.entry_id);

    app.send(post_form(&path, "", Some(&cookie))).await;
    let again = app.send(post_form(&path, "", Some(&cookie))).await;
    assert_eq!(again.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_guest_deleting_a_residents_entry_is_not_found() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    app.add_user("ospite", Role::Guest, Locale::It).await;
    let mine = app.log(&me, "2026-01-15T18:00:00Z", 4, &[], NEW).await;
    let cookie = app.sign_in("ospite").await;

    let response = app
        .send(post_form(
            &format!("/entries/{}/delete", mine.report.entry_id),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
