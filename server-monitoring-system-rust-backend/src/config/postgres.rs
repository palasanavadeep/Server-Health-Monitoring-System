use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing;

/// PostgreSQL connection pool manager.
/// Mirrors Node.js PostgresConnection class with identical pool settings.
pub async fn create_pool(connection_string: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .idle_timeout(Duration::from_secs(30))
        .acquire_timeout(Duration::from_secs(2))
        .after_connect(|_conn, _meta| {
            Box::pin(async move {
                tracing::debug!("New PG connection established");
                Ok(())
            })
        })
        .connect(connection_string)
        .await?;

    tracing::info!("PG Pool Created");

    // Test connection (mirrors testConnection)
    let row: (chrono::NaiveDateTime,) = sqlx::query_as("SELECT NOW()")
        .fetch_one(&pool)
        .await?;

    tracing::info!("PG connected successfully at {}", row.0);

    Ok(pool)
}
