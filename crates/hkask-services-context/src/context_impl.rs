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

use crate::cns;
use crate::governance;
use crate::infra;
use crate::storage;

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
    /// Infrastructure context — inference, memory, MCP, pods,
    /// wallet, daemon, matrix, seams, gas calibration, federation.
    infra: infra::InfraContext,

    /// Governance context — OCAP, consent, dispatch, A2A, escalations.
    governance: governance::GovernanceContext,

    /// CNS context — variety sensing, cybernetic regulation,
    /// loop orchestration, event audit trail, energy estimation.
    cns: cns::CnsContext,

    /// Storage context — registry, goals, specs, agents, users,
    /// sovereignty boundaries, wallet store.
    storage: storage::StorageContext,

    /// System WebID for signing capabilities.
    system_webid: WebID,

    /// Signals CuratorPod activation.
    curator_ready: Option<tokio::sync::oneshot::Receiver<()>>,

    /// Configuration used to build this context.
    config: ServiceConfig,
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
            shared_port: ctx.infra().inference.clone(),
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

    // --- Sub-contexts ---

    /// CNS context — variety sensing, cybernetic regulation,
    /// loop orchestration, events, and energy estimation.
    pub fn cns(&self) -> &cns::CnsContext {
        &self.cns
    }

    /// Storage context — registry, goals, specs, agents,
    /// users, sovereignty boundaries, and wallet store.
    pub fn storage(&self) -> &storage::StorageContext {
        &self.storage
    }

    /// Memory — episodic and semantic storage ports.
    pub fn memory(&self) -> (&Arc<dyn EpisodicStoragePort>, &Arc<dyn SemanticStoragePort>) {
        (&self.infra.episodic, &self.infra.semantic)
    }

    /// Public seam watcher — delegated to infra context.
    pub async fn seam_summary(&self) -> Option<SeamSummary> {
        self.infra.seam_summary().await
    }

    // --- Governance ---
    /// Consolidated governance context — OCAP, consent, dispatch,
    /// A2A registration, escalation queue, curation signals.
    pub fn governance(&self) -> &governance::GovernanceContext {
        &self.governance
    }

    /// Infrastructure context — inference, memory, MCP, pods,
    /// wallet, daemon, matrix, seams, gas calibration, federation.
    pub fn infra(&self) -> &infra::InfraContext {
        &self.infra
    }

    /// System WebID + A2A runtime.
    pub fn identity(&self) -> (&WebID, &Arc<hkask_agents::A2ARuntime>) {
        (&self.system_webid, &self.governance.a2a)
    }

    /// Await CuratorPod activation. Consumes the oneshot — call once.
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
                    .governance
                    .consent
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

        let per_agent_memory = Self::build_per_agent_memory(db, Some(Arc::clone(&self.cns.events)));
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
