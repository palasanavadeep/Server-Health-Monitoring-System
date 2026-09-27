//! In-process TTL caches and quota tracker.
//!
//! All caches use `moka::future::Cache` which provides:
//! - Async-aware eviction
//! - Built-in single-flight via `try_get_with` (concurrent misses collapse
//!   into a single loader call, preventing thundering-herd on TTL expiry)
//!
//! The quota tracker uses `dashmap` for O(1) lock-free atomic increments.

pub mod api_key_cache;
pub mod ingest_quota_tracker;
pub mod tenant_config_cache;
