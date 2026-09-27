-- ============================================================================
-- Migration: v2-full — Upgrade from base schema to v2 multi-tenant + histogram
-- ============================================================================
--
-- Your current schema has:
--   endpoint_metrics  — basic columns only (no histogram, no status, no apdex)
--   processed_metric_events — unchanged
--   NO: histogram_profiles, client_metric_config
--
-- This single script brings the database fully up to the v2 state.
-- Safe to run with the server stopped, or while no workers are writing.
--
-- Run it as:
--   docker exec -i server-monitoring-postgres \
--     psql -U postgres -d server_monitoring < scripts/migrate-v2-full.sql
-- ============================================================================

BEGIN;

-- ── 1. histogram_profiles ────────────────────────────────────────────────────
--  Named bucket-boundary sets used as per-tenant read-time lenses.
--  The CHECK constraint enforces every bound is one of the 23 canonical values.

CREATE TABLE IF NOT EXISTS histogram_profiles (
    profile_name   VARCHAR(64)        NOT NULL PRIMARY KEY,
    description    TEXT               NOT NULL,
    bucket_bounds  DOUBLE PRECISION[] NOT NULL,
    is_system      BOOLEAN            NOT NULL DEFAULT TRUE,
    created_at     TIMESTAMP          NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT chk_bucket_bounds_subset CHECK (
        bucket_bounds <@ ARRAY[
            5,10,25,50,75,100,150,200,300,400,500,750,
            1000,1500,2000,3000,5000,7500,10000,15000,20000,30000,60000
        ]::double precision[]
    )
);

INSERT INTO histogram_profiles (profile_name, description, bucket_bounds, is_system) VALUES
(
    'standard',
    '23-bucket general-purpose profile suitable for typical REST APIs',
    ARRAY[5,10,25,50,75,100,150,200,300,400,500,750,
          1000,1500,2000,3000,5000,7500,10000,15000,20000,30000,60000]::double precision[],
    TRUE
),
(
    'latency_focused',
    'High-resolution sub-500ms profile for user-facing or latency-sensitive APIs',
    ARRAY[5,10,25,50,75,100,150,200,300,400,500,750,1000,2000,5000,20000]::double precision[],
    TRUE
),
(
    'long_running',
    'Optimised for batch jobs or APIs with expected latency of 1–60 seconds',
    ARRAY[100,200,500,1000,1500,2000,3000,5000,7500,10000,15000,20000,30000,60000]::double precision[],
    TRUE
)
ON CONFLICT (profile_name) DO NOTHING;

-- ── 2. client_metric_config ──────────────────────────────────────────────────
--  Per-tenant metric settings. Defaults apply if no row exists.

CREATE TABLE IF NOT EXISTS client_metric_config (
    client_id            VARCHAR(64)      NOT NULL PRIMARY KEY,
    apdex_threshold_ms   DOUBLE PRECISION NOT NULL DEFAULT 500.0,
    histogram_profile    VARCHAR(64)      NOT NULL DEFAULT 'standard'
                         REFERENCES histogram_profiles (profile_name),
    data_retention_days  INTEGER          NOT NULL DEFAULT 90,
    daily_ingest_quota   BIGINT,
    created_at           TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ── 3. Add 23 cumulative histogram columns to endpoint_metrics ───────────────
--  Each column counts requests where latency ≤ the stated bound.
--  DEFAULT 0 means this is a metadata-only change — instant on any table size.
--  Existing rows are correct: all buckets = 0 for historical rows (no data loss).

ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_5ms     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_10ms    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_25ms    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_50ms    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_75ms    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_100ms   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_150ms   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_200ms   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_300ms   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_400ms   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_500ms   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_750ms   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_1000ms  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_1500ms  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_2000ms  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_3000ms  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_5000ms  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_7500ms  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_10000ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_15000ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_20000ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_30000ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_60000ms INTEGER NOT NULL DEFAULT 0;

-- ── 4. Add HTTP status class counters ────────────────────────────────────────
--  Used for status distribution (1xx/2xx/3xx/4xx/5xx) in analytics responses.

ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS status_1xx INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS status_2xx INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS status_3xx INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS status_4xx INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS status_5xx INTEGER NOT NULL DEFAULT 0;

-- ── 5. Add basic indexes for common query patterns ───────────────────────────

CREATE INDEX IF NOT EXISTS idx_em_client_time_desc
    ON endpoint_metrics (client_id, time_bucket DESC);

CREATE INDEX IF NOT EXISTS idx_em_client_service_time_desc
    ON endpoint_metrics (client_id, service_name, time_bucket DESC);

COMMIT;

-- ── 6. Verify ────────────────────────────────────────────────────────────────
--  After the transaction commits, this shows what you now have.

SELECT
    column_name,
    data_type,
    column_default
FROM information_schema.columns
WHERE table_name = 'endpoint_metrics'
  AND table_schema = 'public'
ORDER BY ordinal_position;
