//! hKask MCP Scholar — Semantic Scholar Graph API wrapper + local bitemporal persistence
//!
//! Provides academic paper search, citation graph traversal, and local SQLite storage
//! for offline access. Uses the hexagonal adapter pattern: HttpScholarApi for upstream
//! and PersistingScholarApi decorator for write-through caching.

use async_trait::async_trait;
use hkask_mcp::server::{
    CredentialRequirement, McpToolError, McpToolOutput, emit_tool_span,
    resolve_credential, run_stdio_server, validate_identifier,
};
use hkask_types::McpErrorKind;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const S2_API_BASE: &str = "https://api.semanticscholar.org/graph/v1";

const DEFAULT_FIELDS: &str = "paperId,title,abstract,year,citationCount,referenceCount,url,authors,venue,publicationDate,externalIds";
const BATCH_MAX: usize = 500;
const GRAPH_MAX_DEPTH: u32 = 3;
const GRAPH_MAX_FANOUT: usize = 100;
const GRAPH_DEFAULT_BUDGET: u32 = 200;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub fields: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PaperDetailsRequest {
    pub paper_id: String,
    pub fields: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PaperBatchRequest {
    pub paper_ids: Vec<String>,
    pub fields: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CitationsRequest {
    pub paper_id: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub fields: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReferencesRequest {
    pub paper_id: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub fields: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthorRequest {
    pub author_id: String,
    pub fields: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecommendationsRequest {
    pub positive_paper_ids: Vec<String>,
    pub negative_paper_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphSegmentRequest {
    pub seeds: Option<Vec<String>>,
    pub query: Option<String>,
    pub depth: Option<u32>,
    pub mode: Option<String>,
    pub budget: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoreGetPaperRequest {
    pub paper_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoreGraphRequest {
    pub seeds: Vec<String>,
}

// =============================================================================
// Domain types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub paper_id: String,
    pub title: Option<String>,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub year: Option<i32>,
    pub citation_count: Option<i64>,
    pub reference_count: Option<i64>,
    pub url: Option<String>,
    pub authors: Option<Vec<AuthorBrief>>,
    pub venue: Option<String>,
    pub publication_date: Option<String>,
    pub external_ids: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorBrief {
    pub author_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub author_id: String,
    pub name: Option<String>,
    pub paper_count: Option<i64>,
    pub citation_count: Option<i64>,
    pub h_index: Option<i32>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationEntry {
    pub paper: Paper,
    pub contexts: Option<Vec<String>>,
    pub intents: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceEntry {
    pub paper: Paper,
    pub contexts: Option<Vec<String>>,
    pub intents: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub total: Option<i64>,
    pub offset: Option<i32>,
    pub papers: Vec<Paper>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSegment {
    pub seeds: Vec<String>,
    pub depth: u32,
    pub mode: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub truncated: bool,
    pub upstream_calls: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub paper_id: String,
    pub title: Option<String>,
    pub year: Option<i32>,
    pub citation_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String,
}

// =============================================================================
// Error type
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ScholarError {
    #[error("Invalid arguments: {0}")]
    BadArgs(String),
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Provider unavailable: {0}")]
    Unavailable(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Store error: {0}")]
    StoreError(String),
}

impl ScholarError {
    pub fn kind(&self) -> McpErrorKind {
        match self {
            ScholarError::BadArgs(_) => McpErrorKind::InvalidArgument,
            ScholarError::HttpError(_) => McpErrorKind::Internal,
            ScholarError::RateLimited(_) => McpErrorKind::RateLimited,
            ScholarError::NotFound(_) => McpErrorKind::NotFound,
            ScholarError::Unavailable(_) => McpErrorKind::Unavailable,
            ScholarError::Internal(_) => McpErrorKind::Internal,
            ScholarError::StoreError(_) => McpErrorKind::Internal,
        }
    }
}

impl From<ScholarError> for McpToolError {
    fn from(e: ScholarError) -> Self {
        McpToolError::new(e.kind(), e.to_string())
    }
}

// =============================================================================
// Outbound port trait
// =============================================================================

#[async_trait]
trait ScholarApi: Send + Sync {
    async fn get_paper(&self, paper_id: &str, fields: Option<&str>) -> Result<Paper, ScholarError>;
    async fn get_papers_batch(&self, ids: &[&str], fields: Option<&str>) -> Result<Vec<Option<Paper>>, ScholarError>;
    async fn search_papers(&self, query: &str, limit: Option<u32>, offset: Option<u32>, fields: Option<&str>) -> Result<SearchResult, ScholarError>;
    async fn list_citations(&self, paper_id: &str, offset: Option<u32>, limit: Option<u32>, fields: Option<&str>) -> Result<Vec<CitationEntry>, ScholarError>;
    async fn list_references(&self, paper_id: &str, offset: Option<u32>, limit: Option<u32>, fields: Option<&str>) -> Result<Vec<ReferenceEntry>, ScholarError>;
    async fn get_author(&self, author_id: &str, fields: Option<&str>) -> Result<Author, ScholarError>;
    async fn recommend(&self, positive_ids: &[&str], negative_ids: &[&str]) -> Result<Vec<Paper>, ScholarError>;
}

// =============================================================================
// HTTP adapter
// =============================================================================

struct HttpScholarApi {
    client: reqwest::Client,
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl HttpScholarApi {
    fn new(api_key: Option<String>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::USER_AGENT, "hkask-mcp-scholar".parse().unwrap());
        if let Some(ref key) = api_key {
            headers.insert("x-api-key", key.parse().unwrap());
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");
        Self { client, api_key }
    }

    fn fields_or_default<'a>(&'a self, fields: Option<&'a str>) -> &'a str {
        fields.unwrap_or(DEFAULT_FIELDS)
    }
}

fn parse_paper(v: &serde_json::Value) -> Option<Paper> {
    let paper_id = v.get("paperId")?.as_str()?.to_string();
    let title = v.get("title").and_then(|t| t.as_str()).map(|s| s.to_string());
    if title.is_none() || title.as_ref().map_or(true, |t| t.is_empty()) {
        if v.get("title").is_none() {
            return None;
        }
    }
    let authors = v.get("authors").and_then(|a| a.as_array()).map(|arr| {
        arr.iter().filter_map(|a| {
            Some(AuthorBrief {
                author_id: a.get("authorId").and_then(|v| v.as_str()).map(|s| s.to_string()),
                name: a.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            })
        }).collect::<Vec<_>>()
    });

    Some(Paper {
        paper_id,
        title,
        abstract_text: v.get("abstract").and_then(|v| v.as_str()).map(|s| s.to_string()),
        year: v.get("year").and_then(|v| v.as_i64()).map(|v| v as i32),
        citation_count: v.get("citationCount").and_then(|v| v.as_i64()),
        reference_count: v.get("referenceCount").and_then(|v| v.as_i64()),
        url: v.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
        authors,
        venue: v.get("venue").and_then(|v| v.as_str()).map(|s| s.to_string()),
        publication_date: v.get("publicationDate").and_then(|v| v.as_str()).map(|s| s.to_string()),
        external_ids: v.get("externalIds").cloned(),
    })
}

#[async_trait]
impl ScholarApi for HttpScholarApi {
    async fn get_paper(&self, paper_id: &str, fields: Option<&str>) -> Result<Paper, ScholarError> {
        let f = self.fields_or_default(fields);
        let url = format!("{S2_API_BASE}/paper/{paper_id}?fields={f}");
        let resp = self.client.get(&url).send().await.map_err(|e| ScholarError::Unavailable(format!("S2 request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        parse_paper(&v).ok_or_else(|| ScholarError::Internal("Failed to parse paper".to_string()))
    }

    async fn get_papers_batch(&self, ids: &[&str], fields: Option<&str>) -> Result<Vec<Option<Paper>>, ScholarError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        if ids.len() > BATCH_MAX {
            return Err(ScholarError::BadArgs(format!("Batch size {} exceeds maximum of {BATCH_MAX}", ids.len())));
        }
        let f = self.fields_or_default(fields);
        let url = format!("{S2_API_BASE}/paper/batch?fields={f}");
        let payload = serde_json::json!({ "ids": ids });
        let resp = self.client.post(&url).json(&payload).send().await.map_err(|e| ScholarError::Unavailable(format!("S2 batch request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let arr: Vec<serde_json::Value> = serde_json::from_str(&body).map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        Ok(arr.iter().map(|v| parse_paper(v)).collect())
    }

    async fn search_papers(&self, query: &str, limit: Option<u32>, offset: Option<u32>, fields: Option<&str>) -> Result<SearchResult, ScholarError> {
        let f = self.fields_or_default(fields);
        let limit = limit.unwrap_or(10).min(100);
        let offset = offset.unwrap_or(0);
        let url = format!("{S2_API_BASE}/paper/search?query={}&limit={limit}&offset={offset}&fields={f}", urlencoding::encode(query));
        let resp = self.client.get(&url).send().await.map_err(|e| ScholarError::Unavailable(format!("S2 search failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        let total = v.get("total").and_then(|v| v.as_i64());
        let papers = v.get("data").and_then(|d| d.as_array())
            .map(|arr| arr.iter().filter_map(|p| parse_paper(p)).collect::<Vec<_>>())
            .unwrap_or_default();
        Ok(SearchResult { total, offset: Some(offset as i32), papers })
    }

    async fn list_citations(&self, paper_id: &str, offset: Option<u32>, limit: Option<u32>, fields: Option<&str>) -> Result<Vec<CitationEntry>, ScholarError> {
        let f = self.fields_or_default(fields);
        let limit = limit.unwrap_or(100).min(500);
        let offset = offset.unwrap_or(0);
        let url = format!("{S2_API_BASE}/paper/{paper_id}/citations?fields={f}&offset={offset}&limit={limit}");
        let resp = self.client.get(&url).send().await.map_err(|e| ScholarError::Unavailable(format!("S2 citations request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        v.get("data").and_then(|d| d.as_array())
            .map(|arr| arr.iter().filter_map(|item| {
                let citing_paper = parse_paper(item.get("citingPaper")?)?;
                Some(CitationEntry {
                    paper: citing_paper,
                    contexts: item.get("contexts").and_then(|c| c.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
                    intents: item.get("intents").and_then(|i| i.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
                })
            }).collect())
            .ok_or_else(|| ScholarError::Internal("Failed to parse citations".to_string()))
    }

    async fn list_references(&self, paper_id: &str, offset: Option<u32>, limit: Option<u32>, fields: Option<&str>) -> Result<Vec<ReferenceEntry>, ScholarError> {
        let f = self.fields_or_default(fields);
        let limit = limit.unwrap_or(100).min(500);
        let offset = offset.unwrap_or(0);
        let url = format!("{S2_API_BASE}/paper/{paper_id}/references?fields={f}&offset={offset}&limit={limit}");
        let resp = self.client.get(&url).send().await.map_err(|e| ScholarError::Unavailable(format!("S2 references request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        v.get("data").and_then(|d| d.as_array())
            .map(|arr| arr.iter().filter_map(|item| {
                let cited_paper = parse_paper(item.get("citedPaper")?)?;
                Some(ReferenceEntry {
                    paper: cited_paper,
                    contexts: item.get("contexts").and_then(|c| c.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
                    intents: item.get("intents").and_then(|i| i.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
                })
            }).collect())
            .ok_or_else(|| ScholarError::Internal("Failed to parse references".to_string()))
    }

    async fn get_author(&self, author_id: &str, fields: Option<&str>) -> Result<Author, ScholarError> {
        let default_fields = "authorId,name,paperCount,citationCount,hIndex,url";
        let f = fields.unwrap_or(default_fields);
        let url = format!("{S2_API_BASE}/author/{author_id}?fields={f}");
        let resp = self.client.get(&url).send().await.map_err(|e| ScholarError::Unavailable(format!("S2 author request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        Ok(Author {
            author_id: v.get("authorId").and_then(|v| v.as_str()).unwrap_or(author_id).to_string(),
            name: v.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            paper_count: v.get("paperCount").and_then(|v| v.as_i64()),
            citation_count: v.get("citationCount").and_then(|v| v.as_i64()),
            h_index: v.get("hIndex").and_then(|v| v.as_i64()).map(|v| v as i32),
            url: v.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    async fn recommend(&self, positive_ids: &[&str], negative_ids: &[&str]) -> Result<Vec<Paper>, ScholarError> {
        let url = format!("{S2_API_BASE}/recommendations/v1/papers/");
        let mut payload = serde_json::json!({ "positivePaperIds": positive_ids });
        if !negative_ids.is_empty() {
            payload["negativePaperIds"] = serde_json::json!(negative_ids);
        }
        let resp = self.client.post(&url).json(&payload).send().await.map_err(|e| ScholarError::Unavailable(format!("S2 recommend request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        Ok(v.get("recommendedPapers").and_then(|d| d.as_array())
            .map(|arr| arr.iter().filter_map(|p| parse_paper(p)).collect())
            .unwrap_or_default())
    }
}

fn classify_s2_error(status: reqwest::StatusCode, body: &str) -> ScholarError {
    match status.as_u16() {
        401 | 403 => ScholarError::Unavailable(format!("S2 auth error: {status}")),
        404 => ScholarError::NotFound(format!("S2 resource not found")),
        429 => ScholarError::RateLimited(format!("S2 rate limited")),
        502 | 503 => ScholarError::Unavailable(format!("S2 unavailable: {status}")),
        _ if status.is_server_error() => ScholarError::Unavailable(format!("S2 server error: {status}")),
        _ => ScholarError::HttpError(format!("S2 API error {status}: {}", body.chars().take(200).collect::<String>())),
    }
}

// =============================================================================
// ScholarStore (SQLite persistence)
// =============================================================================

struct ScholarStore {
    conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
}

impl ScholarStore {
    fn new(path: &str) -> Result<Arc<Self>, anyhow::Error> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS papers (
                paper_id     TEXT PRIMARY KEY,
                title        TEXT,
                abstract_text TEXT,
                year         INTEGER,
                citation_count INTEGER,
                reference_count INTEGER,
                url          TEXT,
                venue        TEXT,
                publication_date TEXT,
                external_ids TEXT,
                stored_at    TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS authors (
                author_id   TEXT PRIMARY KEY,
                name        TEXT,
                paper_count INTEGER,
                citation_count INTEGER,
                h_index     INTEGER,
                url         TEXT,
                stored_at   TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS paper_authors (
                paper_id    TEXT NOT NULL REFERENCES papers(paper_id) ON DELETE CASCADE,
                author_id   TEXT NOT NULL REFERENCES authors(author_id) ON DELETE CASCADE,
                position    INTEGER DEFAULT 0,
                PRIMARY KEY (paper_id, author_id)
            );

            CREATE TABLE IF NOT EXISTS citations (
                citing_paper_id TEXT NOT NULL,
                cited_paper_id TEXT NOT NULL,
                PRIMARY KEY (citing_paper_id, cited_paper_id)
            );

            CREATE INDEX IF NOT EXISTS idx_citations_cited ON citations(cited_paper_id);
            CREATE INDEX IF NOT EXISTS idx_papers_year ON papers(year);
            CREATE INDEX IF NOT EXISTS idx_paper_authors_author ON paper_authors(author_id);"
        )?;
        Ok(Arc::new(Self { conn: Arc::new(std::sync::Mutex::new(conn)) }))
    }

    fn store_paper(&self, paper: &Paper) -> Result<(), anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO papers (paper_id, title, abstract_text, year, citation_count, reference_count, url, venue, publication_date, external_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                paper.paper_id,
                paper.title,
                paper.abstract_text,
                paper.year,
                paper.citation_count,
                paper.reference_count,
                paper.url,
                paper.venue,
                paper.publication_date,
                paper.external_ids.as_ref().map(|v| v.to_string()),
            ],
        )?;
        if let Some(ref authors) = paper.authors {
            for (pos, author) in authors.iter().enumerate() {
                if let Some(ref aid) = author.author_id {
                    if let Some(ref name) = author.name {
                        conn.execute(
                            "INSERT OR IGNORE INTO authors (author_id, name) VALUES (?1, ?2)",
                            rusqlite::params![aid, name],
                        )?;
                    }
                    conn.execute(
                        "INSERT OR REPLACE INTO paper_authors (paper_id, author_id, position) VALUES (?1, ?2, ?3)",
                        rusqlite::params![paper.paper_id, aid, pos as i32],
                    )?;
                }
            }
        }
        Ok(())
    }

    fn store_citation(&self, citing_id: &str, cited_id: &str) -> Result<(), anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO citations (citing_paper_id, cited_paper_id) VALUES (?1, ?2)",
            rusqlite::params![citing_id, cited_id],
        )?;
        Ok(())
    }

    fn get_paper(&self, paper_id: &str) -> Result<Option<Paper>, anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT paper_id, title, abstract_text, year, citation_count, reference_count, url, venue, publication_date, external_ids FROM papers WHERE paper_id = ?1",
            [paper_id],
            |row| {
                let paper_id: String = row.get(0)?;
                let title: Option<String> = row.get(1)?;
                let abstract_text: Option<String> = row.get(2)?;
                let year: Option<i32> = row.get(3)?;
                let citation_count: Option<i64> = row.get(4)?;
                let reference_count: Option<i64> = row.get(5)?;
                let url: Option<String> = row.get(6)?;
                let venue: Option<String> = row.get(7)?;
                let publication_date: Option<String> = row.get(8)?;
                let external_ids_str: Option<String> = row.get(9)?;
                let external_ids: Option<serde_json::Value> = external_ids_str
                    .and_then(|s| serde_json::from_str(&s).ok());
                Ok(Paper { paper_id, title, abstract_text, year, citation_count, reference_count, url, authors: None, venue, publication_date, external_ids })
            },
        );
        match result {
            Ok(paper) => Ok(Some(paper)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_stats(&self) -> Result<serde_json::Value, anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        let papers: i64 = conn.query_row("SELECT COUNT(*) FROM papers", [], |row| row.get(0))?;
        let authors: i64 = conn.query_row("SELECT COUNT(*) FROM authors", [], |row| row.get(0))?;
        let citations: i64 = conn.query_row("SELECT COUNT(*) FROM citations", [], |row| row.get(0))?;
        Ok(serde_json::json!({ "papers": papers, "authors": authors, "citations": citations }))
    }

    fn traverse_graph(&self, seeds: &[String]) -> Result<serde_json::Value, anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        for seed in seeds {
            if !visited.contains(seed) {
                visited.insert(seed.clone());
                queue.push_back(seed.clone());
            }
        }

        while let Some(pid) = queue.pop_front() {
            if let Ok(paper) = conn.query_row(
                "SELECT paper_id, title, year, citation_count FROM papers WHERE paper_id = ?1",
                [&pid],
                |row| Ok(GraphNode {
                    paper_id: row.get(0)?,
                    title: row.get(1)?,
                    year: row.get(2)?,
                    citation_count: row.get(3)?,
                }),
            ) {
                nodes.push(paper);
            }

            let mut citing_stmt = conn.prepare("SELECT citing_paper_id FROM citations WHERE cited_paper_id = ?1")?;
            let citing: Vec<String> = citing_stmt.query_map([&pid], |row| row.get(0))?.filter_map(|r| r.ok()).collect();
            for cid in &citing {
                edges.push(GraphEdge { source: cid.clone(), target: pid.clone(), edge_type: "cites".to_string() });
                if !visited.contains(cid) {
                    visited.insert(cid.clone());
                    queue.push_back(cid.clone());
                }
            }

            let mut cited_stmt = conn.prepare("SELECT cited_paper_id FROM citations WHERE citing_paper_id = ?1")?;
            let cited: Vec<String> = cited_stmt.query_map([&pid], |row| row.get(0))?.filter_map(|r| r.ok()).collect();
            for cid in &cited {
                edges.push(GraphEdge { source: pid.clone(), target: cid.clone(), edge_type: "cites".to_string() });
                if !visited.contains(cid) {
                    visited.insert(cid.clone());
                    queue.push_back(cid.clone());
                }
            }
        }

        Ok(serde_json::json!({ "nodes": nodes, "edges": edges, "seed_count": seeds.len() }))
    }
}

// =============================================================================
// PersistingScholarApi decorator
// =============================================================================

struct PersistingScholarApi {
    inner: Box<dyn ScholarApi>,
    store: Arc<ScholarStore>,
}

impl PersistingScholarApi {
    fn persist_paper(&self, paper: &Paper) {
        if let Err(e) = self.store.store_paper(paper) {
            tracing::warn!(paper_id = %paper.paper_id, error = %e, "Failed to persist paper to local store");
        }
        if let Some(ref authors) = paper.authors {
            for author in authors {
                if let (Some(aid), Some(name)) = (&author.author_id, &author.name) {
                    let author_obj = Author { author_id: aid.clone(), name: Some(name.clone()), paper_count: None, citation_count: None, h_index: None, url: None };
                    let store = self.store.clone();
                    let _author_json = serde_json::to_value(&author_obj).ok();
                    let _ = store.conn.lock().unwrap().execute(
                        "INSERT OR IGNORE INTO authors (author_id, name) VALUES (?1, ?2)",
                        rusqlite::params![aid, name],
                    );
                }
            }
        }
    }
}

#[async_trait]
impl ScholarApi for PersistingScholarApi {
    async fn get_paper(&self, paper_id: &str, fields: Option<&str>) -> Result<Paper, ScholarError> {
        let result = self.inner.get_paper(paper_id, fields).await?;
        self.persist_paper(&result);
        Ok(result)
    }

    async fn get_papers_batch(&self, ids: &[&str], fields: Option<&str>) -> Result<Vec<Option<Paper>>, ScholarError> {
        let results = self.inner.get_papers_batch(ids, fields).await?;
        for paper in results.iter().flatten() {
            self.persist_paper(paper);
        }
        Ok(results)
    }

    async fn search_papers(&self, query: &str, limit: Option<u32>, offset: Option<u32>, fields: Option<&str>) -> Result<SearchResult, ScholarError> {
        let result = self.inner.search_papers(query, limit, offset, fields).await?;
        for paper in &result.papers {
            self.persist_paper(paper);
        }
        Ok(result)
    }

    async fn list_citations(&self, paper_id: &str, offset: Option<u32>, limit: Option<u32>, fields: Option<&str>) -> Result<Vec<CitationEntry>, ScholarError> {
        let result = self.inner.list_citations(paper_id, offset, limit, fields).await?;
        for entry in &result {
            self.persist_paper(&entry.paper);
            let _ = self.store.store_citation(&entry.paper.paper_id, paper_id);
        }
        Ok(result)
    }

    async fn list_references(&self, paper_id: &str, offset: Option<u32>, limit: Option<u32>, fields: Option<&str>) -> Result<Vec<ReferenceEntry>, ScholarError> {
        let result = self.inner.list_references(paper_id, offset, limit, fields).await?;
        for entry in &result {
            self.persist_paper(&entry.paper);
            let _ = self.store.store_citation(paper_id, &entry.paper.paper_id);
        }
        Ok(result)
    }

    async fn get_author(&self, author_id: &str, fields: Option<&str>) -> Result<Author, ScholarError> {
        let result = self.inner.get_author(author_id, fields).await?;
        if let Err(e) = self.store.conn.lock().unwrap().execute(
            "INSERT OR REPLACE INTO authors (author_id, name, paper_count, citation_count, h_index, url) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![result.author_id, result.name, result.paper_count, result.citation_count, result.h_index, result.url],
        ) {
            tracing::warn!(author_id = %result.author_id, error = %e, "Failed to persist author");
        }
        Ok(result)
    }

    async fn recommend(&self, positive_ids: &[&str], negative_ids: &[&str]) -> Result<Vec<Paper>, ScholarError> {
        let result = self.inner.recommend(positive_ids, negative_ids).await?;
        for paper in &result {
            self.persist_paper(paper);
        }
        Ok(result)
    }
}

// =============================================================================
// Graph Segment (L3 composite — BFS traversal)
// =============================================================================

async fn compute_graph_segment(
    api: &dyn ScholarApi,
    seeds: &[String],
    depth: u32,
    mode: &str,
    budget: u32,
) -> Result<GraphSegment, ScholarError> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut upstream_calls: u32 = 0;
    let mut truncated = false;

    let mut current_level: Vec<String> = seeds.to_vec();
    for seed in seeds {
        visited.insert(seed.clone());
    }

    for d in 0..=depth {
        if current_level.is_empty() {
            break;
        }

        if upstream_calls >= budget {
            truncated = true;
            break;
        }

        let level_ids: Vec<&str> = current_level.iter().map(|s| s.as_str()).collect();

        let papers = if mode == "Full" && !level_ids.is_empty() {
            let batch: Vec<Option<Paper>> = api.get_papers_batch(&level_ids, Some(DEFAULT_FIELDS)).await?;
            upstream_calls += 1;
            batch.into_iter().filter_map(|p| p).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        for paper in &papers {
            nodes.push(GraphNode {
                paper_id: paper.paper_id.clone(),
                title: paper.title.clone(),
                year: paper.year,
                citation_count: paper.citation_count,
            });
        }

        let mut next_level = Vec::new();

        for pid in &current_level {
            if upstream_calls >= budget {
                truncated = true;
                break;
            }

            let refs = match api.list_references(pid, Some(0), Some(GRAPH_MAX_FANOUT as u32), Some("paperId,title,year,citationCount")).await {
                Ok(r) => { upstream_calls += 1; r }
                Err(e) => {
                    tracing::warn!(paper_id = %pid, error = %e, "Failed to fetch references");
                    continue;
                }
            };

            for entry in refs.iter().take(GRAPH_MAX_FANOUT) {
                edges.push(GraphEdge {
                    source: pid.clone(),
                    target: entry.paper.paper_id.clone(),
                    edge_type: "references".to_string(),
                });
                if !visited.contains(&entry.paper.paper_id) {
                    visited.insert(entry.paper.paper_id.clone());
                    next_level.push(entry.paper.paper_id.clone());
                }
            }

            let cits = match api.list_citations(pid, Some(0), Some(50_u32), Some("paperId,title,year,citationCount")).await {
                Ok(c) => { upstream_calls += 1; c }
                Err(e) => {
                    tracing::warn!(paper_id = %pid, error = %e, "Failed to fetch citations");
                    continue;
                }
            };

            for entry in cits.iter().take(GRAPH_MAX_FANOUT) {
                edges.push(GraphEdge {
                    source: entry.paper.paper_id.clone(),
                    target: pid.clone(),
                    edge_type: "cites".to_string(),
                });
                if !visited.contains(&entry.paper.paper_id) {
                    visited.insert(entry.paper.paper_id.clone());
                    next_level.push(entry.paper.paper_id.clone());
                }
            }
        }

        if d < depth {
            current_level = next_level;
        }
    }

    Ok(GraphSegment {
        seeds: seeds.to_vec(),
        depth,
        mode: mode.to_string(),
        nodes,
        edges,
        truncated,
        upstream_calls,
    })
}

// =============================================================================
// ScholarServer
// =============================================================================

pub struct ScholarServer {
    api: PersistingScholarApi,
    store: Arc<ScholarStore>,
}

impl ScholarServer {
    fn new(s2_api_key: Option<String>) -> Result<Self, anyhow::Error> {
        let db_path = std::env::var("HKASK_SCHOLAR_DB").unwrap_or_else(|_| "hkask-scholar.db".to_string());
        let store = ScholarStore::new(&db_path)?;
        let http_api = HttpScholarApi::new(s2_api_key);
        let persisting = PersistingScholarApi {
            inner: Box::new(http_api),
            store: store.clone(),
        };
        Ok(Self { api: persisting, store })
    }
}

#[tool_router(server_handler)]
impl ScholarServer {
    #[tool(description = "Liveness and API health check")]
    async fn scholar_ping(&self) -> String {
        let stats = self.store.get_stats().unwrap_or(serde_json::json!({}));
        McpToolOutput::new(serde_json::json!({
            "status": "ok",
            "version": SERVER_VERSION,
            "store": stats,
        }))
        .to_json_string()
    }

    #[tool(description = "Search Semantic Scholar for papers by relevance")]
    async fn scholar_search(
        &self,
        Parameters(SearchRequest { query, limit, offset, fields }): Parameters<SearchRequest>,
    ) -> String {
        let start = Instant::now();
        if query.is_empty() {
            return McpToolError::invalid_argument("query must not be empty").to_json_string();
        }
        match self.api.search_papers(&query, limit, offset, fields.as_deref()).await {
            Ok(result) => {
                emit_tool_span("scholar_search", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::to_value(&result).unwrap_or_default(), start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("scholar_search", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Get full paper metadata by paper ID")]
    async fn scholar_paper_details(
        &self,
        Parameters(PaperDetailsRequest { paper_id, fields }): Parameters<PaperDetailsRequest>,
    ) -> String {
        let start = Instant::now();
        if let Err(e) = validate_identifier("paper_id", &paper_id, 256) {
            return e.to_json_string();
        }
        match self.api.get_paper(&paper_id, fields.as_deref()).await {
            Ok(paper) => {
                emit_tool_span("scholar_paper_details", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::to_value(&paper).unwrap_or_default(), start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("scholar_paper_details", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Get metadata for up to 500 papers in one call")]
    async fn scholar_paper_batch(
        &self,
        Parameters(PaperBatchRequest { paper_ids, fields }): Parameters<PaperBatchRequest>,
    ) -> String {
        let start = Instant::now();
        if paper_ids.is_empty() {
            return McpToolError::invalid_argument("paper_ids must not be empty").to_json_string();
        }
        if paper_ids.len() > BATCH_MAX {
            return McpToolError::invalid_argument(format!("paper_ids exceeds maximum of {BATCH_MAX}")).to_json_string();
        }
        let ids: Vec<&str> = paper_ids.iter().map(|s| s.as_str()).collect();
        match self.api.get_papers_batch(&ids, fields.as_deref()).await {
            Ok(results) => {
                emit_tool_span("scholar_paper_batch", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::json!({ "results": results }), start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("scholar_paper_batch", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Get papers that cite a given paper")]
    async fn scholar_citations(
        &self,
        Parameters(CitationsRequest { paper_id, limit, offset, fields }): Parameters<CitationsRequest>,
    ) -> String {
        let start = Instant::now();
        if let Err(e) = validate_identifier("paper_id", &paper_id, 256) {
            return e.to_json_string();
        }
        match self.api.list_citations(&paper_id, offset, limit, fields.as_deref()).await {
            Ok(citations) => {
                emit_tool_span("scholar_citations", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::json!({ "citations": citations }), start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("scholar_citations", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Get papers cited by a given paper")]
    async fn scholar_references(
        &self,
        Parameters(ReferencesRequest { paper_id, limit, offset, fields }): Parameters<ReferencesRequest>,
    ) -> String {
        let start = Instant::now();
        if let Err(e) = validate_identifier("paper_id", &paper_id, 256) {
            return e.to_json_string();
        }
        match self.api.list_references(&paper_id, offset, limit, fields.as_deref()).await {
            Ok(references) => {
                emit_tool_span("scholar_references", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::json!({ "references": references }), start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("scholar_references", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Get author profile and stats")]
    async fn scholar_author(
        &self,
        Parameters(AuthorRequest { author_id, fields }): Parameters<AuthorRequest>,
    ) -> String {
        let start = Instant::now();
        if let Err(e) = validate_identifier("author_id", &author_id, 256) {
            return e.to_json_string();
        }
        match self.api.get_author(&author_id, fields.as_deref()).await {
            Ok(author) => {
                emit_tool_span("scholar_author", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::to_value(&author).unwrap_or_default(), start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("scholar_author", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Get recommended papers based on seed papers")]
    async fn scholar_recommendations(
        &self,
        Parameters(RecommendationsRequest { positive_paper_ids, negative_paper_ids }): Parameters<RecommendationsRequest>,
    ) -> String {
        let start = Instant::now();
        if positive_paper_ids.is_empty() {
            return McpToolError::invalid_argument("positive_paper_ids must not be empty").to_json_string();
        }
        let pos: Vec<&str> = positive_paper_ids.iter().map(|s| s.as_str()).collect();
        let neg: Vec<&str> = negative_paper_ids.as_ref().map(|v| v.iter().map(|s| s.as_str()).collect()).unwrap_or_default();
        match self.api.recommend(&pos, &neg).await {
            Ok(papers) => {
                emit_tool_span("scholar_recommendations", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::json!({ "papers": papers }), start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("scholar_recommendations", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "BFS traversal of citation graph with configurable depth")]
    async fn scholar_graph_segment(
        &self,
        Parameters(GraphSegmentRequest { seeds, query, depth, mode, budget }): Parameters<GraphSegmentRequest>,
    ) -> String {
        let start = Instant::now();

        let effective_seeds = match (seeds, query) {
            (Some(s), _) if !s.is_empty() => s,
            (_, Some(q)) => {
                match self.api.search_papers(&q, Some(5), None, Some("paperId")).await {
                    Ok(result) => result.papers.iter().map(|p| p.paper_id.clone()).collect(),
                    Err(e) => {
                        emit_tool_span("scholar_graph_segment", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                        return McpToolError::from(e).to_json_string();
                    }
                }
            }
            _ => return McpToolError::invalid_argument("seeds or query must be provided").to_json_string(),
        };

        let depth = depth.unwrap_or(1).min(GRAPH_MAX_DEPTH);
        let mode = mode.unwrap_or_else(|| "Full".to_string());
        let budget = budget.unwrap_or(GRAPH_DEFAULT_BUDGET);

        match compute_graph_segment(&self.api, &effective_seeds, depth, &mode, budget).await {
            Ok(segment) => {
                emit_tool_span("scholar_graph_segment", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::to_value(&segment).unwrap_or_default(), start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("scholar_graph_segment", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Read a paper from local store (offline)")]
    async fn scholar_store_get_paper(
        &self,
        Parameters(StoreGetPaperRequest { paper_id }): Parameters<StoreGetPaperRequest>,
    ) -> String {
        let start = Instant::now();
        match self.store.get_paper(&paper_id) {
            Ok(Some(paper)) => {
                emit_tool_span("scholar_store_get_paper", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::to_value(&paper).unwrap_or_default(), start).to_json_string()
            }
            Ok(None) => McpToolError::not_found(format!("Paper '{paper_id}' not found in local store")).to_json_string(),
            Err(e) => {
                emit_tool_span("scholar_store_get_paper", "error", start.elapsed().as_millis() as u64, Some(&McpErrorKind::Internal));
                McpToolError::internal(e.to_string()).to_json_string()
            }
        }
    }

    #[tool(description = "Traverse locally-persisted citation graph")]
    async fn scholar_store_graph(
        &self,
        Parameters(StoreGraphRequest { seeds }): Parameters<StoreGraphRequest>,
    ) -> String {
        let start = Instant::now();
        match self.store.traverse_graph(&seeds) {
            Ok(result) => {
                emit_tool_span("scholar_store_graph", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(result, start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("scholar_store_graph", "error", start.elapsed().as_millis() as u64, Some(&McpErrorKind::Internal));
                McpToolError::internal(e.to_string()).to_json_string()
            }
        }
    }

    #[tool(description = "Summary counts of local store")]
    async fn scholar_store_stats(&self) -> String {
        match self.store.get_stats() {
            Ok(stats) => McpToolOutput::new(stats).to_json_string(),
            Err(e) => McpToolError::internal(e.to_string()).to_json_string(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let s2_key = resolve_credential("HKASK_SEMANTIC_SCHOLAR_API_KEY").ok();

    run_stdio_server(
        "hkask-mcp-scholar",
        SERVER_VERSION,
        || ScholarServer::new(s2_key.clone()),
        vec![CredentialRequirement::required(
            "HKASK_SEMANTIC_SCHOLAR_API_KEY",
            "Semantic Scholar API key for upstream tools (store tools work without it)",
        )],
    )
    .await
}