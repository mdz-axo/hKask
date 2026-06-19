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
use hkask_agents::pod::{ActivePods, PodDeployment, PodFactory};
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::{
    CalibratedEnergyEstimator, CnsRuntime, CyberneticsLoop, EnergyEstimator, GovernedTool,
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

use hkask_services_core::ServiceConfig;
use hkask_services_core::ServiceError;
use hkask_services_sovereignty::SovereigntyService;
use hkask_services_wallet::WalletService;

mod contract_monitor;
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

    /// Pod factory for agent lifecycle.
    pod_factory: Arc<PodFactory>,

    /// Active pod registry for runtime lookup.
    active_pods: Arc<ActivePods>,

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
    pub fn config(&self) -> &ServiceConfig {
        &self.config
    }

    /// Access the wallet service for rJoule payments and API key management.
    ///
    pub fn wallet(&self) -> Option<&Arc<WalletService>> {
        self.wallet_service.as_ref()
    }

    /// Access the wallet store for API key lookup and balance queries.
    ///
    pub fn wallet_store(&self) -> Option<&Arc<WalletStore>> {
        self.wallet_store.as_ref()
    }

    /// Access the wallet gas calibrator.
    ///
