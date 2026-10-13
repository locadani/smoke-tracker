//! One module per feature.

pub(crate) mod home;
pub(crate) mod lang;
pub(crate) mod login;

use crate::error::AppError;

pub(crate) async fn not_found() -> AppError {
    AppError::NotFound
}
