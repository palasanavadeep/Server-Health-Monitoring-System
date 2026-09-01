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
use crate::messaging::metrics_publisher::MetricsPublisher;
use crate::resilience::circuit_breaker::CircuitBreaker;
use crate::resilience::retry::{is_retryable, RetryStrategy};
use crate::service::processor::PersistenceService;

/// Persistence consumer — reads from `server_hits`, writes to MongoDB, then
/// publishes to the metrics queue.
///
/// ## Responsibilities (single worker)
///
/// 1. Deserialize `HitEvent` from `server_hits`
/// 2. Call `PersistenceService::persist()` → MongoDB write (idempotent)
/// 3. Call `MetricsPublisher::publish()` → publish MetricsEvent + await publisher confirm
/// 4. ACK the original message — only if both steps 2 and 3 succeeded
///
/// ## Failure model
///
/// - MongoDB failure → NACK + exponential backoff retry
/// - Metrics publish failure → NACK + exponential backoff retry
///   (MongoDB already has the event; duplicate insert is idempotent)
/// - ACK failure → RabbitMQ redelivers → MongoDB dedup + metrics dedup = safe
///
/// ## Concurrency model
///
/// - Main consume loop runs in a single `tokio::spawn` task.
/// - Retry republishes are bounded by a `Semaphore` to prevent unbounded spawning.
/// - Shutdown uses `CancellationToken` for cooperative cancellation.
/// - LruCache is a **performance optimization** only — populated after successful
///   MongoDB + metrics-publish. PostgreSQL is the source of truth for idempotency.
pub struct PersistenceConsumer {
    persistence_service: Arc<PersistenceService>,
    metrics_publisher: Arc<MetricsPublisher>,
    channel: Arc<Mutex<Option<Channel>>>,
    queue_name: String,
    retry_strategy: Arc<RetryStrategy>,
    circuit_breaker: Arc<CircuitBreaker>,
    /// LruCache of recently processed message_ids.
    /// Populated AFTER successful MongoDB persist + metrics publish.
    /// Evicts oldest entries automatically when full.
    processed_ids: Arc<Mutex<LruCache<String, ()>>>,
    poison_messages: Arc<Mutex<HashMap<String, u32>>>,
    stats: Arc<Mutex<ConsumerStats>>,
    cancel_token: CancellationToken,
    retry_semaphore: Arc<Semaphore>,
}

/// Runtime statistics for the persistence consumer.
#[derive(Default)]
pub struct ConsumerStats {
    pub processed: u64,
    pub failed: u64,
    pub retried: u64,
    pub dlq_routed: u64,
}

impl PersistenceConsumer {
    pub fn new(
        persistence_service: Arc<PersistenceService>,
        metrics_publisher: Arc<MetricsPublisher>,
        channel: Channel,
        queue_name: String,
        retry_strategy: Arc<RetryStrategy>,
        circuit_breaker: Arc<CircuitBreaker>,
        idempotency_cache_size: usize,
    ) -> Self {
        let cache_cap =
            NonZeroUsize::new(idempotency_cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());

        Self {
            persistence_service,
            metrics_publisher,
            channel: Arc::new(Mutex::new(Some(channel))),
            queue_name,
            retry_strategy,
            circuit_breaker,
            processed_ids: Arc::new(Mutex::new(LruCache::new(cache_cap))),
            poison_messages: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(ConsumerStats::default())),
            cancel_token: CancellationToken::new(),
            retry_semaphore: Arc::new(Semaphore::new(50)),
        }
    }

    /// Start consuming from the queue.
    ///
    /// Returns a `JoinHandle` that should be awaited on shutdown to allow
    /// in-flight messages to complete.
    pub async fn start(
        &self,
    ) -> Result<JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
        let channel_guard = self.channel.lock().await;
        let channel = channel_guard.as_ref().ok_or("No channel available")?;

        channel.basic_qos(10, Default::default()).await?;

        let consumer = channel
            .basic_consume(
                &self.queue_name,
                &format!("persistence-consumer-{}", chrono::Utc::now().timestamp()),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        tracing::info!(queue = %self.queue_name, "Persistence consumer started");

        // Clone all shared state for the spawned task.
        let persistence_svc = self.persistence_service.clone();
        let publisher = self.metrics_publisher.clone();
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
                        tracing::info!("Persistence consumer received cancellation signal");
                        break;
                    }
                    maybe_delivery = consumer.next() => {
                        match maybe_delivery {
                            Some(Ok(delivery)) => {
                                handle_delivery(
                                    &delivery,
                                    &persistence_svc,
                                    &publisher,
                                    &cb,
                                    &retry,
                                    &ids,
                                    &poison,
                                    &stats,
                                    &ch,
                                    &queue,
                                    &retry_sem,
                                )
                                .await;
                            }
                            Some(Err(e)) => {
                                tracing::error!("Consumer stream error: {}", e);
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

            tracing::info!("Persistence consumer loop ended");
        });

        Ok(handle)
    }

    /// Signal the consumer to stop processing cooperatively.
    pub fn stop(&self) {
        self.cancel_token.cancel();
        tracing::info!("Persistence consumer stop requested");
    }

    /// Get current consumer statistics.
    pub async fn get_stats(&self) -> (u64, u64, u64, u64) {
        let s = self.stats.lock().await;
        (s.processed, s.failed, s.retried, s.dlq_routed)
    }
}

// ── Message handler ────────────────────────────────────────────────────────────

/// Process a single delivered message.
///
/// ## ACK sequence (the critical ordering)
///
/// 1. `persistence_service.persist()` — MongoDB write (idempotent)
/// 2. `metrics_publisher.publish()` — publish MetricsEvent + await publisher confirm
/// 3. `delivery.ack()` — ACK original ONLY after both above succeeded
///
/// Failure at step 1 or 2 → NACK with exponential backoff → redelivery.
/// Failure at step 3 (ACK lost) → redelivery → MongoDB dedup + metrics dedup = safe.
#[allow(clippy::too_many_arguments)]
async fn handle_delivery(
    delivery: &lapin::message::Delivery,
    persistence_svc: &Arc<PersistenceService>,
    publisher: &Arc<MetricsPublisher>,
    cb: &Arc<CircuitBreaker>,
    retry: &Arc<RetryStrategy>,
    ids: &Arc<Mutex<LruCache<String, ()>>>,
    poison: &Arc<Mutex<HashMap<String, u32>>>,
    stats: &Arc<Mutex<ConsumerStats>>,
    ch: &Arc<Mutex<Option<Channel>>>,
    queue: &str,
    retry_sem: &Arc<Semaphore>,
) {
    // Circuit breaker gate — requeue immediately if downstream is unhealthy.
    if !cb.allow_request() {
        tracing::warn!("Circuit breaker open — requeuing message");
        let _ = delivery
            .nack(BasicNackOptions {
                requeue: true,
                ..Default::default()
            })
            .await;
        return;
    }

    // Deserialize directly to typed HitEvent — no Value intermediate.
    let hit_event: HitEvent = match serde_json::from_slice(&delivery.data) {
        Ok(e) => e,
        Err(err) => {
            tracing::error!(
                error = %err,
                raw   = %String::from_utf8_lossy(&delivery.data),
                "Failed to deserialize HitEvent — discarding (unrecoverable)"
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

    // Use AMQP message_id if present, fall back to event_id.
    let message_id = delivery
        .properties
        .message_id()
        .as_ref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| event_id.clone());

    // LruCache fast path — skip DB roundtrip for recently-processed messages.
    // The cache is populated AFTER successful persist + publish (see success arm below),
    // so a cache hit means both operations already completed for this message_id.
    {
        let mut ids_guard = ids.lock().await;
        if ids_guard.get(&message_id).is_some() {
            tracing::debug!(message_id = %message_id, "LruCache hit — skipping duplicate");
            let _ = delivery.ack(BasicAckOptions::default()).await;
            return;
        }
    }

    let retry_count: u32 = extract_retry_count(delivery);

    // ── Step 1: persist to MongoDB ─────────────────────────────────────────────
    let metrics_event = match persistence_svc.persist(hit_event.data).await {
        Ok(me) => me,
        Err(e) => {
            cb.on_failure();
            let err_msg = format!("{e}");
            tracing::error!(
                event_id    = %event_id,
                retry_count = retry_count,
                error       = %err_msg,
                "MongoDB persist failed"
            );
            handle_failure(
                delivery, ch, queue, retry, retry_count, poison,
                stats, retry_sem, &message_id, &err_msg,
            )
            .await;
            return;
        }
    };

    // ── Step 2: publish MetricsEvent + await publisher confirm ─────────────────
    if let Err(e) = publisher.publish(&metrics_event).await {
        // MongoDB already has the event (idempotent on retry), but the metrics
        // queue does not. Do NOT ACK — let RabbitMQ redeliver.
        cb.on_failure();
        let err_msg = format!("{e}");
        tracing::error!(
            event_id    = %event_id,
            retry_count = retry_count,
            error       = %err_msg,
            "MetricsEvent publish failed — will retry without re-persisting to MongoDB"
        );
        handle_failure(
            delivery, ch, queue, retry, retry_count, poison,
            stats, retry_sem, &message_id, &err_msg,
        )
        .await;
        return;
    }

    // ── Step 3: ACK original — only after both steps above succeeded ───────────
    let _ = delivery.ack(BasicAckOptions::default()).await;
    cb.on_success();

    {
        let mut s = stats.lock().await;
        s.processed += 1;
    }

    // Populate LruCache AFTER successful persist + publish (not before).
    // This prevents the cache from hiding events if the process crashes
    // between cache-insert and the actual DB operations.
    {
        let mut id_set = ids.lock().await;
        id_set.put(message_id.clone(), ());
    }

    poison.lock().await.remove(&message_id);

    tracing::info!(
        event_id = %event_id,
        "Event persisted to MongoDB and queued for metrics"
    );
}

// ── Failure handling ───────────────────────────────────────────────────────────

/// Decide whether to retry (with backoff) or route to DLQ.
#[allow(clippy::too_many_arguments)]
async fn handle_failure(
    delivery: &lapin::message::Delivery,
    ch: &Arc<Mutex<Option<Channel>>>,
    queue: &str,
    retry: &Arc<RetryStrategy>,
    retry_count: u32,
    poison: &Arc<Mutex<HashMap<String, u32>>>,
    stats: &Arc<Mutex<ConsumerStats>>,
    retry_sem: &Arc<Semaphore>,
    message_id: &str,
    err_msg: &str,
) {
    // Poison message tracking.
    {
        let mut pm = poison.lock().await;
        let count = pm.entry(message_id.to_string()).or_insert(0);
        *count += 1;
        if *count >= 10 {
            tracing::error!(
                message_id          = %message_id,
                consecutive_failures = *count,
                "Poison message pattern detected"
            );
        }
    }

    if !is_retryable(err_msg) || !retry.should_retry(retry_count) {
        // Route to DLQ — ACK original to remove from main queue.
        route_to_dlq(ch, queue, delivery, retry_count, retry, err_msg).await;
        let _ = delivery.ack(BasicAckOptions::default()).await;
        stats.lock().await.dlq_routed += 1;
    } else {
        // Schedule retry with exponential backoff (bounded by semaphore).
        schedule_retry(retry, retry_count, ch.clone(), queue.to_string(),
                       delivery.data.clone(), retry_sem.clone());
        let _ = delivery.ack(BasicAckOptions::default()).await;
        stats.lock().await.retried += 1;
    }
    stats.lock().await.failed += 1;
}

/// Extract the x-retry-count header from AMQP message properties.
fn extract_retry_count(delivery: &lapin::message::Delivery) -> u32 {
    delivery
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
        .unwrap_or(0)
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
    let dlq_name = format!("{queue}.dlq");
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

        // Publish to DLQ with publisher confirm.
        if let Ok(confirm) = channel
            .basic_publish(
                "",
                &dlq_name,
                BasicPublishOptions::default(),
                &delivery.data,
                props,
            )
            .await
        {
            if let Err(e) = confirm.await {
                tracing::error!("DLQ publish confirm failed: {}", e);
            }
        }
    }
}

/// Schedule a retry republish with exponential backoff.
///
/// Bounded by a semaphore to prevent unbounded task spawning under sustained failures.
/// Uses `schedule_retry` rather than bare `NACK(requeue=true)` to avoid hot loops.
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
        let _permit = match retry_sem.acquire().await {
            Ok(p) => p,
            Err(_) => {
                tracing::error!("Retry semaphore closed — dropping retry task");
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

            if let Ok(confirm) = channel
                .basic_publish("", &queue, BasicPublishOptions::default(), &data, props)
                .await
            {
                if let Err(e) = confirm.await {
                    tracing::error!("Retry publish confirm failed: {}", e);
                }
            }
        }
    });
}
