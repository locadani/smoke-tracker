//! One module per feature.

use crate::error::AppError;

pub(crate) async fn not_found() -> AppError {
    AppError::NotFound
}
