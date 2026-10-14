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
    asset!("app.js", "text/javascript; charset=utf-8"),
    asset!("chart-4.5.1.umd.min.js", "text/javascript; charset=utf-8"),
    asset!("favicon.svg", "image/svg+xml"),
    asset!("graphs.js", "text/javascript; charset=utf-8"),
    asset!("htmx-2.0.11.min.js", "text/javascript; charset=utf-8"),
    asset!("icon-180.png", "image/png"),
    asset!("icon-192.png", "image/png"),
    asset!("icon-512.png", "image/png"),
    asset!("icon.svg", "image/svg+xml"),
    asset!("manifest.webmanifest", "application/manifest+json"),
    asset!("sw.js", "text/javascript; charset=utf-8"),
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
    respond(&path, &headers)
}

/// The service worker at the root, so that its scope is the whole app rather than `/static/`.
pub(crate) async fn service_worker(headers: HeaderMap) -> Response {
    respond("sw.js", &headers)
}

fn respond(path: &str, headers: &HeaderMap) -> Response {
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

    /// The published `dist/chart.umd.min.js` of chart.js 4.5.1 on npm (MIT licence).
    const CHART_JS_SHA256: &str =
        "48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a";

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

    fn asset(path: &str) -> &'static Asset {
        ASSETS.iter().find(|a| a.path == path).unwrap()
    }

    /// Width and height from a PNG's header chunk.
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let number = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        (number(16), number(20))
    }

    #[test]
    fn the_manifest_has_what_installing_needs() {
        let manifest: serde_json::Value =
            serde_json::from_slice(asset("manifest.webmanifest").body).unwrap();
        assert_eq!(manifest["display"], "standalone");
        assert_eq!(manifest["start_url"], "/");
        assert_eq!(manifest["scope"], "/");
        assert_eq!(manifest["shortcuts"][0]["name"], "New smell report");
        assert_eq!(manifest["shortcuts"][0]["url"], "/");
        let icons = manifest["icons"].as_array().unwrap();
        for icon in icons {
            let src = icon["src"].as_str().unwrap();
            let file = asset(src.strip_prefix("/static/").unwrap());
            if let Some(size) = icon["sizes"].as_str().unwrap().split_once('x') {
                let (width, height) = png_size(file.body);
                assert_eq!(
                    (width.to_string(), height.to_string()),
                    (size.0.to_owned(), size.1.to_owned()),
                    "{src}"
                );
            }
        }
        for size in ["192x192", "512x512"] {
            assert!(icons.iter().any(|i| i["sizes"] == size), "{size}");
        }
        assert!(icons.iter().any(|i| i["purpose"] == "maskable"));
    }

    #[test]
    fn the_apple_touch_icon_is_180_pixels() {
        assert_eq!(png_size(asset("icon-180.png").body), (180, 180));
    }

    #[test]
    fn the_vendored_libraries_are_the_published_releases() {
        for (prefix, sha256) in [("htmx-", HTMX_SHA256), ("chart-", CHART_JS_SHA256)] {
            let library = ASSETS.iter().find(|a| a.path.starts_with(prefix)).unwrap();
            assert_eq!(
                hex::encode(Sha256::digest(library.body)),
                sha256,
                "{prefix}"
            );
        }
    }
}
