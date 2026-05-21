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
//! - `GET /api/pods` — List pods
//! - `POST /api/pods` — Create pod
//! - `POST /api/pods/:id/activate` — Activate pod
//! - `POST /api/pods/:id/deactivate` — Deactivate pod
//! - `GET /api/pods/:id/status` — Get pod status
//! - `POST /api/chat` — Curator chat

use hkask_agents::adapters::acp_runtime::AcpRuntimeAdapter;
use hkask_agents::adapters::cns_emitter::CnsEmitterAdapter;
use hkask_agents::adapters::git_cas::GitCasAdapter;
use hkask_agents::adapters::memory_storage::MemoryStorageAdapter;
use hkask_agents::adapters::mcp_runtime::McpRuntimeAdapter;
use hkask_agents::pod::PodManager;
use hkask_cns::rate_limit::{RateLimitConfig, RateLimiter};
use hkask_cns::spans::SpanEmitter;
use hkask_templates::SqliteRegistry;
use hkask_types::{CapabilityChecker, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
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
    /// Pod manager
    pub pod_manager: Arc<PodManager>,
    /// Capability checker for OCAP verification
    pub capability_checker: Arc<CapabilityChecker>,
    /// System WebID for signing capabilities
    pub system_webid: WebID,
    /// CNS span emitter for audit trail
    pub cns_emitter: Arc<SpanEmitter>,
    /// Rate limiter for API endpoints
    pub rate_limiter: Arc<RateLimiter>,
}

impl ApiState {
    pub fn new(
        registry: SqliteRegistry,
        mcp_runtime: hkask_mcp::runtime::McpRuntime,
        pod_manager: PodManager,
        capability_secret: &[u8],
        system_webid: WebID,
    ) -> Self {
        let observer_webid = WebID::new();
        let rate_limiter = RateLimiter::new(RateLimitConfig {
            max_tokens: 100,
            refill_interval: std::time::Duration::from_millis(600),
        });
        Self {
            registry: Arc::new(tokio::sync::Mutex::new(registry)),
            mcp_runtime: Arc::new(mcp_runtime),
            pod_manager: Arc::new(pod_manager),
            capability_checker: Arc::new(CapabilityChecker::new(capability_secret)),
            system_webid,
            cns_emitter: Arc::new(SpanEmitter::new(observer_webid)),
            rate_limiter: Arc::new(rate_limiter),
        }
    }

    /// Create ApiState with default adapters
    pub fn with_defaults(
        registry: SqliteRegistry,
        mcp_runtime: hkask_mcp::runtime::McpRuntime,
        capability_secret: &[u8],
        system_webid: WebID,
    ) -> Self {
        let git_cas = GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates"));
        let acp_runtime = AcpRuntimeAdapter::new();
        let observer_webid = WebID::new();
        let cns_emitter_adapter = CnsEmitterAdapter::new(observer_webid);
        let mcp_runtime_adapter = McpRuntimeAdapter::new();
        let memory_storage = MemoryStorageAdapter::in_memory().unwrap();
        let pod_manager = PodManager::new(
            git_cas,
            acp_runtime,
            cns_emitter_adapter,
            mcp_runtime_adapter,
            memory_storage,
        );
        Self::new(
            registry,
            mcp_runtime,
            pod_manager,
            capability_secret,
            system_webid,
        )
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

/// Create pod request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodRequest {
    pub template: String,
    pub persona_yaml: String,
    pub name: Option<String>,
}

/// Create pod response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodResponse {
    pub pod_id: String,
}

/// Pod status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PodStatusResponse {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: String,
    pub webid: String,
    pub agent_type: String,
    pub template: String,
    pub created_at: i64,
}

/// List pods response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListPodsResponse {
    pub pods: Vec<PodStatusResponse>,
}

/// Create API router with OpenAPI documentation
pub fn create_router(state: ApiState) -> OpenApiRouter {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::templates_router().into())
        .merge(routes::bots_router().into())
        .merge(routes::pods_router().into())
        .merge(routes::mcp_router().into())
        .merge(routes::cns_router().into())
        .merge(routes::chat_router().into())
        .with_state(state)
}

/// Build OpenAPI spec
pub fn create_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
