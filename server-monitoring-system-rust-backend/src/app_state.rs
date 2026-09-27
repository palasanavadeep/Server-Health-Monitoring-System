use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::cache::api_key_cache::ApiKeyCache;
use crate::cache::ingest_quota_tracker::IngestQuotaTracker;
use crate::config::settings::AppConfig;
use crate::resilience::circuit_breaker::CircuitBreaker;
use crate::service::analytics::AnalyticsService;
use crate::service::auth::AuthService;
use crate::service::client::ClientService;
use crate::service::ingest::IngestService;
use crate::service::tenant_config::TenantConfigService;

/// Application-wide dependency injection container.
///
/// All services and caches are wrapped in `Arc` for cheap cloning across
/// request handlers. `db` holds the SeaORM connection pool (also cheap to clone).
/// This struct is registered with actix-web as `web::Data<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub auth_service:         Arc<AuthService>,
    pub client_service:       Arc<ClientService>,
    pub analytics_service:    Arc<AnalyticsService>,
    pub ingest_service:       Arc<IngestService>,
    pub tenant_config_service: Arc<TenantConfigService>,

    /// In-process API key TTL cache — avoids MongoDB on every ingest request.
    pub api_key_cache:        Arc<ApiKeyCache>,
    /// Per-client daily ingest quota tracker.
    pub quota_tracker:        Arc<IngestQuotaTracker>,

    /// SeaORM connection — exposed for health checks and direct queries.
    pub db:              DatabaseConnection,
    pub config:          AppConfig,
    /// Shared circuit breaker — exposed for health endpoint diagnostics.
    pub circuit_breaker: Arc<CircuitBreaker>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        auth_service:          AuthService,
        client_service:        ClientService,
        analytics_service:     AnalyticsService,
        ingest_service:        IngestService,
        tenant_config_service: TenantConfigService,
        api_key_cache:         Arc<ApiKeyCache>,
        quota_tracker:         Arc<IngestQuotaTracker>,
        db:                    DatabaseConnection,
        config:                AppConfig,
        circuit_breaker:       Arc<CircuitBreaker>,
    ) -> Self {
        Self {
            auth_service:          Arc::new(auth_service),
            client_service:        Arc::new(client_service),
            analytics_service:     Arc::new(analytics_service),
            ingest_service:        Arc::new(ingest_service),
            tenant_config_service: Arc::new(tenant_config_service),
            api_key_cache,
            quota_tracker,
            db,
            config,
            circuit_breaker,
        }
    }
}
