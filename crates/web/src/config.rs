//! Web settings derived from the environment by the binary.

use std::net::IpAddr;

use axum::http::{HeaderName, Uri};
use thiserror::Error;

/// The public address of the app (`APP_BASE_URL`). It decides the cookie flags, HSTS, and the
/// origin that state-changing requests must come from, so a proxy's internal `Host` header
/// never matters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseUrl {
    origin: String,
    https: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BaseUrlError {
    #[error("APP_BASE_URL must be an absolute http:// or https:// URL, got {0:?}")]
    NotAbsolute(String),
    #[error("APP_BASE_URL must not have a path, query or fragment, got {0:?}")]
    HasPath(String),
    #[error("APP_BASE_URL must use https:// unless the host is localhost, got {0:?}")]
    InsecureHost(String),
}

impl BaseUrl {
    pub fn parse(raw: &str) -> Result<Self, BaseUrlError> {
        let trimmed = raw.trim().trim_end_matches('/');
        let not_absolute = || BaseUrlError::NotAbsolute(raw.to_owned());
        if trimmed.contains('#') {
            return Err(BaseUrlError::HasPath(raw.to_owned()));
        }
        let uri: Uri = trimmed.parse().map_err(|_| not_absolute())?;
        let (Some(scheme), Some(authority)) = (uri.scheme_str(), uri.authority()) else {
            return Err(not_absolute());
        };
        if authority.as_str().contains('@') {
            return Err(not_absolute());
        }
        if !matches!(uri.path(), "" | "/") || uri.query().is_some() {
            return Err(BaseUrlError::HasPath(raw.to_owned()));
        }
        let https = match scheme.to_ascii_lowercase().as_str() {
            "https" => true,
            "http" => false,
            _ => return Err(not_absolute()),
        };
        let host = authority.host().to_ascii_lowercase();
        if !https && !is_loopback(&host) {
            return Err(BaseUrlError::InsecureHost(raw.to_owned()));
        }
        let default_port = if https { 443 } else { 80 };
        let scheme = if https { "https" } else { "http" };
        let origin = match authority.port_u16().filter(|&port| port != default_port) {
            Some(port) => format!("{scheme}://{host}:{port}"),
            None => format!("{scheme}://{host}"),
        };
        Ok(Self { origin, https })
    }

    /// `scheme://host[:port]`, lowercase, without a default port: what browsers send as `Origin`.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn is_https(&self) -> bool {
        self.https
    }
}

fn is_loopback(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

#[derive(Debug, Clone)]
pub struct WebConfig {
    pub base_url: BaseUrl,
    /// A header set by a trusted reverse proxy that carries the client's address
    /// (`CLIENT_IP_HEADER`). Without it the peer address is used.
    pub client_ip_header: Option<HeaderName>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origin_drops_trailing_slashes_default_ports_and_case() {
        for raw in [
            "https://Smoke.Example.org",
            "https://smoke.example.org/",
            "https://smoke.example.org:443/",
            " https://smoke.example.org ",
        ] {
            let url = BaseUrl::parse(raw).unwrap();
            assert_eq!(url.origin(), "https://smoke.example.org", "{raw}");
            assert!(url.is_https());
        }
        assert_eq!(
            BaseUrl::parse("https://smoke.example.org:8443")
                .unwrap()
                .origin(),
            "https://smoke.example.org:8443"
        );
    }

    #[test]
    fn plain_http_is_only_for_loopback_hosts() {
        for raw in [
            "http://localhost:8080",
            "http://127.0.0.1:8080",
            "http://[::1]:8080",
            "http://app.localhost",
        ] {
            let url = BaseUrl::parse(raw).unwrap();
            assert!(!url.is_https(), "{raw}");
        }
        assert_eq!(
            BaseUrl::parse("http://192.168.1.20:8080"),
            Err(BaseUrlError::InsecureHost(
                "http://192.168.1.20:8080".into()
            ))
        );
        assert_eq!(
            BaseUrl::parse("http://smoke.example.org"),
            Err(BaseUrlError::InsecureHost(
                "http://smoke.example.org".into()
            ))
        );
    }

    #[test]
    fn base_urls_with_paths_or_odd_schemes_are_rejected() {
        assert!(matches!(
            BaseUrl::parse("https://smoke.example.org/app"),
            Err(BaseUrlError::HasPath(_))
        ));
        assert!(matches!(
            BaseUrl::parse("https://smoke.example.org/?a=1"),
            Err(BaseUrlError::HasPath(_))
        ));
        assert!(matches!(
            BaseUrl::parse("https://smoke.example.org/#top"),
            Err(BaseUrlError::HasPath(_))
        ));
        for raw in [
            "smoke.example.org",
            "ftp://smoke.example.org",
            "https://u@smoke.example.org",
            "",
        ] {
            assert!(
                matches!(BaseUrl::parse(raw), Err(BaseUrlError::NotAbsolute(_))),
                "{raw}"
            );
        }
    }
}
