-- PostgreSQL initialization script
-- Run automatically by the postgres container on first startup

-- Create the endpoint_metrics table for aggregated API hit metrics
CREATE TABLE IF NOT EXISTS endpoint_metrics (
    id              BIGSERIAL PRIMARY KEY,
    client_id       VARCHAR(64)     NOT NULL,
    service_name    VARCHAR(255)    NOT NULL,
    endpoint        VARCHAR(512)    NOT NULL,
    method          VARCHAR(16)     NOT NULL,
    total_hits      INTEGER         NOT NULL DEFAULT 0,
    error_hits      INTEGER         NOT NULL DEFAULT 0,
    avg_latency     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    min_latency     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    max_latency     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    time_bucket     TIMESTAMP       NOT NULL,
    created_at      TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT uq_endpoint_metrics
        UNIQUE (client_id, service_name, endpoint, method, time_bucket)
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_em_client_id
    ON endpoint_metrics (client_id);

CREATE INDEX IF NOT EXISTS idx_em_time_bucket
    ON endpoint_metrics (time_bucket);

CREATE INDEX IF NOT EXISTS idx_em_client_time
    ON endpoint_metrics (client_id, time_bucket);

CREATE INDEX IF NOT EXISTS idx_em_service_endpoint
    ON endpoint_metrics (service_name, endpoint, method);

-- ── Deduplication table ───────────────────────────────────────────────────────
--
-- Records every event_id processed by the metrics worker to prevent
-- double-counting under at-least-once RabbitMQ redelivery.
--
-- RETENTION POLICY: Rows are retained for 30 days. This must exceed the
-- maximum possible RabbitMQ message replay window. If old messages can be
-- replayed after 30 days, increase the retention period accordingly.
-- Cleanup is performed by the periodic cleanup job in MetricsProcessorService.
CREATE TABLE IF NOT EXISTS processed_metric_events (
    event_id     VARCHAR(128) PRIMARY KEY,
    processed_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient time-based cleanup queries
CREATE INDEX IF NOT EXISTS idx_pme_processed_at
    ON processed_metric_events (processed_at);

