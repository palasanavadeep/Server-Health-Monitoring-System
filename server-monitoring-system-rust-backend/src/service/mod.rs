//! Service layer — pure business logic, no HTTP types.
//!
//! Services encapsulate domain operations and orchestrate repository calls.
//! They receive trait-object repositories through dependency injection.

pub mod analytics;
pub mod auth;
pub mod client;
pub mod ingest;
pub mod metrics_processor;
pub mod processor;
