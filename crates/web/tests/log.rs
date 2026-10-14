//! The Log screen (spec §9, screen 1) and the "How to rate" guide (spec §5).

mod common;

use axum::http::StatusCode;
use common::page::Page;
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
    assert!(
        !Page::parse(&html).exists(".banner"),
        "no event, no banner: {html}"
    );
    assert!(
        html.contains(r#"<form method="post" action="/log""#),
        "{html}"
    );
    let page = Page::parse(&html);
    let bedroom_id = app.room("room-bedroom").await;
    let bedroom = format!(r#".room:has(input[name=room][value="{bedroom_id}"])"#);
    assert_eq!(
        page.attrs(&format!("{bedroom} .levels input[type=radio]"), "value"),
        (0..=10).map(|level| level.to_string()).collect::<Vec<_>>(),
        "levels 0 to 10"
    );
    assert_eq!(page.text(&format!("{bedroom} legend")), "Level in Bedroom");
    assert_eq!(
        page.text(&format!(
            "{bedroom} .level:has(input[value='3']) .level-text"
        )),
        "Light. Clearly smoke, but weak. Easy to ignore."
    );
    assert_eq!(
        page.text(&format!("{bedroom} .level-hint")),
        "Pick a level from 0 to 10."
    );
    assert_eq!(page.texts(".rooms > .group"), ["Inside", "Common areas"]);
    let rooms = page.texts(".rooms .room-check");
    assert_eq!(rooms.first().map(String::as_str), Some("Living room"));
    assert_eq!(rooms.last().map(String::as_str), Some("Stairwell (shared)"));
    assert!(
        !rooms.iter().any(|room| room == "Kitchen"),
        "hidden rooms are not offered"
    );
    assert!(
        page.texts("label.chip > input[name=symptom] + span")
            .contains(&"Eye irritation".to_owned())
    );
    assert!(
        page.exists("details.collapsible:not([open]) input[type=datetime-local][name=observed_at]")
    );
    assert!(page.exists("details.collapsible:not([open]) textarea[name=note]"));
    assert!(page.exists("input[type=number][name=pm25]"));
    let page = Page::parse(&html);
    assert_eq!(
        page.texts(r#"form[action="/log"] .actions button[name=action]"#),
        ["No smell", "Log"]
    );
    assert_eq!(
        page.attrs(
            r#"form[action="/log"] .actions button[name=action]"#,
            "value"
        ),
        ["no-smell", "log"]
    );
    assert!(page.exists("button[value=no-smell][formnovalidate][data-variant=secondary]"));
    assert_eq!(
        page.texts(".stats dt").last().map(String::as_str),
        Some("clean checks")
    );
    assert_eq!(
        page.texts(".stats dd").last().map(String::as_str),
        Some("0")
    );
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
    let page = Page::parse(&html);
    assert_eq!(
        page.text(".banner > p"),
        "Smell event in progress since 19:05 2 reports · highest level 6 (Kitchen)"
    );
    assert!(page.exists(&format!(
        r#".banner form[action="/events/{}/close"] button"#,
        first.report.event_id.unwrap().0
    )));
    assert!(
        html.contains(">Update: situation changed</button>"),
        "{html}"
    );
    assert_eq!(
        page.texts(".room-check:has(input[name=room][checked])"),
        ["Bedroom"],
        "my last rooms are pre-selected, not the partner's Kitchen"
    );
    assert!(page.exists(&format!(r#"input[name=room][value="{}"]"#, kitchen.0)));
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
    for hidden in ["since 19:05", "Update: situation changed"] {
        assert!(!html.contains(hidden), "{hidden}: {html}");
    }
    assert!(!Page::parse(&html).exists(".banner"), "{html}");
    assert!(html.contains(">Log</button>"), "{html}");

    app.log(
        &guest,
        "2026-01-15T19:50:00Z",
        3,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert_eq!(
        Page::parse(&html).text(".banner > p"),
        "Smell event in progress since 19:05 1 report · highest level 3 (Living room)",
        "the guest's own report only"
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
    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(html.contains("since 19:05"), "{html}");
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
    let tiles = async |cookie: &str| {
        let page = Page::parse(&text(app.send(get_with("/", cookie)).await).await);
        assert_eq!(page.text(".stats-caption"), "This month");
        page.texts(".stats dd")
            .into_iter()
            .zip(page.texts(".stats dt"))
            .collect::<Vec<_>>()
    };
    let pair = |n: &str, label: &str| (n.to_owned(), label.to_owned());
    assert_eq!(
        tiles(&cookie).await,
        [
            pair("2", "events"),
            pair("3", "reports"),
            pair("2", "bedroom days"),
            pair("0", "clean checks")
        ]
    );
    let cookie = app.sign_in("guest").await;
    assert_eq!(
        tiles(&cookie).await[..3],
        [
            pair("2", "events"),
            pair("2", "reports"),
            pair("1", "bedroom day")
        ]
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
    assert!(!Page::parse(&html).exists(".banner"), "auto-closed: {html}");
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
    let page = Page::parse(&html);
    assert!(page.exists(r#"a[href="/guide/rating"][data-dialog=rating-guide]"#));
    let sheet = "dialog#rating-guide.sheet[aria-labelledby=rating-guide-title]";
    assert_eq!(
        page.text(&format!("{sheet} h2#rating-guide-title")),
        "Come valutare"
    );
    assert!(page.exists(&format!(
        "{sheet} form[method=dialog] button[aria-label=Chiudi]"
    )));
    assert_eq!(page.count(&format!("{sheet} tr")), 11);
    assert_eq!(page.text(&format!("{sheet} tr td")), "Pulito");
    assert_eq!(
        page.texts(&format!("{sheet} tr td"))[1],
        "Appena percepibile"
    );
    assert_eq!(
        page.text(&format!("{sheet} p")),
        "Valuta quello che senti adesso, non quanto sei infastidito. Tra due numeri, scegli il più basso."
    );
    assert!(page.exists(&format!(r#"{sheet} a[href="/guide/rating"]"#)));
    assert!(page.exists(r#"script[type=module][src="/static/js/app.js"]"#));

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
    let script = app.send(get("/static/js/dialog.js")).await;
    assert_eq!(script.status(), StatusCode::OK);
    assert!(text(script).await.contains("showModal()"));
    assert_eq!(
        location(&app.send(get("/guide/rating")).await),
        Some("/login")
    );
}
