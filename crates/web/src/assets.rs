//! Static files, compiled into the binary (spec §10.3). Each is listed here explicitly; a test
//! checks the list matches the `static/` folder.

use std::sync::LazyLock;

use axum::extract::Path;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};

struct Asset {
    path: &'static str,
    content_type: &'static str,
    body: &'static [u8],
}

macro_rules! asset {
    ($path:literal, $content_type:literal) => {
        Asset {
            path: $path,
            content_type: $content_type,
            body: include_bytes!(concat!("../static/", $path)),
        }
    };
}

const ASSETS: &[Asset] = &[
    asset!("app.css", "text/css; charset=utf-8"),
    asset!("favicon.svg", "image/svg+xml"),
    asset!("htmx-2.0.11.min.js", "text/javascript; charset=utf-8"),
];

/// A strong validator per asset: the first 16 hex characters of its SHA-256.
static ETAGS: LazyLock<Vec<String>> = LazyLock::new(|| {
    ASSETS
        .iter()
        .map(|asset| format!("\"{}\"", &hex::encode(Sha256::digest(asset.body))[..16]))
        .collect()
});

/// `Cache-Control: no-cache` makes browsers revalidate, which costs a `304` and no body.
pub(crate) async fn serve(Path(path): Path<String>, headers: HeaderMap) -> Response {
    let Some(index) = ASSETS.iter().position(|asset| asset.path == path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let asset = &ASSETS[index];
    let etag = ETAGS[index].as_str();
    let cache = [(ETAG, etag), (CACHE_CONTROL, "no-cache")];
    if headers
        .get(IF_NONE_MATCH)
        .is_some_and(|value| value.as_bytes() == etag.as_bytes())
    {
        return (StatusCode::NOT_MODIFIED, cache).into_response();
    }
    (cache, [(CONTENT_TYPE, asset.content_type)], asset.body).into_response()
}

pub(crate) async fn healthz() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// The published `dist/htmx.min.js` of htmx.org 2.0.11 on npm.
    const HTMX_SHA256: &str = "d6fdc75f204e6bdefa99b69bf1e6d4ac69b8a364f77929f45c13476b4000f717";

    #[test]
    fn every_file_in_the_static_folder_is_served() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
        let on_disk: BTreeSet<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        let listed: BTreeSet<String> = ASSETS.iter().map(|a| a.path.to_owned()).collect();
        assert_eq!(on_disk, listed);
    }

    #[test]
    fn the_vendored_htmx_is_the_published_release() {
        let htmx = ASSETS.iter().find(|a| a.path.starts_with("htmx-")).unwrap();
        assert_eq!(hex::encode(Sha256::digest(htmx.body)), HTMX_SHA256);
    }
}
