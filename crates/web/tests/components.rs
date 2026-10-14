//! The Askama macros of `templates/components.html`, rendered on their own.

mod common;

use askama::Template;
use common::page::Page;

struct Language {
    value: &'static str,
    name: &'static str,
    current: bool,
}

#[derive(Template)]
#[template(
    ext = "html",
    source = r#"{% import "components.html" as c %}
{% call c::card("Rooms") %}<p>body</p>{% endcall %}
{% call c::collapsible("Time", "now", "Change", open) %}<input name="observed_at">{% endcall %}
{% call c::chip("checkbox", "symptom", 3, "Cough", true) %}{% endcall %}
{% call c::chip("radio", "area", "inside", "Inside", false) %}{% endcall %}
{% call c::field("Username") %}<input name="username">{% endcall %}
{% call c::post_button("/events/1/reopen", "Reopen", "secondary") %}{% endcall %}
{% call c::notice("Saved.", false) %}{% endcall %}
{% call c::notice("Pick a room.", true) %}{% endcall %}
{% call c::row("/events/4", "21:40 – in progress", "3 reports") %}<span class="badge">6</span>{% endcall %}
{% call c::row("", "21:00", "") %}{% endcall %}
{% call c::back("/events", "Events") %}{% endcall %}
{% call c::languages(languages, "Language", "settings") %}{% endcall %}"#
)]
struct Gallery {
    open: bool,
    languages: Vec<Language>,
}

fn gallery(open: bool) -> Page {
    let languages = vec![
        Language {
            value: "en",
            name: "English",
            current: true,
        },
        Language {
            value: "it",
            name: "Italiano",
            current: false,
        },
    ];
    Page::parse(&Gallery { open, languages }.render().unwrap())
}

#[test]
fn a_card_has_its_title_and_body() {
    let page = gallery(false);
    assert_eq!(page.text("section.card > h2"), "Rooms");
    assert!(page.exists("section.card > p"));
}

#[test]
fn a_collapsible_card_is_a_details_element_open_on_request() {
    assert!(gallery(false).exists("details.collapsible:not([open]) input[name=observed_at]"));
    let open = gallery(true);
    assert!(open.exists("details.collapsible[open]"));
    assert_eq!(
        open.text("details.collapsible > summary"),
        "Time · now Change"
    );
    assert!(open.exists("summary .action use[href='/static/icons.svg#chevron-down']"));
}

#[test]
fn chips_keep_the_native_input() {
    let page = gallery(false);
    assert!(page.exists("label.chip > input[type=checkbox][name=symptom][value='3'][checked]"));
    assert!(page.exists("label.chip > input[type=radio][name=area][value=inside]:not([checked])"));
    assert_eq!(page.texts("label.chip > span"), ["Cough", "Inside"]);
}

#[test]
fn fields_buttons_and_notices() {
    let page = gallery(false);
    assert_eq!(page.text("label.field > span"), "Username");
    assert!(page.exists("label.field > input[name=username]"));
    assert_eq!(
        page.text(
            "form[method=post][action='/events/1/reopen'] > button.button[data-variant=secondary]"
        ),
        "Reopen"
    );
    assert_eq!(page.text("p.notice[role=status]"), "Saved.");
    assert_eq!(page.text("p.notice[role=alert]"), "Pick a room.");
}

#[test]
fn rows_link_with_a_chevron_only_when_they_have_an_address() {
    let page = gallery(false);
    assert_eq!(
        page.text("a.row[href='/events/4'] .row-text > strong"),
        "21:40 – in progress"
    );
    assert_eq!(page.text("a.row .row-text > span"), "3 reports");
    assert!(page.exists("a.row > .badge + .row-text + svg.icon"));
    assert!(page.exists("div.row .row-text > strong"));
    assert!(!page.exists("div.row svg"));
    assert!(!page.exists("div.row .row-text > span"));
}

#[test]
fn the_back_link_and_the_language_buttons() {
    let page = gallery(false);
    assert_eq!(page.text("a.back[href='/events']"), "Events");
    assert!(page.exists("form.segmented[action='/lang'] > input[name=return_to][value=settings]"));
    assert_eq!(
        page.attr("button[name=lang][aria-current=true]", "value")
            .as_deref(),
        Some("en")
    );
    assert_eq!(page.count("button[name=lang]"), 2);
}

#[test]
fn icons_are_decorative_and_exist_in_the_sprite() {
    let sprite = include_str!("../static/icons.svg");
    let page = gallery(true);
    let icons = page.attrs("svg.icon > use", "href");
    assert!(!icons.is_empty());
    for href in icons {
        let id = href.strip_prefix("/static/icons.svg#").expect("the sprite");
        assert!(sprite.contains(&format!("<symbol id=\"{id}\"")), "{id}");
    }
    assert_eq!(
        page.count("svg.icon"),
        page.count("svg.icon[aria-hidden=true][focusable=false]")
    );
}
