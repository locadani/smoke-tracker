use std::cell::Cell;
use std::error::Error;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind, panic_any};
use std::rc::Rc;

use smoke_exercises::ex10_middleware::{
    Handler, Request, Response, chain, panic_message, require_user, security_headers,
};

fn page(calls: Rc<Cell<u32>>) -> Handler {
    Box::new(move |_| {
        calls.set(calls.get() + 1);
        Response::new(200, "the page")
    })
}

#[test]
fn a_signed_in_request_reaches_the_page() {
    let calls = Rc::new(Cell::new(0));
    let app = require_user(page(calls.clone()));
    let response = app(&Request::new("/events", true));
    assert_eq!(response, Response::new(200, "the page"));
    assert_eq!(calls.get(), 1);
}

#[test]
fn a_signed_out_request_is_redirected_and_never_reaches_the_page() {
    let calls = Rc::new(Cell::new(0));
    let app = require_user(page(calls.clone()));
    let response = app(&Request::new("/events", false));
    assert_eq!(response.status, 303);
    assert_eq!(response.header("location"), Some("/login"));
    assert_eq!(calls.get(), 0);
}

#[test]
fn security_headers_are_added_to_a_response() {
    let app = security_headers(false, page(Rc::default()));
    let response = app(&Request::new("/", true));
    assert_eq!(response.body, "the page");
    assert_eq!(response.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(response.header("x-frame-options"), Some("DENY"));
    assert_eq!(response.header("referrer-policy"), Some("same-origin"));
    assert_eq!(response.header("cache-control"), Some("no-store"));
    assert_eq!(response.header("strict-transport-security"), None);
}

#[test]
fn strict_transport_security_only_over_https() {
    let app = security_headers(true, page(Rc::default()));
    let response = app(&Request::new("/", true));
    assert_eq!(
        response.header("strict-transport-security"),
        Some("max-age=63072000")
    );
}

#[test]
fn invite_links_send_no_referrer() {
    let app = security_headers(true, page(Rc::default()));
    let response = app(&Request::new("/invite/0123abcd", false));
    assert_eq!(response.header("referrer-policy"), Some("no-referrer"));
    let other = app(&Request::new("/people/invite", false));
    assert_eq!(other.header("referrer-policy"), Some("same-origin"));
}

#[test]
fn the_pages_own_cache_control_is_kept_but_other_headers_are_replaced() {
    let app = security_headers(
        false,
        Box::new(|_| {
            let mut response = Response::new(200, "asset");
            response.set_header("cache-control", "no-cache");
            response.set_header("x-frame-options", "SAMEORIGIN");
            response
        }),
    );
    let response = app(&Request::new("/static/app.css", true));
    assert_eq!(response.header("cache-control"), Some("no-cache"));
    assert_eq!(response.header("x-frame-options"), Some("DENY"));
    let frames = response
        .headers
        .iter()
        .filter(|(name, _)| name == "x-frame-options")
        .count();
    assert_eq!(frames, 1);
}

#[test]
fn layers_nest_so_a_redirect_gets_the_security_headers_too() {
    let calls = Rc::new(Cell::new(0));
    let app = security_headers(true, require_user(page(calls.clone())));
    let response = app(&Request::new("/events", false));
    assert_eq!(response.status, 303);
    assert_eq!(response.header("x-frame-options"), Some("DENY"));
    assert_eq!(response.header("cache-control"), Some("no-store"));
    assert_eq!(calls.get(), 0);
}

#[derive(Debug)]
struct Layer {
    text: &'static str,
    source: Option<Box<Layer>>,
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text)
    }
}

impl Error for Layer {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|layer| layer as &(dyn Error + 'static))
    }
}

#[test]
fn an_error_prints_with_all_its_causes() {
    let error = Layer {
        text: "database error",
        source: Some(Box::new(Layer {
            text: "no such table: users",
            source: None,
        })),
    };
    assert_eq!(chain(&error), "database error: no such table: users");
}

#[test]
fn an_error_without_a_cause_is_just_its_message() {
    let error = Layer {
        text: "not found",
        source: None,
    };
    assert_eq!(chain(&error), "not found");
}

#[test]
fn three_levels_are_all_listed() {
    let error = Layer {
        text: "request failed",
        source: Some(Box::new(Layer {
            text: "database error",
            source: Some(Box::new(Layer {
                text: "disk full",
                source: None,
            })),
        })),
    };
    assert_eq!(chain(&error), "request failed: database error: disk full");
}

fn caught(work: impl FnOnce()) -> Box<dyn std::any::Any + Send> {
    catch_unwind(AssertUnwindSafe(work)).unwrap_err()
}

#[test]
fn a_literal_panic_message_is_recovered() {
    assert_eq!(panic_message(caught(|| panic!("boom"))), "boom");
}

#[test]
fn a_formatted_panic_message_is_recovered() {
    let id = 7;
    assert_eq!(
        panic_message(caught(|| panic!("event {id} vanished"))),
        "event 7 vanished"
    );
}

#[test]
fn an_unknown_panic_payload_gets_a_generic_message() {
    assert_eq!(
        panic_message(caught(|| panic_any(42_u32))),
        "a handler panicked"
    );
}
