use std::cell::Cell;

use smoke_exercises::ex15_pwa_assets::{
    Asset, Reply, etag_of, network_first, respond, svg, svg_path,
};

static ASSETS: &[Asset] = &[
    Asset {
        path: "app.css",
        content_type: "text/css; charset=utf-8",
        body: b"body { margin: 0 }",
    },
    Asset {
        path: "app.js",
        content_type: "text/javascript; charset=utf-8",
        body: b"console.log('hi')",
    },
];

#[test]
fn an_etag_is_a_quoted_hash_of_sixteen_hex_digits() {
    let tag = etag_of(b"hello");
    assert_eq!(tag.len(), 18);
    assert!(tag.starts_with('"') && tag.ends_with('"'));
    assert!(
        tag[1..17]
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}

#[test]
fn equal_bodies_share_a_tag_and_different_bodies_do_not() {
    assert_eq!(etag_of(b"same"), etag_of(b"same"));
    assert_ne!(etag_of(b"one"), etag_of(b"two"));
}

#[test]
fn an_unknown_path_is_not_found() {
    assert_eq!(respond(ASSETS, "missing.css", None), Reply::NotFound);
}

#[test]
fn a_known_path_is_served_with_its_tag() {
    let reply = respond(ASSETS, "app.css", None);
    assert_eq!(
        reply,
        Reply::Found {
            etag: etag_of(b"body { margin: 0 }"),
            content_type: "text/css; charset=utf-8",
            body: b"body { margin: 0 }",
        }
    );
}

#[test]
fn a_browser_with_the_current_tag_gets_not_modified() {
    let tag = etag_of(b"console.log('hi')");
    assert_eq!(
        respond(ASSETS, "app.js", Some(&tag)),
        Reply::NotModified { etag: tag.clone() }
    );
}

#[test]
fn a_browser_with_an_old_tag_gets_the_new_file() {
    let reply = respond(ASSETS, "app.js", Some("\"0000000000000000\""));
    assert!(matches!(reply, Reply::Found { .. }));
}

#[test]
fn the_network_answer_wins_and_the_cache_is_not_even_asked() {
    let asked = Cell::new(false);
    let got = network_first(
        || Ok::<_, ()>("fresh"),
        || {
            asked.set(true);
            Some("stale")
        },
    );
    assert_eq!(got, Some("fresh"));
    assert!(!asked.get());
}

#[test]
fn when_the_network_fails_the_cached_copy_is_used() {
    let got = network_first(|| Err::<&str, _>("offline"), || Some("stale"));
    assert_eq!(got, Some("stale"));
}

#[test]
fn when_both_fail_there_is_nothing() {
    let got = network_first(|| Err::<&str, _>("offline"), || None);
    assert_eq!(got, None);
}

fn grid() -> Vec<Vec<bool>> {
    vec![
        vec![true, false, true],
        vec![false, true, false],
        vec![true, false, false],
    ]
}

#[test]
fn every_dark_module_becomes_one_square_inside_the_quiet_zone() {
    assert_eq!(
        svg_path(&grid(), 4),
        "M4,4h1v1h-1zM6,4h1v1h-1zM5,5h1v1h-1zM4,6h1v1h-1z"
    );
}

#[test]
fn an_empty_grid_draws_no_path() {
    assert_eq!(svg_path(&[], 4), "");
    assert_eq!(svg_path(&[vec![false, false]], 0), "");
}

#[test]
fn the_drawing_is_sized_by_the_grid_and_the_margin() {
    let drawing = svg(&grid(), 4);
    assert!(drawing.starts_with("<svg "));
    assert!(drawing.ends_with("</svg>"));
    assert!(drawing.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(drawing.contains("viewBox=\"0 0 11 11\""));
    assert!(drawing.contains(&format!(
        "<path d=\"{}\" fill=\"#000\"/>",
        svg_path(&grid(), 4)
    )));
}
