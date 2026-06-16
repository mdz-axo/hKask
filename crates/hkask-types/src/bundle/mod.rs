//! BundleManifest type system — skill bundling for hKask
//!
//! Re-export facade. Submodules organized by concern:
//! - `manifest`: BundleManifest, BundleManifestStep, BundleSkill, SkillPolarity, ValidationResult
//! - `config`: ConvergenceConfig, GasConfig, ErrorHandlingConfig, OcapConfig, CnsConfig, AuditConfig
//! - `composition`: BundleComplementarity, BundleConflict (enum types are pub(crate))
//! - `cascade`: CascadePhase (pub(crate))

pub mod cascade;
pub mod composition;
pub mod config;
pub mod manifest;

pub use composition::*;
pub use config::*;
pub use manifest::*;
// cascade types are pub(crate) — not re-exported
