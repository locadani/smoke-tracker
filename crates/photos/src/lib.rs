//! Photo upload validation and storage: a local folder for development, a Google Cloud Storage
//! bucket in production.

mod google;
mod reader;
mod store;
mod upload;

pub use google::GoogleAuth;
pub use reader::{Pm25Reader, TextAnnotation, pick_pm25};
pub use store::{GcsPhotoStore, LocalPhotoStore, PhotoStore, pending_name};
pub use upload::{ValidPhoto, validate};

/// Why a photo could not be accepted, stored or read back.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PhotoError {
    #[error("the photo is larger than the limit")]
    TooLarge,
    #[error("the file is not a JPEG")]
    NotJpeg,
    #[error("the photo is empty")]
    Empty,
    #[error("the photo is not in the store")]
    Missing,
    #[error("the stored photo does not match its fingerprint")]
    Mismatch,
    #[error("photo storage is unavailable: {0}")]
    Unavailable(String),
}
