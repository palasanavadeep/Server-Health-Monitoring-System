use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::settings::AppConfig;
use crate::service::analytics::AnalyticsService;
use crate::service::auth::AuthService;
use crate::service::client::ClientService;
use crate::service::ingest::IngestService;

/// Application-wide dependency injection container.
///
/// All services are wrapped in `Arc` for cheap cloning across request handlers.
/// `db` holds the SeaORM PostgreSQL connection — cheap to clone, pool-backed.
/// This struct is registered with actix-web as `web::Data<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub client_service: Arc<ClientService>,
    pub analytics_service: Arc<AnalyticsService>,
    pub ingest_service: Arc<IngestService>,
    /// SeaORM connection for direct DB access when needed (e.g., health checks).
    pub db: DatabaseConnection,
    pub config: AppConfig,
}

impl AppState {
    /// Construct the application state from assembled services and connections.
    pub fn new(
        auth_service: AuthService,
        client_service: ClientService,
        analytics_service: AnalyticsService,
        ingest_service: IngestService,
        db: DatabaseConnection,
        config: AppConfig,
    ) -> Self {
        Self {
            auth_service: Arc::new(auth_service),
            client_service: Arc::new(client_service),
            analytics_service: Arc::new(analytics_service),
            ingest_service: Arc::new(ingest_service),
            db,
            config,
        }
    }
}
