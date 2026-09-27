//! In-process TTL cache for per-tenant metric configuration.
//!
//! Tenant configs change infrequently (admin operations only), so caching
//! them for 60 seconds eliminates a database round-trip on every analytics
//! query without risking stale data for longer than one TTL cycle.
//!
//! Uses `moka::future::Cache` with `try_get_with` for single-flight on cache
//! misses — concurrent requests for the same tenant during a miss collapse
//! into a single DB load.

use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;

use crate::domain::tenant_config::TenantConfig;

/// In-process TTL cache for [`TenantConfig`].
///
/// Wrap in `Arc` for cheap cloning across request handlers.
pub struct TenantConfigCache {
    inner: Cache<String, Arc<TenantConfig>>,
}

impl TenantConfigCache {
    pub fn new(ttl_secs: u64, max_capacity: u64) -> Self {
        Self {
            inner: Cache::builder()
                .time_to_live(Duration::from_secs(ttl_secs))
                .max_capacity(max_capacity)
                .build(),
        }
    }

    /// Return a cached config or populate it via `loader`.
    ///
    /// Concurrent calls for the same `client_id` during a miss are collapsed
    /// into a single loader invocation (moka single-flight via `try_get_with`).
    pub async fn get_or_load<F, Fut>(
        &self,
        client_id: &str,
        loader:    F,
    ) -> Result<Arc<TenantConfig>, crate::error::app_error::AppError>
    where
        F:   FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<TenantConfig, crate::error::app_error::AppError>>
               + Send,
    {
        self.inner
            .try_get_with(client_id.to_string(), async move { loader().await.map(Arc::new) })
            .await
            .map_err(|e| crate::error::app_error::AppError::internal(e.to_string()))
    }

    /// Evict a client's config from the cache.
    ///
    /// Call after any admin update to `client_metric_config` so the next
    /// request picks up the fresh config within one TTL cycle.
    pub async fn invalidate(&self, client_id: &str) {
        self.inner.invalidate(client_id).await;
    }
}
