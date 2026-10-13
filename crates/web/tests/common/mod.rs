//! An app on a temporary SQLite database with a fake clock, driven with in-memory requests.

#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::header::{CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE};
use axum::http::{HeaderName, Request, Response, StatusCode};
use chrono::TimeDelta;
use http_body_util::BodyExt;
use smoke_domain::clock::{Clock, FakeClock, SystemClock};
use smoke_domain::records::{NewUser, User};
use smoke_domain::repos::{Commit, Transactional, UserWrites};
use smoke_domain::types::{Locale, NewPassword, Role, Username};
use smoke_storage_sqlite::SqliteStore;
use smoke_web::config::{BaseUrl, WebConfig};
use smoke_web::{AppState, password};
use tempfile::TempDir;
use tower::ServiceExt;

pub const BASE: &str = "https://smoke.example.org";
pub const PASSWORD: &str = "correct horse battery";
pub const CLIENT: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10));

pub struct TestApp {
    pub router: Router,
    pub store: SqliteStore,
    pub clock: Arc<FakeClock>,
    pub db_url: String,
    _dir: TempDir,
}

impl TestApp {
    pub async fn new() -> Self {
        Self::with_config(BASE, None).await
    }

    /// `tower-sessions` stamps session expiry with the real time, so the fake clock starts at
    /// the real time too.
    pub async fn with_config(base_url: &str, client_ip_header: Option<&'static str>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db_url = format!("sqlite://{}", dir.path().join("web.db").display());
        let clock = Arc::new(FakeClock::new(SystemClock.now()));
        let store = SqliteStore::connect(&db_url, clock.clone()).await.unwrap();
        store.migrate().await.unwrap();
        let config = WebConfig {
            base_url: BaseUrl::parse(base_url).unwrap(),
            client_ip_header: client_ip_header.map(HeaderName::from_static),
        };
        let router = smoke_web::router(AppState::new(store.clone(), clock.clone(), config));
        Self {
            router,
            store,
            clock,
            db_url,
            _dir: dir,
        }
    }

    /// Guests and viewers expire in a week; residents never.
    pub async fn add_user(&self, name: &str, role: Role, locale: Locale) -> User {
        let hash = password::hash(&NewPassword::new(PASSWORD.to_owned()).unwrap()).unwrap();
        let mut tx = self.store.begin(None).await.unwrap();
        let user = tx
            .create_user(NewUser {
                username: Username::new(name).unwrap(),
                password_hash: Some(hash),
                role,
                locale,
                expires_at: (role != Role::Resident).then(|| self.clock.now() + TimeDelta::days(7)),
            })
            .await
            .unwrap();
        tx.commit().await.unwrap();
        user
    }

    pub async fn send(&self, request: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(request).await.unwrap()
    }

    pub async fn attempt(&self, username: &str, password: &str) -> Response<Body> {
        self.send(login_request(username, password)).await
    }

    /// Signs in and returns the session cookie as `name=value`.
    pub async fn sign_in(&self, username: &str) -> String {
        let response = self.attempt(username, PASSWORD).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "sign-in failed");
        session_cookie(&response).expect("a session cookie")
    }
}

pub fn get(path: &str) -> Request<Body> {
    Request::get(path).body(Body::empty()).unwrap()
}

pub fn get_with(path: &str, cookie: &str) -> Request<Body> {
    Request::get(path)
        .header(COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

/// A form post as a browser on the app's own pages sends it.
pub fn post_form(path: &str, body: &str, cookie: Option<&str>) -> Request<Body> {
    let mut builder = Request::post(path)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("origin", BASE)
        .header("sec-fetch-site", "same-origin");
    if let Some(cookie) = cookie {
        builder = builder.header(COOKIE, cookie);
    }
    let mut request = builder.body(Body::from(body.to_owned())).unwrap();
    from(&mut request, CLIENT);
    request
}

pub fn login_request(username: &str, password: &str) -> Request<Body> {
    let body = format!(
        "username={}&password={}",
        encode(username),
        encode(password)
    );
    post_form("/login", &body, None)
}

/// Marks the request as coming from `ip`, as the server's `ConnectInfo` would.
pub fn from(request: &mut Request<Body>, ip: IpAddr) {
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::new(ip, 40_000)));
}

fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' => (b as char).to_string(),
            b' ' => "+".to_owned(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn set_cookies(response: &Response<Body>) -> Vec<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect()
}

/// The session cookie a response sets, as `name=value`.
pub fn session_cookie(response: &Response<Body>) -> Option<String> {
    set_cookies(response)
        .into_iter()
        .find(|c| c.starts_with("__Host-session=") || c.starts_with("session="))
        .map(|c| c.split(';').next().unwrap().to_owned())
}

pub fn location(response: &Response<Body>) -> Option<&str> {
    response
        .headers()
        .get(LOCATION)
        .map(|v| v.to_str().unwrap())
}

pub async fn text(response: Response<Body>) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}
