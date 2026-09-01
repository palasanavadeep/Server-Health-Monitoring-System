use lapin::{
    options::QueueDeclareOptions, types::FieldTable, Channel, Connection, ConnectionProperties,
};

use super::settings::RabbitMqConfig;

/// RabbitMQ connection manager with queue setup (main + DLQ).
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

    /// Connect to RabbitMQ, declare main queue and dead-letter queue.
    pub async fn connect(&mut self) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref channel) = self.channel {
            return Ok(channel.clone());
        }

        if self.is_connecting {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if let Some(ref channel) = self.channel {
                return Ok(channel.clone());
            }
        }

        self.is_connecting = true;

        tracing::info!("Connecting to RabbitMQ: {}", self.config.url);

        let connection = Connection::connect(
            &self.config.url,
            ConnectionProperties::default().with_connection_name("server-monitoring".into()),
        )
        .await?;

        let channel = connection.create_channel().await?;

        // Persistence DLQ — dead-letter target for the main server_hits queue.
        let dlq_name = format!("{}.dlq", self.config.queue);

        channel
            .queue_declare(
                &dlq_name,
                QueueDeclareOptions { durable: true, ..Default::default() },
                FieldTable::default(),
            )
            .await?;

        // Main queue — durable, routes failures to persistence DLQ.
        let mut main_args = FieldTable::default();
        main_args.insert(
            "x-dead-letter-exchange".into(),
            lapin::types::AMQPValue::LongString("".into()),
        );
        main_args.insert(
            "x-dead-letter-routing-key".into(),
            lapin::types::AMQPValue::LongString(dlq_name.clone().into()),
        );

        channel
            .queue_declare(
                &self.config.queue,
                QueueDeclareOptions { durable: true, ..Default::default() },
                main_args,
            )
            .await?;

        // ── Metrics pipeline topology (startup-critical) ───────────────────────
        //
        // These declarations must succeed before any worker starts consuming.
        // If either fails, connect() returns Err and the orchestrator restarts.

        // Metrics DLQ — dead-letter target for the metrics queue.
        let metrics_dlq_name = format!("{}.metrics.dlq", self.config.queue);

        channel
            .queue_declare(
                &metrics_dlq_name,
                QueueDeclareOptions { durable: true, ..Default::default() },
                FieldTable::default(),
            )
            .await?;

        // Metrics queue — durable, routes failures to metrics DLQ.
        let metrics_queue_name = format!("{}.metrics", self.config.queue);

        let mut metrics_args = FieldTable::default();
        metrics_args.insert(
            "x-dead-letter-exchange".into(),
            lapin::types::AMQPValue::LongString("".into()),
        );
        metrics_args.insert(
            "x-dead-letter-routing-key".into(),
            lapin::types::AMQPValue::LongString(metrics_dlq_name.clone().into()),
        );

        channel
            .queue_declare(
                &metrics_queue_name,
                QueueDeclareOptions { durable: true, ..Default::default() },
                metrics_args,
            )
            .await?;

        tracing::info!(
            queue         = %self.config.queue,
            metrics_queue = %metrics_queue_name,
            "RabbitMQ connected — all queues declared"
        );

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

    /// Get connection status as a human-readable string.
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

    /// Close the RabbitMQ connection gracefully.
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

    /// Access the underlying config.
    pub fn config(&self) -> &RabbitMqConfig {
        &self.config
    }
}
