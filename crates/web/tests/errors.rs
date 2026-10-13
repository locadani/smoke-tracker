//! Error pages: translated, and with the request id for internal errors (spec §12).

mod common;

use axum::http::StatusCode;
use common::*;

#[tokio::test]
async fn unknown_pages_get_a_translated_not_found_page() {
    let app = TestApp::new().await;
    let response = app.send(get("/nothing-here")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers()["content-type"],
        "text/html; charset=utf-8"
    );
    assert!(text(response).await.contains("<h1>Page not found</h1>"));

    let italian = app.send(get_with("/nothing-here", "lang=it")).await;
    assert!(text(italian).await.contains("<h1>Pagina non trovata</h1>"));

    let turkish = app
        .send(
            axum::http::Request::get("/nothing-here")
                .header("accept-language", "tr-TR,tr;q=0.9")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    assert!(text(turkish).await.contains("<h1>Sayfa bulunamadı</h1>"));
}
