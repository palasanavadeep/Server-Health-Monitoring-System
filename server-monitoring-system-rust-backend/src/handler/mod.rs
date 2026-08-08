//! HTTP request handlers — receive requests, delegate to services, return responses.
//!
//! Handlers contain no business logic. They extract request data,
//! call the appropriate service, and format the HTTP response.

pub mod analytics;
pub mod auth;
pub mod client;
pub mod health;
pub mod ingest;
