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
//! - `POST /api/mcp/invoke` — Invoke an MCP tool
//! - `GET /api/cns/health` — CNS health status
//! - `GET /api/cns/alerts` — Algedonic alerts
//! - `GET /api/cns/variety` — CNS variety counters
//! - `GET /api/pods` — List pods
//! - `POST /api/pods` — Create pod
//! - `POST /api/pods/:id/activate` — Activate pod
//! - `POST /api/pods/:id/deactivate` — Deactivate pod
//! - `GET /api/pods/:id/status` — Get pod status
//! - `POST /api/chat` — Curator chat
//! - `GET /api/sovereignty/status` — User sovereignty status
//! - `POST /api/sovereignty/consent/grant` — Grant explicit consent
//! - `POST /api/sovereignty/consent/revoke` — Revoke explicit consent

//! - `GET /api/sovereignty/access/check` — Check data access status
//! - `POST /api/episodic/store` — Store episodic triple
//! - `GET /api/episodic/query` — Query episodic memories
//! - `GET /api/episodic/usage` — Episodic storage usage

mod git_cas;

pub mod error;
pub mod middleware;
pub mod openapi;
pub mod routes;

pub use error::ApiError;

// Re-export route types for OpenAPI schema generation
pub use routes::{AcpRegisterRequest, AcpRegisterResponse};
pub use routes::{
    ChatRequest, ChatResponse, CnsHealthResponse, CnsVarietyResponse, CreatePodRequest,
    CreatePodResponse, GrantCapabilityRequest, ListPodsResponse, ModelEntry, ModelListResponse,
    ModelSearchQuery, PodStatusResponse, SpecCoherenceResponse, SpecListResponse,
    SpecWritingQualityResponse, TemplateResponse, VarietyCounterResponse,
};

use std::sync::Arc;

use hkask_services::AgentService;
use hkask_services::WalletService;
use utoipa::OpenApi;

use git_cas::{GitCasBundle, init_git_cas};

use openapi::ApiDoc;

/// API state — composes `AgentService` for all shared infrastructure.
///
/// The `agent_service` field is the single source of truth for domain
/// objects. Surface-specific fields (spec store, git CAS, wallet service)
/// are the ONLY fields that don't come from `AgentService`.
#[derive(Clone)]
pub struct ApiState {
    /// Agent service — single source of truth for all shared infrastructure.
    /// All domain objects (registry, escalation queue, consent manager, etc.)
    /// come from here. Surface code derives service types via domain accessors.
    pub agent_service: Arc<AgentService>,
    /// Spec store for MDS specifications — surface-specific
    pub spec_store: Option<Arc<hkask_storage::SqliteSpecStore>>,
    /// Git CAS adapter for template archival (legacy — template loading only) — surface-specific
    pub git_cas: Arc<hkask_mcp::GitCasAdapter>,
    /// Git CAS port for all CAS operations (hexagonal boundary) — surface-specific
    pub git_cas_port: Arc<dyn hkask_types::ports::git_cas::GitCASPort>,
    /// Wallet service for rJoule payments and API key management — surface-specific.
    /// Built from config during `ApiState::with_defaults()` or `from_service_context()`.
    pub wallet_service: Option<Arc<WalletService>>,
    /// API key authentication service for Bearer token verification.
    /// Built from the wallet store when a wallet service is available.
    pub api_key_auth_service: Option<Arc<middleware::api_key_auth::ApiKeyAuthService>>,
}

impl ApiState {
    /// Create ApiState with default adapters via `AgentService::build()`.
    ///
    /// Resolves configuration from environment variables and keychain,
    /// builds an `AgentService` with all shared infrastructure, then
    /// constructs `ApiState` from it with surface-specific defaults.
    ///
    /// The API server is headless and cannot run interactive onboarding — the caller
    /// is responsible for ensuring secrets are available via the keystore.
    /// Run `kask chat` interactively first to complete onboarding and store secrets.
    pub async fn with_defaults() -> Result<Self, ApiError> {
        let config = hkask_services::ServiceConfig::from_env().map_err(|e| ApiError::Internal {
            message: format!("Failed to resolve service config: {e}"),
        })?;
        let ctx = hkask_services::AgentService::build(config)
            .await
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to build service context: {e}"),
            })?;
        Self::from_service_context(ctx).await
    }

    /// Create ApiState from a pre-built `AgentService`.
    ///
    /// This is the canonical construction path for API surfaces that compose
    /// `AgentService::build()`. All shared infrastructure (CNS, loop system,
    /// governed tool, pod manager, stores) comes from `AgentService`.
    /// Surface-specific fields (git CAS) are constructed from AgentService
    /// fields or initialized to defaults.
    pub async fn from_service_context(ctx: AgentService) -> Result<Self, ApiError> {
        // Surface-specific: Git CAS adapters (legacy template archival)
        let GitCasBundle {
            git_cas,
            git_cas_port,
        } = init_git_cas()?;

        // Extract wallet service before moving ctx into Arc
        let wallet_service = ctx.wallet().cloned();
        // Build API key auth service if wallet store and wallet service are available
        let api_key_auth_service = match (ctx.wallet_store().cloned(), wallet_service.clone()) {
            (Some(store), Some(svc)) => Some(Arc::new(
                middleware::api_key_auth::ApiKeyAuthService::new(store, svc),
            )),
            _ => None,
        };

        Ok(Self {
            agent_service: Arc::new(ctx),
            // Surface-specific fields only
            spec_store: None,
            git_cas,
            git_cas_port,
            wallet_service,
            api_key_auth_service,
        })
    }

    /// Set the spec store for MDS specifications
    pub fn with_spec_store(mut self, store: Arc<hkask_storage::SqliteSpecStore>) -> Self {
        self.spec_store = Some(store);
        self
    }

    /// Attach a wallet service for rJoule payments and API key management.
    pub fn with_wallet_service(mut self, svc: Arc<WalletService>) -> Self {
        self.wallet_service = Some(svc);
        self
    }

    /// Start the loop system (all registered loops begin their tick cycles).
    ///
    /// Call this after the API server starts listening. The loops run in
    /// background tokio tasks until `shutdown_loops()` is called.
    pub async fn start_loops(&self) -> Result<(), hkask_types::InfrastructureError> {
        let loops = self.agent_service.loop_system();
        tracing::info!(
            target: "hkask.api",
            loops = ?loops.registered_loop_ids().await,
            "Starting loop system"
        );
        loops.start().await
    }

    /// Signal the loop system to shut down.
    pub fn shutdown_loops(&self) {
        tracing::info!(target: "hkask.api", "Shutting down loop system");
        let loops = self.agent_service.loop_system();
        loops.shutdown();
    }
}

/// Create API router with OpenAPI documentation and authentication
pub fn create_router(state: ApiState) -> Result<utoipa_axum::router::OpenApiRouter, String> {
    let auth_service = std::sync::Arc::new(middleware::AuthService::from_config(
        state.agent_service.config(),
    ));

    let mut router = utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::templates_router())
        .merge(routes::bots_router())
        .merge(routes::pods_router())
        .merge(routes::mcp_router())
        .merge(routes::cns_router())
        .merge(routes::sovereignty_router())
        .merge(routes::chat_router())
        .merge(routes::models_router())
        .merge(routes::acp_router())
        .merge(routes::bundles_router())
        .merge(routes::spec_router())
        .merge(routes::curator_router())
        .merge(routes::episodic_router())
        .merge(routes::consolidation_router())
        .merge(routes::git_router())
        .merge(routes::backup_router())
        .merge(routes::goal_router())
        .merge(routes::settings_router())
        .merge(routes::wallet_router())
        .layer(axum::middleware::from_fn_with_state(
            auth_service,
            middleware::auth_middleware,
        ));

    // Apply API key auth middleware if available (allows Bearer token auth on wallet routes)
    if let Some(api_key_auth) = &state.api_key_auth_service {
        router = router.layer(axum::middleware::from_fn_with_state(
            Arc::clone(api_key_auth),
            middleware::api_key_auth::api_key_auth_middleware,
        ));
    }

    Ok(router.with_state(state))
}

/// Build OpenAPI spec with all route paths collected from the router.
///
/// Builds the full `OpenApiRouter` (without state or auth middleware) to
/// collect `#[utoipa::path]` metadata from `routes!()` calls, then extracts
/// the complete OpenAPI specification including paths.
pub fn create_openapi() -> utoipa::openapi::OpenApi {
    use utoipa_axum::router::OpenApiRouter;

    let router: OpenApiRouter<ApiState> = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::templates_router())
        .merge(routes::bots_router())
        .merge(routes::pods_router())
        .merge(routes::mcp_router())
        .merge(routes::cns_router())
        .merge(routes::sovereignty_router())
        .merge(routes::chat_router())
        .merge(routes::models_router())
        .merge(routes::acp_router())
        .merge(routes::bundles_router())
        .merge(routes::spec_router())
        .merge(routes::curator_router())
        .merge(routes::episodic_router())
        .merge(routes::consolidation_router())
        .merge(routes::git_router())
        .merge(routes::backup_router())
        .merge(routes::goal_router())
        .merge(routes::settings_router())
        .merge(routes::wallet_router());
    router.into_openapi()
}
