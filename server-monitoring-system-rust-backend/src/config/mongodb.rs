use ::mongodb::{Client, Database, options::ClientOptions};
use tracing;

use super::app_config::MongoConfig;

/// MongoDB connection manager.
/// Mirrors Node.js MongoConnection class.
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

    /// Connect to MongoDB.
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
