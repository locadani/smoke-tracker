//! Uploading a sensor photo with the Log form, reading it back, and serving it (photo spec §4,
//! §7, §9).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::records::LogEntry;
use smoke_domain::repos::{EntryRepo, EventRepo};
use smoke_domain::types::{Locale, Role};

const NOW: &str = "2026-11-02T20:00:00Z";
const TAKEN: &str = "2026-11-02T19:58:30Z";
const LIMIT: usize = 307_200;

async fn entries(app: &TestApp) -> Vec<LogEntry> {
    app.store
        .entries_between(t("2026-01-01T00:00:00Z"), t("2027-01-01T00:00:00Z"))
        .await
        .unwrap()
}

/// The `photo_sha256`, `photo_bytes` and `photo_taken_at` values a page carries as hidden fields.
fn references(html: &str) -> Vec<(String, String)> {
    ["photo_sha256", "photo_bytes", "photo_taken_at"]
        .into_iter()
        .map(|name| {
            let marker = format!(r#"name="{name}" value=""#);
            let start = html
                .find(&marker)
                .unwrap_or_else(|| panic!("{name} in {html}"))
                + marker.len();
            let end = start + html[start..].find('"').unwrap();
            (name.to_owned(), html[start..end].to_owned())
        })
        .collect()
}

fn form_body(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", v.replace('+', "%2B").replace(':', "%3A")))
        .collect::<Vec<_>>()
        .join("&")
}

async fn signed_in_resident() -> (TestApp, String) {
    let app = TestApp::new().await;
    app.at(NOW);
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    (app, cookie)
}

#[tokio::test]
async fn a_reading_with_a_photo_is_saved() {
    let (app, cookie) = signed_in_resident().await;
    let photo = jpeg(2_000, 1);
    let response = app
        .send(post_multipart(
            "/log",
            &[
                ("pm25", "19.0"),
                ("photo_taken_at", TAKEN),
                ("action", "log"),
            ],
            Some(&photo),
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/?saved=pm25"));

    let all = entries(&app).await;
    let reading = all[0].reading.as_ref().unwrap();
    let stored = reading.photo.as_ref().expect("a photo");
    assert_eq!(stored.bytes, 2_000);
    assert_eq!(stored.taken_at, t(TAKEN));
    assert!(
        stored.key.as_str().starts_with("photos/2026/11/"),
        "{:?}",
        stored.key
    );

    let served = app
        .send(get_with(&format!("/photos/{}", reading.id), &cookie))
        .await;
    assert_eq!(served.status(), StatusCode::OK);
    assert_eq!(served.headers()["content-type"], "image/jpeg");
    let bytes = http_body_util::BodyExt::collect(served.into_body())
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(bytes.as_ref(), photo.as_slice());
}

#[tokio::test]
async fn a_photo_survives_the_event_question() {
    let (app, cookie) = signed_in_resident().await;
    let me = app.add_user("other", Role::Resident, Locale::En).await;
    app.log(
        &me,
        "2026-11-02T18:05:00Z",
        4,
        &[],
        EventChoice::New { replacing: None },
    )
    .await;
    let photo = jpeg(3_000, 2);
    let question = app
        .send(post_multipart(
            "/log",
            &[
                ("room", "1"),
                ("level-1", "5"),
                ("pm25", "21"),
                ("photo_taken_at", TAKEN),
                ("action", "log"),
            ],
            Some(&photo),
            Some(&cookie),
        ))
        .await;
    assert_eq!(question.status(), StatusCode::OK);
    let html = text(question).await;
    assert!(html.contains("Same smell still going?"), "{html}");
    assert!(
        entries(&app).await.iter().all(|e| e.reading.is_none()),
        "nothing saved yet"
    );
    let mut answer = references(&html);
    assert_eq!(answer[1].1, "3000");
    let event = app.store.open_event().await.unwrap().unwrap();
    answer.extend([
        ("room".to_owned(), "1".to_owned()),
        ("level-1".to_owned(), "5".to_owned()),
        ("pm25".to_owned(), "21".to_owned()),
        ("action".to_owned(), "log".to_owned()),
        ("asked".to_owned(), format!("same-smell-{}", event.id)),
        ("choice".to_owned(), format!("join-{}", event.id)),
    ]);
    let response = app
        .send(post_form("/log", &form_body(&answer), Some(&cookie)))
        .await;
    assert_eq!(location(&response), Some("/?saved=report"));
    let with_photo: Vec<_> = entries(&app)
        .await
        .into_iter()
        .filter_map(|e| e.reading.and_then(|r| r.photo))
        .collect();
    assert_eq!(with_photo.len(), 1);
    assert_eq!(with_photo[0].bytes, 3_000);
}

#[tokio::test]
async fn a_photo_survives_a_refused_form() {
    let (app, cookie) = signed_in_resident().await;
    let photo = jpeg(1_500, 3);
    let refused = app
        .send(post_multipart(
            "/log",
            &[
                ("pm25", "21"),
                ("observed_at", "2030-01-01T10:00"),
                ("photo_taken_at", TAKEN),
                ("action", "log"),
            ],
            Some(&photo),
            Some(&cookie),
        ))
        .await;
    assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = text(refused).await;
    assert!(html.contains("cannot be in the future"), "{html}");
    assert!(entries(&app).await.is_empty());
    let page = common::page::Page::parse(&html);
    assert!(page.exists("[data-photo-attached]:not([hidden])"), "{html}");
    assert!(page.exists("[data-photo-remove][type=button]"), "{html}");
    assert_eq!(page.count("input[name=photo_sha256]"), 1, "{html}");

    let mut again = references(&html);
    again.extend([
        ("pm25".to_owned(), "21".to_owned()),
        ("action".to_owned(), "log".to_owned()),
    ]);
    let response = app
        .send(post_form("/log", &form_body(&again), Some(&cookie)))
        .await;
    assert_eq!(location(&response), Some("/?saved=pm25"));
    let all = entries(&app).await;
    assert_eq!(all.len(), 1);
    assert_eq!(
        all[0]
            .reading
            .as_ref()
            .unwrap()
            .photo
            .as_ref()
            .unwrap()
            .bytes,
        1_500
    );
}

#[tokio::test]
async fn a_forged_photo_reference_is_refused() {
    let (app, cookie) = signed_in_resident().await;
    let sha = "a".repeat(64);
    let upload = jpeg(1_000, 4);
    let first = app
        .send(post_multipart(
            "/log",
            &[("pm25", "x"), ("photo_taken_at", TAKEN)],
            Some(&upload),
            Some(&cookie),
        ))
        .await;
    let real = references(&text(first).await);

    let wrong_length = vec![
        ("photo_sha256".to_owned(), real[0].1.clone()),
        ("photo_bytes".to_owned(), "999".to_owned()),
        ("photo_taken_at".to_owned(), TAKEN.to_owned()),
    ];
    let cases = [
        // Never uploaded.
        vec![
            ("photo_sha256".to_owned(), sha.clone()),
            ("photo_bytes".to_owned(), "1000".to_owned()),
            ("photo_taken_at".to_owned(), TAKEN.to_owned()),
        ],
        // Not a fingerprint.
        vec![
            ("photo_sha256".to_owned(), "../../etc/passwd".to_owned()),
            ("photo_bytes".to_owned(), "1000".to_owned()),
            ("photo_taken_at".to_owned(), TAKEN.to_owned()),
        ],
        // The right file with another length.
        wrong_length,
        // Zero, too large, or missing pieces.
        vec![
            ("photo_sha256".to_owned(), real[0].1.clone()),
            ("photo_bytes".to_owned(), "0".to_owned()),
            ("photo_taken_at".to_owned(), TAKEN.to_owned()),
        ],
        vec![
            ("photo_sha256".to_owned(), real[0].1.clone()),
            ("photo_bytes".to_owned(), "9999999".to_owned()),
            ("photo_taken_at".to_owned(), TAKEN.to_owned()),
        ],
        vec![("photo_sha256".to_owned(), real[0].1.clone())],
    ];
    for mut case in cases {
        case.extend([
            ("pm25".to_owned(), "21".to_owned()),
            ("action".to_owned(), "log".to_owned()),
        ]);
        let response = app
            .send(post_form("/log", &form_body(&case), Some(&cookie)))
            .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{case:?}"
        );
        let html = text(response).await;
        assert!(
            html.contains("The photo could not be used"),
            "{case:?}: {html}"
        );
        assert!(
            !html.contains("photo_sha256"),
            "a refused reference is not carried on"
        );
    }
    assert!(entries(&app).await.is_empty());
}

#[tokio::test]
async fn a_photo_without_pm25_is_refused() {
    let (app, cookie) = signed_in_resident().await;
    let response = app
        .send(post_multipart(
            "/log",
            &[
                ("room", "1"),
                ("level-1", "5"),
                ("photo_taken_at", TAKEN),
                ("action", "log"),
            ],
            Some(&jpeg(1_000, 5)),
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = text(response).await;
    assert!(html.contains("A photo needs a PM2.5 value"), "{html}");
    assert!(entries(&app).await.is_empty());
}

#[tokio::test]
async fn an_oversized_photo_is_refused() {
    let (app, cookie) = signed_in_resident().await;
    for len in [LIMIT + 1, LIMIT + 40_000] {
        let response = app
            .send(post_multipart(
                "/log",
                &[("pm25", "21"), ("photo_taken_at", TAKEN)],
                Some(&jpeg(len, 6)),
                Some(&cookie),
            ))
            .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "{len}");
    }
    assert!(entries(&app).await.is_empty());
}

#[tokio::test]
async fn a_photo_of_exactly_the_limit_is_accepted_by_both_routes() {
    let (app, cookie) = signed_in_resident().await;
    let photo = jpeg(LIMIT, 7);
    let read = app
        .send(post_multipart(
            "/log/pm25/read",
            &[],
            Some(&photo),
            Some(&cookie),
        ))
        .await;
    assert_eq!(read.status(), StatusCode::OK);

    let response = app
        .send(post_multipart(
            "/log",
            &[
                ("pm25", "19.0"),
                ("photo_taken_at", TAKEN),
                ("action", "log"),
            ],
            Some(&photo),
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/?saved=pm25"));
    let all = entries(&app).await;
    let stored = all[0].reading.as_ref().unwrap().photo.as_ref().unwrap();
    assert_eq!(stored.bytes as usize, LIMIT);
}

#[tokio::test]
async fn a_png_is_refused() {
    let (app, cookie) = signed_in_resident().await;
    let png = [&[0x89, b'P', b'N', b'G'][..], &[7u8; 500][..]].concat();
    let response = app
        .send(post_multipart(
            "/log",
            &[("pm25", "21"), ("photo_taken_at", TAKEN)],
            Some(&png),
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text(response).await.contains("The photo could not be used"));
    assert!(entries(&app).await.is_empty());

    let read = app
        .send(post_multipart(
            "/log/pm25/read",
            &[],
            Some(&png),
            Some(&cookie),
        ))
        .await;
    assert_eq!(read.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn anonymous_photo_posts_are_refused_before_the_body() {
    let app = TestApp::new().await;
    let huge = jpeg(LIMIT + 200_000, 7);
    for path in ["/log", "/log/pm25/read"] {
        let response = app
            .send(post_multipart(
                path,
                &[("photo_taken_at", TAKEN)],
                Some(&huge),
                None,
            ))
            .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(location(&response), Some("/login"), "{path}");
    }
}

#[tokio::test]
async fn the_read_route_returns_null_without_a_reader() {
    let (app, cookie) = signed_in_resident().await;
    let response = app
        .send(post_multipart(
            "/log/pm25/read",
            &[],
            Some(&jpeg(800, 8)),
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_eq!(text(response).await, r#"{"value":null}"#);
    assert!(entries(&app).await.is_empty());

    let none = app
        .send(post_multipart("/log/pm25/read", &[], None, Some(&cookie)))
        .await;
    assert_eq!(none.status(), StatusCode::BAD_REQUEST);
    let big = app
        .send(post_multipart(
            "/log/pm25/read",
            &[],
            Some(&jpeg(LIMIT + 1, 8)),
            Some(&cookie),
        ))
        .await;
    assert_eq!(big.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn guests_cannot_see_other_peoples_photos() {
    let (app, cookie) = signed_in_resident().await;
    app.add_user("visitor", Role::Guest, Locale::En).await;
    let guest = app.sign_in("visitor").await;
    app.send(post_multipart(
        "/log",
        &[("pm25", "19"), ("photo_taken_at", TAKEN)],
        Some(&jpeg(900, 9)),
        Some(&cookie),
    ))
    .await;
    let id = entries(&app).await[0].reading.as_ref().unwrap().id;
    let path = format!("/photos/{id}");
    assert_eq!(
        app.send(get_with(&path, &guest)).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(app.send(get(&path)).await.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        app.send(get_with(&path, &cookie)).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        app.send(get_with("/photos/999", &cookie)).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_reading_without_a_photo_has_no_photo_page() {
    let (app, cookie) = signed_in_resident().await;
    let me = app.add_user("other", Role::Resident, Locale::En).await;
    let reading = app.log_pm25(&me, "2026-11-02T19:00:00Z", 12.0).await;
    let response = app
        .send(get_with(&format!("/photos/{}", reading.id), &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn photos_are_never_cached() {
    let (app, cookie) = signed_in_resident().await;
    app.send(post_multipart(
        "/log",
        &[("pm25", "19"), ("photo_taken_at", TAKEN)],
        Some(&jpeg(900, 9)),
        Some(&cookie),
    ))
    .await;
    let id = entries(&app).await[0].reading.as_ref().unwrap().id;
    let response = app.send(get_with(&format!("/photos/{id}"), &cookie)).await;
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let worker =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/static/sw.js")).unwrap();
    assert!(worker.contains("/photos/") && worker.contains("/log/pm25/read"));
}

#[tokio::test]
async fn other_routes_keep_the_16_kb_limit() {
    let (app, cookie) = signed_in_resident().await;
    let padding = "x".repeat(20 * 1024);
    let response = app
        .send(post_form(
            "/settings/password",
            &format!("current={padding}"),
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body = format!("pm25=19&note={}", "x".repeat(20 * 1024));
    let urlencoded = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(urlencoded.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
