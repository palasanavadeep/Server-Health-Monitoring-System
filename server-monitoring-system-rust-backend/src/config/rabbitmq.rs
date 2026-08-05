use lapin::{
    options::{QueueDeclareOptions},
    types::FieldTable,
    Channel, Connection, ConnectionProperties,
};
use tracing;

use super::app_config::RabbitMqConfig;

/// RabbitMQ connection manager.
/// Mirrors Node.js RabbitMQConnection class.
pub struct RabbitMqConnection {
    connection: Option<Connection>,
    channel: Option<Channel>,
    config: RabbitMqConfig,
    is_connecting: bool,
}

impl RabbitMqConnection {
    pub fn new(config: RabbitMqConfig) -> Self {
        Self {
            connection: None,
            channel: None,
            config,
            is_connecting: false,
        }
    }

    /// Connect to RabbitMQ and set up queues (main + DLQ).
    pub async fn connect(&mut self) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref channel) = self.channel {
            return Ok(channel.clone());
        }

        if self.is_connecting {
            // Wait for ongoing connection
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if let Some(ref channel) = self.channel {
                return Ok(channel.clone());
            }
        }

        self.is_connecting = true;

        tracing::info!("Connecting to RabbitMQ: {}", self.config.url);

        let connection = Connection::connect(
            &self.config.url,
            ConnectionProperties::default()
                .with_connection_name("server-monitoring".into()),
        )
        .await?;

        let channel = connection.create_channel().await?;

        // DLQ name
        let dlq_name = format!("{}.dlq", self.config.queue);

        // Declare DLQ
        channel
            .queue_declare(
                &dlq_name,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        // Declare main queue with dead letter routing
        let mut args = FieldTable::default();
        args.insert(
            "x-dead-letter-exchange".into(),
            lapin::types::AMQPValue::LongString("".into()),
        );
        args.insert(
            "x-dead-letter-routing-key".into(),
            lapin::types::AMQPValue::LongString(dlq_name.clone().into()),
        );

        channel
            .queue_declare(
                &self.config.queue,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                args,
            )
            .await?;

        tracing::info!("RabbitMQ connected, queue: {}", self.config.queue);

        self.connection = Some(connection);
        self.channel = Some(channel.clone());
        self.is_connecting = false;

        Ok(channel)
    }

    /// Get the current channel.
    pub fn get_channel(&self) -> Option<&Channel> {
        self.channel.as_ref()
    }

    /// Get the current connection.
    pub fn get_connection(&self) -> Option<&Connection> {
        self.connection.as_ref()
    }

    /// Get connection status.
    pub fn get_status(&self) -> &str {
        match (&self.connection, &self.channel) {
            (Some(conn), Some(_)) => {
                if conn.status().connected() {
                    "connected"
                } else {
                    "closing"
                }
            }
            _ => "disconnected",
        }
    }

    /// Close the RabbitMQ connection.
    pub async fn close(&mut self) {
        if let Some(channel) = self.channel.take() {
            if let Err(e) = channel.close(200, "Shutdown").await {
                tracing::error!("Error closing RabbitMQ channel: {}", e);
            }
        }
        if let Some(connection) = self.connection.take() {
            if let Err(e) = connection.close(200, "Shutdown").await {
                tracing::error!("Error closing RabbitMQ connection: {}", e);
            }
        }
        tracing::info!("RabbitMQ connection closed");
    }

    /// Get config reference.
    pub fn config(&self) -> &RabbitMqConfig {
        &self.config
    }
}
