-- Migration v2c — New query indexes (non-transactional)
--
-- !! IMPORTANT: Run this file OUTSIDE a transaction. !!
-- CREATE INDEX CONCURRENTLY will fail if wrapped in BEGIN/COMMIT.
-- For refinery or sqlx-migrate: mark this file as non-transactional.
--
-- Safe to run at any time; CONCURRENTLY does not block reads or writes.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_em_client_time_desc
    ON endpoint_metrics (client_id, time_bucket DESC);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_em_client_service_time_desc
    ON endpoint_metrics (client_id, service_name, time_bucket DESC);
