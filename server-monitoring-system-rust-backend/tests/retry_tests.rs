use server_monitoring::resilience::retry::{is_retryable, RetryStrategy};

#[test]
fn should_retry_within_max_retries() {
    let strategy = RetryStrategy::new(3, 1000, 30000, 0.3);
    assert!(strategy.should_retry(0));
    assert!(strategy.should_retry(1));
    assert!(strategy.should_retry(2));
    assert!(!strategy.should_retry(3));
    assert!(!strategy.should_retry(10));
}

#[test]
fn delay_increases_exponentially() {
    // Use zero jitter for deterministic testing
    let strategy = RetryStrategy::new(5, 1000, 60000, 0.0);

    let d0 = strategy.delay(0); // 1000 * 2^0 = 1000
    let d1 = strategy.delay(1); // 1000 * 2^1 = 2000
    let d2 = strategy.delay(2); // 1000 * 2^2 = 4000

    assert_eq!(d0, 1000);
    assert_eq!(d1, 2000);
    assert_eq!(d2, 4000);
}

#[test]
fn delay_capped_at_max() {
    let strategy = RetryStrategy::new(10, 1000, 5000, 0.0);

    let d5 = strategy.delay(5); // 1000 * 2^5 = 32000 -> capped at 5000
    assert_eq!(d5, 5000);
}

#[test]
fn delay_includes_jitter() {
    let strategy = RetryStrategy::new(5, 1000, 60000, 0.5);

    // With jitter, delay should be >= base and <= base + jitter_amount
    let d0 = strategy.delay(0);
    // Base: 1000, jitter: up to 0.5 * 1000 = 500
    assert!(d0 >= 1000);
    assert!(d0 <= 1500);
}

#[test]
fn is_retryable_matches_known_patterns() {
    assert!(is_retryable("connection closed by peer"));
    assert!(is_retryable("ECONNRESET"));
    assert!(is_retryable("Channel ended unexpectedly"));
    assert!(is_retryable("heartbeat timeout detected"));
    assert!(is_retryable("ETIMEDOUT after 30s"));
}

#[test]
fn is_retryable_case_insensitive() {
    assert!(is_retryable("CHANNEL CLOSED"));
    assert!(is_retryable("Connection Lost"));
}

#[test]
fn is_retryable_rejects_non_retryable() {
    assert!(!is_retryable("invalid JSON payload"));
    assert!(!is_retryable("authentication failed"));
    assert!(!is_retryable("not found"));
}
