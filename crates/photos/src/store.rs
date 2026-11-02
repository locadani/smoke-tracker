use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use smoke_domain::records::{PhotoKey, Sha256Hex};

use crate::upload::fingerprint;
use crate::{GoogleAuth, PhotoError, ValidPhoto};

pub fn pending_name(sha: &Sha256Hex) -> String {
    format!("pending/{}.jpg", sha.as_str())
}

/// Where photos live. Enum dispatch rather than a trait object: there are exactly two backends.
#[derive(Debug)]
pub enum PhotoStore {
    Local(LocalPhotoStore),
    Gcs(GcsPhotoStore),
}

impl PhotoStore {
    pub async fn put_pending(&self, photo: &ValidPhoto) -> Result<(), PhotoError> {
        match self {
            Self::Local(s) => s.put_pending(photo).await,
            Self::Gcs(s) => s.put_pending(photo).await,
        }
    }

    pub async fn promote(
        &self,
        sha: &Sha256Hex,
        bytes: u32,
        at: DateTime<Utc>,
    ) -> Result<PhotoKey, PhotoError> {
        match self {
            Self::Local(s) => s.promote(sha, bytes, at).await,
            Self::Gcs(s) => s.promote(sha, bytes, at).await,
        }
    }

    pub async fn get(&self, key: &PhotoKey) -> Result<Option<Vec<u8>>, PhotoError> {
        match self {
            Self::Local(s) => s.get(key).await,
            Self::Gcs(s) => s.get(key).await,
        }
    }

    pub async fn list_photos(&self) -> Result<Vec<PhotoKey>, PhotoError> {
        match self {
            Self::Local(s) => s.list_photos().await,
            Self::Gcs(s) => s.list_photos().await,
        }
    }
}

fn check_promoted(stored: &[u8], expected_len: u32, sha: &Sha256Hex) -> Result<(), PhotoError> {
    if stored.len() as u64 == u64::from(expected_len) && fingerprint(stored) == *sha {
        Ok(())
    } else {
        Err(PhotoError::Mismatch)
    }
}

fn unavailable(e: impl std::fmt::Display) -> PhotoError {
    PhotoError::Unavailable(e.to_string())
}

#[derive(Debug)]
pub struct LocalPhotoStore {
    root: PathBuf,
}

impl LocalPhotoStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    async fn write_new(&self, name: &str, bytes: &[u8]) -> Result<(), PhotoError> {
        use tokio::io::AsyncWriteExt;
        let path = self.path(name);
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await.map_err(unavailable)?;
        }
        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(mut file) => {
                file.write_all(bytes).await.map_err(unavailable)?;
                file.flush().await.map_err(unavailable)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(unavailable(e)),
        }
    }

    async fn read(&self, name: &str) -> Result<Option<Vec<u8>>, PhotoError> {
        match tokio::fs::read(self.path(name)).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(unavailable(e)),
        }
    }

    async fn put_pending(&self, photo: &ValidPhoto) -> Result<(), PhotoError> {
        self.write_new(&pending_name(&photo.sha256), &photo.bytes)
            .await
    }

    async fn promote(
        &self,
        sha: &Sha256Hex,
        bytes: u32,
        at: DateTime<Utc>,
    ) -> Result<PhotoKey, PhotoError> {
        let pending = self
            .read(&pending_name(sha))
            .await?
            .ok_or(PhotoError::Missing)?;
        let key = PhotoKey::for_photo(sha, at);
        self.write_new(key.as_str(), &pending).await?;
        let stored = self.read(key.as_str()).await?.ok_or(PhotoError::Missing)?;
        check_promoted(&stored, bytes, sha)?;
        Ok(key)
    }

    async fn get(&self, key: &PhotoKey) -> Result<Option<Vec<u8>>, PhotoError> {
        self.read(key.as_str()).await
    }

    async fn list_photos(&self) -> Result<Vec<PhotoKey>, PhotoError> {
        let mut keys = Vec::new();
        let mut dirs = vec![self.path("photos")];
        while let Some(dir) = dirs.pop() {
            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(entries) => entries,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(unavailable(e)),
            };
            while let Some(entry) = entries.next_entry().await.map_err(unavailable)? {
                let path = entry.path();
                if entry.file_type().await.map_err(unavailable)?.is_dir() {
                    dirs.push(path);
                } else if let Some(key) = self.key_of(&path) {
                    keys.push(key);
                }
            }
        }
        keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(keys)
    }

    fn key_of(&self, path: &Path) -> Option<PhotoKey> {
        let relative = path.strip_prefix(&self.root).ok()?;
        let name = relative
            .components()
            .map(|c| c.as_os_str().to_str())
            .collect::<Option<Vec<_>>>()?
            .join("/");
        PhotoKey::parse(&name).ok()
    }
}

const GCS_API: &str = "https://storage.googleapis.com/storage/v1/b";
const GCS_UPLOAD_API: &str = "https://storage.googleapis.com/upload/storage/v1/b";

/// Percent-encodes everything except RFC 3986 unreserved characters, so an object name's `/`
/// becomes `%2F` inside a URL path segment or query value.
fn encode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(char::from(b));
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn upload_url(bucket: &str, name: &str) -> String {
    format!(
        "{GCS_UPLOAD_API}/{}/o?uploadType=media&name={}&ifGenerationMatch=0",
        encode(bucket),
        encode(name)
    )
}

fn rewrite_url(bucket: &str, src: &str, dst: &str, rewrite_token: Option<&str>) -> String {
    let b = encode(bucket);
    let mut url = format!(
        "{GCS_API}/{b}/o/{}/rewriteTo/b/{b}/o/{}?ifGenerationMatch=0",
        encode(src),
        encode(dst)
    );
    if let Some(token) = rewrite_token {
        url.push_str("&rewriteToken=");
        url.push_str(&encode(token));
    }
    url
}

fn download_url(bucket: &str, name: &str) -> String {
    format!("{GCS_API}/{}/o/{}?alt=media", encode(bucket), encode(name))
}

fn list_url(bucket: &str, page_token: Option<&str>) -> String {
    let mut url = format!(
        "{GCS_API}/{}/o?prefix=photos%2F&fields=items(name),nextPageToken",
        encode(bucket)
    );
    if let Some(token) = page_token {
        url.push_str("&pageToken=");
        url.push_str(&encode(token));
    }
    url
}

#[derive(Debug)]
pub struct GcsPhotoStore {
    bucket: String,
    auth: GoogleAuth,
}

#[derive(Deserialize)]
struct RewriteResponse {
    done: bool,
    #[serde(rename = "rewriteToken")]
    rewrite_token: Option<String>,
}

#[derive(Deserialize)]
struct ListResponse {
    #[serde(default)]
    items: Vec<ListItem>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct ListItem {
    name: String,
}

impl GcsPhotoStore {
    pub fn new(bucket: String, auth: GoogleAuth) -> Self {
        Self { bucket, auth }
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, PhotoError> {
        let token = self.auth.token().await?;
        request
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| PhotoError::Unavailable("storage request failed".to_owned()))
    }

    fn status_error(status: reqwest::StatusCode) -> PhotoError {
        PhotoError::Unavailable(format!("storage answered {status}"))
    }

    async fn put_pending(&self, photo: &ValidPhoto) -> Result<(), PhotoError> {
        let url = upload_url(&self.bucket, &pending_name(&photo.sha256));
        let request = self
            .auth
            .client()
            .post(url)
            .header("Content-Type", "image/jpeg")
            .body(photo.bytes.clone());
        let response = self.send(request).await?;
        match response.status() {
            s if s.is_success() => Ok(()),
            reqwest::StatusCode::PRECONDITION_FAILED => Ok(()),
            s => Err(Self::status_error(s)),
        }
    }

    async fn promote(
        &self,
        sha: &Sha256Hex,
        bytes: u32,
        at: DateTime<Utc>,
    ) -> Result<PhotoKey, PhotoError> {
        let src = pending_name(sha);
        let key = PhotoKey::for_photo(sha, at);
        let mut token: Option<String> = None;
        loop {
            let url = rewrite_url(&self.bucket, &src, key.as_str(), token.as_deref());
            let response = self.send(self.auth.client().post(url)).await?;
            match response.status() {
                reqwest::StatusCode::PRECONDITION_FAILED => break,
                reqwest::StatusCode::NOT_FOUND => return Err(PhotoError::Missing),
                s if !s.is_success() => return Err(Self::status_error(s)),
                _ => {}
            }
            let body: RewriteResponse = response
                .json()
                .await
                .map_err(|_| PhotoError::Unavailable("unreadable rewrite response".to_owned()))?;
            if body.done {
                break;
            }
            token = Some(
                body.rewrite_token
                    .ok_or_else(|| PhotoError::Unavailable("rewrite did not finish".to_owned()))?,
            );
        }
        let stored = self.get(&key).await?.ok_or(PhotoError::Missing)?;
        check_promoted(&stored, bytes, sha)?;
        Ok(key)
    }

    async fn get(&self, key: &PhotoKey) -> Result<Option<Vec<u8>>, PhotoError> {
        let url = download_url(&self.bucket, key.as_str());
        let response = self.send(self.auth.client().get(url)).await?;
        match response.status() {
            reqwest::StatusCode::NOT_FOUND => Ok(None),
            s if s.is_success() => response
                .bytes()
                .await
                .map(|b| Some(b.to_vec()))
                .map_err(|_| PhotoError::Unavailable("storage download failed".to_owned())),
            s => Err(Self::status_error(s)),
        }
    }

    async fn list_photos(&self) -> Result<Vec<PhotoKey>, PhotoError> {
        let mut keys = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let url = list_url(&self.bucket, page.as_deref());
            let response = self.send(self.auth.client().get(url)).await?;
            if !response.status().is_success() {
                return Err(Self::status_error(response.status()));
            }
            let body: ListResponse = response
                .json()
                .await
                .map_err(|_| PhotoError::Unavailable("unreadable list response".to_owned()))?;
            keys.extend(
                body.items
                    .iter()
                    .filter_map(|item| PhotoKey::parse(&item.name).ok()),
            );
            match body.next_page_token {
                Some(next) => page = Some(next),
                None => break,
            }
        }
        keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate;

    fn at() -> DateTime<Utc> {
        "2026-11-02T13:35:00Z".parse().unwrap()
    }

    fn photo(fill: u8) -> ValidPhoto {
        let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
        bytes.resize(64, fill);
        validate(bytes, 1000).unwrap()
    }

    fn local() -> (tempfile::TempDir, LocalPhotoStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalPhotoStore::new(dir.path().to_path_buf());
        (dir, store)
    }

    #[test]
    fn the_key_is_the_month_and_fingerprint() {
        let p = photo(1);
        let sha = p.sha256.as_str();
        assert_eq!(pending_name(&p.sha256), format!("pending/{sha}.jpg"));
        assert_eq!(
            PhotoKey::for_photo(&p.sha256, at()).as_str(),
            format!("photos/2026/11/{sha}.jpg")
        );
    }

    #[tokio::test]
    async fn pending_then_promote_then_get_returns_the_bytes() {
        let (_dir, store) = local();
        let p = photo(1);
        store.put_pending(&p).await.unwrap();
        store.put_pending(&p).await.unwrap();
        let key = store.promote(&p.sha256, 64, at()).await.unwrap();
        assert_eq!(store.get(&key).await.unwrap(), Some(p.bytes));
    }

    #[tokio::test]
    async fn promoting_twice_is_fine() {
        let (_dir, store) = local();
        let p = photo(1);
        store.put_pending(&p).await.unwrap();
        let first = store.promote(&p.sha256, 64, at()).await.unwrap();
        let second = store.promote(&p.sha256, 64, at()).await.unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn promoting_a_missing_pending_photo_fails() {
        let (_dir, store) = local();
        let err = store.promote(&photo(1).sha256, 64, at()).await.unwrap_err();
        assert_eq!(err, PhotoError::Missing);
    }

    #[tokio::test]
    async fn promoting_a_tampered_pending_photo_fails() {
        let (dir, store) = local();
        let p = photo(1);
        store.put_pending(&p).await.unwrap();
        let path = dir.path().join(pending_name(&p.sha256));
        let mut tampered = p.bytes.clone();
        tampered[10] ^= 0xFF;
        std::fs::write(path, tampered).unwrap();
        let err = store.promote(&p.sha256, 64, at()).await.unwrap_err();
        assert_eq!(err, PhotoError::Mismatch);
    }

    #[tokio::test]
    async fn promoting_with_the_wrong_length_fails() {
        let (_dir, store) = local();
        let p = photo(1);
        store.put_pending(&p).await.unwrap();
        let err = store.promote(&p.sha256, 63, at()).await.unwrap_err();
        assert_eq!(err, PhotoError::Mismatch);
    }

    #[tokio::test]
    async fn a_missing_key_is_none() {
        let (_dir, store) = local();
        let key = PhotoKey::for_photo(&photo(1).sha256, at());
        assert_eq!(store.get(&key).await.unwrap(), None);
    }

    #[tokio::test]
    async fn list_photos_lists_only_permanent_photos() {
        let (_dir, store) = local();
        assert!(store.list_photos().await.unwrap().is_empty());
        let (a, b) = (photo(1), photo(2));
        store.put_pending(&a).await.unwrap();
        store.put_pending(&b).await.unwrap();
        let key = store.promote(&a.sha256, 64, at()).await.unwrap();
        assert_eq!(store.list_photos().await.unwrap(), vec![key]);
    }

    #[test]
    fn gcs_upload_never_overwrites() {
        let url = upload_url("bkt", "pending/ab.jpg");
        assert_eq!(
            url,
            "https://storage.googleapis.com/upload/storage/v1/b/bkt/o\
             ?uploadType=media&name=pending%2Fab.jpg&ifGenerationMatch=0"
        );
    }

    #[test]
    fn gcs_promote_copies_server_side() {
        let url = rewrite_url("bkt", "pending/ab.jpg", "photos/2026/11/ab.jpg", None);
        assert_eq!(
            url,
            "https://storage.googleapis.com/storage/v1/b/bkt/o/pending%2Fab.jpg\
             /rewriteTo/b/bkt/o/photos%2F2026%2F11%2Fab.jpg?ifGenerationMatch=0"
        );
        assert!(rewrite_url("bkt", "a", "b", Some("t+k=")).ends_with("&rewriteToken=t%2Bk%3D"));
    }

    #[test]
    fn gcs_download_and_list_urls_are_encoded() {
        assert_eq!(
            download_url("bkt", "photos/2026/11/ab.jpg"),
            "https://storage.googleapis.com/storage/v1/b/bkt/o/photos%2F2026%2F11%2Fab.jpg?alt=media"
        );
        assert!(list_url("bkt", None).contains("prefix=photos%2F"));
        assert!(list_url("bkt", Some("x/y")).ends_with("&pageToken=x%2Fy"));
    }
}
