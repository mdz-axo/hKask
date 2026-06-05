//! Git Archival Commands — Direct GitHub REST API Integration
//!
//! Implements registry archival operations using the GitHub Contents API
//! and Commits API directly, without MCP transport.
//!
//! ℏKask - A Minimal Viable Container for Agents

use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use hkask_mcp::runtime::McpRuntime;
use hkask_mcp::server::{api_get, api_put, resolve_credential};
use hkask_types::CapabilityChecker;
use serde_json::json;

const GITHUB_API_BASE: &str = "https://api.github.com";
const DEFAULT_REGISTRY_PATH: &str = "registry";

/// Build an authenticated reqwest::Client for GitHub API calls.
///
/// Resolves the GitHub token from keychain/env and sets default headers
/// (Authorization, Accept, User-Agent) following the same pattern as
/// `hkask-mcp-github`.
fn build_github_client() -> Result<reqwest::Client, String> {
    let token = resolve_credential("HKASK_GITHUB_TOKEN")
        .map_err(|e| format!("GitHub token not available: {e}"))?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github+json".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::USER_AGENT,
        "hKask-archival/0.22.0".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))
}

/// Archive registry to GitHub repository
///
/// Creates or updates a file in the repository using the GitHub Contents API.
/// If the file already exists, its SHA is fetched first for conflict detection.
pub async fn archive_registry_to_git(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    repo_owner: &str,
    repo_name: &str,
    branch: &str,
    path: &str,
    content: &str,
) -> Result<String, String> {
    let client = build_github_client()?;
    let encoded_content = BASE64_STANDARD.encode(content.as_bytes());

    // Get the current file SHA if it exists (required for updates)
    let file_url =
        format!("{GITHUB_API_BASE}/repos/{repo_owner}/{repo_name}/contents/{path}?ref={branch}");

    let current_sha = match api_get(&client, "github", &file_url).await {
        Ok(json) => json
            .get("sha")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
        Err(_) => None,
    };

    let url = format!("{GITHUB_API_BASE}/repos/{repo_owner}/{repo_name}/contents/{path}");

    let mut payload = json!({
        "message": format!("chore: archive registry to {path}"),
        "content": encoded_content,
        "branch": branch,
    });

    if let Some(sha) = current_sha {
        payload
            .as_object_mut()
            .unwrap()
            .insert("sha".to_string(), json!(sha));
    }

    let result = api_put(&client, "github", &url, &payload)
        .await
        .map_err(|e| format!("Failed to archive registry: {e}"))?;

    let commit_sha = result
        .get("commit")
        .and_then(|c| c.get("sha"))
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");

    Ok(format!("Archived to {path} (commit {commit_sha})"))
}

/// Restore registry from GitHub repository
///
/// Fetches file content from the repository using the GitHub Contents API.
/// The `target_path` parameter specifies the file path in the repo (defaults
/// to "registry" when called with ".").
pub async fn restore_registry_from_git(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    repo_owner: &str,
    repo_name: &str,
    git_ref: &str,
    target_path: &str,
) -> Result<String, String> {
    let client = build_github_client()?;

    let remote_path = if target_path == "." {
        DEFAULT_REGISTRY_PATH
    } else {
        target_path
    };

    let url = format!(
        "{GITHUB_API_BASE}/repos/{repo_owner}/{repo_name}/contents/{remote_path}?ref={git_ref}"
    );

    let json = api_get(&client, "github", &url)
        .await
        .map_err(|e| format!("Failed to fetch file: {e}"))?;

    let encoded = json
        .get("content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| "No content field in GitHub response".to_string())?;

    let decoded = BASE64_STANDARD
        .decode(encoded.trim())
        .map_err(|e| format!("Failed to decode base64 content: {e}"))?;

    String::from_utf8(decoded).map_err(|e| format!("Content is not valid UTF-8: {e}"))
}

/// List archived registry versions
///
/// Lists commits that touched the registry file using the GitHub Commits API.
pub async fn list_registry_archives(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    repo_owner: &str,
    repo_name: &str,
) -> Result<Vec<String>, String> {
    let client = build_github_client()?;

    let url = format!(
        "{GITHUB_API_BASE}/repos/{repo_owner}/{repo_name}/commits?path={DEFAULT_REGISTRY_PATH}"
    );

    let json = api_get(&client, "github", &url)
        .await
        .map_err(|e| format!("Failed to list archives: {e}"))?;

    let commits = json
        .as_array()
        .ok_or_else(|| "Expected array of commits".to_string())?;

    let shas: Vec<String> = commits
        .iter()
        .filter_map(|c| c.get("sha").and_then(|s| s.as_str()).map(|s| s.to_string()))
        .collect();

    Ok(shas)
}

/// Create registry snapshot (commit)
///
/// Reads the local registry database, serializes it to JSON, and pushes it
/// to GitHub as a snapshot commit using the Contents API.
pub async fn create_registry_snapshot(
    _runtime: &McpRuntime,
    _checker: &CapabilityChecker,
    repo_owner: &str,
    repo_name: &str,
    message: &str,
) -> Result<String, String> {
    let client = build_github_client()?;

    // Read the local registry and serialize to JSON
    let registry_content = read_local_registry()?;

    let encoded_content = BASE64_STANDARD.encode(registry_content.as_bytes());

    // Get the current file SHA if it exists (required for updates)
    let file_url = format!(
        "{GITHUB_API_BASE}/repos/{repo_owner}/{repo_name}/contents/{DEFAULT_REGISTRY_PATH}"
    );

    let current_sha = match api_get(&client, "github", &file_url).await {
        Ok(json) => json
            .get("sha")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
        Err(_) => None,
    };

    let mut payload = json!({
        "message": message,
        "content": encoded_content,
    });

    if let Some(sha) = current_sha {
        payload
            .as_object_mut()
            .unwrap()
            .insert("sha".to_string(), json!(sha));
    }

    let result = api_put(&client, "github", &file_url, &payload)
        .await
        .map_err(|e| format!("Failed to create snapshot: {e}"))?;

    let commit_sha = result
        .get("commit")
        .and_then(|c| c.get("sha"))
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");

    Ok(format!("Snapshot created (commit {commit_sha})"))
}

/// Read the local registry database and serialize it to JSON.
fn read_local_registry() -> Result<String, String> {
    use crate::commands::config::{registry_db_path, resolve_db_passphrase};
    use hkask_storage::{AgentRegistryStore, Database};

    let db_path = registry_db_path();
    let passphrase = resolve_db_passphrase().map_err(|e| format!("{e}"))?;

    let db = if db_path == ":memory:" {
        Database::in_memory()
    } else {
        Database::open(&db_path, &passphrase)
    }
    .map_err(|e| format!("Failed to open database: {e}"))?;

    let store = AgentRegistryStore::new(db.conn_arc());
    store
        .initialize_schema()
        .map_err(|e| format!("Failed to initialize schema: {e}"))?;

    let agents = store
        .list()
        .map_err(|e| format!("Failed to list agents: {e}"))?;

    serde_json::to_string_pretty(&agents).map_err(|e| format!("Failed to serialize registry: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Temporarily remove `HKASK_GITHUB_TOKEN` from the environment, run `f`,
    /// then restore the previous value (if any).
    ///
    /// Note: This only controls the env-var path of `resolve_credential`.
    /// If the OS keychain happens to have an entry for `HKASK_GITHUB_TOKEN`,
    /// the function will still succeed. The tests that use this helper are
    /// therefore specifically verifying the env-var failure path.
    fn without_github_token<T>(f: impl FnOnce() -> T) -> T {
        let saved = std::env::var("HKASK_GITHUB_TOKEN").ok();
        // SAFETY: Test-only helper that temporarily removes an env var and
        // restores it. Safe because: (1) no other thread reads this var at
        // test time, (2) we always restore the original value.
        unsafe { std::env::remove_var("HKASK_GITHUB_TOKEN") };
        let result = f();
        if let Some(val) = saved {
            // SAFETY: Same rationale — restoring the original value.
            unsafe { std::env::set_var("HKASK_GITHUB_TOKEN", val) };
        }
        result
    }

    /// Verify that `build_github_client()` returns a clear error when
    /// `HKASK_GITHUB_TOKEN` is not available via environment variable.
    ///
    /// This tests the env-var path only. If the OS keychain has a stored
    /// token for `HKASK_GITHUB_TOKEN`, the function may succeed despite the
    /// env var being absent — in that case the test will fail, indicating
    /// that a keychain entry is present.
    #[test]
    fn build_github_client_returns_error_without_token() {
        let result = without_github_token(|| build_github_client());

        match result {
            Err(msg) => {
                assert!(
                    msg.contains("GitHub token not available") || msg.contains("token"),
                    "Expected error about missing token, got: {msg}"
                );
            }
            Ok(_) => {
                // If this fires, the OS keychain has an entry for HKASK_GITHUB_TOKEN.
                // The env-var failure path is still correct — the keychain simply
                // provided a fallback. This is acceptable but worth noting.
                panic!(
                    "build_github_client() succeeded without HKASK_GITHUB_TOKEN env var. \
                     This likely means the OS keychain has a stored token. \
                     Remove the keychain entry or adjust this test."
                );
            }
        }
    }

    /// Verify that `archive_registry_to_git` returns an appropriate error
    /// when called without a valid GitHub client (no token available).
    ///
    /// Since `archive_registry_to_git` calls `build_github_client()` internally,
    /// it should fail fast with a clear error about the missing token.
    #[tokio::test]
    async fn archive_registry_to_git_requires_valid_client() {
        let saved = std::env::var("HKASK_GITHUB_TOKEN").ok();
        // SAFETY: Test-only — temporarily removes env var; restored below.
        unsafe { std::env::remove_var("HKASK_GITHUB_TOKEN") };

        let runtime = McpRuntime::new();
        let checker = CapabilityChecker::new(b"test");

        let result = archive_registry_to_git(
            &runtime, &checker, "owner", "repo", "main", "registry", "content",
        )
        .await;

        // Restore env var regardless of test outcome
        if let Some(val) = saved {
            // SAFETY: Restoring the original value after test completion.
            unsafe { std::env::set_var("HKASK_GITHUB_TOKEN", val) };
        }

        match result {
            Err(msg) => {
                assert!(
                    msg.contains("GitHub token not available") || msg.contains("token"),
                    "Expected error about missing token, got: {msg}"
                );
            }
            Ok(_) => {
                panic!(
                    "archive_registry_to_git() succeeded without HKASK_GITHUB_TOKEN env var. \
                     This likely means the OS keychain has a stored token. \
                     Remove the keychain entry or adjust this test."
                );
            }
        }
    }
}
