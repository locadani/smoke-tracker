use std::fmt;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::sync::Mutex;

use crate::PhotoError;

const METADATA_TOKEN_URL: &str =
    "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";
const REFRESH_MARGIN_SECS: u64 = 60;

/// Access tokens for the runtime service account, fetched from the metadata server and cached.
/// It also owns the one HTTP client every Google call shares.
pub struct GoogleAuth {
    client: reqwest::Client,
    cached: Mutex<Option<CachedToken>>,
}

struct CachedToken {
    token: String,
    refresh_at: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl GoogleAuth {
    pub fn new() -> Result<Self, PhotoError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| PhotoError::Unavailable(e.to_string()))?;
        Ok(Self {
            client,
            cached: Mutex::new(None),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_fixed_token(token: &str) -> Result<Self, PhotoError> {
        let auth = Self::new()?;
        *auth.cached.try_lock().expect("fresh auth is not locked") = Some(CachedToken {
            token: token.to_owned(),
            refresh_at: Instant::now() + Duration::from_secs(3600),
        });
        Ok(auth)
    }

    pub(crate) fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub async fn token(&self) -> Result<String, PhotoError> {
        let mut cached = self.cached.lock().await;
        if let Some(c) = cached.as_ref()
            && Instant::now() < c.refresh_at
        {
            return Ok(c.token.clone());
        }
        let fresh = self.fetch().await?;
        let token = fresh.token.clone();
        *cached = Some(fresh);
        Ok(token)
    }

    async fn fetch(&self) -> Result<CachedToken, PhotoError> {
        let unavailable = |what: &str| PhotoError::Unavailable(format!("metadata token: {what}"));
        let response = self
            .client
            .get(METADATA_TOKEN_URL)
            .header("Metadata-Flavor", "Google")
            .send()
            .await
            .map_err(|_| unavailable("request failed"))?;
        if !response.status().is_success() {
            return Err(unavailable(&format!("status {}", response.status())));
        }
        let body: TokenResponse = response
            .json()
            .await
            .map_err(|_| unavailable("unreadable response"))?;
        let lifetime = body.expires_in.saturating_sub(REFRESH_MARGIN_SECS);
        Ok(CachedToken {
            token: body.access_token,
            refresh_at: Instant::now() + Duration::from_secs(lifetime),
        })
    }
}

impl fmt::Debug for GoogleAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GoogleAuth").finish_non_exhaustive()
    }
}
