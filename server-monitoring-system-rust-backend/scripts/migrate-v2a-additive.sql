-- Migration v2a — Additive only (safe to run BEFORE deploying v2 code)
--
-- This migration adds the new tables and columns without removing anything.
-- Old code continues to write to the original columns during the deploy window.
-- New v2 code writes to the new columns and reads from them.
--
-- Run order: migrate-v2a-additive.sql → deploy code → verify → migrate-v2b-cleanup.sql

-- ── Step 1: Add histogram_profiles table ────────────────────────────────────

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

INSERT INTO histogram_profiles (profile_name, description, bucket_bounds, is_system)
VALUES
(
    'standard',
    '23-bucket general-purpose profile suitable for typical REST APIs',
    ARRAY[5,10,25,50,75,100,150,200,300,400,500,750,
          1000,1500,2000,3000,5000,7500,10000,15000,20000,30000,60000]::double precision[],
    TRUE
),
(
    'latency_focused',
    'High-resolution sub-500ms profile — ideal for user-facing or latency-sensitive APIs',
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

-- ── Step 2: Add client_metric_config table ───────────────────────────────────

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

-- ── Step 3: Add 23 new histogram columns (DEFAULT 0 = metadata-only, instant) ─

ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_5ms     INTEGER NOT NULL DEFAULT 0;
-- latency_le_10ms already exists — skip
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_25ms    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_50ms    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_75ms    INTEGER NOT NULL DEFAULT 0;
-- latency_le_100ms already exists — skip
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_150ms   INTEGER NOT NULL DEFAULT 0;
-- latency_le_200ms already exists — skip
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_300ms   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_400ms   INTEGER NOT NULL DEFAULT 0;
-- latency_le_500ms already exists — skip
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_750ms   INTEGER NOT NULL DEFAULT 0;
-- latency_le_1000ms already exists — skip
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_1500ms  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_2000ms  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_3000ms  INTEGER NOT NULL DEFAULT 0;
-- latency_le_5000ms already exists — skip
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_7500ms  INTEGER NOT NULL DEFAULT 0;
-- latency_le_10000ms already exists — skip
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_15000ms INTEGER NOT NULL DEFAULT 0;
-- latency_le_20000ms already exists — skip (was latency_le_20000ms in v1)
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_30000ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE endpoint_metrics ADD COLUMN IF NOT EXISTS latency_le_60000ms INTEGER NOT NULL DEFAULT 0;

-- ── Step 4: Add new analytics indexes ────────────────────────────────────────
-- NOTE: Run this step OUTSIDE a transaction if your migration runner wraps scripts.
-- The CONCURRENTLY flag requires no wrapping transaction.

CREATE INDEX IF NOT EXISTS idx_em_client_time_desc
    ON endpoint_metrics (client_id, time_bucket DESC);

CREATE INDEX IF NOT EXISTS idx_em_client_service_time_desc
    ON endpoint_metrics (client_id, service_name, time_bucket DESC);
