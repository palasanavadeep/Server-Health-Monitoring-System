# Architecture — Server Monitoring System (Rust Backend)

> **Version:** v2.1 (September 2026)
> Covers: API server, Persistence Consumer, Metrics Worker, data pipeline, complete PostgreSQL schema, all analytics algorithms, caching strategy, security model, and known trade-offs.

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Binary Entry Points](#2-binary-entry-points)
3. [Module Tree](#3-module-tree)
4. [Data Pipeline](#4-data-pipeline)
5. [Database Schema](#5-database-schema)
6. [Idempotency & Delivery Guarantees](#6-idempotency--delivery-guarantees)
7. [Metrics Computation Algorithms](#7-metrics-computation-algorithms)
8. [API Reference](#8-api-reference)
9. [Security Model](#9-security-model)
10. [Caching Layer](#10-caching-layer)
11. [Configuration](#11-configuration)
12. [Deployment](#12-deployment)
13. [Known Trade-offs & Future Work](#13-known-trade-offs--future-work)

---

## 1. System Overview

The backend is split into **three independent processes** communicating via RabbitMQ:

```
SDK / Agent
    │ POST /api/hit  (with API key)
    ▼
┌─────────────────────────────────────────────────────┐
│              API Server (Actix-Web)                 │
│  • Validates API key (in-process TTL cache)         │
│  • Enforces per-client ingest quota (in-process)    │
│  • Publishes HitEvent to RabbitMQ                   │
│  • Serves all analytics queries from PostgreSQL     │
│  • All JWT auth — zero DB calls for analytics auth  │
└─────────────────────────────────────────────────────┘
                  │ server_hits queue
                  ▼
┌─────────────────────────────────────────────────────┐
│             Persistence Consumer                    │
│  • Writes raw HitEvent to MongoDB (event log)       │
│  • Publishes MetricsEvent to metrics_events queue   │
│  • Publisher confirm → ACK original hit             │
└─────────────────────────────────────────────────────┘
                  │ metrics_events queue
                  ▼
┌─────────────────────────────────────────────────────┐
│               Metrics Worker                        │
│  • Dedup via processed_metric_events INSERT          │
│  • Upserts endpoint_metrics (23-bucket histogram)   │
│  • Atomic transaction: dedup + upsert together      │
│  • ACK only after commit                            │
└─────────────────────────────────────────────────────┘
```

### Storage Roles

| Store      | Purpose                                                           |
|------------|-------------------------------------------------------------------|
| MongoDB    | Raw hit event log — source of truth for individual requests       |
| PostgreSQL | Aggregated metrics — source of truth for all analytics queries    |
| RabbitMQ   | Durable message transport between the three processes             |

---

## 2. Binary Entry Points

| Binary (`src/bin/`)  | Description                                       |
|----------------------|---------------------------------------------------|
| `api_server.rs`      | Starts the Actix-Web HTTP server                  |
| `consumer.rs`        | Starts the Persistence Consumer                   |
| `metrics_worker.rs`  | Starts the Metrics Worker                         |

Each binary initialises its own connection pools and dependency graph independently.
Scaling is per-binary: run N Metrics Workers to increase aggregation throughput.

---

## 3. Module Tree

```
src/
├── bin/
│   ├── api_server.rs         ← HTTP server entry point
│   ├── consumer.rs           ← Persistence Consumer entry point
│   └── metrics_worker.rs     ← Metrics Worker entry point
├── app_state.rs              ← Dependency injection container (Arc-wrapped)
├── cache/
│   ├── api_key_cache.rs      ← In-process TTL cache for API key validation
│   ├── ingest_quota_tracker.rs ← Per-client daily ingest quota (DashMap)
│   └── tenant_config_cache.rs  ← Per-tenant config TTL cache (moka)
├── config/
│   ├── database.rs           ← MongoDB + SeaORM connection setup
│   ├── messaging.rs          ← RabbitMQ connection
│   ├── settings.rs           ← AppConfig — typed env-var configuration
│   └── telemetry.rs          ← tracing-subscriber initialisation
├── domain/
│   ├── api_key.rs            ← ApiKey, ApiKeyPermissions, ApiKeySecurity
│   ├── ingest.rs             ← HitEvent, MetricsEvent (wire types)
│   ├── metrics.rs            ← Aggregated domain types, histogram algorithms
│   ├── role.rs               ← Role enum (SuperAdmin, Admin, Viewer)
│   ├── tenant_config.rs      ← TenantConfig with compute_percentiles / compute_apdex
│   └── user.rs               ← User, JwtClaims (with embedded permissions)
├── dto/
│   ├── request/              ← Inbound request body structs
│   └── response/
│       └── analytics.rs      ← All analytics response DTOs
├── error/
│   └── app_error.rs          ← AppError enum with to_response()
├── handler/
│   ├── analytics.rs          ← 5 analytics handlers + ServiceQuery param
│   ├── auth.rs               ← Login, register, profile handlers
│   ├── client.rs             ← Client/API key management handlers
│   ├── health.rs             ← Health check handler
│   └── ingest.rs             ← POST /api/hit ingest handler
├── messaging/
│   ├── consumer.rs           ← RabbitMQ AMQP consumer loop
│   ├── metrics_consumer.rs   ← Metrics Worker consumer loop
│   └── producer.rs           ← EventProducer with retry + circuit breaker
├── middleware/
│   ├── authenticate.rs       ← JWT validation → AuthenticatedUser extension
│   ├── request_logger.rs     ← Structured request/response logging
│   └── validate_api_key.rs   ← API key middleware (uses ApiKeyCache)
├── repository/
│   ├── api_key_repo.rs       ← MongoDB API key CRUD
│   ├── client_repo.rs        ← MongoDB client CRUD
│   ├── metrics_repo.rs       ← PostgreSQL metrics queries (trait + impl)
│   ├── tenant_config_repo.rs ← PostgreSQL tenant config CRUD
│   └── user_repo.rs          ← MongoDB user CRUD
├── resilience/
│   ├── circuit_breaker.rs    ← Atomic state machine (Closed/Open/Half-Open)
│   └── retry.rs              ← Exponential backoff + jitter retry strategy
├── router/
│   └── routes.rs             ← Route configuration and scope definitions
├── service/
│   ├── analytics.rs          ← Analytics orchestration (no SQL, no serialisation)
│   ├── auth.rs               ← Login, token generation, profile management
│   ├── client.rs             ← Client and API key lifecycle
│   ├── ingest.rs             ← Ingest quota check + event publish
│   ├── metrics_processor.rs  ← Metrics Worker: process_event wrapper
│   └── tenant_config.rs      ← TenantConfig retrieval with TTL cache
└── util/
    ├── response.rs           ← ResponseFormatter (unified JSON envelope)
    └── security.rs           ← Password hashing, validation
```

---

## 4. Data Pipeline

### 4.1 Ingest Path (hot path)

```
POST /api/hit
  1. validate_api_key middleware
     ├── ApiKeyCache.get_or_load(sha256(key), || load_from_mongo())
     │   Cache hit  → zero MongoDB calls
     │   Cache miss → one MongoDB call, single-flight collapse (moka)
     └── Check: is_active, expiry, client_active, IP allow-list, origin, can_ingest
  2. authenticate middleware (JWT validation — CPU only, no DB)
  3. ingest handler
     ├── quota_tracker.check_and_increment(client_id, quota, 1)
     │   Exceeded → 429 Too Many Requests
     └── event_producer.publish(HitEvent)
         ├── Circuit breaker check
         └── RabbitMQ publish with publisher confirm
             Retry: exponential backoff + jitter (configurable)
```

### 4.2 Persistence Path

```
RabbitMQ (server_hits)
  → consumer.rs
  → MongoDB: hits.insert_one(hit_document)
  → RabbitMQ (metrics_events): publish MetricsEvent
  → ACK original hit (publisher confirm required)
```

### 4.3 Metrics Aggregation Path

```
RabbitMQ (metrics_events)
  → metrics_worker.rs
  → PostgreSQL transaction:
      BEGIN
      INSERT INTO processed_metric_events (event_id) ON CONFLICT DO NOTHING
        → rows_affected == 0 → COMMIT (duplicate, safe skip)
      Compute LatencyHistogram::from_latency_ms(latency_ms)
      Compute status class (1xx/2xx/3xx/4xx/5xx)
      UPSERT endpoint_metrics (UPSERT_ENDPOINT_METRICS const, not format!)
        ON CONFLICT (client_id, service_name, endpoint, method, time_bucket)
        DO UPDATE: total_hits+1, running avg_latency, SUM histogram buckets
      COMMIT
  → ACK (only after commit)
  → NACK + requeue on any error → idempotent redelivery
```

---

## 5. Database Schema

### 5.1 `endpoint_metrics` — Core Analytics Table

```sql
CREATE TABLE endpoint_metrics (
    id            BIGSERIAL        PRIMARY KEY,
    client_id     VARCHAR(64)      NOT NULL,
    service_name  VARCHAR(255)     NOT NULL,
    endpoint      VARCHAR(512)     NOT NULL,
    method        VARCHAR(16)      NOT NULL,

    -- Throughput counters
    total_hits    INTEGER          NOT NULL DEFAULT 0,
    error_hits    INTEGER          NOT NULL DEFAULT 0,

    -- Running latency statistics
    avg_latency   DOUBLE PRECISION NOT NULL DEFAULT 0.0,  -- weighted running average
    min_latency   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    max_latency   DOUBLE PRECISION NOT NULL DEFAULT 0.0,

    -- 23-bucket cumulative histogram (Prometheus-style)
    -- Each bucket i is incremented when latency_ms <= upper_bound[i]
    -- Buckets: 5,10,25,50,75,100,150,200,300,400,500,750,
    --          1000,1500,2000,3000,5000,7500,10000,15000,20000,30000,60000 ms
    latency_le_5ms      INTEGER NOT NULL DEFAULT 0,
    -- ... 23 columns total ...
    latency_le_60000ms  INTEGER NOT NULL DEFAULT 0,

    -- HTTP status class distribution
    status_1xx    INTEGER NOT NULL DEFAULT 0,
    status_2xx    INTEGER NOT NULL DEFAULT 0,
    status_3xx    INTEGER NOT NULL DEFAULT 0,
    status_4xx    INTEGER NOT NULL DEFAULT 0,
    status_5xx    INTEGER NOT NULL DEFAULT 0,

    time_bucket   TIMESTAMP        NOT NULL,  -- truncated to clock-hour
    created_at    TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT uq_endpoint_metrics
        UNIQUE (client_id, service_name, endpoint, method, time_bucket)
);

-- Primary query index: filter by client + time range
CREATE INDEX idx_em_client_time_desc
    ON endpoint_metrics (client_id, time_bucket DESC);

-- Service-level analytics: filter by client + service + time range
CREATE INDEX idx_em_client_service_time_desc
    ON endpoint_metrics (client_id, service_name, time_bucket DESC);

-- Endpoint-level analytics: point lookups
CREATE INDEX idx_em_service_endpoint
    ON endpoint_metrics (service_name, endpoint, method);
```

**Key properties:**
- One row per `(client_id, service_name, endpoint, method, time_bucket)` hour.
- Percentiles and Apdex are **never stored** — computed at query time from histogram.
- Service-level metrics use `GROUP BY service_name` — no separate table needed.
- All counter columns are additive, making them safe under at-least-once delivery.

### 5.2 `processed_metric_events` — Deduplication Log

```sql
CREATE TABLE processed_metric_events (
    event_id     VARCHAR(128) PRIMARY KEY,
    processed_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_pme_processed_at ON processed_metric_events (processed_at);
```

Retention: controlled by `METRIC_EVENT_DEDUP_RETENTION_DAYS` (default 90 days).

### 5.3 `histogram_profiles` — Read-Time Lens Configuration

```sql
CREATE TABLE histogram_profiles (
    profile_name   VARCHAR(64)        NOT NULL PRIMARY KEY,
    description    TEXT               NOT NULL,
    bucket_bounds  DOUBLE PRECISION[] NOT NULL,  -- subset of the 23 canonical bounds
    is_system      BOOLEAN            NOT NULL DEFAULT TRUE,
    created_at     TIMESTAMP          NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT chk_bucket_bounds_subset CHECK (
        bucket_bounds <@ ARRAY[5,10,25,...,60000]::double precision[]
    )
);
```

**System profiles (read-only):**
- `standard` — all 23 buckets, general-purpose
- `latency_focused` — 16 high-resolution sub-1000ms buckets
- `long_running` — 14 buckets optimised for 1–60 second APIs

**Important:** These profiles control which buckets are *displayed* to the user. Storage always uses all 23 columns — no data loss when the profile is changed.

### 5.4 `client_metric_config` — Per-Tenant Settings

```sql
CREATE TABLE client_metric_config (
    client_id            VARCHAR(64)      NOT NULL PRIMARY KEY,
    apdex_threshold_ms   DOUBLE PRECISION NOT NULL DEFAULT 500.0,
    histogram_profile    VARCHAR(64)      NOT NULL DEFAULT 'standard'
                         REFERENCES histogram_profiles (profile_name),
    data_retention_days  INTEGER          NOT NULL DEFAULT 90,
    daily_ingest_quota   BIGINT,          -- NULL = unlimited
    created_at           TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

Defaults apply if no row exists for a `client_id` — queries never fail due to missing config.

---

## 6. Idempotency & Delivery Guarantees

### Event Deduplication

Every `MetricsEvent` carries a globally-unique `event_id` (UUID v4 assigned at origin). The Metrics Worker's transaction:

1. `INSERT INTO processed_metric_events (event_id) ON CONFLICT DO NOTHING`
2. If `rows_affected == 0` → duplicate, commit empty transaction, skip upsert
3. If `rows_affected == 1` → new event, execute upsert, commit both atomically

This provides **exactly-once semantics** on top of RabbitMQ's at-least-once delivery.

### Running Average Formula

```
new_avg = (old_avg × old_count + new_latency) / (old_count + 1)
```

Implemented in the upsert ON CONFLICT clause as:
```sql
avg_latency = (
    endpoint_metrics.avg_latency * endpoint_metrics.total_hits
    + EXCLUDED.avg_latency
) / NULLIF(endpoint_metrics.total_hits + 1, 0)
```

### Circuit Breaker (RabbitMQ publish path)

```
States: Closed → Open → Half-Open → Closed
Thresholds: configurable via CB_FAILURE_THRESHOLD, CB_COOLDOWN_MS, CB_HALF_OPEN_ATTEMPTS
```

When the circuit is Open, publish attempts fail-fast with a 503 error to the caller.

---

## 7. Metrics Computation Algorithms

All metrics are computed at **read time** from the raw stored counters. No derived metrics are stored.

### 7.1 Latency Percentile Estimation

```
Algorithm: Linear interpolation within the bucket that straddles the target count.

target_count = ceil(quantile × total_hits)
for each bucket i (ascending upper_bound):
    if cumulative_count[i] >= target_count:
        fraction = (target_count − prev_count) / (count[i] − prev_count)
        estimate = lower_bound[i] + fraction × bucket_width
        return round(estimate, 2)
return 60_000.0  (all requests exceed 60 s)
```

**Accuracy:** Worst-case error ≤ bucket width (max 2500 ms in the 5–7.5 s range, <25 ms for the first 15 buckets).

### 7.2 Apdex Score

```
T  = tenant's apdex_threshold_ms (from client_metric_config)
4T = frustrated boundary

satisfied  = cumulative count of the highest bucket with upper_bound ≤ T
tolerating = (count at 4T) − satisfied
frustrated = total_hits − satisfied − tolerating

Apdex = (satisfied + tolerating / 2) / total_hits   ∈ [0.0, 1.0]
```

The Apdex T threshold is a per-tenant runtime value — changing it retroactively re-evaluates all historical data without any re-processing.

### 7.3 Service Health Classification

Computed at read time from the service's Apdex score and error rate:

| Status     | Condition                                 |
|------------|-------------------------------------------|
| `healthy`  | Apdex ≥ 0.9 **and** error rate ≤ 1.0%    |
| `degraded` | Apdex ≥ 0.7 **and** error rate ≤ 5.0%    |
| `critical` | Everything else                           |

### 7.4 Throughput (requests per minute)

```
throughput_rpm = total_hits / window_minutes
```

- `/percentiles` endpoint: uses exact `end_time − start_time` window (accurate).
- `/apis` paginated list: uses `COUNT(*) × 60 minutes` (approximate — see Known Trade-offs).

---

## 8. API Reference

All routes are in `src/router/routes.rs`. Base path: `/api`.

### Authentication Routes (no JWT required)

| Method | Path                | Description                          |
|--------|---------------------|--------------------------------------|
| POST   | `/auth/register`    | Register first super-admin           |
| POST   | `/auth/login`       | Login, returns JWT                   |
| GET    | `/auth/profile`     | Get own profile (JWT required)       |
| PUT    | `/auth/profile`     | Update own profile (JWT required)    |

### Ingest Route (API key required)

| Method | Path      | Description                                   |
|--------|-----------|-----------------------------------------------|
| POST   | `/api/hit`| Submit a single request hit event             |

### Analytics Routes (JWT required, `can_view_analytics` claim)

| Method | Path                     | Description                                       |
|--------|--------------------------|---------------------------------------------------|
| GET    | `/api/analytics/stats`   | Aggregated statistics for a time range            |
| GET    | `/api/analytics/dashboard` | Dashboard: stats + top endpoints + time series  |
| GET    | `/api/analytics/apis`    | Paginated endpoint metrics table                  |
| GET    | `/api/analytics/percentiles` | Full metrics for a single endpoint           |
| GET    | `/api/analytics/services` | Service-level analytics (fleet or single)        |

#### `GET /api/analytics/services`

Dual-purpose endpoint controlled by `?serviceName=`:

| Mode              | Query param       | Response                                             |
|-------------------|-------------------|------------------------------------------------------|
| Fleet overview    | (none)            | `ServiceHealthResponse[]` — one row per service      |
| Service detail    | `?serviceName=X`  | `ServiceMetricsResponse` — full metrics + endpoints  |

**Common query params:** `startTime`, `endTime` (ms epoch or RFC3339; default last 24h), `clientId` (super-admin only).

**Service health response fields:** `serviceName`, `totalHits`, `errorHits`, `errorRate`, `avgLatency`, `p99Latency`, `apdex`, `throughputRpm`, `endpointCount`, `status`, `statusDistribution`.

**Service metrics response:** includes all the above plus `percentiles` (p50/p75/p90/p95/p99), a full `endpoints` list with per-endpoint `apdexScore` and `p99Latency`, and `timeRange`.

### Client / API Key Management (JWT required, admin role)

| Method | Path                           | Description                          |
|--------|--------------------------------|--------------------------------------|
| POST   | `/api/clients`                 | Create a new client                  |
| GET    | `/api/clients`                 | List all clients (super-admin only)  |
| POST   | `/api/clients/{id}/api-keys`   | Generate API key for a client        |
| PUT    | `/api/clients/{id}/api-keys/{kid}` | Update API key permissions       |
| DELETE | `/api/clients/{id}/api-keys/{kid}` | Revoke an API key                |

---

## 9. Security Model

### JWT Claims

JWT tokens embed all authorisation data at login time, eliminating database round-trips on every request:

```rust
pub struct JwtClaims {
    pub user_id:           String,
    pub email:             String,
    pub username:          String,
    pub role:              String,
    pub client_id:         Option<String>,
    pub is_super_admin:    bool,   // role == SuperAdmin, embedded at login
    pub can_view_analytics: bool,  // permissions.can_view_analytics, embedded at login
    pub iat: i64,
    pub exp: i64,
}
```

**Trade-off:** A revoked `can_view_analytics` permission takes up to `JWT_EXPIRES_IN` (default 24h) to propagate. This is the standard JWT trade-off, acceptable for analytics permissions.

### API Key Validation

- Raw key is never logged or stored in memory beyond the single validation call.
- Cache keys use `sha256(raw_api_key)` — plaintext is never held in the cache.
- TTL is short (default 60 s) so revoked keys expire within one minute.
- Revocation handlers call `ApiKeyCache::invalidate_by_raw_key()` for immediate local effect.
- **Cross-instance revocation:** Each instance has its own cache. A revoked key can be used on other instances for up to TTL seconds. Acceptable trade-off — use Redis for strict cross-instance revocation.

### Security Checks in `validate_api_key` middleware

1. Extract key from `X-API-Key` header
2. Load from cache or MongoDB
3. Check `is_active = true`
4. Check `expires_at > now` (if set)
5. Check `client.is_active = true`
6. Check IP allow-list (if configured)
7. Check allowed origin (if configured)
8. Check `permissions.can_ingest = true`

---

## 10. Caching Layer

### ApiKeyCache (`cache/api_key_cache.rs`)

| Property         | Value                                |
|------------------|--------------------------------------|
| Implementation   | `moka::future::Cache` (async-native) |
| Key              | `sha256(raw_api_key)` (hex string)   |
| Value            | `Arc<CachedApiKeyEntry>`             |
| TTL              | Configurable (default 60 s)          |
| Max capacity     | Configurable (default 10 000 entries)|
| Single-flight    | `try_get_with` — concurrent misses for the same key collapsed into one DB load |

### TenantConfigCache (`cache/tenant_config_cache.rs`)

| Property         | Value                                |
|------------------|--------------------------------------|
| Implementation   | `moka::future::Cache`                |
| Key              | `client_id: String`                  |
| Value            | `Arc<TenantConfig>`                  |
| TTL              | Configurable (default 300 s)         |

### IngestQuotaTracker (`cache/ingest_quota_tracker.rs`)

| Property         | Value                                |
|------------------|--------------------------------------|
| Implementation   | `DashMap<String, Arc<AtomicI64>>`    |
| Key              | `"{client_id}:{YYYY-MM-DD}"`         |
| Granularity      | Per-client, per UTC day              |
| Pruning          | Background task every 10 minutes (removes previous-day entries) |
| Multi-instance   | Each instance tracks independently — soft limit, not billing-grade |

---

## 11. Configuration

All configuration is via environment variables. See `.env.sample` for full reference.

| Variable                    | Default    | Description                                     |
|-----------------------------|------------|-------------------------------------------------|
| `PORT`                      | `8080`     | HTTP server port                                |
| `MONGO_URI`                 | —          | MongoDB connection string                       |
| `PG_HOST` / `PG_PORT` / `PG_DATABASE` / `PG_USER` / `PG_PASSWORD` | — | PostgreSQL connection |
| `RABBITMQ_URL`              | —          | AMQP connection string                          |
| `JWT_SECRET`                | —          | HMAC-SHA256 signing key (min 32 chars in prod)  |
| `JWT_EXPIRES_IN`            | `24h`      | Token lifetime (`Nh`, `Nd`, `Nm` format)        |
| `ENVIRONMENT`               | `development` | Controls log format (JSON in production)     |
| `API_KEY_CACHE_TTL_SECS`    | `60`       | API key cache entry lifetime                    |
| `API_KEY_CACHE_MAX_CAPACITY`| `10000`    | API key cache max entries                       |
| `TENANT_CONFIG_CACHE_TTL_SECS` | `300`  | Tenant config cache entry lifetime              |
| `CB_FAILURE_THRESHOLD`      | `5`        | Circuit breaker: failures before opening        |
| `CB_COOLDOWN_MS`            | `30000`    | Circuit breaker: cooldown before half-open      |
| `METRIC_EVENT_DEDUP_RETENTION_DAYS` | `90` | Dedup record retention                      |

---

## 12. Deployment

### Docker Compose (development)

```bash
docker-compose up -d
```

Services: `api-server`, `consumer`, `metrics-worker`, `mongo`, `postgres`, `rabbitmq`.

### Production Recommendations

- Run `api-server` behind a reverse proxy (NGINX / Traefik) for TLS termination.
- Run ≥2 `api-server` instances for HA; all are stateless except in-process cache.
- Run ≥2 `consumer` and `metrics-worker` instances — RabbitMQ handles load balancing.
- Use PostgreSQL connection pooling (pgBouncer) for `metrics-worker` at high throughput.
- Set `ENVIRONMENT=production` — switches tracing output to structured JSON.
- Use a secrets manager (AWS Secrets Manager, HashiCorp Vault) for `JWT_SECRET`, DB credentials.

### Health Check

```
GET /health  →  200 OK
{
  "status": "ok",
  "postgres": "connected",
  "mongoDb": "connected",
  "circuitBreaker": "closed"
}
```

---

## 13. Known Trade-offs & Future Work

### Current Limitations

| ID  | Severity | Description | Resolution path |
|-----|----------|-------------|-----------------|
| T1  | 🟡 Medium | **Throughput approximation in `/apis` list**: `active_buckets × 60 min` overstates throughput for sparse endpoints. Exact throughput requires adding a time-range filter to the paginated list API. | Add `startTime`/`endTime` params to `/api/analytics/apis`; update COUNT query together with data query (see code comment in `metrics_repo.rs`). |
| T2  | 🟡 Medium | **Ingest quota is per-instance**: With N instances a client can ingest N × quota events/day. Acceptable for abuse prevention; not suitable for billing. | Replace `DashMap` with Redis `INCR`/`EXPIRE` for cross-instance coordination. |
| T3  | 🟡 Medium | **ApiKeyCache is per-instance**: Revoked keys may work on other instances for up to the TTL window (60 s). | Publish revocation events to Redis Pub/Sub; each instance subscribes and calls `invalidate()`. |
| T4  | 🟢 Low   | **JWT permission propagation delay**: `can_view_analytics` revocation takes up to `JWT_EXPIRES_IN` to propagate. | Short-lived tokens + token blacklist (Redis SET with TTL) for immediate revocation. |
| T5  | 🟢 Low   | **`recentActivity` JSON field typo**: The `DashboardResponse` field is serialised as `"recentActitivy"` to match the original Node.js API contract. Fixing it is a breaking change. | Coordinate with all consumers; fix in next major API version bump. |

### Performance Headroom

The current architecture can sustain approximately:
- **Ingest:** ~50 000 events/s per `api-server` instance (limited by RabbitMQ publish throughput)
- **Analytics queries:** Limited by PostgreSQL; add `BRIN` indexes or TimescaleDB for >100M rows
- **API key validation:** Cache hit = ~0.1 ms; cache miss = MongoDB latency (~2–5 ms)
