//! Server Monitoring System — library crate.
//!
//! Organised as a clean modular monolith. Each layer has a single responsibility:
//!
//! ```text
//! server_monitoring
//! ├── config/      — Settings, DB connections, telemetry
//! ├── domain/      — Pure business entities (User, Client, ApiKey…)
//! ├── dto/         — HTTP I/O: request bodies + typed response shapes
//! ├── entity/      — SeaORM table models (PostgreSQL, never cross service boundary)
//! ├── error/       — Centralised AppError (thiserror)
//! ├── repository/  — Data access traits + Mongo / SeaORM implementations
//! ├── service/     — Business logic
//! ├── handler/     — Thin HTTP handlers (no business logic)
//! ├── router/      — Route registration
//! ├── middleware/  — actix-web middleware (JWT, rate-limit…)
//! ├── messaging/   — RabbitMQ producer + consumer
//! ├── cache/       — In-process caches (API key TTL, tenant config, quota tracker)
//! ├── resilience/  — Circuit breaker + retry strategy
//! └── util/        — Shared utilities (response formatter, security, IP)
//! ```

pub mod app_state;

// Infrastructure
pub mod cache;
pub mod config;

// Core domain — business entities only
pub mod domain;

// I/O contracts — HTTP request bodies + response shapes
pub mod dto;

// DB table models — SeaORM entities (PostgreSQL)
pub mod entity;

// Error handling
pub mod error;

// Data access
pub mod repository;

// Business logic
pub mod service;

// HTTP layer
pub mod handler;
pub mod middleware;
pub mod router;

// Messaging
pub mod messaging;

// Resilience primitives
pub mod resilience;

// Utilities
pub mod util;
