//! Shared dependency graph assembled once at startup.
//!
//! `ServiceContext` owns the canonical instances of all shared infrastructure:
//! registry, MCP runtime, CNS, loop system, escalation queue, memory adapters,
//! etc. Both `ReplState` and `ApiState` compose a `ServiceContext` and add
//! only their surface-specific presentation fields.
//!
//! Construction happens via `ServiceContext::build(config)`, which replaces
//! the four independent assembly paths currently in the codebase:
//! - `ReplState` init in `cli/repl/init.rs` (~325 lines)
//! - `ApiState::new()` in `api/lib.rs` (~400 lines)
//! - `build_loop_system()` in `api/loop_system.rs` (~130 lines)
//! - `commands/loops.rs` (~113 lines)

use std::sync::Arc;
use tokio::sync::RwLock;

use hkask_agents::CuratorContext;
use hkask_agents::EscalationQueue;
use hkask_agents::LoopSystem;
use hkask_agents::communication::MessageDispatch;
use hkask_agents::consent::ConsentManager;
use hkask_agents::curator_agent::CuratorAgent;
use hkask_agents::ensemble::session::SessionManager;
use hkask_agents::loop_system::CyberneticsLoopHandle;
use hkask_agents::pod::PodManager;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::{
    CnsRuntime, CompositeGasEstimator, CyberneticsLoop, GasEstimator, GovernedTool, load_set_points,
};
use hkask_mcp::McpDispatcher;
use hkask_mcp::raw_tool_port::RawMcpToolPort;
use hkask_mcp::runtime::McpRuntime;
use hkask_memory::{
    ConsolidationBridge, EpisodicLoop, EpisodicMemory, SemanticLoop, SemanticMemory,
};
use hkask_storage::goals::SqliteGoalRepository;
use hkask_storage::nu_event_store::NuEventStore;
use hkask_storage::user_store::UserStore;
use hkask_storage::{
    ConsentStore, Database, DatabaseError, EmbeddingStore, SovereigntyBoundaryStore,
    SqliteSpecStore, StandingSessionStore, TripleStore, in_memory_db,
};
use hkask_templates::OkapiConfig;
use hkask_templates::SqliteRegistry;
use hkask_types::CuratorHandle;
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::loops::HkaskLoop;
use hkask_types::ports::InferencePort;

use crate::ServiceConfig;
use crate::ServiceError;

/// Shared dependency graph assembled once at startup.
///
/// `ServiceContext` replaces the independent assembly in `ReplState`,
/// `ApiState`, `build_loop_system()`, and `commands/loops.rs`. Surfaces
/// compose this struct and add only presentation-specific fields.
///
/// Construct via `ServiceContext::build(config)`. The config provides all
/// deployment-varying parameters (DB paths, secrets, thresholds, model names).
/// The builder resolves the dependency graph canonically: stores → CNS →
/// loop system → governed tool → session manager.
///
/// `#[non_exhaustive]` prevents external crates from constructing this struct
/// with struct literal syntax — use `ServiceContext::build()` instead.
#[non_exhaustive]
pub struct ServiceContext {
    /// Template registry.
    pub registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,

    /// MCP runtime for tool discovery and invocation.
    pub mcp_runtime: Arc<McpRuntime>,

    /// MCP dispatcher for OCAP-protected tool invocation.
    pub mcp_dispatcher: Arc<McpDispatcher>,

    /// CNS runtime for variety sensing and algedonic alerts.
    pub cns_runtime: Arc<RwLock<CnsRuntime>>,

    /// Cybernetics loop for gas budget regulation.
    pub cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,

    /// Loop system for 6-loop regulation.
    pub loop_system: Arc<LoopSystem>,

    /// Message dispatch for inter-loop communication.
    pub dispatch: Arc<MessageDispatch>,

    /// Inference port for model invocation.
    pub inference_port: Option<Arc<dyn InferencePort>>,

    /// Episodic memory storage (private, agent-scoped).
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,

    /// Semantic memory storage (public, shared).
    pub semantic_storage: Arc<dyn SemanticStoragePort>,

    /// Escalation queue for Curator escalations.
    pub escalation_queue: Arc<EscalationQueue>,

    /// Consent manager for user sovereignty.
    pub consent_manager: Arc<ConsentManager>,

    /// Goal repository for the goal coordination substrate.
    pub goal_repo: Arc<SqliteGoalRepository>,

    /// Pod manager for agent lifecycle.
    pub pod_manager: Arc<PodManager>,

    /// Capability checker for OCAP verification.
    ///
    /// Backed by `config.mcp_secret` — the inter-process HMAC key. Use this
    /// checker to derive tokens for any service operation that needs a verifiable
    /// capability token (e.g., `ChatService::chat()` memory access tokens).
    pub capability_checker: Arc<hkask_types::CapabilityChecker>,

    /// System WebID for signing capabilities.
    pub system_webid: WebID,

    /// Event sink for CNS audit trail.
    pub event_sink: Arc<dyn NuEventSink>,

    /// Standing session store for ensemble persistence.
    pub standing_session_store: Arc<StandingSessionStore>,

    /// Sovereignty boundary store for Magna Carta compliance queries.
    pub sovereignty_boundary_store: SovereigntyBoundaryStore,

    /// Spec store for specification capture, validation, and cultivation.
    pub spec_store: SqliteSpecStore,

    /// Ensemble session manager for chat and deliberation coordination.
    pub session_manager: Arc<RwLock<hkask_agents::ensemble::session::SessionManager>>,

    /// ACP runtime for capability token management and agent registration.
    pub acp_runtime: Arc<hkask_agents::AcpRuntime>,

    /// Agent registry store for persistent agent records.
    pub agent_registry_store: hkask_storage::AgentRegistryStore,

    /// User store for replicant identity and authentication.
    pub user_store: Arc<std::sync::Mutex<UserStore>>,

    /// Configuration used to build this context.
    pub config: ServiceConfig,
}

/// Open an escalation queue from config.
pub fn open_escalation_queue(config: &ServiceConfig) -> Result<Arc<EscalationQueue>, ServiceError> {
    let db = Database::open(&config.db_path, &config.db_passphrase)?;
    Ok(Arc::new(EscalationQueue::new(db.conn_arc())?))
}

/// Open a spec store from config.
pub fn open_spec_store(config: &ServiceConfig) -> Result<SqliteSpecStore, ServiceError> {
    let db = Database::open(&config.db_path, &config.db_passphrase)?;
    let store = SqliteSpecStore::new(db.conn_arc());
    store.init_schema().map_err(ServiceError::Spec)?;
    Ok(store)
}

/// Open a consent manager from config.
pub fn open_consent_manager(
    config: &ServiceConfig,
) -> Result<(Arc<ConsentManager>, SovereigntyBoundaryStore), ServiceError> {
    let db = Database::open(&config.db_path, &config.db_passphrase)?;
    let conn = db.conn_arc();
    let consent_store = ConsentStore::new(Arc::clone(&conn));
    consent_store
        .initialize_schema()
        .map_err(ServiceError::ConsentStore)?;
    let cm = Arc::new(ConsentManager::new(consent_store));
    let sovereignty_boundary_store = SovereigntyBoundaryStore::new(conn);
    sovereignty_boundary_store
        .initialize_schema()
        .map_err(ServiceError::SovereigntyStore)?;
    Ok((cm, sovereignty_boundary_store))
}

/// Build an ACP runtime + agent registry store from config.
pub fn open_agent_registry(
    config: &ServiceConfig,
) -> Result<
    (
        Arc<hkask_agents::AcpRuntime>,
        hkask_storage::AgentRegistryStore,
    ),
    ServiceError,
> {
    let db = Database::open(&config.db_path, &config.db_passphrase)?;
    let conn = db.conn_arc();
    let acp = Arc::new(hkask_agents::AcpRuntime::new(&config.acp_secret));
    let store = hkask_storage::AgentRegistryStore::new(conn);
    store
        .initialize_schema()
        .map_err(ServiceError::AgentRegistryStore)?;
    Ok((acp, store))
}

impl ServiceContext {
    /// Assemble all shared infrastructure from a `ServiceConfig`.
    ///
    /// This is the canonical construction path that replaces the four
    /// independent assemblies currently in the codebase. It resolves
    /// secrets, opens databases, constructs CNS/loop system, governed
    /// tool membrane, and session manager in the correct dependency order.
    ///
    /// # Dependency order
    ///
    /// 1. Database connections (primary + per-purpose)
    /// 2. Stores (consent, escalation, goals, standing sessions)
    /// 3. CNS runtime + event sink
    /// 4. Loop system + cybernetics loop
    /// 5. GovernedTool membrane + MCP dispatcher
    /// 6. ACP runtime + pod manager
    /// 7. Inference port (optional, based on config)
    /// 8. Memory adapters (episodic + semantic)
    pub async fn build(config: ServiceConfig) -> Result<Self, ServiceError> {
        // ── 1. System identity ──────────────────────────────────────────────
        let system_webid = WebID::from_persona(config.agent_name.as_bytes());

        // ── 2. Database connections ──────────────────────────────────────────
        let open_db = || -> Result<Database, DatabaseError> {
            if config.in_memory {
                Ok(in_memory_db())
            } else {
                Database::open(&config.db_path, &config.db_passphrase)
            }
        };

        let primary_conn = open_db()?.conn_arc();
        let consent_conn = open_db()?.conn_arc();
        let escalation_conn = open_db()?.conn_arc();
        let goal_conn = open_db()?.conn_arc();

        // ── 3. Stores ───────────────────────────────────────────────────────
        let consent_store = ConsentStore::new(consent_conn);
        consent_store
            .initialize_schema()
            .map_err(ServiceError::ConsentStore)?;
        let consent_manager = Arc::new(ConsentManager::new(consent_store));

        let escalation_queue = Arc::new(EscalationQueue::new(escalation_conn)?);

        let goal_sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&goal_conn)));
        let goal_repo = Arc::new(SqliteGoalRepository::new(goal_conn).with_telemetry(goal_sink));

        let standing_conn = open_db()?.conn_arc();
        let standing_session_store = Arc::new(StandingSessionStore::new(standing_conn));

        let sovereignty_conn = open_db()?.conn_arc();
        let sovereignty_boundary_store = SovereigntyBoundaryStore::new(sovereignty_conn);
        sovereignty_boundary_store
            .initialize_schema()
            .map_err(ServiceError::SovereigntyStore)?;

        let spec_conn = open_db()?.conn_arc();
        let spec_store = SqliteSpecStore::new(spec_conn);
        spec_store.init_schema().map_err(ServiceError::Spec)?;

        let user_conn = open_db()?.conn_arc();
        let user_store = Arc::new(std::sync::Mutex::new(UserStore::new(user_conn)));
        {
            let guard = user_store.lock().map_err(|_| {
                ServiceError::UserStore(hkask_storage::user_store::UserStoreError::Infra(
                    hkask_types::InfrastructureError::LockPoisoned,
                ))
            })?;
            guard.initialize_schema().map_err(ServiceError::UserStore)?;
        }

        // ── 4. CNS runtime + event sink ──────────────────────────────────────
        let cns_runtime = Arc::new(RwLock::new(CnsRuntime::with_threshold(
            config.cns_threshold,
        )));
        // Use the primary DB for CNS events so they persist in production.
        let cns_event_sink: Arc<dyn NuEventSink> =
            Arc::new(NuEventStore::new(Arc::clone(&primary_conn)));

        // ── 5. Loop system ──────────────────────────────────────────────────
        let dispatch = Arc::new(MessageDispatch::new());
        let loop_system = Arc::new(LoopSystem::new(Arc::clone(&dispatch)));
        let dispatch_sender = loop_system.dispatch_sender();

        // Cybernetics loop
        let set_points = load_set_points();
        let cybernetics_loop = CyberneticsLoop::with_set_points(
            Arc::clone(&cns_runtime),
            set_points,
            dispatch_sender.clone(),
        )
        .with_event_sink(Arc::clone(&cns_event_sink))
        .with_communication_queue_depth(loop_system.communication_queue_depth_counter());
        let cybernetics_loop = Arc::new(RwLock::new(cybernetics_loop));

        loop_system
            .register_loop(Arc::new(CyberneticsLoopHandle(Arc::clone(
                &cybernetics_loop,
            ))))
            .await;

        // Inference loop (optional — only if inference port is available)
        let inference_port: Option<Arc<dyn InferencePort>> = if config.in_memory {
            None
        } else {
            let okapi_config = OkapiConfig {
                base_url: config.okapi_base_url.clone(),
                ..OkapiConfig::default()
            };
            match hkask_templates::OkapiInference::new(&config.default_model, okapi_config) {
                Ok(inference) => {
                    let port: Arc<dyn InferencePort> = Arc::new(inference);
                    let inference_loop = hkask_agents::InferenceLoop::new()
                        .with_gas_budget(config.gas_budget_cap, config.gas_replenish_rate)
                        .with_model(&config.default_model);
                    loop_system.register_loop(Arc::new(inference_loop)).await;
                    Some(port)
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.services",
                        error = %e,
                        "Inference port initialization failed — inference unavailable"
                    );
                    None
                }
            }
        };

        // Episodic + Semantic loops
        // F9: Respect config.in_memory — use file-backed DB when persistence is configured.
        // User Sovereignty Guardrail: user configured persistent storage, must get persistence.
        let mem_db = if config.in_memory {
            in_memory_db()
        } else {
            let path = config
                .effective_memory_db_path()
                .expect("effective_memory_db_path returns Some when !in_memory");
            let passphrase = config
                .memory_passphrase
                .as_deref()
                .unwrap_or(&config.db_passphrase);
            Database::open(&path, passphrase)?
        };
        let mem_conn = mem_db.conn_arc();
        let triple_store = TripleStore::new(Arc::clone(&mem_conn));
        let episodic_memory = Arc::new(EpisodicMemory::new(triple_store));
        let storage_budget = episodic_memory.storage_budget();
        let episodic_loop =
            EpisodicLoop::new(Arc::clone(&episodic_memory), system_webid, storage_budget);
        loop_system.register_loop(Arc::new(episodic_loop)).await;

        let triple_store2 = TripleStore::new(Arc::clone(&mem_conn));
        let embedding_store = EmbeddingStore::new(Arc::clone(&mem_conn));
        let semantic_memory = Arc::new(SemanticMemory::new(triple_store2, embedding_store));
        let semantic_loop = SemanticLoop::new(Arc::clone(&semantic_memory));
        loop_system.register_loop(Arc::new(semantic_loop)).await;

        // Memory adapter for API-facing storage ports — creates owned instances
        // from the same shared connection as the loops above, ensuring writes
        // through the storage port are visible to loops via the shared database.
        let memory_adapter = Arc::new(
            hkask_agents::adapters::memory_loop_adapter::MemoryLoopAdapter::new(
                EpisodicMemory::new(TripleStore::new(Arc::clone(&mem_conn))),
                SemanticMemory::new(
                    TripleStore::new(Arc::clone(&mem_conn)),
                    EmbeddingStore::new(Arc::clone(&mem_conn)),
                ),
            ),
        );
        let episodic_storage: Arc<dyn EpisodicStoragePort> = memory_adapter.clone();
        let semantic_storage: Arc<dyn SemanticStoragePort> = memory_adapter.clone();

        // ── 6. Curation loop ─────────────────────────────────────────────────
        let cns_for_curator: Arc<CnsRuntime> = Arc::new(cns_runtime.read().await.clone());
        let acp_runtime = Arc::new(hkask_agents::AcpRuntime::new(&config.acp_secret));
        let acp_port: Arc<dyn hkask_agents::ports::AcpPort> = acp_runtime.clone();
        let curator_context = Arc::new(
            CuratorContext::new(
                CuratorHandle::system(),
                cns_for_curator,
                Arc::clone(&dispatch),
                Arc::clone(&escalation_queue),
            )
            .with_acp(acp_port)
            .with_loop_dispatch_tx(loop_system.dispatch_sender()),
        );
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
        loop_system.register_loop(curation_loop).await;

        // ── 7. GovernedTool membrane + MCP dispatcher ────────────────────────
        let mcp_runtime = McpRuntime::new();
        let raw_tool_port = Arc::new(RawMcpToolPort::new(mcp_runtime.clone()));
        let estimator: Arc<dyn GasEstimator> = Arc::new(CompositeGasEstimator::new());
        let governed_tool = Arc::new(GovernedTool::new(
            raw_tool_port,
            Arc::clone(&cybernetics_loop),
            Arc::clone(&cns_event_sink),
            estimator,
            system_webid,
            loop_system.dispatch_sender(),
        ));
        let mcp_dispatcher = Arc::new(McpDispatcher::with_governed_tool(
            mcp_runtime.clone(),
            &config.mcp_secret,
            governed_tool.clone(),
        ));
        let mcp_runtime = Arc::new(mcp_runtime);

        // ── 8. Pod manager + capability checker ──────────────────────────────
        let capability_checker = Arc::new(hkask_types::CapabilityChecker::new(&config.mcp_secret));
        let mcp_runtime_adapter = hkask_agents::adapters::mcp_runtime::FullMcpAdapter::new(
            Arc::new(hkask_types::CapabilityChecker::new(&config.acp_secret)),
            Arc::new((*mcp_runtime).clone()),
            tokio::runtime::Handle::current(),
        );
        let pod_manager = Arc::new(PodManager::new(
            Some(Arc::new(hkask_mcp::GitCasAdapter::from_path(
                std::path::PathBuf::from(&config.template_cache_path),
            ))),
            Some(acp_runtime.clone()),
            Some(Arc::new(mcp_runtime_adapter)),
            Some(Arc::clone(&episodic_storage) as Arc<dyn EpisodicStoragePort>),
            Some(Arc::clone(&semantic_storage) as Arc<dyn SemanticStoragePort>),
            None,
            Some(Arc::new(hkask_types::CapabilityChecker::new(
                &config.acp_secret,
            ))),
            Some(governed_tool.clone()),
            None,
        ));

        // ── 9. Registry ─────────────────────────────────────────────────────
        let registry = Arc::new(tokio::sync::Mutex::new(
            SqliteRegistry::new_with_conn(primary_conn.clone()).map_err(ServiceError::Template)?,
        ));

        // Agent registry store — uses the primary DB for persistent agent records.
        let agent_registry_store = hkask_storage::AgentRegistryStore::new(primary_conn.clone());
        agent_registry_store
            .initialize_schema()
            .map_err(ServiceError::AgentRegistryStore)?;

        // Restore ACP state from persistent storage
        let registered_agents = agent_registry_store
            .list()
            .map_err(ServiceError::AgentRegistryStore)?;
        if !registered_agents.is_empty() {
            use std::str::FromStr;
            let agents: Vec<hkask_agents::acp::AcpAgent> = registered_agents
                .iter()
                .map(|ra| hkask_agents::acp::AcpAgent {
                    webid: hkask_types::WebID::from_str(&ra.definition.name).unwrap_or_else(|_| {
                        hkask_types::WebID::from_persona(ra.definition.name.as_bytes())
                    }),
                    agent_type: ra.definition.agent_kind,
                    capabilities: ra.definition.capabilities.clone(),
                    registered_at: chrono::DateTime::parse_from_rfc3339(&ra.registered_at)
                        .map(|dt| dt.timestamp())
                        .unwrap_or(0),
                    active: true,
                })
                .collect();
            let tokens = std::collections::HashMap::new();
            acp_runtime
                .restore_from_storage(agents, tokens)
                .await
                .map_err(ServiceError::Acp)?;
        };

        // ── 10. Session manager for ensemble coordination ────────────────────
        let session_manager = Arc::new(RwLock::new(SessionManager::new(system_webid)));

        Ok(Self {
            registry,
            mcp_runtime,
            mcp_dispatcher,
            cns_runtime,
            cybernetics_loop,
            loop_system,
            dispatch,
            inference_port,
            episodic_storage,
            semantic_storage,
            escalation_queue,
            consent_manager,
            goal_repo,
            pod_manager,
            capability_checker,
            system_webid,
            event_sink: cns_event_sink,
            standing_session_store,
            sovereignty_boundary_store,
            spec_store,
            session_manager,
            acp_runtime,
            agent_registry_store,
            user_store,
            config,
        })
    }
}
