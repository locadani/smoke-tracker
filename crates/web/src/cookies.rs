//! Reading cookies from a request.

use axum::http::HeaderMap;
use axum::http::header::{ACCEPT_LANGUAGE, COOKIE};
use tower_sessions::cookie::Cookie;

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

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

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
}
