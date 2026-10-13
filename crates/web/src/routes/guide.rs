//! The rating guide (spec §5): a compact version in a popup on the Log screen, and this full
//! page. The texts live in the translation files so they are easy to edit.

use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;
use smoke_domain::types::Level;

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::i18n::Tr;

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

/// Every key [`levels`] looks up, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<String> {
    (Level::MIN..=Level::MAX)
        .flat_map(|v| [format!("intensity-{v}-name"), format!("intensity-{v}-desc")])
        .collect()
}

#[derive(Template)]
#[template(path = "guide.html")]
struct GuidePage {
    tr: Tr,
    signed_in: Option<String>,
    levels: Vec<GuideRow>,
}

pub(crate) async fn rating(CurrentUser(user): CurrentUser, tr: Tr) -> Result<Response, AppError> {
    page(
        StatusCode::OK,
        &GuidePage {
            tr,
            signed_in: Some(user.username.to_string()),
            levels: levels(tr),
        },
    )
}
