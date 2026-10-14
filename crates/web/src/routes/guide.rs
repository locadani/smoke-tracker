//! The guides of spec §5, under Settings → Guides: *How to rate* (also as a compact popup on the
//! Log screen) and *How to log*. The texts live in the translation files so they are easy to
//! edit, and the README repeats the English ones (a test keeps them equal).

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use smoke_domain::repos::Store;
use smoke_domain::types::Level;

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::header::SignedIn;
use crate::i18n::Tr;
use crate::state::AppState;

/// One row of the guide.
pub(crate) struct GuideRow {
    pub(crate) value: u8,
    pub(crate) name: String,
    pub(crate) description: String,
}

pub(crate) fn levels(tr: Tr) -> Vec<GuideRow> {
    (Level::MIN..=Level::MAX)
        .map(|value| GuideRow {
            value,
            name: tr.t(&format!("intensity-{value}-name")),
            description: tr.t(&format!("intensity-{value}-desc")),
        })
        .collect()
}

/// The situations of the "How to log" table, in the spec's order.
const SITUATIONS: [&str; 3] = ["mistake", "changed", "gone"];

/// The rules of thumb, numbered from 1.
const RULES: u8 = 5;

/// One row of the "How to log" table: what happened, and what to do about it.
pub(crate) struct Situation {
    pub(crate) happened: String,
    pub(crate) action: String,
}

pub(crate) fn situations(tr: Tr) -> Vec<Situation> {
    SITUATIONS
        .iter()
        .map(|s| Situation {
            happened: tr.t(&format!("guide-log-{s}")),
            action: tr.t(&format!("guide-log-{s}-action")),
        })
        .collect()
}

/// The rules of thumb. The auto-close rule names the configured hour (`AUTO_CLOSE_HOUR`).
pub(crate) fn rules(tr: Tr, auto_close_hour: u32) -> Vec<String> {
    let hour = format!("{auto_close_hour:02}:00");
    (1..=RULES)
        .map(|n| tr.t1(&format!("guide-log-rule-{n}"), "hour", &hour))
        .collect()
}

/// Every key [`levels`], [`situations`] and [`rules`] look up, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<String> {
    let levels = (Level::MIN..=Level::MAX)
        .flat_map(|v| [format!("intensity-{v}-name"), format!("intensity-{v}-desc")]);
    let situations = SITUATIONS
        .iter()
        .flat_map(|s| [format!("guide-log-{s}"), format!("guide-log-{s}-action")]);
    let rules = (1..=RULES).map(|n| format!("guide-log-rule-{n}"));
    levels.chain(situations).chain(rules).collect()
}

#[derive(Template)]
#[template(path = "guide.html")]
struct GuidePage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    levels: Vec<GuideRow>,
}

pub(crate) async fn rating<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
) -> Result<Response, AppError> {
    page(
        StatusCode::OK,
        &GuidePage {
            tr,
            signed_in: SignedIn::of(&user, state.clock.now()),
            levels: levels(tr),
        },
    )
}

#[derive(Template)]
#[template(path = "guide_logging.html")]
struct LoggingPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    situations: Vec<Situation>,
    rules: Vec<String>,
}

pub(crate) async fn logging<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
) -> Result<Response, AppError> {
    page(
        StatusCode::OK,
        &LoggingPage {
            tr,
            signed_in: SignedIn::of(&user, state.clock.now()),
            situations: situations(tr),
            rules: rules(tr, state.config.schedule.hour()),
        },
    )
}

#[cfg(test)]
mod tests {
    use smoke_domain::types::Locale;

    use super::*;

    #[test]
    fn the_auto_close_rule_names_the_configured_hour() {
        let rules = rules(Tr(Locale::En), 5);
        assert_eq!(rules.len(), 5);
        assert!(rules[3].contains("(05:00 auto-close)"), "{}", rules[3]);
    }
}
