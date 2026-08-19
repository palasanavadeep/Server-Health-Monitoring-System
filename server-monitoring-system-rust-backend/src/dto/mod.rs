//! HTTP I/O contracts — request bodies and response shapes.
//!
//! # Structure
//! ```text
//! dto/
//! ├── request/   — Deserialized from incoming HTTP request bodies
//! └── response/  — Serialized into HTTP response data payloads
//! ```
//!
//! **Rule:** DTOs never import from `entity/`. Services map domain → response DTOs.

pub mod request;
pub mod response;
