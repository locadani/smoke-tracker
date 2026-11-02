//! What every handler can reach.

use std::sync::Arc;

use smoke_domain::clock::Clock;
use smoke_photos::{PhotoStore, Pm25Reader};
use tokio::sync::Semaphore;

use crate::auth::limiter::LoginLimiter;
use crate::auth::password::CONCURRENT_HASHES;
use crate::config::WebConfig;

/// Shared application state. `S` is the storage backend, chosen once at startup; it is a type
/// parameter rather than a trait object, so each backend gets its own compiled router.
pub struct AppState<S> {
    pub(crate) store: S,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) config: Arc<WebConfig>,
    pub(crate) limiter: Arc<LoginLimiter>,
    /// Caps parallel Argon2 runs at `CONCURRENT_HASHES` to bound their memory.
    pub(crate) hash_permits: Arc<Semaphore>,
    pub(crate) photos: Arc<PhotoStore>,
    pub(crate) reader: Arc<Pm25Reader>,
}

impl<S> AppState<S> {
    pub fn new(
        store: S,
        clock: Arc<dyn Clock>,
        config: WebConfig,
        photos: PhotoStore,
        reader: Pm25Reader,
    ) -> Self {
        Self {
            store,
            clock,
            config: Arc::new(config),
            limiter: Arc::new(LoginLimiter::default()),
            hash_permits: Arc::new(Semaphore::new(CONCURRENT_HASHES)),
            photos: Arc::new(photos),
            reader: Arc::new(reader),
        }
    }
}

impl<S: Clone> Clone for AppState<S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            clock: self.clock.clone(),
            config: self.config.clone(),
            limiter: self.limiter.clone(),
            hash_permits: self.hash_permits.clone(),
            photos: self.photos.clone(),
            reader: self.reader.clone(),
        }
    }
}
