//! An app on a temporary SQLite database with a fake clock, driven with in-memory requests.

#![allow(dead_code)]

pub mod page;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::header::{CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE};
use axum::http::{HeaderName, Request, Response, StatusCode};
use chrono::{DateTime, TimeDelta, Utc};
use http_body_util::BodyExt;
use smoke_domain::clock::{Clock, FakeClock, SystemClock};
use smoke_domain::logbook::{EventChoice, Logged, SmellEntry, log_reading, log_smell};
use smoke_domain::records::{NewUser, RoomLevel, User};
use smoke_domain::records::{Pm25Reading, ReportValues};
use smoke_domain::repos::{Commit, ListRepo, Transactional, UserWrites};
use smoke_domain::schedule::DailySchedule;
use smoke_domain::starter::seed_starter_lists;
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{Level, Pm25, RoomId, SymptomId};
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
        seed_starter_lists(&store).await.unwrap();
        let config = WebConfig {
            base_url: BaseUrl::parse(base_url).unwrap(),
            client_ip_header: client_ip_header.map(HeaderName::from_static),
            schedule: schedule(),
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

    /// Sets the fake clock. Sessions still work: `tower-sessions` stamps their expiry with the
    /// real time, which is later than any instant the tests use.
    pub fn at(&self, instant: &str) {
        self.clock.set(t(instant));
    }

    /// The id of a starter room, e.g. `room("room-bedroom")`.
    pub async fn room(&self, key: &str) -> RoomId {
        let rooms = self.store.rooms().await.unwrap();
        rooms
            .iter()
            .find(|r| r.i18n_key.as_deref() == Some(key))
            .unwrap()
            .id
    }

    /// The id of a starter symptom, e.g. `symptom("symptom-cough")`.
    pub async fn symptom(&self, key: &str) -> SymptomId {
        let symptoms = self.store.symptoms().await.unwrap();
        symptoms
            .iter()
            .find(|s| s.i18n_key.as_deref() == Some(key))
            .unwrap()
            .id
    }

    /// Logs a report through the domain, as the Log form would, at `at` (RFC 3339): every room
    /// in `rooms` at `level`, or the living room alone when `rooms` is empty.
    pub async fn log(
        &self,
        user: &User,
        at: &str,
        level: u8,
        rooms: &[RoomId],
        choice: EventChoice,
    ) -> Logged {
        let level = Level::new(level).unwrap();
        let rooms = match rooms {
            [] => vec![self.room("room-living-room").await],
            rooms => rooms.to_vec(),
        };
        let rooms = rooms
            .into_iter()
            .map(|room| RoomLevel { room, level })
            .collect();
        self.log_values(user, at, rooms, &[], choice).await
    }

    /// Logs "No smell" for the whole home through the domain, at `at` (RFC 3339).
    pub async fn no_smell(&self, user: &User, at: &str) -> Logged {
        let choice = EventChoice::New { replacing: None };
        self.log_values(user, at, vec![], &[], choice).await
    }

    /// Logs a report with these room levels and symptoms through the domain, at `at`.
    pub async fn log_values(
        &self,
        user: &User,
        at: &str,
        rooms: Vec<RoomLevel>,
        symptoms: &[SymptomId],
        choice: EventChoice,
    ) -> Logged {
        let entry = SmellEntry {
            observed_at: t(at),
            values: ReportValues::new(rooms, symptoms.to_vec(), None).unwrap(),
            pm25: None,
        };
        let mut tx = self.store.begin(Some(user.id)).await.unwrap();
        let logged = log_smell(
            &mut tx,
            &user.actor(),
            entry,
            choice,
            &schedule(),
            self.clock.now(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        logged
    }

    /// Logs a PM2.5 reading on its own, as the Log button does with only a PM2.5 value.
    pub async fn log_pm25(&self, user: &User, at: &str, value: f64) -> Pm25Reading {
        let mut tx = self.store.begin(Some(user.id)).await.unwrap();
        let reading = log_reading(
            &mut tx,
            &user.actor(),
            t(at),
            Pm25::new(value).unwrap(),
            self.clock.now(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        reading
    }

    /// Runs the auto-close sweep at the fake clock's time, as the server's timer would.
    pub async fn sweep(&self) {
        auto_close(&self.store, &schedule(), self.clock.now())
            .await
            .unwrap();
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

pub fn t(instant: &str) -> DateTime<Utc> {
    instant.parse().unwrap()
}

/// Rome, auto-close at 04:00: what the test app is configured with.
pub fn schedule() -> DailySchedule {
    DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap()
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
        .find(|c| c.starts_with("__Host-session="))
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
