use rand::Rng;

/// Retry strategy with exponential backoff and jitter.
///
/// Formula: `min(base_delay * 2^attempt + jitter, max_delay)`
/// where `jitter = random(0..1) * jitter_factor * base_delay`.
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

    /// Calculate the delay in milliseconds for a given attempt number.
    pub fn delay(&self, attempt: u32) -> u64 {
        let exp_delay = self.base_delay_ms * 2u64.pow(attempt);
        let mut rng = rand::thread_rng();
        let jitter = (rng.gen::<f64>() * self.jitter_factor * self.base_delay_ms as f64) as u64;
        let total = exp_delay + jitter;
        total.min(self.max_delay_ms)
    }

    /// Check if another retry should be attempted.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }

    /// Asynchronously wait for the computed backoff delay.
    pub async fn wait(&self, attempt: u32) {
        let delay = self.delay(attempt);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
    }
}

/// Known retryable error patterns for messaging systems.
const RETRYABLE_PATTERNS: &[&str] = &[
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

/// Check if an error message matches a known retryable pattern.
pub fn is_retryable(error_message: &str) -> bool {
    let lower = error_message.to_lowercase();
    RETRYABLE_PATTERNS
        .iter()
        .any(|p| lower.contains(&p.to_lowercase()))
}
