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

mod energy;
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

use std::collections::HashMap;
use std::sync::Arc;

use hkask_services::AgentService;
use utoipa::OpenApi;

use energy::ApiEnergyGovernanceAdapter;
use git_cas::{GitCasBundle, init_git_cas};

use openapi::ApiDoc;

/// API state — composes `AgentService` for all shared infrastructure.
///
/// The `agent_service` field is the single source of truth for domain
/// objects. Surface-specific fields (standing sessions map, git CAS,
/// ensemble inferencer, gas governance) are the ONLY fields that don't
/// come from `AgentService`.
#[derive(Clone)]
pub struct ApiState {
    /// Agent service — single source of truth for all shared infrastructure.
    /// All domain objects (registry, escalation queue, consent manager, etc.)
    /// come from here. Surface code derives service types via domain accessors.
    pub agent_service: Arc<AgentService>,
    /// Standing ensemble sessions (keyed by session ID) — surface-specific live state
    pub standing_sessions: Arc<
        tokio::sync::RwLock<
            HashMap<String, Arc<tokio::sync::RwLock<hkask_agents::ensemble::StandingSession>>>,
        >,
    >,
    /// Ensemble inferencer (optional — for ensemble inference) — surface-specific
    pub ensemble_inferencer: Option<Arc<hkask_agents::ensemble::adapters::InferencePortAdapter>>,
    /// Spec store for MDS specifications — surface-specific
    pub spec_store: Option<Arc<hkask_storage::SqliteSpecStore>>,
    /// Git CAS adapter for template archival (legacy — template loading only) — surface-specific
    pub git_cas: Arc<hkask_mcp::GitCasAdapter>,
    /// Git CAS port for all CAS operations (hexagonal boundary) — surface-specific
    pub git_cas_port: Arc<dyn hkask_types::ports::git_cas::GitCASPort>,
    /// CNS gas governance port for ensemble sessions — surface-specific
    /// Wired through the CyberneticsLoop so CNS can sense ensemble gas usage.
    pub gas_governance: Arc<dyn hkask_agents::ensemble::GasGovernancePort>,
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
        Self::from_service_context(ctx, None).await
    }

    /// Create ApiState from a pre-built `AgentService`.
    ///
    /// This is the canonical construction path for API surfaces that compose
    /// `AgentService::build()`. All shared infrastructure (CNS, loop system,
    /// governed tool, pod manager, stores) comes from `AgentService`.
    /// Surface-specific fields (ensemble inferencer, git CAS, gas governance)
    /// are constructed from AgentService fields or initialized to defaults.
    ///
    /// # Arguments
    ///
    /// * `ctx` — A fully-wired `AgentService` from `AgentService::build(config).await`
    /// * `ensemble_inferencer` — Optional ensemble inference adapter (surface-specific)
    ///
    /// # Surface-specific fields
    ///
    /// - `ensemble_inferencer` — passed through from caller
    /// - `gas_governance` — built from AgentService's cybernetics loop
    /// - `git_cas` / `git_cas_port` — built via `init_git_cas()`
    /// - `standing_sessions` — initialized to empty map
    /// - `spec_store` — initialized to None
    /// - `cns_runtime` — cloned from AgentService's shared CnsRuntime
    pub async fn from_service_context(
        ctx: AgentService,
        ensemble_inferencer: Option<Arc<hkask_agents::ensemble::adapters::InferencePortAdapter>>,
    ) -> Result<Self, ApiError> {
        // Surface-specific: gas governance from cybernetics loop + system webid
        let cybernetics = ctx.cybernetics_loop();
        let (webid, _) = ctx.identity();
        let gas_governance: Arc<dyn hkask_agents::ensemble::GasGovernancePort> =
            Arc::new(ApiEnergyGovernanceAdapter::new(
                cybernetics.clone(),
                *webid,
                energy::API_ENSEMBLE_ENERGY_CAP,
            ));

        // Surface-specific: Git CAS adapters (legacy template archival)
        let GitCasBundle {
            git_cas,
            git_cas_port,
        } = init_git_cas()?;

        Ok(Self {
            agent_service: Arc::new(ctx),
            // Surface-specific fields only
            standing_sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            ensemble_inferencer,
            spec_store: None,
            git_cas,
            git_cas_port,
            gas_governance,
        })
    }

    /// Create a circuit-breaker-wrapped ensemble inferencer.
    ///
    /// Returns `None` if no ensemble inferencer is configured.
    /// When available, wraps the base `InferencePortAdapter` with a
    /// `CircuitBreakerInferenceAdapter` using inference-appropriate defaults.
    pub fn ensemble_inferencer_with_breaker(
        &self,
    ) -> Option<Arc<hkask_agents::ensemble::adapters::CircuitBreakerInferenceAdapter>> {
        self.ensemble_inferencer.as_ref().map(|adapter| {
            let breaker: Arc<dyn hkask_types::ports::CircuitBreakerPort> = Arc::new(
                hkask_cns::CircuitBreaker::default_for_inference("ensemble-inference"),
            );
            Arc::new(
                hkask_agents::ensemble::adapters::CircuitBreakerInferenceAdapter::new(
                    (**adapter).clone(),
                    breaker,
                ),
            )
        })
    }

    /// Set the spec store for MDS specifications
    pub fn with_spec_store(mut self, store: Arc<hkask_storage::SqliteSpecStore>) -> Self {
        self.spec_store = Some(store);
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

    Ok(
        utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
            .merge(routes::templates_router())
            .merge(routes::bots_router())
            .merge(routes::pods_router())
            .merge(routes::mcp_router())
            .merge(routes::cns_router())
            .merge(routes::sovereignty_router())
            .merge(routes::chat_router())
            .merge(routes::models_router())
            .merge(routes::ensemble_router())
            .merge(routes::acp_router())
            .merge(routes::bundles_router())
            .merge(routes::spec_router())
            .merge(routes::curator_router())
            .merge(routes::episodic_router())
            .merge(routes::consolidation_router())
            .merge(routes::git_router())
            .merge(routes::goal_router())
            .merge(routes::settings_router())
            .layer(axum::middleware::from_fn_with_state(
                auth_service,
                middleware::auth_middleware,
            ))
            .with_state(state),
    )
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
        .merge(routes::ensemble_router())
        .merge(routes::acp_router())
        .merge(routes::bundles_router())
        .merge(routes::spec_router())
        .merge(routes::curator_router())
        .merge(routes::episodic_router())
        .merge(routes::consolidation_router())
        .merge(routes::git_router())
        .merge(routes::goal_router())
        .merge(routes::settings_router());
    router.into_openapi()
}
