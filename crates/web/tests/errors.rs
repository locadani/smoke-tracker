//! Error pages: translated, and with the request id for internal errors (spec §12).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::types::{Locale, Role};
use sqlx::SqlitePool;

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

#[tokio::test]
async fn an_internal_error_shows_the_request_id_and_nothing_else() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let pool = SqlitePool::connect(&app.db_url).await.unwrap();
    sqlx::raw_sql("ALTER TABLE users RENAME TO users_gone")
        .execute(&pool)
        .await
        .unwrap();

    let response = app.send(get_with("/", &cookie)).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let html = text(response).await;
    assert!(
        html.contains(&format!("Something went wrong (ref: {request_id}).")),
        "{html}"
    );
    assert!(!html.contains("users"), "no internals leak: {html}");
}
