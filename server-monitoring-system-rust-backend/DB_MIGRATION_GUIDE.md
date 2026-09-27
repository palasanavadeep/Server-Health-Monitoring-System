# DB Migration Guide — Server Monitoring System

> **Version:** v2.1 (September 2026)
> This guide covers all PostgreSQL schema migrations from the initial v1 schema to the current v2.1 production schema.

---

## Quick Reference

| Your starting state            | Script to run                                              |
|--------------------------------|------------------------------------------------------------|
| Fresh database (no tables)     | `scripts/init-postgres.sql`                                |
| v1 schema (basic counters)     | `scripts/migrate-v2-full.sql`                              |
| v1 with some histogram columns | `scripts/migrate-v2a-additive.sql` → deploy → `migrate-v2b-cleanup.sql` → `migrate-v2c-indexes.sql` |
| Already on v2                  | Nothing — schema is current                                |

---

## 1. Schema Version History

| Version | What changed |
|---------|-------------|
| **v1**  | Basic `endpoint_metrics` table: `total_hits`, `error_hits`, `avg_latency`, `min_latency`, `max_latency`, `time_bucket`, `service_name` |
| **v2**  | Added 23 cumulative histogram columns (`latency_le_*`), 5 status class counters (`status_*`), `histogram_profiles` table, `client_metric_config` table, performance indexes |
| **v2.1**| No schema changes — code-level improvements only (service-level analytics via `GROUP BY service_name`, JWT permission embedding, API key caching) |

---

## 2. Fresh Installation (no existing DB)

Run once against a new PostgreSQL database:

```bash
# Docker Compose setup
docker exec -i server-monitoring-postgres \
  psql -U postgres -d server_monitoring \
  < scripts/init-postgres.sql

# Or direct psql
psql -h localhost -U postgres -d server_monitoring \
  -f scripts/init-postgres.sql
```

This creates:
- `endpoint_metrics` with all 23 histogram columns and status counters
- `processed_metric_events` with dedup index
- `histogram_profiles` with 3 system profiles (`standard`, `latency_focused`, `long_running`)
- `client_metric_config` with FK reference to `histogram_profiles`
- All query indexes

**Verify:**
```sql
SELECT table_name FROM information_schema.tables
WHERE table_schema = 'public' ORDER BY table_name;
-- Should show: client_metric_config, endpoint_metrics,
--              histogram_profiles, processed_metric_events
```

---

## 3. Migrating a Running v1 Database (Zero-Downtime)

The v2 migration is split into three scripts to enable a rolling deploy with zero downtime.

### Step 1 — Add columns (safe, run BEFORE deploying new code)

```bash
docker exec -i server-monitoring-postgres \
  psql -U postgres -d server_monitoring \
  < scripts/migrate-v2a-additive.sql
```

**What this does:**
- Creates `histogram_profiles` and `client_metric_config` tables
- Adds 23 `latency_le_*` columns to `endpoint_metrics` with `DEFAULT 0`
- Adds 5 `status_*` columns with `DEFAULT 0`
- Creates performance indexes

**Safety:** All `ALTER TABLE ADD COLUMN IF NOT EXISTS` with `DEFAULT 0` are instant metadata operations in PostgreSQL 11+. No table rewrite. No downtime. Old code continues writing its existing columns unchanged.

**Verify step 1:**
```sql
SELECT column_name FROM information_schema.columns
WHERE table_name = 'endpoint_metrics'
  AND column_name LIKE 'latency_le_%'
ORDER BY ordinal_position;
-- Should return 23 rows
```

### Step 2 — Deploy the new Rust binary

```bash
# Docker Compose rolling update
docker-compose up -d --no-deps api-server metrics-worker consumer
```

The new binary now writes to all 23 histogram columns and reads from them.
Old rows (pre-migration) have all histogram columns = 0 — they contribute 0 to percentile estimates.

### Step 3 — Clean up old columns (optional, run AFTER verifying deploy)

```bash
docker exec -i server-monitoring-postgres \
  psql -U postgres -d server_monitoring \
  < scripts/migrate-v2b-cleanup.sql
```

> ⚠️ **Warning:** This script drops columns that the v1 binary wrote to (e.g. `apdex_score` if it existed). Make sure all instances are running the v2 binary before running this.

### Step 4 — Additional performance indexes (optional)

```bash
docker exec -i server-monitoring-postgres \
  psql -U postgres -d server_monitoring \
  < scripts/migrate-v2c-indexes.sql
```

> ℹ️ **Note:** `migrate-v2c-indexes.sql` uses `CREATE INDEX CONCURRENTLY` which cannot run inside a transaction. Run it directly, not inside a `BEGIN…COMMIT` block.

---

## 4. Migrating from v1 (one-shot, requires brief downtime)

If you can take a maintenance window (even 5–10 minutes), use the single combined script:

```bash
# Stop the workers first
docker-compose stop metrics-worker consumer

# Run the combined migration
docker exec -i server-monitoring-postgres \
  psql -U postgres -d server_monitoring \
  < scripts/migrate-v2-full.sql

# Verify
docker exec -it server-monitoring-postgres \
  psql -U postgres -d server_monitoring \
  -c "SELECT COUNT(*) AS histogram_columns FROM information_schema.columns
      WHERE table_name = 'endpoint_metrics' AND column_name LIKE 'latency_%';"
# Expected: 23

# Deploy new code and start services
docker-compose up -d
```

---

## 5. Per-Tenant Configuration

After migrating, you can configure per-tenant settings:

```sql
-- Set a custom Apdex T threshold for a client (default: 500 ms)
INSERT INTO client_metric_config (client_id, apdex_threshold_ms, histogram_profile)
VALUES ('your-client-id-here', 200.0, 'latency_focused')
ON CONFLICT (client_id) DO UPDATE
  SET apdex_threshold_ms = EXCLUDED.apdex_threshold_ms,
      histogram_profile   = EXCLUDED.histogram_profile,
      updated_at          = NOW();

-- Set a daily ingest quota (NULL = unlimited)
UPDATE client_metric_config
SET daily_ingest_quota = 1000000
WHERE client_id = 'your-client-id-here';

-- Set data retention (days, default 90)
UPDATE client_metric_config
SET data_retention_days = 30
WHERE client_id = 'your-client-id-here';
```

**Available histogram profiles:**

| Profile name      | Buckets | Best for                                      |
|-------------------|---------|-----------------------------------------------|
| `standard`        | 23      | General-purpose REST APIs                     |
| `latency_focused` | 16      | User-facing APIs, latency < 1 s               |
| `long_running`    | 14      | Batch jobs, background tasks (1–60 s range)   |

---

## 6. Data Retention Management

Old rows are deleted by the Metrics Worker's nightly retention job. You can also run it manually:

```sql
-- Preview what would be deleted (dry run)
SELECT client_id, COUNT(*) AS rows_to_delete,
       MIN(time_bucket) AS oldest, MAX(time_bucket) AS newest
FROM endpoint_metrics em
JOIN client_metric_config cmc ON em.client_id = cmc.client_id
WHERE em.time_bucket < NOW() - (cmc.data_retention_days || ' days')::INTERVAL
GROUP BY em.client_id;

-- Actually delete (replace '90 days' with your policy)
DELETE FROM endpoint_metrics
WHERE time_bucket < NOW() - INTERVAL '90 days';

-- Delete old deduplication records
DELETE FROM processed_metric_events
WHERE processed_at < NOW() - INTERVAL '90 days';
```

---

## 7. Index Maintenance

Recommended periodic maintenance (run during low-traffic windows):

```sql
-- Reclaim space after bulk deletions
VACUUM ANALYZE endpoint_metrics;
VACUUM ANALYZE processed_metric_events;

-- Check index bloat (run as postgres superuser)
SELECT relname AS table, n_dead_tup, n_live_tup,
       round(n_dead_tup::numeric / NULLIF(n_live_tup + n_dead_tup, 0) * 100, 1) AS dead_pct
FROM pg_stat_user_tables
WHERE schemaname = 'public'
ORDER BY n_dead_tup DESC;
```

---

## 8. Rollback Procedures

### Rollback Step 1 (before code deploy)

If you need to undo the additive migration:

```sql
-- Remove new columns (only if no v2 code has written to them yet)
ALTER TABLE endpoint_metrics
  DROP COLUMN IF EXISTS latency_le_5ms,
  DROP COLUMN IF EXISTS latency_le_25ms,
  -- ... all new columns ...
  DROP COLUMN IF EXISTS status_5xx;

DROP TABLE IF EXISTS client_metric_config;
DROP TABLE IF EXISTS histogram_profiles;
```

### Rollback Code Only (after step 2, if step 3 not run)

If the new binary has a bug and you need to roll back code but keep the schema:
1. Deploy the previous binary (it ignores the new columns — they just have defaults).
2. Old code writes to old columns normally.
3. New columns accumulate 0s for the rollback period.
4. Re-deploy v2 code when ready — it resumes writing to all columns.

> ℹ️ **There is no data loss** in a code-only rollback because `migrate-v2a-additive.sql` only adds columns with defaults. The v1 binary simply never writes to them.

---

## 9. Health Check Queries

```sql
-- How many metric rows exist per client?
SELECT client_id,
       COUNT(*)                        AS total_rows,
       SUM(total_hits)                 AS total_hits,
       MIN(time_bucket)                AS oldest_bucket,
       MAX(time_bucket)                AS newest_bucket
FROM endpoint_metrics
GROUP BY client_id
ORDER BY total_hits DESC;

-- How many distinct (service, endpoint, method) tuples?
SELECT client_id,
       COUNT(DISTINCT endpoint || '|' || method)                AS unique_operations,
       COUNT(DISTINCT service_name)                             AS unique_services,
       COUNT(DISTINCT service_name || '|' || endpoint || '|' || method) AS unique_service_operations
FROM endpoint_metrics
GROUP BY client_id;

-- Is the dedup table growing? (should stay bounded by retention policy)
SELECT COUNT(*) AS dedup_rows, MIN(processed_at), MAX(processed_at)
FROM processed_metric_events;

-- Are histogram columns being populated? (should be > 0 for recent rows)
SELECT time_bucket,
       SUM(latency_le_100ms) AS fast_requests,
       SUM(total_hits)       AS total_requests,
       round(100.0 * SUM(latency_le_100ms) / NULLIF(SUM(total_hits), 0), 1) AS pct_under_100ms
FROM endpoint_metrics
WHERE time_bucket > NOW() - INTERVAL '24 hours'
GROUP BY time_bucket
ORDER BY time_bucket DESC
LIMIT 24;
```
