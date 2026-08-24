use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions},
    types::FieldTable,
    BasicProperties, Channel,
};
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::ingest::HitEvent;
use crate::resilience::circuit_breaker::CircuitBreaker;
use crate::resilience::retry::{is_retryable, RetryStrategy};
use crate::service::processor::ProcessorService;

/// Event consumer — processes messages from RabbitMQ with idempotency,
/// retry, and dead-letter queue routing.
///
/// Deserializes incoming bytes directly into typed `HitEvent` structs,
/// bypassing `serde_json::Value` intermediate representation on the hot path.
///
/// ## Concurrency model
///
/// - The main consume loop runs in a single `tokio::spawn` task whose
///   `JoinHandle` is returned from `start()`.
/// - Retry republishes are bounded by a `Semaphore` to prevent unbounded
///   task spawning under sustained failures.
/// - Shutdown uses a `CancellationToken` for cooperative cancellation.
/// - The idempotency cache uses a bounded `LruCache` instead of an
///   unbounded `HashSet`.
pub struct EventConsumer {
    processor_service: Arc<ProcessorService>,
    channel: Arc<Mutex<Option<Channel>>>,
    queue_name: String,
    retry_strategy: Arc<RetryStrategy>,
    circuit_breaker: Arc<CircuitBreaker>,
    processed_ids: Arc<Mutex<LruCache<String, ()>>>,
    poison_messages: Arc<Mutex<HashMap<String, u32>>>,
    stats: Arc<Mutex<ConsumerStats>>,
    cancel_token: CancellationToken,
    /// Bounds the number of concurrent retry tasks to prevent resource exhaustion.
    retry_semaphore: Arc<Semaphore>,
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
        idempotency_cache_size: usize,
    ) -> Self {
        let cache_cap =
            NonZeroUsize::new(idempotency_cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());

        Self {
            processor_service,
            channel: Arc::new(Mutex::new(Some(channel))),
            queue_name,
            retry_strategy,
            circuit_breaker,
            processed_ids: Arc::new(Mutex::new(LruCache::new(cache_cap))),
            poison_messages: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(ConsumerStats::default())),
            cancel_token: CancellationToken::new(),
            // Allow up to 50 concurrent retry tasks
            retry_semaphore: Arc::new(Semaphore::new(50)),
        }
    }

    /// Start consuming messages from the queue.
    ///
    /// Returns a `JoinHandle` that the caller should `.await` on shutdown
    /// to ensure all in-flight work completes.
    pub async fn start(&self) -> Result<JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
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

        tracing::info!(queue = %self.queue_name, "Started consuming from queue");

        let processor = self.processor_service.clone();
        let cb = self.circuit_breaker.clone();
        let retry = self.retry_strategy.clone();
        let ids = self.processed_ids.clone();
        let poison = self.poison_messages.clone();
        let stats = self.stats.clone();
        let ch = self.channel.clone();
        let queue = self.queue_name.clone();
        let cancel = self.cancel_token.clone();
        let retry_sem = self.retry_semaphore.clone();

        let handle = tokio::spawn(async move {
            use futures::StreamExt;
            let mut consumer = consumer;

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        tracing::info!("Consumer received cancellation signal");
                        break;
                    }
                    maybe_delivery = consumer.next() => {
                        match maybe_delivery {
                            Some(Ok(delivery)) => {
                                handle_delivery(
                                    &delivery,
                                    &processor,
                                    &cb,
                                    &retry,
                                    &ids,
                                    &poison,
                                    &stats,
                                    &ch,
                                    &queue,
                                    &retry_sem,
                                ).await;
                            }
                            Some(Err(e)) => {
                                tracing::error!("Consumer error: {}", e);
                                break;
                            }
                            None => {
                                tracing::info!("Consumer stream ended");
                                break;
                            }
                        }
                    }
                }
            }

            tracing::info!("Consumer loop ended");
        });

        tracing::info!("Event consumer is running");
        Ok(handle)
    }

    /// Signal the consumer to stop processing cooperatively.
    pub fn stop(&self) {
        self.cancel_token.cancel();
        tracing::info!("Consumer stop requested");
    }

    /// Get current consumer statistics.
    pub async fn get_stats(&self) -> (u64, u64, u64, u64) {
        let s = self.stats.lock().await;
        (s.processed, s.failed, s.retried, s.dlq_routed)
    }
}

// ── Extracted message handler ──────────────────────────────────────────────────

/// Process a single delivered message — extracted from the monolithic closure
/// for readability and testability.
async fn handle_delivery(
    delivery: &lapin::message::Delivery,
    processor: &Arc<ProcessorService>,
    cb: &Arc<CircuitBreaker>,
    retry: &Arc<RetryStrategy>,
    ids: &Arc<Mutex<LruCache<String, ()>>>,
    poison: &Arc<Mutex<HashMap<String, u32>>>,
    stats: &Arc<Mutex<ConsumerStats>>,
    ch: &Arc<Mutex<Option<Channel>>>,
    queue: &str,
    retry_sem: &Arc<Semaphore>,
) {
    // Circuit breaker gate
    if !cb.allow_request() {
        tracing::warn!("Circuit breaker open, requeuing message");
        if let Err(e) = delivery
            .nack(BasicNackOptions {
                requeue: true,
                ..Default::default()
            })
            .await
        {
            tracing::error!("Failed to nack message: {}", e);
        }
        return;
    }

    // Deserialize directly to typed HitEvent — no Value intermediate.
    let hit_event: HitEvent = match serde_json::from_slice(&delivery.data) {
        Ok(e) => e,
        Err(err) => {
            tracing::error!(
                error = %err,
                raw = %String::from_utf8_lossy(&delivery.data),
                "Failed to deserialize HitEvent — discarding message"
            );
            let _ = delivery.ack(BasicAckOptions::default()).await;
            return;
        }
    };

    if hit_event.event_type != "API_HIT" {
        tracing::error!(
            event_type = %hit_event.event_type,
            "Unknown event type — discarding"
        );
        let _ = delivery.ack(BasicAckOptions::default()).await;
        return;
    }

    let event_id = hit_event.data.event_id.clone();

    // Extract message ID from AMQP properties or event payload
    let message_id = delivery
        .properties
        .message_id()
        .as_ref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| event_id.clone());

    // Idempotency check — LRU cache automatically evicts oldest entries
    {
        let mut ids_guard = ids.lock().await;
        if ids_guard.get(&message_id).is_some() {
            tracing::debug!(
                message_id = %message_id,
                "Duplicate message skipped"
            );
            let _ = delivery.ack(BasicAckOptions::default()).await;
            return;
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

    // Pass typed HitEventData directly to the processor
    match processor.process_event(hit_event.data).await {
        Ok(()) => {
            let _ = delivery.ack(BasicAckOptions::default()).await;
            cb.on_success();

            {
                let mut s = stats.lock().await;
                s.processed += 1;
            }

            // Track processed IDs — LRU automatically evicts oldest when full
            {
                let mut id_set = ids.lock().await;
                id_set.put(message_id.clone(), ());
            }

            poison.lock().await.remove(&message_id);
        }
        Err(e) => {
            cb.on_failure();
            {
                let mut s = stats.lock().await;
                s.failed += 1;
            }
            let err_msg = format!("{}", e);

            // Poison message tracking
            {
                let mut pm = poison.lock().await;
                let count = pm.entry(message_id.clone()).or_insert(0);
                *count += 1;
                if *count >= 10 {
                    tracing::error!(
                        message_id = %message_id,
                        consecutive_failures = *count,
                        "Poison message pattern detected"
                    );
                }
            }

            if !is_retryable(&err_msg) || !retry.should_retry(retry_count) {
                // Route to DLQ
                route_to_dlq(ch, queue, delivery, retry_count, retry, &err_msg).await;

                let _ = delivery.ack(BasicAckOptions::default()).await;
                let mut s = stats.lock().await;
                s.dlq_routed += 1;
            } else {
                // Schedule retry with exponential backoff (bounded by semaphore)
                schedule_retry(
                    retry,
                    retry_count,
                    ch.clone(),
                    queue.to_string(),
                    delivery.data.clone(),
                    retry_sem.clone(),
                );

                let _ = delivery.ack(BasicAckOptions::default()).await;
                let mut s = stats.lock().await;
                s.retried += 1;
            }
        }
    }
}

/// Route a message to the dead-letter queue with diagnostic headers.
async fn route_to_dlq(
    ch: &Arc<Mutex<Option<Channel>>>,
    queue: &str,
    delivery: &lapin::message::Delivery,
    retry_count: u32,
    retry: &Arc<RetryStrategy>,
    err_msg: &str,
) {
    let dlq_name = format!("{}.dlq", queue);
    let ch_guard = ch.lock().await;
    if let Some(channel) = ch_guard.as_ref() {
        let reason = if retry_count >= retry.max_retries {
            "MAX_RETRIES_EXCEEDED"
        } else {
            "NON_RETRYABLE"
        };

        let mut headers = FieldTable::default();
        headers.insert(
            "x-dlq-reason".into(),
            lapin::types::AMQPValue::LongString(reason.into()),
        );
        headers.insert(
            "x-dlq-error".into(),
            lapin::types::AMQPValue::LongString(err_msg.into()),
        );
        headers.insert(
            "x-dlq-timestamp".into(),
            lapin::types::AMQPValue::LongLongInt(chrono::Utc::now().timestamp()),
        );

        let props = BasicProperties::default()
            .with_delivery_mode(2)
            .with_headers(headers);

        let _ = channel
            .basic_publish(
                "",
                &dlq_name,
                BasicPublishOptions::default(),
                &delivery.data,
                props,
            )
            .await;
    }
}

/// Schedule a retry republish with exponential backoff.
///
/// Bounded by a semaphore to prevent unbounded task spawning under
/// sustained failure conditions.
fn schedule_retry(
    retry: &Arc<RetryStrategy>,
    retry_count: u32,
    ch: Arc<Mutex<Option<Channel>>>,
    queue: String,
    data: Vec<u8>,
    retry_sem: Arc<Semaphore>,
) {
    let delay = retry.delay(retry_count);
    let new_retry = retry_count + 1;

    tokio::spawn(async move {
        // Acquire semaphore permit — blocks if too many retries are in flight
        let _permit = match retry_sem.acquire().await {
            Ok(p) => p,
            Err(_) => {
                tracing::error!("Retry semaphore closed, dropping retry");
                return;
            }
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

        let ch_guard = ch.lock().await;
        if let Some(channel) = ch_guard.as_ref() {
            let mut headers = FieldTable::default();
            headers.insert(
                "x-retry-count".into(),
                lapin::types::AMQPValue::LongInt(new_retry as i32),
            );
            headers.insert(
                "x-retry-timestamp".into(),
                lapin::types::AMQPValue::LongLongInt(chrono::Utc::now().timestamp()),
            );

            let props = BasicProperties::default()
                .with_delivery_mode(2)
                .with_headers(headers);

            if let Err(e) = channel
                .basic_publish("", &queue, BasicPublishOptions::default(), &data, props)
                .await
            {
                tracing::error!("Failed to schedule retry: {}", e);
            }
        }
    });
}
