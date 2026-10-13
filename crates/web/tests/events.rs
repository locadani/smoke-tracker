//! The Events list and an event's page: timeline, large-gap warning, close and reopen
//! (spec §7.3, §7.7, §9 screen 3).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::repos::EventRepo;
use smoke_domain::types::{CloseReason, Locale, Role};

#[tokio::test]
async fn the_list_groups_a_months_events_by_day_newest_first() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let first = app
        .log(&me, "2026-01-10T18:00:00Z", 3, &[bedroom], EventChoice::New)
        .await;
    let join = EventChoice::Join(first.report.event_id);
    app.log(&me, "2026-01-10T19:00:00Z", 7, &[], join).await;
    let second = app
        .log(&me, "2026-01-12T07:30:00Z", 5, &[], EventChoice::New)
        .await;
    app.log_pm25(&me, "2026-01-11T13:00:00Z", 12.5).await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
    assert!(html.contains("<strong>01/2026</strong>"), "{html}");
    assert!(html.contains(r#"href="/events?month=2025-12""#), "{html}");
    assert!(html.contains(r#"href="/events?month=2026-02""#), "{html}");
    let day12 = html.find("<h2>12/01/2026</h2>").unwrap();
    let day11 = html.find("<h2>11/01/2026</h2>").unwrap();
    let day10 = html.find("<h2>10/01/2026</h2>").unwrap();
    assert!(day12 < day11 && day11 < day10, "newest first: {html}");
    assert!(
        html.contains(&format!(
            r#"<a href="/events/{}">19:00–20:00</a> · 2 reports · max 7"#,
            first.report.event_id
        )),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            r#"<a href="/events/{}">08:30–"#,
            second.report.event_id
        )),
        "{html}"
    );
    assert!(html.contains("14:00 · PM2.5 12.5 µg/m³"), "{html}");

    let empty = text(app.send(get_with("/events?month=2025-11", &cookie)).await).await;
    assert!(empty.contains("No events this month."), "{empty}");
    let default = text(app.send(get_with("/events?month=junk", &cookie)).await).await;
    assert!(default.contains("<strong>01/2026</strong>"), "{default}");
    for far in ["262142-12", "-262143-01", "0001-01"] {
        let uri = format!("/events?month={far}");
        let html = text(app.send(get_with(&uri, &cookie)).await).await;
        assert!(html.contains("<strong>01/2026</strong>"), "{far}: {html}");
    }
}

#[tokio::test]
async fn guests_list_only_events_they_own_or_reported_in() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let private = app
        .log(&me, "2026-01-10T18:00:00Z", 8, &[], EventChoice::New)
        .await;
    let shared = app
        .log(&me, "2026-01-11T18:00:00Z", 9, &[], EventChoice::New)
        .await;
    let join = EventChoice::Join(shared.report.event_id);
    app.log(&guest, "2026-01-11T18:30:00Z", 4, &[], join).await;
    let cookie = app.sign_in("guest").await;

    let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
    assert!(
        !html.contains(&format!("/events/{}", private.report.event_id)),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            r#"<a href="/events/{}">19:00–19:30</a> · 1 report · max 4"#,
            shared.report.event_id
        )),
        "only the guest's own report counts: {html}"
    );
}

#[tokio::test]
async fn the_event_page_shows_the_timeline_and_warns_about_large_gaps() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let first = app
        .log(&me, "2026-01-15T09:00:00Z", 3, &[bedroom], EventChoice::New)
        .await;
    let id = first.report.event_id;
    app.log(
        &guest,
        "2026-01-15T10:00:00Z",
        5,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let late = app
        .log(&me, "2026-01-15T14:00:00Z", 6, &[], EventChoice::Join(id))
        .await;
    app.log_pm25(&me, "2026-01-15T10:30:00Z", 30.0).await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(html.contains("<h1>Event of 15/01/2026</h1>"), "{html}");
    assert!(html.contains("Started by daniele"), "{html}");
    assert!(
        html.contains(&format!(
            r#"<a href="/reports/{}">intensity 3 · Bedroom</a>"#,
            first.report.id
        )),
        "{html}"
    );
    assert!(html.contains("<span class=\"by\">(guest)</span>"), "{html}");
    let reading = html.find("PM2.5 30 µg/m³").unwrap();
    let gap = html
        .find("More than 3 hours since the previous report")
        .unwrap();
    let after_gap = html
        .find(&format!(r#"<a href="/reports/{}">"#, late.report.id))
        .unwrap();
    assert!(reading < gap && gap < after_gap, "{html}");
    assert_eq!(html.matches("More than 3 hours").count(), 1);
}

#[tokio::test]
async fn guests_see_only_their_own_entries_and_not_other_peoples_events() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let kitchen = app.room("room-kitchen").await;
    let shared = app
        .log(&me, "2026-01-15T09:00:00Z", 9, &[kitchen], EventChoice::New)
        .await;
    let id = shared.report.event_id;
    app.log(
        &guest,
        "2026-01-15T09:30:00Z",
        4,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let private = app
        .log(&me, "2026-01-15T19:00:00Z", 8, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("guest").await;

    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(html.contains("intensity 4"), "{html}");
    for leak in ["intensity 9", "Kitchen", "daniele", "Started by"] {
        assert!(!html.contains(leak), "{leak} leaked: {html}");
    }
    let other = app
        .send(get_with(
            &format!("/events/{}", private.report.event_id),
            &cookie,
        ))
        .await;
    assert_eq!(other.status(), StatusCode::NOT_FOUND);
    let missing = app.send(get_with("/events/999", &cookie)).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn closing_by_hand_needs_an_end_between_the_last_report_and_now() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let open = app
        .log(&me, "2026-01-15T19:00:00Z", 5, &[], EventChoice::New)
        .await;
    let id = open.report.event_id;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(
        html.contains(r#"name="closed_at" value="2026-01-15T21:00""#),
        "now is the default: {html}"
    );

    let path = format!("/events/{id}/close");
    let early = app
        .send(post_form(
            &path,
            "closed_at=2026-01-15T19:30",
            Some(&cookie),
        ))
        .await;
    assert_eq!(early.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        text(early)
            .await
            .contains("The end cannot be before the last report.")
    );
    let late = app
        .send(post_form(
            &path,
            "closed_at=2026-01-15T23:00",
            Some(&cookie),
        ))
        .await;
    assert!(
        text(late)
            .await
            .contains("The end cannot be in the future.")
    );

    let response = app
        .send(post_form(
            &path,
            "closed_at=2026-01-15T20:40",
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some(&*format!("/events/{id}")));
    let event = app.store.event(id).await.unwrap().unwrap();
    assert_eq!(
        event.closed.map(|c| (c.at, c.reason)),
        Some((t("2026-01-15T19:40:00Z"), CloseReason::Manual))
    );
    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(html.contains("20:00–20:40 · closed by hand"), "{html}");
    assert!(html.contains(">Reopen</button>"), "{html}");
}

#[tokio::test]
async fn the_log_screen_closes_the_open_event_now() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let open = app
        .log(&me, "2026-01-15T19:00:00Z", 5, &[], EventChoice::New)
        .await;
    let id = open.report.event_id;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains(&format!(
            r#"<form method="post" action="/events/{id}/close">"#
        )),
        "{html}"
    );
    let response = app
        .send(post_form(
            &format!("/events/{id}/close"),
            "return_to=log",
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some("/"));
    let event = app.store.event(id).await.unwrap().unwrap();
    assert_eq!(event.closed.map(|c| c.at), Some(t("2026-01-15T20:00:00Z")));
}

#[tokio::test]
async fn only_residents_and_owning_guests_can_close_or_reopen() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let open = app
        .log(&me, "2026-01-15T19:00:00Z", 5, &[], EventChoice::New)
        .await;
    let id = open.report.event_id;
    app.log(
        &guest,
        "2026-01-15T19:30:00Z",
        4,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let cookie = app.sign_in("guest").await;

    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(!html.contains("Close event"), "{html}");
    let log = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!log.contains("/close"), "{log}");
    let response = app
        .send(post_form(&format!("/events/{id}/close"), "", Some(&cookie)))
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(app.store.event(id).await.unwrap().unwrap().is_open());
}

#[tokio::test]
async fn reopening_is_offered_only_while_no_other_event_is_open() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let earlier = app
        .log(&me, "2026-01-15T10:00:00Z", 5, &[], EventChoice::New)
        .await;
    let id = earlier.report.event_id;
    let current = app
        .log(&me, "2026-01-15T20:00:00Z", 5, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(!html.contains("Reopen"), "{html}");
    let refused = app
        .send(post_form(
            &format!("/events/{id}/reopen"),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        text(refused)
            .await
            .contains("Another event is open. Close it first.")
    );

    let close = format!("/events/{}/close", current.report.event_id);
    app.send(post_form(&close, "", Some(&cookie))).await;
    let response = app
        .send(post_form(
            &format!("/events/{id}/reopen"),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some(&*format!("/events/{id}")));
    assert!(app.store.event(id).await.unwrap().unwrap().is_open());
}

#[tokio::test]
async fn closing_or_reopening_an_event_a_guest_cannot_see_answers_not_found() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let open = app
        .log(&me, "2026-01-15T19:00:00Z", 5, &[], EventChoice::New)
        .await;
    let id = open.report.event_id;
    let guest = app.sign_in("guest").await;

    let close = app
        .send(post_form(&format!("/events/{id}/close"), "", Some(&guest)))
        .await;
    assert_eq!(close.status(), StatusCode::NOT_FOUND);
    assert!(app.store.event(id).await.unwrap().unwrap().is_open());

    let resident = app.sign_in("daniele").await;
    app.send(post_form(
        &format!("/events/{id}/close"),
        "",
        Some(&resident),
    ))
    .await;
    assert!(!app.store.event(id).await.unwrap().unwrap().is_open());
    let reopen = app
        .send(post_form(&format!("/events/{id}/reopen"), "", Some(&guest)))
        .await;
    assert_eq!(reopen.status(), StatusCode::NOT_FOUND);
    assert!(!app.store.event(id).await.unwrap().unwrap().is_open());
}
