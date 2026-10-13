//! Saving the Log form and the event questions (spec §7.1–§7.6).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::repos::{EventRepo, ReadingRepo, ReportRepo};
use smoke_domain::types::{CloseReason, EventId, Locale, ReportId, Role};

/// The value of the hidden `asked` field on a question page.
fn asked(html: &str) -> String {
    let start = html.find(r#"name="asked" value=""#).expect("a question") + 20;
    html[start..start + html[start..].find('"').unwrap()].to_owned()
}

#[tokio::test]
async fn logging_with_nothing_open_saves_at_once_and_says_so() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let cookie = app.sign_in("daniele").await;

    let body = format!(
        "room={0}&level-{0}=6&note=from+the+balcony&action=smell",
        bedroom.0
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/?saved=report"));

    let open = app.store.open_event().await.unwrap().unwrap();
    let reports = app.store.reports_for_event(open.id).await.unwrap();
    assert_eq!(reports[0].values.room_ids(), [bedroom]);
    assert_eq!(reports[0].values.intensity().get(), 6);
    assert_eq!(reports[0].values.note(), Some("from the balcony"));
    assert_eq!(reports[0].observed_at, t("2026-01-15T20:00:00Z"));
    let html = text(app.send(get_with("/?saved=report", &cookie)).await).await;
    assert!(html.contains("Report saved."), "{html}");
}

#[tokio::test]
async fn with_an_event_open_the_same_smell_question_comes_first() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let first = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            4,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let cookie = app.sign_in("daniele").await;

    let question = app
        .send(post_form(
            "/log",
            "room=1&level-1=7&action=smell",
            Some(&cookie),
        ))
        .await;
    assert_eq!(question.status(), StatusCode::OK);
    let html = text(question).await;
    assert!(html.contains("<h1>Same smell still going?</h1>"), "{html}");
    assert!(html.contains("(started 19:05)"), "{html}");
    assert!(html.contains(r#"<input type="hidden" name="level-1" value="7">"#));
    let id = first.report.event_id.unwrap();
    assert!(html.contains(&format!(r#"value="join-{id}">Yes, update</button>"#)));
    assert!(html.contains(r#"value="new" class="secondary">No, it had stopped: new event"#));

    let body = format!(
        "room=1&level-1=7&action=smell&asked={}&choice=join-{id}",
        asked(&html)
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=report"));
    assert_eq!(app.store.reports_for_event(id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn answering_no_closes_the_open_event_at_its_last_report() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let first = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            4,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let cookie = app.sign_in("daniele").await;
    let body = format!(
        "room=1&level-1=3&action=smell&asked=same-smell-{}&choice=new",
        first.report.event_id.unwrap()
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=report"));
    let old = app
        .store
        .event(first.report.event_id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        old.closed.map(|c| (c.at, c.reason)),
        Some((t("2026-01-15T18:05:00Z"), CloseReason::NewEvent))
    );
    assert_ne!(
        app.store.open_event().await.unwrap().unwrap().id,
        first.report.event_id.unwrap()
    );
}

#[tokio::test]
async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let kitchen = app.room("room-kitchen").await;
    let first = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            9,
            &[kitchen],
            EventChoice::New { replacing: None },
        )
        .await;
    let closed_earlier = app
        .log(
            &me,
            "2026-01-15T10:00:00Z",
            8,
            &[kitchen],
            EventChoice::New {
                replacing: first.report.event_id,
            },
        )
        .await;
    let cookie = app.sign_in("guest").await;

    let html = text(
        app.send(post_form(
            "/log",
            "room=1&level-1=2&action=smell",
            Some(&cookie),
        ))
        .await,
    )
    .await;
    assert!(
        html.contains("A smell event is ongoing (started 19:05). Is this the same smell?"),
        "{html}"
    );
    for private in ["daniele", "Kitchen", ">9<", "ended"] {
        assert!(!html.contains(private), "{private} leaked: {html}");
    }

    let past = text(
        app.send(post_form(
            "/log",
            "room=1&level-1=2&action=smell&observed_at=2026-01-15T11:30",
            Some(&cookie),
        ))
        .await,
    )
    .await;
    assert!(past.contains("<h1>Which event was this?</h1>"), "{past}");
    let earlier = closed_earlier.report.event_id.unwrap();
    assert!(
        past.contains(&format!(
            r#"value="join-{earlier}" class="secondary">Started 11:00</button>"#
        )),
        "{past}"
    );
    assert!(
        !past.contains(&format!("join-{}", first.report.event_id.unwrap())),
        "not started yet"
    );
    for private in ["daniele", "Kitchen", "ended"] {
        assert!(!past.contains(private), "{private} leaked: {past}");
    }
}

#[tokio::test]
async fn a_stale_answer_gets_the_current_question_instead_of_being_applied() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    let first = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            4,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let cookie = app.sign_in("daniele").await;
    let question = text(
        app.send(post_form(
            "/log",
            "room=1&level-1=7&action=smell",
            Some(&cookie),
        ))
        .await,
    )
    .await;
    // Meanwhile the partner's phone answers "No, it had stopped".
    let second = app
        .log(
            &partner,
            "2026-01-15T19:58:00Z",
            5,
            &[],
            EventChoice::New {
                replacing: first.report.event_id,
            },
        )
        .await;

    let body = format!(
        "room=1&level-1=7&action=smell&asked={}&choice=join-{}",
        asked(&question),
        first.report.event_id.unwrap()
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert_eq!(
        asked(&html),
        format!("same-smell-{}", second.report.event_id.unwrap())
    );
    assert!(html.contains("(started 20:58)"), "{html}");
    assert_eq!(
        app.store
            .reports_for_event(first.report.event_id.unwrap())
            .await
            .unwrap()
            .len(),
        1,
        "nothing joined the closed event"
    );
}

#[tokio::test]
async fn a_past_report_after_the_end_of_the_chosen_event_moves_its_end() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let past = app
        .log(
            &me,
            "2026-01-15T10:00:00Z",
            4,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = past.report.event_id.unwrap();
    app.log(
        &me,
        "2026-01-15T20:00:00Z",
        4,
        &[],
        EventChoice::New {
            replacing: Some(id),
        },
    )
    .await;
    let cookie = app.sign_in("daniele").await;
    let html = text(
        app.send(post_form(
            "/log",
            "room=1&level-1=5&action=smell&observed_at=2026-01-15T12:00",
            Some(&cookie),
        ))
        .await,
    )
    .await;
    assert!(html.contains("Started 11:00, ended 11:00"), "{html}");
    assert!(
        !html.contains("Started 21:00"),
        "not started by then: {html}"
    );
    assert!(html.contains(r#"value="new" class="secondary">New event</button>"#));

    let body = format!(
        "room=1&level-1=5&action=smell&observed_at=2026-01-15T12:00&asked=which-event&choice=join-{id}"
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=report"));
    let event = app.store.event(id).await.unwrap().unwrap();
    assert_eq!(event.closed.map(|c| c.at), Some(t("2026-01-15T11:00:00Z")));
    assert_eq!(app.store.reports_for_event(id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn after_an_auto_close_the_next_log_offers_a_continuation() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let evening = app
        .log(
            &me,
            "2026-01-15T20:00:00Z",
            4,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = evening.report.event_id.unwrap();
    let cookie = app.sign_in("daniele").await;
    app.at("2026-01-16T06:00:00Z");

    let html = text(
        app.send(post_form(
            "/log",
            "room=1&level-1=5&action=smell",
            Some(&cookie),
        ))
        .await,
    )
    .await;
    assert!(
        html.contains("<h1>The event from 15/01/2026 21:00 was auto-closed.</h1>"),
        "{html}"
    );
    assert_eq!(asked(&html), format!("auto-closed-{id}"));
    let body = format!("room=1&level-1=5&action=smell&asked=auto-closed-{id}&choice=continue-{id}");
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=report"));
    let event = app.store.event(id).await.unwrap().unwrap();
    assert!(event.is_open());
    assert!(event.auto_close_notice_seen_at.is_some());
    assert!(app.store.unseen_auto_closed().await.unwrap().is_empty());
}

#[tokio::test]
async fn while_another_event_is_open_the_notice_is_a_dismissable_banner() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    app.log(
        &me,
        "2026-01-15T20:00:00Z",
        4,
        &[],
        EventChoice::New { replacing: None },
    )
    .await;
    app.at("2026-01-16T06:00:00Z");
    app.sweep().await;
    app.log(
        &partner,
        "2026-01-16T06:00:00Z",
        4,
        &[],
        EventChoice::New { replacing: None },
    )
    .await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains("<span>The event from 15/01/2026 21:00 was auto-closed.</span>"),
        "{html}"
    );
    let partners = text(app.send(get_with("/", &app.sign_in("partner").await)).await).await;
    assert!(
        !partners.contains("auto-closed"),
        "only the owner: {partners}"
    );

    let response = app
        .send(post_form("/notices/dismiss", "", Some(&cookie)))
        .await;
    assert_eq!(location(&response), Some("/"));
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!html.contains("auto-closed"), "{html}");
}

#[tokio::test]
async fn pm25_alone_needs_no_room_and_joins_the_open_event() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let open = app
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            4,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let cookie = app.sign_in("daniele").await;
    let response = app
        .send(post_form("/log", "pm25=35.5&action=pm25", Some(&cookie)))
        .await;
    assert_eq!(location(&response), Some("/?saved=pm25"));
    let readings = app
        .store
        .readings_for_event(open.report.event_id.unwrap())
        .await
        .unwrap();
    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].value.ugm3(), 35.5);

    let with_smell = format!(
        "room=1&level-1=6&pm25=40&action=smell&asked=same-smell-{0}&choice=join-{0}",
        open.report.event_id.unwrap()
    );
    app.send(post_form("/log", &with_smell, Some(&cookie)))
        .await;
    assert_eq!(
        app.store
            .readings_for_event(open.report.event_id.unwrap())
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn invalid_input_shows_the_form_again_with_what_was_typed() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let cookie = app.sign_in("daniele").await;
    let body = format!("room={}&note=kept+as+typed&action=smell", bedroom.0);
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = text(response).await;
    assert!(
        html.contains("Pick a level from 0 to 10 for each selected room."),
        "{html}"
    );
    assert!(html.contains(">kept as typed</textarea>"), "{html}");
    assert!(
        html.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
        "{html}"
    );
    let missing = app
        .send(post_form("/log", "action=pm25", Some(&cookie)))
        .await;
    assert!(
        text(missing)
            .await
            .contains("Enter a PM2.5 value to log it on its own.")
    );
    assert_eq!(app.store.open_event().await.unwrap(), None);
}

#[tokio::test]
async fn viewers_cannot_log() {
    let app = TestApp::new().await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let cookie = app.sign_in("lawyer").await;
    let response = app
        .send(post_form(
            "/log",
            "room=1&level-1=5&action=smell",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(text(response).await.contains("<h1>Not allowed</h1>"));
    assert_eq!(app.store.event(EventId(1)).await.unwrap(), None);
}

#[tokio::test]
async fn no_smell_with_nothing_open_saves_a_check_without_asking() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("guest", Role::Guest, Locale::En).await;
    let cookie = app.sign_in("guest").await;

    let response = app
        .send(post_form("/log", "action=no-smell", Some(&cookie)))
        .await;
    assert_eq!(location(&response), Some("/?saved=no-smell"));
    assert_eq!(app.store.open_event().await.unwrap(), None);
    let check = app.store.report(ReportId(1)).await.unwrap().unwrap();
    assert_eq!(check.event_id, None);
    assert!(check.values.rooms().is_empty(), "the whole home");
    let html = text(app.send(get_with("/?saved=no-smell", &cookie)).await).await;
    assert!(html.contains("Saved: no smell."), "{html}");
    assert!(html.contains(" · 1 clean check"), "{html}");
}

#[tokio::test]
async fn no_smell_with_an_event_open_joins_it_without_the_question() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let first = app
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            6,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let cookie = app.sign_in("guest").await;
    let bedroom = app.room("room-bedroom").await;

    let body = format!("room={0}&level-{0}=0&action=smell", bedroom.0);
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=no-smell"));
    let open = app.store.open_event().await.unwrap().unwrap();
    assert_eq!(Some(open.id), first.report.event_id, "still open");
    let reports = app.store.reports_for_event(open.id).await.unwrap();
    assert_eq!(reports.len(), 2);
    assert!(reports[1].values.is_no_smell());
}

#[tokio::test]
async fn a_past_no_smell_report_is_offered_no_event_instead_of_a_new_one() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;

    let body = "action=no-smell&observed_at=2026-01-15T12:00";
    let question = text(app.send(post_form("/log", body, Some(&cookie))).await).await;
    assert!(
        question.contains("<h1>Which event was this?</h1>"),
        "{question}"
    );
    assert!(
        question.contains(">No event: just a check</button>"),
        "{question}"
    );
    assert!(!question.contains(">New event</button>"), "{question}");

    let answer = format!("{body}&asked={}&choice=new", asked(&question));
    let response = app.send(post_form("/log", &answer, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=no-smell"));
    let check = app.store.report(ReportId(1)).await.unwrap().unwrap();
    assert_eq!(check.event_id, None);
    assert_eq!(app.store.event(EventId(1)).await.unwrap(), None);
}

#[tokio::test]
async fn no_smell_records_selected_rooms_at_zero_and_refuses_a_higher_level() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let bedroom = app.room("room-bedroom").await;

    let body = format!("room={0}&level-{0}=4&action=no-smell", bedroom.0);
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        text(response)
            .await
            .contains("No smell records the selected rooms at 0.")
    );

    let body = format!("room={}&action=no-smell", bedroom.0);
    app.send(post_form("/log", &body, Some(&cookie))).await;
    let check = app.store.report(ReportId(1)).await.unwrap().unwrap();
    assert_eq!(check.values.level_in(bedroom).map(|l| l.get()), Some(0));
}
