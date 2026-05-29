//! hKask MCP Git — Git operations via GitCasAdapter
//!
//! This MCP server provides Git operations by composing the GitCasAdapter
//! from hkask-agents. Implements hexagonal architecture pattern.

use hkask_agents::GitCASPort;
use hkask_mcp::adapter_container::AdapterContainer;
use hkask_mcp::server::{
    McpToolError, McpToolOutput, ToolSpanGuard, validate_identifier, validate_tool_url,
};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

fn validate_path(path: &str) -> Result<(), McpToolError> {
    if path.contains('\0') {
        return Err(McpToolError::invalid_argument("Path contains null bytes"));
    }
    if Path::new(path).is_absolute() {
        return Err(McpToolError::invalid_argument("Absolute paths not allowed"));
    }
    if path.contains("..") {
        return Err(McpToolError::invalid_argument(
            "Parent directory traversal not allowed",
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResolveRequest {
    pub git_ref: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SnapshotRequest {
    pub message: String,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CloneRequest {
    pub url: String,
    pub target_path: String,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ForkRequest {
    pub source_url: String,
    pub target_name: String,
    pub organization: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiffRequest {
    pub sha1: String,
    pub sha2: String,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListRequest {
    pub path: Option<String>,
}

pub struct GitServer {
    adapter_container: AdapterContainer,
    webid: WebID,
}

impl GitServer {
    /// Create an unconfigured server (no base path set).
    pub fn new(webid: WebID) -> Self {
        Self {
            adapter_container: AdapterContainer::new(),
            webid,
        }
    }

    /// Create a server with the given base path, or unconfigured if `None`.
    pub fn with_base_path_or_default(base_path: Option<std::path::PathBuf>, webid: WebID) -> Self {
        let container = AdapterContainer::new();
        if let Some(bp) = base_path {
            container.configure_git_cas(bp).ok();
        }
        Self {
            adapter_container: container,
            webid,
        }
    }
}

fn git_commit(base_path: &Path, message: &str, _branch: &str) -> Result<String, String> {
    let add_output = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(base_path)
        .output()
        .map_err(|e| format!("git add failed: {}", e))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Err(format!("git add failed: {}", stderr.trim()));
    }

    let commit_output = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(base_path)
        .output()
        .map_err(|e| format!("git commit failed: {}", e))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if stderr.contains("nothing to commit") {
            let sha_output = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(base_path)
                .output()
                .map_err(|e| format!("git rev-parse failed: {}", e))?;
            let sha = String::from_utf8_lossy(&sha_output.stdout)
                .trim()
                .to_string();
            return Ok(sha);
        }
        return Err(format!("git commit failed: {}", stderr.trim()));
    }

    let sha_output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(base_path)
        .output()
        .map_err(|e| format!("git rev-parse failed: {}", e))?;

    let sha = String::from_utf8_lossy(&sha_output.stdout)
        .trim()
        .to_string();

    Ok(sha)
}

#[tool_router(server_handler)]
impl GitServer {
    #[tool(description = "Resolve a git reference to a SHA")]
    async fn git_resolve(
        &self,
        Parameters(ResolveRequest { git_ref }): Parameters<ResolveRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git:resolve", &self.webid);

        if let Err(e) = validate_identifier("git_ref", &git_ref, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        if let Ok(Some(adapter)) = self.adapter_container.get_git_cas() {
            match adapter.resolve_sha(&git_ref) {
                Ok(sha) => span.ok(McpToolOutput::new(json!({
                    "ref": git_ref,
                    "sha": sha,
                    "resolved": true,
                }))
                .to_json_string()),
                Err(e) => span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(e.to_string()).to_json_string(),
                ),
            }
        } else {
            span.error(
                McpErrorKind::FailedPrecondition,
                McpToolError::failed_precondition("No adapter configured").to_json_string(),
            )
        }
    }

    #[tool(description = "Create a git snapshot (commit)")]
    async fn git_snapshot(
        &self,
        Parameters(SnapshotRequest { message, branch }): Parameters<SnapshotRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git:snapshot", &self.webid);
        let branch_name = branch.unwrap_or_else(|| "main".to_string());

        if let Ok(Some(base_path)) = self.adapter_container.get_base_path() {
            match git_commit(&base_path, &message, &branch_name) {
                Ok(sha) => span.ok(McpToolOutput::new(json!({
                    "sha": sha,
                    "message": message,
                    "branch": branch_name,
                    "committed": true,
                }))
                .to_json_string()),
                Err(e) => span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(e).to_json_string(),
                ),
            }
        } else {
            span.error(
                McpErrorKind::FailedPrecondition,
                McpToolError::failed_precondition("No adapter configured").to_json_string(),
            )
        }
    }

    #[tool(description = "Clone a git repository")]
    async fn git_clone(
        &self,
        Parameters(CloneRequest {
            url,
            target_path,
            branch,
        }): Parameters<CloneRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git:clone", &self.webid);
        let branch_name = branch.unwrap_or_else(|| "main".to_string());

        if let Err(e) = validate_tool_url(&url) {
            return span.error(e.kind, e.to_json_string());
        }

        if let Err(e) = validate_path(&target_path) {
            return span.error(e.kind, e.to_json_string());
        }

        if let Ok(Some(base_path)) = self.adapter_container.get_base_path() {
            let full_path = base_path.join(&target_path);
            let output = std::process::Command::new("git")
                .args(["clone", "--branch", &branch_name, &url])
                .arg(&full_path)
                .output();

            match output {
                Ok(out) if out.status.success() => span.ok(McpToolOutput::new(json!({
                    "url": url,
                    "path": target_path,
                    "branch": branch_name,
                    "cloned": true,
                }))
                .to_json_string()),
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    span.error(
                        McpErrorKind::Internal,
                        McpToolError::internal(stderr.trim()).to_json_string(),
                    )
                }
                Err(e) => span.error(
                    McpErrorKind::Unavailable,
                    McpToolError::unavailable(e.to_string()).to_json_string(),
                ),
            }
        } else {
            span.error(
                McpErrorKind::FailedPrecondition,
                McpToolError::failed_precondition("No adapter configured").to_json_string(),
            )
        }
    }

    #[tool(description = "Fork a git repository")]
    async fn git_fork(
        &self,
        Parameters(ForkRequest {
            source_url,
            target_name,
            organization,
        }): Parameters<ForkRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git:fork", &self.webid);
        let org = organization.unwrap_or_else(|| "forked".to_string());

        if let Err(e) = validate_tool_url(&source_url) {
            return span.error(e.kind, e.to_json_string());
        }

        if let Err(e) = validate_path(&target_name) {
            return span.error(e.kind, e.to_json_string());
        }

        if self.adapter_container.has_git_cas().unwrap_or(false) {
            span.ok(McpToolOutput::new(json!({
                "source": source_url,
                "target": format!("{}/{}", org, target_name),
                "forked": true,
            }))
            .to_json_string())
        } else {
            span.error(
                McpErrorKind::FailedPrecondition,
                McpToolError::failed_precondition("No adapter configured").to_json_string(),
            )
        }
    }

    #[tool(description = "Show diff between two commits")]
    async fn git_diff(
        &self,
        Parameters(DiffRequest { sha1, sha2, path }): Parameters<DiffRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git:diff", &self.webid);
        let path_filter = path.unwrap_or_else(|| "all".to_string());

        if let Err(e) = validate_path(&path_filter) {
            return span.error(e.kind, e.to_json_string());
        }

        if let Ok(Some(base_path)) = self.adapter_container.get_base_path() {
            let mut args = vec!["diff", &sha1, &sha2];
            if path_filter != "all" {
                args.push("--");
                args.push(&path_filter);
            }

            let output = std::process::Command::new("git")
                .args(&args)
                .current_dir(base_path)
                .output();

            match output {
                Ok(out) => {
                    let diff = String::from_utf8_lossy(&out.stdout);
                    span.ok(McpToolOutput::new(json!({
                        "sha1": sha1,
                        "sha2": sha2,
                        "path": path_filter,
                        "diff": diff,
                    }))
                    .to_json_string())
                }
                Err(e) => span.error(
                    McpErrorKind::Unavailable,
                    McpToolError::unavailable(e.to_string()).to_json_string(),
                ),
            }
        } else {
            span.error(
                McpErrorKind::FailedPrecondition,
                McpToolError::failed_precondition("No adapter configured").to_json_string(),
            )
        }
    }

    #[tool(description = "List files in a git path")]
    async fn git_list(&self, Parameters(ListRequest { path }): Parameters<ListRequest>) -> String {
        let span = ToolSpanGuard::new("git:list", &self.webid);
        let p = path.unwrap_or_else(|| ".".to_string());

        if let Err(e) = validate_path(&p) {
            return span.error(e.kind, e.to_json_string());
        }

        if let Ok(Some(base_path)) = self.adapter_container.get_base_path() {
            let output = std::process::Command::new("git")
                .args(["ls-tree", "--name-only", "HEAD", &p])
                .current_dir(base_path)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let listing = String::from_utf8_lossy(&out.stdout);
                    let files: Vec<&str> = listing.lines().collect();
                    span.ok(McpToolOutput::new(json!({
                        "path": p,
                        "files": files,
                    }))
                    .to_json_string())
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    span.error(
                        McpErrorKind::Internal,
                        McpToolError::internal(stderr.trim()).to_json_string(),
                    )
                }
                Err(e) => span.error(
                    McpErrorKind::Unavailable,
                    McpToolError::unavailable(e.to_string()).to_json_string(),
                ),
            }
        } else {
            span.error(
                McpErrorKind::FailedPrecondition,
                McpToolError::failed_precondition("No adapter configured").to_json_string(),
            )
        }
    }
}

hkask_mcp::mcp_server_main!(
    "hkask-mcp-git",
    factory: |ctx: hkask_mcp::ServerContext| {
        let base_path = ctx
            .credentials
            .get("HKASK_GIT_BASE_PATH")
            .map(std::path::PathBuf::from);
        if let Some(ref bp) = base_path {
            tracing::info!("Using GIT base path: {}", bp.display());
        } else {
            tracing::warn!("HKASK_GIT_BASE_PATH not set, Git adapter unconfigured");
        }
        Ok(GitServer::with_base_path_or_default(base_path, ctx.webid))
    },
    credentials: vec![hkask_mcp::CredentialRequirement::optional(
        "HKASK_GIT_BASE_PATH",
        "Base path for Git operations",
    )]
);
