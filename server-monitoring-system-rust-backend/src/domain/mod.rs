//! Domain layer — pure business entities, value objects, and enums.
//!
//! This module contains the core data structures of the application.
//! Domain types should have minimal framework dependencies (only serde/bson for persistence).
//! They are shared across all layers (service, repository, handler).

pub mod api_hit;
pub mod api_key;
pub mod client;
pub mod event;
pub mod ingest;
pub mod metrics;
pub mod role;
pub mod user;
