use ::mongodb::{options::ClientOptions, Client, Database};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

use super::settings::MongoConfig;

/// MongoDB connection manager.
pub struct MongoConnection {
    client: Option<Client>,
    database: Option<Database>,
    config: MongoConfig,
}

impl MongoConnection {
    pub fn new(config: MongoConfig) -> Self {
        Self {
            client: None,
            database: None,
            config,
        }
    }

    /// Establish a connection to MongoDB and return the database handle.
    pub async fn connect(&mut self) -> Result<Database, Box<dyn std::error::Error>> {
        if let Some(ref db) = self.database {
            tracing::info!("MongoDB already connected");
            return Ok(db.clone());
        }

        let mut client_options = ClientOptions::parse(&self.config.uri).await?;
        client_options.app_name = Some("server-monitoring".to_string());

        let client = Client::with_options(client_options)?;

        // Ping to verify connection
        client
            .database("admin")
            .run_command(bson::doc! { "ping": 1 })
            .await?;

        let database = client.database(&self.config.db_name);

        tracing::info!("MongoDB connected: {}", self.config.uri);

        self.client = Some(client);
        self.database = Some(database.clone());

        Ok(database)
    }

    /// Disconnect from MongoDB.
    pub async fn disconnect(&mut self) {
        if self.client.is_some() {
            self.client = None;
            self.database = None;
            tracing::info!("MongoDB disconnected!");
        }
    }

    /// Get the active database handle.
    pub fn get_database(&self) -> Option<&Database> {
        self.database.as_ref()
    }
}

/// Create a PostgreSQL connection pool.
pub async fn create_pg_pool(connection_string: &str) -> Result<PgPool, sqlx::Error> {
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

    // Test connection
    let row: (chrono::NaiveDateTime,) = sqlx::query_as("SELECT NOW()")
        .fetch_one(&pool)
        .await?;

    tracing::info!("PG connected successfully at {}", row.0);

    Ok(pool)
}
