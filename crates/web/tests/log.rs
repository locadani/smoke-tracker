//! The Log screen (spec §9, screen 1) and the "How to rate" guide (spec §5).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::repos::{Commit, EventRepo, ListWrites, Transactional, UserWrites};
use smoke_domain::types::{CloseReason, Locale, Role};

#[tokio::test]
async fn the_form_offers_visible_rooms_by_area_and_the_symptoms() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let kitchen = app.room("room-kitchen").await;
    let mut tx = app.store.begin(None).await.unwrap();
    tx.set_room_hidden(kitchen, true).await.unwrap();
    tx.commit().await.unwrap();
    let cookie = app.sign_in("daniele").await;

    let response = app.send(get_with("/", &cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert!(html.contains("<h1>Log a smell</h1>"), "{html}");
    assert!(html.contains("No smell event in progress."), "{html}");
    assert!(
        html.contains(r#"<form method="post" action="/log""#),
        "{html}"
    );
    let bedroom_id = app.room("room-bedroom").await;
    let level_row = format!(r#"name="level-{bedroom_id}""#);
    assert_eq!(html.matches(&level_row).count(), 11, "levels 0 to 10");
    assert!(html.contains("<legend>Level in Bedroom</legend>"), "{html}");
    let inside = html.find("Inside").unwrap();
    let common = html.find("Common areas").unwrap();
    let bedroom = html.find("<span>Bedroom</span>").unwrap();
    let stairwell = html.find("<span>Stairwell (shared)</span>").unwrap();
    assert!(inside < bedroom && bedroom < common && common < stairwell);
    assert!(!html.contains("Kitchen"), "hidden rooms are not offered");
    assert!(html.contains(r#"name="symptom""#) && html.contains("Eye irritation"));
    assert!(html.contains(">Log smell</button>"), "{html}");
    assert!(html.contains(">Log PM2.5 only</button>"), "{html}");
}

#[tokio::test]
async fn with_an_event_open_the_form_offers_an_update_with_my_last_rooms() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    let (bedroom, kitchen) = (
        app.room("room-bedroom").await,
        app.room("room-kitchen").await,
    );
    let first = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            4,
            &[bedroom],
            EventChoice::New { replacing: None },
        )
        .await;
    let event = EventChoice::Join(first.report.event_id.unwrap());
    app.log(&partner, "2026-01-15T19:55:00Z", 6, &[kitchen], event)
        .await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains("Smell event in progress since 19:05."),
        "{html}"
    );
    assert!(
        html.contains(">Update: situation changed</button>"),
        "{html}"
    );
    assert!(
        html.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
        "my last rooms are pre-selected: {html}"
    );
    assert!(
        html.contains(&format!(r#"value="{}"><span>Kitchen"#, kitchen.0)),
        "not the partner's: {html}"
    );
}

#[tokio::test]
async fn viewers_and_expired_guests_see_no_form() {
    let app = TestApp::new().await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let cookie = app.sign_in("lawyer").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!html.contains(r#"action="/log""#), "{html}");
    assert!(html.contains("You can look at the data but not log."));

    let mut tx = app.store.begin(None).await.unwrap();
    tx.expire_user(guest.id).await.unwrap();
    tx.commit().await.unwrap();
    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!html.contains(r#"action="/log""#), "{html}");
}

#[tokio::test]
async fn guests_see_the_open_event_only_if_they_own_it_or_reported_in_it() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let stranger = app.add_user("stranger", Role::Guest, Locale::En).await;
    let first = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            4,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = first.report.event_id.unwrap();
    let close = format!(r#"action="/events/{}/close""#, id.0);

    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    for hidden in [
        "since 19:05",
        "Update: situation changed",
        "No smell event in progress.",
    ] {
        assert!(!html.contains(hidden), "{hidden}: {html}");
    }
    assert!(html.contains(">Log smell</button>"), "{html}");

    app.log(
        &guest,
        "2026-01-15T19:50:00Z",
        3,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains("Smell event in progress since 19:05."),
        "{html}"
    );
    assert!(
        html.contains(">Update: situation changed</button>"),
        "{html}"
    );
    assert!(!html.contains(&close), "{html}");

    let mut tx = app.store.begin(None).await.unwrap();
    tx.expire_user(stranger.id).await.unwrap();
    tx.expire_user(guest.id).await.unwrap();
    tx.commit().await.unwrap();
    let cookie = app.sign_in("stranger").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!html.contains("since 19:05"), "{html}");
    assert!(!html.contains("No smell event in progress."), "{html}");
    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(html.contains("since 19:05."), "{html}");
    assert!(!html.contains(r#"action="/log""#), "{html}");
}

#[tokio::test]
async fn the_summary_counts_this_month_and_guests_count_only_their_own() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    app.log(
        &me,
        "2025-12-31T20:00:00Z",
        5,
        &[bedroom],
        EventChoice::New { replacing: None },
    )
    .await;
    let first = app
        .log(
            &me,
            "2026-01-10T20:00:00Z",
            5,
            &[bedroom],
            EventChoice::New { replacing: None },
        )
        .await;
    let join = EventChoice::Join(first.report.event_id.unwrap());
    app.log(&guest, "2026-01-10T21:00:00Z", 3, &[], join).await;
    app.log(
        &guest,
        "2026-01-12T20:00:00Z",
        6,
        &[bedroom],
        EventChoice::New { replacing: None },
    )
    .await;

    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains("This month: 2 events · 3 reports · 2 days with bedroom smoke"),
        "{html}"
    );
    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains("This month: 2 events · 2 reports · 1 day with bedroom smoke"),
        "{html}"
    );
}

#[tokio::test]
async fn opening_the_log_screen_runs_the_auto_close_sweep() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let logged = app
        .log(
            &me,
            "2026-01-15T20:00:00Z",
            5,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let cookie = app.sign_in("daniele").await;
    app.at("2026-01-16T07:00:00Z");

    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(html.contains("No smell event in progress."), "{html}");
    let event = app
        .store
        .event(logged.report.event_id.unwrap())
        .await
        .unwrap();
    assert_eq!(
        event.unwrap().closed.map(|c| c.reason),
        Some(CloseReason::Auto0400)
    );
}

#[tokio::test]
async fn how_to_rate_opens_a_dialog_with_a_link_to_the_full_guide() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::It).await;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains(r#"<a href="/guide/rating" data-dialog="rating-guide""#),
        "{html}"
    );
    assert!(html.contains(r#"<dialog id="rating-guide""#), "{html}");
    assert!(html.contains(r#"<form method="dialog""#), "{html}");
    assert!(
        html.contains(r#"<tr><th scope="row">0</th><td>Pulito</td></tr>"#),
        "{html}"
    );
    assert!(html.contains("<td>Appena percepibile</td>"), "{html}");
    assert!(
        html.contains("Tra due numeri, scegli il più basso."),
        "{html}"
    );
    assert!(html.contains(r#"<script src="/static/app.js" defer></script>"#));

    let guide = app.send(get_with("/guide/rating", &cookie)).await;
    assert_eq!(guide.status(), StatusCode::OK);
    let guide = text(guide).await;
    assert!(guide.contains("<h1>Come valutare</h1>"), "{guide}");
    assert!(
        guide.contains("Hai controllato: nessun odore di fumo."),
        "{guide}"
    );
    assert!(
        guide.contains("Come una stanza piena di fumo. Devi uscire."),
        "{guide}"
    );
    let script = app.send(get("/static/app.js")).await;
    assert_eq!(script.status(), StatusCode::OK);
    assert!(text(script).await.contains("showModal()"));
    assert_eq!(
        location(&app.send(get("/guide/rating")).await),
        Some("/login")
    );
}
