//! Exercise 15: static assets, caching and a QR-style drawing. `'static` data, closures and
//! `String` building.
//!
//! Practises: `docs/learning/05-people-and-pwa.md`, sections 4, 6 and 7.
//! Mirrors: `smoke_web::assets::{respond, ETAGS}`, the network-first `fetch` handler in
//! `static/sw.js`, and `smoke_web::qr::svg`.
//! Run: `cargo test --test ex15_pwa_assets`
//!
//! Hint: `respond` finds the asset with `iter().find(...)`; an `ETag` is a quoted validator, and
//! a browser that already has the file sends it back in `If-None-Match`, so equal means
//! "304, send no body". `write!(out, "...")` (from `std::fmt::Write`) appends formatted text to
//! a `String`; its `Result` cannot fail for a `String`, so `.expect(...)` is fine there.
//!
//! The app's tags are the first 16 hex characters of a SHA-256; the standard library has none,
//! so `etag_of` uses `DefaultHasher`.

use std::fmt::Write;
use std::hash::{DefaultHasher, Hash, Hasher};

pub struct Asset {
    pub path: &'static str,
    pub content_type: &'static str,
    pub body: &'static [u8],
}

/// A strong validator for `body`: a double quote, 16 lowercase hex digits of its hash, a double
/// quote. Equal bodies give equal tags and different bodies (almost always) different ones.
/// `format!("{:016x}", number)` writes a number as 16 hex digits.
pub fn etag_of(body: &[u8]) -> String {
    todo!()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    NotFound,
    /// `If-None-Match` equals the asset's tag: no body.
    NotModified {
        etag: String,
    },
    Found {
        etag: String,
        content_type: &'static str,
        body: &'static [u8],
    },
}

/// The reply for `path` among `assets`.
pub fn respond(assets: &'static [Asset], path: &str, if_none_match: Option<&str>) -> Reply {
    todo!()
}

/// The service worker's strategy for assets: ask the network first; if that fails, fall back
/// to the cached copy. `cached` must not run when the network answered (take it as `FnOnce` and
/// call it only on the error path). `None` when both fail.
pub fn network_first<T, E>(
    fetch: impl FnOnce() -> Result<T, E>,
    cached: impl FnOnce() -> Option<T>,
) -> Option<T> {
    todo!()
}

/// The `d` attribute of an SVG `<path>` that draws every dark module as a 1x1 square: for the
/// module in column `x` and row `y`, `M{x+quiet},{y+quiet}h1v1h-1z`, rows top to bottom and
/// each row left to right, all concatenated with no separator.
pub fn svg_path(modules: &[Vec<bool>], quiet_zone: usize) -> String {
    todo!()
}

/// The whole drawing: an `<svg>` element with `xmlns="http://www.w3.org/2000/svg"`,
/// `viewBox="0 0 S S"` where S is the number of rows plus twice the quiet zone, and one
/// `<path d="..." fill="#000"/>` inside.
pub fn svg(modules: &[Vec<bool>], quiet_zone: usize) -> String {
    todo!()
}
