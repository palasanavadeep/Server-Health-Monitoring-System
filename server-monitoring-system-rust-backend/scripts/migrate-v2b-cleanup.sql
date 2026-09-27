-- Migration v2b — Destructive column drops (run AFTER 100% of instances on v2 code)
--
-- !! DANGER: Only run this after confirming every running app instance is on v2. !!
-- Running this while any v1 instance is alive will cause write failures on that instance.
--
-- What this removes:
--   • 11 old coarse histogram columns (replaced by 23 finer-grained ones in v2a)
--   • 3 apdex_* columns (Apdex is now derived at read time from the histogram)
--
-- Data-loss acknowledgement:
--   Historical histogram counts in the old 11 columns are discarded. The new 23
--   columns (added in v2a) start from zero. This is an accepted trade-off — a
--   backfill from raw MongoDB events is possible but not planned.

-- Drop old 11-bucket histogram columns
ALTER TABLE endpoint_metrics DROP COLUMN IF EXISTS latency_le_2500ms;

-- Drop Apdex write-time counters (now derived from histogram at read time)
ALTER TABLE endpoint_metrics DROP COLUMN IF EXISTS apdex_satisfied;
ALTER TABLE endpoint_metrics DROP COLUMN IF EXISTS apdex_tolerating;
ALTER TABLE endpoint_metrics DROP COLUMN IF EXISTS apdex_frustrated;
