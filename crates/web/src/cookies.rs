//! Reading cookies from a request and building the `lang` cookie.

use axum::http::header::{ACCEPT_LANGUAGE, COOKIE};
use axum::http::{HeaderMap, HeaderValue};
use smoke_domain::types::Locale;
use tower_sessions::cookie::time::Duration;
use tower_sessions::cookie::{Cookie, SameSite};

use crate::i18n::LANG_COOKIE;

/// The value of the first cookie called `name`, across all `Cookie` headers.
pub(crate) fn get(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(Cookie::split_parse)
        .filter_map(Result::ok)
        .find(|cookie| cookie.name() == name)
        .map(|cookie| cookie.value().to_owned())
}

pub(crate) fn accept_language(headers: &HeaderMap) -> Option<&str> {
    headers.get(ACCEPT_LANGUAGE).and_then(|v| v.to_str().ok())
}

/// Remembers the language for a year. Not secret, but still `HttpOnly`: no script needs it.
/// Always Secure, like the session cookie (decision #145).
pub(crate) fn lang(locale: Locale) -> HeaderValue {
    let cookie = Cookie::build((LANG_COOKIE, locale.as_str()))
        .path("/")
        .max_age(Duration::days(365))
        .same_site(SameSite::Lax)
        .http_only(true)
        .secure(true)
        .build();
    HeaderValue::from_str(&cookie.to_string()).expect("a cookie made of ASCII is a valid header")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookies_are_found_across_headers() {
        let mut headers = HeaderMap::new();
        headers.append(COOKIE, HeaderValue::from_static("a=1; lang=it"));
        headers.append(COOKIE, HeaderValue::from_static("b=2"));
        assert_eq!(get(&headers, "lang").as_deref(), Some("it"));
        assert_eq!(get(&headers, "b").as_deref(), Some("2"));
        assert_eq!(get(&headers, "missing"), None);
    }

    #[test]
    fn the_lang_cookie_is_long_lived_and_secure() {
        assert_eq!(
            lang(Locale::Tr),
            "lang=tr; HttpOnly; SameSite=Lax; Secure; Path=/; Max-Age=31536000"
        );
    }
}
