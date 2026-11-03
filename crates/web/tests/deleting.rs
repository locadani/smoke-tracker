//! Deleting an entry from its page (spec 2026-11-03 "Screens", "After deleting").

mod common;

use axum::http::StatusCode;
use common::page::Page;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::records::User;
use smoke_domain::types::{EventId, Locale, Role};

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

/// Logs `count` more reports of `user` into `event`, a few minutes apart after `18:00`.
async fn add_reports(app: &TestApp, user: &User, event: EventId, count: u32, first_minute: u32) {
    for index in 0..count {
        let at = format!("2026-01-15T18:{:02}:00Z", first_minute + index);
        app.log(user, &at, 4, &[], EventChoice::Join(event)).await;
    }
}

/// An event opened by `owner` at 18:00 holding that one report.
async fn event_of(app: &TestApp, owner: &User) -> EventId {
    app.log(owner, "2026-01-15T18:00:00Z", 4, &[], NEW)
        .await
        .report
        .event_id
        .unwrap()
}

async fn event_page(app: &TestApp, id: EventId, cookie: &str, query: &str) -> String {
    text(
        app.send(get_with(&format!("/events/{}{query}", id.0), cookie))
            .await,
    )
    .await
}

#[tokio::test]
async fn only_my_entries_offers_delete_and_confirms_with_the_count() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let cookie = app.sign_in("daniele").await;

    let single = event_of(&app, &me).await;
    let html = event_page(&app, single, &cookie, "?delete").await;
    assert_eq!(
        Page::parse(&html).text("#confirm-delete-question"),
        "Eliminare questo evento e la sua registrazione?"
    );
    app.send(post_form(
        &format!("/events/{}/delete", single.0),
        "",
        Some(&cookie),
    ))
    .await;

    let many = event_of(&app, &me).await;
    add_reports(&app, &me, many, 2, 10).await;
    let plain = event_page(&app, many, &cookie, "").await;
    let page = Page::parse(&plain);
    assert_eq!(page.attrs("a[data-variant=danger]", "href"), ["?delete"]);
    assert_eq!(page.texts("a[data-variant=danger]"), ["Elimina"]);
    assert!(!plain.contains("Non si può annullare."), "{plain}");

    let html = event_page(&app, many, &cookie, "?delete").await;
    let page = Page::parse(&html);
    assert_eq!(
        page.text("#confirm-delete-question"),
        "Eliminare questo evento e le sue 3 registrazioni?"
    );
    assert_eq!(
        page.attrs("form[action$='/delete']", "action"),
        [format!("/events/{}/delete", many.0)]
    );
    assert_eq!(
        page.attrs("a[data-variant=secondary]", "href"),
        [format!("/events/{}", many.0)]
    );
    assert!(!page.exists("a[data-variant=danger]"), "{html}");
}

#[tokio::test]
async fn deleting_an_event_lands_on_history_and_the_event_is_gone() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let id = event_of(&app, &me).await;
    add_reports(&app, &me, id, 1, 10).await;
    let cookie = app.sign_in("daniele").await;

    let response = app
        .send(post_form(
            &format!("/events/{}/delete", id.0),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/events"));
    let gone = app
        .send(get_with(&format!("/events/{}", id.0), &cookie))
        .await;
    assert_eq!(gone.status(), StatusCode::NOT_FOUND);
    let history = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
    assert!(!history.contains(&format!("/events/{}", id.0)), "{history}");
}

#[tokio::test]
async fn another_resident_greys_the_button_with_recap() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let duygu = app.add_user("duygu", Role::Resident, Locale::It).await;
    let id = event_of(&app, &me).await;
    add_reports(&app, &me, id, 2, 10).await;
    add_reports(&app, &duygu, id, 1, 20).await;
    let cookie = app.sign_in("daniele").await;

    for query in ["", "?delete"] {
        let html = event_page(&app, id, &cookie, query).await;
        let page = Page::parse(&html);
        assert_eq!(
            page.attrs("button[data-variant=danger][disabled]", "disabled")
                .len(),
            1,
            "{html}"
        );
        assert!(!page.exists("a[data-variant=danger]"), "{html}");
        assert!(!page.exists("#confirm-delete-question"), "{html}");
        assert_eq!(
            page.attrs("button[data-variant=danger][disabled]", "aria-describedby"),
            ["delete-recap delete-reason".to_owned()]
        );
        assert!(page.exists("p#delete-recap.delete-note"), "{html}");
        assert!(page.exists("p#delete-reason.delete-note"), "{html}");
        assert_eq!(
            page.texts(".delete-note"),
            [
                "4 registrazioni: tu 3 · duygu 1".to_owned(),
                "Per eliminare l'evento, duygu deve prima eliminare le sue.".to_owned()
            ]
        );
    }
}

#[tokio::test]
async fn three_people_use_the_others_sentence() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let duygu = app.add_user("duygu", Role::Resident, Locale::It).await;
    let marta = app.add_user("marta", Role::Resident, Locale::It).await;
    let id = event_of(&app, &me).await;
    add_reports(&app, &duygu, id, 1, 10).await;
    add_reports(&app, &marta, id, 1, 20).await;
    let cookie = app.sign_in("daniele").await;

    let html = event_page(&app, id, &cookie, "").await;
    assert_eq!(
        Page::parse(&html).texts(".delete-note"),
        [
            "3 registrazioni: tu 1 · duygu 1 · marta 1".to_owned(),
            "Per eliminare l'evento, gli altri devono prima eliminare le loro.".to_owned()
        ]
    );
}

#[tokio::test]
async fn a_guests_entries_never_block_a_resident() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let guest = app.add_user("ospite", Role::Guest, Locale::It).await;
    let id = event_of(&app, &me).await;
    add_reports(&app, &guest, id, 1, 10).await;
    let cookie = app.sign_in("daniele").await;

    let html = event_page(&app, id, &cookie, "?delete").await;
    assert_eq!(
        Page::parse(&html).text("#confirm-delete-question"),
        "Eliminare questo evento e le sue 2 registrazioni?"
    );
    let response = app
        .send(post_form(
            &format!("/events/{}/delete", id.0),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some("/events"));
}

#[tokio::test]
async fn a_guest_sees_others_counted_not_named() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let duygu = app.add_user("duygu", Role::Resident, Locale::It).await;
    let guest = app.add_user("ospite", Role::Guest, Locale::It).await;
    let id = event_of(&app, &me).await;
    add_reports(&app, &me, id, 1, 10).await;
    add_reports(&app, &duygu, id, 1, 20).await;
    add_reports(&app, &guest, id, 1, 30).await;
    let cookie = app.sign_in("ospite").await;

    let html = event_page(&app, id, &cookie, "").await;
    assert_eq!(
        Page::parse(&html).texts(".delete-note"),
        [
            "4 registrazioni: tu 1 · altri 3".to_owned(),
            "Per eliminare l'evento, gli altri devono prima eliminare le loro.".to_owned()
        ]
    );
    for name in ["daniele", "duygu"] {
        assert!(!html.contains(name), "{name}: {html}");
    }
}

#[tokio::test]
async fn a_resident_with_one_blocker_among_guests_is_named() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let duygu = app.add_user("duygu", Role::Resident, Locale::It).await;
    let guest = app.add_user("ospite", Role::Guest, Locale::It).await;
    let id = event_of(&app, &duygu).await;
    add_reports(&app, &guest, id, 1, 10).await;
    add_reports(&app, &me, id, 1, 20).await;
    let cookie = app.sign_in("daniele").await;

    let html = event_page(&app, id, &cookie, "").await;
    assert_eq!(
        Page::parse(&html).texts(".delete-note"),
        [
            "3 registrazioni: tu 1 · duygu 1 · ospite 1".to_owned(),
            "Per eliminare l'evento, duygu deve prima eliminare le sue.".to_owned()
        ]
    );
}

#[tokio::test]
async fn no_button_when_none_is_mine() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::It).await;
    let duygu = app.add_user("duygu", Role::Resident, Locale::It).await;
    app.add_user("vedo", Role::Viewer, Locale::It).await;
    let id = event_of(&app, &duygu).await;
    let resident = app.sign_in("daniele").await;
    let viewer = app.sign_in("vedo").await;

    for (cookie, status) in [
        (&resident, StatusCode::FORBIDDEN),
        (&viewer, StatusCode::FORBIDDEN),
    ] {
        for query in ["", "?delete"] {
            let html = event_page(&app, id, cookie, query).await;
            let page = Page::parse(&html);
            for css in [
                "a[data-variant=danger]",
                "button[data-variant=danger]",
                "#confirm-delete-question",
                ".delete-note",
                ".cluster",
            ] {
                assert!(!page.exists(css), "{css}: {html}");
            }
        }
        let refused = app
            .send(post_form(
                &format!("/events/{}/delete", id.0),
                "",
                Some(cookie),
            ))
            .await;
        assert_eq!(refused.status(), status);
    }
}

#[tokio::test]
async fn a_forged_post_when_blocked_redirects_to_the_event() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let duygu = app.add_user("duygu", Role::Resident, Locale::It).await;
    let id = event_of(&app, &me).await;
    add_reports(&app, &duygu, id, 1, 10).await;
    let cookie = app.sign_in("daniele").await;

    let response = app
        .send(post_form(
            &format!("/events/{}/delete", id.0),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        location(&response),
        Some(format!("/events/{}", id.0).as_str())
    );
    assert_eq!(app.reports_for_event(id).await.len(), 2);
}

#[tokio::test]
async fn deleting_an_emptied_event_is_not_found() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    let id = event_of(&app, &me).await;
    let cookie = app.sign_in("daniele").await;
    let path = format!("/events/{}/delete", id.0);

    app.send(post_form(&path, "", Some(&cookie))).await;
    let again = app.send(post_form(&path, "", Some(&cookie))).await;
    assert_eq!(again.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_guest_posting_a_delete_for_an_event_they_cannot_open_is_not_found() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::It).await;
    app.add_user("ospite", Role::Guest, Locale::It).await;
    let id = event_of(&app, &me).await;
    let cookie = app.sign_in("ospite").await;

    let response = app
        .send(post_form(
            &format!("/events/{}/delete", id.0),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(app.reports_for_event(id).await.len(), 1);
}

#[tokio::test]
async fn moving_to_a_new_event_is_offered_only_where_it_changes_something() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let alone = app.log(&me, "2026-01-15T10:00:00Z", 4, &[], NEW).await;
    let first = app
        .log(
            &me,
            "2026-01-15T18:00:00Z",
            4,
            &[],
            EventChoice::New {
                replacing: alone.report.event_id,
            },
        )
        .await;
    let second = app
        .log(
            &me,
            "2026-01-15T18:30:00Z",
            3,
            &[],
            EventChoice::Join(first.report.event_id.unwrap()),
        )
        .await;
    let cookie = app.sign_in("daniele").await;

    for (entry, offered) in [
        (alone.report.entry_id, false),
        (first.report.entry_id, true),
        (second.report.entry_id, true),
    ] {
        let html = text(
            app.send(get_with(&format!("/entries/{entry}"), &cookie))
                .await,
        )
        .await;
        assert_eq!(html.contains(r#"value="new""#), offered, "{entry}: {html}");
    }
}

#[tokio::test]
async fn deleting_the_last_report_leaves_the_reading_on_its_own() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let report = app.log(&me, "2026-01-15T18:00:00Z", 4, &[], NEW).await;
    let reading = app.log_pm25(&me, "2026-01-15T18:10:00Z", 12.0).await;
    let cookie = app.sign_in("daniele").await;

    let response = app
        .send(post_form(
            &format!("/entries/{}/delete", report.report.entry_id),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some("/events"));
    let event = app
        .send(get_with(
            &format!("/events/{}", report.report.event_id.unwrap()),
            &cookie,
        ))
        .await;
    assert_eq!(event.status(), StatusCode::NOT_FOUND);
    let alone = app
        .send(get_with(&format!("/entries/{}", reading.entry_id), &cookie))
        .await;
    assert_eq!(alone.status(), StatusCode::OK);
    let history = text(app.send(get_with("/events", &cookie)).await).await;
    assert!(history.contains("12"), "{history}");
}
