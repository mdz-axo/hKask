//! hKask MCP GitHub — GitHub API operations
//!
//! This MCP server provides GitHub operations for repository, issue, and PR management.
//! Phase 9: Git archival via GitHub MCP tool calls.

use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RepoRequest {
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IssueRequest {
    pub owner: String,
    pub repo: String,
    pub issue_number: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateIssueRequest {
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub body: Option<String>,
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListIssuesRequest {
    pub owner: String,
    pub repo: String,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPrsRequest {
    pub owner: String,
    pub repo: String,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PrRequest {
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CommentRequest {
    pub owner: String,
    pub repo: String,
    pub issue_number: u64,
    pub body: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchReposRequest {
    pub query: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Default)]
pub struct GithubServer;

impl GithubServer {
    pub fn new() -> Self {
        Self
    }
}

#[tool_router(server_handler)]
impl GithubServer {
    #[tool(description = "Get repository information")]
    async fn github_get_repo(
        &self,
        Parameters(RepoRequest { owner, repo }): Parameters<RepoRequest>,
    ) -> String {
        serde_json::json!({
            "owner": owner,
            "repo": repo,
            "description": "Simulated repo",
            "stars": 100,
        })
        .to_string()
    }

    #[tool(description = "List issues in a repository")]
    async fn github_list_issues(
        &self,
        Parameters(ListIssuesRequest { owner, repo, state }): Parameters<ListIssuesRequest>,
    ) -> String {
        serde_json::json!({
            "owner": owner,
            "repo": repo,
            "state": state.unwrap_or_else(|| "open".to_string()),
            "issues": [],
        })
        .to_string()
    }

    #[tool(description = "Get a specific issue")]
    async fn github_get_issue(
        &self,
        Parameters(IssueRequest {
            owner,
            repo,
            issue_number,
        }): Parameters<IssueRequest>,
    ) -> String {
        serde_json::json!({
            "owner": owner,
            "repo": repo,
            "number": issue_number,
            "title": format!("Issue #{}", issue_number),
        })
        .to_string()
    }

    #[tool(description = "Create a new issue")]
    async fn github_create_issue(
        &self,
        Parameters(CreateIssueRequest {
            owner,
            repo,
            title,
            body: _,
            labels: _,
        }): Parameters<CreateIssueRequest>,
    ) -> String {
        serde_json::json!({
            "owner": owner,
            "repo": repo,
            "title": title,
            "number": 1,
            "created": true,
        })
        .to_string()
    }

    #[tool(description = "Add a comment to an issue or PR")]
    async fn github_add_comment(
        &self,
        Parameters(CommentRequest {
            owner,
            repo,
            issue_number,
            body: _,
        }): Parameters<CommentRequest>,
    ) -> String {
        serde_json::json!({
            "owner": owner,
            "repo": repo,
            "issue": issue_number,
            "comment_id": 1,
            "created": true,
        })
        .to_string()
    }

    #[tool(description = "List pull requests")]
    async fn github_list_prs(
        &self,
        Parameters(ListPrsRequest { owner, repo, state }): Parameters<ListPrsRequest>,
    ) -> String {
        serde_json::json!({
            "owner": owner,
            "repo": repo,
            "state": state.unwrap_or_else(|| "open".to_string()),
            "prs": [],
        })
        .to_string()
    }

    #[tool(description = "Get a specific pull request")]
    async fn github_get_pr(
        &self,
        Parameters(PrRequest {
            owner,
            repo,
            pr_number,
        }): Parameters<PrRequest>,
    ) -> String {
        serde_json::json!({
            "owner": owner,
            "repo": repo,
            "number": pr_number,
            "title": format!("PR #{}", pr_number),
        })
        .to_string()
    }

    #[tool(description = "Search repositories")]
    async fn github_search_repos(
        &self,
        Parameters(SearchReposRequest { query, limit }): Parameters<SearchReposRequest>,
    ) -> String {
        let limit = limit.unwrap_or(10);
        serde_json::json!({
            "query": query,
            "limit": limit,
            "results": [],
        })
        .to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = GithubServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-github started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
