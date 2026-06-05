//! hKask MCP GitHub — GitHub REST API v3 operations
//!
//! This MCP server provides GitHub operations for repository, issue, and PR management.
//! Phase 9: Git archival via GitHub MCP tool calls.

use hkask_mcp::server::{
    CredentialRequirement, McpToolError, ServerContext, ToolSpanGuard, api_get, api_post,
    classify_http_error, resolve_credential, run_stdio_server, validate_identifier,
};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_API_BASE: &str = "https://api.github.com";

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

fn build_client() -> Result<reqwest::Client, McpToolError> {
    let token = resolve_credential("HKASK_GITHUB_TOKEN").map_err(|_| {
        McpToolError::failed_precondition("HKASK_GITHUB_TOKEN not found in keychain or environment")
    })?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github+json".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::USER_AGENT,
        "hkask-mcp-github".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| McpToolError::internal(format!("Failed to build HTTP client: {e}")))
}

fn validate_owner_repo(owner: &str, repo: &str) -> Result<(), McpToolError> {
    validate_identifier("owner", owner, 64)?;
    validate_identifier("repo", repo, 128)?;
    Ok(())
}

fn extract_repo_summary(v: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "owner": v["owner"]["login"].as_str().unwrap_or(""),
        "repo": v["name"].as_str().unwrap_or(""),
        "full_name": v["full_name"].as_str().unwrap_or(""),
        "description": v["description"],
        "stars": v["stargazers_count"],
        "forks": v["forks_count"],
        "open_issues": v["open_issues_count"],
        "language": v["language"],
        "default_branch": v["default_branch"],
        "private": v["private"],
        "html_url": v["html_url"],
    })
}

fn extract_issue_summary(v: &serde_json::Value) -> serde_json::Value {
    let labels: Vec<serde_json::Value> = v["labels"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|l| serde_json::json!({ "name": l["name"], "color": l["color"] }))
                .collect()
        })
        .unwrap_or_default();

    serde_json::json!({
        "number": v["number"],
        "title": v["title"],
        "state": v["state"],
        "labels": labels,
        "user": v["user"]["login"],
        "created_at": v["created_at"],
        "updated_at": v["updated_at"],
    })
}

fn extract_pr_summary(v: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "number": v["number"],
        "title": v["title"],
        "state": v["state"],
        "user": v["user"]["login"],
        "head": v["head"]["ref"],
        "base": v["base"]["ref"],
        "created_at": v["created_at"],
        "updated_at": v["updated_at"],
        "draft": v["draft"],
    })
}

pub struct GithubServer {
    webid: WebID,
    client: reqwest::Client,
}

impl GithubServer {
    pub fn new(webid: WebID) -> Result<Self, anyhow::Error> {
        let client = build_client()?;
        Ok(Self { webid, client })
    }
}

#[tool_router(server_handler)]
impl GithubServer {
    #[tool(description = "Get repository information")]
    async fn github_get_repo(
        &self,
        Parameters(RepoRequest { owner, repo }): Parameters<RepoRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("github_get_repo", &self.webid);
        if let Err(e) = validate_owner_repo(&owner, &repo) {
            return span.error(e.kind, e.to_json_string());
        }
        let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}");
        match api_get(&self.client, "GitHub", &url).await {
            Ok(v) => span.ok_json(extract_repo_summary(&v)),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "List issues in a repository")]
    async fn github_list_issues(
        &self,
        Parameters(ListIssuesRequest { owner, repo, state }): Parameters<ListIssuesRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("github_list_issues", &self.webid);
        if let Err(e) = validate_owner_repo(&owner, &repo) {
            return span.error(e.kind, e.to_json_string());
        }
        let state = state.unwrap_or_else(|| "open".to_string());
        let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/issues?state={state}");
        match api_get(&self.client, "GitHub", &url).await {
            Ok(v) => {
                let issues: Vec<serde_json::Value> = v
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter(|i| i.get("pull_request").is_none())
                            .map(extract_issue_summary)
                            .collect()
                    })
                    .unwrap_or_default();
                span.ok_json(
                    serde_json::json!({ "owner": owner, "repo": repo, "state": state, "issues": issues }),
                )
            }
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
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
        let span = ToolSpanGuard::new("github_get_issue", &self.webid);
        if let Err(e) = validate_owner_repo(&owner, &repo) {
            return span.error(e.kind, e.to_json_string());
        }
        let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/issues/{issue_number}");
        match api_get(&self.client, "GitHub", &url).await {
            Ok(v) => {
                let labels: Vec<serde_json::Value> = v["labels"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|l| serde_json::json!({ "name": l["name"], "color": l["color"] }))
                            .collect()
                    })
                    .unwrap_or_default();
                span.ok_json(serde_json::json!({
                    "owner": owner, "repo": repo,
                    "number": v["number"], "title": v["title"], "state": v["state"],
                    "body": v["body"], "labels": labels, "user": v["user"]["login"],
                    "assignees": v["assignees"], "milestone": v["milestone"],
                    "comments": v["comments"], "created_at": v["created_at"],
                    "updated_at": v["updated_at"], "html_url": v["html_url"],
                }))
            }
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Create a new issue")]
    async fn github_create_issue(
        &self,
        Parameters(CreateIssueRequest {
            owner,
            repo,
            title,
            body,
            labels,
        }): Parameters<CreateIssueRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("github_create_issue", &self.webid);
        if let Err(e) = validate_owner_repo(&owner, &repo) {
            return span.error(e.kind, e.to_json_string());
        }
        if title.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("title must not be empty").to_json_string(),
            );
        }
        let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/issues");
        let mut payload = serde_json::json!({ "title": title });
        if let Some(ref b) = body {
            payload["body"] = serde_json::Value::String(b.clone());
        }
        if let Some(ref l) = labels {
            payload["labels"] = serde_json::Value::Array(
                l.iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }
        match api_post(&self.client, "GitHub", &url, &payload).await {
            Ok(v) => span.ok_json(serde_json::json!({
                "owner": owner, "repo": repo,
                "number": v["number"], "title": v["title"],
                "state": v["state"], "html_url": v["html_url"], "created": true,
            })),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Add a comment to an issue or PR")]
    async fn github_add_comment(
        &self,
        Parameters(CommentRequest {
            owner,
            repo,
            issue_number,
            body,
        }): Parameters<CommentRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("github_add_comment", &self.webid);
        if let Err(e) = validate_owner_repo(&owner, &repo) {
            return span.error(e.kind, e.to_json_string());
        }
        if body.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("body must not be empty").to_json_string(),
            );
        }
        let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/issues/{issue_number}/comments");
        let payload = serde_json::json!({ "body": body });
        match api_post(&self.client, "GitHub", &url, &payload).await {
            Ok(v) => span.ok_json(serde_json::json!({
                "owner": owner, "repo": repo, "issue": issue_number,
                "comment_id": v["id"], "html_url": v["html_url"], "created": true,
            })),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "List pull requests")]
    async fn github_list_prs(
        &self,
        Parameters(ListPrsRequest { owner, repo, state }): Parameters<ListPrsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("github_list_prs", &self.webid);
        if let Err(e) = validate_owner_repo(&owner, &repo) {
            return span.error(e.kind, e.to_json_string());
        }
        let state = state.unwrap_or_else(|| "open".to_string());
        let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/pulls?state={state}");
        match api_get(&self.client, "GitHub", &url).await {
            Ok(v) => {
                let prs: Vec<serde_json::Value> = v
                    .as_array()
                    .map(|arr| arr.iter().map(extract_pr_summary).collect())
                    .unwrap_or_default();
                span.ok_json(
                    serde_json::json!({ "owner": owner, "repo": repo, "state": state, "prs": prs }),
                )
            }
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
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
        let span = ToolSpanGuard::new("github_get_pr", &self.webid);
        if let Err(e) = validate_owner_repo(&owner, &repo) {
            return span.error(e.kind, e.to_json_string());
        }
        let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/pulls/{pr_number}");
        match api_get(&self.client, "GitHub", &url).await {
            Ok(v) => span.ok_json(serde_json::json!({
                "owner": owner, "repo": repo,
                "number": v["number"], "title": v["title"], "state": v["state"],
                "body": v["body"], "user": v["user"]["login"],
                "head": v["head"]["ref"], "head_repo": v["head"]["repo"]["full_name"],
                "base": v["base"]["ref"], "merged": v["merged"],
                "mergeable": v["mergeable"], "draft": v["draft"],
                "additions": v["additions"], "deletions": v["deletions"],
                "changed_files": v["changed_files"],
                "created_at": v["created_at"], "updated_at": v["updated_at"],
                "html_url": v["html_url"],
            })),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Search repositories")]
    async fn github_search_repos(
        &self,
        Parameters(SearchReposRequest { query, limit }): Parameters<SearchReposRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("github_search_repos", &self.webid);
        if query.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("query must not be empty").to_json_string(),
            );
        }
        let limit = limit.unwrap_or(10);
        let url = format!("{GITHUB_API_BASE}/search/repositories");
        let resp = self
            .client
            .get(url)
            .query(&[("q", query.as_str()), ("per_page", &limit.to_string())])
            .send()
            .await;
        match resp {
            Ok(http_resp) => {
                let status = http_resp.status();
                let body = http_resp.text().await.unwrap_or_default();
                if !status.is_success() {
                    let e = classify_http_error("GitHub", status, &body);
                    return span.error(e.kind, e.to_json_string());
                }
                match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(v) => {
                        let results: Vec<serde_json::Value> = v["items"]
                            .as_array()
                            .map(|arr| arr.iter().map(extract_repo_summary).collect())
                            .unwrap_or_default();
                        span.ok_json(serde_json::json!({
                            "query": query, "limit": limit,
                            "total_count": v["total_count"], "results": results,
                        }))
                    }
                    Err(e) => span.internal_error(
                        serde_json::json!({"error": format!("Failed to parse response: {e}")}),
                    ),
                }
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("GitHub request failed: {e}")).to_json_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
        "hkask-mcp-github",
        SERVER_VERSION,
        |ctx: ServerContext| GithubServer::new(ctx.webid),
        vec![CredentialRequirement::required(
            "HKASK_GITHUB_TOKEN",
            "GitHub personal access token for API authentication",
        )],
    )
    .await
}
