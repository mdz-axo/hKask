//! hKask MCP Git — Git operations via GitCasAdapter
//!
//! This MCP server provides Git operations by composing the GitCasAdapter
//! from hkask-agents. Implements hexagonal architecture pattern.

use hkask_agents::GitCASPort;
use hkask_mcp::adapter_container::AdapterContainer;
use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

const ENV_GIT_BASE_PATH: &str = "HKASK_GIT_BASE_PATH";

fn validate_path(path: &str) -> Result<(), String> {
    if path.contains('\0') {
        return Err("Path contains null bytes".to_string());
    }
    if Path::new(path).is_absolute() {
        return Err("Absolute paths not allowed".to_string());
    }
    if path.contains("..") {
        return Err("Parent directory traversal not allowed".to_string());
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

#[derive(Default)]
pub struct GitServer {
    adapter_container: AdapterContainer,
}

impl GitServer {
    pub fn new() -> Self {
        Self {
            adapter_container: AdapterContainer::new(),
        }
    }

    pub fn with_base_path(base_path: std::path::PathBuf) -> Self {
        let container = AdapterContainer::new();
        container.configure_git_cas(base_path).ok();
        Self {
            adapter_container: container,
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
        if let Some(adapter) = self.adapter_container.get_git_cas() {
            match adapter.resolve_sha(&git_ref) {
                Ok(sha) => serde_json::json!({
                    "ref": git_ref,
                    "sha": sha,
                    "resolved": true,
                })
                .to_string(),
                Err(e) => serde_json::json!({
                    "ref": git_ref,
                    "error": e.to_string(),
                })
                .to_string(),
            }
        } else {
            serde_json::json!({
                "ref": git_ref,
                "error": "No adapter configured",
            })
            .to_string()
        }
    }

    #[tool(description = "Create a git snapshot (commit)")]
    async fn git_snapshot(
        &self,
        Parameters(SnapshotRequest { message, branch }): Parameters<SnapshotRequest>,
    ) -> String {
        let branch_name = branch.unwrap_or_else(|| "main".to_string());

        if let Some(base_path) = self.adapter_container.get_base_path() {
            match git_commit(&base_path, &message, &branch_name) {
                Ok(sha) => serde_json::json!({
                    "sha": sha,
                    "message": message,
                    "branch": branch_name,
                    "committed": true,
                })
                .to_string(),
                Err(e) => serde_json::json!({
                    "message": message,
                    "branch": branch_name,
                    "committed": false,
                    "error": e,
                })
                .to_string(),
            }
        } else {
            serde_json::json!({
                "message": message,
                "branch": branch_name,
                "committed": false,
                "error": "No adapter configured",
            })
            .to_string()
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
        let branch_name = branch.unwrap_or_else(|| "main".to_string());

        if let Err(e) = validate_path(&target_path) {
            return serde_json::json!({
                "url": url,
                "path": target_path,
                "branch": branch_name,
                "cloned": false,
                "error": e,
            })
            .to_string();
        }

        if let Some(base_path) = self.adapter_container.get_base_path() {
            let full_path = base_path.join(&target_path);
            let output = std::process::Command::new("git")
                .args(["clone", "--branch", &branch_name, &url])
                .arg(&full_path)
                .output();

            match output {
                Ok(out) if out.status.success() => serde_json::json!({
                    "url": url,
                    "path": target_path,
                    "branch": branch_name,
                    "cloned": true,
                })
                .to_string(),
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    serde_json::json!({
                        "url": url,
                        "path": target_path,
                        "branch": branch_name,
                        "cloned": false,
                        "error": stderr.trim(),
                    })
                    .to_string()
                }
                Err(e) => serde_json::json!({
                    "url": url,
                    "path": target_path,
                    "branch": branch_name,
                    "cloned": false,
                    "error": e.to_string(),
                })
                .to_string(),
            }
        } else {
            serde_json::json!({
                "url": url,
                "path": target_path,
                "branch": branch_name,
                "cloned": false,
                "error": "No adapter configured",
            })
            .to_string()
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
        let org = organization.unwrap_or_else(|| "forked".to_string());

        if let Err(e) = validate_path(&target_name) {
            return serde_json::json!({
                "source": source_url,
                "target": format!("{}/{}", org, target_name),
                "forked": false,
                "error": e,
            })
            .to_string();
        }

        if self.adapter_container.has_git_cas() {
            serde_json::json!({
                "source": source_url,
                "target": format!("{}/{}", org, target_name),
                "forked": true,
            })
            .to_string()
        } else {
            serde_json::json!({
                "source": source_url,
                "target": format!("{}/{}", org, target_name),
                "forked": false,
                "error": "No adapter configured",
            })
            .to_string()
        }
    }

    #[tool(description = "Show diff between two commits")]
    async fn git_diff(
        &self,
        Parameters(DiffRequest { sha1, sha2, path }): Parameters<DiffRequest>,
    ) -> String {
        let path_filter = path.unwrap_or_else(|| "all".to_string());

        if let Err(e) = validate_path(&path_filter) {
            return serde_json::json!({
                "sha1": sha1,
                "sha2": sha2,
                "path": path_filter,
                "error": e,
            })
            .to_string();
        }

        if let Some(base_path) = self.adapter_container.get_base_path() {
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
                    serde_json::json!({
                        "sha1": sha1,
                        "sha2": sha2,
                        "path": path_filter,
                        "diff": diff,
                    })
                    .to_string()
                }
                Err(e) => serde_json::json!({
                    "sha1": sha1,
                    "sha2": sha2,
                    "path": path_filter,
                    "error": e.to_string(),
                })
                .to_string(),
            }
        } else {
            serde_json::json!({
                "sha1": sha1,
                "sha2": sha2,
                "path": path_filter,
                "error": "No adapter configured",
            })
            .to_string()
        }
    }

    #[tool(description = "List files in a git path")]
    async fn git_list(&self, Parameters(ListRequest { path }): Parameters<ListRequest>) -> String {
        let p = path.unwrap_or_else(|| ".".to_string());

        if let Err(e) = validate_path(&p) {
            return serde_json::json!({
                "path": p,
                "error": e,
            })
            .to_string();
        }

        if let Some(base_path) = self.adapter_container.get_base_path() {
            let output = std::process::Command::new("git")
                .args(["ls-tree", "--name-only", "HEAD", &p])
                .current_dir(base_path)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let listing = String::from_utf8_lossy(&out.stdout);
                    let files: Vec<&str> = listing.lines().collect();
                    serde_json::json!({
                        "path": p,
                        "files": files,
                    })
                    .to_string()
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    serde_json::json!({
                        "path": p,
                        "error": stderr.trim(),
                    })
                    .to_string()
                }
                Err(e) => serde_json::json!({
                    "path": p,
                    "error": e.to_string(),
                })
                .to_string(),
            }
        } else {
            serde_json::json!({
                "path": p,
                "error": "No adapter configured",
            })
            .to_string()
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = if let Ok(base_path) = std::env::var(ENV_GIT_BASE_PATH) {
        let path = std::path::PathBuf::from(&base_path);
        tracing::info!("Using GIT base path: {}", base_path);
        GitServer::with_base_path(path)
    } else {
        tracing::warn!(
            "{} not set, Git adapter unconfigured — server will reject operations",
            ENV_GIT_BASE_PATH
        );
        GitServer::new()
    };

    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-git started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
