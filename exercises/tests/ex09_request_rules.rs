use std::net::{IpAddr, Ipv4Addr};

use smoke_exercises::ex09_request_rules::{
    Locale, client_ip, from_accept_language, is_same_origin, logged_path, negotiate,
};

const APP: &str = "https://smoke.example.org";

#[test]
fn the_account_language_wins_over_everything() {
    assert_eq!(
        negotiate(Some(Locale::Tr), Some("it"), Some("en")),
        Locale::Tr
    );
}

#[test]
fn without_an_account_the_cookie_then_the_browser_decide() {
    assert_eq!(negotiate(None, Some(" it "), Some("tr")), Locale::It);
    assert_eq!(negotiate(None, None, Some("tr, en;q=0.5")), Locale::Tr);
    assert_eq!(negotiate(None, None, None), Locale::En);
}

#[test]
fn an_unusable_cookie_is_skipped_not_an_error() {
    assert_eq!(negotiate(None, Some("klingon"), Some("tr")), Locale::Tr);
    assert_eq!(negotiate(None, Some(""), None), Locale::En);
    assert_eq!(negotiate(None, Some("klingon"), Some("de")), Locale::En);
}

#[test]
fn the_highest_quality_supported_language_wins() {
    assert_eq!(
        from_accept_language("fr;q=0.9, it;q=0.8, tr;q=0.7"),
        Some(Locale::It)
    );
    assert_eq!(from_accept_language("it;q=0.5, tr;q=0.9"), Some(Locale::Tr));
}

#[test]
fn the_first_of_equals_wins_and_a_missing_quality_is_one() {
    assert_eq!(from_accept_language("tr, it"), Some(Locale::Tr));
    assert_eq!(from_accept_language("it;q=0.9, tr"), Some(Locale::Tr));
    assert_eq!(from_accept_language("it;q=0.8, tr;q=0.8"), Some(Locale::It));
}

#[test]
fn regions_and_case_are_ignored() {
    assert_eq!(from_accept_language("IT-ch"), Some(Locale::It));
    assert_eq!(from_accept_language("tr-TR,tr;q=0.9"), Some(Locale::Tr));
}

#[test]
fn a_zero_or_broken_quality_excludes_the_language() {
    assert_eq!(from_accept_language("it;q=0, tr;q=0.1"), Some(Locale::Tr));
    assert_eq!(from_accept_language("it;q=oops"), None);
    assert_eq!(from_accept_language("de, fr;q=0.8, *;q=0.1"), None);
    assert_eq!(from_accept_language(""), None);
}

#[test]
fn fetch_metadata_decides_when_the_browser_sent_it() {
    assert!(is_same_origin(Some("same-origin"), None, APP));
    for site in ["same-site", "cross-site", "none"] {
        assert!(!is_same_origin(Some(site), Some(APP), APP), "{site}");
    }
}

#[test]
fn without_fetch_metadata_the_origin_must_match_ignoring_case() {
    assert!(is_same_origin(None, Some(APP), APP));
    assert!(is_same_origin(None, Some("https://SMOKE.example.org"), APP));
    assert!(!is_same_origin(None, Some("https://evil.example"), APP));
    assert!(!is_same_origin(None, Some("null"), APP));
}

#[test]
fn a_request_with_neither_header_is_refused() {
    assert!(!is_same_origin(None, None, APP));
}

#[test]
fn the_proxys_entry_is_the_last_one() {
    let peer = Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
    assert_eq!(
        client_ip(Some("203.0.113.9"), peer),
        "203.0.113.9".parse::<IpAddr>().unwrap()
    );
    assert_eq!(
        client_ip(Some("6.6.6.6, 203.0.113.9"), peer),
        "203.0.113.9".parse::<IpAddr>().unwrap()
    );
    assert_eq!(
        client_ip(Some("1.1.1.1,  2001:db8::7 "), peer),
        "2001:db8::7".parse::<IpAddr>().unwrap()
    );
}

#[test]
fn a_missing_or_broken_header_falls_back_to_the_peer_then_to_nothing() {
    let peer = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    assert_eq!(client_ip(None, Some(peer)), peer);
    assert_eq!(client_ip(Some("not an address"), Some(peer)), peer);
    assert_eq!(client_ip(Some(""), Some(peer)), peer);
    assert_eq!(client_ip(None, None), IpAddr::V4(Ipv4Addr::UNSPECIFIED));
}

#[test]
fn invite_tokens_are_cut_from_logged_paths() {
    assert_eq!(logged_path("/invite/0123abcd"), "/invite/…");
    assert_eq!(logged_path("/events/3"), "/events/3");
    assert_eq!(logged_path("/people/invite"), "/people/invite");
}
