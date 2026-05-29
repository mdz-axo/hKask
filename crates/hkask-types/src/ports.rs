//! Hexagonal port traits — Infrastructure abstractions
//!
//! Port traits that enable crates to depend on abstractions
//! rather than concrete implementations in hkask-agents.
//! This inverts dependency direction: MCP (infrastructure) should
//! not depend on agents (orchestration).

use crate::error::GitError;
use crate::template::TemplateCrate;

/// Git CAS Port — Hexagonal boundary for template crate loading
///
/// Implementations:
/// - `GitCasAdapter` — Production adapter using gix (in hkask-agents)
pub trait GitCASPort: Send + Sync {
    /// Load a template crate from the content-addressable store
    fn load_template_crate(&self, crate_name: &str) -> Result<TemplateCrate, GitError>;

    /// Resolve the current SHA for a crate
    fn resolve_sha(&self, crate_name: &str) -> Result<String, GitError>;
}
