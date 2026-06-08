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
pub mod curator;
pub mod ensemble;
pub mod error;
pub mod inference;
pub mod pods;
pub mod sovereignty;

pub use config::ServiceConfig;
pub use config::{DEFAULT_DB_PATH, DEFAULT_OKAPI_BASE_URL};
pub use context::ServiceContext;
pub use curator::{CuratorContext, CuratorService, MetacognitionSummary};
pub use ensemble::{EnsembleContext, EnsembleService, map_participant_role};
pub use error::ServiceError;
pub use inference::{InferenceContext, InferenceService, ModelInfo};
pub use pods::{PodContext, PodService};
pub use sovereignty::{
    AccessCheck, SovereigntyContext, SovereigntyService, SovereigntyStatus, parse_data_category,
};
