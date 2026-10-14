//! The frame around every page (look-and-feel spec §3.3, §6.5): the header with the avatar
//! menu and the tab bar when signed in; the language buttons at the bottom when signed out.

mod common;

use common::page::Page;
use common::*;
use smoke_domain::types::{Locale, Role};

async fn page(app: &TestApp, path: &str, cookie: &str) -> Page {
    Page::parse(&text(app.send(get_with(path, cookie)).await).await)
}

#[tokio::test]
async fn the_avatar_menu_names_the_account_and_signs_out() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let home = page(&app, "/", &cookie).await;
    assert_eq!(home.text("header .brand"), "Smoke Tracker");
    assert_eq!(home.text("details.menu[data-menu] > summary.avatar"), "D");
    assert_eq!(
        home.attr("summary.avatar", "aria-label").as_deref(),
        Some("Account: daniele")
    );
    assert_eq!(home.text(".menu-panel > p"), "Signed in as daniele.");
    assert_eq!(
        home.text(r#".menu-panel form[method=post][action="/logout"] button"#),
        "Sign out"
    );
    assert!(!home.exists("footer"), "no language buttons when signed in");
}

#[tokio::test]
async fn the_tab_bar_marks_the_current_screen_and_shows_people_to_residents_only() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let resident = app.sign_in("daniele").await;
    for (path, current) in [
        ("/", "Log"),
        ("/events", "Events"),
        ("/graphs", "Graphs"),
        ("/people", "People"),
        ("/settings", "Settings"),
        ("/guide/rating", "Settings"),
    ] {
        let page = page(&app, path, &resident).await;
        assert_eq!(
            page.texts("nav.tabbar > a"),
            ["Log", "Events", "Graphs", "People", "Settings"],
            "{path}"
        );
        assert_eq!(
            page.texts("nav.tabbar > a[aria-current=page]"),
            [current],
            "{path}"
        );
        assert_eq!(
            page.count("nav.tabbar svg.icon[aria-hidden=true]"),
            5,
            "{path}"
        );
    }
    let guest = app.sign_in("guest").await;
    let page = page(&app, "/", &guest).await;
    assert_eq!(
        page.attrs("nav.tabbar > a", "href"),
        ["/", "/events", "/graphs", "/settings"]
    );
}

#[tokio::test]
async fn signed_out_pages_end_with_the_language_buttons_and_have_no_tab_bar() {
    let app = TestApp::new().await;
    for path in ["/login", "/offline", "/install"] {
        let page = Page::parse(&text(app.send(get(path)).await).await);
        assert!(!page.exists("nav.tabbar"), "{path}");
        assert!(!page.exists("details.menu"), "{path}");
        assert_eq!(
            page.attrs(r#"footer form[action="/lang"] button[name=lang]"#, "value"),
            ["en", "it", "tr"],
            "{path}"
        );
        assert_eq!(
            page.text("footer button[aria-current=true]"),
            "English",
            "{path}"
        );
    }
}
