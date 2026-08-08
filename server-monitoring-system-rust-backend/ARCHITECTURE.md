# Architecture: Server Monitoring System (Rust)

## Overview

The backend is organized as a **modular monolith**. Each domain module is self-contained and has clearly defined boundaries, making it straightforward to extract any module into a microservice in the future without rewriting the core logic.

## Module Tree

```
src/
├── bin/
│   ├── api_server.rs       ← HTTP API server entry point
│   └── consumer.rs         ← Message consumer entry point
│
├── lib.rs                  ← Crate root / module declarations
├── app_state.rs            ← Dependency injection container (AppState)
│
├── config/                 ← Infrastructure setup
│   ├── settings.rs         ← Typed configuration from env vars (AppConfig)
│   ├── database.rs         ← MongoDB + PostgreSQL connection managers
│   ├── messaging.rs        ← RabbitMQ connection manager
│   └── telemetry.rs        ← Structured logging (tracing/tracing-subscriber)
│
├── domain/                 ← Pure business entities (no framework deps)
│   ├── role.rs             ← Role enum (SuperAdmin, ClientAdmin, ClientViewer)
│   ├── user.rs             ← User entity + JwtClaims + UserResponse DTO
│   ├── client.rs           ← Client organization entity
│   ├── api_key.rs          ← ApiKey entity + permissions + security sub-structs
│   ├── api_hit.rs          ← Raw API hit event entity
│   └── event.rs            ← Messaging event type constants
│
├── error/
│   └── app_error.rs        ← Centralized AppError (thiserror) + actix ResponseError
│
├── repository/             ← Data access layer
│   ├── user_repo.rs        ← UserRepository trait + MongoUserRepository
│   ├── client_repo.rs      ← ClientRepository trait + MongoClientRepository
│   ├── api_key_repo.rs     ← ApiKeyRepository trait + MongoApiKeyRepository
│   ├── api_hit_repo.rs     ← ApiHitRepository trait + MongoApiHitRepository
│   └── metrics_repo.rs     ← MetricsRepository trait + PgMetricsRepository
│
├── service/                ← Business logic (pure, no HTTP types)
│   ├── auth.rs             ← AuthService (registration, login, JWT)
│   ├── client.rs           ← ClientService (clients, users, API keys)
│   ├── ingest.rs           ← IngestService (validate + publish hits)
│   ├── analytics.rs        ← AnalyticsService (stats, dashboard, timeseries)
│   └── processor.rs        ← ProcessorService (consume + dual-write)
│
├── handler/                ← HTTP handlers (no business logic)
│   ├── health.rs           ← GET /health, GET /
│   ├── auth.rs             ← Auth endpoints
│   ├── client.rs           ← Client/API key endpoints
│   ├── ingest.rs           ← POST /api/hit/
│   └── analytics.rs        ← Analytics endpoints
│
├── router/
│   └── routes.rs           ← Central route registration (single source of truth)
│
├── middleware/
│   ├── authenticate.rs     ← JWT validation → injects AuthenticatedUser
│   ├── authorize.rs        ← Role-based access control
│   ├── rate_limiter.rs     ← In-memory rate limiting
│   ├── request_logger.rs   ← Structured request/response logging
│   └── validate_api_key.rs ← API key validation for ingest endpoint
│
├── messaging/
│   ├── producer.rs         ← EventProducer (publish to RabbitMQ)
│   └── consumer.rs         ← EventConsumer (consume from RabbitMQ)
│
├── resilience/
│   ├── circuit_breaker.rs  ← CircuitBreaker (Closed→Open→HalfOpen)
│   └── retry.rs            ← RetryStrategy (exponential backoff + jitter)
│
└── util/
    ├── response.rs         ← ResponseFormatter (standard API envelope)
    ├── security.rs         ← SecurityUtils (password validation)
    ├── ip.rs               ← IpUtils (CIDR matching)
    └── validation.rs       ← Request validation schemas (validator crate)
```

## Architectural Layers

```
┌─────────────────────────────────────────────────────────┐
│                     HTTP Handlers                        │
│            (handler/ — no business logic)                │
├─────────────────────────────────────────────────────────┤
│                    Service Layer                         │
│           (service/ — pure business logic)               │
├─────────────────────────────────────────────────────────┤
│                  Repository Layer                        │
│       (repository/ — trait + concrete impls)             │
├───────────────────────┬─────────────────────────────────┤
│       MongoDB         │         PostgreSQL               │
│  (raw events, users,  │   (aggregated metrics —          │
│   clients, API keys)  │    endpoint_metrics table)       │
└───────────────────────┴─────────────────────────────────┘
```

## Key Design Decisions

### Modular Monolith
All modules share a single deployment unit but have clean boundaries. To extract a module as a microservice: create a new crate, move the `service/` + `repository/` files, replace the `AppState` injection with an HTTP/gRPC client.

### Repository Pattern + Trait Abstractions
Every repository is defined as a `trait` (e.g. `UserRepository`) with a concrete implementation (e.g. `MongoUserRepository`). Services receive `Arc<dyn Trait>` allowing:
- Swap storage backends without touching services
- Use mock repositories in unit tests

### Role Enum (OOP: Encapsulation)
`Role` is a proper enum with `Display`, `FromStr`, and `serde` implementations. No more string comparisons in business logic — all comparisons use typed values.

### AppError via thiserror (OOP: Polymorphism)
`AppError` implements `actix_web::ResponseError` and provides typed constructors (`AppError::bad_request()`, `AppError::not_found()`, etc.). All error paths go through one place, guaranteeing consistent JSON error responses.

### Circuit Breaker + Retry (OOP: State Machine)
`CircuitBreaker` is a thread-safe state machine (Closed → Open → HalfOpen → Closed). `RetryStrategy` provides exponential backoff with jitter. Both are shared via `Arc` across producer and consumer.

### Dual-Write Pattern
`ProcessorService` first writes the raw event to MongoDB, then upserts aggregated metrics to PostgreSQL. If the metrics update fails after the raw write succeeds, it logs a non-fatal error rather than re-queuing the message, avoiding duplication.

### Builder Pattern via AppConfig
`AppConfig::from_env()` constructs the entire config from environment variables with sensible defaults, acting as a fluent builder for the config sub-structs (`MongoConfig`, `JwtConfig`, etc.).

## Data Flow

```
SDK / Client App
      │
      ▼  POST /api/hit/  (x-api-key)
┌─────────────┐
│ IngestSvc   │──→ Validate → Enrich → Publish → RabbitMQ queue
└─────────────┘
                                                      │
                                                      ▼
                                             ┌─────────────────┐
                                             │  EventConsumer  │
                                             │  (consumer bin) │
                                             └────────┬────────┘
                                                      │
                                          ┌───────────▼───────────┐
                                          │    ProcessorSvc       │
                                          │  1. Save → MongoDB    │
                                          │  2. Upsert → PgSQL    │
                                          └───────────────────────┘
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `NODE_ENV` | `development` | Environment name |
| `PORT` | `5000` | HTTP server port |
| `MONGO_URI` | `mongodb://localhost:27017/server_monitoring` | MongoDB connection string |
| `MONGO_DB_NAME` | `server_monitoring` | MongoDB database name |
| `PG_HOST` | `localhost` | PostgreSQL host |
| `PG_PORT` | `5432` | PostgreSQL port |
| `PG_DATABASE` | `server_monitoring` | PostgreSQL database |
| `PG_USER` | `postgres` | PostgreSQL user |
| `PG_PASSWORD` | `postgres` | PostgreSQL password |
| `RABBITMQ_URL` | `amqp://localhost:5672` | RabbitMQ URL |
| `RABBITMQ_QUEUE` | `server_hits` | Queue name |
| `JWT_SECRET` | — | JWT signing secret |
| `JWT_EXPIRES_IN` | `24h` | JWT expiry duration |
| `RATE_LIMIT_WINDOW_MS` | `900000` | Rate limit window |
| `RATE_LIMIT_MAX_REQUESTS` | `1000` | Max requests per window |
| `VALID_API_KEYS` | — | Comma-separated admin API keys |
