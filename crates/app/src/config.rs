//! Configuration from the environment only (spec §10.5); `.env.example` lists every variable.

use std::net::SocketAddr;
use std::path::PathBuf;

use chrono_tz::Tz;
use smoke_domain::schedule::DailySchedule;
use smoke_photos::{
    GcsPhotoStore, GoogleAuth, LocalPhotoStore, PhotoError, PhotoStore, Pm25Reader,
};
use smoke_web::config::{BaseUrl, WebConfig};
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{0} is not set (see .env.example)")]
    Missing(&'static str),
    #[error("{name} is invalid: {reason}")]
    Invalid { name: &'static str, reason: String },
}

/// Reads a variable from the process environment.
pub fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// `DATABASE_URL`, needed by every command: `sqlite://…` or `postgres://…`.
pub fn database_url(get: impl Fn(&str) -> Option<String>) -> Result<String, ConfigError> {
    get("DATABASE_URL")
        .filter(|url| !url.trim().is_empty())
        .ok_or(ConfigError::Missing("DATABASE_URL"))
}

/// What `serve` needs beyond the database.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub web: WebConfig,
    pub photos: PhotoConfig,
    pub bind_addr: SocketAddr,
}

/// Where photos live: `PHOTO_BUCKET` selects Cloud Storage, otherwise a local folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoLocation {
    Local(PathBuf),
    Gcs { bucket: String },
}

/// Which reader turns a sensor photo into a number (`PM25_READER`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderKind {
    None,
    Vision,
}

/// Photo storage and reading, shared by `serve` and `verify-audit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoConfig {
    pub location: PhotoLocation,
    pub reader: ReaderKind,
}

impl PhotoConfig {
    pub const DEFAULT_DIR: &str = "data/photos";

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let set = |name: &str| get(name).filter(|v| !v.trim().is_empty());
        let location = match set("PHOTO_BUCKET") {
            Some(bucket) => PhotoLocation::Gcs {
                bucket: bucket.trim().to_owned(),
            },
            None => PhotoLocation::Local(PathBuf::from(
                set("PHOTO_DIR").unwrap_or_else(|| Self::DEFAULT_DIR.to_owned()),
            )),
        };
        let reader = match set("PM25_READER").as_deref().map(str::trim) {
            None | Some("none") => ReaderKind::None,
            Some("vision") => ReaderKind::Vision,
            Some(other) => {
                return Err(invalid(
                    "PM25_READER",
                    format!("expected vision or none, got {other:?}"),
                ));
            }
        };
        Ok(Self { location, reader })
    }

    /// The configured store. Google calls authenticate through the metadata server only.
    pub fn build_store(&self) -> Result<PhotoStore, PhotoError> {
        Ok(match &self.location {
            PhotoLocation::Local(dir) => PhotoStore::Local(LocalPhotoStore::new(dir.clone())),
            PhotoLocation::Gcs { bucket } => {
                PhotoStore::Gcs(GcsPhotoStore::new(bucket.clone(), GoogleAuth::new()?))
            }
        })
    }

    pub fn build_reader(&self) -> Result<Pm25Reader, PhotoError> {
        Ok(match self.reader {
            ReaderKind::None => Pm25Reader::None,
            ReaderKind::Vision => Pm25Reader::CloudVision(GoogleAuth::new()?),
        })
    }
}

impl ServeConfig {
    pub const DEFAULT_PHOTO_MAX_BYTES: usize = 307_200;
    pub const MIN_PHOTO_MAX_BYTES: usize = 10_240;
    pub const MAX_PHOTO_MAX_BYTES: usize = 5_242_880;
    pub const DEFAULT_TIMEZONE: &str = "Europe/Rome";
    pub const DEFAULT_AUTO_CLOSE_HOUR: u32 = 4;
    pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let base_url = get("APP_BASE_URL").ok_or(ConfigError::Missing("APP_BASE_URL"))?;
        let base_url = BaseUrl::parse(&base_url).map_err(|e| invalid("APP_BASE_URL", e))?;
        let timezone = get("APP_TIMEZONE").unwrap_or_else(|| Self::DEFAULT_TIMEZONE.to_owned());
        let timezone: Tz = timezone
            .trim()
            .parse()
            .map_err(|_| invalid("APP_TIMEZONE", format!("unknown time zone {timezone:?}")))?;
        let hour = match get("AUTO_CLOSE_HOUR") {
            Some(hour) => hour
                .trim()
                .parse()
                .map_err(|_| invalid("AUTO_CLOSE_HOUR", format!("not an hour: {hour:?}")))?,
            None => Self::DEFAULT_AUTO_CLOSE_HOUR,
        };
        let schedule =
            DailySchedule::new(timezone, hour).map_err(|e| invalid("AUTO_CLOSE_HOUR", e))?;
        let bind_addr = get("BIND_ADDR").unwrap_or_else(|| Self::DEFAULT_BIND_ADDR.to_owned());
        let bind_addr = bind_addr
            .trim()
            .parse()
            .map_err(|_| invalid("BIND_ADDR", format!("not an address:port: {bind_addr:?}")))?;
        let client_ip_header = get("CLIENT_IP_HEADER")
            .filter(|h| !h.trim().is_empty())
            .map(|h| {
                h.trim()
                    .parse()
                    .map_err(|_| invalid("CLIENT_IP_HEADER", format!("not a header name: {h:?}")))
            })
            .transpose()?;
        let photo_max_bytes = match get("PHOTO_MAX_BYTES") {
            Some(raw) => raw
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|n| (Self::MIN_PHOTO_MAX_BYTES..=Self::MAX_PHOTO_MAX_BYTES).contains(n))
                .ok_or_else(|| {
                    invalid(
                        "PHOTO_MAX_BYTES",
                        format!(
                            "expected {} to {} bytes, got {raw:?}",
                            Self::MIN_PHOTO_MAX_BYTES,
                            Self::MAX_PHOTO_MAX_BYTES
                        ),
                    )
                })?,
            None => Self::DEFAULT_PHOTO_MAX_BYTES,
        };
        let photos = PhotoConfig::from_lookup(&get)?;
        if !base_url.is_https() {
            warn!(
                "APP_BASE_URL is plain http: for local development only. Cookies stay Secure, \
                 so use Chrome or Firefox: Safari drops them on http://localhost. For Safari, \
                 put Caddy in front (see the README) and set APP_BASE_URL to its https address"
            );
        }
        Ok(Self {
            web: WebConfig {
                base_url,
                client_ip_header,
                schedule,
                photo_max_bytes,
            },
            photos,
            bind_addr,
        })
    }
}

fn invalid(name: &'static str, reason: impl ToString) -> ConfigError {
    ConfigError::Invalid {
        name,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name| map.get(name).cloned()
    }

    fn error_for(pairs: &[(&str, &str)]) -> ConfigError {
        let mut all = vec![("APP_BASE_URL", "https://smoke.example.org")];
        all.extend_from_slice(pairs);
        ServeConfig::from_lookup(lookup(&all)).unwrap_err()
    }

    #[test]
    fn only_the_base_url_is_required_to_serve() {
        let config =
            ServeConfig::from_lookup(lookup(&[("APP_BASE_URL", "https://smoke.example.org/")]))
                .unwrap();
        assert_eq!(config.web.base_url.origin(), "https://smoke.example.org");
        assert_eq!(config.web.schedule.tz(), chrono_tz::Europe::Rome);
        assert_eq!(config.web.schedule.hour(), 4);
        assert_eq!(config.bind_addr, "127.0.0.1:8080".parse().unwrap());
        assert_eq!(config.web.client_ip_header, None);
        assert_eq!(config.web.photo_max_bytes, 307_200);
        assert_eq!(
            config.photos,
            PhotoConfig {
                location: PhotoLocation::Local(PathBuf::from("data/photos")),
                reader: ReaderKind::None,
            }
        );
        assert_eq!(
            ServeConfig::from_lookup(lookup(&[])).unwrap_err(),
            ConfigError::Missing("APP_BASE_URL")
        );
    }

    #[test]
    fn every_variable_can_be_set() {
        let config = ServeConfig::from_lookup(lookup(&[
            ("APP_BASE_URL", "https://smoke.example.org"),
            ("APP_TIMEZONE", "Europe/Istanbul"),
            ("AUTO_CLOSE_HOUR", "5"),
            ("BIND_ADDR", "0.0.0.0:3000"),
            ("CLIENT_IP_HEADER", "Fly-Client-IP"),
            ("PHOTO_MAX_BYTES", "10240"),
            ("PHOTO_BUCKET", " smoke-photos "),
            ("PHOTO_DIR", "elsewhere"),
            ("PM25_READER", "vision"),
        ]))
        .unwrap();
        assert_eq!(config.web.photo_max_bytes, 10_240);
        assert_eq!(
            config.photos,
            PhotoConfig {
                location: PhotoLocation::Gcs {
                    bucket: "smoke-photos".into()
                },
                reader: ReaderKind::Vision,
            }
        );
        assert_eq!(config.web.schedule.tz(), chrono_tz::Europe::Istanbul);
        assert_eq!(config.web.schedule.hour(), 5);
        assert_eq!(config.bind_addr, "0.0.0.0:3000".parse().unwrap());
        assert_eq!(config.web.client_ip_header.unwrap(), "fly-client-ip");
    }

    #[test]
    fn invalid_values_name_the_variable() {
        for (pairs, name) in [
            (
                &[("APP_BASE_URL", "http://192.168.1.2")][..],
                "APP_BASE_URL",
            ),
            (&[("APP_TIMEZONE", "Mars/Olympus")][..], "APP_TIMEZONE"),
            (&[("AUTO_CLOSE_HOUR", "24")][..], "AUTO_CLOSE_HOUR"),
            (&[("AUTO_CLOSE_HOUR", "four")][..], "AUTO_CLOSE_HOUR"),
            (&[("BIND_ADDR", "8080")][..], "BIND_ADDR"),
            (&[("PHOTO_MAX_BYTES", "10239")][..], "PHOTO_MAX_BYTES"),
            (&[("PHOTO_MAX_BYTES", "5242881")][..], "PHOTO_MAX_BYTES"),
            (&[("PHOTO_MAX_BYTES", "300KB")][..], "PHOTO_MAX_BYTES"),
            (&[("PM25_READER", "tesseract")][..], "PM25_READER"),
            (
                &[("CLIENT_IP_HEADER", "bad header")][..],
                "CLIENT_IP_HEADER",
            ),
        ] {
            match error_for(pairs) {
                ConfigError::Invalid { name: got, .. } => assert_eq!(got, name, "{pairs:?}"),
                other => panic!("{pairs:?}: {other:?}"),
            }
        }
    }

    #[test]
    fn the_size_limit_bounds_are_inclusive() {
        for raw in ["10240", "5242880"] {
            let config = ServeConfig::from_lookup(lookup(&[
                ("APP_BASE_URL", "https://smoke.example.org"),
                ("PHOTO_MAX_BYTES", raw),
            ]))
            .unwrap();
            assert_eq!(config.web.photo_max_bytes.to_string(), raw);
        }
    }

    #[test]
    fn a_bucket_selects_cloud_storage_and_a_folder_selects_local() {
        let local = PhotoConfig::from_lookup(lookup(&[("PHOTO_DIR", "pics")])).unwrap();
        assert_eq!(local.location, PhotoLocation::Local(PathBuf::from("pics")));
        let gcs = PhotoConfig::from_lookup(lookup(&[("PHOTO_BUCKET", "b")])).unwrap();
        assert_eq!(gcs.location, PhotoLocation::Gcs { bucket: "b".into() });
        let blank = PhotoConfig::from_lookup(lookup(&[("PHOTO_BUCKET", " ")])).unwrap();
        assert!(matches!(blank.location, PhotoLocation::Local(_)));
    }

    #[test]
    fn the_database_url_is_required_and_not_blank() {
        assert_eq!(
            database_url(lookup(&[("DATABASE_URL", "sqlite://data/smoke.db")])).unwrap(),
            "sqlite://data/smoke.db"
        );
        assert_eq!(
            database_url(lookup(&[("DATABASE_URL", " ")])),
            Err(ConfigError::Missing("DATABASE_URL"))
        );
        assert_eq!(
            database_url(lookup(&[])),
            Err(ConfigError::Missing("DATABASE_URL"))
        );
    }
}
