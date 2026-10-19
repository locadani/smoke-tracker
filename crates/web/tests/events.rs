//! The Events list and an event's page: timeline, large-gap warning, close and reopen
//! (spec §7.3, §7.7, §9 screen 3).

mod common;

use axum::http::StatusCode;
use common::page::Page;
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
        .log(
            &me,
            "2026-01-10T18:00:00Z",
            3,
            &[bedroom],
            EventChoice::New { replacing: None },
        )
        .await;
    let join = EventChoice::Join(first.report.event_id.unwrap());
    app.log(&me, "2026-01-10T19:00:00Z", 7, &[], join).await;
    let second = app
        .log(
            &me,
            "2026-01-12T07:30:00Z",
            5,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    app.log_pm25(&me, "2026-01-11T13:00:00Z", 12.5).await;
    let cookie = app.sign_in("daniele").await;

    let page = Page::parse(&text(app.send(get_with("/events?month=2026-01", &cookie)).await).await);
    assert_eq!(page.text("nav.switcher strong"), "01/2026");
    assert_eq!(
        page.attrs("nav.switcher a", "href"),
        ["/events?month=2025-12", "/events?month=2026-02"]
    );
    assert_eq!(
        page.attrs("nav.switcher a", "aria-label"),
        ["Previous month", "Next month"]
    );
    assert_eq!(
        page.texts("h2.overline"),
        ["12/01/2026", "11/01/2026", "10/01/2026"],
        "newest first"
    );
    let event = |id| format!(r#"a.row[href="/events/{id}"]"#);
    let first = event(first.report.event_id.unwrap());
    assert_eq!(page.text(&format!("{first} .badge[data-tone=warn]")), "7");
    assert_eq!(
        page.text(&format!("{first} .row-text")),
        "19:00–20:00 2 reports · max 7"
    );
    let second = event(second.report.event_id.unwrap());
    assert!(
        page.text(&format!("{second} .row-text strong"))
            .starts_with("08:30–")
    );
    assert_eq!(page.text("div.row .badge[data-tone=muted]"), "PM");
    assert_eq!(page.text("div.row .row-text"), "14:00 PM2.5 12.5 µg/m³");

    let empty = text(app.send(get_with("/events?month=2025-11", &cookie)).await).await;
    assert!(empty.contains("No events this month."), "{empty}");
    for far in ["junk", "262142-12", "-262143-01", "0001-01"] {
        let uri = format!("/events?month={far}");
        let page = Page::parse(&text(app.send(get_with(&uri, &cookie)).await).await);
        assert_eq!(page.text("nav.switcher strong"), "01/2026", "{far}");
    }
}

#[tokio::test]
async fn guests_list_only_events_they_own_or_reported_in() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let private = app
        .log(
            &me,
            "2026-01-10T18:00:00Z",
            8,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let shared = app
        .log(
            &me,
            "2026-01-11T18:00:00Z",
            9,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let join = EventChoice::Join(shared.report.event_id.unwrap());
    app.log(&guest, "2026-01-11T18:30:00Z", 4, &[], join).await;
    let cookie = app.sign_in("guest").await;

    let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
    assert!(
        !html.contains(&format!("/events/{}", private.report.event_id.unwrap())),
        "{html}"
    );
    let row = format!(
        r#"a.row[href="/events/{}"] .row-text"#,
        shared.report.event_id.unwrap()
    );
    assert_eq!(
        Page::parse(&html).text(&row),
        "19:00–19:30 1 report · max 4",
        "only the guest's own report counts"
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
        .log(
            &me,
            "2026-01-15T09:00:00Z",
            3,
            &[bedroom],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = first.report.event_id.unwrap();
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
    let page = Page::parse(&html);
    assert_eq!(page.text("h1"), "Event of 15/01/2026");
    assert!(
        page.text("h1 + p").ends_with("Started by daniele"),
        "{html}"
    );
    assert_eq!(page.attr("a.back", "href").as_deref(), Some("/events"));
    assert_eq!(
        page.text(&format!(
            r#".timeline a[href="/reports/{}"]"#,
            first.report.id
        )),
        "intensity 3 · Bedroom 3"
    );
    assert_eq!(
        page.texts(".timeline .entry > .muted"),
        ["daniele", "guest", "daniele", "daniele"]
    );
    assert_eq!(
        page.texts(".timeline > li .at"),
        ["10:00", "11:00", "11:30", "15:00"],
        "reports and readings in time order"
    );
    assert_eq!(
        page.text(".timeline > li[data-kind=reading] .entry"),
        "PM2.5 30 µg/m³ daniele"
    );
    let items = page.attrs(".timeline > li", "class");
    assert_eq!(items, ["gap"], "one gap warning");
    let order: Vec<String> = page.texts(".timeline > li");
    let gap = order
        .iter()
        .position(|item| item.starts_with("More than 3 hours"))
        .unwrap();
    assert!(
        order[gap - 1].contains("PM2.5 30") && order[gap + 1].starts_with("15:00"),
        "{order:?}"
    );
    assert!(page.exists(&format!(
        r#".timeline a[href="/reports/{}"]"#,
        late.report.id
    )));
}

/// Look-and-feel spec §6.4: the warning offers to move the report after the gap, through the
/// existing move route, to those allowed to move it.
#[tokio::test]
async fn the_gap_warning_moves_the_report_after_it_to_a_new_event() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let first = app
        .log(
            &me,
            "2026-01-15T09:00:00Z",
            3,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = first.report.event_id.unwrap();
    app.log(
        &guest,
        "2026-01-15T09:30:00Z",
        4,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let late = app
        .log(&me, "2026-01-15T13:45:00Z", 6, &[], EventChoice::Join(id))
        .await;
    let path = format!("/events/{id}");

    let resident = app.sign_in("daniele").await;
    let page = Page::parse(&text(app.send(get_with(&path, &resident)).await).await);
    let form = format!(
        r#".gap form[method=post][action="/reports/{}/move"]"#,
        late.report.id
    );
    assert!(page.exists(&format!(
        "{form} input[type=hidden][name=target][value=new]"
    )));
    assert_eq!(
        page.text(&format!("{form} button")),
        "Move 14:45 to a new event"
    );

    let guest = app.sign_in("guest").await;
    let page = Page::parse(&text(app.send(get_with(&path, &guest)).await).await);
    assert!(
        !page.exists(".gap"),
        "the guest sees only their own report: no gap"
    );

    let moved = app
        .send(post_form(
            &format!("/reports/{}/move", late.report.id),
            "target=new",
            Some(&resident),
        ))
        .await;
    assert_eq!(
        location(&moved),
        Some(format!("/reports/{}", late.report.id).as_str())
    );
    let report = app.report(late.report.id).await;
    assert_ne!(report.event_id, Some(id), "a new event");
    let page = Page::parse(&text(app.send(get_with(&path, &resident)).await).await);
    assert!(!page.exists(".gap"), "the gap is gone with the report");
}

#[tokio::test]
async fn the_gap_warning_offers_no_move_without_permission_or_for_a_check() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let first = app
        .log(
            &guest,
            "2026-01-15T09:00:00Z",
            3,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = first.report.event_id.unwrap();
    app.log(
        &guest,
        "2026-01-15T13:00:00Z",
        5,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let other = app
        .log(
            &me,
            "2026-01-15T14:00:00Z",
            2,
            &[],
            EventChoice::New {
                replacing: Some(id),
            },
        )
        .await;
    let other_id = other.report.event_id.unwrap();
    app.log_values(
        &me,
        "2026-01-15T18:00:00Z",
        vec![],
        &[],
        EventChoice::Join(other_id),
    )
    .await;
    let path = format!("/events/{id}");

    let viewer = app.sign_in("lawyer").await;
    let page = Page::parse(&text(app.send(get_with(&path, &viewer)).await).await);
    assert!(page.exists(".gap"), "viewers see the warning");
    assert!(!page.exists(".gap form"), "but may not move reports");

    let resident = app.sign_in("daniele").await;
    let page = Page::parse(
        &text(
            app.send(get_with(&format!("/events/{other_id}"), &resident))
                .await,
        )
        .await,
    );
    assert!(page.exists(".gap"), "a check after a gap");
    assert!(
        !page.exists(".gap form"),
        "a no-smell report cannot start an event"
    );
}

#[tokio::test]
async fn guests_see_only_their_own_entries_and_not_other_peoples_events() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let kitchen = app.room("room-kitchen").await;
    let shared = app
        .log(
            &me,
            "2026-01-15T09:00:00Z",
            9,
            &[kitchen],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = shared.report.event_id.unwrap();
    app.log(
        &guest,
        "2026-01-15T09:30:00Z",
        4,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let private = app
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            8,
            &[],
            EventChoice::New {
                replacing: Some(id),
            },
        )
        .await;
    let cookie = app.sign_in("guest").await;

    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(html.contains("intensity 4"), "{html}");
    for leak in ["intensity 9", "Kitchen", "daniele", "Started by"] {
        assert!(!html.contains(leak), "{leak} leaked: {html}");
    }
    let other = app
        .send(get_with(
            &format!("/events/{}", private.report.event_id.unwrap()),
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
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            5,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = open.report.event_id.unwrap();
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
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            5,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = open.report.event_id.unwrap();
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
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            5,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = open.report.event_id.unwrap();
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
        .log(
            &me,
            "2026-01-15T10:00:00Z",
            5,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = earlier.report.event_id.unwrap();
    let current = app
        .log(
            &me,
            "2026-01-15T20:00:00Z",
            5,
            &[],
            EventChoice::New {
                replacing: Some(id),
            },
        )
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

    let close = format!("/events/{}/close", current.report.event_id.unwrap());
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
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            5,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = open.report.event_id.unwrap();
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

#[tokio::test]
async fn the_list_shows_each_days_checks_to_those_who_may_see_them() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let mine = app.no_smell(&me, "2026-01-12T08:00:00Z").await;
    let theirs = app.no_smell(&guest, "2026-01-13T08:00:00Z").await;

    let resident = app.sign_in("daniele").await;
    let html = text(app.send(get_with("/events?month=2026-01", &resident)).await).await;
    let page = Page::parse(&html);
    for check in [&mine, &theirs] {
        let row = format!(r#"a.row[href="/reports/{}"]"#, check.report.id);
        assert_eq!(
            page.text(&format!("{row} .row-text")),
            "09:00 no smell (check)"
        );
        assert_eq!(page.text(&format!("{row} .badge")), "0");
    }
    assert!(!html.contains("No events this month."), "{html}");

    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
    assert!(
        html.contains(&format!("/reports/{}", theirs.report.id)),
        "{html}"
    );
    assert!(
        !html.contains(&format!("/reports/{}", mine.report.id)),
        "{html}"
    );
}

/// A repeated query parameter takes its first value instead of answering 400 (M6 carry-over).
#[tokio::test]
async fn a_repeated_month_takes_the_first_one() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let response = app
        .send(get_with("/events?month=2025-11&month=2025-12", &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page = Page::parse(&text(response).await);
    assert_eq!(page.text("nav.switcher strong"), "11/2025");
    for path in [
        "/?saved=report&saved=pm25",
        "/settings?saved=password&saved=lists",
        "/people?saved=expired&saved=x",
    ] {
        let response = app.send(get_with(path, &cookie)).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(
            Page::parse(&text(response).await).count(".notice[role=status]"),
            1,
            "{path}"
        );
    }
}
