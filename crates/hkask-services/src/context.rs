//! Agent operational context — the full environment an agent needs to function.
//!
//! `AgentService` is the canonical composition root for hKask. It assembles
//! every piece of shared infrastructure an agent requires: CNS for variety
//! sensing, cybernetics for energy budgeting, MCP for tool discovery, wallet
//! for rJoule payments, memory for episodic/semantic recall, and all stores
//! (consent, goals, specs, registry, sovereignty).
//!
//! Both `ReplState` and `ApiState` compose an `AgentService` and add only
//! their surface-specific presentation fields. This replaces four independent
//! assembly paths that previously existed:
//! - `ReplState` init in `cli/repl/init.rs` (~325 lines)
//! - `ApiState::new()` in `api/lib.rs` (~400 lines)
//! - `build_loop_system()` in `api/loop_system.rs` (~130 lines)
//! - `commands/loops.rs` (~113 lines)
//!
//! # Adding new fields
//!
//! `AgentService` is the agent's operational world — not a dumping ground.
//! Before adding a field, apply the deletion test:
//! 1. Does the agent need this to function? If not, it belongs elsewhere.
//! 2. Does it already have a home crate/module? If yes, access it there.
//! 3. Is it surface-specific (CLI-only or API-only)? If yes, put it in the surface.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use hkask_agents::CuratorContext;
use hkask_agents::LoopSystem;
use hkask_agents::consent::ConsentManager;
use hkask_agents::curator_agent::CuratorAgent;
use hkask_agents::loop_system::CyberneticsLoopHandle;
use hkask_agents::pod::PodManager;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::{
    CnsRuntime, CompositeEnergyEstimator, CyberneticsLoop, EnergyEstimator, GovernedTool,
    SeamWatcher, SnapshotLoop, load_set_points,
};
use hkask_mcp::McpDispatcher;
use hkask_mcp::RawMcpToolPort;
use hkask_mcp::runtime::McpRuntime;
use hkask_memory::{
    ConsolidationBridge, EpisodicLoop, EpisodicMemory, SemanticLoop, SemanticMemory,
};
use hkask_storage::EscalationQueue;
use hkask_storage::goals::SqliteGoalRepository;
use hkask_storage::nu_event_store::NuEventStore;
use hkask_storage::user_store::UserStore;
use hkask_storage::{
    ConsentStore, Database, EmbeddingStore, SovereigntyBoundaryStore, SqliteSpecStore, TripleStore,
    WalletStore, in_memory_db,
};
use hkask_templates::SqliteRegistry;
use hkask_types::CapabilityChecker;
use hkask_types::CuratorHandle;
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::loops::HkaskLoop;
use hkask_types::loops::{CurationInput, CuratorDirective, ToolConsumptionEvent};
use hkask_types::ports::InferencePort;
use hkask_types::ports::git_cas::GitCASPort;
use hkask_types::wallet::WalletId;

use crate::ServiceConfig;
use crate::ServiceError;
use crate::SovereigntyService;
use crate::WalletService;

/// Agent operational context — canonical composition root for hKask.
///
/// Holds every piece of shared infrastructure an agent needs: CNS,
/// cybernetics, MCP, wallet, memory, stores, pod manager, Matrix transport.
/// Surfaces (`ReplState`, `ApiState`) compose this struct and add only
/// presentation-specific fields.
///
/// Construct via `AgentService::build(config)`. The config provides all
/// deployment-varying parameters (DB paths, secrets, thresholds, model names).
/// The builder resolves the dependency graph canonically: stores → CNS →
/// loop system → governed tool → pod manager.
///
/// # Field discipline
///
/// This is the agent's operational world — not a dumping ground. Before
/// adding a field, apply the deletion test (see module docs). Every field
/// here must be something an agent needs to function, not something that
/// was convenient to stash.
///
/// `#[non_exhaustive]` prevents external crates from constructing this struct
/// with struct literal syntax — use `AgentService::build()` instead.
#[non_exhaustive]
pub struct AgentService {
    /// Template registry.
    registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,

    /// MCP runtime for tool discovery and invocation.
    mcp_runtime: Arc<McpRuntime>,

    /// MCP dispatcher for OCAP-protected tool invocation.
    mcp_dispatcher: Arc<McpDispatcher>,

    /// CNS runtime for variety sensing and algedonic alerts.
    cns_runtime: Arc<RwLock<CnsRuntime>>,

    /// Cybernetics loop for energy budget regulation.
    cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,

    /// Loop system for 6-loop regulation.
    loop_system: Arc<LoopSystem>,

    /// Inference port for model invocation.
    inference_port: Option<Arc<dyn InferencePort>>,

    /// Episodic memory storage (private, agent-scoped).
    episodic_storage: Arc<dyn EpisodicStoragePort>,

    /// Semantic memory storage (public, shared).
    semantic_storage: Arc<dyn SemanticStoragePort>,

    /// Escalation queue for Curator escalations.
    escalation_queue: Arc<EscalationQueue>,

    /// Consent manager for user sovereignty.
    consent_manager: Arc<ConsentManager>,

    /// Goal repository for the goal coordination substrate.
    goal_repo: Arc<SqliteGoalRepository>,

    /// Channel for emitting CurationInput (GoalTransition, alerts, spec drift).
    curation_inbox_tx: Option<tokio::sync::mpsc::UnboundedSender<CurationInput>>,

    /// Pod manager for agent lifecycle.
    pod_manager: Arc<PodManager>,

    /// Capability checker for OCAP verification.
    ///
    /// Backed by `config.mcp_secret` — the inter-process HMAC key. Use this
    /// checker to derive tokens for any service operation that needs a verifiable
    /// capability token (e.g., `ChatService::chat()` memory access tokens).
    capability_checker: Arc<hkask_types::CapabilityChecker>,

    /// System WebID for signing capabilities.
    system_webid: WebID,

    /// Event sink for CNS audit trail.
    event_sink: Arc<dyn NuEventSink>,

    /// Sovereignty boundary store for Magna Carta compliance queries.
    sovereignty_boundary_store: SovereigntyBoundaryStore,

    /// Spec store for specification capture, validation, and cultivation.
    spec_store: SqliteSpecStore,

    /// ACP runtime for capability token management and agent registration.
    acp_runtime: Arc<hkask_agents::AcpRuntime>,

    /// Agent registry store for persistent agent records.
    agent_registry_store: hkask_storage::AgentRegistryStore,

    /// User store for replicant identity and authentication.
    user_store: Arc<std::sync::Mutex<UserStore>>,

    /// Daemon handler — bridges Unix socket queries to PodManager and UserStore.
    daemon_handler: Arc<crate::daemon_handler::ServiceDaemonHandler>,

    /// Matrix transport for agent-to-agent and human-to-agent communication.
    /// Owned by the daemon, shared with REPL, pod activation, and MCP wrapper.
    /// Wrapped in Mutex because login/reconnect take &mut self.
    matrix_transport: Option<Arc<tokio::sync::Mutex<hkask_communication::matrix::MatrixTransport>>>,

    /// R7.3 public seam watcher — loaded at startup, checked periodically.
    /// Wrapped in RwLock for shared mutable access between the periodic
    /// background task (which calls `check_drift(&mut self)`) and the
    /// Curator (which reads summary data).
    /// None if the inventory JSON file is missing (non-fatal).
    seam_watcher: Arc<RwLock<Option<SeamWatcher>>>,

    /// Configuration used to build this context.
    config: ServiceConfig,

    /// Wallet service for rJoule payments, deposits, withdrawals, and API key management.
    /// Constructed during build() with the wallet_config from ServiceConfig.
    wallet_service: Option<Arc<WalletService>>,

    /// Wallet store — shared between WalletManager, ApiKeyIssuer, and API key auth middleware.
    wallet_store: Option<Arc<WalletStore>>,
}

/// Per-agent memory infrastructure — storage ports and ConsolidationService
/// constructed from a single agent-scoped Database connection.
///
/// All components share the same underlying DB, so consolidation operates
/// on the agent's actual episodic and semantic triples.
pub struct PerAgentMemory {
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,
    pub semantic_storage: Arc<dyn SemanticStoragePort>,
    pub consolidation_service: hkask_memory::ConsolidationService,
}

impl AgentService {
    // === Configuration ===

    /// Access configuration.
    pub fn config(&self) -> &ServiceConfig {
        &self.config
    }

    /// Access the wallet service for rJoule payments and API key management.
    pub fn wallet(&self) -> Option<&Arc<WalletService>> {
        self.wallet_service.as_ref()
    }

    /// Access the wallet store for API key lookup and balance queries.
    pub fn wallet_store(&self) -> Option<&Arc<WalletStore>> {
        self.wallet_store.as_ref()
    }

    // === Named accessors (replaces positional tuple group methods) ===
    // # REQ: P4 (Clear Boundaries)

    // --- Memory ---
    pub fn memory(&self) -> (&Arc<dyn EpisodicStoragePort>, &Arc<dyn SemanticStoragePort>) {
        (&self.episodic_storage, &self.semantic_storage)
    }

    // --- Storage ---
    /// Template registry (tokio-Mutex-guarded for async lock compatibility).
    pub fn registry(&self) -> &Arc<tokio::sync::Mutex<SqliteRegistry>> {
        &self.registry
    }
    /// Goal repository.
    pub fn goal_repo(&self) -> &Arc<SqliteGoalRepository> {
        &self.goal_repo
    }

    // --- CNS ---
    /// CNS runtime for variety sensing and health checks.
    pub fn cns_runtime(&self) -> &Arc<RwLock<CnsRuntime>> {
        &self.cns_runtime
    }
    /// Cybernetics loop for energy budget regulation.
    pub fn cybernetics_loop(&self) -> &Arc<RwLock<CyberneticsLoop>> {
        &self.cybernetics_loop
    }
    /// Loop system for 6-loop regulation.
    pub fn loop_system(&self) -> &Arc<LoopSystem> {
        &self.loop_system
    }
    /// CNS event sink for the audit trail.
    pub fn event_sink(&self) -> &Arc<dyn NuEventSink> {
        &self.event_sink
    }
    /// R7.3 public seam watcher — None if inventory unavailable at startup.
    /// Returns a read lock on the watcher. For summary data, call
    /// `.read().await` and then `.as_ref().map(|w| w.summary())`.
    pub fn seam_watcher(&self) -> &Arc<RwLock<Option<SeamWatcher>>> {
        &self.seam_watcher
    }

    // --- Governance ---
    /// Capability checker for OCAP verification.
    /// # REQ: P4 (OCAP), P1 (User Sovereignty)
    pub fn capability_checker(&self) -> &Arc<CapabilityChecker> {
        &self.capability_checker
    }
    /// MCP dispatcher for OCAP-gated tool invocation.
    pub fn mcp_dispatcher(&self) -> &Arc<McpDispatcher> {
        &self.mcp_dispatcher
    }
    /// Escalation queue for Curator escalations.
    pub fn escalation_queue(&self) -> &Arc<EscalationQueue> {
        &self.escalation_queue
    }

    // --- Coordination ---
    /// Shared inference port (returns a clone of the `Option<Arc>`).
    pub fn inference_port(&self) -> Option<Arc<dyn InferencePort>> {
        self.inference_port.clone()
    }
    /// MCP runtime for tool discovery and invocation.
    pub fn mcp_runtime(&self) -> &Arc<McpRuntime> {
        &self.mcp_runtime
    }
    /// Pod manager for agent lifecycle.
    pub fn pod_manager(&self) -> &Arc<PodManager> {
        &self.pod_manager
    }

    // --- Identity ---
    /// System WebID + ACP runtime.
    pub fn identity(&self) -> (&WebID, &Arc<hkask_agents::AcpRuntime>) {
        (&self.system_webid, &self.acp_runtime)
    }

    /// Sovereignty: consent management service.
    /// consent_manager is PRIVATE — no raw store access.
    /// # REQ: P1 (User Sovereignty), P2 (Affirmative Consent)
    pub fn sovereignty(&self) -> SovereigntyService {
        SovereigntyService::new(self.consent_manager.clone())
    }

    // === Category 4: Internal implementation (crate-visible only) ===

    /// Access ACP runtime for agent registration and capability management.
    pub(crate) fn acp_runtime(&self) -> &Arc<hkask_agents::AcpRuntime> {
        &self.acp_runtime
    }

    /// Access curation inbox transmitter.
    pub fn curation_inbox_tx(&self) -> &Option<tokio::sync::mpsc::UnboundedSender<CurationInput>> {
        &self.curation_inbox_tx
    }

    /// Access sovereignty boundary store for Magna Carta compliance.
    /// TODO: Category 4 — migrate to service methods.
    pub fn sovereignty_boundary_store(&self) -> &SovereigntyBoundaryStore {
        &self.sovereignty_boundary_store
    }

    // === Surface-specific fields:

    /// Access spec store for specification capture, validation, and cultivation.
    /// TODO: Move to ApiState.
    pub fn spec_store(&self) -> &SqliteSpecStore {
        &self.spec_store
    }

    /// Access agent registry store for persistent agent records.
    /// TODO: Move to ApiState.
    pub fn agent_registry_store(&self) -> &hkask_storage::AgentRegistryStore {
        &self.agent_registry_store
    }

    /// Access user store for replicant identity and authentication.
    /// TODO: Move to ApiState.
    pub fn user_store(&self) -> &Arc<std::sync::Mutex<UserStore>> {
        &self.user_store
    }

    /// Access daemon handler for MCP binary communication.
    pub fn daemon_handler(&self) -> &Arc<crate::daemon_handler::ServiceDaemonHandler> {
        &self.daemon_handler
    }

    /// Access the shared Matrix transport, if connected.
    ///
    /// Returns `None` if Matrix is not configured or Conduit is unreachable.
    /// The transport is wrapped in a Mutex because `login`/`reconnect` take `&mut self`.
    pub fn matrix_transport(
        &self,
    ) -> Option<&Arc<tokio::sync::Mutex<hkask_communication::matrix::MatrixTransport>>> {
        self.matrix_transport.as_ref()
    }

    /// Build per-agent memory infrastructure from an agent-scoped Database.
    ///
    /// Constructs storage ports (`EpisodicStoragePort`, `SemanticStoragePort`)
    /// and a `ConsolidationService` — all sharing the same underlying DB
    /// connection so consolidation operates on the agent's actual triples.
    ///
    /// This is used by the REPL to build agent-scoped memory (separate from
    /// the shared `AgentService` memory adapted for loops).
    pub fn build_per_agent_memory(db: Database) -> PerAgentMemory {
        let conn = db.conn_arc();

        // EpisodicMemory + SemanticMemory for ConsolidationService
        let ts1 = TripleStore::new(Arc::clone(&conn));
        let episodic_memory = Arc::new(EpisodicMemory::new(ts1));
        let ts2 = TripleStore::new(Arc::clone(&conn));
        let emb = EmbeddingStore::new(Arc::clone(&conn));
        let semantic_memory = Arc::new(SemanticMemory::new(ts2, emb));

        // ConsolidationService from the shared memories
        let bridge = Arc::new(ConsolidationBridge::new(
            Arc::clone(&episodic_memory),
            Arc::clone(&semantic_memory),
        ));
        let handle = CuratorHandle::system();
        let token = handle.issue_consolidation_token();
        let consolidation_service =
            hkask_memory::ConsolidationService::new(bridge, semantic_memory, token);

        // Storage ports via MemoryLoopAdapter — uses the same connection
        let adapter = Arc::new(hkask_agents::adapters::MemoryLoopAdapter::new(
            EpisodicMemory::new(TripleStore::new(Arc::clone(&conn))),
            SemanticMemory::new(
                TripleStore::new(Arc::clone(&conn)),
                EmbeddingStore::new(Arc::clone(&conn)),
            ),
        ));

        PerAgentMemory {
            episodic_storage: adapter.clone() as Arc<dyn EpisodicStoragePort>,
            semantic_storage: adapter as Arc<dyn SemanticStoragePort>,
            consolidation_service,
        }
    }
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

impl AgentService {
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
        // Open ONE database and share the connection across all stores.
        // In production, all connections hit the same file on disk.
        // In test (in_memory), a single shared in-memory DB enables cross-store
        // operations — consent records visible to CNS, goals visible to memory, etc.
        let db = if config.in_memory {
            in_memory_db()
        } else {
            Database::open(&config.db_path, &config.db_passphrase)?
        };
        let shared_conn = db.conn_arc();

        let primary_conn = Arc::clone(&shared_conn);
        let consent_conn = Arc::clone(&shared_conn);
        let escalation_conn = Arc::clone(&shared_conn);
        let goal_conn = Arc::clone(&shared_conn);
        let sovereignty_conn = Arc::clone(&shared_conn);
        let spec_conn = Arc::clone(&shared_conn);
        let user_conn = Arc::clone(&shared_conn);

        // ── 3. Stores ───────────────────────────────────────────────────────
        // Shared channel for CurationInput — Cybernetics sends Alert,
        // SpecCurator sends SpecDrift, GoalStore sends GoalTransition.
        let (curation_inbox_tx, curation_inbox_rx) =
            tokio::sync::mpsc::unbounded_channel::<CurationInput>();

        let consent_store = ConsentStore::new(consent_conn);
        consent_store
            .initialize_schema()
            .map_err(ServiceError::ConsentStore)?;
        let consent_manager = Arc::new(ConsentManager::new(consent_store));

        let escalation_queue = Arc::new(EscalationQueue::new(escalation_conn)?);

        let goal_sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&goal_conn)));
        let goal_repo = Arc::new(SqliteGoalRepository::new(goal_conn).with_telemetry(goal_sink));

        let sovereignty_boundary_store = SovereigntyBoundaryStore::new(sovereignty_conn);
        sovereignty_boundary_store
            .initialize_schema()
            .map_err(ServiceError::SovereigntyStore)?;

        let spec_store = SqliteSpecStore::new(spec_conn);
        spec_store.init_schema().map_err(ServiceError::Spec)?;

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

        // ── 4a. Seam watcher (R7.3) — load public seam inventory, register domains ──
        // Non-fatal: if no inventory is available, seam watching is silently disabled.
        // Uses embedded JSON (compile-time) with file path override (HKASK_SEAM_INVENTORY_PATH).
        let seam_watcher: Arc<RwLock<Option<SeamWatcher>>> = {
            let cns = cns_runtime.read().await;
            if let Some(watcher) = SeamWatcher::load() {
                watcher.register_domains(&cns).await;
                let summary = watcher.summary();
                tracing::info!(
                    target: "bootstrap",
                    crates = %summary.crate_count,
                    coverage_pct = %summary.coverage_pct,
                    total_items = %summary.total_items,
                    "Seam watcher initialized — R7.3 watching the public seam"
                );
                Arc::new(RwLock::new(Some(watcher)))
            } else {
                tracing::info!(
                    target: "bootstrap",
                    "Seam watcher skipped — no inventory available (non-fatal)"
                );
                Arc::new(RwLock::new(None))
            }
        };

        // Use the primary DB for CNS events so they persist in production.
        let cns_event_sink: Arc<dyn NuEventSink> =
            Arc::new(NuEventStore::new(Arc::clone(&primary_conn)));

        // ── 4b. Spawn periodic seam drift check (R7.3 background watcher) ──
        // Runs on a configurable interval (default: 30 minutes). Checks for
        // drift from the previous snapshot, increments variety, and emits CNS spans.
        {
            let watcher_lock = Arc::clone(&seam_watcher);
            let cns = Arc::clone(&cns_runtime);
            let event_sink = Arc::clone(&cns_event_sink);

            let interval_secs: u64 = std::env::var("HKASK_SEAM_CHECK_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1800); // default: 30 minutes

            tokio::spawn(async move {
                tracing::info!(
                    target: "cns.architecture.seam",
                    interval_secs = %interval_secs,
                    "Seam periodic drift check started — R7.3 watching every {}s",
                    interval_secs
                );

                // Initial check immediately after startup
                {
                    let cns_rt = cns.read().await;
                    let mut guard = watcher_lock.write().await;
                    if let Some(ref mut watcher) = *guard {
                        let drifts = watcher.check_drift(&cns_rt, &*event_sink).await;
                        if !drifts.is_empty() {
                            tracing::info!(
                                target: "cns.architecture.seam",
                                drift_count = %drifts.len(),
                                "Initial seam drift check complete"
                            );
                        }
                    }
                }

                // Periodic loop
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(interval_secs));
                loop {
                    interval.tick().await;

                    let cns_rt = cns.read().await;
                    let mut guard = watcher_lock.write().await;
                    if let Some(ref mut watcher) = *guard {
                        // Try refresh first (only works if file path is set)
                        let refreshed = watcher.refresh();
                        if refreshed {
                            tracing::debug!(
                                target: "cns.architecture.seam",
                                "Seam inventory refreshed before periodic check"
                            );
                        }

                        let drifts = watcher.check_drift(&cns_rt, &*event_sink).await;
                        if !drifts.is_empty() {
                            let degradations: Vec<_> =
                                drifts.iter().filter(|d| d.delta_pct < 0.0).collect();
                            let improvements: Vec<_> =
                                drifts.iter().filter(|d| d.delta_pct > 0.0).collect();
                            tracing::info!(
                                target: "cns.architecture.seam",
                                total_drifts = %drifts.len(),
                                degradations = %degradations.len(),
                                improvements = %improvements.len(),
                                "Periodic seam drift check complete"
                            );
                        }
                    } else {
                        tracing::debug!(
                            target: "cns.architecture.seam",
                            "Periodic seam check skipped — no watcher"
                        );
                    }
                    drop(guard);
                    drop(cns_rt);
                }
            });
        }

        // ── 5. Loop system ──────────────────────────────────────────────────
        let loop_system = Arc::new(LoopSystem::new());

        // Direct tool consumption channel: GovernedTool → Cybernetics.
        let (tool_consumption_tx, tool_consumption_rx) =
            tokio::sync::mpsc::unbounded_channel::<ToolConsumptionEvent>();

        // Direct curator directive channel: Curation → Cybernetics.
        let (curator_directive_tx, curator_directive_rx) =
            tokio::sync::mpsc::unbounded_channel::<CuratorDirective>();

        // Cybernetics loop
        let set_points = load_set_points();
        let cybernetics_loop =
            CyberneticsLoop::with_set_points(Arc::clone(&cns_runtime), set_points)
                .with_event_sink(Arc::clone(&cns_event_sink))
                .with_alerts_channel(curation_inbox_tx.clone())
                .with_tool_consumption_channel(tool_consumption_rx)
                .with_curator_directive_channel(curator_directive_rx);
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
            let router = hkask_inference::InferenceRouter::new(config.inference_config.clone());
            let raw_port: Arc<dyn InferencePort> = Arc::new(router);
            // Wrap with GovernedInference membrane for energy budget enforcement
            let governed_port: Arc<dyn InferencePort> =
                Arc::new(hkask_cns::GovernedInference::new(
                    raw_port,
                    Arc::clone(&cybernetics_loop),
                    Arc::clone(&cns_event_sink),
                    system_webid,
                ));
            let inference_loop = hkask_agents::InferenceLoop::new()
                .with_energy_budget(config.energy_budget_cap, config.gas_replenish_rate)
                .with_model(&config.default_model);
            loop_system.register_loop(Arc::new(inference_loop)).await;
            Some(governed_port)
        };

        // Episodic + Semantic loops
        // F9: Respect config.in_memory — use file-backed DB when persistence is configured.
        // User Sovereignty Guardrail: user configured persistent storage, must get persistence.
        //
        // In in_memory mode, share the main database connection so memory stores
        // coexist with consent, goals, specs, and CNS events — enabling cross-store
        // queries and CNS observation of memory operations (P6, P9).
        let mem_conn = if config.in_memory {
            Arc::clone(&shared_conn)
        } else {
            let path = config
                .effective_memory_db_path()
                .expect("effective_memory_db_path returns Some when !in_memory");
            let passphrase = config
                .memory_passphrase
                .as_deref()
                .unwrap_or(&config.db_passphrase);
            Database::open(&path, passphrase)?.conn_arc()
        };
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
                Some(curator_directive_tx),
                Arc::clone(&escalation_queue),
            )
            .with_acp(acp_port),
        );
        let consolidation_bridge = Arc::new(ConsolidationBridge::new(
            Arc::clone(&episodic_memory),
            Arc::clone(&semantic_memory),
        ));
        let curator_agent = CuratorAgent::with_consolidation(
            curator_context,
            Default::default(),
            Arc::clone(&consolidation_bridge),
            Some(curation_inbox_rx),
            Some(curation_inbox_tx.clone()),
        );
        let curation_loop: Arc<dyn HkaskLoop> = curator_agent.curation_loop().clone();
        loop_system.register_loop(curation_loop).await;

        // ── 6b. Snapshot loop (Cybernetics sub-function) ─────────────────────
        let git_cas_port: Arc<dyn GitCASPort> = match hkask_mcp::GixCasAdapter::from_env() {
            Ok(adapter) => Arc::new(adapter),
            Err(e) => {
                tracing::warn!(target: "hkask.services", error = %e, "Git CAS port from env failed — using fallback");
                Arc::new(
                    hkask_mcp::GixCasAdapter::new(PathBuf::from("/tmp/hkask-templates")).map_err(
                        |e| {
                            ServiceError::Infra(hkask_types::InfrastructureError::Io(e.to_string()))
                        },
                    )?,
                )
            }
        };
        let snapshot_loop = SnapshotLoop::new(Arc::clone(&git_cas_port));
        loop_system.register_loop(Arc::new(snapshot_loop)).await;

        // ── 6c. Backup loop (daily snapshots via BackupService) ──────────────
        let backup_service = Arc::new(crate::BackupService::new(Arc::clone(&git_cas_port)));
        let backup_loop = crate::backup::r#loop::BackupLoop::new(backup_service);
        loop_system.register_loop(Arc::new(backup_loop)).await;

        // ── 7. GovernedTool membrane + MCP dispatcher ────────────────────────
        let mcp_runtime = McpRuntime::new();
        let raw_tool_port = Arc::new(RawMcpToolPort::new(mcp_runtime.clone()));
        let estimator: Arc<dyn EnergyEstimator> = Arc::new(CompositeEnergyEstimator::new());
        let governed_tool = Arc::new(
            GovernedTool::new(
                raw_tool_port,
                Arc::clone(&cybernetics_loop),
                Arc::clone(&cns_event_sink),
                estimator,
                system_webid,
            )
            .with_tool_consumption_channel(tool_consumption_tx),
        );
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

        // Register Matrix auto-registration hook for pod activation.
        // When a pod is activated, the replicant gets a Matrix account on Conduit.
        // Uses MatrixTransport from the core communication crate.
        {
            let homeserver_url = std::env::var("HKASK_MATRIX_URL")
                .unwrap_or_else(|_| "http://localhost:8008".to_string());
            pod_manager
                .register_activation_hook(Box::new(move |webid, pod_name| {
                    let url = homeserver_url.clone();
                    let name = pod_name.clone();
                    tokio::spawn(async move {
                        register_pod_on_matrix(&url, &webid, &name).await;
                    });
                }))
                .await;
        }

        // ── 8b. Daemon handler + listener ──────────────────────────────────
        // Skip daemon socket binding in test mode (in_memory) — the Unix socket
        // address conflicts when multiple tests run in parallel.
        let daemon_handler = Arc::new(crate::daemon_handler::ServiceDaemonHandler::new(
            Arc::clone(&pod_manager),
            Arc::clone(&user_store),
            inference_port.clone(),
        ));
        if !config.in_memory {
            let mut daemon_listener = hkask_mcp::daemon::DaemonListener::new();
            daemon_listener.bind().await.map_err(|e| {
                ServiceError::Infra(hkask_types::InfrastructureError::Io(format!(
                    "Failed to bind daemon socket: {}",
                    e
                )))
            })?;
            // Spawn daemon serve loop in background (fire-and-forget)
            let serve_handler = Arc::clone(&daemon_handler);
            tokio::spawn(async move {
                if let Err(e) = daemon_listener.serve(serve_handler).await {
                    tracing::error!(
                        target: "hkask.daemon",
                        error = %e,
                        "Daemon listener serve loop exited with error"
                    );
                }
            });
        }

        // ── 8c. Matrix transport + 7R7 listener ────────────────────────────
        // Resolve Matrix credentials from keychain (stored during onboarding
        // or bootstrap), create transport, login, and start the passive 7R7
        // listener. Non-blocking: if Conduit isn't running, the daemon
        // continues without Matrix and the transport remains None.
        let matrix_transport: Option<
            Arc<tokio::sync::Mutex<hkask_communication::matrix::MatrixTransport>>,
        > = {
            let homeserver_url = std::env::var("HKASK_MATRIX_URL")
                .unwrap_or_else(|_| "http://localhost:8008".to_string());

            let keychain = hkask_keystore::Keychain::default();

            // Resolve credentials: curator bot → replicant → env vars
            let credentials = {
                // 1. Try matrix-bot-curator (system bot account from bootstrap)
                if let Ok(password) = keychain.retrieve_by_key("matrix-bot-curator") {
                    Some(("@hkask-curator:localhost".to_string(), password))
                }
                // 2. Fall back to replicant account (from onboarding)
                else if let (Ok(username), Ok(password)) = (
                    keychain.retrieve_by_key("matrix-replicant-username"),
                    keychain.retrieve_by_key("matrix-replicant-password"),
                ) {
                    Some((username, password))
                }
                // 3. Fall back to environment variables (backward compat)
                else if let (Ok(username), Ok(password)) = (
                    std::env::var("HKASK_MATRIX_AGENT_USERNAME"),
                    std::env::var("HKASK_MATRIX_AGENT_PASSWORD"),
                ) {
                    Some((username, password))
                } else {
                    None
                }
            };

            match credentials {
                Some((username, password)) => {
                    let mut transport =
                        hkask_communication::matrix::MatrixTransport::new(&homeserver_url);
                    match transport.login(&username, &password).await {
                        Ok(()) => {
                            let transport = Arc::new(tokio::sync::Mutex::new(transport));

                            // Start 7R7 passive listener — polls rooms, emits CNS spans.
                            // Does NOT classify, escalate, or moderate.
                            let listener = hkask_communication::listener::SevenR7Listener::new(
                                transport.clone(),
                                30, // poll every 30 seconds
                            );
                            listener.start().await;

                            tracing::info!(
                                target: "cns.communication.matrix.daemon",
                                username = %username,
                                homeserver = %homeserver_url,
                                "Matrix transport connected and 7R7 listener started"
                            );

                            Some(transport)
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "cns.communication.matrix.daemon",
                                username = %username,
                                error = %e,
                                "Matrix login failed — Conduit may not be running. Continuing without Matrix."
                            );
                            None
                        }
                    }
                }
                None => {
                    tracing::info!(
                        target: "cns.communication.matrix.daemon",
                        "No Matrix credentials found in keychain or environment. Continuing without Matrix."
                    );
                    None
                }
            }
        };

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

        // ── 10. Wallet — rJoule payments, deposits, API keys ─────────────────
        let (wallet_service, wallet_store): (Option<Arc<WalletService>>, Option<Arc<WalletStore>>) = {
            let wallet_conn = Arc::clone(&shared_conn);
            let wallet_store = Arc::new(WalletStore::new(wallet_conn));

            let svc = WalletService::build(
                &config.wallet_config,
                Arc::clone(&wallet_store),
                Arc::clone(&cns_event_sink),
                Arc::clone(&cybernetics_loop),
            )?;

            let wallet_manager = svc.manager();

            // Ensure default wallet exists (system-wide fallback)
            let default_wallet = WalletId::default();
            wallet_manager
                .ensure_wallet(default_wallet)
                .map_err(|e| ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: "Failed to ensure default wallet".into(),
                })?;

            // Bind wallets to all registered replicants (multi-wallet foundation).
            // Each replicant gets a unique WalletId derived from its name.
            // Replicants without wallets get one created; existing bindings are preserved.
            {
                let user_guard = user_store.lock().map_err(|_| {
                    ServiceError::UserStore(hkask_storage::user_store::UserStoreError::Infra(
                        hkask_types::InfrastructureError::LockPoisoned,
                    ))
                })?;
                // Get the system replicant to discover the user_id
                if let Ok(Some(system_identity)) = user_guard.get_replicant(&config.agent_name) {
                    let user_id = system_identity.user_id;
                    let replicants = user_guard
                        .list_replicants(&user_id)
                        .map_err(ServiceError::UserStore)?;
                    for identity in &replicants {
                        // Skip replicants that already have a wallet bound
                        if identity.wallet_id.is_some() {
                            tracing::debug!(
                                target: "cns.wallet",
                                replicant = %identity.replicant_name,
                                wallet_id = %identity.wallet_id.as_ref().unwrap(),
                                "Wallet already bound — skipping"
                            );
                            continue;
                        }
                        // Derive a deterministic WalletId from the replicant name
                        let wallet_id = WalletId::from_name(&identity.replicant_name);
                        if let Err(e) = wallet_manager.ensure_wallet(wallet_id) {
                            tracing::warn!(
                                target: "cns.wallet",
                                replicant = %identity.replicant_name,
                                error = %e,
                                "Failed to create wallet for replicant"
                            );
                            continue;
                        }
                        if let Err(e) =
                            user_guard.set_wallet_id(&identity.replicant_name, wallet_id)
                        {
                            tracing::warn!(
                                target: "cns.wallet",
                                replicant = %identity.replicant_name,
                                error = %e,
                                "Failed to bind wallet to replicant"
                            );
                        } else {
                            tracing::info!(
                                target: "cns.wallet",
                                replicant = %identity.replicant_name,
                                wallet_id = %wallet_id,
                                "Wallet created and bound to replicant"
                            );
                        }
                    }
                } else {
                    tracing::info!(
                        target: "cns.wallet",
                        "No system replicant found — skipping wallet binding"
                    );
                }
            }

            // Spawn deposit monitor as background task
            let monitor_manager = Arc::clone(&wallet_manager);
            let interval_secs: u64 = std::env::var("HKASK_DEPOSIT_MONITOR_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30); // default: poll every 30 seconds
            tokio::spawn(async move {
                tracing::info!(
                    target: "cns.wallet.deposit",
                    interval_secs = %interval_secs,
                    "Deposit monitor started — polling every {}s",
                    interval_secs
                );
                if let Err(e) = monitor_manager.start_deposit_monitor(interval_secs).await {
                    tracing::error!(
                        target: "cns.wallet.deposit",
                        error = %e,
                        "Deposit monitor loop exited with error"
                    );
                }
            });

            (Some(svc), Some(wallet_store))
        };

        Ok(Self {
            registry,
            mcp_runtime,
            mcp_dispatcher,
            cns_runtime,
            cybernetics_loop,
            loop_system,
            inference_port,
            episodic_storage,
            semantic_storage,
            escalation_queue,
            consent_manager,
            goal_repo,
            curation_inbox_tx: Some(curation_inbox_tx.clone()),
            pod_manager,
            capability_checker,
            system_webid,
            event_sink: cns_event_sink,
            sovereignty_boundary_store,
            spec_store,
            acp_runtime,
            agent_registry_store,
            user_store,
            daemon_handler,
            matrix_transport,
            seam_watcher,
            config,
            wallet_service,
            wallet_store,
        })
    }
}

/// Register a pod's replicant as a Matrix user on Conduit.
///
/// Called from the pod activation hook. Uses MatrixTransport from the
/// core communication crate for registration.
async fn register_pod_on_matrix(homeserver_url: &str, _webid: &hkask_types::WebID, pod_name: &str) {
    let localpart = pod_name.to_lowercase().replace(' ', "-");
    let username = format!("{}-bot", localpart);
    let password = uuid::Uuid::new_v4().to_string();

    let _transport = hkask_communication::matrix::MatrixTransport::new(homeserver_url);

    // Register directly on Conduit — MatrixTransport doesn't have a register
    // method (it uses login), so we use the same HTTP approach as onboarding.
    let url = format!(
        "{}/_matrix/client/v3/register",
        homeserver_url.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "username": &username,
        "password": &password,
        "initial_device_display_name": format!("hKask Pod: {}", pod_name),
        "auth": {"type": "m.login.dummy"}
    });

    match reqwest::Client::new()
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            let full_id = format!("@{}:localhost", username);
            let keychain = hkask_keystore::Keychain::default();
            let _ = keychain.store_by_key(&format!("matrix-pod-{}", pod_name), &password);
            tracing::info!(
                target: "cns.communication.matrix.pod_registered",
                pod = %pod_name,
                matrix_id = %full_id,
                "Pod replicant registered on Matrix"
            );
        }
        Ok(response) => {
            tracing::warn!(
                target: "cns.communication.matrix.pod_registered",
                pod = %pod_name,
                status = %response.status().as_u16(),
                "Matrix registration for pod failed — Conduit may not be running"
            );
        }
        Err(e) => {
            tracing::warn!(
                target: "cns.communication.matrix.pod_registered",
                pod = %pod_name,
                error = %e,
                "Matrix registration for pod failed — Conduit unreachable"
            );
        }
    }
}
