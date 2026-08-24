use server_monitoring::resilience::circuit_breaker::{CircuitBreaker, CircuitState};

#[test]
fn starts_closed() {
    let cb = CircuitBreaker::new(3, 5000, 2);
    assert_eq!(cb.get_state(), CircuitState::Closed);
}

#[test]
fn allows_requests_when_closed() {
    let cb = CircuitBreaker::new(3, 5000, 2);
    assert!(cb.allow_request());
}

#[test]
fn opens_after_threshold_failures() {
    let cb = CircuitBreaker::new(3, 5000, 2);

    cb.on_failure();
    assert_eq!(cb.get_state(), CircuitState::Closed);

    cb.on_failure();
    assert_eq!(cb.get_state(), CircuitState::Closed);

    cb.on_failure();
    assert_eq!(cb.get_state(), CircuitState::Open);
    assert!(!cb.allow_request());
}

#[test]
fn success_resets_failure_count() {
    let cb = CircuitBreaker::new(3, 5000, 2);

    cb.on_failure();
    cb.on_failure();
    cb.on_success(); // resets
    cb.on_failure();
    cb.on_failure();
    // Only 2 consecutive after reset, not 3 — should still be closed
    assert_eq!(cb.get_state(), CircuitState::Closed);
}

#[test]
fn transitions_to_half_open_after_cooldown() {
    // Use a tiny cooldown for testing
    let cb = CircuitBreaker::new(2, 1, 1); // 1ms cooldown

    cb.on_failure();
    cb.on_failure();
    assert_eq!(cb.get_state(), CircuitState::Open);

    // Sleep past the cooldown
    std::thread::sleep(std::time::Duration::from_millis(10));

    // allow_request should transition to HalfOpen
    assert!(cb.allow_request());
    assert_eq!(cb.get_state(), CircuitState::HalfOpen);
}

#[test]
fn closes_after_half_open_successes() {
    let cb = CircuitBreaker::new(2, 1, 2); // need 2 successes in half-open

    // Open the circuit
    cb.on_failure();
    cb.on_failure();
    assert_eq!(cb.get_state(), CircuitState::Open);

    // Wait for cooldown
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(cb.allow_request()); // transitions to HalfOpen

    cb.on_success();
    assert_eq!(cb.get_state(), CircuitState::HalfOpen);

    cb.on_success();
    assert_eq!(cb.get_state(), CircuitState::Closed);
}

#[test]
fn failure_in_half_open_reopens() {
    let cb = CircuitBreaker::new(2, 1, 2);

    // Open the circuit
    cb.on_failure();
    cb.on_failure();

    // Wait and transition to half-open
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(cb.allow_request());
    assert_eq!(cb.get_state(), CircuitState::HalfOpen);

    // Fail during half-open
    cb.on_failure();
    assert_eq!(cb.get_state(), CircuitState::Open);
}

#[test]
fn get_stats_returns_correct_counts() {
    let cb = CircuitBreaker::new(5, 5000, 2);

    cb.on_failure();
    cb.on_failure();

    let (state, failures, successes) = cb.get_stats();
    assert_eq!(state, CircuitState::Closed);
    assert_eq!(failures, 2);
    assert_eq!(successes, 0);
}
