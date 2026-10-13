//! A report's page: values, both times, History, "Correct a mistake" and "Move to another
//! event" (spec §6.1, §7.7, §9 screen 4).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::repos::{EventRepo, ReportRepo};
use smoke_domain::types::{Locale, Role};

#[tokio::test]
async fn a_report_shows_both_times_its_values_and_its_history() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let logged = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            6,
            &[bedroom],
            EventChoice::New { replacing: None },
        )
        .await;
    let cookie = app.sign_in("daniele").await;

    let response = app
        .send(get_with(&format!("/reports/{}", logged.report.id), &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert!(
        html.contains("<dd>15/01/2026 19:05</dd>"),
        "observed: {html}"
    );
    assert!(
        html.contains("<dd>15/01/2026 21:00</dd>"),
        "recorded: {html}"
    );
    assert!(html.contains("<dd>6 · Strong</dd>"), "{html}");
    assert!(html.contains("<dd>Bedroom 6</dd>"), "{html}");
    assert!(html.contains("<dd>daniele</dd>"), "{html}");
    assert!(
        html.contains("<li>15/01/2026 21:00 · daniele · logged</li>"),
        "{html}"
    );
    assert!(!html.contains("edited"), "{html}");
    assert!(html.contains(">Correct a mistake</a>"), "{html}");
}

#[tokio::test]
async fn correcting_a_mistake_is_recorded_and_badged() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let (bedroom, kitchen) = (
        app.room("room-bedroom").await,
        app.room("room-kitchen").await,
    );
    let logged = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            6,
            &[bedroom],
            EventChoice::New { replacing: None },
        )
        .await;
    let id = logged.report.id;
    let cookie = app.sign_in("daniele").await;

    let form = text(
        app.send(get_with(&format!("/reports/{id}/correct"), &cookie))
            .await,
    )
    .await;
    assert!(
        form.contains(&format!(r#"name="level-{}" value="6" checked>"#, bedroom.0)),
        "{form}"
    );
    assert!(
        form.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
        "{form}"
    );
    assert!(
        form.contains(r#"name="observed_at" value="2026-01-15T19:05""#),
        "{form}"
    );
    assert!(form.contains(r#"<dialog id="rating-guide""#), "{form}");

    app.at("2026-01-16T08:15:00Z");
    let body = format!(
        "room={0}&level-{0}=7&room={1}&level-{1}=3&observed_at=2026-01-15T19:05&note=",
        bedroom.0, kitchen.0
    );
    let response = app
        .send(post_form(
            &format!("/reports/{id}/correct"),
            &body,
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some(&*format!("/reports/{id}")));
    let html = text(app.send(get_with(&format!("/reports/{id}"), &cookie)).await).await;
    assert!(
        html.contains(r#"<span class="badge">edited</span>"#),
        "{html}"
    );
    assert!(
        html.contains(
            "<li>16/01/2026 09:15 · daniele · corrected: Bedroom 6 → 7, added Kitchen 3</li>"
        ),
        "{html}"
    );
}

#[tokio::test]
async fn a_correction_must_stay_inside_the_event() {
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
    let id = logged.report.id;
    let cookie = app.sign_in("daniele").await;
    let response = app
        .send(post_form(
            &format!("/reports/{id}/correct"),
            "room=1&level-1=6&observed_at=2026-01-15T18:00",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = text(response).await;
    assert!(html.contains("That time is outside the event"), "{html}");
    let unchanged = app.store.report(id).await.unwrap().unwrap();
    assert_eq!(unchanged.observed_at, t("2026-01-15T18:05:00Z"));
}

#[tokio::test]
async fn moving_offers_that_days_events_and_a_new_one() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let morning = app
        .log(
            &me,
            "2026-01-15T10:00:00Z",
            4,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let evening = app
        .log(
            &me,
            "2026-01-15T20:00:00Z",
            5,
            &[],
            EventChoice::New {
                replacing: morning.report.event_id,
            },
        )
        .await;
    let joined = app
        .log(
            &me,
            "2026-01-15T11:00:00Z",
            5,
            &[],
            EventChoice::Join(morning.report.event_id.unwrap()),
        )
        .await;
    let stray = app
        .log(
            &me,
            "2026-01-15T20:00:00Z",
            3,
            &[],
            EventChoice::Join(evening.report.event_id.unwrap()),
        )
        .await;
    let cookie = app.sign_in("daniele").await;
    let path = format!("/reports/{}", joined.report.id);
    let html = text(app.send(get_with(&path, &cookie)).await).await;
    assert!(
        html.contains(r#"<option value="new">New event</option>"#),
        "{html}"
    );
    assert!(
        !html.contains("event-"),
        "the evening event had not started: {html}"
    );

    let path = format!("/reports/{}", stray.report.id);
    let html = text(app.send(get_with(&path, &cookie)).await).await;
    let morning_id = morning.report.event_id.unwrap();
    assert!(
        html.contains(&format!(
            r#"<option value="event-{morning_id}">Started 11:00, ended 12:00</option>"#
        )),
        "{html}"
    );
    let response = app
        .send(post_form(
            &format!("{path}/move"),
            &format!("target=event-{morning_id}"),
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some(&*path));
    let moved = app.store.report(stray.report.id).await.unwrap().unwrap();
    assert_eq!(moved.event_id, Some(morning_id));
    let event = app.store.event(morning_id).await.unwrap().unwrap();
    assert_eq!(event.closed.map(|c| c.at), Some(t("2026-01-15T20:00:00Z")));
    let html = text(app.send(get_with(&path, &cookie)).await).await;
    assert!(
        html.contains(&format!(
            "moved: event #{} → #{morning_id}",
            evening.report.event_id.unwrap()
        )),
        "{html}"
    );
}

#[tokio::test]
async fn guests_cannot_see_or_change_other_peoples_reports() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let mine = app
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            8,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let theirs = app
        .log(
            &guest,
            "2026-01-15T19:30:00Z",
            4,
            &[],
            EventChoice::Join(mine.report.event_id.unwrap()),
        )
        .await;
    let cookie = app.sign_in("guest").await;
    let id = mine.report.id;
    for id in [id.0, id.0 + 1000] {
        for response in [
            app.send(get_with(&format!("/reports/{id}"), &cookie)).await,
            app.send(get_with(&format!("/reports/{id}/correct"), &cookie))
                .await,
            app.send(post_form(
                &format!("/reports/{id}/correct"),
                "room=1&level-1=1",
                Some(&cookie),
            ))
            .await,
            app.send(post_form(
                &format!("/reports/{id}/move"),
                "target=new",
                Some(&cookie),
            ))
            .await,
            app.send(post_form(
                &format!("/reports/{id}/move"),
                "target=nonsense",
                Some(&cookie),
            ))
            .await,
        ] {
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }
    assert_eq!(app.store.report(id).await.unwrap().unwrap(), mine.report);

    let own = text(
        app.send(get_with(&format!("/reports/{}", theirs.report.id), &cookie))
            .await,
    )
    .await;
    assert!(own.contains(">Correct a mistake</a>"), "{own}");
    assert!(
        !own.contains("daniele") && !own.contains("<dd>guest</dd>"),
        "{own}"
    );
}

#[tokio::test]
async fn viewers_see_reports_but_cannot_change_them() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let logged = app
        .log(
            &me,
            "2026-01-15T19:00:00Z",
            8,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let cookie = app.sign_in("lawyer").await;
    let id = logged.report.id;
    let html = text(app.send(get_with(&format!("/reports/{id}"), &cookie)).await).await;
    assert!(html.contains("<dd>daniele</dd>"), "{html}");
    assert!(
        !html.contains("Correct a mistake") && !html.contains("/move"),
        "{html}"
    );
    let response = app
        .send(get_with(&format!("/reports/{id}/correct"), &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn guests_see_other_events_in_the_move_options_without_their_end() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let morning = app
        .log(
            &me,
            "2026-01-15T10:00:00Z",
            4,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let theirs = app
        .log(
            &guest,
            "2026-01-15T19:30:00Z",
            3,
            &[],
            EventChoice::New {
                replacing: morning.report.event_id,
            },
        )
        .await;
    app.at("2026-01-16T03:30:00Z");
    app.sweep().await;
    let morning_event = app
        .store
        .event(morning.report.event_id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert!(morning_event.closed.is_some(), "the morning event is over");
    let cookie = app.sign_in("guest").await;

    let html = text(
        app.send(get_with(&format!("/reports/{}", theirs.report.id), &cookie))
            .await,
    )
    .await;
    assert!(
        html.contains(&format!(
            r#"<option value="event-{}">Started 11:00</option>"#,
            morning.report.event_id.unwrap()
        )),
        "{html}"
    );
    assert!(
        !html.to_lowercase().contains("ended"),
        "a guest never learns when another event ended: {html}"
    );
}

#[tokio::test]
async fn a_check_has_no_event_and_cannot_be_corrected_to_a_smell() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let check = app.no_smell(&me, "2026-01-15T19:00:00Z").await.report;
    let cookie = app.sign_in("daniele").await;
    let id = check.id;

    let html = text(app.send(get_with(&format!("/reports/{id}"), &cookie)).await).await;
    assert!(html.contains("<dd>None: a no-smell check</dd>"), "{html}");
    assert!(html.contains("<dd>whole home</dd>"), "{html}");
    assert!(html.contains("<dd>0 · Clean</dd>"), "{html}");
    assert!(
        !html.contains(r#"value="new""#),
        "a check never opens an event: {html}"
    );

    let form = text(
        app.send(get_with(&format!("/reports/{id}/correct"), &cookie))
            .await,
    )
    .await;
    assert!(form.contains("This is a no-smell check"), "{form}");
    let bedroom = app.room("room-bedroom").await;
    let body = format!(
        "room={0}&level-{0}=3&observed_at=2026-01-15T20:00",
        bedroom.0
    );
    let refused = app
        .send(post_form(
            &format!("/reports/{id}/correct"),
            &body,
            Some(&cookie),
        ))
        .await;
    assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text(refused).await.contains("its levels stay at 0"));

    let body = format!(
        "room={0}&level-{0}=0&observed_at=2026-01-15T20:00",
        bedroom.0
    );
    let response = app
        .send(post_form(
            &format!("/reports/{id}/correct"),
            &body,
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some(&*format!("/reports/{id}")));
}

#[tokio::test]
async fn a_correction_with_no_room_is_refused_unless_the_report_covered_the_whole_home() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let smell = app
        .log(
            &me,
            "2026-01-15T18:05:00Z",
            6,
            &[bedroom],
            EventChoice::New { replacing: None },
        )
        .await
        .report;
    let check = app.no_smell(&me, "2026-01-15T19:30:00Z").await.report;
    let cookie = app.sign_in("daniele").await;

    let path = format!("/reports/{}/correct", smell.id);
    let body = "observed_at=2026-01-15T19:05&note=kept";
    let refused = app.send(post_form(&path, body, Some(&cookie))).await;
    assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = text(refused).await;
    assert!(html.contains("Pick at least one room"), "{html}");
    assert!(html.contains("kept"), "what was typed stays: {html}");
    assert_eq!(app.store.report(smell.id).await.unwrap().unwrap(), smell);

    let path = format!("/reports/{}/correct", check.id);
    let body = "observed_at=2026-01-15T20:30&note=looked+twice";
    let saved = app.send(post_form(&path, body, Some(&cookie))).await;
    assert_eq!(location(&saved), Some(&*format!("/reports/{}", check.id)));
    let after = app.store.report(check.id).await.unwrap().unwrap();
    assert!(after.values.rooms().is_empty(), "still the whole home");
    assert_eq!(after.values.note(), Some("looked twice"));
}

#[tokio::test]
async fn guests_cannot_see_or_change_other_peoples_checks() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let check = app.no_smell(&me, "2026-01-15T19:00:00Z").await.report;
    let cookie = app.sign_in("guest").await;
    let id = check.id;
    for response in [
        app.send(get_with(&format!("/reports/{id}"), &cookie)).await,
        app.send(get_with(&format!("/reports/{id}/correct"), &cookie))
            .await,
        app.send(post_form(
            &format!("/reports/{id}/correct"),
            "observed_at=2026-01-15T20:00",
            Some(&cookie),
        ))
        .await,
        app.send(post_form(
            &format!("/reports/{id}/move"),
            "target=nonsense",
            Some(&cookie),
        ))
        .await,
    ] {
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    assert_eq!(app.store.report(id).await.unwrap().unwrap(), check);
}
