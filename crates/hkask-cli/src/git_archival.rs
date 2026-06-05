//! Git Archival Commands — GitHub MCP Integration
//!
//! Phase 9: Git archival via GitHub MCP tool calls.
//! G9: Each call mints a CapabilityToken via the ACP secret for OCAP accountability.
//! ℏKask - A Minimal Viable Container for Agents
//!
//! TODO: MCP transport not yet implemented. These stubs return errors until the
//! transport layer is wired up. Scope: implement HTTP transport for GitHub API calls
//! (create/update file, get file content, list commits) with capability token minting.

use hkask_mcp::runtime::McpRuntime;
use hkask_types::CapabilityChecker;

/// Archive registry to GitHub repository
///
/// TODO: Wire MCP transport to call GitHub's CreateOrUpdateFile API.
pub async fn archive_registry_to_git(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    _repo_owner: &str,
    _repo_name: &str,
    _branch: &str,
    _path: &str,
    _content: &str,
) -> Result<String, String> {
    Err("Git archival not yet implemented — MCP transport pending".to_string())
}

/// Restore registry from GitHub repository
///
/// TODO: Wire MCP transport to call GitHub's GetFileContent API.
pub async fn restore_registry_from_git(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    _repo_owner: &str,
    _repo_name: &str,
    _git_ref: &str,
    _target_path: &str,
) -> Result<String, String> {
    Err("Git restore not yet implemented — MCP transport pending".to_string())
}

/// List archived registry versions
///
/// TODO: Wire MCP transport to call GitHub's ListCommits API.
pub async fn list_registry_archives(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    _repo_owner: &str,
    _repo_name: &str,
) -> Result<Vec<String>, String> {
    Err("Git archive listing not yet implemented — MCP transport pending".to_string())
}

/// Create registry snapshot (commit)
///
/// TODO: Wire MCP transport to call GitHub's CreateOrUpdateFile API with snapshot metadata.
pub async fn create_registry_snapshot(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    _repo_owner: &str,
    _repo_name: &str,
    _message: &str,
) -> Result<String, String> {
    Err("Git snapshot not yet implemented — MCP transport pending".to_string())
}
