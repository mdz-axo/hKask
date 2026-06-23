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
use hkask_agents::curator::SemanticIndex;
use hkask_agents::curator::sync_port::SemanticIndexSyncPort;
use hkask_agents::curator_agent::CuratorAgent;
use hkask_agents::loop_system::CyberneticsLoopHandle;
use hkask_agents::pod::ActivePods;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_capability::CapabilityChecker;
use hkask_cns::types::loops::CuratorHandle;
use hkask_cns::types::loops::HkaskLoop;
use hkask_cns::types::loops::{CurationInput, CuratorDirective, ToolConsumptionEvent};
use hkask_cns::{
    CalibratedEnergyEstimator, CnsRuntime, CyberneticsLoop, EnergyEstimator, GovernedTool,
    SeamWatcher, SnapshotLoop, load_set_points,
};
use hkask_federation::sync::FederationLinkManager;
use hkask_federation::sync::FederationSync;
use hkask_federation::sync::transport::InMemoryFederationTransport;
use hkask_mcp::McpDispatcher;
use hkask_mcp::RawMcpToolPort;
use hkask_mcp::runtime::McpRuntime;
use hkask_memory::{
    ConsolidationBridge, EpisodicLoop, EpisodicMemory, SemanticLoop, SemanticMemory,
};
use hkask_ports::InferencePort;
use hkask_ports::federation::{FederationDispatch, FederationSyncPort};
use hkask_ports::git_cas::GitCASPort;
use hkask_storage::EscalationQueue;
use hkask_storage::goals::SqliteGoalRepository;
use hkask_storage::nu_event_store::NuEventStore;
use hkask_storage::user_store::UserStore;
use hkask_storage::{
    ConsentStore, Database, EmbeddingStore, SovereigntyBoundaryStore, SqliteSpecStore, TripleStore,
    WalletStore, in_memory_db,
};
use hkask_templates::SqliteRegistry;
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::id::WalletId;

use hkask_services_core::ServiceConfig;
use hkask_services_core::ServiceError;
use hkask_services_sovereignty::SovereigntyService;
use hkask_services_wallet::WalletService;

mod matrix;
mod seam_monitor;

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
    pod_manager: Arc<ActivePods>,

    /// Capability checker for OCAP verification.
    ///
    /// Backed by `config.mcp_secret` — the inter-process HMAC key. Use this
    /// checker to derive tokens for any service operation that needs a verifiable
    /// capability token (e.g., `ChatService::chat()` memory access tokens).
    capability_checker: Arc<hkask_capability::CapabilityChecker>,

    /// System WebID for signing capabilities.
    system_webid: WebID,

    /// Event sink for CNS audit trail.
    event_sink: Arc<dyn NuEventSink>,

    /// Calibrated energy estimator with a background gas-table refresh loop.
    energy_estimator: Arc<hkask_cns::CalibratedEnergyEstimator>,

    /// Sovereignty boundary store for Magna Carta compliance queries.
    sovereignty_boundary_store: SovereigntyBoundaryStore,

    /// Spec store for specification capture, validation, and cultivation.
    spec_store: SqliteSpecStore,

    /// A2A runtime for capability token management and agent registration.
    a2a_runtime: Arc<hkask_agents::A2ARuntime>,

    /// Agent registry store for persistent agent records.
    agent_registry_store: hkask_storage::AgentRegistryStore,

    /// User store for replicant identity and authentication.
    user_store: Arc<std::sync::Mutex<UserStore>>,

    /// Daemon handler — bridges Unix socket queries to PodManager and UserStore.
    daemon_handler: Arc<hkask_services_daemon::ServiceDaemonHandler>,

    /// Matrix transport for agent-to-agent and human-to-agent communication.
    /// Owned by the daemon, shared with REPL, pod activation, and MCP wrapper.
    /// Wrapped in Mutex because login/reconnect take &mut self.
    matrix_transport: Option<Arc<tokio::sync::Mutex<hkask_communication::matrix::MatrixTransport>>>,

    /// Signals CuratorPod activation. Consumed by callers that need to
    /// await curator readiness before accepting requests.
    curator_ready: Option<tokio::sync::oneshot::Receiver<()>>,

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

    /// Wallet gas calibrator — runtime calibration of gas→rJoule conversion rate.
    wallet_gas_calibrator: Option<Arc<hkask_cns::WalletGasCalibrator>>,

    /// Federation link manager — set when federation is enabled.
    federation_link_manager: Option<Arc<dyn FederationDispatch>>,
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

impl From<&AgentService> for hkask_services_inference_svc::InferenceContext {
    fn from(ctx: &AgentService) -> Self {
        Self {
            shared_port: ctx.inference_port(),
            default_model: ctx.config().default_model.clone(),
            inference_config: ctx.config().inference_config.clone(),
        }
    }
}

impl AgentService {
    // === Configuration ===

    /// Access configuration.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns reference to ServiceConfig
    pub fn config(&self) -> &ServiceConfig {
        &self.config
    }

    /// Access the wallet service for rJoule payments and API key management.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns Some(&`Arc<WalletService>`) if wallet configured; None otherwise
    pub fn wallet(&self) -> Option<&Arc<WalletService>> {
        self.wallet_service.as_ref()
    }

    /// Access the wallet store for API key lookup and balance queries.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns Some(&`Arc<WalletStore>`) if wallet store configured; None otherwise
    pub fn wallet_store(&self) -> Option<&Arc<WalletStore>> {
        self.wallet_store.as_ref()
    }

    /// Access the wallet gas calibrator.
    ///
    /// \[P7\] Motivating: Evolutionary Architecture — parameter emerged from real usage and is calibrated at runtime.
    /// pre:  self must be fully built
    /// post: returns Some(&`Arc<WalletGasCalibrator>`) if wallet is configured; None otherwise
    pub fn wallet_gas_calibrator(&self) -> Option<&Arc<hkask_cns::WalletGasCalibrator>> {
        self.wallet_gas_calibrator.as_ref()
    }

    // === Named accessors (replaces positional tuple group methods) ===
    // # REQ: P4 (Clear Boundaries)
    // # expect: "Service boundaries enforce OCAP membranes"

    // --- Memory ---
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns (&episodic_storage, &semantic_storage) tuple
    pub fn memory(&self) -> (&Arc<dyn EpisodicStoragePort>, &Arc<dyn SemanticStoragePort>) {
        (&self.episodic_storage, &self.semantic_storage)
    }

    // --- Storage ---
    /// Template registry (tokio-Mutex-guarded for async lock compatibility).
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns &Arc<`Mutex<SqliteRegistry>`>
    pub fn registry(&self) -> &Arc<tokio::sync::Mutex<SqliteRegistry>> {
        &self.registry
    }
    /// Goal repository.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns `&Arc<SqliteGoalRepository>`
    pub fn goal_repo(&self) -> &Arc<SqliteGoalRepository> {
        &self.goal_repo
    }

    // --- CNS ---
    /// CNS runtime for variety sensing and health checks.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns &Arc<Rw`Lock<CnsRuntime>`>
    pub fn cns_runtime(&self) -> &Arc<RwLock<CnsRuntime>> {
        &self.cns_runtime
    }
    /// Cybernetics loop for energy budget regulation.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns &Arc<Rw`Lock<CyberneticsLoop>`>
    pub fn cybernetics_loop(&self) -> &Arc<RwLock<CyberneticsLoop>> {
        &self.cybernetics_loop
    }
    /// Loop system for 6-loop regulation.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns `&Arc<LoopSystem>`
    pub fn loop_system(&self) -> &Arc<LoopSystem> {
        &self.loop_system
    }
    /// CNS event sink for the audit trail.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns `&Arc<dyn NuEventSink>`
    pub fn event_sink(&self) -> &Arc<dyn NuEventSink> {
        &self.event_sink
    }

    /// Calibrated energy estimator with a background gas-table refresh loop.
    ///
    /// \[P7\] Motivating: Evolutionary Architecture — parameter emerged from real usage and is calibrated at runtime.
    /// pre:  self must be fully built
    /// post: returns &`Arc<CalibratedEnergyEstimator>` sharing the same background
    ///       calibration loop as the service's governed tool
    pub fn energy_estimator(&self) -> &Arc<hkask_cns::CalibratedEnergyEstimator> {
        &self.energy_estimator
    }

    /// R7.3 public seam watcher — None if inventory unavailable at startup.
    /// Returns a read lock on the watcher. For summary data, call
    /// `.read().await` and then `.as_ref().map(|w| w.summary())`.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns ``&Arc<RwLock<Option<SeamWatcher>>>``
    pub fn seam_watcher(&self) -> &Arc<RwLock<Option<SeamWatcher>>> {
        &self.seam_watcher
    }

    // --- Governance ---
    /// Capability checker for OCAP verification.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns `&Arc<CapabilityChecker>`
    /// # REQ: P4 (OCAP), P1 (User Sovereignty)
    /// # expect: "Service boundaries enforce OCAP membranes"
    pub fn capability_checker(&self) -> &Arc<CapabilityChecker> {
        &self.capability_checker
    }
    /// MCP dispatcher for OCAP-gated tool invocation.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns `&Arc<McpDispatcher>`
    pub fn mcp_dispatcher(&self) -> &Arc<McpDispatcher> {
        &self.mcp_dispatcher
    }
    /// Escalation queue for Curator escalations.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns `&Arc<EscalationQueue>`
    pub fn escalation_queue(&self) -> &Arc<EscalationQueue> {
        &self.escalation_queue
    }

    // --- Coordination ---
    /// Shared inference port (returns a clone of the `Option<Arc>`).
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns Some(`Arc<dyn InferencePort>`) if configured; None otherwise
    pub fn inference_port(&self) -> Option<Arc<dyn InferencePort>> {
        self.inference_port.clone()
    }
    /// MCP runtime for tool discovery and invocation.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns `&Arc<McpRuntime>`
    pub fn mcp_runtime(&self) -> &Arc<McpRuntime> {
        &self.mcp_runtime
    }
    /// Pod manager for agent lifecycle.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns `&Arc<ActivePods>`
    pub fn pod_manager(&self) -> &Arc<ActivePods> {
        &self.pod_manager
    }

    // --- Identity ---
    /// System WebID + A2A runtime.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns (&WebID, &`Arc<A2ARuntime>`) tuple
    pub fn identity(&self) -> (&WebID, &Arc<hkask_agents::A2ARuntime>) {
        (&self.system_webid, &self.a2a_runtime)
    }

    /// Sovereignty: consent management service.
    /// consent_manager is PRIVATE — no raw store access.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns SovereigntyService wrapping the consent manager
    /// # REQ: P1 (User Sovereignty), P2 (Affirmative Consent)
    /// # expect: "My service operations flow through sovereignty-verifying boundaries"
    pub fn sovereignty(&self) -> SovereigntyService {
        SovereigntyService::new(self.consent_manager.clone())
    }

    // === Category 4: Internal implementation (crate-visible only) ===

    /// Access A2A runtime for agent registration and capability management.
    ///
    /// \[P3\] Motivating: Generative Space — A2A runtime access without ambient authority.
    /// pre:  self must be fully built
    /// post: returns &`Arc<A2ARuntime>` reference
    pub fn a2a_runtime(&self) -> &Arc<hkask_agents::A2ARuntime> {
        &self.a2a_runtime
    }

    /// Access curation inbox transmitter.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns &Option<Unbounded`Sender<CurationInput>`>
    pub fn curation_inbox_tx(&self) -> &Option<tokio::sync::mpsc::UnboundedSender<CurationInput>> {
        &self.curation_inbox_tx
    }

    /// Access sovereignty boundary store for Magna Carta compliance.
    /// TODO: Category 4 — migrate to service methods.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns &SovereigntyBoundaryStore
    pub fn sovereignty_boundary_store(&self) -> &SovereigntyBoundaryStore {
        &self.sovereignty_boundary_store
    }

    // === Surface-specific fields:

    /// Access spec store for specification capture, validation, and cultivation.
    /// TODO: Move to ApiState.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns &SqliteSpecStore
    pub fn spec_store(&self) -> &SqliteSpecStore {
        &self.spec_store
    }

    /// Access agent registry store for persistent agent records.
    /// TODO: Move to ApiState.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns &AgentRegistryStore
    pub fn agent_registry_store(&self) -> &hkask_storage::AgentRegistryStore {
        &self.agent_registry_store
    }

    /// Access user store for replicant identity and authentication.
    /// TODO: Move to ApiState.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns &Arc<`Mutex<UserStore>`>
    pub fn user_store(&self) -> &Arc<std::sync::Mutex<UserStore>> {
        &self.user_store
    }

    /// Access daemon handler for MCP binary communication.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns `&Arc<ServiceDaemonHandler>`
    pub fn daemon_handler(&self) -> &Arc<hkask_services_daemon::ServiceDaemonHandler> {
        &self.daemon_handler
    }

    /// Access the federation dispatch port, if federation is enabled.
    ///
    /// Returns None if federation is not configured or disabled.
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence
    pub fn federation_dispatch(&self) -> Option<&Arc<dyn FederationDispatch>> {
        self.federation_link_manager.as_ref()
    }

    /// Access the shared Matrix transport, if connected.
    ///
    /// Returns `None` if Matrix is not configured or Conduit is unreachable.
    /// The transport is wrapped in a Mutex because `login`/`reconnect` take `&mut self`.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be fully built
    /// post: returns Some(&Arc<`Mutex<MatrixTransport>`>) if connected; None otherwise
    pub fn matrix_transport(
        &self,
    ) -> Option<&Arc<tokio::sync::Mutex<hkask_communication::matrix::MatrixTransport>>> {
        self.matrix_transport.as_ref()
    }

    /// Await CuratorPod activation. Consumes the oneshot — call once.
    /// Returns `Ok(())` when the CuratorPod is ready, or `Err` if
    /// curator initialization failed or timed out.
    pub async fn curator_ready(&mut self) -> Result<(), String> {
        let rx = self
            .curator_ready
            .take()
            .ok_or_else(|| "curator_ready already consumed".to_string())?;
        rx.await
            .map_err(|_| "CuratorPod failed to activate — check startup logs".to_string())
    }

    /// Build per-agent memory infrastructure from an agent-scoped Database.
    ///
    /// Constructs storage ports (`EpisodicStoragePort`, `SemanticStoragePort`)
    /// and a `ConsolidationService` — all sharing the same underlying DB
    /// connection so consolidation operates on the agent's actual triples.
    ///
    /// This is used by the REPL to build agent-scoped memory (separate from
    /// the shared `AgentService` memory adapted for loops).
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  db must be a valid opened Database
    /// post: returns PerAgentMemory with episodic_storage, semantic_storage, and consolidation_service all sharing the same DB
    pub fn build_per_agent_memory(
        db: Database,
        cns_event_sink: Option<Arc<dyn NuEventSink>>,
    ) -> PerAgentMemory {
        let conn = db.conn_arc();

        // EpisodicMemory + SemanticMemory for ConsolidationService
        let ts1 = TripleStore::new(Arc::clone(&conn));
        let mut episodic_memory = EpisodicMemory::new(ts1);
        if let Some(ref sink) = cns_event_sink {
            episodic_memory = episodic_memory.with_cns(Arc::clone(sink));
        }
        let episodic_memory = Arc::new(episodic_memory);
        let ts2 = TripleStore::new(Arc::clone(&conn));
        let emb = EmbeddingStore::new(Arc::clone(&conn));
        let mut semantic_memory = SemanticMemory::new(ts2, emb);
        if let Some(ref sink) = cns_event_sink {
            semantic_memory = semantic_memory.with_cns(Arc::clone(sink));
        }
        let semantic_memory = Arc::new(semantic_memory);

        // ConsolidationService from the shared memories
        let bridge = Arc::new(ConsolidationBridge::new(
            Arc::clone(&episodic_memory),
            Arc::clone(&semantic_memory),
        ));
        let curator_id = *CuratorHandle::system().curator_id();
        let token = hkask_capability::ConsolidationToken::new(curator_id);
        let consolidation_service =
            hkask_memory::ConsolidationService::new(bridge, semantic_memory, token);

        // Storage ports via MemoryLoopForwarder — uses the same connection
        let mut adapter_epi = EpisodicMemory::new(TripleStore::new(Arc::clone(&conn)));
        let mut adapter_sem = SemanticMemory::new(
            TripleStore::new(Arc::clone(&conn)),
            EmbeddingStore::new(Arc::clone(&conn)),
        );
        if let Some(ref sink) = cns_event_sink {
            adapter_epi = adapter_epi.with_cns(Arc::clone(sink));
            adapter_sem = adapter_sem.with_cns(Arc::clone(sink));
        }
        let adapter = Arc::new(hkask_agents::adapters::MemoryLoopForwarder::new(
            adapter_epi,
            adapter_sem,
        ));

        PerAgentMemory {
            episodic_storage: adapter.clone() as Arc<dyn EpisodicStoragePort>,
            semantic_storage: adapter as Arc<dyn SemanticStoragePort>,
            consolidation_service,
        }
    }
}

impl AgentService {
    /// Assemble all shared infrastructure from a `ServiceConfig`.
    ///
    /// This is the canonical construction path that replaces the four
    /// independent assemblies currently in the codebase. It resolves
    /// secrets, opens databases, constructs CNS/loop system, governed
    /// tool membrane, and session manager in the correct dependency order.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config must be a valid ServiceConfig with resolved secrets
    /// post: returns fully assembled AgentService with all infrastructure wired; Err on any construction step failure
    /// # Dependency order
    ///
    /// 1. Database connections (primary + per-purpose)
    /// 2. Stores (consent, escalation, goals, standing sessions)
    /// 3. CNS runtime + event sink
    /// 4. Loop system + cybernetics loop
    /// 5. GovernedTool membrane + MCP dispatcher
    /// 6. A2A runtime + pod manager
    /// 7. Inference port (optional, based on config)
    /// 8. Memory adapters (episodic + semantic)
    pub async fn build(config: ServiceConfig) -> Result<Self, ServiceError> {
        let system_webid = WebID::from_persona(config.agent_name.as_bytes());

        // ── Foundation: database, stores, CNS, seam watcher ──────────────
        let mut foundation = build_foundation(&config).await?;

        // ── Loops: cybernetics, inference, episodic, semantic, curation ──
        let loops = build_loops(&config, &mut foundation, system_webid).await?;

        // ── MCP + pods: governed tool, dispatcher, pod manager, daemon ───
        let mcp_pods = build_mcp_and_pods(&config, &loops, &foundation, system_webid).await?;

        // ── Matrix transport + 7R7 listener ──────────────────────────────
        let matrix_transport = build_matrix().await;

        // ── Registry + wallet: agent records, A2A restore, rJoule ───────
        let reg_wallet = build_registry_and_wallet(&config, &foundation, &loops, &mcp_pods).await?;

        Ok(Self {
            registry: reg_wallet.registry,
            mcp_runtime: mcp_pods.mcp_runtime,
            mcp_dispatcher: mcp_pods.mcp_dispatcher,
            cns_runtime: foundation.cns_runtime,
            cybernetics_loop: loops.cybernetics_loop,
            loop_system: loops.loop_system,
            inference_port: loops.inference_port,
            episodic_storage: loops.episodic_storage,
            semantic_storage: loops.semantic_storage,
            escalation_queue: foundation.escalation_queue,
            consent_manager: foundation.consent_manager,
            goal_repo: foundation.goal_repo,
            curation_inbox_tx: Some(foundation.curation_inbox_tx.clone()),
            pod_manager: mcp_pods.pod_manager,
            capability_checker: mcp_pods.capability_checker,
            system_webid,
            event_sink: foundation.cns_event_sink,
            energy_estimator: mcp_pods.energy_estimator,
            sovereignty_boundary_store: foundation.sovereignty_boundary_store,
            spec_store: foundation.spec_store,
            a2a_runtime: loops.a2a_runtime,
            agent_registry_store: reg_wallet.agent_registry_store,
            user_store: foundation.user_store,
            daemon_handler: mcp_pods.daemon_handler,
            matrix_transport,
            curator_ready: Some(mcp_pods.curator_ready),
            seam_watcher: foundation.seam_watcher,
            config,
            wallet_service: reg_wallet.wallet_service,
            wallet_store: reg_wallet.wallet_store,
            wallet_gas_calibrator: reg_wallet.wallet_gas_calibrator,
            federation_link_manager: loops.federation_link_manager,
        })
    }
}

// ── Build helpers ──────────────────────────────────────────────────────────
// Extracted from build() for readability. Each helper constructs one
// subsystem and returns an intermediate struct consumed by the next step.

/// Foundation: database connections, stores, CNS runtime, seam watcher.
struct Foundation {
    db: Database,
    primary_conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
    curation_inbox_tx: tokio::sync::mpsc::UnboundedSender<CurationInput>,
    curation_inbox_rx: Option<tokio::sync::mpsc::UnboundedReceiver<CurationInput>>,
    consent_manager: Arc<ConsentManager>,
    escalation_queue: Arc<EscalationQueue>,
    goal_repo: Arc<SqliteGoalRepository>,
    sovereignty_boundary_store: SovereigntyBoundaryStore,
    spec_store: SqliteSpecStore,
    user_store: Arc<std::sync::Mutex<UserStore>>,
    cns_runtime: Arc<RwLock<CnsRuntime>>,
    seam_watcher: Arc<RwLock<Option<SeamWatcher>>>,
    cns_event_sink: Arc<dyn NuEventSink>,
    /// Concrete event store used for gas report queries and calibration.
    gas_event_store: Arc<NuEventStore>,
}

async fn build_foundation(config: &ServiceConfig) -> Result<Foundation, ServiceError> {
    let db = if config.in_memory {
        in_memory_db()
    } else {
        Database::open(&config.db_path, &config.db_passphrase).map_err(|e| {
            ServiceError::Storage {
                message: e.to_string(),
            }
        })?
    };
    let shared_conn = db.conn_arc();

    let primary_conn = Arc::clone(&shared_conn);
    let consent_conn = Arc::clone(&shared_conn);
    let escalation_conn = Arc::clone(&shared_conn);
    let goal_conn = Arc::clone(&shared_conn);
    let sovereignty_conn = Arc::clone(&shared_conn);
    let spec_conn = Arc::clone(&shared_conn);
    let user_conn = Arc::clone(&shared_conn);

    // Shared channel for CurationInput.
    let (curation_inbox_tx, curation_inbox_rx) =
        tokio::sync::mpsc::unbounded_channel::<CurationInput>();

    let consent_store = ConsentStore::new(consent_conn);
    consent_store
        .initialize_schema()
        .map_err(|e| ServiceError::ConsentStore {
            message: e.to_string(),
        })?;
    let consent_manager = Arc::new(ConsentManager::new(consent_store));

    let escalation_queue =
        Arc::new(
            EscalationQueue::new(escalation_conn).map_err(|e| ServiceError::Escalation {
                message: e.to_string(),
            })?,
        );

    let goal_sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&goal_conn)));
    let goal_repo = Arc::new(SqliteGoalRepository::new(goal_conn).with_telemetry(goal_sink));

    let sovereignty_boundary_store = SovereigntyBoundaryStore::new(sovereignty_conn);
    sovereignty_boundary_store
        .initialize_schema()
        .map_err(|e| ServiceError::SovereigntyStore {
            message: e.to_string(),
        })?;

    let spec_store = SqliteSpecStore::new(spec_conn);
    spec_store.init_schema().map_err(|e| ServiceError::Spec {
        message: e.to_string(),
    })?;

    let user_store = Arc::new(std::sync::Mutex::new(UserStore::new(user_conn)));
    {
        let guard = user_store.lock().map_err(|_| ServiceError::UserStore {
            message: hkask_types::InfrastructureError::LockPoisoned.to_string(),
        })?;
        guard
            .initialize_schema()
            .map_err(|e| ServiceError::UserStore {
                message: e.to_string(),
            })?;
    }

    // CNS runtime
    let cns_runtime = Arc::new(RwLock::new(CnsRuntime::with_threshold(
        config.cns_threshold,
    )));

    // Seam watcher (R7.3) — non-fatal if inventory unavailable.
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

    // CNS event sink + gas event store — share one NuEventStore on primary DB.
    let gas_event_store: Arc<NuEventStore> = Arc::new(NuEventStore::new(Arc::clone(&primary_conn)));
    let cns_event_sink: Arc<dyn NuEventSink> = Arc::clone(&gas_event_store) as Arc<dyn NuEventSink>;

    // Triple store for kanban task bridge — contract violations create tasks here.
    let _triple_store = Arc::new(TripleStore::new(Arc::clone(&primary_conn)));

    // Spawn periodic seam drift check (R7.3 background watcher).
    spawn_seam_drift_check(&seam_watcher, &cns_runtime, &cns_event_sink);

    // Spawn periodic contract test monitor — runs cargo test on priority crates
    // and emits cns.contract.violated spans on REQ-tagged failures.
    let _workspace_root = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    Ok(Foundation {
        db,
        primary_conn,
        curation_inbox_tx,
        curation_inbox_rx: Some(curation_inbox_rx),
        consent_manager,
        escalation_queue,
        goal_repo,
        sovereignty_boundary_store,
        spec_store,
        user_store,
        cns_runtime,
        seam_watcher,
        cns_event_sink,
        gas_event_store,
    })
}

fn spawn_seam_drift_check(
    seam_watcher: &Arc<RwLock<Option<SeamWatcher>>>,
    cns_runtime: &Arc<RwLock<CnsRuntime>>,
    event_sink: &Arc<dyn NuEventSink>,
) {
    self::seam_monitor::spawn_seam_drift_check(seam_watcher, cns_runtime, event_sink)
}

/// Loops: cybernetics, inference, episodic, semantic, curation, snapshot, backup.
struct Loops {
    loop_system: Arc<LoopSystem>,
    backup_loop: Arc<hkask_services_backup::BackupLoop>,
    cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,
    inference_port: Option<Arc<dyn InferencePort>>,
    episodic_storage: Arc<dyn EpisodicStoragePort>,
    semantic_storage: Arc<dyn SemanticStoragePort>,
    tool_consumption_tx: tokio::sync::mpsc::UnboundedSender<ToolConsumptionEvent>,
    a2a_runtime: Arc<hkask_agents::A2ARuntime>,
    /// Federation link manager — set when federation is enabled.
    federation_link_manager: Option<Arc<dyn FederationDispatch>>,
}

async fn build_loops(
    config: &ServiceConfig,
    f: &mut Foundation,
    system_webid: WebID,
) -> Result<Loops, ServiceError> {
    let loop_system = Arc::new(LoopSystem::new());

    let (tool_consumption_tx, tool_consumption_rx) =
        tokio::sync::mpsc::unbounded_channel::<ToolConsumptionEvent>();
    let (curator_directive_tx, curator_directive_rx) =
        tokio::sync::mpsc::unbounded_channel::<CuratorDirective>();

    // Cybernetics loop
    let set_points = load_set_points();
    let cybernetics_loop = CyberneticsLoop::with_set_points(Arc::clone(&f.cns_runtime), set_points)
        .with_event_sink(Arc::clone(&f.cns_event_sink))
        .with_alerts_channel(f.curation_inbox_tx.clone())
        .with_tool_consumption_channel(tool_consumption_rx)
        .with_curator_directive_channel(curator_directive_rx);
    let cybernetics_loop = Arc::new(RwLock::new(cybernetics_loop));
    loop_system
        .register_loop(Arc::new(CyberneticsLoopHandle(Arc::clone(
            &cybernetics_loop,
        ))))
        .await;

    // Inference loop (optional)
    let inference_port: Option<Arc<dyn InferencePort>> = if config.in_memory {
        None
    } else {
        let router = hkask_inference::InferenceRouter::new(config.inference_config.clone());
        let raw_port: Arc<dyn InferencePort> = Arc::new(router);
        let governed_port: Arc<dyn InferencePort> = Arc::new(hkask_cns::GovernedInference::new(
            raw_port,
            Arc::clone(&cybernetics_loop),
            Arc::clone(&f.cns_event_sink),
            system_webid,
        ));
        let inference_loop = hkask_agents::InferenceLoop::new()
            .with_energy_budget(config.energy_budget_cap, config.gas_replenish_rate)
            .with_model(&config.default_model);
        loop_system.register_loop(Arc::new(inference_loop)).await;
        Some(governed_port)
    };

    // Episodic + Semantic memory
    let mem_conn = if config.in_memory {
        Arc::clone(&f.db.conn_arc())
    } else {
        let path = config
            .effective_memory_db_path()
            .expect("effective_memory_db_path returns Some when !in_memory");
        let passphrase = config
            .memory_passphrase
            .as_deref()
            .unwrap_or(&config.db_passphrase);
        Database::open(&path, passphrase)
            .map_err(|e| ServiceError::Storage {
                message: e.to_string(),
            })?
            .conn_arc()
    };
    let triple_store = TripleStore::new(Arc::clone(&mem_conn));
    let half_life_secs = config.decay_half_life_months * 30.0 * 24.0 * 3600.0;
    let episodic_memory = Arc::new(
        EpisodicMemory::new(triple_store)
            .with_decay_half_life_secs(half_life_secs)
            .with_cns(Arc::clone(&f.cns_event_sink)),
    );
    let storage_budget = episodic_memory.storage_budget();
    let episodic_loop =
        EpisodicLoop::new(Arc::clone(&episodic_memory), system_webid, storage_budget);
    loop_system.register_loop(Arc::new(episodic_loop)).await;

    let triple_store2 = TripleStore::new(Arc::clone(&mem_conn));
    let embedding_store = EmbeddingStore::new(Arc::clone(&mem_conn));
    let semantic_memory = Arc::new(
        SemanticMemory::new(triple_store2, embedding_store).with_cns(Arc::clone(&f.cns_event_sink)),
    );
    let semantic_loop = SemanticLoop::new(Arc::clone(&semantic_memory));
    loop_system.register_loop(Arc::new(semantic_loop)).await;

    // Memory adapter — with CNS observability on its own store instances
    let memory_adapter = Arc::new(
        hkask_agents::adapters::memory_loop_adapter::MemoryLoopForwarder::new(
            EpisodicMemory::new(TripleStore::new(Arc::clone(&mem_conn)))
                .with_cns(Arc::clone(&f.cns_event_sink)),
            SemanticMemory::new(
                TripleStore::new(Arc::clone(&mem_conn)),
                EmbeddingStore::new(Arc::clone(&mem_conn)),
            )
            .with_cns(Arc::clone(&f.cns_event_sink)),
        ),
    );
    let episodic_storage: Arc<dyn EpisodicStoragePort> = memory_adapter.clone();
    let semantic_storage: Arc<dyn SemanticStoragePort> = memory_adapter.clone();

    // Curation loop
    let cns_for_curator: Arc<CnsRuntime> = Arc::new(f.cns_runtime.read().await.clone());
    let a2a_runtime = Arc::new(hkask_agents::A2ARuntime::new(&config.a2a_secret));
    let a2a_port: Arc<dyn hkask_agents::ports::A2APort> = a2a_runtime.clone();
    let curator_context = Arc::new(
        CuratorContext::new(
            CuratorHandle::system(),
            cns_for_curator,
            Some(curator_directive_tx.clone()),
            Arc::clone(&f.escalation_queue),
        )
        .with_a2a(a2a_port),
    );
    let consolidation_bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let curator_agent = CuratorAgent::with_consolidation(
        curator_context,
        Default::default(),
        Arc::clone(&consolidation_bridge),
        Some(
            f.curation_inbox_rx
                .take()
                .expect("curation_inbox_rx consumed once"),
        ),
        Some(f.curation_inbox_tx.clone()),
    );
    let curation_loop: Arc<dyn HkaskLoop> = curator_agent.curation_loop().clone();
    loop_system.register_loop(curation_loop).await;

    // ── Federation (opt-in via HKASK_FEDERATION_ENABLED=1) ─────────────
    let federation_link_manager: Option<Arc<dyn FederationDispatch>> = if std::env::var(
        "HKASK_FEDERATION_ENABLED",
    )
    .as_deref()
        == Ok("1")
    {
        let local_replica = system_webid.to_string();
        let transport: Arc<dyn hkask_ports::federation::FederationTransport> =
            Arc::new(InMemoryFederationTransport::for_replica(
                &InMemoryFederationTransport::new_shared(),
                local_replica.clone(),
            ));
        let link_manager = Arc::new(FederationLinkManager::new(
            local_replica.clone(),
            Arc::clone(&transport),
            Arc::clone(&f.cns_event_sink),
        ));
        let dispatch: Arc<dyn FederationDispatch> = link_manager.clone();
        // Build FederationSync with SemanticIndexSyncPort
        let triple_store = TripleStore::new(Arc::clone(&mem_conn));
        let semantic_index = Arc::new(std::sync::Mutex::new(SemanticIndex::new(triple_store)));
        let sync_port: Arc<dyn FederationSyncPort> =
            Arc::new(SemanticIndexSyncPort::new(Arc::clone(&semantic_index)));
        let fed_sync = Arc::new(FederationSync::new(
            local_replica.clone(),
            Arc::clone(&transport),
            sync_port,
            link_manager,
            Arc::clone(&f.cns_event_sink),
        ));
        // Spawn background sync loop
        let (fed_cancel_tx, fed_cancel_rx) = tokio::sync::watch::channel(false);
        let fed_sync_clone: Arc<FederationSync> = Arc::clone(&fed_sync);
        tokio::spawn(async move { fed_sync_clone.run(fed_cancel_rx).await });
        drop(fed_cancel_tx);
        tracing::info!(target: "cns.federation.sync", replica = %local_replica, "Federation sync loop started");
        Some(dispatch)
    } else {
        None
    };

    // ── Federation end ─────────────────────────────────────────────────

    // Wire federation dispatcher into CuratorAgent if present.
    // The CLI path uses federation_dispatch() directly; this enables
    // the internal directive dispatch path (CurationLoop → CuratorAgent).
    if let Some(ref lm) = federation_link_manager {
        let _ = curator_agent.with_federation(Arc::clone(lm));
    }

    // Snapshot + Backup loops
    let git_cas_port: Arc<dyn GitCASPort> = match hkask_mcp::GixCasAdapter::from_env() {
        Ok(adapter) => Arc::new(adapter),
        Err(e) => {
            tracing::warn!(target: "hkask.services", error = %e, "Git CAS port from env failed — using fallback");
            Arc::new(
                hkask_mcp::GixCasAdapter::new(PathBuf::from("/tmp/hkask-templates")).map_err(
                    |e| ServiceError::Infra(hkask_types::InfrastructureError::Io(e.to_string())),
                )?,
            )
        }
    };
    let snapshot_loop = SnapshotLoop::new(Arc::clone(&git_cas_port));
    loop_system.register_loop(Arc::new(snapshot_loop)).await;
    let backup_service = Arc::new(hkask_services_backup::BackupService::new(
        Arc::clone(&git_cas_port),
        hkask_services_backup::load_backup_config(),
    ));
    let backup_loop = Arc::new(hkask_services_backup::BackupLoop::new(backup_service));
    loop_system
        .register_loop(backup_loop.clone() as Arc<dyn HkaskLoop>)
        .await;

    Ok(Loops {
        loop_system,
        backup_loop,
        cybernetics_loop,
        inference_port,
        episodic_storage,
        semantic_storage,
        tool_consumption_tx,
        a2a_runtime,
        federation_link_manager,
    })
}

/// MCP + pods: governed tool, dispatcher, pod manager, daemon handler.
struct McpPods {
    mcp_runtime: Arc<McpRuntime>,
    mcp_dispatcher: Arc<McpDispatcher>,
    pod_manager: Arc<ActivePods>,
    capability_checker: Arc<CapabilityChecker>,
    daemon_handler: Arc<hkask_services_daemon::ServiceDaemonHandler>,
    energy_estimator: Arc<hkask_cns::CalibratedEnergyEstimator>,
    /// Keeps the CuratorSync cancellation channel alive.
    #[allow(dead_code)]
    _curator_cancel: tokio::sync::watch::Sender<bool>,
    /// Signals when the CuratorPod has been activated (or failed).
    curator_ready: tokio::sync::oneshot::Receiver<()>,
}

async fn build_mcp_and_pods(
    config: &ServiceConfig,
    l: &Loops,
    f: &Foundation,
    system_webid: WebID,
) -> Result<McpPods, ServiceError> {
    // GovernedTool membrane
    let mcp_runtime = McpRuntime::new();
    let raw_tool_port = Arc::new(RawMcpToolPort::new(mcp_runtime.clone()));
    let energy_estimator: Arc<CalibratedEnergyEstimator> = Arc::new(
        CalibratedEnergyEstimator::new(Arc::clone(&f.gas_event_store))
            .with_event_sink(Arc::clone(&f.cns_event_sink)),
    );
    energy_estimator
        .clone()
        .spawn_calibration(hkask_cns::DEFAULT_CALIBRATION_INTERVAL);
    let estimator: Arc<dyn EnergyEstimator> =
        Arc::clone(&energy_estimator) as Arc<dyn EnergyEstimator>;
    let governed_tool = Arc::new(
        GovernedTool::new(
            raw_tool_port,
            Arc::clone(&l.cybernetics_loop),
            Arc::clone(&f.cns_event_sink),
            estimator,
            system_webid,
        )
        .with_tool_consumption_channel(l.tool_consumption_tx.clone()),
    );
    let mcp_dispatcher = Arc::new(McpDispatcher::with_governed_tool(
        mcp_runtime.clone(),
        governed_tool.clone(),
    ));
    let mcp_runtime = Arc::new(mcp_runtime);

    // Pod manager — anchor the capability checker to the system OCAP authority
    // (derived from the master key) so locally-issued tokens verify and forged
    // tokens are rejected. Fails the build if the OCAP key is unavailable.
    let capability_checker = Arc::new(
        hkask_agents::pod::system_capability_checker().map_err(|e| {
            ServiceError::Infra(hkask_types::InfrastructureError::Io(format!(
                "OCAP authority key unavailable: {e}"
            )))
        })?,
    );
    let mcp_runtime_adapter = hkask_agents::adapters::mcp_runtime::FullMcpAdapter::new(
        Arc::clone(&capability_checker),
        Arc::new((*mcp_runtime).clone()),
        tokio::runtime::Handle::current(),
    );
    let mut pods = hkask_agents::pod::ActivePods::new()
        .with_a2a_runtime(
            l.a2a_runtime.clone() as Arc<dyn hkask_agents::ports::A2APort + Send + Sync>
        )
        .with_factory_and_ports(
            Arc::new(hkask_agents::pod::PodFactory::new(
                Arc::new(hkask_templates::TemplateCrateLoader::from_path(
                    std::path::PathBuf::from(&config.template_cache_path),
                )),
                Arc::new(hkask_agents::DenyAllConsent),
                // Pod storage lives alongside the main DB, not inside it.
                std::path::Path::new(&config.db_path)
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .to_path_buf(),
            )),
            Arc::new(mcp_runtime_adapter),
            Some(governed_tool.clone()),
            Some(Arc::clone(&capability_checker)),
            None,
            Arc::clone(&l.episodic_storage) as Arc<dyn EpisodicStoragePort>,
            Arc::clone(&l.semantic_storage) as Arc<dyn SemanticStoragePort>,
        );
    if let Some(inf) = l.inference_port.clone() {
        pods = pods.with_inference_port(inf);
    }
    pods = pods.with_matrix_homeserver(
        std::env::var("HKASK_MATRIX_URL").unwrap_or_else(|_| "http://localhost:8008".to_string()),
    );
    let pod_manager: Arc<hkask_agents::pod::ActivePods> = Arc::new(pods);

    // Register pod state producer — snapshots each pod's db before daily backup.
    l.backup_loop.add_producer(Arc::new(PodBackupProducer {
        pod_manager: Arc::clone(&pod_manager),
    }));
    l.backup_loop.add_producer(Arc::new(ConfigProducer));

    // Start CuratorPod + CuratorSync (semantic aggregation loop).
    // Runs as a background task for the lifetime of the service.
    // A oneshot signals readiness so callers can await curator activation.
    let (curator_cancel_tx, curator_cancel_rx) = tokio::sync::watch::channel(false);
    let (curator_ready_tx, curator_ready_rx) = tokio::sync::oneshot::channel();
    let curator_pm = Arc::clone(&pod_manager);
    let curator_data_dir = std::path::Path::new(&config.db_path)
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    tokio::spawn(async move {
        match curator_pm
            .ensure_curator(curator_data_dir, curator_cancel_rx)
            .await
        {
            Ok(Some(_)) => {
                tracing::info!(target: "hkask.startup", "CuratorPod activated and CuratorSync running");
                let _ = curator_ready_tx.send(());
            }
            Ok(None) => {
                tracing::info!(target: "hkask.startup", "CuratorPod already active");
                let _ = curator_ready_tx.send(());
            }
            Err(e) => {
                tracing::error!(target: "hkask.startup", error = %e, "Failed to start CuratorPod");
                // Don't send on oneshot — callers awaiting curator_ready
                // will observe the sender dropped (recv_err).
            }
        }
    });
    // cancel_tx stored in McpPods for lifecycle

    // Matrix auto-registration — deferred to per-pod activation

    // Daemon handler + listener (skip socket in test mode)
    let daemon_handler = Arc::new(hkask_services_daemon::ServiceDaemonHandler::new(
        Arc::clone(&pod_manager),
        Arc::clone(&f.user_store),
        Some(Arc::clone(&f.cns_runtime)),
        Some(Arc::new(f.spec_store.clone())),
        l.inference_port.clone(),
    ));
    if !config.in_memory {
        let mut daemon_listener = hkask_mcp::daemon::DaemonListener::new();
        daemon_listener.bind().await.map_err(|e| {
            ServiceError::Infra(hkask_types::InfrastructureError::Io(format!(
                "Failed to bind daemon socket: {}",
                e
            )))
        })?;
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

    Ok(McpPods {
        mcp_runtime,
        mcp_dispatcher,
        pod_manager,
        capability_checker,
        daemon_handler,
        energy_estimator,
        _curator_cancel: curator_cancel_tx,
        curator_ready: curator_ready_rx,
    })
}

// ── Artifact producers (backup integration) ────────────────────────────

use async_trait::async_trait;
use hkask_services_backup::BackupError;
use hkask_services_backup::producers::ArtifactProducer;
use hkask_services_backup::scope::ArtifactType;

/// Produces PodState artifacts by querying ActivePods for all activated pods.
struct PodBackupProducer {
    pod_manager: Arc<hkask_agents::pod::ActivePods>,
}

#[async_trait]
impl ArtifactProducer for PodBackupProducer {
    fn artifact_types(&self) -> &[ArtifactType] {
        &[ArtifactType::PodState]
    }

    async fn produce(
        &self,
        cas: &dyn hkask_ports::git_cas::GitCASPort,
    ) -> Result<usize, BackupError> {
        let pods = self.pod_manager.pod_db_paths().await;
        let repo_id = ArtifactType::PodState.repo_id();
        let mut count = 0usize;

        for (pod_id, db_path) in pods {
            let pod_data = match std::fs::read(&db_path) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(
                        target: "cns.backup", pod_id = %pod_id, error = %e,
                        "Failed to read pod.db — skipping"
                    );
                    continue;
                }
            };

            let artifact = hkask_services_backup::serialization::serialize_artifact(
                &ArtifactType::PodState,
                &pod_id,
                &serde_json::json!({"pod_id": &pod_id}),
            )
            .map_err(|e| BackupError::Serialization(format!("PodState {pod_id}: {e}")))?;

            cas.put_blob(&repo_id, &artifact).await?;
            cas.put_blob(&repo_id, &pod_data).await?;
            count += 1;
        }

        Ok(count)
    }
}

/// Produces backup configuration as a Settings artifact — self-backup.
struct ConfigProducer;

#[async_trait]
impl ArtifactProducer for ConfigProducer {
    fn artifact_types(&self) -> &[ArtifactType] {
        &[ArtifactType::Settings]
    }

    async fn produce(
        &self,
        cas: &dyn hkask_ports::git_cas::GitCASPort,
    ) -> Result<usize, BackupError> {
        let config_path = hkask_services_backup::config::backup_config_path();
        if !config_path.exists() {
            return Ok(0);
        }
        let data = std::fs::read_to_string(&config_path)
            .map_err(|e| BackupError::Config(format!("Failed to read backup config: {e}")))?;
        let artifact = hkask_services_backup::serialization::serialize_artifact(
            &ArtifactType::Settings,
            "backup-config",
            &serde_json::json!({"path": config_path.to_string_lossy(), "content": data}),
        )
        .map_err(|e| BackupError::Serialization(format!("Config: {e}")))?;
        cas.put_blob(&ArtifactType::Settings.repo_id(), &artifact)
            .await?;
        Ok(1)
    }
}

/// Matrix transport + 7R7 listener. Non-blocking: returns None if Conduit unreachable.
async fn build_matrix()
-> Option<Arc<tokio::sync::Mutex<hkask_communication::matrix::MatrixTransport>>> {
    self::matrix::build_matrix().await
}

/// Registry + wallet: agent records, A2A restore, rJoule payments.
struct RegWallet {
    registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,
    agent_registry_store: hkask_storage::AgentRegistryStore,
    wallet_service: Option<Arc<WalletService>>,
    wallet_store: Option<Arc<WalletStore>>,
    wallet_gas_calibrator: Option<Arc<hkask_cns::WalletGasCalibrator>>,
}

async fn build_registry_and_wallet(
    config: &ServiceConfig,
    f: &Foundation,
    l: &Loops,
    _mcp: &McpPods,
) -> Result<RegWallet, ServiceError> {
    // Registry
    let registry = Arc::new(tokio::sync::Mutex::new(
        SqliteRegistry::new_with_conn(f.primary_conn.clone()).map_err(|e| {
            ServiceError::Template {
                message: e.to_string(),
            }
        })?,
    ));

    // Agent registry store
    let agent_registry_store = hkask_storage::AgentRegistryStore::new(f.primary_conn.clone());
    agent_registry_store
        .initialize_schema()
        .map_err(|e| ServiceError::AgentRegistryStore {
            message: e.to_string(),
        })?;

    // Restore A2A state from persistent storage
    let registered_agents =
        agent_registry_store
            .list()
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })?;
    if !registered_agents.is_empty() {
        use std::str::FromStr;
        let agents: Vec<hkask_agents::a2a::A2AAgent> = registered_agents
            .iter()
            .map(|ra| hkask_agents::a2a::A2AAgent {
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
        // A2A restore is async — await directly since build() is async.
        l.a2a_runtime
            .restore_from_storage(agents, tokens)
            .await
            .map_err(|e| ServiceError::A2A {
                message: e.to_string(),
            })?;
    }

    // Wallet — non-fatal if config or build fails (daemon can run without wallet)
    let (wallet_service, wallet_store, wallet_gas_calibrator) = match build_wallet(config, f, l) {
        Ok(tuple) => tuple,
        Err(e) => {
            tracing::warn!(target: "cns.wallet", error = %e, "Wallet unavailable — running without rJoule");
            (None, None, None)
        }
    };

    Ok(RegWallet {
        registry,
        agent_registry_store,
        wallet_service,
        wallet_store,
        wallet_gas_calibrator,
    })
}

/// Build wallet subsystem — returns (service, store, gas_calibrator) or error.
#[allow(clippy::type_complexity)]
fn build_wallet(
    config: &ServiceConfig,
    f: &Foundation,
    l: &Loops,
) -> Result<
    (
        Option<Arc<WalletService>>,
        Option<Arc<WalletStore>>,
        Option<Arc<hkask_cns::WalletGasCalibrator>>,
    ),
    ServiceError,
> {
    // Per-agent wallet: each agent gets their own wallet database at
    // agents/{name}/wallet.db, encrypted with the same passphrase as pod.db.
    // This gives each replicant sovereign control over their rJoule balances,
    // API keys, and encumbrances — no shared wallet state across agents.
    let wallet_db_path = if config.in_memory {
        None
    } else {
        let path = hkask_types::agent_paths::agent_wallet_db(&config.agent_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        Some(path)
    };

    let wallet_conn = if let Some(ref path) = wallet_db_path {
        let path_str = path.to_string_lossy().to_string();
        match hkask_storage::Database::open(&path_str, &config.db_passphrase) {
            Ok(db) => {
                tracing::info!(
                    target: "cns.wallet",
                    path = %path_str,
                    agent = %config.agent_name,
                    "Per-agent wallet database opened"
                );
                db.conn_arc()
            }
            Err(e) => {
                tracing::warn!(
                    target: "cns.wallet",
                    path = %path_str,
                    error = %e,
                    "Failed to open per-agent wallet DB, falling back to shared connection"
                );
                Arc::clone(&f.db.conn_arc())
            }
        }
    } else {
        Arc::clone(&f.db.conn_arc())
    };
    let wallet_store = Arc::new(WalletStore::new(wallet_conn));
    let svc = WalletService::build(
        &config.wallet_config,
        Arc::clone(&wallet_store),
        Arc::clone(&f.cns_event_sink),
        Arc::clone(&l.cybernetics_loop),
    )?;
    let svc = Arc::new(
        svc.as_ref()
            .clone()
            .with_consent_manager(Arc::clone(&f.consent_manager)),
    );
    let wallet_manager = svc.manager();

    // Ensure default wallet
    let default_wallet = WalletId::default();
    wallet_manager
        .ensure_wallet(default_wallet)
        .map_err(|e| ServiceError::Wallet {
            source: Some(Box::new(e)),
            message: "Failed to ensure default wallet".into(),
        })?;

    // Bind wallets to replicants
    {
        let user_guard = f.user_store.lock().map_err(|_| ServiceError::UserStore {
            message: hkask_types::InfrastructureError::LockPoisoned.to_string(),
        })?;
        if let Ok(Some(system_identity)) = user_guard.get_replicant(&config.agent_name) {
            let user_id = system_identity.user_id;
            let replicants =
                user_guard
                    .list_replicants(&user_id)
                    .map_err(|e| ServiceError::UserStore {
                        message: e.to_string(),
                    })?;
            for identity in &replicants {
                if identity.wallet_id.is_some() {
                    continue;
                }
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
                if let Err(e) = user_guard.set_wallet_id(&identity.replicant_name, wallet_id) {
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
        }
    }

    // Spawn deposit monitor
    let monitor_manager = Arc::clone(wallet_manager);
    let interval_secs: u64 = std::env::var("HKASK_DEPOSIT_MONITOR_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
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

    // Spawn wallet gas calibrator (P9 feedback loop for gas→rJoule rate).
    let wallet_gas_calibrator = {
        let calibrator = Arc::new(
            hkask_cns::WalletGasCalibrator::new(
                Arc::clone(&f.gas_event_store),
                Arc::clone(wallet_manager),
            )
            .with_event_sink(Arc::clone(&f.cns_event_sink)),
        );
        calibrator
            .clone()
            .spawn_calibration(hkask_cns::DEFAULT_WALLET_CALIBRATION_INTERVAL);
        Some(calibrator)
    };

    Ok((Some(svc), Some(wallet_store), wallet_gas_calibrator))
}
