use rand::Rng;

/// RetryStrategy - Exponential backoff with jitter.
/// Mirrors Node.js RetryStrategy class exactly.
///
/// Formula: min(baseDelay * 2^attempt + jitter, maxDelay)
/// where jitter = random(0..1) * jitterFactor * baseDelay
pub struct RetryStrategy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter_factor: f64,
}

impl RetryStrategy {
    pub fn new(
        max_retries: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
        jitter_factor: f64,
    ) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            max_delay_ms,
            jitter_factor,
        }
    }

    /// Calculate delay for a given attempt number.
    /// Mirrors Node.js delay(attempt).
    pub fn delay(&self, attempt: u32) -> u64 {
        let exp_delay = self.base_delay_ms * 2u64.pow(attempt);
        let mut rng = rand::thread_rng();
        let jitter = (rng.gen::<f64>() * self.jitter_factor * self.base_delay_ms as f64) as u64;
        let total = exp_delay + jitter;
        total.min(self.max_delay_ms)
    }

    /// Check if another retry should be attempted.
    /// Mirrors Node.js shouldRetry(attempt).
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }

    /// Wait for the computed delay (async).
    /// Mirrors Node.js wait(attempt).
    pub async fn wait(&self, attempt: u32) {
        let delay = self.delay(attempt);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
    }
}

/// Check if an error is retryable.
/// Mirrors Node.js isRetryable(error) function.
pub fn is_retryable(error_message: &str) -> bool {
    let retryable_patterns = [
        "channel closed",
        "connection closed",
        "ECONNRESET",
        "ECONNREFUSED",
        "ETIMEDOUT",
        "buffer full",
        "heartbeat timeout",
        "frame_error",
        "Channel ended",
        "connection lost",
        "EPIPE",
        "stream error",
    ];

    let lower = error_message.to_lowercase();
    retryable_patterns
        .iter()
        .any(|p| lower.contains(&p.to_lowercase()))
}
