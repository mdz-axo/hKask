//! hKask MCP Git — Git operations via GitCASPort
//!
//! This MCP server provides Git operations by composing the GitCASPort
//! trait implementation (GixCasAdapter). All operations route through
//! the hexagonal port, not through raw shell commands.

use hkask_mcp::GixCasAdapter;
use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_types::WebID;
use hkask_types::ports::git_cas::{GitCASPort, GitCasError, RepoId};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

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

/// Parse a RepoId from a string, returning a default if empty.
fn parse_repo_id(repo: &str) -> RepoId {
    match repo {
        "registry" | "" => RepoId::Registry,
        "memory" => RepoId::Memory,
        "cns-audit" => RepoId::CnsAudit,
        "sovereignty" => RepoId::Sovereignty,
        "goals-specs" => RepoId::GoalsSpecs,
        "sessions" => RepoId::Sessions,
        "vault" => RepoId::Vault,
        _ => RepoId::Registry, // fallback
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResolveRequest {
    pub git_ref: String,
    #[serde(default)]
    pub repo: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SnapshotRequest {
    pub message: String,
    #[serde(default)]
    pub repo: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiffRequest {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub repo: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListRequest {
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub repo: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VerifyRequest {
    #[serde(default)]
    pub repo: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LogRequest {
    #[serde(default = "default_max_count")]
    pub max_count: usize,
    #[serde(default)]
    pub repo: String,
}

fn default_max_count() -> usize {
    20
}

pub struct GitServer {
    port: Arc<dyn GitCASPort>,
    webid: WebID,
}

impl GitServer {
    /// Create a server using the default GixCasAdapter resolved from environment.
    pub fn from_env(webid: WebID) -> Result<Self, GitCasError> {
        let adapter = GixCasAdapter::from_env()?;
        Ok(Self {
            port: Arc::new(adapter),
            webid,
        })
    }

    /// Create a server with a specific base path for the CAS adapter.
    pub fn with_base_path(base_path: std::path::PathBuf, webid: WebID) -> Self {
        let adapter = GixCasAdapter::new(base_path).unwrap_or_else(|_| {
            // Fall back to from_env if the path doesn't work
            GixCasAdapter::from_env().expect("Failed to create GixCasAdapter")
        });
        Self {
            port: Arc::new(adapter),
            webid,
        }
    }

    /// Create a server with an injected GitCASPort (for testing).
    pub fn with_port(port: Arc<dyn GitCASPort>, webid: WebID) -> Self {
        Self { port, webid }
    }
}

#[tool_router(server_handler)]
impl GitServer {
    #[tool(description = "Resolve a git reference to a SHA")]
    async fn git_resolve(
        &self,
        Parameters(ResolveRequest { git_ref, repo }): Parameters<ResolveRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git_resolve", &self.webid);

        validate_field!(span, "git_ref", &git_ref, 256);

        let repo_id = parse_repo_id(&repo);
        match self.port.resolve_ref(&repo_id, &git_ref).await {
            Ok(commit) => span.ok_json(json!({
                "ref": git_ref,
                "sha": commit.to_string(),
                "repo": repo_id.dir_name(),
                "resolved": true,
            })),
            Err(e) => span.internal_error(json!({"error": e.to_string()})),
        }
    }

    #[tool(description = "Create a git snapshot (commit)")]
    async fn git_snapshot(
        &self,
        Parameters(SnapshotRequest { message, repo }): Parameters<SnapshotRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git_snapshot", &self.webid);

        validate_field!(span, "message", &message, 1024);

        let repo_id = parse_repo_id(&repo);
        match self.port.snapshot(&repo_id, &message).await {
            Ok(commit) => span.ok_json(json!({
                "sha": commit.to_string(),
                "message": message,
                "repo": repo_id.dir_name(),
                "committed": true,
            })),
            Err(e) => span.internal_error(json!({"error": e.to_string()})),
        }
    }

    #[tool(description = "Show diff between two commits")]
    async fn git_diff(
        &self,
        Parameters(DiffRequest { from, to, repo }): Parameters<DiffRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git_diff", &self.webid);

        validate_field!(span, "from", &from, 64);
        validate_field!(span, "to", &to, 64);

        let repo_id = parse_repo_id(&repo);
        match self.port.diff(&repo_id, &from, &to).await {
            Ok(diffs) => {
                let diff_entries: Vec<serde_json::Value> = diffs
                    .iter()
                    .map(|d| {
                        json!({
                            "path": d.path,
                            "kind": format!("{:?}", d.kind),
                            "content": d.content,
                        })
                    })
                    .collect();
                span.ok_json(json!({
                    "from": from,
                    "to": to,
                    "repo": repo_id.dir_name(),
                    "diffs": diff_entries,
                }))
            }
            Err(e) => span.internal_error(json!({"error": e.to_string()})),
        }
    }

    #[tool(description = "List files in a git tree")]
    async fn git_list(
        &self,
        Parameters(ListRequest { prefix, repo }): Parameters<ListRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git_list", &self.webid);

        let prefix = if prefix.is_empty() { "" } else { &prefix };
        if let Err(e) = validate_path(prefix) {
            return span.error(e.kind, e.to_json_string());
        }

        let repo_id = parse_repo_id(&repo);
        match self.port.list_tree(&repo_id, "HEAD", prefix).await {
            Ok(entries) => {
                let files: Vec<serde_json::Value> = entries
                    .iter()
                    .map(|e| {
                        json!({
                            "path": e.path,
                            "hash": e.content_hash.to_string(),
                            "kind": format!("{:?}", e.kind),
                        })
                    })
                    .collect();
                span.ok_json(json!({
                    "prefix": prefix,
                    "repo": repo_id.dir_name(),
                    "files": files,
                }))
            }
            Err(e) => span.internal_error(json!({"error": e.to_string()})),
        }
    }

    #[tool(description = "Verify content integrity of a repository")]
    async fn git_verify(
        &self,
        Parameters(VerifyRequest { repo }): Parameters<VerifyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git_verify", &self.webid);

        let repo_id = parse_repo_id(&repo);
        match self.port.verify(&repo_id).await {
            Ok(report) => span.ok_json(json!({
                "repo": report.repo.dir_name(),
                "total_blobs": report.total_blobs,
                "verified_blobs": report.verified_blobs,
                "corrupt_hashes": report.corrupt_hashes.iter().map(|h| h.to_string()).collect::<Vec<_>>(),
                "integrity": report.corrupt_hashes.is_empty(),
            })),
            Err(e) => span.internal_error(json!({"error": e.to_string()})),
        }
    }

    #[tool(description = "List snapshot history for a repository")]
    async fn git_log(
        &self,
        Parameters(LogRequest { max_count, repo }): Parameters<LogRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("git_log", &self.webid);

        let repo_id = parse_repo_id(&repo);
        match self.port.log(&repo_id, max_count).await {
            Ok(entries) => {
                let logs: Vec<serde_json::Value> = entries
                    .iter()
                    .map(|e| {
                        json!({
                            "commit": e.commit.to_string(),
                            "message": e.message,
                            "timestamp": e.timestamp_secs,
                        })
                    })
                    .collect();
                span.ok_json(json!({
                    "repo": repo_id.dir_name(),
                    "entries": logs,
                }))
            }
            Err(e) => span.internal_error(json!({"error": e.to_string()})),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let webid = WebID::new();
    let server = GitServer::from_env(webid)?;

    hkask_mcp::run_server(
        "hkask-mcp-git",
        env!("CARGO_PKG_VERSION"),
        |_ctx| Ok(server),
        vec![hkask_mcp::CredentialRequirement::optional(
            "HKASK_CAS_HOME",
            "Base path for Git CAS operations",
        )],
    )
    .await
}
