//! The Graphs screen (spec §9 screen 5): what each chart is given, and who sees what (spec §4).

mod common;

use axum::http::StatusCode;
use common::page::Page;
use common::*;
use serde_json::Value;
use smoke_domain::logbook::EventChoice;
use smoke_domain::records::RoomLevel;
use smoke_domain::types::{Level, Locale, Role};

/// The Chart.js configurations on a page, in page order, read back from their `data-chart`
/// attributes.
fn charts(html: &str) -> Vec<Value> {
    html.match_indices(r#"data-chart=""#)
        .map(|(start, attribute)| {
            let rest = &html[start + attribute.len()..];
            let escaped = &rest[..rest.find('"').unwrap()];
            let json = escaped
                .replace("&#34;", "\"")
                .replace("&#39;", "'")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&");
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}: {json}"))
        })
        .collect()
}

fn labels(chart: &Value) -> Vec<&str> {
    chart["data"]["datasets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["label"].as_str().unwrap())
        .collect()
}

/// The heatmap row of `room`: a cell per local hour, coloured and filled where `levels` has one.
fn heat_row(room: &str, levels: &[(usize, u8)]) -> String {
    let cells: String = (0..24)
        .map(|hour| match levels.iter().find(|(h, _)| *h == hour) {
            Some((_, level)) => format!(r#"<td class="heat-{level}">{level}</td>"#),
            None => "<td></td>".to_owned(),
        })
        .collect();
    format!(r#"<tr><th scope="row">{room}</th>{cells}</tr>"#)
}

/// Two residents and a guest, each with a report in January 2026; one PM2.5 reading each from
/// a resident and the guest.
async fn household() -> (TestApp, String, String) {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let first = app
        .log(
            &me,
            "2026-01-10T18:00:00Z",
            3,
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let event = first.report.event_id.unwrap();
    app.log(
        &partner,
        "2026-01-10T19:00:00Z",
        7,
        &[],
        EventChoice::Join(event),
    )
    .await;
    app.log(
        &guest,
        "2026-01-12T07:30:00Z",
        5,
        &[],
        EventChoice::New { replacing: None },
    )
    .await;
    app.log_pm25(&me, "2026-01-10T18:30:00Z", 35.5).await;
    app.log_pm25(&guest, "2026-01-12T07:35:00Z", 12.0).await;
    let resident = app.sign_in("daniele").await;
    let guest = app.sign_in("guest").await;
    (app, resident, guest)
}

#[tokio::test]
async fn residents_see_one_line_per_person_and_pm25_on_its_own_axis() {
    let (app, resident, _) = household().await;
    let html = text(app.send(get_with("/graphs", &resident)).await).await;
    assert!(
        html.contains(r#"<a href="/graphs" aria-current="page">"#),
        "{html}"
    );
    assert!(Page::parse(&html).exists("form.filters[method=get][action=\"/graphs\"]"));
    assert!(!html.contains("only your own"), "{html}");
    let intensity = &charts(&html)[0];
    assert_eq!(
        labels(intensity),
        ["daniele", "partner", "guest", "PM2.5 (µg/m³)"]
    );
    let datasets = &intensity["data"]["datasets"];
    assert_eq!(datasets[0]["yAxisID"], "level");
    assert_eq!(datasets[1]["data"][0]["y"], 7);
    assert_eq!(
        datasets[1]["data"][0]["x"],
        t("2026-01-10T19:00:00Z").timestamp_millis()
    );
    assert_eq!(datasets[3]["yAxisID"], "pm25");
    assert_eq!(datasets[3]["data"][0]["y"], 35.5);
    assert_eq!(datasets[3]["data"].as_array().unwrap().len(), 2);
    let scales = &intensity["options"]["scales"];
    assert_eq!(scales["level"]["max"], 10);
    assert_eq!(scales["pm25"]["position"], "right");
    assert_eq!(
        scales["x"]["min"],
        t("2025-12-31T23:00:00Z").timestamp_millis()
    );
    assert_eq!(intensity["time"]["zone"], "Europe/Rome");
    assert_eq!(intensity["time"]["locale"], "en-GB");
}

#[tokio::test]
async fn guests_see_graphs_of_their_own_reports_only() {
    let (app, _, guest) = household().await;
    let html = text(app.send(get_with("/graphs", &guest)).await).await;
    assert!(
        html.contains("These graphs show only your own reports and readings."),
        "{html}"
    );
    assert!(!html.contains("daniele"), "{html}");
    assert!(!html.contains("partner"), "{html}");
    let charts = charts(&html);
    assert_eq!(labels(&charts[0]), ["guest", "PM2.5 (µg/m³)"]);
    assert_eq!(charts[0]["data"]["datasets"][1]["data"][0]["y"], 12.0);
    let events: Vec<u64> = charts[1]["data"]["datasets"][0]["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_u64().unwrap())
        .collect();
    assert_eq!(events.iter().sum::<u64>(), 1, "only the guest's own event");
    assert_eq!(events[11], 1, "on 12 January");
    assert!(
        html.contains(&heat_row("Living room", &[(8, 5)])),
        "only the guest's own report, at 08:30 in Rome: {html}"
    );
}

#[tokio::test]
async fn a_month_counts_events_per_day() {
    let (app, resident, _) = household().await;
    let html = text(app.send(get_with("/graphs", &resident)).await).await;
    assert!(html.contains("<h2>Events per day</h2>"), "{html}");
    let events = &charts(&html)[1];
    let labels = events["data"]["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 31);
    assert_eq!(labels[0], "01/01");
    let counts = &events["data"]["datasets"][0]["data"];
    assert_eq!(counts[9], 1, "10 January: one event, two people");
    assert_eq!(counts[11], 1, "12 January");
    assert_eq!(
        counts
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| **n != 0)
            .count(),
        2
    );
}

#[tokio::test]
async fn this_year_counts_events_per_week() {
    let (app, resident, _) = household().await;
    let html = text(app.send(get_with("/graphs?range=year", &resident)).await).await;
    assert!(html.contains(r#"value="year" checked"#), "{html}");
    assert!(html.contains("<h2>Events per week</h2>"), "{html}");
    let events = &charts(&html)[1];
    let labels = events["data"]["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 53);
    assert_eq!(labels[0], "29/12");
    assert_eq!(labels[1], "05/01");
    let counts = &events["data"]["datasets"][0]["data"];
    assert_eq!(counts[1], 1, "the week of 10 January");
    assert_eq!(counts[2], 1, "the week of 12 January");
}

#[tokio::test]
async fn an_unusable_address_shows_this_month_and_an_empty_one_says_so() {
    let (app, resident, _) = household().await;
    app.at("2026-03-05T12:00:00Z");
    for path in ["/graphs", "/graphs?month=2026-13", "/graphs?year=1999"] {
        let response = app.send(get_with(path, &resident)).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let html = text(response).await;
        assert!(
            Page::parse(&html)
                .text("details.collapsible summary")
                .contains("This month"),
            "{path}: {html}"
        );
        assert!(
            html.contains("Nothing was logged in this period."),
            "{path}: {html}"
        );
        assert!(charts(&html).is_empty(), "{path}");
    }
}

#[tokio::test]
async fn the_graphs_need_a_sign_in() {
    let app = TestApp::new().await;
    let response = app.send(get("/graphs")).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));
}

#[tokio::test]
async fn the_room_table_averages_each_rooms_level_by_local_hour() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let living = app.room("room-living-room").await;
    let bedroom = app.room("room-bedroom").await;
    let at = |room, level| RoomLevel {
        room,
        level: Level::new(level).unwrap(),
    };
    let first = app
        .log(
            &me,
            "2026-01-10T18:00:00Z",
            6,
            &[living],
            EventChoice::New { replacing: None },
        )
        .await;
    let join = EventChoice::Join(first.report.event_id.unwrap());
    app.log_values(
        &me,
        "2026-01-10T18:40:00Z",
        vec![at(living, 3), at(bedroom, 0)],
        &[],
        join,
    )
    .await;
    app.no_smell(&me, "2026-01-11T09:00:00Z").await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with("/graphs", &cookie)).await).await;
    assert!(html.contains("<h2>Rooms by hour of day</h2>"), "{html}");
    let living_row = heat_row("Living room", &[(19, 5)]);
    let bedroom_row = heat_row("Bedroom", &[(19, 0)]);
    assert!(
        html.contains(&living_row),
        "(6 + 3) / 2 rounds to 5: {html}"
    );
    assert!(html.contains(&bedroom_row), "a clean check is a 0: {html}");
    assert!(html.find(&living_row) < html.find(&bedroom_row));
    assert_eq!(html.matches(r#"<th scope="row">"#).count(), 2, "{html}");
}

#[tokio::test]
async fn symptoms_and_areas_are_compared() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    let living = app.room("room-living-room").await;
    let stairwell = app.room("room-stairwell").await;
    let cough = app.symptom("symptom-cough").await;
    let headache = app.symptom("symptom-headache").await;
    let at = |room, level| RoomLevel {
        room,
        level: Level::new(level).unwrap(),
    };
    let first = app
        .log_values(
            &me,
            "2026-01-10T18:00:00Z",
            vec![at(living, 4), at(stairwell, 2)],
            &[cough, headache],
            EventChoice::New { replacing: None },
        )
        .await;
    let join = EventChoice::Join(first.report.event_id.unwrap());
    app.log_values(
        &partner,
        "2026-01-10T19:00:00Z",
        vec![at(living, 7)],
        &[cough],
        join,
    )
    .await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with("/graphs", &cookie)).await).await;
    let charts = charts(&html);
    let symptoms = &charts[2];
    assert_eq!(symptoms["options"]["indexAxis"], "y");
    assert_eq!(
        symptoms["data"]["labels"],
        serde_json::json!(["Cough", "Headache"])
    );
    assert_eq!(
        symptoms["data"]["datasets"][0]["data"],
        serde_json::json!([2, 1])
    );
    let areas = &charts[3];
    assert_eq!(labels(areas), ["Inside", "Common areas"]);
    assert_eq!(areas["data"]["datasets"][0]["data"][9], 7);
    assert_eq!(areas["data"]["datasets"][1]["data"][9], 2);
    assert_eq!(areas["data"]["datasets"][0]["data"][10], Value::Null);
    assert_eq!(areas["options"]["scales"]["y"]["max"], 10);
}

#[tokio::test]
async fn a_period_without_symptoms_or_rooms_says_so() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    app.no_smell(&me, "2026-01-11T09:00:00Z").await;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with("/graphs", &cookie)).await).await;
    assert!(
        html.contains("No room was checked in this period."),
        "{html}"
    );
    assert!(
        html.contains("No symptoms were reported in this period."),
        "{html}"
    );
    assert_eq!(charts(&html).len(), 3, "intensity, events and areas");
}

/// What the filters leave: the intensity chart's line labels and the heatmap's rooms.
async fn filtered(app: &TestApp, cookie: &str, query: &str) -> (Vec<String>, Vec<String>) {
    let html = text(
        app.send(get_with(&format!("/graphs?{query}"), cookie))
            .await,
    )
    .await;
    let lines = charts(&html)
        .first()
        .map(|chart| labels(chart).into_iter().map(str::to_owned).collect())
        .unwrap_or_default();
    (lines, Page::parse(&html).texts(".heatmap tbody th"))
}

/// A resident, a partner and a viewer; reports in three rooms at three times of day.
async fn rooms_and_times() -> (TestApp, String, Vec<(&'static str, i64)>) {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let stairwell = app.room("room-stairwell").await;
    let kitchen = app.room("room-kitchen").await;
    let at = |room, level| RoomLevel {
        room,
        level: Level::new(level).unwrap(),
    };
    // 23:00 in Rome: night.
    let night = app
        .log_values(
            &me,
            "2026-01-10T22:00:00Z",
            vec![at(bedroom, 6)],
            &[],
            EventChoice::New { replacing: None },
        )
        .await;
    let join = EventChoice::Join(night.report.event_id.unwrap());
    // 23:30: the partner, on the stairwell.
    app.log_values(
        &partner,
        "2026-01-10T22:30:00Z",
        vec![at(stairwell, 3)],
        &[],
        join,
    )
    .await;
    // 09:00 on the 12th: morning, the kitchen.
    app.log_values(
        &me,
        "2026-01-12T08:00:00Z",
        vec![at(kitchen, 4)],
        &[],
        EventChoice::New { replacing: None },
    )
    .await;
    // 15:00 on the 13th: a clean check of the whole home.
    app.no_smell(&me, "2026-01-13T14:00:00Z").await;
    let cookie = app.sign_in("daniele").await;
    let ids = vec![
        ("bedroom", bedroom.0),
        ("stairwell", stairwell.0),
        ("kitchen", kitchen.0),
        ("partner", partner.id.0),
    ];
    (app, cookie, ids)
}

#[tokio::test]
async fn each_filter_narrows_the_graphs() {
    let (app, cookie, ids) = rooms_and_times().await;
    let id = |name| ids.iter().find(|(n, _)| *n == name).unwrap().1;
    let strings = |items: &[&str]| items.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();

    let (lines, rooms) = filtered(&app, &cookie, "").await;
    assert_eq!(lines, strings(&["daniele", "partner"]));
    assert_eq!(
        rooms,
        strings(&["Kitchen", "Bedroom", "Stairwell (shared)"])
    );

    let dates = "from=2026-01-12&to=2026-01-31";
    assert_eq!(
        filtered(&app, &cookie, dates).await,
        (strings(&["daniele"]), strings(&["Kitchen"]))
    );

    let room = format!("room={}&area=common", id("bedroom"));
    assert_eq!(
        filtered(&app, &cookie, &room).await,
        (strings(&["daniele"]), strings(&["Bedroom"])),
        "rooms win over the area"
    );
    assert_eq!(
        filtered(&app, &cookie, "area=common").await,
        (strings(&["partner"]), strings(&["Stairwell (shared)"]))
    );

    let person = format!("by={}", id("partner"));
    assert_eq!(
        filtered(&app, &cookie, &person).await.0,
        strings(&["partner"])
    );

    assert_eq!(
        filtered(&app, &cookie, "time=morning").await.1,
        strings(&["Kitchen"])
    );
    assert_eq!(
        filtered(&app, &cookie, "time=night").await.1,
        strings(&["Bedroom", "Stairwell (shared)"])
    );

    assert_eq!(
        filtered(&app, &cookie, "min=5").await,
        (strings(&["daniele"]), strings(&["Bedroom"]))
    );

    let html = text(app.send(get_with("/graphs?clean=hide", &cookie)).await).await;
    let points = charts(&html)[0]["data"]["datasets"][0]["data"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(points, 2, "daniele's two reports without the clean check");
    let html = text(app.send(get_with("/graphs", &cookie)).await).await;
    let points = charts(&html)[0]["data"]["datasets"][0]["data"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(points, 3);
}

#[tokio::test]
async fn the_filters_card_shows_what_is_chosen() {
    let (app, cookie, ids) = rooms_and_times().await;
    let bedroom = ids[0].1;
    let path = format!(
        "/graphs?room={bedroom}&by={}&time=night&min=5&clean=hide",
        ids[3].1
    );
    let page = Page::parse(&text(app.send(get_with(&path, &cookie)).await).await);
    assert_eq!(
        page.text("details.collapsible summary"),
        "Filters · This month · Bedroom · partner · 22–06 · level 5+ · no clean checks Change"
    );
    assert!(page.exists(&format!("input[name=room][value='{bedroom}'][checked]")));
    assert_eq!(page.text("select[name=by] option[selected]"), "partner");
    assert!(page.exists("input[name=time][value=night][checked]"));
    assert_eq!(
        page.attr("input[type=range][name=min]", "value").as_deref(),
        Some("5")
    );
    assert_eq!(page.text("output[for=min-level]"), "5+");
    assert!(page.exists("input[name=clean][value=hide][checked]"));
    assert_eq!(
        page.attr("a.button[data-variant=secondary]", "href")
            .as_deref(),
        Some("/graphs")
    );
}

#[tokio::test]
async fn a_viewer_sees_everyone_and_may_choose_a_person() {
    let (app, _, _) = rooms_and_times().await;
    let viewer = app.sign_in("lawyer").await;
    let html = text(app.send(get_with("/graphs", &viewer)).await).await;
    assert!(!html.contains("only your own"), "{html}");
    let charts = charts(&html);
    assert_eq!(labels(&charts[0]), ["daniele", "partner"]);
    assert_eq!(
        Page::parse(&html).texts("select[name=by] option"),
        ["Everyone", "daniele", "lawyer", "partner"]
    );
}

#[tokio::test]
async fn guests_get_no_person_filter_and_cannot_pick_someone_else() {
    let (app, _, guest) = household().await;
    let html = text(app.send(get_with("/graphs?by=1", &guest)).await).await;
    assert!(!Page::parse(&html).exists("select[name=by]"), "{html}");
    assert_eq!(labels(&charts(&html)[0]), ["guest", "PM2.5 (µg/m³)"]);
    assert!(!html.contains("daniele"), "{html}");
}

#[tokio::test]
async fn a_repeated_filter_takes_the_first_value() {
    let (app, cookie, _) = rooms_and_times().await;
    let response = app
        .send(get_with("/graphs?range=7d&range=year&min=4&min=9", &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page = Page::parse(&text(response).await);
    assert!(page.exists("input[name=range][value='7d'][checked]"));
    assert_eq!(page.attr("input[name=min]", "value").as_deref(), Some("4"));
}
