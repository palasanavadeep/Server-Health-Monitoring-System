//! Server Monitoring System — library crate.
//!
//! This crate is organised as a modular monolith so that any module can be
//! decoupled into a microservice with minimal friction.
//!
//! # Module tree
//! ```
//! server_monitoring
//! ├── app_state       — DI container (AppState)
//! ├── config/         — Settings, DB connections, telemetry
//! ├── domain/         — Pure business entities (User, Client, ApiKey, …)
//! ├── error/          — Centralised error type (AppError via thiserror)
//! ├── repository/     — Data access layer (trait + Mongo/PG implementations)
//! ├── service/        — Business logic (AuthService, ClientService, …)
//! ├── handler/        — HTTP request handlers (no business logic)
//! ├── router/         — Route registration
//! ├── middleware/     — actix-web middleware (JWT, rate-limit, …)
//! ├── messaging/      — RabbitMQ producer + consumer
//! ├── resilience/     — Circuit breaker + retry strategy
//! └── util/           — Shared utilities (response, security, IP, validation)
//! ```

pub mod app_state;

// Infrastructure
pub mod config;

// Core domain
pub mod domain;

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
