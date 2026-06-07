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
//! - `POST /api/llm/infer` — SOAP inference endpoint for Russell
//! - `POST /api/episodic/store` — Store episodic triple
//! - `GET /api/episodic/query` — Query episodic memories
//! - `GET /api/episodic/usage` — Episodic storage usage

use hkask_agents::CyberneticsLoopHandle;
use hkask_agents::acp::AcpRuntime;
use hkask_agents::adapters::mcp_runtime::FullMcpAdapter;
use hkask_agents::adapters::memory_loop_adapter::MemoryLoopAdapter;
use hkask_agents::communication::dispatch::MessageDispatch;
use hkask_agents::consent::ConsentManager;
use hkask_agents::curator::context::CuratorContext;
use hkask_agents::curator_agent::CuratorAgent;
use hkask_agents::escalation::EscalationQueue;
use hkask_agents::loop_system::LoopSystem;
use hkask_agents::pod::PodManager;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::{
    CnsRuntime, CompositeGasEstimator, CyberneticsLoop, GasCost, GovernedTool, SnapshotLoop,
};
use hkask_memory::{
    ConsolidationBridge, EpisodicLoop, EpisodicMemory, SemanticLoop, SemanticMemory,
};
use hkask_storage::{EmbeddingStore, TripleStore};
use hkask_templates::SqliteRegistry;
use hkask_types::event::NuEventSink;
use hkask_types::loops::HkaskLoop;
use hkask_types::loops::curation::CuratorHandle;
use hkask_types::ports::git_cas::GitCASPort;
use hkask_types::{CapabilityChecker, WebID};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use utoipa::OpenApi;

/// Default gas cap for API ensemble sessions (150k = same as CLI default).
const API_ENSEMBLE_GAS_CAP: u64 = 150_000;

/// Adapter bridging `CyberneticsLoop` to the ensemble's `GasGovernancePort`.
///
/// Provides synchronous access to the CyberneticsLoop's gas governance by
/// using an atomic counter for `can_proceed` (approximate) and a fire-and-forget
/// task spawn for `acquire` (actual budget consumption via async call).
///
/// This is the API-mode equivalent of the CLI's `CyberneticsLoopGasAdapter`.
struct ApiGasGovernanceAdapter {
    loop_ref: Arc<tokio::sync::RwLock<CyberneticsLoop>>,
    agent: WebID,
    gas_used: AtomicU64,
    gas_cap: AtomicU64,
}

impl ApiGasGovernanceAdapter {
    fn new(loop_ref: Arc<tokio::sync::RwLock<CyberneticsLoop>>, agent: WebID, cap: u64) -> Self {
        Self {
            loop_ref,
            agent,
            gas_used: AtomicU64::new(0),
            gas_cap: AtomicU64::new(cap),
        }
    }
}

impl hkask_ensemble::GasGovernancePort for ApiGasGovernanceAdapter {
    fn can_proceed(&self, gas: u64) -> bool {
        let used = self.gas_used.load(Ordering::Relaxed);
        let cap = self.gas_cap.load(Ordering::Relaxed);
        used.saturating_add(gas) <= cap
    }

    fn acquire(&self, gas: u64) {
        self.gas_used.fetch_add(gas, Ordering::Relaxed);
        // Fire-and-forget: report to CyberneticsLoop asynchronously
        let loop_ref = self.loop_ref.clone();
        let agent = self.agent;
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let loop_read = loop_ref.read().await;
                let _ = loop_read.acquire_budget(&agent, GasCost(gas)).await;
            });
        }
    }
}

use utoipa_axum::router::OpenApiRouter;

pub mod error;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod soap_config;

pub use error::ApiError;

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

/// Database-backed stores initialized from a `DbConfig`.
///
/// Extracted from `ApiState::new()` so that store creation is composable
/// and independently testable.
struct Stores {
    consent_manager: Arc<ConsentManager>,
    escalation_queue: Arc<EscalationQueue>,
    goal_repo: Arc<hkask_storage::SqliteGoalRepository>,
    standing_session_store: Arc<hkask_storage::StandingSessionStore>,
}

impl Stores {
    /// Open and initialise all persistent stores.
    ///
    /// Each store gets its own database connection (and therefore its own
    /// connection pool) so a slow store cannot starve another.
    ///
    /// The `git_cas_port` is injected into stores that support CAS write-through
    /// via their `.with_cas()` builder methods, enabling per-mutation audit
    /// trails alongside batch snapshots from the SnapshotLoop.
    fn init(
        db_config: Option<&DbConfig>,
        git_cas_port: Arc<dyn GitCASPort>,
    ) -> Result<Stores, ApiError> {
        let consent_conn = open_db(db_config, "consent")?.conn_arc();
        let consent_store =
            hkask_storage::ConsentStore::new(consent_conn).with_cas(Arc::clone(&git_cas_port));
        consent_store
            .initialize_schema()
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to initialize consent store schema: {e}"),
            })?;
        let consent_manager = Arc::new(ConsentManager::new(consent_store));

        let escalation_conn = open_db(db_config, "escalation")?.conn_arc();
        let escalation_queue =
            Arc::new(
                EscalationQueue::new(escalation_conn).map_err(|e| ApiError::Internal {
                    message: format!("Failed to initialize escalation queue: {e}"),
                })?,
            );

        let goal_conn = open_db(db_config, "goal")?.conn_arc();
        let goal_sink: Arc<dyn NuEventSink> =
            Arc::new(hkask_storage::NuEventStore::new(Arc::clone(&goal_conn)));
        let goal_repo = Arc::new(
            hkask_storage::SqliteGoalRepository::new(goal_conn)
                .with_telemetry(goal_sink)
                .with_cas(Arc::clone(&git_cas_port)),
        );

        let standing_conn = open_db(db_config, "standing session")?.conn_arc();
        let standing_session_store = hkask_storage::StandingSessionStore::new(standing_conn)
            .with_cas(Arc::clone(&git_cas_port));
        standing_session_store
            .initialize_schema()
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to initialize standing session store schema: {e}"),
            })?;
        let standing_session_store = Arc::new(standing_session_store);

        Ok(Stores {
            consent_manager,
            escalation_queue,
            goal_repo,
            standing_session_store,
        })
    }
}

/// Open a persistent database, or fall back to in-memory with a warning.
///
/// Extracts the repeated pattern of `db_config.and_then(...)` → `Database::open`
/// that appeared 4 times in `ApiState::new()`. Returns the `Database`,
/// so callers can extract `.conn_arc()` or use it directly.
fn open_db(
    db_config: Option<&DbConfig>,
    purpose: &str,
) -> Result<hkask_storage::Database, ApiError> {
    match db_config.and_then(|c| c.path.as_deref().zip(c.passphrase.as_deref())) {
        Some((path, passphrase)) => {
            hkask_storage::Database::open(path, passphrase).map_err(|e| ApiError::Internal {
                message: format!("Failed to open {purpose} database: {e}"),
            })
        }
        None => {
            tracing::warn!(
                target: "hkask.api",
                "No persistent database configured — {purpose} store is in-memory and will be lost on restart. \
                 Set HKASK_DB_PATH and HKASK_DB_PASSPHRASE for sovereign persistence."
            );
            Ok(hkask_storage::in_memory_db())
        }
    }
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
    pub inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>>,
    /// CNS gas governance port for ensemble sessions.
    /// Wired through the CyberneticsLoop so CNS can sense ensemble gas usage.
    pub gas_governance: Arc<dyn hkask_ensemble::GasGovernancePort>,
}

/// Git CAS adapter bundle (P2.2).
///
/// Extracted from `ApiState::new()` to keep CAS initialization self-contained
/// and to surface the `expect("Failed to create GixCasAdapter")` failure as a
/// typed `ApiError::Internal` rather than a panic at startup (P4.1).
struct GitCasBundle {
    /// Concrete `GitCasAdapter` (legacy — template loading only).
    git_cas: Arc<hkask_mcp::GitCasAdapter>,
    /// Trait-object `GitCASPort` (hexagonal boundary) used by stores.
    git_cas_port: Arc<dyn hkask_types::ports::git_cas::GitCASPort>,
}

/// Initialize the Git CAS adapter and the trait-object port.
///
/// `git_cas` writes to a fixed on-disk directory; `git_cas_port` resolves
/// from `GIX_*` env vars when present and falls back to the same directory.
///
/// P4.1: Returns `Result<GitCasBundle, ApiError>` so CAS initialization
/// failures surface as typed errors instead of panics. The hard-coded
/// `/tmp/hkask-templates` fallback directory is the documented invariant
/// of this function — if even that cannot be created, returning
/// `ApiError::Internal` is the correct (non-panicking) failure mode.
fn init_git_cas() -> Result<GitCasBundle, ApiError> {
    let git_cas = Arc::new(hkask_mcp::GitCasAdapter::from_path(PathBuf::from(
        "/tmp/hkask-templates",
    )));
    let fallback_path = PathBuf::from("/tmp/hkask-templates");
    let git_cas_port: Arc<dyn hkask_types::ports::git_cas::GitCASPort> = Arc::new(
        hkask_mcp::GixCasAdapter::from_env()
            .or_else(|_| hkask_mcp::GixCasAdapter::new(fallback_path))
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to create GixCasAdapter: {e}"),
            })?,
    );
    Ok(GitCasBundle {
        git_cas,
        git_cas_port,
    })
}

/// Governed MCP tool + dispatcher bundle (P2.2).
///
/// Extracted from `ApiState::new()`. Wraps the gas estimator, raw tool port,
/// and `GovernedTool` membrane into the `McpDispatcher` that all tool
/// invocations route through. Returns the dispatcher plus a cloned
/// `CyberneticsLoop` handle for downstream gas-governance adapters.
struct GovernedMcpTool {
    mcp_dispatcher: Arc<hkask_mcp::dispatch::McpDispatcher>,
    /// Cloned before being moved into `GovernedTool`; needed for the
    /// `ApiGasGovernanceAdapter` that the ensemble session manager consumes.
    cybernetics_loop_for_gas: Arc<tokio::sync::RwLock<CyberneticsLoop>>,
}

/// Build the `GovernedTool` membrane and `McpDispatcher` that route every tool
/// invocation through CNS gas governance.
///
/// P2.2 extraction: this block is the largest single section of
/// `ApiState::new()` (after `Stores::init` and `build_loop_system` were
/// already extracted). Isolating it makes the wiring self-documenting and
/// the failure mode (e.g. tokio handle missing) testable in isolation.
fn build_governed_mcp_tool(
    dispatcher_runtime: hkask_mcp::runtime::McpRuntime,
    cybernetics_loop_rwlock: Arc<tokio::sync::RwLock<CyberneticsLoop>>,
    cns_event_sink: Arc<dyn NuEventSink>,
    loop_system: &LoopSystem,
    system_webid: WebID,
    capability_secret: &[u8],
) -> GovernedMcpTool {
    let raw_tool_port = Arc::new(hkask_mcp::raw_tool_port::RawMcpToolPort::new(
        dispatcher_runtime.clone(),
    ));
    let estimator: Arc<dyn hkask_cns::GasEstimator> = Arc::new(CompositeGasEstimator::new());
    let governed_tool = Arc::new(GovernedTool::new(
        raw_tool_port,
        Arc::clone(&cybernetics_loop_rwlock),
        cns_event_sink,
        estimator,
        system_webid,
        loop_system.dispatch_sender(),
    ));
    let mcp_dispatcher = Arc::new(hkask_mcp::dispatch::McpDispatcher::with_governed_tool(
        dispatcher_runtime,
        capability_secret,
        governed_tool,
    ));
    GovernedMcpTool {
        mcp_dispatcher,
        cybernetics_loop_for_gas: cybernetics_loop_rwlock,
    }
}

/// Ensemble session bundle (P2.2).
///
/// Extracted from `ApiState::new()`. Composes the gas governance adapter
/// (which lets CNS sense ensemble gas usage) with the `SessionManager`
/// that the `/api/chat` route consumes. Also returns the inference port
/// extracted from the optional `ensemble_inferencer`, and the
/// `ensemble_inferencer` itself (for `ensemble_inferencer_with_breaker`).
struct EnsembleSession {
    session_manager: Arc<tokio::sync::RwLock<hkask_ensemble::SessionManager>>,
    gas_governance: Arc<dyn hkask_ensemble::GasGovernancePort>,
    inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>>,
    /// Returned to the caller to be stored on `ApiState`; consumed by
    /// `ensemble_inferencer_with_breaker` for SOAP and ensemble routes.
    ensemble_inferencer: Option<Arc<hkask_ensemble::adapters::InferencePortAdapter>>,
}

/// Wire the ensemble session manager with CNS gas governance so ensemble
/// sessions in API mode respect the L6 budget.
///
/// P2.2 extraction: the inference port is also extracted here because
/// the caller needs it both on the returned bundle and on the final
/// `ApiState` literal — extracting once avoids a second clone.
fn build_ensemble_session(
    ensemble_inferencer: Option<Arc<hkask_ensemble::adapters::InferencePortAdapter>>,
    cybernetics_loop: Arc<tokio::sync::RwLock<CyberneticsLoop>>,
    system_webid: WebID,
) -> EnsembleSession {
    let inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>> =
        ensemble_inferencer.as_ref().map(|ei| Arc::clone(ei.port()));
    let gas_governance: Arc<dyn hkask_ensemble::GasGovernancePort> = Arc::new(
        ApiGasGovernanceAdapter::new(cybernetics_loop, system_webid, API_ENSEMBLE_GAS_CAP),
    );
    let session_manager = Arc::new(tokio::sync::RwLock::new(
        hkask_ensemble::SessionManager::new(system_webid)
            .with_gas_governance(Arc::clone(&gas_governance)),
    ));
    EnsembleSession {
        session_manager,
        gas_governance,
        inference_port,
        ensemble_inferencer,
    }
}

/// Build the LoopSystem with all loops.
///
/// Creates CnsRuntime, MessageDispatch, LoopSystem, and registers:
/// Cybernetics, Episodic, Semantic, Curation, and Snapshot loops.
/// Communication Loop is managed internally by LoopSystem.
/// Inference Loop is registered only if an inference port is provided.
#[allow(clippy::type_complexity)]
fn build_loop_system(
    escalation_queue: Arc<EscalationQueue>,
    dispatch: Arc<MessageDispatch>,
    inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>>,
    system_webid: WebID,
    acp: Option<Arc<dyn hkask_agents::ports::AcpPort>>,
    event_sink: Option<Arc<dyn NuEventSink>>,
    git_cas_port: Arc<dyn GitCASPort>,
) -> Result<
    (
        Arc<LoopSystem>,
        Arc<dyn EpisodicStoragePort>,
        Arc<tokio::sync::RwLock<CyberneticsLoop>>,
    ),
    ApiError,
> {
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
    // Wire CommunicationLoop↔CyberneticsLoop queue depth counter.
    // CommunicationLoop writes, CyberneticsLoop reads — lock-free, Relaxed ordering.
    let cybernetics_loop = cybernetics_loop
        .with_communication_queue_depth(loop_system.communication_queue_depth_counter());
    let cybernetics_loop_rwlock = Arc::new(tokio::sync::RwLock::new(cybernetics_loop));
    // Register loops (register_loop is async, use a small runtime for sync callers)
    let rt = tokio::runtime::Runtime::new().map_err(|e| ApiError::Internal {
        message: format!("Failed to create tokio runtime for loop system: {e}"),
    })?;
    rt.block_on(async {
        loop_system
            .register_loop(Arc::new(CyberneticsLoopHandle(Arc::clone(
                &cybernetics_loop_rwlock,
            ))))
            .await;
    });

    // Inference Loop (optional)
    if inference_port.is_some() {
        let inference_loop =
            hkask_agents::InferenceLoop::new().with_dispatch(loop_system.dispatch_sender());
        rt.block_on(async {
            loop_system.register_loop(Arc::new(inference_loop)).await;
        });
    }

    // Episodic Loop
    let db = hkask_storage::in_memory_db();
    let conn = db.conn_arc();
    let triple_store = TripleStore::new(Arc::clone(&conn));
    let episodic_memory = Arc::new(EpisodicMemory::new(triple_store));
    let storage_budget = episodic_memory.storage_budget();
    let episodic_loop =
        EpisodicLoop::new(Arc::clone(&episodic_memory), system_webid, storage_budget);
    rt.block_on(async {
        loop_system.register_loop(Arc::new(episodic_loop)).await;
    });

    // Semantic Loop
    let db2 = hkask_storage::in_memory_db();
    let conn2 = db2.conn_arc();
    let triple_store2 = TripleStore::new(Arc::clone(&conn2));
    let embedding_store = EmbeddingStore::new(Arc::clone(&conn2));
    let semantic_memory = Arc::new(SemanticMemory::new(triple_store2, embedding_store));
    let semantic_loop = SemanticLoop::new(Arc::clone(&semantic_memory));
    rt.block_on(async {
        loop_system.register_loop(Arc::new(semantic_loop)).await;
    });

    // API-facing memory adapter — shares the same DB connections as the loops
    // so budget reads see API writes immediately.
    let memory_adapter = Arc::new(MemoryLoopAdapter::new(
        EpisodicMemory::new(TripleStore::new(conn)),
        SemanticMemory::new(
            TripleStore::new(Arc::clone(&conn2)),
            EmbeddingStore::new(conn2),
        ),
    ));
    let episodic_storage: Arc<dyn EpisodicStoragePort> = memory_adapter.clone();

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
    curator_context = curator_context.with_loop_dispatch_tx(loop_system.dispatch_sender());
    let curator_context = Arc::new(curator_context);
    let consolidation_bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let curator_agent = CuratorAgent::with_consolidation(
        curator_context,
        Default::default(),
        Arc::clone(&consolidation_bridge),
    );
    let curation_loop: Arc<dyn HkaskLoop> = curator_agent.curation_loop().clone();
    rt.block_on(async {
        loop_system.register_loop(curation_loop).await;
    });

    // Snapshot Loop (CAS — scheduled snapshots based on RetentionPolicy)
    let snapshot_loop = SnapshotLoop::new(Arc::clone(&git_cas_port));
    rt.block_on(async {
        loop_system.register_loop(Arc::new(snapshot_loop)).await;
    });

    drop(rt);
    Ok((
        Arc::new(loop_system),
        episodic_storage,
        cybernetics_loop_rwlock,
    ))
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
        let dispatch = Arc::new(MessageDispatch::new());
        let inference_port_for_loops: Option<Arc<dyn hkask_types::ports::InferencePort>> =
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
        let git_cas = hkask_mcp::GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates"));
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
        let base_url = std::env::var("OKAPI_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());
        let config = hkask_templates::OkapiConfig {
            base_url,
            ..hkask_templates::OkapiConfig::default()
        };
        let inference = hkask_templates::OkapiInference::new(model, config).map_err(|e| {
            ApiError::Internal {
                message: format!("Failed to create Okapi inference: {e}"),
            }
        })?;
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
        .merge(routes::consolidation_router().into())
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
