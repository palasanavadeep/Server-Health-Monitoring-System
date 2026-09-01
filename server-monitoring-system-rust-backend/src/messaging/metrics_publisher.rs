use lapin::{options::BasicPublishOptions, BasicProperties, Channel};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::ingest::MetricsEvent;

/// Publishes `MetricsEvent` messages to the internal metrics queue.
///
/// ## Publisher confirms
///
/// Every publish uses lapin's double-await pattern:
///
/// ```text
/// channel.basic_publish(...)
///     .await?   ← sends the publish command to the broker
///     .await?   ← waits for RabbitMQ's publisher confirm (ack/nack)
/// ```
///
/// Only after the second `.await` returns `Ok` is it safe to ACK the
/// original `server_hits` message. Without this, a network issue between
/// the broker accepting the publish call and actually persisting the message
/// could silently drop the MetricsEvent while the original is ACKed.
///
/// ## Owned by the consumer
///
/// The `PersistenceConsumer` owns this publisher and calls it between
/// `persist()` and `delivery.ack()`. Services must not hold a reference to
/// this type — they remain queue-agnostic.
pub struct MetricsPublisher {
    channel: Arc<Mutex<Option<Channel>>>,
    metrics_queue: String,
}

impl MetricsPublisher {
    pub fn new(channel: Arc<Mutex<Option<Channel>>>, metrics_queue: String) -> Self {
        Self {
            channel,
            metrics_queue,
        }
    }

    /// Publish a `MetricsEvent` and await the RabbitMQ publisher confirm.
    ///
    /// Returns `Ok(())` only after RabbitMQ has confirmed durable receipt.
    /// Returns `Err` if publish fails or is not confirmed — the caller must
    /// not ACK the original message in this case.
    pub async fn publish(
        &self,
        event: &MetricsEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::to_vec(event)?;

        let props = BasicProperties::default()
            .with_delivery_mode(2) // persistent — survives broker restart
            .with_content_type("application/json".into())
            .with_message_id(event.event_id.clone().into())
            .with_timestamp(chrono::Utc::now().timestamp() as u64);

        let ch_guard = self.channel.lock().await;
        let channel = ch_guard
            .as_ref()
            .ok_or("No RabbitMQ channel available for metrics publish")?;

        channel
            .basic_publish(
                "",                          // default exchange
                &self.metrics_queue,
                BasicPublishOptions::default(),
                &payload,
                props,
            )
            .await?  // send publish command
            .await?; // await publisher confirm from RabbitMQ

        tracing::debug!(
            event_id = %event.event_id,
            queue    = %self.metrics_queue,
            "MetricsEvent published with publisher confirm"
        );

        Ok(())
    }

    /// Replace the channel (e.g. after a reconnection).
    pub async fn set_channel(&self, channel: Channel) {
        let mut guard = self.channel.lock().await;
        *guard = Some(channel);
    }
}
