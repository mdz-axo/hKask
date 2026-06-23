//! hKask Service-Layer Foundation — shared error types, configuration, and settings.
//!
//! This crate provides the universal foundation that all other service-layer
//! modules depend on. Extracted from `hkask-services` to enable parallel
//! compilation and clear architectural boundaries.
//!
//! # Modules
//!
//! - `error` — `ServiceError` enum composing all domain error types
//! - `config` — `ServiceConfig` resolved once at startup
//! - `settings` — `HkaskSettings` and canonical settings path

pub mod config;
pub mod error;
pub mod goal;
pub mod identity;
pub mod self_heal;
pub mod settings;

pub use config::{DEFAULT_DB_PATH, ServiceConfig};
pub use error::ServiceError;
pub use goal::{Goal, GoalArtifact, GoalCriterion, GoalState, IllegalGoalTransition};
pub use identity::{
    HumanUser, Invite, InviteStatus, OAuthProvider, RegistrationError, RegistrationRequest,
    ReplicantIdentity, Role, UserSession,
};
pub use settings::{HkaskSettings, load_settings, save_settings, settings_path};
