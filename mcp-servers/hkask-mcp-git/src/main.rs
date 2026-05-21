//! hKask MCP Git — Git operations with gix
//!
//! This MCP server provides Git operations. In production, it wires to
//! the GitCasAdapter from hkask-agents. For now, returns structured stub data.

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router, transport::stdio, ServiceExt};
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

#[derive(Debug, Default)]
pub struct GitServer;

impl GitServer {
    pub fn new() -> Self {
        Self
    }
}

#[tool_router(server_handler)]
impl GitServer {
    #[tool(description = "Resolve a git reference to a SHA")]
    async fn git_resolve(
        &self,
        Parameters(ResolveRequest { git_ref }): Parameters<ResolveRequest>,
    ) -> String {
        let fake_sha = format!("abc123def456_{}", git_ref);
        format!(
            r#"{{"ref":"{}","sha":"{}","resolved":true}}"#,
            git_ref, fake_sha
        )
    }

    #[tool(description = "Create a git snapshot (commit)")]
    async fn git_snapshot(
        &self,
        Parameters(SnapshotRequest { message, branch }): Parameters<SnapshotRequest>,
    ) -> String {
        let sha = format!("snap_{}", message.replace(' ', "_"));
        format!(
            r#"{{"sha":"{}","message":"{}","branch":"{}","committed":true}}"#,
            sha,
            message,
            branch.unwrap_or_else(|| "main".to_string())
        )
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
        format!(
            r#"{{"url":"{}","path":"{}","branch":"{}","cloned":true}}"#,
            url,
            target_path,
            branch.unwrap_or_else(|| "main".to_string())
        )
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
        format!(
            r#"{{"source":"{}","target":"{}/{}","forked":true}}"#,
            source_url, org, target_name
        )
    }

    #[tool(description = "Show diff between two commits")]
    async fn git_diff(
        &self,
        Parameters(DiffRequest { sha1, sha2, path }): Parameters<DiffRequest>,
    ) -> String {
        format!(
            r#"{{"sha1":"{}","sha2":"{}","path":"{}","diff":"simulated diff output"}}"#,
            sha1,
            sha2,
            path.unwrap_or_else(|| "all".to_string())
        )
    }

    #[tool(description = "List files in a git path")]
    async fn git_list(&self, Parameters(ListRequest { path }): Parameters<ListRequest>) -> String {
        let p = path.unwrap_or_else(|| ".".to_string());
        format!(
            r#"{{"path":"{}","files":["file1.rs","file2.rs","Cargo.toml"]}}"#,
            p
        )
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
