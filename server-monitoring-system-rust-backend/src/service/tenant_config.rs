use std::sync::Arc;

use crate::domain::tenant_config::{HistogramProfile, TenantConfig, TenantConfigUpdate};
use crate::error::app_error::AppError;
use crate::repository::tenant_config_repo::{get_or_default, TenantConfigRepository};
use crate::cache::tenant_config_cache::TenantConfigCache;

/// Service layer for per-tenant metric configuration.
///
/// Wraps the repository with a TTL cache so that analytics read paths avoid a
/// database round-trip on every request. Cache is invalidated on every write.
pub struct TenantConfigService {
    repo:  Arc<dyn TenantConfigRepository>,
    cache: Arc<TenantConfigCache>,
}

impl TenantConfigService {
    pub fn new(
        repo:  Arc<dyn TenantConfigRepository>,
        cache: Arc<TenantConfigCache>,
    ) -> Self {
        Self { repo, cache }
    }

    /// Return the effective config for a client, using the cache.
    ///
    /// Falls back to system defaults if the client has no config row.
    pub async fn get_config(&self, client_id: &str) -> Result<Arc<TenantConfig>, AppError> {
        let repo = self.repo.clone();
        let client_id_owned = client_id.to_string();

        self.cache
            .get_or_load(client_id, || async move {
                get_or_default(repo.as_ref(), &client_id_owned).await
            })
            .await
    }

    /// Insert or replace a client's full configuration.
    ///
    /// Evicts the cache entry so the next request sees the updated config.
    pub async fn upsert_config(&self, config: &TenantConfig) -> Result<(), AppError> {
        self.repo.upsert(config).await?;
        self.cache.invalidate(&config.client_id).await;
        Ok(())
    }

    /// Apply a partial update to an existing config row.
    pub async fn update_config(
        &self,
        client_id: &str,
        update:    &TenantConfigUpdate,
    ) -> Result<(), AppError> {
        if self.repo.get(client_id).await?.is_none() {
            let mut cfg = TenantConfig::default();
            cfg.client_id = client_id.to_string();
            if let Some(t) = update.apdex_threshold_ms {
                cfg.apdex_threshold_ms = t;
            }
            if let Some(ret) = update.data_retention_days {
                cfg.data_retention_days = ret;
            }
            if let Some(quota) = update.daily_ingest_quota {
                cfg.daily_ingest_quota = quota;
            }
            if let Some(ref p_name) = update.histogram_profile {
                let profiles = self.repo.list_profiles().await?;
                if let Some(matched) = profiles.into_iter().find(|p| p.name == *p_name) {
                    cfg.histogram_profile = matched;
                }
            }
            self.upsert_config(&cfg).await?;
        } else {
            self.repo.update(client_id, update).await?;
            self.cache.invalidate(client_id).await;
        }
        Ok(())
    }

    /// List all named histogram profiles available for selection.
    pub async fn list_profiles(&self) -> Result<Vec<HistogramProfile>, AppError> {
        self.repo.list_profiles().await
    }

    /// Create a new custom histogram profile.
    ///
    /// The PostgreSQL CHECK constraint on `histogram_profiles.bucket_bounds`
    /// enforces that all bounds are a subset of the 23 canonical boundaries.
    pub async fn create_profile(&self, profile: &HistogramProfile) -> Result<(), AppError> {
        self.repo.create_profile(profile).await
    }
}
