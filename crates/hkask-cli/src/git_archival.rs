//! Git Archival Commands — GitHub MCP Integration
//!
//! Phase 9: Git archival via GitHub MCP tool calls.
//! G9: Each call mints a CapabilityToken via the ACP secret for OCAP accountability.
//! ℏKask - A Minimal Viable Container for Agents
//!
//! NOTE: MCP transport is not yet implemented (T16). These functions return
//! errors until the transport layer is wired up.

use hkask_mcp::runtime::McpRuntime;
use hkask_types::CapabilityChecker;

/// Archive registry to GitHub repository
pub async fn archive_registry_to_git(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    _repo_owner: &str,
    _repo_name: &str,
    _branch: &str,
    _path: &str,
    _content: &str,
) -> Result<String, String> {
    Err("MCP transport not yet implemented".to_string())
}

/// Restore registry from GitHub repository
pub async fn restore_registry_from_git(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    _repo_owner: &str,
    _repo_name: &str,
    _git_ref: &str,
    _target_path: &str,
) -> Result<String, String> {
    Err("MCP transport not yet implemented".to_string())
}

/// List archived registry versions
pub async fn list_registry_archives(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    _repo_owner: &str,
    _repo_name: &str,
) -> Result<Vec<String>, String> {
    Err("MCP transport not yet implemented".to_string())
}

/// Create registry snapshot (commit)
pub async fn create_registry_snapshot(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    _repo_owner: &str,
    _repo_name: &str,
    _message: &str,
) -> Result<String, String> {
    Err("MCP transport not yet implemented".to_string())
}
