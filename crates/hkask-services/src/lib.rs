//! hKask Service Layer — shared domain operations for CLI and API surfaces.
//!
//! This crate provides a unified service layer that eliminates duplication
//! between `hkask-cli` and `hkask-api`. Each surface composes a
//! `ServiceContext` (assembled once at startup) and delegates business logic
//! to domain service modules.
//!
//! # Architecture
//!
//! ```text
//! hkask-cli ──→ hkask-services ──→ hkask-agents
//! hkask-api  ──→ hkask-services ──→ hkask-cns
//!                                 ──→ hkask-memory
//!                                 ──→ hkask-templates
//!                                 ──→ hkask-types
//!                                 ──→ hkask-storage
//! ```
//!
//! Domain crates NEVER depend on `hkask-services`. Neither `hkask-cli` nor
//! `hkask-api` directly depend on domain crates for business operations.

pub mod config;
pub mod context;
pub mod error;
pub mod inference;

pub use config::ServiceConfig;
pub use context::ServiceContext;
pub use error::ServiceError;
pub use inference::{InferenceContext, InferenceService, ModelInfo};
