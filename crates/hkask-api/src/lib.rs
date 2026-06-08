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

mod ensemble;
mod gas;
mod git_cas;
mod governed_tool;
mod loop_system;
mod stores;

pub mod error;
pub mod middleware;
pub mod openapi;
pub mod routes;

pub use error::ApiError;
pub use stores::DbConfig;

// Re-export route types for OpenAPI schema generation
pub use routes::{AcpRegisterRequest, AcpRegisterResponse};
pub use routes::{
    ChatRequest, ChatResponse, CnsHealthResponse, CnsVarietyResponse, CreatePodRequest,
    CreatePodResponse, GrantCapabilityRequest, ListPodsResponse, ModelEntry, ModelListResponse,
    ModelSearchQuery, PodStatusResponse, SpecCaptureRequest, SpecCaptureResponse,
    SpecCultivateResponse, SpecListResponse, SpecValidateRequest, SpecValidateResponse,
    TemplateResponse, VarietyCounterResponse,
};

use std::collections::HashMap;
use std::sync::Arc;

use hkask_agents::acp::AcpRuntime;
use hkask_agents::adapters::mcp_runtime::FullMcpAdapter;
use hkask_agents::adapters::memory_loop_adapter::MemoryLoopAdapter;
use hkask_agents::consent::ConsentManager;
use hkask_agents::escalation::EscalationQueue;
use hkask_agents::loop_system::LoopSystem;
use hkask_agents::pod::PodManager;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::CnsRuntime;
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_storage::{EmbeddingStore, TripleStore};
use hkask_templates::SqliteRegistry;
use hkask_types::event::NuEventSink;
use hkask_types::ports::InferencePort;
use hkask_types::{CapabilityChecker, WebID};
use utoipa::OpenApi;

use ensemble::{EnsembleSession, build_ensemble_session};
use git_cas::{GitCasBundle, init_git_cas};
use governed_tool::{GovernedMcpTool, build_governed_mcp_tool};
use loop_system::build_loop_system;
use stores::Stores;

use openapi::ApiDoc;

/// API state
#[derive(Clone)]
pub struct ApiState {
    /// Template registry
    pub registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,
    /// MCP runtime
    pub mcp_runtime: Arc<hkask_mcp::runtime::McpRuntime>,
    /// MCP dispatcher for OCAP-protected tool invocation
    pub mcp_dispatcher: Arc<hkask_mcp::dispatch::McpDispatcher>,
    /// Pod manager
    pub pod_manager: Arc<PodManager>,
    /// Capability checker for OCAP verification
    pub capability_checker: Arc<CapabilityChecker>,
    /// System WebID for signing capabilities
    pub system_webid: WebID,
    /// CNS span emitter for audit trail
    /// Ensemble inferencer (optional — for ensemble inference)
    pub ensemble_inferencer: Option<Arc<hkask_ensemble::adapters::InferencePortAdapter>>,
    /// Spec store for DDMVSS specifications
    pub spec_store: Option<Arc<dyn hkask_storage::SpecStore + Send + Sync>>,
    /// Consent manager for user sovereignty
    pub consent_manager: Arc<ConsentManager>,
    /// Escalation queue for Curator escalations
    pub escalation_queue: Arc<EscalationQueue>,
    /// Git CAS adapter for template archival (legacy — template loading only)
    pub git_cas: Arc<hkask_mcp::GitCasAdapter>,
    /// Git CAS port for all CAS operations (hexagonal boundary)
    pub git_cas_port: Arc<dyn hkask_types::ports::git_cas::GitCASPort>,
    /// Standing ensemble sessions (keyed by session ID)
    pub standing_sessions: Arc<
        tokio::sync::RwLock<
            HashMap<String, Arc<tokio::sync::RwLock<hkask_ensemble::StandingSession>>>,
        >,
    >,
    /// Standing session storage port (persistent or in-memory)
    pub standing_session_store: Arc<hkask_storage::StandingSessionStore>,
    /// Ensemble session manager for chat/deliberation
    pub session_manager: Arc<tokio::sync::RwLock<hkask_ensemble::SessionManager>>,
    /// Goal repository for the goal coordination substrate. Mirrors the CLI
    /// `kask goal` surface for MCP ≡ CLI ≡ API parity.
    pub goal_repo: Arc<hkask_storage::SqliteGoalRepository>,
    /// Loop system for 6-loop regulation (Cybernetics, Episodic, Semantic, Curation, Snapshot)
    pub loop_system: Arc<LoopSystem>,
    /// Episodic memory storage — private, agent-scoped (via port trait)
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,
    /// CNS runtime for real-time variety and health data
    pub cns_runtime: Arc<CnsRuntime>,
    /// General-purpose inference port (shared across requests)
    pub inference_port: Option<Arc<dyn InferencePort>>,
    /// Service configuration for InferenceService calls.
    pub service_config: hkask_services::ServiceConfig,
    /// CNS gas governance port for ensemble sessions.
    /// Wired through the CyberneticsLoop so CNS can sense ensemble gas usage.
    pub gas_governance: Arc<dyn hkask_ensemble::GasGovernancePort>,
}

impl ApiState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: SqliteRegistry,
        mcp_runtime: hkask_mcp::runtime::McpRuntime,
        pod_manager: PodManager,
        capability_secret: &[u8],
        system_webid: WebID,
        ensemble_inferencer: Option<Arc<hkask_ensemble::adapters::InferencePortAdapter>>,
        db_config: Option<&DbConfig>,
        acp: Option<Arc<dyn hkask_agents::ports::AcpPort>>,
    ) -> Result<Self, ApiError> {
        // ── 1. Persistent stores ──
        // git_cas_port is created before stores so each can attach it for CAS write-through.
        let GitCasBundle {
            git_cas,
            git_cas_port,
        } = init_git_cas()?;
        let stores = Stores::init(db_config, Arc::clone(&git_cas_port))?;

        // ── 2. Loop system + CNS event sink ──
        let dispatch = Arc::new(hkask_agents::communication::dispatch::MessageDispatch::new());
        let inference_port_for_loops: Option<Arc<dyn InferencePort>> =
            ensemble_inferencer.as_ref().map(|ei| Arc::clone(ei.port()));
        let cns_event_conn = hkask_storage::in_memory_db().conn_arc();
        let cns_event_sink: Arc<dyn NuEventSink> =
            Arc::new(hkask_storage::NuEventStore::new(cns_event_conn));
        let (loop_system, episodic_storage, cybernetics_loop_rwlock) = build_loop_system(
            Arc::clone(&stores.escalation_queue),
            dispatch,
            inference_port_for_loops,
            system_webid,
            acp,
            Some(Arc::clone(&cns_event_sink)),
            Arc::clone(&git_cas_port),
        )?;

        // ── 3. GovernedTool membrane + McpDispatcher ──
        let GovernedMcpTool {
            mcp_dispatcher,
            cybernetics_loop_for_gas,
        } = build_governed_mcp_tool(
            mcp_runtime.clone(),
            cybernetics_loop_rwlock,
            cns_event_sink,
            &loop_system,
            system_webid,
            capability_secret,
        );

        // ── 4. Ensemble session manager with CNS gas governance ──
        let EnsembleSession {
            session_manager,
            gas_governance,
            inference_port,
            ensemble_inferencer,
        } = build_ensemble_session(ensemble_inferencer, cybernetics_loop_for_gas, system_webid);

        Ok(Self {
            registry: Arc::new(tokio::sync::Mutex::new(registry)),
            mcp_runtime: Arc::new(mcp_runtime),
            mcp_dispatcher,
            pod_manager: Arc::new(pod_manager),
            capability_checker: Arc::new(CapabilityChecker::new(capability_secret)),
            system_webid,
            ensemble_inferencer, // returned from build_ensemble_session
            spec_store: None,
            consent_manager: stores.consent_manager,
            escalation_queue: stores.escalation_queue,
            git_cas,
            git_cas_port,
            standing_sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            standing_session_store: stores.standing_session_store,
            session_manager,
            goal_repo: stores.goal_repo,
            loop_system,
            episodic_storage,
            cns_runtime: Arc::new(CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD)),
            inference_port,
            service_config: hkask_services::ServiceConfig::from_env().unwrap_or_else(|e| {
                tracing::warn!(target: "hkask.api", error = %e, "Failed to resolve service config from env, using in-memory");
                hkask_services::ServiceConfig::in_memory()
            }),
            gas_governance,
        })
    }

    /// Create ApiState with default adapters.
    ///
    /// The `acp_secret` is the HMAC secret for ACP token signing. It should be
    /// derived from the master key (via `hkask_keystore::master_key::derive_all_internal_secrets`)
    /// or resolved from the environment/keychain (via `hkask_keystore::resolve`).
    ///
    /// The API server is headless and cannot run interactive onboarding — the caller
    /// is responsible for providing a valid ACP secret. Run `kask chat` interactively
    /// first to complete onboarding and store secrets.
    pub fn with_defaults(
        registry: SqliteRegistry,
        mcp_runtime: hkask_mcp::runtime::McpRuntime,
        capability_secret: &[u8],
        acp_secret: &[u8],
        system_webid: WebID,
        db_config: Option<&DbConfig>,
    ) -> Result<Self, ApiError> {
        let git_cas =
            hkask_mcp::GitCasAdapter::from_path(std::path::PathBuf::from("/tmp/hkask-templates"));
        let acp_runtime = Arc::new(AcpRuntime::new(acp_secret));
        let acp_port: Arc<dyn hkask_agents::ports::AcpPort> = acp_runtime.clone();
        let mcp_runtime_adapter = FullMcpAdapter::new(
            Arc::new(CapabilityChecker::new(acp_secret)),
            Arc::new(mcp_runtime.clone()),
            tokio::runtime::Handle::current(),
        );

        // Use MemoryLoopAdapter (routes through hkask-memory domain logic)
        let db = hkask_storage::in_memory_db();
        let conn = db.conn_arc();
        let triple_store = TripleStore::new(Arc::clone(&conn));
        let episodic_memory_for_adapter = EpisodicMemory::new(triple_store);
        let triple_store2 = TripleStore::new(Arc::clone(&conn));
        let embedding_store = EmbeddingStore::new(conn);
        let semantic_memory = SemanticMemory::new(triple_store2, embedding_store);
        let memory_adapter = Arc::new(MemoryLoopAdapter::new(
            episodic_memory_for_adapter,
            semantic_memory,
        ));
        let episodic_storage: Arc<dyn EpisodicStoragePort> = memory_adapter.clone();
        let semantic_storage: Arc<dyn SemanticStoragePort> = memory_adapter.clone();
        let pod_manager = PodManager::new(
            Arc::new(git_cas),
            acp_runtime,
            Arc::new(mcp_runtime_adapter),
            episodic_storage,
            semantic_storage,
        )
        .with_capability_checker(CapabilityChecker::new(acp_secret));
        Self::new(
            registry,
            mcp_runtime,
            pod_manager,
            capability_secret,
            system_webid,
            None,
            db_config,
            Some(acp_port),
        )
    }

    /// Create ApiState with consent manager
    pub fn with_consent_manager(mut self, consent_manager: ConsentManager) -> Self {
        self.consent_manager = Arc::new(consent_manager);
        self
    }

    /// Create ApiState with ensemble inferencer wrapped in a circuit breaker
    pub fn with_ensemble_inferencer(
        registry: SqliteRegistry,
        mcp_runtime: hkask_mcp::runtime::McpRuntime,
        pod_manager: PodManager,
        capability_secret: &[u8],
        system_webid: WebID,
        model: &str,
        db_config: Option<&DbConfig>,
    ) -> Result<Self, ApiError> {
        let ctx = hkask_services::InferenceContext::from_parts(
            None,
            model,
            std::env::var("OKAPI_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string()),
        );
        let inference =
            hkask_services::InferenceService::resolve_port(&ctx, model).map_err(|e| {
                ApiError::Internal {
                    message: format!("Failed to create Okapi inference: {e}"),
                }
            })?;
        let adapter = Arc::new(hkask_ensemble::adapters::InferencePortAdapter::new(
            inference,
        ));
        Self::new(
            registry,
            mcp_runtime,
            pod_manager,
            capability_secret,
            system_webid,
            Some(adapter),
            db_config,
            None,
        )
    }

    /// Create a circuit-breaker-wrapped ensemble inferencer.
    ///
    /// Returns `None` if no ensemble inferencer is configured.
    /// When available, wraps the base `InferencePortAdapter` with a
    /// `CircuitBreakerInferenceAdapter` using inference-appropriate defaults.
    pub fn ensemble_inferencer_with_breaker(
        &self,
    ) -> Option<Arc<hkask_ensemble::adapters::CircuitBreakerInferenceAdapter>> {
        self.ensemble_inferencer.as_ref().map(|adapter| {
            let breaker: Arc<dyn hkask_types::ports::CircuitBreakerPort> = Arc::new(
                hkask_cns::CircuitBreaker::default_for_inference("ensemble-inference"),
            );
            Arc::new(
                hkask_ensemble::adapters::CircuitBreakerInferenceAdapter::new(
                    (**adapter).clone(),
                    breaker,
                ),
            )
        })
    }

    /// Set the ensemble session manager to share state with the CLI.
    ///
    /// By default, `ApiState::new()` creates its own `SessionManager`.
    /// Call this to replace it with the CLI's instance so both CLI and
    /// API routes operate on the same sessions. Use `SessionManager::clone_shared()`
    /// to obtain a handle from the CLI's singleton.
    pub fn with_session_manager(
        mut self,
        session_manager: Arc<tokio::sync::RwLock<hkask_ensemble::SessionManager>>,
    ) -> Self {
        self.session_manager = session_manager;
        self
    }

    /// Set the spec store for DDMVSS specifications
    pub fn with_spec_store(
        mut self,
        store: Arc<dyn hkask_storage::SpecStore + Send + Sync>,
    ) -> Self {
        self.spec_store = Some(store);
        self
    }

    /// Start the loop system (all registered loops begin their tick cycles).
    ///
    /// Call this after the API server starts listening. The loops run in
    /// background tokio tasks until `shutdown_loops()` is called.
    pub async fn start_loops(&self) -> Result<(), hkask_types::InfrastructureError> {
        tracing::info!(
            target: "hkask.api",
            loops = ?self.loop_system.registered_loop_ids().await,
            "Starting loop system"
        );
        self.loop_system.start().await
    }

    /// Signal the loop system to shut down.
    ///
    /// Call this during graceful server shutdown. All loop tick tasks
    /// will stop after their current cycle completes.
    pub fn shutdown_loops(&self) {
        tracing::info!(target: "hkask.api", "Shutting down loop system");
        self.loop_system.shutdown();
    }
}

/// Create API router with OpenAPI documentation and authentication
pub fn create_router(state: ApiState) -> Result<utoipa_axum::router::OpenApiRouter, String> {
    let auth_service = std::sync::Arc::new(
        middleware::AuthService::new()
            .map_err(|e| format!("Failed to initialize auth service: {}", e))?,
    );

    Ok(
        utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
            .merge(routes::templates_router().into())
            .merge(routes::bots_router().into())
            .merge(routes::pods_router().into())
            .merge(routes::mcp_router().into())
            .merge(routes::cns_router().into())
            .merge(routes::sovereignty_router().into())
            .merge(routes::chat_router().into())
            .merge(routes::models_router().into())
            .merge(routes::ensemble_router().into())
            .merge(routes::acp_router().into())
            .merge(routes::bundles_router().into())
            .merge(routes::spec_router().into())
            .merge(routes::curator_router().into())
            .merge(routes::episodic_router().into())
            .merge(routes::consolidation_router().into())
            .merge(routes::git_router().into())
            .merge(routes::goal_router().into())
            .layer(axum::middleware::from_fn_with_state(
                auth_service,
                middleware::auth_middleware,
            ))
            .with_state(state),
    )
}

/// Build OpenAPI spec
pub fn create_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
