//! Exercise 9: small pure functions that decide things about a request.
//!
//! Practises: `docs/learning/03-web-and-auth.md`, sections 9 and 10.
//! Mirrors: `smoke_web::i18n::{negotiate, from_accept_language}`,
//! `smoke_web::security::is_same_origin`, `smoke_web::auth::client_ip::resolve` and
//! `smoke_web::logged_path`.
//! Run: `cargo test --test ex09_request_rules`
//!
//! Hint: none of these need a loop over indices. `Option::or_else`, `and_then`, `map_or`,
//! `is_some_and`, `str::split`, `str::rsplit`, `strip_prefix` and `parse` do most of the work.

use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    It,
    Tr,
}

/// Given: "en", "it" or "tr" (nothing else, no case folding).
impl FromStr for Locale {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "en" => Ok(Self::En),
            "it" => Ok(Self::It),
            "tr" => Ok(Self::Tr),
            _ => Err(()),
        }
    }
}

/// The language of a request: the account's language when signed in, else the `lang` cookie,
/// else the browser's `Accept-Language`, else English. An unusable value is skipped, never an
/// error. The cookie is trimmed before parsing.
pub fn negotiate(
    account: Option<Locale>,
    cookie: Option<&str>,
    accept_language: Option<&str>,
) -> Locale {
    todo!()
}

/// The supported language with the highest quality in an `Accept-Language` value such as
/// `"fr;q=0.9, it-IT;q=0.8, tr;q=0.8"`; the first of equals wins. A missing `q` is 1.0, a
/// quality of 0 (or one that is not a number) excludes the language, the region part (`-IT`)
/// and the case of the tag are ignored.
pub fn from_accept_language(header: &str) -> Option<Locale> {
    todo!()
}

/// Whether a state-changing request comes from the app itself. `Sec-Fetch-Site` decides when
/// the browser sent it (only `same-origin` passes); otherwise `Origin` must equal `app_origin`,
/// ignoring case. A request with neither is refused.
pub fn is_same_origin(
    sec_fetch_site: Option<&str>,
    origin: Option<&str>,
    app_origin: &str,
) -> bool {
    todo!()
}

/// The client's address behind a trusted proxy: `X-Forwarded-For: client, proxy1` lists what
/// each hop saw, and only the LAST entry was added by the proxy we trust. A missing or
/// unparsable header falls back to the TCP `peer`, and with no peer either to `0.0.0.0`.
pub fn client_ip(forwarded_for: Option<&str>, peer: Option<IpAddr>) -> IpAddr {
    todo!()
}

/// The path as the logs may show it: an invite link's token works like a password until it is
/// used, so everything under `/invite/` is shown as `/invite/…` (with the real ellipsis
/// character). Other paths are returned as they are, without copying.
pub fn logged_path(path: &str) -> &str {
    todo!()
}
