//! Per-client daily ingest quota tracker.
//!
//! ## Design
//! Tracks how many events each client has ingested today using in-process
//! atomic counters. No database or Redis dependency.
//!
//! ## Known trade-offs (documented, accepted)
//! - **Multi-instance:** Each instance tracks independently. Across N instances
//!   a client can ingest up to N × `daily_quota` events before any single
//!   instance rejects them. This is a soft limit — appropriate for rate-limiting
//!   abusive clients, not for billing enforcement.
//! - **Restart loss:** Counters reset on process restart. The process effectively
//!   receives a fresh quota budget for the remainder of the day.
//! - **Trigger for upgrade:** If exact cross-instance quota is required (e.g.
//!   billing), replace with `INCR`/`EXPIRE` in Redis or a PostgreSQL counter
//!   table with atomic `UPDATE ... RETURNING`.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use chrono::Utc;
use dashmap::DashMap;

// ── Result type ───────────────────────────────────────────────────────────────

/// The result of a quota check-and-increment operation.
#[derive(Debug, Clone)]
pub enum QuotaCheckResult {
    /// Client has no configured quota — all requests are allowed.
    Unlimited,
    /// Quota not yet reached. Contains the counter value after this increment.
    Allowed { count: i64 },
    /// Quota exceeded. Contains the count and the configured limit.
    Exceeded { count: i64, limit: i64 },
}

impl QuotaCheckResult {
    pub fn is_allowed(&self) -> bool {
        !matches!(self, QuotaCheckResult::Exceeded { .. })
    }
}

// ── Tracker ───────────────────────────────────────────────────────────────────

/// In-process per-client daily ingest quota tracker.
///
/// Wrap in `Arc` for cheap cloning across request handlers.
pub struct IngestQuotaTracker {
    /// key: `"{client_id}:{YYYY-MM-DD}"` (UTC date)
    counters: DashMap<String, Arc<AtomicI64>>,
}

impl IngestQuotaTracker {
    pub fn new() -> Self {
        Self { counters: DashMap::new() }
    }

    /// Atomically increment the client's daily counter by `increment` and check
    /// whether the configured `quota` is exceeded.
    ///
    /// For single-event ingest, pass `increment = 1`.
    /// For batch ingest, pass `increment = events.len()` (checked upfront,
    /// before any events are processed).
    ///
    /// O(1) — no locks, no I/O.
    pub fn check_and_increment(
        &self,
        client_id:  &str,
        quota:      Option<i64>,
        increment:  i64,
    ) -> QuotaCheckResult {
        let Some(limit) = quota else {
            return QuotaCheckResult::Unlimited;
        };

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let key   = format!("{client_id}:{today}");

        let count = self.counters
            .entry(key)
            .or_insert_with(|| Arc::new(AtomicI64::new(0)))
            .fetch_add(increment, Ordering::Relaxed)
            + increment;

        if count > limit {
            QuotaCheckResult::Exceeded { count, limit }
        } else {
            QuotaCheckResult::Allowed { count }
        }
    }

    /// Remove stale entries from previous days.
    ///
    /// Call this from a background task every hour. Entries whose key does not
    /// end with today's UTC date are discarded, freeing memory.
    pub fn prune_stale_entries(&self) {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        self.counters.retain(|key, _| key.ends_with(&today));
    }
}

impl Default for IngestQuotaTracker {
    fn default() -> Self {
        Self::new()
    }
}
