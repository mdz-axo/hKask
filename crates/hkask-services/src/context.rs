//! Shared dependency graph assembled once at startup.
//!
//! `ServiceContext` owns the canonical instances of all shared infrastructure:
//! registry, MCP runtime, CNS, loop system, escalation queue, memory adapters,
//! etc. Both `ReplState` and `ApiState` compose a `ServiceContext` and add
//! only their surface-specific presentation fields.
//!
//! Construction happens via `ServiceContext::build(config)`, which replaces
//! the four independent assembly paths currently in the codebase.

use std::sync::Arc;
use tokio::sync::RwLock;

use hkask_agents::EscalationQueue;
use hkask_agents::LoopSystem;
use hkask_agents::communication::dispatch::MessageDispatch;
use hkask_agents::consent::ConsentManager;
use hkask_agents::pod::PodManager;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::CnsRuntime;
use hkask_cns::CyberneticsLoop;
use hkask_mcp::runtime::McpRuntime;
use hkask_storage::SqliteGoalRepository;
use hkask_templates::SqliteRegistry;
use hkask_types::CapabilityChecker;
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::ports::InferencePort;

use crate::ServiceConfig;
use crate::ServiceError;

/// Shared dependency graph assembled once at startup.
///
/// `ServiceContext` replaces the independent assembly in `ReplState`,
/// `ApiState`, `build_loop_system()`, and `commands/loops.rs`. Surfaces
/// compose this struct and add only presentation-specific fields.
///
/// **Not yet wired** — fields will be added as domains are extracted.
/// The `build()` method will be populated during Task 3 (ServiceContext
/// extraction).
pub struct ServiceContext {
    /// Template registry.
    pub registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,

    /// MCP runtime for tool discovery and invocation.
    pub mcp_runtime: Arc<McpRuntime>,

    /// MCP dispatcher for OCAP-protected tool invocation.
    pub mcp_dispatcher: Arc<hkask_mcp::dispatch::McpDispatcher>,

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
    pub capability_checker: Arc<CapabilityChecker>,

    /// System WebID for signing capabilities.
    pub system_webid: WebID,

    /// Event sink for CNS audit trail.
    pub event_sink: Arc<dyn NuEventSink>,
}

impl ServiceContext {
    /// Assemble all shared infrastructure from a `ServiceConfig`.
    ///
    /// This is the canonical construction path that replaces the four
    /// independent assemblies currently in the codebase. Will be
    /// progressively wired during Task 3.
    pub fn build(_config: ServiceConfig) -> Result<Self, ServiceError> {
        // Task 3 will progressively fill in this method as domains are
        // extracted. The initial stub returns an error to indicate that
        // the full assembly path is not yet implemented.
        Err(ServiceError::Cns(
            "ServiceContext::build() not yet implemented — see Task 3".to_string(),
        ))
    }
}
