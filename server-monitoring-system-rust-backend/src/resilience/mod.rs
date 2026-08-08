//! Resilience patterns — circuit breaker, retry strategy, and backoff.
//!
//! These are general-purpose primitives used by both the message producer
//! and consumer. They are decoupled from any specific messaging technology.

pub mod circuit_breaker;
pub mod retry;
