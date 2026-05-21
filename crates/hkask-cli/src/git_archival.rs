//! Git Archival Commands — GitHub MCP Integration
//!
//! Phase 9: Git archival via GitHub MCP tool calls.
//! ℏKask v0.21.2

use hkask_mcp::runtime::McpRuntime;

/// Archive registry to GitHub repository
pub async fn archive_registry_to_git(
    runtime: &McpRuntime,
    repo_owner: &str,
    repo_name: &str,
    branch: &str,
    path: &str,
    content: &str,
) -> Result<String, String> {
    let result = runtime
        .call_tool(
            "github",
            "create_file",
            serde_json::json!({
                "owner": repo_owner,
                "repo": repo_name,
                "branch": branch,
                "path": path,
                "content": content,
            }),
        )
        .await
        .map_err(|e| format!("GitHub MCP call failed: {}", e))?;

    let sha = result.get("commit").and_then(|c| c.get("sha"))
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");

    Ok(format!("Archived to {}/{}/{} at SHA {}", repo_owner, repo_name, branch, sha))
}

/// Restore registry from GitHub repository
pub async fn restore_registry_from_git(
    runtime: &McpRuntime,
    repo_owner: &str,
    repo_name: &str,
    git_ref: &str,
    target_path: &str,
) -> Result<String, String> {
    runtime
        .call_tool(
            "git",
            "checkout",
            serde_json::json!({
                "repo": format!("{}/{}", repo_owner, repo_name),
                "ref": git_ref,
                "target": target_path,
            }),
        )
        .await
        .map_err(|e| format!("Git MCP call failed: {}", e))?;

    Ok(format!("Restored from {}/{}/{} to {}", repo_owner, repo_name, git_ref, target_path))
}

/// List archived registry versions
pub async fn list_registry_archives(
    runtime: &McpRuntime,
    repo_owner: &str,
    repo_name: &str,
) -> Result<Vec<String>, String> {
    let result = runtime
        .call_tool(
            "git",
            "log",
            serde_json::json!({
                "repo": format!("{}/{}", repo_owner, repo_name),
                "max_count": 10,
            }),
        )
        .await
        .map_err(|e| format!("Git MCP call failed: {}", e))?;

    let commits = result
        .get("commits")
        .and_then(|c| c.as_array())
        .map(|arr| arr.iter().filter_map(|c| c.get("sha").and_then(|s| s.as_str())).map(String::from).collect())
        .unwrap_or_default();

    Ok(commits)
}

/// Create registry snapshot (commit)
pub async fn create_registry_snapshot(
    runtime: &McpRuntime,
    repo_owner: &str,
    repo_name: &str,
    message: &str,
) -> Result<String, String> {
    let result = runtime
        .call_tool(
            "git",
            "commit",
            serde_json::json!({
                "repo": format!("{}/{}", repo_owner, repo_name),
                "message": message,
            }),
        )
        .await
        .map_err(|e| format!("Git MCP call failed: {}", e))?;

    let sha = result.get("sha").and_then(|s| s.as_str()).unwrap_or("unknown");
    Ok(format!("Created snapshot {} with message: {}", sha, message))
}
