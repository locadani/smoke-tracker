//! The client's address for the per-address sign-in limit.

use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderName};

use crate::state::AppState;

/// The configured proxy header's address if there is one, else the TCP peer's address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientIp(pub IpAddr);

impl<S: Send + Sync> FromRequestParts<AppState<S>> for ClientIp {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState<S>,
    ) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0.ip());
        Ok(Self(resolve(
            &parts.headers,
            state.config.client_ip_header.as_ref(),
            peer,
        )))
    }
}

/// For a list such as `X-Forwarded-For: client, proxy1`, the last entry is the one the trusted
/// proxy added; earlier entries are whatever the client claimed.
fn resolve(headers: &HeaderMap, header: Option<&HeaderName>, peer: Option<IpAddr>) -> IpAddr {
    header
        .and_then(|name| headers.get(name))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit(',').next())
        .and_then(|value| value.trim().parse().ok())
        .or(peer)
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    const PEER: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

    fn forwarded(value: &'static str) -> HeaderMap {
        HeaderMap::from_iter([(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static(value),
        )])
    }

    #[test]
    fn without_a_configured_header_the_peer_is_used() {
        assert_eq!(resolve(&forwarded("203.0.113.9"), None, Some(PEER)), PEER);
    }

    #[test]
    fn the_last_entry_of_the_configured_header_wins() {
        let name = HeaderName::from_static("x-forwarded-for");
        assert_eq!(
            resolve(
                &forwarded("198.51.100.7, 203.0.113.9"),
                Some(&name),
                Some(PEER)
            ),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            resolve(&forwarded("garbage"), Some(&name), Some(PEER)),
            PEER
        );
        assert_eq!(
            resolve(&HeaderMap::new(), Some(&name), None),
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        );
    }
}
