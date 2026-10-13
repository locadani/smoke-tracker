//! Response headers every page carries, and the cross-site request check (spec §12).

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, ORIGIN, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::config::WebConfig;

/// Same-origin resources only: no inline scripts or styles, no CDNs, no framing.
pub(crate) const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
    img-src 'self'; connect-src 'self'; font-src 'self'; manifest-src 'self'; \
    worker-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; \
    frame-ancestors 'none'";

const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=(), payment=()";
const HSTS: &str = "max-age=63072000";

/// Adds the security headers. HSTS only over https; `Cache-Control: no-store` unless the
/// response set its own (static assets do), so signed-in pages never sit in a shared cache.
pub(crate) async fn headers(State(https): State<bool>, request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("same-origin"));
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(PERMISSIONS_POLICY),
    );
    if https {
        headers.insert(STRICT_TRANSPORT_SECURITY, HeaderValue::from_static(HSTS));
    }
    headers
        .entry(CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
    response
}

/// Refuses state-changing requests that a browser marks as cross-site (CSRF defence on top of
/// `SameSite=Lax`).
pub(crate) async fn check_origin(
    State(config): State<Arc<WebConfig>>,
    request: Request,
    next: Next,
) -> Response {
    let safe = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    if safe || is_same_origin(request.headers(), config.base_url.origin()) {
        next.run(request).await
    } else {
        tracing::warn!("refused a cross-site request");
        StatusCode::FORBIDDEN.into_response()
    }
}

/// `Sec-Fetch-Site` decides when the browser sends it; otherwise `Origin` must equal the app's
/// own origin. A request with neither is refused: every browser this app supports sends one.
fn is_same_origin(headers: &HeaderMap, origin: &str) -> bool {
    if let Some(site) = headers.get("sec-fetch-site") {
        return site.as_bytes() == b"same-origin";
    }
    headers
        .get(ORIGIN)
        .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(origin.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: &str = "https://smoke.example.org";

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        pairs
            .iter()
            .map(|(k, v)| (HeaderName::from_static(k), HeaderValue::from_static(v)))
            .collect()
    }

    #[test]
    fn fetch_metadata_decides_when_present() {
        assert!(is_same_origin(
            &headers(&[("sec-fetch-site", "same-origin")]),
            APP
        ));
        for site in ["same-site", "cross-site", "none"] {
            let h = headers(&[("sec-fetch-site", site), ("origin", APP)]);
            assert!(!is_same_origin(&h, APP), "{site}");
        }
    }

    #[test]
    fn without_fetch_metadata_the_origin_must_match() {
        assert!(is_same_origin(&headers(&[("origin", APP)]), APP));
        assert!(is_same_origin(
            &headers(&[("origin", "https://SMOKE.example.org")]),
            APP
        ));
        assert!(!is_same_origin(
            &headers(&[("origin", "https://evil.example")]),
            APP
        ));
        assert!(!is_same_origin(&headers(&[("origin", "null")]), APP));
        assert!(!is_same_origin(&HeaderMap::new(), APP));
    }
}
