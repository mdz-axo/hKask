//! hKask MCP Scholar — Semantic Scholar Graph API wrapper + local bitemporal persistence
//!
//! Provides academic paper search, citation graph traversal, and local SQLite storage
//! for offline access. Uses the hexagonal adapter pattern: HttpScholarApi for upstream
//! and PersistingScholarApi decorator for write-through caching.

mod api;
mod store;
mod types;

use hkask_mcp::server::{
    CredentialRequirement, McpToolError, McpToolOutput, emit_tool_span,
    resolve_credential, run_stdio_server, validate_identifier,
};
use hkask_types::McpErrorKind;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use api::{PersistingScholarApi, ScholarApi, HttpScholarApi};
use store::ScholarStore;
use types::*;

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

        if mode == "Full" && !level_ids.is_empty() {
            let batch: Vec<Option<Paper>> = api.get_papers_batch(&level_ids, Some(DEFAULT_FIELDS)).await?;
            upstream_calls += 1;
            for paper in batch.into_iter().flatten() {
                nodes.push(GraphNode {
                    paper_id: paper.paper_id.clone(),
                    title: paper.title,
                    year: paper.year,
                    citation_count: paper.citation_count,
                });
            }
        }

        let mut next_level = Vec::new();

        for pid in &current_level {
            if upstream_calls >= budget {
                truncated = true;
                break;
            }

            let refs = match api
                .list_references(
                    pid,
                    Some(0),
                    Some(GRAPH_MAX_FANOUT as u32),
                    Some("paperId,title,year,citationCount"),
                )
                .await
            {
                Ok(r) => {
                    upstream_calls += 1;
                    r
                }
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

            let cits = match api
                .list_citations(pid, Some(0), Some(50_u32), Some("paperId,title,year,citationCount"))
                .await
            {
                Ok(c) => {
                    upstream_calls += 1;
                    c
                }
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

pub struct ScholarServer {
    api: PersistingScholarApi,
    store: Arc<ScholarStore>,
}

impl ScholarServer {
    fn new(s2_api_key: Option<String>) -> Result<Self, anyhow::Error> {
        let db_path =
            std::env::var("HKASK_SCHOLAR_DB").unwrap_or_else(|_| "hkask-scholar.db".to_string());
        let store = ScholarStore::new(&db_path)?;
        let http_api = HttpScholarApi::new(s2_api_key);
        let persisting = PersistingScholarApi {
            inner: Box::new(http_api),
            store: store.clone(),
        };
        Ok(Self {
            api: persisting,
            store,
        })
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
                McpToolOutput::with_timing(serde_json::to_value(&result).unwrap_or_default(), start)
                    .to_json_string()
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
                McpToolOutput::with_timing(serde_json::to_value(&paper).unwrap_or_default(), start)
                    .to_json_string()
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
            return McpToolError::invalid_argument(format!("paper_ids exceeds maximum of {BATCH_MAX}"))
                .to_json_string();
        }
        let ids: Vec<&str> = paper_ids.iter().map(|s| s.as_str()).collect();
        match self.api.get_papers_batch(&ids, fields.as_deref()).await {
            Ok(results) => {
                emit_tool_span("scholar_paper_batch", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::json!({ "results": results }), start)
                    .to_json_string()
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
                McpToolOutput::with_timing(serde_json::json!({ "citations": citations }), start)
                    .to_json_string()
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
                McpToolOutput::with_timing(serde_json::json!({ "references": references }), start)
                    .to_json_string()
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
                McpToolOutput::with_timing(serde_json::to_value(&author).unwrap_or_default(), start)
                    .to_json_string()
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
            return McpToolError::invalid_argument("positive_paper_ids must not be empty")
                .to_json_string();
        }
        let pos: Vec<&str> = positive_paper_ids.iter().map(|s| s.as_str()).collect();
        let neg: Vec<&str> = negative_paper_ids
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        match self.api.recommend(&pos, &neg).await {
            Ok(papers) => {
                emit_tool_span("scholar_recommendations", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(serde_json::json!({ "papers": papers }), start)
                    .to_json_string()
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
                McpToolOutput::with_timing(serde_json::to_value(&segment).unwrap_or_default(), start)
                    .to_json_string()
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
                McpToolOutput::with_timing(serde_json::to_value(&paper).unwrap_or_default(), start)
                    .to_json_string()
            }
            Ok(None) => McpToolError::not_found(format!("Paper '{paper_id}' not found in local store"))
                .to_json_string(),
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
