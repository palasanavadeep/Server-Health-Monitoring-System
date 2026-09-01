use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions},
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

use crate::domain::ingest::MetricsEvent;
use crate::resilience::circuit_breaker::CircuitBreaker;
use crate::resilience::retry::{is_retryable, RetryStrategy};
use crate::service::metrics_processor::MetricsProcessorService;

/// Metrics consumer — reads from `server_hits.metrics`, writes to PostgreSQL.
///
/// ## Single responsibility
///
/// This consumer knows only about PostgreSQL analytics. It has no knowledge of
/// MongoDB or the upstream persistence pipeline.
///
/// ## Idempotency (two-layer)
///
/// Layer 1 — LruCache fast path:
///   Populated ONLY after a successful PostgreSQL commit. A cache hit means the
///   event was already committed — safe to ACK immediately without a DB roundtrip.
///   The cache is bounded and will evict oldest entries; this layer is a
///   **performance optimization only**, not a correctness guarantee.
///
/// Layer 2 — PostgreSQL dedup table (source of truth):
///   `process_metrics()` delegates to an atomic transaction:
///     INSERT processed_metric_events ON CONFLICT DO NOTHING
///     + UPSERT endpoint_metrics (only if new event_id)
///   This is the final safety net — survives worker restarts, cache eviction,
///   and any scenario where the LruCache cannot help.
///
/// ## Failure model
///
/// - PG fails → NACK + exponential backoff retry (no hot loops)
/// - PG succeeds → ACK fails → redelivery → `processed_metric_events` dedup → skip
/// - Worker restart → LruCache empty → DB dedup catches duplicates
pub struct MetricsConsumer {
    metrics_processor: Arc<MetricsProcessorService>,
    channel: Arc<Mutex<Option<Channel>>>,
    queue_name: String,
    retry_strategy: Arc<RetryStrategy>,
    circuit_breaker: Arc<CircuitBreaker>,
    /// LruCache of committed event_ids — fast path for recent duplicates.
    /// Populated AFTER successful PostgreSQL commit, not before.
    processed_ids: Arc<Mutex<LruCache<String, ()>>>,
    poison_messages: Arc<Mutex<HashMap<String, u32>>>,
    stats: Arc<Mutex<MetricsConsumerStats>>,
    cancel_token: CancellationToken,
    retry_semaphore: Arc<Semaphore>,
}

/// Runtime statistics for the metrics consumer.
#[derive(Default)]
pub struct MetricsConsumerStats {
    pub processed: u64,
    pub skipped_duplicates: u64,
    pub failed: u64,
    pub retried: u64,
    pub dlq_routed: u64,
}

impl MetricsConsumer {
    pub fn new(
        metrics_processor: Arc<MetricsProcessorService>,
        channel: Channel,
        queue_name: String,
        retry_strategy: Arc<RetryStrategy>,
        circuit_breaker: Arc<CircuitBreaker>,
        idempotency_cache_size: usize,
    ) -> Self {
        let cache_cap =
            NonZeroUsize::new(idempotency_cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());

        Self {
            metrics_processor,
            channel: Arc::new(Mutex::new(Some(channel))),
            queue_name,
            retry_strategy,
            circuit_breaker,
            processed_ids: Arc::new(Mutex::new(LruCache::new(cache_cap))),
            poison_messages: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(MetricsConsumerStats::default())),
            cancel_token: CancellationToken::new(),
            retry_semaphore: Arc::new(Semaphore::new(50)),
        }
    }

    /// Start consuming from the metrics queue.
    ///
    /// Returns a `JoinHandle` that should be awaited on shutdown.
    pub async fn start(
        &self,
    ) -> Result<JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
        let channel_guard = self.channel.lock().await;
        let channel = channel_guard.as_ref().ok_or("No channel available")?;

        channel.basic_qos(10, Default::default()).await?;

        let consumer = channel
            .basic_consume(
                &self.queue_name,
                &format!("metrics-consumer-{}", chrono::Utc::now().timestamp()),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        tracing::info!(queue = %self.queue_name, "Metrics consumer started");

        let processor = self.metrics_processor.clone();
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
                        tracing::info!("Metrics consumer received cancellation signal");
                        break;
                    }
                    maybe_delivery = consumer.next() => {
                        match maybe_delivery {
                            Some(Ok(delivery)) => {
                                handle_metrics_delivery(
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
                                )
                                .await;
                            }
                            Some(Err(e)) => {
                                tracing::error!("Metrics consumer stream error: {}", e);
                                break;
                            }
                            None => {
                                tracing::info!("Metrics consumer stream ended");
                                break;
                            }
                        }
                    }
                }
            }

            tracing::info!("Metrics consumer loop ended");
        });

        Ok(handle)
    }

    /// Signal cooperative shutdown.
    pub fn stop(&self) {
        self.cancel_token.cancel();
        tracing::info!("Metrics consumer stop requested");
    }

    /// Get current consumer statistics.
    pub async fn get_stats(&self) -> (u64, u64, u64, u64, u64) {
        let s = self.stats.lock().await;
        (s.processed, s.skipped_duplicates, s.failed, s.retried, s.dlq_routed)
    }
}

// ── Message handler ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn handle_metrics_delivery(
    delivery: &lapin::message::Delivery,
    processor: &Arc<MetricsProcessorService>,
    cb: &Arc<CircuitBreaker>,
    retry: &Arc<RetryStrategy>,
    ids: &Arc<Mutex<LruCache<String, ()>>>,
    poison: &Arc<Mutex<HashMap<String, u32>>>,
    stats: &Arc<Mutex<MetricsConsumerStats>>,
    ch: &Arc<Mutex<Option<Channel>>>,
    queue: &str,
    retry_sem: &Arc<Semaphore>,
) {
    // Circuit breaker gate.
    if !cb.allow_request() {
        tracing::warn!("Circuit breaker open — requeuing metrics message");
        let _ = delivery
            .nack(lapin::options::BasicNackOptions {
                requeue: true,
                ..Default::default()
            })
            .await;
        return;
    }

    // Deserialize MetricsEvent.
    let metrics_event: MetricsEvent = match serde_json::from_slice(&delivery.data) {
        Ok(e) => e,
        Err(err) => {
            tracing::error!(
                error = %err,
                raw   = %String::from_utf8_lossy(&delivery.data),
                "Failed to deserialize MetricsEvent — discarding (unrecoverable)"
            );
            let _ = delivery.ack(BasicAckOptions::default()).await;
            return;
        }
    };

    let event_id = metrics_event.event_id.clone();

    // ── Layer 1: LruCache fast path ────────────────────────────────────────────
    // Only populated after successful PG commit — a cache hit guarantees the
    // event was already committed and is safe to skip.
    {
        let mut ids_guard = ids.lock().await;
        if ids_guard.get(&event_id).is_some() {
            tracing::debug!(event_id = %event_id, "LruCache hit — skipping metrics duplicate");
            let _ = delivery.ack(BasicAckOptions::default()).await;
            stats.lock().await.skipped_duplicates += 1;
            return;
        }
    }

    let retry_count = extract_retry_count(delivery);

    // ── Layer 2: process through PostgreSQL transaction ─────────────────────────
    match processor.process_metrics(metrics_event).await {
        Ok(true) => {
            // New event: PG transaction committed successfully.
            // Insert into LruCache AFTER commit — not before.
            {
                let mut id_set = ids.lock().await;
                id_set.put(event_id.clone(), ());
            }
            let _ = delivery.ack(BasicAckOptions::default()).await;
            cb.on_success();
            poison.lock().await.remove(&event_id);
            stats.lock().await.processed += 1;
            tracing::debug!(event_id = %event_id, "Metrics event committed");
        }
        Ok(false) => {
            // Duplicate event_id — already in processed_metric_events.
            // ACK safely: no metrics were changed, no double-count.
            let _ = delivery.ack(BasicAckOptions::default()).await;
            cb.on_success();
            // Also populate LruCache to speed up future duplicates.
            {
                let mut id_set = ids.lock().await;
                id_set.put(event_id.clone(), ());
            }
            stats.lock().await.skipped_duplicates += 1;
            tracing::debug!(event_id = %event_id, "Duplicate metrics event — DB dedup");
        }
        Err(e) => {
            cb.on_failure();
            let err_msg = format!("{e}");
            tracing::error!(
                event_id    = %event_id,
                retry_count = retry_count,
                error       = %err_msg,
                "PostgreSQL metrics processing failed"
            );

            // Poison message tracking.
            {
                let mut pm = poison.lock().await;
                let count = pm.entry(event_id.clone()).or_insert(0);
                *count += 1;
                if *count >= 10 {
                    tracing::error!(
                        event_id             = %event_id,
                        consecutive_failures = *count,
                        "Poison metrics message pattern detected"
                    );
                }
            }

            if !is_retryable(&err_msg) || !retry.should_retry(retry_count) {
                route_to_dlq(ch, queue, delivery, retry_count, retry, &err_msg).await;
                let _ = delivery.ack(BasicAckOptions::default()).await;
                stats.lock().await.dlq_routed += 1;
            } else {
                schedule_retry(
                    retry, retry_count, ch.clone(),
                    queue.to_string(), delivery.data.clone(), retry_sem.clone(),
                );
                let _ = delivery.ack(BasicAckOptions::default()).await;
                stats.lock().await.retried += 1;
            }
            stats.lock().await.failed += 1;
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

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

        if let Ok(confirm) = channel
            .basic_publish("", &dlq_name, BasicPublishOptions::default(), &delivery.data, props)
            .await
        {
            if let Err(e) = confirm.await {
                tracing::error!("Metrics DLQ publish confirm failed: {}", e);
            }
        }
    }
}

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
                tracing::error!("Metrics retry semaphore closed — dropping retry task");
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
                    tracing::error!("Metrics retry publish confirm failed: {}", e);
                }
            }
        }
    });
}
