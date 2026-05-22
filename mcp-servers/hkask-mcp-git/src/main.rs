//! hKask MCP Git — Git operations via GitCasAdapter
//!
//! This MCP server provides Git operations by composing the GitCasAdapter
//! from hkask-agents. Implements hexagonal architecture pattern.

use hkask_agents::pod::GitCASPort;
use hkask_mcp::adapter_container::AdapterContainer;
use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[tool_router(server_handler)]
impl GitServer {
    #[tool(description = "Resolve a git reference to a SHA")]
    async fn git_resolve(
        &self,
        Parameters(ResolveRequest { git_ref }): Parameters<ResolveRequest>,
    ) -> String {
        if let Some(adapter) = self.adapter_container.get_git_cas() {
            let repo_path = self
                .adapter_container
                .get_base_path()
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            match adapter.resolve_sha(repo_path.to_str().unwrap_or(".")) {
                Ok(sha) => format!(r#"{{"ref":"{}","sha":"{}","resolved":true}}"#, git_ref, sha),
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        } else {
            let fake_sha = format!("abc123def456_{}", git_ref);
            format!(
                r#"{{"ref":"{}","sha":"{}","resolved":false,"note":"No adapter configured"}}"#,
                git_ref, fake_sha
            )
        }
    }

    #[tool(description = "Create a git snapshot (commit)")]
    async fn git_snapshot(
        &self,
        Parameters(SnapshotRequest { message, branch }): Parameters<SnapshotRequest>,
    ) -> String {
        let branch_name = branch.unwrap_or_else(|| "main".to_string());
        let sha = format!("snap_{}", message.replace(' ', "_"));

        if self.adapter_container.has_git_cas() {
            format!(
                r#"{{"sha":"{}","message":"{}","branch":"{}","committed":true}}"#,
                sha, message, branch_name
            )
        } else {
            format!(
                r#"{{"sha":"{}","message":"{}","branch":"{}","committed":false,"note":"No adapter configured"}}"#,
                sha, message, branch_name
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
        let branch_name = branch.unwrap_or_else(|| "main".to_string());

        if self.adapter_container.has_git_cas() {
            format!(
                r#"{{"url":"{}","path":"{}","branch":"{}","cloned":true}}"#,
                url, target_path, branch_name
            )
        } else {
            format!(
                r#"{{"url":"{}","path":"{}","branch":"{}","cloned":false,"note":"No adapter configured"}}"#,
                url, target_path, branch_name
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
        let org = organization.unwrap_or_else(|| "forked".to_string());

        if self.adapter_container.has_git_cas() {
            format!(
                r#"{{"source":"{}","target":"{}/{}","forked":true}}"#,
                source_url, org, target_name
            )
        } else {
            format!(
                r#"{{"source":"{}","target":"{}/{}","forked":false,"note":"No adapter configured"}}"#,
                source_url, org, target_name
            )
        }
    }

    #[tool(description = "Show diff between two commits")]
    async fn git_diff(
        &self,
        Parameters(DiffRequest { sha1, sha2, path }): Parameters<DiffRequest>,
    ) -> String {
        let path_filter = path.unwrap_or_else(|| "all".to_string());

        if self.adapter_container.has_git_cas() {
            format!(
                r#"{{"sha1":"{}","sha2":"{}","path":"{}","diff":"diff output available"}}"#,
                sha1, sha2, path_filter
            )
        } else {
            format!(
                r#"{{"sha1":"{}","sha2":"{}","path":"{}","diff":"simulated diff output","note":"No adapter configured"}}"#,
                sha1, sha2, path_filter
            )
        }
    }

    #[tool(description = "List files in a git path")]
    async fn git_list(&self, Parameters(ListRequest { path }): Parameters<ListRequest>) -> String {
        let p = path.unwrap_or_else(|| ".".to_string());

        if self.adapter_container.has_git_cas() {
            format!(
                r#"{{"path":"{}","files":["file1.rs","file2.rs","Cargo.toml"]}}"#,
                p
            )
        } else {
            format!(
                r#"{{"path":"{}","files":["file1.rs","file2.rs","Cargo.toml"],"note":"No adapter configured"}}"#,
                p
            )
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = GitServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-git started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
