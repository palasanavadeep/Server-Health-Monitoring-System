//! SeaORM entity modules.
//!
//! Each module corresponds to one PostgreSQL table.
//! These types are **internal to the repository layer** —
//! services and handlers never import from here directly.

pub mod endpoint_metrics;

pub use endpoint_metrics::Entity as EndpointMetrics;
