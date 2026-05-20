//! hKask API — HTTP API with OpenAPI
//!
//! **Endpoints:**
//! - `GET /api/templates` — List templates
//! - `GET /api/templates/:id` — Get template
//! - `POST /api/templates` — Register template
//! - `GET /api/templates/search/:term` — Search templates by lexicon
//! - `GET /api/bots/:id/capabilities` — List bot capabilities
//! - `POST /api/bots/:id/grant` — Grant capability
//! - `GET /api/mcp/servers` — List MCP servers
//! - `GET /api/mcp/tools` — List tools
//! - `GET /api/mcp/tools/:name` — Get tool definition
//! - `GET /api/cns/health` — CNS health status
//! - `GET /api/cns/alerts` — Algedonic alerts
//! - `GET /api/cns/variety` — CNS variety counters
//! - `POST /api/chat` — Curator chat

use hkask_templates::SqliteRegistry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

pub mod openapi;
pub mod routes;

use openapi::ApiDoc;

/// API state
#[derive(Clone)]
pub struct ApiState {
    /// Template registry
    pub registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,
    /// MCP runtime
    pub mcp_runtime: Arc<hkask_mcp::runtime::McpRuntime>,
}

impl ApiState {
    pub fn new(registry: SqliteRegistry, mcp_runtime: hkask_mcp::runtime::McpRuntime) -> Self {
        Self {
            registry: Arc::new(tokio::sync::Mutex::new(registry)),
            mcp_runtime: Arc::new(mcp_runtime),
        }
    }
}

/// Template response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TemplateResponse {
    pub id: String,
    pub template_type: String,
    pub description: String,
    pub source_path: String,
    pub lexicon_terms: Vec<String>,
}

/// Capability request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GrantCapabilityRequest {
    pub capability: String,
}

/// CNS health response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CnsHealthResponse {
    pub overall_deficit: u64,
    pub critical_count: usize,
    pub warning_count: usize,
    pub healthy: bool,
}

/// Chat request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChatRequest {
    pub input: String,
    pub template_id: Option<String>,
}

/// Chat response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChatResponse {
    pub output: String,
    pub template_id: String,
}

/// CNS variety counter response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VarietyCounterResponse {
    pub variety: u64,
    pub total: u64,
    pub entropy: f64,
}

/// CNS variety response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CnsVarietyResponse {
    pub domains: Vec<String>,
    pub total_deficit: u64,
    pub counters: HashMap<String, VarietyCounterResponse>,
}

/// Tool response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ToolResponse {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_id: String,
}

/// Error response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub details: Option<serde_json::Value>,
}

/// Create API router with OpenAPI documentation
pub fn create_router(state: ApiState) -> OpenApiRouter {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::templates_router().into())
        .merge(routes::bots_router().into())
        .merge(routes::mcp_router().into())
        .merge(routes::cns_router().into())
        .merge(routes::chat_router().into())
        .with_state(state)
}

/// Build OpenAPI spec
pub fn create_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_state_new() {
        let registry = SqliteRegistry::new(None).unwrap();
        let mcp_runtime = hkask_mcp::runtime::McpRuntime::new();
        let state = ApiState::new(registry, mcp_runtime);
        assert_eq!(state.mcp_runtime.tool_count().await, 0);
    }
}
