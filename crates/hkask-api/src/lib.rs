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
//! - `GET /api/sovereignty/killzone` — Kill zone status
//! - `GET /api/sovereignty/access/check` — Check data access status
//! - `POST /api/llm/infer` — SOAP inference endpoint for Russell
//! - `POST /api/episodic/store` — Store episodic triple
//! - `GET /api/episodic/query` — Query episodic memories
//! - `GET /api/episodic/usage` — Episodic storage usage

use hkask_agents::CyberneticsLoopHandle;
use hkask_agents::acp::AcpRuntime;
use hkask_agents::adapters::git_cas::GitCasAdapter;
use hkask_agents::adapters::mcp_runtime::McpRuntimeAdapter;
use hkask_agents::adapters::memory_loop_adapter::MemoryLoopAdapter;
use hkask_agents::communication::dispatch::MessageDispatch;
use hkask_agents::consent::ConsentManager;
use hkask_agents::curator::context::CuratorContext;
use hkask_agents::curator_agent::CuratorAgent;
use hkask_agents::escalation::EscalationQueue;
use hkask_agents::loop_system::LoopSystem;
use hkask_agents::pod::PodManager;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::{CnsRuntime, CompositeGasEstimator, CyberneticsLoop, GovernedTool};
use hkask_memory::{
    ConsolidationBridge, EpisodicLoop, EpisodicMemory, SemanticLoop, SemanticMemory,
};
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_templates::SqliteRegistry;
use hkask_types::event::NuEventSink;
use hkask_types::loops::HkaskLoop;
use hkask_types::loops::curation::CuratorHandle;
use hkask_types::ports::ToolPort;
use hkask_types::{CapabilityChecker, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod soap_config;

// Re-export route types for OpenAPI schema generation
pub use routes::{AcpRegisterRequest, AcpRegisterResponse};
pub use routes::{
    ChatRequest, ChatResponse, CnsHealthResponse, CnsVarietyResponse, CreatePodRequest,
    CreatePodResponse, EventRecord, GrantCapabilityRequest, ListPodsResponse, ModelEntry,
    ModelListResponse, ModelSearchQuery, ObjectiveData, PodStatusResponse, SeverityCounts,
    SoapInferAuthRequest, SoapInferRequest, SoapInferResponse, SpecCaptureRequest,
    SpecCaptureResponse, SpecCultivateResponse, SpecListResponse, SpecValidateRequest,
    SpecValidateResponse, TemplateResponse, ValidationErrorType, VarietyCounterResponse,
};

use openapi::ApiDoc;

/// Database configuration for persistent storage
pub struct DbConfig {
    pub path: Option<String>,
    pub passphrase: Option<String>,
}

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
    /// Ensemble inferencer (optional - for Russell SOAP inference)
    pub ensemble_inferencer: Option<Arc<hkask_ensemble::adapters::InferencePortAdapter>>,
    /// Spec store for DDMVSS specifications
    pub spec_store: Option<Arc<dyn hkask_storage::SpecStore + Send + Sync>>,
    /// Consent manager for user sovereignty
    pub consent_manager: Arc<ConsentManager>,
    /// Escalation queue for Curator escalations
    pub escalation_queue: Arc<EscalationQueue>,
    /// Git CAS adapter for template archival
    pub git_cas: Arc<dyn hkask_agents::ports::GitCASPort>,
    /// Standing ensemble sessions (keyed by session ID)
    pub standing_sessions: Arc<
        tokio::sync::RwLock<
            HashMap<String, Arc<tokio::sync::RwLock<hkask_ensemble::StandingSession>>>,
        >,
    >,
    /// Standing session storage port (persistent or in-memory)
    pub standing_session_store: Option<Arc<dyn hkask_types::ports::StandingSessionPort>>,
    /// Ensemble session manager for chat/deliberation
    pub session_manager: Arc<tokio::sync::RwLock<hkask_ensemble::SessionManager>>,
    /// Goal repository for the goal coordination substrate. Mirrors the CLI
    /// `kask goal` surface for MCP ≡ CLI ≡ API parity.
    pub goal_repo: Arc<hkask_storage::SqliteGoalRepository>,
    /// Loop system for 6-loop regulation (Cybernetics, Episodic, Semantic, Curation)
    pub loop_system: Arc<LoopSystem>,
    /// Episodic memory for first-person experience storage and recall
    pub episodic_memory: Arc<EpisodicMemory>,
    /// CNS runtime for real-time variety and health data
    pub cns_runtime: Arc<CnsRuntime>,
    /// General-purpose inference port (shared across requests)
    pub inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>>,
}

/// Build the LoopSystem with all 6 loops.
///
/// Creates CnsRuntime, MessageDispatch, LoopSystem, and registers:
/// Cybernetics, Episodic, Semantic, and Curation loops.
/// Communication Loop is managed internally by LoopSystem.
/// Inference Loop is registered only if an inference port is provided.
fn build_loop_system(
    escalation_queue: Arc<EscalationQueue>,
    dispatch: Arc<MessageDispatch>,
    inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>>,
    system_webid: WebID,
    acp: Option<Arc<dyn hkask_agents::ports::AcpPort>>,
    event_sink: Option<Arc<dyn NuEventSink>>,
) -> (
    Arc<LoopSystem>,
    Arc<EpisodicMemory>,
    Arc<tokio::sync::RwLock<CyberneticsLoop>>,
) {
    let loop_system = LoopSystem::new(Arc::clone(&dispatch));

    // Cybernetics Loop
    let cns_rwlock: Arc<tokio::sync::RwLock<CnsRuntime>> = Arc::new(tokio::sync::RwLock::new(
        CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD),
    ));
    let cybernetics_dispatch_tx = loop_system.dispatch_sender();
    let set_points = hkask_cns::load_set_points();
    let cybernetics_loop = CyberneticsLoop::with_set_points(
        Arc::clone(&cns_rwlock),
        set_points,
        cybernetics_dispatch_tx,
    );
    let cybernetics_loop = match event_sink {
        Some(sink) => cybernetics_loop.with_event_sink(sink),
        None => cybernetics_loop,
    };
    let cybernetics_loop_rwlock = Arc::new(tokio::sync::RwLock::new(cybernetics_loop));
    // Register loops (register_loop is async, use a small runtime for sync callers)
    let rt = tokio::runtime::Runtime::new().expect("loop system runtime");
    rt.block_on(async {
        loop_system
            .register_loop(Arc::new(CyberneticsLoopHandle(Arc::clone(
                &cybernetics_loop_rwlock,
            ))))
            .await;
    });

    // Inference Loop (optional)
    if let Some(port) = inference_port {
        let inference_loop = hkask_agents::InferenceLoop::new(port);
        rt.block_on(async {
            loop_system.register_loop(Arc::new(inference_loop)).await;
        });
    }

    // Episodic Loop
    let db = Database::in_memory().expect("in-memory db");
    let conn = db.conn_arc();
    let triple_store = TripleStore::new(Arc::clone(&conn));
    let episodic_memory = Arc::new(EpisodicMemory::new(triple_store));
    let storage_budget = episodic_memory.storage_budget();
    let episodic_loop =
        EpisodicLoop::new(Arc::clone(&episodic_memory), system_webid, storage_budget);
    // API-facing episodic memory backed by the same connection
    let episodic_memory_api = Arc::new(EpisodicMemory::new(TripleStore::new(conn)));
    rt.block_on(async {
        loop_system.register_loop(Arc::new(episodic_loop)).await;
    });

    // Semantic Loop
    let db2 = Database::in_memory().expect("in-memory db");
    let conn2 = db2.conn_arc();
    let triple_store2 = TripleStore::new(Arc::clone(&conn2));
    let embedding_store = EmbeddingStore::new(conn2);
    let semantic_memory = Arc::new(SemanticMemory::new(triple_store2, embedding_store));
    let semantic_loop = SemanticLoop::new(Arc::clone(&semantic_memory));
    rt.block_on(async {
        loop_system.register_loop(Arc::new(semantic_loop)).await;
    });

    // Curation Loop (via CuratorAgent)
    let curator_handle = CuratorHandle::system();
    let mut curator_context = CuratorContext::new(
        curator_handle.clone(),
        Arc::new(CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD)),
        dispatch,
        escalation_queue,
    );
    if let Some(acp_port) = acp {
        curator_context = curator_context.with_acp(acp_port);
    }
    let curator_context = Arc::new(curator_context);
    let consolidation_bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let curator_agent =
        CuratorAgent::with_consolidation(curator_context, Default::default(), consolidation_bridge);
    let curation_loop: Arc<dyn HkaskLoop> = curator_agent.curation_loop().clone();
    rt.block_on(async {
        loop_system.register_loop(curation_loop).await;
    });

    drop(rt);
    (
        Arc::new(loop_system),
        episodic_memory_api,
        cybernetics_loop_rwlock,
    )
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
    ) -> Self {
        let consent_conn = match db_config
            .and_then(|c| c.path.as_deref().zip(c.passphrase.as_deref()))
        {
            Some((path, passphrase)) => hkask_storage::Database::open(path, passphrase)
                .expect("Failed to open consent database")
                .conn_arc(),
            _ => {
                tracing::warn!(
                    target: "hkask.api",
                    "No persistent database configured — consent records are in-memory and will be lost on restart. \
                     Set HKASK_API_DB and HKASK_DB_PASSPHRASE for sovereign persistence."
                );
                hkask_storage::Database::in_memory()
                    .expect("in-memory db")
                    .conn_arc()
            }
        };
        let consent_store = hkask_storage::ConsentStore::new(consent_conn);
        consent_store
            .initialize_schema()
            .expect("consent store schema init");
        let consent_manager = Arc::new(ConsentManager::new(consent_store));

        let escalation_conn = match db_config
            .and_then(|c| c.path.as_deref().zip(c.passphrase.as_deref()))
        {
            Some((path, passphrase)) => hkask_storage::Database::open(path, passphrase)
                .expect("Failed to open escalation database")
                .conn_arc(),
            _ => {
                tracing::warn!(
                    target: "hkask.api",
                    "No persistent database configured — escalation queue is in-memory and will be lost on restart. \
                     Set HKASK_API_DB and HKASK_DB_PASSPHRASE for sovereign persistence."
                );
                hkask_storage::Database::in_memory()
                    .expect("in-memory db")
                    .conn_arc()
            }
        };
        let escalation_queue =
            Arc::new(EscalationQueue::new(escalation_conn).expect("escalation queue init"));
        let git_cas: Arc<dyn hkask_agents::ports::GitCASPort> = Arc::new(GitCasAdapter::from_path(
            PathBuf::from("/tmp/hkask-templates"),
        ));
        let dispatcher_runtime = hkask_mcp::runtime::McpRuntime::new();

        // Build the LoopSystem with shared dispatch and escalation queue
        let dispatch = Arc::new(MessageDispatch::new());
        let inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>> =
            ensemble_inferencer.as_ref().map(|ei| Arc::clone(ei.port()));

        // Create CNS event sink for governance observability (shared by
        // CyberneticsLoop and GovernedTool)
        let cns_event_conn = hkask_storage::Database::in_memory()
            .expect("cns event db")
            .conn_arc();
        let cns_event_sink: Arc<dyn NuEventSink> =
            Arc::new(hkask_storage::NuEventStore::new(cns_event_conn));

        let (loop_system, episodic_memory, cybernetics_loop_rwlock) = build_loop_system(
            Arc::clone(&escalation_queue),
            dispatch,
            inference_port,
            system_webid,
            acp,
            Some(Arc::clone(&cns_event_sink)),
        );

        // Create raw tool port (ungoverned executor)
        let raw_tool_port: Arc<dyn ToolPort> = Arc::new(
            hkask_mcp::raw_tool_port::RawMcpToolPort::new(dispatcher_runtime.clone()),
        );

        // Create GovernedTool membrane with CompositeGasEstimator
        let estimator: Arc<dyn hkask_cns::GasEstimator> = Arc::new(CompositeGasEstimator::new());
        let governed_tool: Arc<dyn ToolPort> = Arc::new(GovernedTool::new(
            raw_tool_port,
            cybernetics_loop_rwlock,
            cns_event_sink,
            estimator,
            system_webid,
            loop_system.dispatch_sender(),
        ));

        // Wire GovernedTool into McpDispatcher
        let mcp_dispatcher = Arc::new(hkask_mcp::dispatch::McpDispatcher::with_governed_tool(
            dispatcher_runtime,
            capability_secret,
            governed_tool,
        ));
        // Goal repository wired with a CNS denial sink over a shared connection,
        // mirroring the CLI integration (ADR-029). Capability denials persist
        // as `cns.tool.goal.capability.denied` ν-events.
        let goal_conn = match db_config.and_then(|c| c.path.as_deref().zip(c.passphrase.as_deref()))
        {
            Some((path, passphrase)) => hkask_storage::Database::open(path, passphrase)
                .expect("Failed to open goal database")
                .conn_arc(),
            _ => hkask_storage::Database::in_memory()
                .expect("in-memory db")
                .conn_arc(),
        };
        let goal_sink: Arc<dyn NuEventSink> =
            Arc::new(hkask_storage::NuEventStore::new(Arc::clone(&goal_conn)));
        let goal_repo =
            Arc::new(hkask_storage::SqliteGoalRepository::new(goal_conn).with_telemetry(goal_sink));

        // Standing session store (persistent or in-memory)
        let standing_conn =
            match db_config.and_then(|c| c.path.as_deref().zip(c.passphrase.as_deref())) {
                Some((path, passphrase)) => hkask_storage::Database::open(path, passphrase)
                    .expect("Failed to open standing session database")
                    .conn_arc(),
                None => hkask_storage::Database::in_memory()
                    .expect("in-memory standing session db")
                    .conn_arc(),
            };
        let standing_session_store = hkask_storage::StandingSessionStore::new(standing_conn);
        standing_session_store
            .initialize_schema()
            .expect("standing session schema init");
        let standing_session_store: Option<Arc<dyn hkask_types::ports::StandingSessionPort>> =
            Some(Arc::new(standing_session_store));

        // Ensemble session manager
        let session_manager = Arc::new(tokio::sync::RwLock::new(
            hkask_ensemble::SessionManager::new(system_webid),
        ));

        // Extract inference port before moving ensemble_inferencer into struct
        let inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>> =
            ensemble_inferencer.as_ref().map(|ei| Arc::clone(ei.port()));

        Self {
            registry: Arc::new(tokio::sync::Mutex::new(registry)),
            mcp_runtime: Arc::new(mcp_runtime),
            mcp_dispatcher,
            pod_manager: Arc::new(pod_manager),
            capability_checker: Arc::new(CapabilityChecker::new(capability_secret)),
            system_webid,
            ensemble_inferencer,
            spec_store: None,
            consent_manager,
            escalation_queue,
            git_cas,
            standing_sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            standing_session_store,
            session_manager,
            goal_repo,
            loop_system,
            episodic_memory,
            cns_runtime: Arc::new(CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD)),
            inference_port,
        }
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
    ) -> Self {
        let git_cas = GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates"));
        let acp_runtime = Arc::new(AcpRuntime::new(acp_secret));
        let acp_port: Arc<dyn hkask_agents::ports::AcpPort> = acp_runtime.clone();
        let mcp_runtime_adapter = McpRuntimeAdapter::new().with_runtime(
            Arc::new(mcp_runtime.clone()),
            tokio::runtime::Handle::current(),
        );

        // Use MemoryLoopAdapter (routes through hkask-memory domain logic)
        let db = Database::in_memory().expect("in-memory db");
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
    ) -> Self {
        let base_url = std::env::var("OKAPI_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());
        let config = hkask_templates::OkapiConfig {
            base_url,
            ..hkask_templates::OkapiConfig::default()
        };
        let inference = hkask_templates::OkapiInference::new(model, config)
            .expect("Failed to create Okapi inference");
        let port: Arc<dyn hkask_types::ports::InferencePort> = Arc::new(inference);
        let adapter = Arc::new(hkask_ensemble::adapters::InferencePortAdapter::new(port));
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
    pub async fn start_loops(&self) {
        tracing::info!(
            target: "hkask.api",
            loops = ?self.loop_system.registered_loop_ids().await,
            "Starting loop system"
        );
        self.loop_system.start().await;
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

/// Resolve the SOAP capability secret through the keystore's domain-specific
/// resolution chain.
pub fn resolve_soap_capability_secret() -> Result<[u8; 32], String> {
    hkask_keystore::resolve_capability_key()
        .map(|s| {
            let mut arr = [0u8; 32];
            let bytes: &[u8] = &s;
            let len = bytes.len().min(32);
            arr[..len].copy_from_slice(&bytes[..len]);
            arr
        })
        .map_err(|e| {
            format!(
                "Capability key not available: {}. Run `kask chat` to complete onboarding, \
                 or set HKASK_MASTER_KEY or HKASK_CAPABILITY_KEY.",
                e
            )
        })
}

/// Error response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub details: Option<serde_json::Value>,
}

/// Create API router with OpenAPI documentation and authentication
pub fn create_router(state: ApiState) -> Result<OpenApiRouter, String> {
    let auth_service = std::sync::Arc::new(
        middleware::AuthService::new()
            .map_err(|e| format!("Failed to initialize auth service: {}", e))?,
    );

    Ok(OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::templates_router().into())
        .merge(routes::bots_router().into())
        .merge(routes::pods_router().into())
        .merge(routes::mcp_router().into())
        .merge(routes::cns_router().into())
        .merge(routes::sovereignty_router().into())
        .merge(routes::chat_router().into())
        .merge(routes::models_router().into())
        .merge(routes::ensemble_router().into())
        .merge(routes::soap_infer_router().into())
        .merge(routes::acp_router().into())
        .merge(routes::bundles_router().into())
        .merge(routes::spec_router().into())
        .merge(routes::curator_router().into())
        .merge(routes::episodic_router().into())
        .merge(routes::git_router().into())
        .merge(routes::goal_router().into())
        .layer(axum::middleware::from_fn_with_state(
            auth_service,
            middleware::auth_middleware,
        ))
        .with_state(state))
}

/// Build OpenAPI spec
pub fn create_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
