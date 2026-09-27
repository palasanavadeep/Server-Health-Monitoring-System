-- PostgreSQL initialization script (v2 — multi-tenant, 23-bucket histogram)
-- Run on first container startup or against a fresh database.
--
-- Column naming conventions:
--   latency_le_Xms  — cumulative histogram bucket: count of requests where latency ≤ X ms
--   status_Xxx      — HTTP response status class counter (1xx … 5xx)
--   Apdex is derived at read time from the histogram; it is NOT stored here.

-- ── histogram_profiles ───────────────────────────────────────────────────────
--
-- Named bucket-boundary sets used to customise per-tenant percentile display.
-- Storage always uses the full 23 columns; profiles are a read-time lens.
-- System profiles (is_system = TRUE) are read-only via the API.

CREATE TABLE IF NOT EXISTS histogram_profiles (
    profile_name   VARCHAR(64)        NOT NULL PRIMARY KEY,
    description    TEXT               NOT NULL,
    -- All bounds must be a strict subset of the 23 canonical boundaries below.
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

-- ── client_metric_config ──────────────────────────────────────────────────────
--
-- Per-tenant metric configuration. One row per client.
-- Automatically created with defaults when a client is onboarded.
-- Controls: Apdex T threshold, histogram display profile, retention, quota.

CREATE TABLE IF NOT EXISTS client_metric_config (
    client_id            VARCHAR(64)      NOT NULL PRIMARY KEY,
    -- Apdex T threshold in milliseconds. Requests ≤ T are Satisfied.
    apdex_threshold_ms   DOUBLE PRECISION NOT NULL DEFAULT 500.0,
    -- Named histogram profile from histogram_profiles.
    histogram_profile    VARCHAR(64)      NOT NULL DEFAULT 'standard'
                         REFERENCES histogram_profiles (profile_name),
    -- Metric rows older than this are deleted by the nightly retention job.
    data_retention_days  INTEGER          NOT NULL DEFAULT 90,
    -- Daily ingest quota (NULL = unlimited). Soft limit — see architecture docs.
    daily_ingest_quota   BIGINT,
    created_at           TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ── endpoint_metrics ─────────────────────────────────────────────────────────
--
-- One row per (client_id, service_name, endpoint, method, time_bucket) tuple.
-- time_bucket is truncated to the start of the clock-hour.
-- All numeric columns are additive counters or running aggregates.
-- Apdex and percentiles are computed at read time — NOT stored here.

CREATE TABLE IF NOT EXISTS endpoint_metrics (
    id            BIGSERIAL        PRIMARY KEY,
    client_id     VARCHAR(64)      NOT NULL,
    service_name  VARCHAR(255)     NOT NULL,
    endpoint      VARCHAR(512)     NOT NULL,
    method        VARCHAR(16)      NOT NULL,

    -- Core throughput counters
    total_hits    INTEGER          NOT NULL DEFAULT 0,
    error_hits    INTEGER          NOT NULL DEFAULT 0,

    -- Running latency statistics (Welford-style running average)
    avg_latency   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    min_latency   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    max_latency   DOUBLE PRECISION NOT NULL DEFAULT 0.0,

    -- Cumulative latency histogram (Prometheus-style, upper-bound inclusive).
    --
    -- For each request, every bucket i where latency_ms <= upper_bound[i] is
    -- incremented by 1. This makes increments additive and idempotent under
    -- deduplication. Percentiles and Apdex are derived at query time.
    --
    -- +inf bucket = total_hits (implicit, no separate column needed).
    latency_le_5ms      INTEGER          NOT NULL DEFAULT 0,
    latency_le_10ms     INTEGER          NOT NULL DEFAULT 0,
    latency_le_25ms     INTEGER          NOT NULL DEFAULT 0,
    latency_le_50ms     INTEGER          NOT NULL DEFAULT 0,
    latency_le_75ms     INTEGER          NOT NULL DEFAULT 0,
    latency_le_100ms    INTEGER          NOT NULL DEFAULT 0,
    latency_le_150ms    INTEGER          NOT NULL DEFAULT 0,
    latency_le_200ms    INTEGER          NOT NULL DEFAULT 0,
    latency_le_300ms    INTEGER          NOT NULL DEFAULT 0,
    latency_le_400ms    INTEGER          NOT NULL DEFAULT 0,
    latency_le_500ms    INTEGER          NOT NULL DEFAULT 0,
    latency_le_750ms    INTEGER          NOT NULL DEFAULT 0,
    latency_le_1000ms   INTEGER          NOT NULL DEFAULT 0,
    latency_le_1500ms   INTEGER          NOT NULL DEFAULT 0,
    latency_le_2000ms   INTEGER          NOT NULL DEFAULT 0,
    latency_le_3000ms   INTEGER          NOT NULL DEFAULT 0,
    latency_le_5000ms   INTEGER          NOT NULL DEFAULT 0,
    latency_le_7500ms   INTEGER          NOT NULL DEFAULT 0,
    latency_le_10000ms  INTEGER          NOT NULL DEFAULT 0,
    latency_le_15000ms  INTEGER          NOT NULL DEFAULT 0,
    latency_le_20000ms  INTEGER          NOT NULL DEFAULT 0,
    latency_le_30000ms  INTEGER          NOT NULL DEFAULT 0,
    latency_le_60000ms  INTEGER          NOT NULL DEFAULT 0,

    -- HTTP response status class distribution
    status_1xx    INTEGER          NOT NULL DEFAULT 0,
    status_2xx    INTEGER          NOT NULL DEFAULT 0,
    status_3xx    INTEGER          NOT NULL DEFAULT 0,
    status_4xx    INTEGER          NOT NULL DEFAULT 0,
    status_5xx    INTEGER          NOT NULL DEFAULT 0,

    time_bucket   TIMESTAMP        NOT NULL,
    created_at    TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT uq_endpoint_metrics
        UNIQUE (client_id, service_name, endpoint, method, time_bucket)
);

-- Query indexes for the most common analytics access patterns
CREATE INDEX IF NOT EXISTS idx_em_client_time_desc
    ON endpoint_metrics (client_id, time_bucket DESC);

CREATE INDEX IF NOT EXISTS idx_em_client_service_time_desc
    ON endpoint_metrics (client_id, service_name, time_bucket DESC);

CREATE INDEX IF NOT EXISTS idx_em_service_endpoint
    ON endpoint_metrics (service_name, endpoint, method);

-- ── processed_metric_events ───────────────────────────────────────────────────
--
-- Deduplication log for the Metrics Worker.
-- Every processed event_id is recorded. On redelivery the INSERT returns
-- rows_affected = 0 and the upsert is skipped — exactly-once effect on top
-- of at-least-once RabbitMQ delivery.
--
-- Retention: configurable via METRIC_EVENT_DEDUP_RETENTION_DAYS (default 90).
-- Must exceed the maximum possible RabbitMQ message replay window.

CREATE TABLE IF NOT EXISTS processed_metric_events (
    event_id     VARCHAR(128) PRIMARY KEY,
    processed_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_pme_processed_at
    ON processed_metric_events (processed_at);
