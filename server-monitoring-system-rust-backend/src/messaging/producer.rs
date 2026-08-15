use lapin::{options::BasicPublishOptions, BasicProperties, Channel};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::settings::RabbitMqConfig;
use crate::domain::ingest::HitEvent;
use crate::resilience::circuit_breaker::CircuitBreaker;
use crate::resilience::retry::{is_retryable, RetryStrategy};

/// Event producer — publishes typed `HitEvent` messages to RabbitMQ.
///
/// Serialization happens once per publish call, not once per retry attempt.
pub struct EventProducer {
    channel: Arc<Mutex<Option<Channel>>>,
    config: RabbitMqConfig,
    circuit_breaker: Arc<CircuitBreaker>,
    retry_strategy: Arc<RetryStrategy>,
}

impl EventProducer {
    pub fn new(
        channel: Channel,
        config: RabbitMqConfig,
        circuit_breaker: Arc<CircuitBreaker>,
        retry_strategy: Arc<RetryStrategy>,
    ) -> Self {
        Self {
            channel: Arc::new(Mutex::new(Some(channel))),
            config,
            circuit_breaker,
            retry_strategy,
        }
    }

    /// Publish an API hit event.
    ///
    /// Serializes the typed `HitEvent` **once** before entering the retry loop,
    /// then reuses the same byte buffer on every retry attempt.
    ///
    /// Returns `true` if published, `false` if rejected by the circuit breaker.
    pub async fn publish_api_hit(
        &self,
        event: HitEvent,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if !self.circuit_breaker.allow_request() {
            tracing::warn!("Circuit breaker OPEN, rejecting publish");
            return Ok(false);
        }

        let event_id = event.data.event_id.clone();

        // Serialize once — reused for every retry without re-allocating.
        let payload = serde_json::to_vec(&event)?;
        let mut attempt: u32 = 0;

        loop {
            match self.try_publish(&payload, &event_id).await {
                Ok(()) => {
                    self.circuit_breaker.on_success();
                    tracing::info!(event_id = %event_id, "Event published to RabbitMQ");
                    return Ok(true);
                }
                Err(e) => {
                    let err_msg = format!("{}", e);
                    tracing::error!(
                        attempt = attempt,
                        error = %err_msg,
                        "Failed to publish event"
                    );

                    if !is_retryable(&err_msg) || !self.retry_strategy.should_retry(attempt) {
                        self.circuit_breaker.on_failure();
                        return Err(e);
                    }

                    self.retry_strategy.wait(attempt).await;
                    attempt += 1;
                }
            }
        }
    }

    async fn try_publish(
        &self,
        payload: &[u8],
        event_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let channel_guard = self.channel.lock().await;
        let channel = channel_guard
            .as_ref()
            .ok_or("No RabbitMQ channel available")?;

        let properties = BasicProperties::default()
            .with_delivery_mode(2) // persistent
            .with_content_type("application/json".into())
            .with_message_id(event_id.into())
            .with_timestamp(chrono::Utc::now().timestamp() as u64);

        channel
            .basic_publish(
                "",
                &self.config.queue,
                BasicPublishOptions::default(),
                payload,
                properties,
            )
            .await?
            .await?;

        Ok(())
    }

    /// Replace the channel (for reconnection scenarios).
    pub async fn set_channel(&self, channel: Channel) {
        let mut guard = self.channel.lock().await;
        *guard = Some(channel);
    }
}
