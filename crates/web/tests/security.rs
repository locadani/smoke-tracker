//! Security headers, the cross-site request check and asset serving (spec §12).

mod common;

use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use common::page::Page;
use common::*;
use smoke_domain::types::{Locale, Role};

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; \
    connect-src 'self'; font-src 'self'; manifest-src 'self'; worker-src 'self'; \
    object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

#[tokio::test]
async fn every_response_carries_the_security_headers() {
    let app = TestApp::new().await;
    for path in ["/missing", "/static/app.css", "/healthz"] {
        let response = app.send(get(path)).await;
        let headers = response.headers();
        assert_eq!(headers["content-security-policy"], CSP, "{path}");
        assert_eq!(headers["x-content-type-options"], "nosniff", "{path}");
        assert_eq!(headers["referrer-policy"], "same-origin", "{path}");
        assert_eq!(headers["x-frame-options"], "DENY", "{path}");
        assert_eq!(
            headers["cross-origin-opener-policy"], "same-origin",
            "{path}"
        );
        assert_eq!(
            headers["strict-transport-security"], "max-age=63072000",
            "{path}"
        );
        assert_eq!(headers["x-request-id"].len(), 16, "{path}");
    }
    let page = app.send(get("/missing")).await;
    assert_eq!(page.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn anonymous_requests_set_no_cookies() {
    let app = TestApp::new().await;
    for path in [
        "/missing",
        "/static/app.css",
        "/healthz",
        "/sw.js",
        "/install",
        "/offline",
    ] {
        let response = app.send(get(path)).await;
        assert!(set_cookies(&response).is_empty(), "{path}");
    }
}

#[tokio::test]
async fn cross_site_posts_are_refused_with_a_page() {
    let app = TestApp::new().await;
    let cases: [&[(&str, &str)]; 4] = [
        &[("sec-fetch-site", "cross-site"), ("origin", BASE)],
        &[("sec-fetch-site", "same-site")],
        &[("origin", "https://evil.example")],
        &[],
    ];
    for headers in cases {
        let mut builder =
            Request::post("/login").header(CONTENT_TYPE, "application/x-www-form-urlencoded");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let request = builder
            .body(Body::from(format!("username=daniele&password={PASSWORD}")))
            .unwrap();
        let response = app.send(request).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{headers:?}");
        assert!(session_cookie(&response).is_none());
        assert!(
            text(response).await.contains("<h1>Request blocked</h1>"),
            "{headers:?}"
        );
    }
}

#[tokio::test]
async fn reading_pages_is_never_origin_checked() {
    let app = TestApp::new().await;
    let request = Request::get("/missing")
        .header("sec-fetch-site", "cross-site")
        .body(Body::empty())
        .unwrap();
    assert_eq!(app.send(request).await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn assets_are_served_with_their_type_and_revalidated() {
    let app = TestApp::new().await;
    let response = app.send(get("/static/htmx-2.0.11.min.js")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/javascript; charset=utf-8"
    );
    assert_eq!(response.headers()["cache-control"], "no-cache");
    let etag = response.headers()["etag"].clone();

    let revalidate = Request::get("/static/htmx-2.0.11.min.js")
        .header("if-none-match", etag)
        .body(Body::empty())
        .unwrap();
    let not_modified = app.send(revalidate).await;
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
    assert!(text(not_modified).await.is_empty());

    assert_eq!(
        app.send(get("/static/../Cargo.toml")).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(text(app.send(get("/healthz")).await).await, "ok");
}

#[tokio::test]
async fn on_plain_http_localhost_there_is_no_hsts_and_no_secure_cookie() {
    let app = TestApp::with_config("http://localhost:8080", None).await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let mut request = login_request("daniele", PASSWORD);
    request
        .headers_mut()
        .insert("origin", "http://localhost:8080".parse().unwrap());
    request.headers_mut().remove("sec-fetch-site");
    let response = app.send(request).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(!response.headers().contains_key("strict-transport-security"));
    let cookie = set_cookies(&response)
        .into_iter()
        .find(|c| c.starts_with("session="))
        .expect("a plain session cookie");
    assert!(!cookie.contains("Secure"), "{cookie}");
    assert!(cookie.contains("HttpOnly"), "{cookie}");
}

#[tokio::test]
async fn the_origin_check_uses_the_configured_url_not_the_host_header() {
    let app = TestApp::with_config("https://smoke.example.org/", None).await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let mut request = login_request("daniele", PASSWORD);
    request.headers_mut().remove("sec-fetch-site");
    request
        .headers_mut()
        .insert("host", "10.0.0.5:8080".parse().unwrap());
    assert_eq!(app.send(request).await.status(), StatusCode::SEE_OTHER);
}

/// The first event-handler attribute (whitespace, `on`, lowercase letters, `=`) in `html`.
fn event_handler_attribute(html: &str) -> Option<&str> {
    let bytes = html.as_bytes();
    (0..bytes.len()).find_map(|start| {
        if !bytes[start].is_ascii_whitespace() || !bytes[start + 1..].starts_with(b"on") {
            return None;
        }
        let letters = bytes[start + 3..]
            .iter()
            .take_while(|b| b.is_ascii_lowercase())
            .count();
        let equals = start + 3 + letters;
        (letters > 0 && bytes.get(equals) == Some(&b'=')).then(|| &html[start + 1..=equals])
    })
}

#[test]
fn the_event_handler_scan_finds_attributes_and_not_words() {
    assert_eq!(
        event_handler_attribute(r#"<input onfocus="x">"#),
        Some("onfocus=")
    );
    assert_eq!(
        event_handler_attribute("<form\nonsubmit=\"x\">"),
        Some("onsubmit=")
    );
    assert_eq!(
        event_handler_attribute("<p>once upon a time, on=1</p>"),
        None
    );
    assert_eq!(
        event_handler_attribute(r#"<a data-on="1" href="/on">"#),
        None
    );
}

#[tokio::test]
async fn no_page_needs_inline_scripts_styles_or_eval_and_every_icon_exists() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    app.send(post_form("/log", "action=no-smell", Some(&cookie)))
        .await;
    app.send(post_form(
        "/log",
        "room=1&level-1=4&action=smell",
        Some(&cookie),
    ))
    .await;
    let question = app
        .send(post_form(
            "/log",
            "room=1&level-1=5&action=smell",
            Some(&cookie),
        ))
        .await;
    let past_check_question = app
        .send(post_form(
            "/log",
            "action=no-smell&observed_at=2026-01-15T12:00",
            Some(&cookie),
        ))
        .await;
    let pages = [
        text(question).await,
        text(past_check_question).await,
        text(app.send(get_with("/events", &cookie)).await).await,
        text(app.send(get_with("/events/1", &cookie)).await).await,
        text(app.send(get_with("/reports/1", &cookie)).await).await,
        text(app.send(get_with("/reports/2", &cookie)).await).await,
        text(app.send(get_with("/reports/2/correct", &cookie)).await).await,
        text(app.send(get("/login")).await).await,
        text(app.send(get_with("/", &cookie)).await).await,
        text(app.send(get_with("/guide/rating", &cookie)).await).await,
        text(app.send(get_with("/settings", &cookie)).await).await,
        text(app.send(get_with("/people", &cookie)).await).await,
        text(app.send(get_with("/people/invite", &cookie)).await).await,
        text(
            app.send(post_form("/people/1/link", "", Some(&cookie)))
                .await,
        )
        .await,
        text(app.send(get_with("/install?welcome", &cookie)).await).await,
        text(app.send(get("/offline")).await).await,
        text(app.send(get_with("/graphs", &cookie)).await).await,
        text(app.send(get_with("/graphs?year=2026", &cookie)).await).await,
        text(app.send(get_with("/guide/logging", &cookie)).await).await,
        text(app.send(get("/missing")).await).await,
        text(app.attempt("daniele", "not the password").await).await,
    ];
    assert!(pages[1].contains("No event: just a check"), "{}", pages[1]);
    assert!(pages[4].contains("None: a no-smell check"), "{}", pages[4]);
    let link = pages[13].find("/invite/").expect("a reset link");
    let accept = &pages[13][link..link + "/invite/".len() + 64];
    let accept_page = text(app.send(get(accept)).await).await;
    assert!(
        accept_page.contains("<h1>Choose a new password</h1>"),
        "{accept_page}"
    );
    let pages = pages.into_iter().chain([accept_page]);
    let sprite = include_str!("../static/icons.svg");
    for html in pages {
        assert!(
            html.contains(r#"<meta name="htmx-config" content='{"includeIndicatorStyles":false,"allowEval":false,"allowScriptTags":false}'>"#),
            "htmx must not inject styles or eval: {html}"
        );
        for forbidden in [
            "<style",
            " style=",
            "<script>",
            " onclick=",
            " onload=",
            " hx-on",
            "javascript:",
        ] {
            assert!(!html.contains(forbidden), "{forbidden} in {html}");
        }
        assert_eq!(event_handler_attribute(&html), None, "{html}");
        for script in html.match_indices("<script").map(|(i, _)| &html[i..]) {
            assert!(script.starts_with("<script src=\"/static/"), "{script}");
        }
        for icon in Page::parse(&html).attrs("svg use", "href") {
            let id = icon.strip_prefix("/static/icons.svg#").expect("the sprite");
            assert!(sprite.contains(&format!("<symbol id=\"{id}\"")), "{id}");
        }
    }
}

/// Every template's source, by file name.
fn templates() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/templates");
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&path).unwrap())
        })
        .collect()
}

#[test]
fn no_template_has_inline_styles_or_scripts() {
    let templates = templates();
    assert!(templates.iter().any(|(name, _)| name == "base.html"));
    for (name, source) in templates {
        for forbidden in ["<style", " style=", "javascript:", " hx-on"] {
            assert!(!source.contains(forbidden), "{forbidden} in {name}");
        }
        assert_eq!(event_handler_attribute(&source), None, "{name}");
        for script in source.match_indices("<script").map(|(i, _)| &source[i..]) {
            let tag = &script[..script.find('>').unwrap()];
            assert!(tag.contains(" src=\"/static/"), "{name}: {tag}");
            assert!(
                script[tag.len()..].starts_with("></script>"),
                "{name}: a script body"
            );
        }
    }
}

#[tokio::test]
async fn oversized_form_posts_are_refused() {
    let app = TestApp::new().await;
    let body = format!("username=daniele&password={}", "x".repeat(20 * 1024));
    let response = app.send(post_form("/login", &body, None)).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
