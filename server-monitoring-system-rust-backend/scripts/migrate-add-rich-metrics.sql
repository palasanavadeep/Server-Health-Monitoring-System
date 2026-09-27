-- Migration: rename histogram and status columns to professional naming convention.
-- Run against any existing database that was created with the old init-postgres.sql.
-- Safe to run multiple times (IF EXISTS / IF NOT EXISTS guards).
--
-- Old naming            New naming
-- ─────────────────────────────────────
-- lat_le_10             latency_le_10ms
-- lat_le_25             latency_le_25ms
-- lat_le_50             latency_le_50ms
-- lat_le_100            latency_le_100ms
-- lat_le_200            latency_le_200ms
-- lat_le_500            latency_le_500ms
-- lat_le_1000           latency_le_1000ms
-- lat_le_2500           latency_le_2500ms
-- lat_le_5000           latency_le_5000ms
-- lat_le_10000          latency_le_10000ms
-- lat_le_20000          latency_le_20000ms
-- http_1xx              status_1xx
-- http_2xx              status_2xx
-- http_3xx              status_3xx
-- http_4xx              status_4xx
-- http_5xx              status_5xx

ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_10    TO latency_le_10ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_25    TO latency_le_25ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_50    TO latency_le_50ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_100   TO latency_le_100ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_200   TO latency_le_200ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_500   TO latency_le_500ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_1000  TO latency_le_1000ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_2500  TO latency_le_2500ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_5000  TO latency_le_5000ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_10000 TO latency_le_10000ms;
ALTER TABLE endpoint_metrics
    RENAME COLUMN lat_le_20000 TO latency_le_20000ms;

ALTER TABLE endpoint_metrics
    RENAME COLUMN http_1xx     TO status_1xx;
ALTER TABLE endpoint_metrics
    RENAME COLUMN http_2xx     TO status_2xx;
ALTER TABLE endpoint_metrics
    RENAME COLUMN http_3xx     TO status_3xx;
ALTER TABLE endpoint_metrics
    RENAME COLUMN http_4xx     TO status_4xx;
ALTER TABLE endpoint_metrics
    RENAME COLUMN http_5xx     TO status_5xx;
