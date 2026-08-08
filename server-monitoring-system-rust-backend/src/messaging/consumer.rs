use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions},
    types::FieldTable,
    BasicProperties, Channel,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::resilience::circuit_breaker::CircuitBreaker;
use crate::resilience::retry::{is_retryable, RetryStrategy};
use crate::service::processor::ProcessorService;

/// Event consumer — processes messages from RabbitMQ with idempotency,
/// retry, and dead-letter queue routing.
pub struct EventConsumer {
    processor_service: Arc<ProcessorService>,
    channel: Arc<Mutex<Option<Channel>>>,
    queue_name: String,
    retry_strategy: Arc<RetryStrategy>,
    circuit_breaker: Arc<CircuitBreaker>,
    processed_ids: Arc<Mutex<HashSet<String>>>,
    poison_messages: Arc<Mutex<HashMap<String, u32>>>,
    stats: Arc<Mutex<ConsumerStats>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

/// Runtime statistics for the consumer.
#[derive(Default)]
pub struct ConsumerStats {
    pub processed: u64,
    pub failed: u64,
    pub retried: u64,
    pub dlq_routed: u64,
}

impl EventConsumer {
    pub fn new(
        processor_service: Arc<ProcessorService>,
        channel: Channel,
        queue_name: String,
        retry_strategy: Arc<RetryStrategy>,
        circuit_breaker: Arc<CircuitBreaker>,
    ) -> Self {
        Self {
            processor_service,
            channel: Arc::new(Mutex::new(Some(channel))),
            queue_name,
            retry_strategy,
            circuit_breaker,
            processed_ids: Arc::new(Mutex::new(HashSet::new())),
            poison_messages: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(ConsumerStats::default())),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start consuming messages from the queue.
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let channel_guard = self.channel.lock().await;
        let channel = channel_guard.as_ref().ok_or("No channel available")?;

        channel.basic_qos(10, Default::default()).await?;

        let consumer = channel
            .basic_consume(
                &self.queue_name,
                &format!("consumer-{}", chrono::Utc::now().timestamp()),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        self.is_running
            .store(true, std::sync::atomic::Ordering::Relaxed);

        tracing::info!(queue = %self.queue_name, "Started consuming from queue");

        let processor = self.processor_service.clone();
        let cb = self.circuit_breaker.clone();
        let retry = self.retry_strategy.clone();
        let ids = self.processed_ids.clone();
        let poison = self.poison_messages.clone();
        let stats = self.stats.clone();
        let ch = self.channel.clone();
        let queue = self.queue_name.clone();
        let running = self.is_running.clone();

        tokio::spawn(async move {
            use futures::StreamExt;
            let mut consumer = consumer;

            while let Some(delivery_result) = consumer.next().await {
                if !running.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                match delivery_result {
                    Ok(delivery) => {
                        // Circuit breaker gate
                        if !cb.allow_request() {
                            tracing::warn!("Circuit breaker open, requeuing message");
                            if let Err(e) = delivery
                                .nack(BasicNackOptions { requeue: true, ..Default::default() })
                                .await
                            {
                                tracing::error!("Failed to nack message: {}", e);
                            }
                            continue;
                        }

                        let content = String::from_utf8_lossy(&delivery.data).to_string();

                        // Parse JSON
                        let message_data: serde_json::Value = match serde_json::from_str(&content) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::error!("Message parsing failed: {}", e);
                                let _ = delivery.ack(BasicAckOptions::default()).await;
                                continue;
                            }
                        };

                        // Validate message type
                        let msg_type = message_data
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if msg_type != "API_HIT" {
                            tracing::error!("Unknown event type: {}", msg_type);
                            let _ = delivery.ack(BasicAckOptions::default()).await;
                            continue;
                        }

                        // Extract message ID
                        let message_id = delivery
                            .properties
                            .message_id()
                            .as_ref()
                            .map(|s| s.to_string())
                            .or_else(|| {
                                message_data
                                    .get("messageId")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            })
                            .unwrap_or_else(|| "unknown".to_string());

                        // Idempotency check
                        {
                            let ids_guard = ids.lock().await;
                            if ids_guard.contains(&message_id) {
                                tracing::debug!(message_id = %message_id, "Duplicate message skipped");
                                let _ = delivery.ack(BasicAckOptions::default()).await;
                                continue;
                            }
                        }

                        let retry_count: u32 = delivery
                            .properties
                            .headers()
                            .as_ref()
                            .and_then(|h| h.inner().get("x-retry-count"))
                            .and_then(|v| match v {
                                lapin::types::AMQPValue::ShortInt(i) => Some(*i as u32),
                                lapin::types::AMQPValue::LongInt(i) => Some(*i as u32),
                                lapin::types::AMQPValue::LongLongInt(i) => Some(*i as u32),
                                _ => None,
                            })
                            .unwrap_or(0);

                        let event_data = message_data
                            .get("data")
                            .cloned()
                            .unwrap_or(serde_json::json!({}));

                        match processor.process_event(event_data).await {
                            Ok(()) => {
                                let _ = delivery.ack(BasicAckOptions::default()).await;
                                cb.on_success();
                                let mut s = stats.lock().await;
                                s.processed += 1;

                                // Track processed IDs (bounded at 100k)
                                let mut id_set = ids.lock().await;
                                id_set.insert(message_id);
                                if id_set.len() > 100_000 {
                                    if let Some(first) = id_set.iter().next().cloned() {
                                        id_set.remove(&first);
                                    }
                                }

                                poison.lock().await.remove(msg_type);
                            }
                            Err(e) => {
                                cb.on_failure();
                                let mut s = stats.lock().await;
                                s.failed += 1;
                                let err_msg = format!("{}", e);

                                // Poison message tracking
                                {
                                    let mut pm = poison.lock().await;
                                    let count = pm.entry(msg_type.to_string()).or_insert(0);
                                    *count += 1;
                                    if *count >= 10 {
                                        tracing::error!(
                                            event_type = %msg_type,
                                            consecutive_failures = *count,
                                            "Poison message pattern detected"
                                        );
                                    }
                                }

                                if !is_retryable(&err_msg) || !retry.should_retry(retry_count) {
                                    // Route to DLQ
                                    let dlq_name = format!("{}.dlq", queue);
                                    let ch_guard = ch.lock().await;
                                    if let Some(channel) = ch_guard.as_ref() {
                                        let reason = if retry_count >= retry.max_retries {
                                            "MAX_RETRIES_EXCEEDED"
                                        } else {
                                            "NON_RETRYABLE"
                                        };

                                        let mut headers = FieldTable::default();
                                        headers.insert("x-dlq-reason".into(), lapin::types::AMQPValue::LongString(reason.into()));
                                        headers.insert("x-dlq-error".into(), lapin::types::AMQPValue::LongString(err_msg.clone().into()));
                                        headers.insert("x-dlq-timestamp".into(), lapin::types::AMQPValue::LongLongInt(chrono::Utc::now().timestamp()));

                                        let props = BasicProperties::default()
                                            .with_delivery_mode(2)
                                            .with_headers(headers);

                                        let _ = channel
                                            .basic_publish("", &dlq_name, BasicPublishOptions::default(), &delivery.data, props)
                                            .await;
                                    }

                                    let _ = delivery.ack(BasicAckOptions::default()).await;
                                    s.dlq_routed += 1;
                                } else {
                                    // Schedule retry with backoff
                                    let delay = retry.delay(retry_count);
                                    let ch_clone = ch.clone();
                                    let queue_clone = queue.clone();
                                    let data = delivery.data.clone();
                                    let new_retry = retry_count + 1;

                                    tokio::spawn(async move {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                                        let ch_guard = ch_clone.lock().await;
                                        if let Some(channel) = ch_guard.as_ref() {
                                            let mut headers = FieldTable::default();
                                            headers.insert("x-retry-count".into(), lapin::types::AMQPValue::LongInt(new_retry as i32));
                                            headers.insert("x-retry-timestamp".into(), lapin::types::AMQPValue::LongLongInt(chrono::Utc::now().timestamp()));

                                            let props = BasicProperties::default()
                                                .with_delivery_mode(2)
                                                .with_headers(headers);

                                            if let Err(e) = channel
                                                .basic_publish("", &queue_clone, BasicPublishOptions::default(), &data, props)
                                                .await
                                            {
                                                tracing::error!("Failed to schedule retry: {}", e);
                                            }
                                        }
                                    });

                                    let _ = delivery.ack(BasicAckOptions::default()).await;
                                    s.retried += 1;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Consumer error: {}", e);
                        break;
                    }
                }
            }

            tracing::info!("Consumer loop ended");
        });

        tracing::info!("Event consumer is running");
        Ok(())
    }

    /// Signal the consumer to stop processing.
    pub fn stop(&self) {
        self.is_running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Consumer stop requested");
    }

    /// Get current consumer statistics.
    pub async fn get_stats(&self) -> (u64, u64, u64, u64) {
        let s = self.stats.lock().await;
        (s.processed, s.failed, s.retried, s.dlq_routed)
    }
}
