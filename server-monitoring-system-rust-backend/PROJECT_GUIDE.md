# Project Guide: Server Monitoring System (Rust)

## Quick Start

```bash
# 1. Copy environment variables
cp .env.example .env

# 2. Run the API server
cargo run --bin api-server

# 3. Run the consumer (separate terminal)
cargo run --bin consumer

# 4. Build for production
cargo build --release
```

## Running with Docker (recommended)

```bash
docker-compose up --build
```

## Project Structure at a Glance

| Directory | Purpose |
|---|---|
| `src/bin/` | Binary entry points (api_server, consumer) |
| `src/domain/` | Business entities — edit here to add new fields |
| `src/repository/` | Database access — swap storage by implementing the trait |
| `src/service/` | Business logic — all validation and orchestration lives here |
| `src/handler/` | HTTP layer — thin, just call service + format response |
| `src/router/routes.rs` | **Single file** that owns all route registrations |
| `src/middleware/` | Cross-cutting concerns (auth, rate-limit, logging) |
| `src/messaging/` | RabbitMQ producer + consumer |
| `src/resilience/` | Circuit breaker + retry strategy |
| `src/util/` | Shared helpers (response, security, IP, validation) |
| `src/config/` | Typed configuration + DB/MQ connection setup |
| `src/error/app_error.rs` | **One** error type used everywhere |

## How to Add a New Feature

### 1. Add a domain entity

Create `src/domain/my_entity.rs` and register it in `src/domain/mod.rs`:

```rust
// src/domain/my_entity.rs
use serde::{Deserialize, Serialize};
use bson::oid::ObjectId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyEntity {
    #[serde(rename = "_id")]
    pub id: Option<ObjectId>,
    pub name: String,
}
```

### 2. Add a repository

Create `src/repository/my_repo.rs` with a trait + MongoDB implementation:

```rust
#[async_trait]
pub trait MyRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<MyEntity>, AppError>;
}

pub struct MongoMyRepository { collection: mongodb::Collection<MyEntity> }

#[async_trait]
impl MyRepository for MongoMyRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<MyEntity>, AppError> {
        // ...
    }
}
```

Register in `src/repository/mod.rs`:
```rust
pub mod my_repo;
```

### 3. Add a service

```rust
pub struct MyService {
    repo: Arc<dyn MyRepository>,
}
impl MyService {
    pub fn new(repo: Arc<dyn MyRepository>) -> Self { Self { repo } }
    pub async fn do_something(&self) -> Result<MyEntity, AppError> { /* ... */ }
}
```

Register in `src/service/mod.rs`:
```rust
pub mod my_service;
```

### 4. Add a handler

```rust
pub async fn my_handler(state: web::Data<AppState>) -> HttpResponse {
    match state.my_service.do_something().await {
        Ok(data) => HttpResponse::Ok().json(ResponseFormatter::success(
            serde_json::to_value(data).unwrap(),
            "Success", 200,
        )),
        Err(e) => e.to_response(),
    }
}
```

### 5. Register the route

Open `src/router/routes.rs` and add:

```rust
cfg.route("/api/my-resource", web::get().to(handler::my_handler));
```

### 6. Wire into AppState

Add the service field to `src/app_state.rs` and inject in `src/bin/api_server.rs`.

---

## How to Add a New Role

1. Add the variant to `Role` enum in `src/domain/role.rs`:
   ```rust
   pub enum Role {
       SuperAdmin,
       ClientAdmin,
       ClientViewer,
       NewRole,  // ← add here
   }
   ```
2. Add the `as_str()` match arm and serde rename attribute.
3. Update `CLIENT_ROLES` if it should be assignable to client users.

---

## How to Decouple a Module into a Microservice

Because the project is a **modular monolith**, the boundary work is already done:

1. Create a new Rust crate/workspace member.
2. Move `src/domain/<entity>.rs`, `src/repository/<repo>.rs`, and `src/service/<service>.rs`.
3. In the main monolith, replace the direct `Arc<dyn Repo>` injection with an HTTP/gRPC client that implements the same repository trait.
4. The handlers and router don't change.

---

## API Reference

### Authentication

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/api/auth/onboard-super-admin` | None | Create first super admin |
| `POST` | `/api/auth/login` | None | Login, sets `authToken` cookie |
| `GET` | `/api/auth/logout` | None | Clears `authToken` cookie |
| `GET` | `/api/auth/profile` | JWT | Get current user profile |
| `PUT` | `/api/auth/profile` | JWT | Update profile |
| `POST` | `/api/auth/register` | JWT (SuperAdmin) | Create new user |
| `PATCH` | `/api/auth/users/{userId}/deactivate` | JWT | Deactivate a user |

### Client Management

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/api/admin/clients/onboard` | JWT (SuperAdmin) | Create client |
| `POST` | `/api/admin/clients/{clientId}/users` | JWT | Create client user |
| `POST` | `/api/admin/clients/{clientId}/api/keys` | JWT | Create API key |
| `GET` | `/api/admin/clients/{clientId}/api/keys` | JWT | List API keys |
| `PUT` | `/api/admin/clients/{clientId}/api/keys/{keyId}` | JWT | Update API key |
| `DELETE` | `/api/admin/clients/{clientId}/api/keys/{keyId}` | JWT | Delete API key |
| `PATCH` | `/api/admin/clients/{clientId}/api/keys/{keyId}/activate` | JWT | Activate key |
| `PATCH` | `/api/admin/clients/{clientId}/api/keys/{keyId}/deactivate` | JWT | Deactivate key |
| `POST` | `/api/admin/clients/{clientId}/api/keys/{keyId}/rotate` | JWT | Rotate key value |

### Ingest

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/api/hit/` | `x-api-key` header | Ingest an API hit event |

### Analytics

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/api/analytics/stats` | JWT | Overall statistics |
| `GET` | `/api/analytics/dashboard` | JWT | Dashboard data |
| `GET` | `/api/analytics/apis` | JWT | Paginated per-endpoint metrics |

### Utility

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/health` | None | Health check |
| `GET` | `/` | None | Service info |

---

## Error Response Format

All errors return a consistent envelope:

```json
{
  "success": false,
  "message": "User not found",
  "error": null,
  "statusCode": 404,
  "timestamp": "2026-08-06T10:00:00.000Z"
}
```

## Success Response Format

```json
{
  "success": true,
  "message": "Profile retrieved successfully",
  "data": { "...": "..." },
  "statusCode": 200,
  "timestamp": "2026-08-06T10:00:00.000Z"
}
```

## Testing

```bash
# Run all tests
cargo test

# Run library tests only
cargo test --lib

# Run with output
cargo test -- --nocapture
```

## Key Design Patterns in Use

| Pattern | Location | Purpose |
|---|---|---|
| Repository Pattern | `repository/` | Decouple storage from business logic |
| Service Layer | `service/` | Encapsulate business rules |
| Dependency Injection | `app_state.rs` | `Arc<dyn Trait>` passed via `web::Data` |
| Circuit Breaker | `resilience/circuit_breaker.rs` | Protect downstream services |
| Retry with Backoff | `resilience/retry.rs` | Handle transient failures |
| Builder Pattern | `config/settings.rs` | `AppConfig::from_env()` |
| State Machine | `resilience/circuit_breaker.rs` | Closed/Open/HalfOpen states |
| DTO Pattern | `domain/user.rs` | `User` → `UserResponse` (strips password) |
| Dual-Write | `service/processor.rs` | Atomic-ish write to Mongo + PG |
