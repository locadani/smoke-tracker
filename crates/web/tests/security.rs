//! Security headers, the cross-site request check and asset serving (spec §12).

mod common;

use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use common::*;

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
    for path in ["/missing", "/static/app.css", "/healthz"] {
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
