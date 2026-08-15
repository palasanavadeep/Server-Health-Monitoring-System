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
