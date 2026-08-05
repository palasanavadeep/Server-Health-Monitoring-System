use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// CircuitBreaker states mirroring Node.js CircuitBreaker class exactly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// CircuitBreaker - Identical logic to Node.js implementation.
/// States: CLOSED → OPEN → HALF_OPEN → CLOSED
pub struct CircuitBreaker {
    state: std::sync::Mutex<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    failure_threshold: u32,
    cooldown_ms: u64,
    half_open_max_attempts: u32,
    last_failure_time: AtomicU64,
}

impl CircuitBreaker {
    pub fn new(
        failure_threshold: u32,
        cooldown_ms: u64,
        half_open_max_attempts: u32,
    ) -> Self {
        Self {
            state: std::sync::Mutex::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            failure_threshold,
            cooldown_ms,
            half_open_max_attempts,
            last_failure_time: AtomicU64::new(0),
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Check if a request should be allowed through.
    /// Mirrors Node.js allowRequest().
    pub fn allow_request(&self) -> bool {
        let mut state = self.state.lock().unwrap();

        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let now = Self::now_ms();
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);

                if now - last_failure >= self.cooldown_ms {
                    // Transition to half-open
                    *state = CircuitState::HalfOpen;
                    self.success_count.store(0, Ordering::Relaxed);
                    tracing::info!("Circuit breaker: OPEN -> HALF_OPEN");
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                let success = self.success_count.load(Ordering::Relaxed);
                success < self.half_open_max_attempts
            }
        }
    }

    /// Record a successful operation.
    /// Mirrors Node.js onSuccess().
    pub fn on_success(&self) {
        let mut state = self.state.lock().unwrap();

        match *state {
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.half_open_max_attempts {
                    *state = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    tracing::info!("Circuit breaker: HALF_OPEN -> CLOSED");
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// Record a failed operation.
    /// Mirrors Node.js onFailure().
    pub fn on_failure(&self) {
        let mut state = self.state.lock().unwrap();
        self.last_failure_time
            .store(Self::now_ms(), Ordering::Relaxed);

        match *state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.failure_threshold {
                    *state = CircuitState::Open;
                    tracing::warn!(
                        "Circuit breaker: CLOSED -> OPEN (failures: {})",
                        count
                    );
                }
            }
            CircuitState::HalfOpen => {
                *state = CircuitState::Open;
                self.success_count.store(0, Ordering::Relaxed);
                tracing::warn!("Circuit breaker: HALF_OPEN -> OPEN");
            }
            CircuitState::Open => {
                // Already open, just update last failure time
            }
        }
    }

    /// Get current state.
    pub fn get_state(&self) -> CircuitState {
        *self.state.lock().unwrap()
    }

    /// Get stats.
    pub fn get_stats(&self) -> (CircuitState, u32, u32) {
        let state = *self.state.lock().unwrap();
        let failures = self.failure_count.load(Ordering::Relaxed);
        let successes = self.success_count.load(Ordering::Relaxed);
        (state, failures, successes)
    }
}
