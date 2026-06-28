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
    SeamSummary, SeamWatcher, load_set_points,
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
use hkask_ports::federation::{FederationDispatch, FederationSyncPort};
use hkask_ports::{ConsolidationOutcome, ConsolidationRequest, InferencePort};
use hkask_storage::EscalationQueue;
use hkask_storage::goals::SqliteGoalRepository;
use hkask_storage::nu_event_store::NuEventStore;
use hkask_storage::user_store::UserStore;
use hkask_storage::{
    ConsentStore, Database, EmbeddingStore, SovereigntyBoundaryStore, SqliteSpecStore, TripleStore,
    WalletStore, in_memory_db,
};
use hkask_templates::SqliteRegistry;
use hkask_types::DataCategory;
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::id::WalletId;

use hkask_services_core::{ServiceConfig, ServiceError};

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
    daemon_handler: Arc<hkask_services_runtime::ServiceDaemonHandler>,

    /// Matrix transport for agent-to-agent and human-to-agent communication.
    /// Owned by the daemon, shared with REPL, pod activation, and MCP wrapper.
    /// Wrapped in Mutex because login/reconnect take &mut self.
    matrix_transport: Option<Arc<tokio::sync::Mutex<hkask_communication::matrix::MatrixTransport>>>,

    /// Signals CuratorPod activation. Consumed by callers that need to
    /// await curator readiness before accepting requests.
    curator_ready: Option<tokio::sync::oneshot::Receiver<()>>,

    /// Public seam watcher — loaded at startup, checked periodically.
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

    /// Wallet gas calibrator — background loop calibrates gas→rJoule exchange rate (P9).
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

impl From<&AgentService> for hkask_services_core::InferenceContext {
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

    /// Access the wallet gas calibrator (background conversion-rate loop).
    ///
    /// \[P9\] Motivating: Homeostatic Self-Regulation — exposes calibration loop handle
    /// pre:  self must be fully built
    /// post: returns Some(&`Arc<WalletGasCalibrator>`) if wallet configured; None otherwise
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

    /// Public seam watcher — None if inventory unavailable at startup.
    /// Returns a read lock on the watcher. For summary data, call
    /// `.read().await` and then `.as_ref().map(|w| w.summary())`.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// Fetch the current seam watcher summary, if available.
    ///
    /// pre:  self must be fully built
    /// post: returns Some(SeamSummary) if watcher is active; None otherwise
    pub async fn seam_summary(&self) -> Option<SeamSummary> {
        let guard = self.seam_watcher.read().await;
        guard.as_ref().map(|w| w.summary())
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
    /// post: returns `Arc<ConsentManager>` wrapping the consent manager
    /// # REQ: P1 (User Sovereignty), P2 (Affirmative Consent)
    /// # expect: "My service operations flow through sovereignty-verifying boundaries"
    pub fn sovereignty(&self) -> Arc<ConsentManager> {
        self.consent_manager.clone()
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
    pub fn daemon_handler(&self) -> &Arc<hkask_services_runtime::ServiceDaemonHandler> {
        &self.daemon_handler
    }

    /// Access the federation dispatch port, if federation is enabled.
    ///
    /// Returns None if federation is not configured or disabled.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence
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
        let consolidation_service =
            hkask_memory::ConsolidationService::new(bridge, Arc::clone(&semantic_memory));

        // Storage ports via MemoryLoopForwarder — reuse configured memories
        let adapter = Arc::new(hkask_agents::adapters::MemoryLoopForwarder::new(
            Arc::clone(&episodic_memory),
            Arc::clone(&semantic_memory),
        ));

        PerAgentMemory {
            episodic_storage: adapter.clone() as Arc<dyn EpisodicStoragePort>,
            semantic_storage: adapter as Arc<dyn SemanticStoragePort>,
            consolidation_service,
        }
    }

    /// Consolidate episodic memory into semantic memory for a specific agent.
    ///
    /// This is the canonical entry point for all user- and Curator-triggered
    /// consolidation operations. It verifies P2 affirmative consent for both
    /// `EpisodicMemory` and `SemanticMemory` before opening the agent's
    /// per-agent memory DB and running the consolidation pipeline.
    ///
    /// \[P2\] Constraining: Affirmative Consent — consolidation is blocked unless
    /// both memory categories are explicitly granted for the target agent's WebID.
    /// \[P4\] Constraining: Clear Boundaries — all consolidation flows through
    /// `AgentService`, preventing direct `Database::open` bypasses.
    ///
    /// pre:  agent_name is non-empty; request is a valid ConsolidationRequest
    /// post: returns ConsolidationOutcome on success; Err(ConsentDenied) if either
    ///       memory category lacks consent; Err(Storage) on DB open failure;
    ///       Err(Consolidation) on pipeline failure
    pub fn consolidate_agent_memory(
        &self,
        agent_name: &str,
        request: ConsolidationRequest,
    ) -> Result<ConsolidationOutcome, ServiceError> {
        let target_webid = WebID::for_agent_name(agent_name);

        // P2: require explicit consent for both sovereign memory categories.
        let categories = [DataCategory::EpisodicMemory, DataCategory::SemanticMemory];
        let missing: Vec<String> = categories
            .iter()
            .filter(|cat| {
                !self
                    .consent_manager
                    .has_consent(&target_webid.to_string(), cat)
                    .unwrap_or(false)
            })
            .map(|cat| cat.to_string())
            .collect();

        if !missing.is_empty() {
            let grant_help = if agent_name == "curator" {
                "Grant consent with: kask sovereignty grant --category <category> --agent curator"
                    .to_string()
            } else {
                "Grant consent with: kask sovereignty grant --category <category>".to_string()
            };
            return Err(ServiceError::ConsentDenied {
                message: format!(
                    "consolidation denied for agent {} — missing consent for: {}. {grant_help}",
                    target_webid.redacted_display(),
                    missing.join(", ")
                ),
            });
        }

        let db_path = hkask_types::agent_paths::agent_memory_db(agent_name);
        let passphrase = self
            .config
            .memory_passphrase
            .as_deref()
            .unwrap_or(&self.config.db_passphrase);

        let db = Database::open(&db_path.to_string_lossy(), passphrase).map_err(|e| {
            ServiceError::Storage {
                message: e.to_string(),
            }
        })?;

        let per_agent_memory = Self::build_per_agent_memory(db, Some(Arc::clone(&self.event_sink)));
        per_agent_memory
            .consolidation_service
            .consolidate(&target_webid, request)
            .map_err(|e| ServiceError::Consolidation {
                source: None,
                message: e,
            })
    }

    /// Query consolidation status for an agent without running consolidation.
    ///
    /// Opens the per-agent memory DB temporarily and returns
    /// (candidates, semantic_count, low_confidence_count).
    /// This is the canonical read path for consolidation status — the REPL,
    /// TUI bridge, and CLI status display all route through this method.
    ///
    /// pre:  agent_name is non-empty
    /// post: returns (candidates, semantic_count, low_confidence) on success;
    ///       Err(Storage) on DB open failure
    pub fn consolidation_status_for(
        &self,
        agent_name: &str,
    ) -> Result<(usize, usize, usize), ServiceError> {
        let target_webid = WebID::for_agent_name(agent_name);

        let db_path = hkask_types::agent_paths::agent_memory_db(agent_name);
        let passphrase = self
            .config
            .memory_passphrase
            .as_deref()
            .unwrap_or(&self.config.db_passphrase);

        let db = Database::open(&db_path.to_string_lossy(), passphrase).map_err(|e| {
            ServiceError::Storage {
                message: e.to_string(),
            }
        })?;

        let per_agent_memory = Self::build_per_agent_memory(db, None);
        let cs = &per_agent_memory.consolidation_service;
        let candidates = cs.consolidation_candidate_count(&target_webid);
        let semantic_count = cs.semantic_triple_count();
        let low_confidence = cs.semantic_low_confidence_count(0.33);

        Ok((candidates, semantic_count, low_confidence))
    }
}

mod build;
