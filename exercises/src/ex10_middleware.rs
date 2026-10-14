//! Exercise 10: middleware as closures, and errors on their way out.
//!
//! Practises: `docs/learning/03-web-and-auth.md`, sections 3 and 7.
//! Mirrors: `smoke_web::auth::require_user`, `smoke_web::security::headers`,
//! `smoke_web::error::{chain, panic_response}`.
//! Run: `cargo test --test ex10_middleware`
//!
//! Hint: a middleware takes the next handler and returns a new handler that may act before and
//! after calling it. Return `Box::new(move |request| { ... })`: `move` makes the closure take
//! ownership of `next` (and any flag), and `Box<dyn Fn>` lets every layer have the same type. `Fn`
//! (not `FnMut`) is enough because the layers keep no state of their own.

use std::any::Any;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub path: String,
    pub signed_in: bool,
}

impl Request {
    pub fn new(path: &str, signed_in: bool) -> Self {
        Self {
            path: path.to_owned(),
            signed_in,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    pub fn new(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.to_owned(),
        }
    }

    /// Given: a `303 See Other` to `location`.
    pub fn redirect(location: &str) -> Self {
        let mut response = Self::new(303, "");
        response.set_header("location", location);
        response
    }

    /// Given: the value of a header (names are lowercase).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Given: sets a header, replacing an earlier one of the same name.
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.retain(|(key, _)| key != name);
        self.headers.push((name.to_owned(), value.to_owned()));
    }
}

pub type Handler = Box<dyn Fn(&Request) -> Response>;

/// A signed-out request never reaches `next`: it is redirected to `/login`.
pub fn require_user(next: Handler) -> Handler {
    todo!()
}

/// Calls `next`, then adds to its response (replacing any header of the same name):
///
/// - `x-content-type-options: nosniff` and `x-frame-options: DENY`
/// - `referrer-policy: no-referrer` for paths under `/invite/`, `same-origin` for the rest
/// - `strict-transport-security: max-age=63072000`, only when `https` is true
/// - `cache-control: no-store`, only if the response did not set its own
pub fn security_headers(https: bool, next: Handler) -> Handler {
    todo!()
}

/// An error and all its sources, joined with ": ", for example
/// `database error: no such table: users`. Walk `Error::source` until it returns `None`.
pub fn chain(error: &(dyn Error + 'static)) -> String {
    todo!()
}

/// The message of a caught panic (what `std::panic::catch_unwind` returns in its `Err`). A
/// `panic!("literal")` carries a `&'static str`, a `panic!("{x}")` a `String`; anything else
/// becomes "a handler panicked". Use `Box<dyn Any>::downcast_ref`.
pub fn panic_message(panic: Box<dyn Any + Send + 'static>) -> String {
    todo!()
}
