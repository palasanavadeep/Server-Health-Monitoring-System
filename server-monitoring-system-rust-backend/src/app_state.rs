use std::sync::Arc;

use crate::config::app_config::AppConfig;
use crate::services::auth::service::AuthService;
use crate::services::client::service::ClientService;
use crate::services::ingest::service::IngestService;
use crate::services::analytics::service::AnalyticsService;

/// AppState - Shared application state / DI container.
/// Each service module is self-contained; AppState provides the wiring.
/// This design makes it straightforward to extract any service into
/// its own microservice in the future.
pub struct AppState {
    pub config: AppConfig,
    pub auth_service: Arc<AuthService>,
    pub client_service: Arc<ClientService>,
    pub ingest_service: Arc<IngestService>,
    pub analytics_service: Arc<AnalyticsService>,
}
