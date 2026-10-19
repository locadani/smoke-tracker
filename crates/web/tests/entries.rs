//! A log entry's page at `/entries/{id}`: time, author, PM2.5 value, rooms, symptoms and note,
//! plus the links to it from the timeline and the Events list (spec 2026-10-19 §7).

mod common;

use axum::http::StatusCode;
use common::page::Page;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::repos::{Commit, EntryRepo, EntryWrites, Transactional};
use smoke_domain::types::{Locale, LogEntryId, Role};

async fn entry_ids(app: &TestApp) -> Vec<LogEntryId> {
    app.store
        .entries_between(t("2026-01-01T00:00:00Z"), t("2026-02-01T00:00:00Z"))
        .await
        .unwrap()
        .iter()
        .map(|entry| entry.id)
        .collect()
}

#[tokio::test]
async fn an_entry_page_shows_value_rooms_symptoms_and_note() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let headache = app.symptom("symptom-headache").await;
    let cookie = app.sign_in("daniele").await;
    let body = format!(
        "room={0}&level-{0}=4&symptom={1}&pm25=30&note=acrid&action=log",
        bedroom.0, headache.0
    );
    app.send(post_form("/log", &body, Some(&cookie))).await;
    let id = entry_ids(&app).await[0];

    let response = app.send(get_with(&format!("/entries/{id}"), &cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let page = Page::parse(&text(response).await);
    assert_eq!(page.text("h1"), "Log entry");
    let facts = page.texts(".facts div");
    for expected in [
        "Observed 15/01/2026 21:00",
        "PM2.5 30 µg/m³",
        "Intensity 4 · Noticeable",
        "Rooms Bedroom 4",
        "Symptoms Headache",
        "Note acrid",
    ] {
        assert!(
            facts.iter().any(|f| f.contains(expected)),
            "{expected} in {facts:?}"
        );
    }
    assert!(page.exists(".facts dd"), "a fact list");
    assert!(
        page.attrs("a.button", "href")
            .iter()
            .any(|href| href.ends_with("/correct")),
        "a combined entry with a report can be corrected"
    );
}

#[tokio::test]
async fn a_pm25_only_entry_page_has_no_edit_button() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let reading = app.log_pm25(&me, "2026-01-15T19:00:00Z", 12.0).await;
    let cookie = app.sign_in("daniele").await;

    let html = text(
        app.send(get_with(&format!("/entries/{}", reading.entry_id), &cookie))
            .await,
    )
    .await;
    let page = Page::parse(&html);
    assert!(
        page.texts(".facts div")
            .iter()
            .any(|f| f.contains("PM2.5 12 µg/m³")),
        "{html}"
    );
    assert!(
        !page.exists(r#"a[href$="/correct"]"#),
        "no Edit button: {html}"
    );
    assert!(
        !page.exists(r#"form[action$="/move"]"#),
        "no Move form: {html}"
    );
    assert_eq!(
        page.text("p.muted"),
        "Only a PM2.5 reading: there is nothing to correct here."
    );
    assert!(!page.exists(".history"), "no History: {html}");
    assert!(
        !html.contains("Correct a mistake") && !html.contains("Intensity"),
        "{html}"
    );
    assert!(
        page.texts(".facts div")
            .iter()
            .any(|f| f.contains("Event") && f.contains("None")),
        "{html}"
    );
    assert!(!html.contains("no-smell check"), "{html}");
}

#[tokio::test]
async fn a_guest_without_access_to_the_event_gets_no_link_to_it() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let logged = app
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            6,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let event = logged.report.event_id.expect("an event").0;
    let reading = app.log_pm25(&guest, "2026-01-15T19:30:00Z", 14.0).await;
    assert_eq!(reading.event_id.map(|id| id.0), Some(event));
    let cookie = app.sign_in("guest").await;

    let html = text(
        app.send(get_with(&format!("/entries/{}", reading.entry_id), &cookie))
            .await,
    )
    .await;
    let page = Page::parse(&html);
    assert!(
        !page.exists(&format!(r#"a[href="/events/{event}"]"#)),
        "{html}"
    );
    assert!(page.exists(r#"a[href="/events"]"#), "{html}");
}

#[tokio::test]
async fn pm25_lines_in_the_timeline_link_to_the_entry_page() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let first = app
        .log(
            &me,
            "2026-01-15T10:00:00Z",
            3,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let reading = app.log_pm25(&me, "2026-01-15T10:30:00Z", 30.0).await;
    let cookie = app.sign_in("daniele").await;

    let event = first.report.event_id.unwrap();
    let html = text(
        app.send(get_with(&format!("/events/{event}"), &cookie))
            .await,
    )
    .await;
    let page = Page::parse(&html);
    assert_eq!(
        page.text(&format!(
            r#".timeline a[href="/entries/{}"]"#,
            reading.entry_id
        )),
        "PM2.5 30 µg/m³"
    );
}

#[tokio::test]
async fn pm_rows_in_the_events_list_link_to_the_entry_page() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let reading = app.log_pm25(&me, "2026-01-12T08:00:00Z", 18.0).await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
    let page = Page::parse(&html);
    let row = format!(r#"a.row[href="/entries/{}"]"#, reading.entry_id);
    assert_eq!(
        page.text(&format!("{row} .row-text")),
        "09:00 PM2.5 18 µg/m³"
    );
    assert_eq!(page.text(&format!("{row} .badge")), "PM");
}

#[tokio::test]
async fn reports_url_redirects_to_the_entry_page() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let logged = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            6,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let cookie = app.sign_in("daniele").await;

    let response = app
        .send(get_with(&format!("/reports/{}", logged.report.id), &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        location(&response),
        Some(format!("/entries/{}", logged.report.entry_id).as_str())
    );
}

#[tokio::test]
async fn guests_cannot_see_other_peoples_entries() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let mine = app.log_pm25(&me, "2026-01-15T19:00:00Z", 12.0).await;
    let theirs = app.log_pm25(&guest, "2026-01-15T19:30:00Z", 14.0).await;
    let cookie = app.sign_in("guest").await;

    let response = app
        .send(get_with(&format!("/entries/{}", mine.entry_id), &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = app
        .send(get_with(&format!("/entries/{}", theirs.entry_id), &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn reports_redirect_does_not_leak_other_peoples_entries() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let mine = app
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            8,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let cookie = app.sign_in("guest").await;

    for id in [mine.report.id.0, mine.report.id.0 + 1000] {
        let response = app.send(get_with(&format!("/reports/{id}"), &cookie)).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn a_deleted_entry_answers_404() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let logged = app
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            8,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = logged.report.entry_id;
    let mut tx = app.store.begin(Some(me.id)).await.unwrap();
    tx.delete_entry(id).await.unwrap();
    tx.commit().await.unwrap();
    let cookie = app.sign_in("daniele").await;

    for path in [
        format!("/entries/{id}"),
        format!("/reports/{}", logged.report.id),
    ] {
        let response = app.send(get_with(&path, &cookie)).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn a_pm25_only_entry_page_shows_its_note() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    app.send(post_form(
        "/log",
        "pm25=12&note=hazy+after+cooking&action=log",
        Some(&cookie),
    ))
    .await;
    let id = entry_ids(&app).await[0];

    let html = text(app.send(get_with(&format!("/entries/{id}"), &cookie)).await).await;
    let facts = Page::parse(&html).texts(".facts div");
    assert!(
        facts.iter().any(|f| f == "Note hazy after cooking"),
        "{facts:?}"
    );
    assert!(facts.iter().any(|f| f == "PM2.5 12 µg/m³"), "{facts:?}");
}
