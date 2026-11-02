use sha2::{Digest, Sha256};
use smoke_domain::records::Sha256Hex;

use crate::PhotoError;

const JPEG_MAGIC: [u8; 3] = [0xFF, 0xD8, 0xFF];

/// Bytes that passed [`validate`], with their SHA-256 fingerprint.
#[derive(Debug, Clone)]
pub struct ValidPhoto {
    pub bytes: Vec<u8>,
    pub sha256: Sha256Hex,
}

pub fn validate(bytes: Vec<u8>, max: usize) -> Result<ValidPhoto, PhotoError> {
    if bytes.is_empty() {
        return Err(PhotoError::Empty);
    }
    if bytes.len() > max {
        return Err(PhotoError::TooLarge);
    }
    if !bytes.starts_with(&JPEG_MAGIC) {
        return Err(PhotoError::NotJpeg);
    }
    let sha256 = fingerprint(&bytes);
    Ok(ValidPhoto { bytes, sha256 })
}

/// The SHA-256 fingerprint of a photo's bytes, as [`validate`] computes it.
pub fn fingerprint(bytes: &[u8]) -> Sha256Hex {
    let digest = hex::encode(Sha256::digest(bytes));
    Sha256Hex::parse(&digest).expect("a hex-encoded SHA-256 is 64 lowercase hex characters")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg(len: usize) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
        bytes.resize(len, 7);
        bytes
    }

    #[test]
    fn an_oversized_photo_is_refused() {
        assert_eq!(validate(jpeg(101), 100).unwrap_err(), PhotoError::TooLarge);
        assert!(validate(jpeg(100), 100).is_ok());
    }

    #[test]
    fn a_png_is_refused() {
        let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3];
        assert_eq!(validate(png, 100).unwrap_err(), PhotoError::NotJpeg);
    }

    #[test]
    fn an_empty_upload_is_refused() {
        assert_eq!(validate(vec![], 100).unwrap_err(), PhotoError::Empty);
    }

    #[test]
    fn the_fingerprint_is_sha256_of_the_bytes() {
        let photo = validate(vec![0xFF, 0xD8, 0xFF], 100).unwrap();
        assert_eq!(
            photo.sha256.as_str(),
            "6e568e1f67fba258184c78181539e5e8fdee447e49bb706fc0ea34fbf12336a5"
        );
    }
}
