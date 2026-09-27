//! In-process TTL cache for validated API keys.
//!
//! ## Purpose
//! Avoids a MongoDB round-trip on every ingest request. A validated API key
//! entry is cached for `ttl_secs` (default: 60 s). On TTL expiry the next
//! request triggers a single loader call — concurrent requests during expiry
//! are collapsed via `try_get_with` (moka's built-in single-flight).
//!
//! ## Security
//! - Cache keys are `sha256(raw_api_key)` — plaintext is never stored.
//! - TTL is deliberately short (60 s) so revoked keys become invalid quickly.
//! - Revocation handlers must call `invalidate()` for immediate local effect.
//!   Cross-instance propagation within the same TTL window is a known
//!   accepted trade-off (see ARCHITECTURE.md).

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use moka::future::Cache;
use sha2::{Digest, Sha256};

use crate::domain::api_key::{ApiKeyPermissions, ApiKeySecurity};

// ── Cached value ──────────────────────────────────────────────────────────────

/// Everything the middleware needs to authorise a request, pre-loaded from MongoDB.
///
/// This struct is intentionally flat — no nested `Arc`s — to make cloning cheap.
#[derive(Debug, Clone)]
pub struct CachedApiKeyEntry {
    pub client_id:     String,
    pub key_id:        String,
    pub permissions:   ApiKeyPermissions,
    pub security:      ApiKeySecurity,
    pub is_active:     bool,
    pub expires_at:    Option<DateTime<Utc>>,
    pub client_active: bool,
}

// ── Cache ─────────────────────────────────────────────────────────────────────

/// In-process API key validation cache.
///
/// Wrap in `Arc` for cheap cloning across request handlers.
pub struct ApiKeyCache {
    inner: Cache<String, Arc<CachedApiKeyEntry>>,
}

impl ApiKeyCache {
    pub fn new(ttl_secs: u64, max_capacity: u64) -> Self {
        Self {
            inner: Cache::builder()
                .time_to_live(Duration::from_secs(ttl_secs))
                .max_capacity(max_capacity)
                .build(),
        }
    }

    /// Hash a raw API key for use as the cache key.
    ///
    /// Using a hash ensures the plaintext key is never persisted in-process
    /// (e.g. in a heap dump or tracing output).
    pub fn hash_key(raw_api_key: &str) -> String {
        let digest = Sha256::digest(raw_api_key.as_bytes());
        hex::encode(digest)
    }

    /// Return a cached entry or populate it via `loader`.
    ///
    /// Concurrent calls for the same key during a cache miss are collapsed into
    /// a single `loader` invocation (moka's single-flight guarantee via
    /// `try_get_with`). This prevents a thundering-herd against MongoDB when
    /// a popular key's entry expires under load.
    pub async fn get_or_load<F, Fut>(
        &self,
        raw_api_key: &str,
        loader: F,
    ) -> Result<Arc<CachedApiKeyEntry>, crate::error::app_error::AppError>
    where
        F:   FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<CachedApiKeyEntry, crate::error::app_error::AppError>>
               + Send,
    {
        let key_hash = Self::hash_key(raw_api_key);
        self.inner
            .try_get_with(key_hash, async move { loader().await.map(Arc::new) })
            .await
            .map_err(|e| crate::error::app_error::AppError::internal(e.to_string()))
    }

    /// Remove a specific key from the cache.
    ///
    /// Call after any operation that changes key validity:
    /// delete, deactivate, rotate, or permission update.
    ///
    /// Note: only clears the local instance's cache. Other running instances
    /// will serve the old entry until their TTL expires (≤ 60 s).
    pub async fn invalidate_by_raw_key(&self, raw_api_key: &str) {
        let key_hash = Self::hash_key(raw_api_key);
        self.inner.invalidate(&key_hash).await;
    }

    /// Remove an entry by its pre-computed sha256 hash.
    ///
    /// Use when the raw key is no longer available (e.g. after rotation).
    pub async fn invalidate_by_hash(&self, key_hash: &str) {
        self.inner.invalidate(key_hash).await;
    }
}
