//! SeaORM entity for the `endpoint_metrics` PostgreSQL table.
//!
//! This table stores aggregated per-endpoint, per-time-bucket API hit metrics.
//! Rows are upserted (ON CONFLICT DO UPDATE) by the consumer on every processed event.
//!
//! **Do not import this module above the repository layer.**

use sea_orm::entity::prelude::*;

/// SeaORM model — mirrors `endpoint_metrics` table schema exactly.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "endpoint_metrics")]
pub struct Model {
    /// Auto-increment primary key.
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,

    /// Client organisation identifier (MongoDB ObjectId as hex string).
    pub client_id: String,

    /// Name of the monitored service (e.g., `"payment-service"`).
    pub service_name: String,

    /// HTTP path or route pattern (e.g., `"/api/v1/orders"`).
    pub endpoint: String,

    /// HTTP method in uppercase (e.g., `"GET"`, `"POST"`).
    pub method: String,

    /// Total request count in this bucket.
    pub total_hits: i32,

    /// Count of requests with status ≥ 400.
    pub error_hits: i32,

    /// Weighted average response latency (ms).
    pub avg_latency: f64,

    /// Minimum response latency observed in this bucket (ms).
    pub min_latency: f64,

    /// Maximum response latency observed in this bucket (ms).
    pub max_latency: f64,

    /// Start of the 1-hour time bucket this row covers.
    pub time_bucket: DateTimeUtc,

    /// Row creation timestamp (set by DB default).
    pub created_at: DateTimeUtc,

    /// Row last-updated timestamp (updated on upsert).
    pub updated_at: DateTimeUtc,
}

/// No relations — this is a standalone metrics table.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
