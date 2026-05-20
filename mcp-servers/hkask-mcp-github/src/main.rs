//! hKask MCP GitHub — GitHub repository, issue, and PR operations

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter},
    model::*,
    schemars, tool, tool_router, tool_handler,
};
use rmcp::handler::server::wrapper::parameters::Parameters;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::collections::HashMap;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_API_BASE: &str = "https://api.github.com";

/// GitHub repository info
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RepoInfo {
    pub full_name: String,
    pub description: Option<String>,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub default_branch: String,
    pub private: bool,
}

/// Issue info
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct IssueInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub labels: Vec<String>,
    pub created_at: String,
}

/// Pull request info
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PullRequestInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub merged: bool,
    pub created_at: String,
}

/// GitHub server implementation
pub struct GitHubServer {
    tool_router: ToolRouter<GitHubServer>,
    client: Client,
    token: Option<String>,
}

impl GitHubServer {
    pub fn new() -> Self {
        let token = std::env::var("GITHUB_TOKEN").ok();
        let client = Client::builder()
            .user_agent("hkask-mcp-github")
            .build()
            .unwrap_or_default();

        Self {
            tool_router: Self::tool_router(),
            client,
            token,
        }
    }

    async fn get_headers(&self) -> HashMap<&str, String> {
        let mut headers = HashMap::new();
        headers.insert("Accept", "application/vnd.github.v3+json".to_string());
        if let Some(token) = &self.token {
            headers.insert("Authorization", format!("Bearer {}", token));
        }
        headers
    }
}

#[tool_router]
impl GitHubServer {
    #[tool(description = "Get repository information")]
    async fn github_get_repo(&self, owner: String, repo: String) -> String {
        let url = format!("{}/repos/{}/{}", GITHUB_API_BASE, owner, repo);
        
        match self.client.get(&url).headers(self.get_headers().await.into()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "List issues in a repository")]
    async fn github_list_issues(&self, owner: String, repo: String, state: Option<String>) -> String {
        let state = state.unwrap_or_else(|| "open".to_string());
        let url = format!("{}/repos/{}/{}/issues?state={}", GITHUB_API_BASE, owner, repo, state);
        
        match self.client.get(&url).headers(self.get_headers().await.into()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get a specific issue")]
    async fn github_get_issue(&self, owner: String, repo: String, issue_number: u64) -> String {
        let url = format!("{}/repos/{}/{}/issues/{}", GITHUB_API_BASE, owner, repo, issue_number);
        
        match self.client.get(&url).headers(self.get_headers().await.into()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Create a new issue")]
    async fn github_create_issue(&self, owner: String, repo: String, title: String, body: Option<String>, labels: Option<Vec<String>>) -> String {
        if self.token.is_none() {
            return serde_json::json!({ "error": "GITHUB_TOKEN required for write operations" }).to_string();
        }

        let url = format!("{}/repos/{}/{}/issues", GITHUB_API_BASE, owner, repo);
        let mut payload = serde_json::json!({ "title": title });
        if let Some(body) = body {
            payload["body"] = serde_json::json!(body);
        }
        if let Some(labels) = labels {
            payload["labels"] = serde_json::json!(labels);
        }

        match self.client.post(&url).headers(self.get_headers().await.into()).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "List pull requests")]
    async fn github_list_prs(&self, owner: String, repo: String, state: Option<String>) -> String {
        let state = state.unwrap_or_else(|| "open".to_string());
        let url = format!("{}/repos/{}/{}/pulls?state={}", GITHUB_API_BASE, owner, repo, state);
        
        match self.client.get(&url).headers(self.get_headers().await.into()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get a specific pull request")]
    async fn github_get_pr(&self, owner: String, repo: String, pr_number: u64) -> String {
        let url = format!("{}/repos/{}/{}/pulls/{}", GITHUB_API_BASE, owner, repo, pr_number);
        
        match self.client.get(&url).headers(self.get_headers().await.into()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Add a comment to an issue or PR")]
    async fn github_add_comment(&self, owner: String, repo: String, issue_number: u64, body: String) -> String {
        if self.token.is_none() {
            return serde_json::json!({ "error": "GITHUB_TOKEN required for write operations" }).to_string();
        }

        let url = format!("{}/repos/{}/{}/issues/{}/comments", GITHUB_API_BASE, owner, repo, issue_number);
        let payload = serde_json::json!({ "body": body });

        match self.client.post(&url).headers(self.get_headers().await.into()).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Search repositories")]
    async fn github_search_repos(&self, query: String, limit: Option<usize>) -> String {
        let limit = limit.unwrap_or(10);
        let url = format!("{}/search/repositories?q={}&per_page={}", GITHUB_API_BASE, query, limit);
        
        match self.client.get(&url).headers(self.get_headers().await.into()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }
}

#[tool_handler]
impl ServerHandler for GitHubServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = GitHubServer::new();
    let service = server.serve_stdio();
    tracing::info!("hkask-mcp-github MCP server started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
