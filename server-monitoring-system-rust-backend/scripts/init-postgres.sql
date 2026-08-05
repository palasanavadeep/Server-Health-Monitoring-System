-- PostgreSQL initialization script
-- Mirrors node-server/scripts/init-postgres.sql exactly

CREATE TABLE IF NOT EXISTS endpoint_metrics (
    id SERIAL PRIMARY KEY,
    client_id VARCHAR(50) NOT NULL,
    service_name VARCHAR(255) NOT NULL,
    endpoint VARCHAR(500) NOT NULL,
    method VARCHAR(10) NOT NULL,
    total_hits INTEGER DEFAULT 0,
    error_hits INTEGER DEFAULT 0,
    avg_latency DOUBLE PRECISION DEFAULT 0,
    min_latency DOUBLE PRECISION DEFAULT 0,
    max_latency DOUBLE PRECISION DEFAULT 0,
    time_bucket TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Unique constraint for upsert
CREATE UNIQUE INDEX IF NOT EXISTS idx_endpoint_metrics_unique
ON endpoint_metrics(client_id, service_name, endpoint, method, time_bucket);

-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_endpoint_metrics_client
ON endpoint_metrics(client_id);

CREATE INDEX IF NOT EXISTS idx_endpoint_metrics_time
ON endpoint_metrics(time_bucket);

CREATE INDEX IF NOT EXISTS idx_endpoint_metrics_service
ON endpoint_metrics(service_name);
