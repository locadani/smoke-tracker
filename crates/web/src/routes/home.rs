//! The signed-in landing page. A placeholder until the Log screen (milestone 4).

use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::i18n::Tr;

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage {
    tr: Tr,
    signed_in: Option<String>,
    username: String,
}

pub(crate) async fn show(CurrentUser(user): CurrentUser, tr: Tr) -> Result<Response, AppError> {
    let username = user.username.to_string();
    page(
        StatusCode::OK,
        &HomePage {
            tr,
            signed_in: Some(username.clone()),
            username,
        },
    )
}
