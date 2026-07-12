//! BundleManifest type system — skill bundling for hKask
//!
//! Re-export facade. Submodules organized by concern:
//! - `manifest`: BundleManifest, BundleManifestStep, BundleSkill, SkillPolarity, ValidationResult
//! - `config`: ConvergenceConfig, BundleGasConfig, ErrorHandlingConfig, OcapConfig, BundleCnsConfig, BundleAuditConfig
//! - `composition`: BundleComplementarity, BundleConflict, ConflictType, ConflictResolution, ComplementarityType
//! - `cascade`: CascadePhase

pub mod cascade;
pub mod composition;
pub mod config;
pub mod manifest;

pub use composition::*;
pub use config::*;
pub use manifest::{BundleManifest, BundleManifestStep, BundleSkill};
// cascade types are pub(crate) — not re-exported

/// CRUD for bundle manifests. Read methods return owned values for HashMap/SQLite compat.
pub trait BundleRegistryIndex {
    fn register_bundle(&mut self, bundle: BundleManifest);
    fn get_bundle(&self, id: &str) -> Option<BundleManifest>;
    fn list_bundles(&self) -> Vec<BundleManifest>;
    fn remove_bundle(&mut self, id: &str) -> Option<BundleManifest>;
    fn find_bundle_by_skills(&self, skill_ids: &[String]) -> Option<BundleManifest>;
}
