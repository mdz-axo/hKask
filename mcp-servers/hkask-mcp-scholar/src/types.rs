//! hKask MCP Scholar — Request and domain types

use hkask_mcp::server::McpToolError;
use hkask_types::McpErrorKind;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const S2_API_BASE: &str = "https://api.semanticscholar.org/graph/v1";
pub const DEFAULT_FIELDS: &str = "paperId,title,abstract,year,citationCount,referenceCount,url,authors,venue,publicationDate,externalIds";
pub const BATCH_MAX: usize = 500;
pub const GRAPH_MAX_DEPTH: u32 = 3;
pub const GRAPH_MAX_FANOUT: usize = 100;
pub const GRAPH_DEFAULT_BUDGET: u32 = 200;

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
