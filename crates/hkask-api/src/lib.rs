//! hKask API — HTTP API with OpenAPI
//!
//! **Endpoints:**
//! - `GET /api/templates` — List templates
//! - `GET /api/templates/:id` — Get template
//! - `POST /api/templates` — Register template
//! - `GET /api/bots/:id/capabilities` — List bot capabilities
//! - `POST /api/bots/:id/grant` — Grant capability
//! - `GET /api/mcp/servers` — List MCP servers
//! - `GET /api/mcp/tools` — List tools
//! - `GET /api/cns/health` — CNS health status
//! - `GET /api/cns/alerts` — Algedonic alerts
//! - `POST /api/chat` — Curator chat

use axum::routing::Router;
use hkask_templates::SqliteRegistry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa::ToSchema;

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

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .merge(routes::templates_router())
        .merge(routes::bots_router())
        .merge(routes::mcp_router())
        .merge(routes::cns_router())
        .merge(routes::chat_router())
        .with_state(state)
}

/// Build OpenAPI spec
pub fn create_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

