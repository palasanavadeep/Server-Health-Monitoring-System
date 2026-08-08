//! Repository layer — data access abstractions and implementations.
//!
//! Each repository defines a trait (interface) for testability and polymorphism,
//! with a concrete MongoDB or PostgreSQL implementation.

pub mod api_hit_repo;
pub mod api_key_repo;
pub mod client_repo;
pub mod metrics_repo;
pub mod user_repo;
