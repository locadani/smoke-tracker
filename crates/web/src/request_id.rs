//! Request ids: generated for every request, logged in its span, returned as `x-request-id`,
//! and shown on error pages so a user can quote one.

use axum::http::{HeaderMap, HeaderValue, Request};
use tower_http::request_id::{MakeRequestId, RequestId};

pub(crate) const HEADER: &str = "x-request-id";

/// 16 hex characters from the operating system's random source.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RandomRequestId;

impl MakeRequestId for RandomRequestId {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).ok()?;
        HeaderValue::from_str(&hex::encode(bytes))
            .ok()
            .map(RequestId::new)
    }
}

/// The request's id, or `-` if it has none.
pub(crate) fn of(headers: &HeaderMap) -> &str {
    headers
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_sixteen_hex_characters_and_differ() {
        let request = Request::new(());
        let mut make = RandomRequestId;
        let a = make.make_request_id(&request).unwrap();
        let b = make.make_request_id(&request).unwrap();
        let text = a.header_value().to_str().unwrap();
        assert_eq!(text.len(), 16);
        assert!(text.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a.header_value(), b.header_value());
    }
}
