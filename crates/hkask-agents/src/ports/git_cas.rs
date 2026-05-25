//! Git CAS Port — Hexagonal boundary for template crate loading

use crate::pod::TemplateCrate;

/// Port trait for Git CAS operations
///
/// Implementations:
/// - `GitCasAdapter` — Production adapter using gix
/// - `MockGitCas` — Testing adapter
pub trait GitCASPort: Send + Sync {
    fn load_template_crate(
        &self,
        crate_name: &str,
    ) -> Result<TemplateCrate, crate::error::GitError>;

    fn resolve_sha(&self, crate_name: &str) -> Result<String, crate::error::GitError>;
}
