//! Static files, compiled into the binary (spec §10.3). Each is listed here explicitly; a test
//! checks the list matches the `static/` folder. The stylesheet is the exception: it is written
//! as several files under `css/` and joined at compile time into the one `/static/app.css`.

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

/// Joins the stylesheet sources, in the order given, into one `&str`, and lists their names.
macro_rules! stylesheet {
    ($($file:literal),+ $(,)?) => {
        (
            concat!($(include_str!(concat!("../css/", $file))),+),
            &[$($file),+],
        )
    };
}

/// The served stylesheet and its sources, in cascade order: `00-layers.css` declares the
/// layers, so it comes first.
const STYLESHEET: (&str, &[&str]) = stylesheet!(
    "00-layers.css",
    "10-reset.css",
    "20-tokens.css",
    "30-base.css",
    "40-layout.css",
    "90-utilities.css",
);

const ASSETS: &[Asset] = &[
    Asset {
        path: "app.css",
        content_type: "text/css; charset=utf-8",
        body: STYLESHEET.0.as_bytes(),
    },
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

    /// The names of the files in `dir` (relative to the crate), sorted.
    fn files_in(dir: &str) -> Vec<String> {
        let path = format!("{}/{dir}", env!("CARGO_MANIFEST_DIR"));
        let mut names: Vec<String> = std::fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn every_file_in_the_static_folder_is_served() {
        let on_disk: BTreeSet<String> = files_in("static").into_iter().collect();
        let listed: BTreeSet<String> = ASSETS
            .iter()
            .map(|a| a.path.to_owned())
            .filter(|path| path != "app.css")
            .collect();
        assert_eq!(on_disk, listed);
    }

    #[test]
    fn the_stylesheet_joins_every_css_file_in_name_order() {
        assert_eq!(files_in("css"), STYLESHEET.1);
        assert!(
            STYLESHEET
                .0
                .contains("@layer reset, tokens, base, layout, components, utilities;"),
            "the layer order is declared"
        );
        let first_rule = STYLESHEET.0.find('{').unwrap();
        let declaration = STYLESHEET.0.find("@layer reset,").unwrap();
        assert!(declaration < first_rule, "the layers are declared first");
    }

    /// The source of each stylesheet file, by name.
    fn css_sources() -> impl Iterator<Item = (&'static str, String)> {
        STYLESHEET.1.iter().map(|&name| {
            let path = format!("{}/css/{name}", env!("CARGO_MANIFEST_DIR"));
            (name, std::fs::read_to_string(path).unwrap())
        })
    }

    #[test]
    fn every_rule_sits_in_a_layer() {
        for (name, source) in css_sources().skip(1) {
            let code = without_comments(&source);
            let code = code.trim_start();
            assert!(code.starts_with("@layer "), "{name} starts outside a layer");
            let body_end = matching_brace(code, code.find('{').unwrap());
            assert!(
                code[body_end + 1..].trim().is_empty(),
                "{name} has rules after its layer block"
            );
        }
    }

    #[test]
    fn colours_are_written_only_as_tokens() {
        for (name, source) in css_sources().filter(|(name, _)| *name != "20-tokens.css") {
            let code = without_comments(&source);
            for function in ["rgb(", "rgba(", "hsl(", "hwb(", "lab(", "lch(", "oklch("] {
                assert!(
                    !code.contains(function),
                    "{name} has a raw colour: {function}"
                );
            }
            let hex = code
                .match_indices('#')
                .find(|(at, _)| code[at + 1..].starts_with(|c: char| c.is_ascii_hexdigit()));
            assert_eq!(hex, None, "{name} has a hex colour");
        }
    }

    fn without_comments(css: &str) -> String {
        let mut out = String::new();
        let mut rest = css;
        while let Some(start) = rest.find("/*") {
            out.push_str(&rest[..start]);
            let end = rest[start..].find("*/").expect("a closed comment");
            rest = &rest[start + end + 2..];
        }
        out.push_str(rest);
        out
    }

    /// The index of the `}` that closes the `{` at `open`.
    fn matching_brace(code: &str, open: usize) -> usize {
        let mut depth = 0;
        for (index, byte) in code.bytes().enumerate().skip(open) {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return index;
                    }
                }
                _ => {}
            }
        }
        panic!("an unclosed block");
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
