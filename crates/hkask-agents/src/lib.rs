//! hKask Agents — Agent Pod Lifecycle and ACP Integration
//!
//! This crate provides:
//! - **Agent Pod**: Runtime container for ACP agents (bots and replicants)
//! - **Lifecycle Management**: Populated → Registered → Activated → Deactivated
//! - **Capability Tokens**: OCAP-based access control with attenuation
//! - **ACP Runtime**: Agent registration, A2A messaging, capability verification
//! - **Hexagonal Ports**: AcpPort, MCPRuntimePort, CnsEmit, GitCASPort
//!
//! # Example
//!
//! ```rust,no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use hkask_agents::pod::PodManager;
//! use hkask_agents::adapters::git_cas::GitCasAdapter;
//! use hkask_agents::acp::AcpRuntime;
//! use hkask_agents::adapters::cns_emitter::CnsEmitterAdapter;
//! use hkask_agents::adapters::mcp_runtime::McpRuntimeAdapter;
//! use hkask_agents::adapters::memory_storage::MemoryStorageAdapter;
//! use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
//! use std::path::PathBuf;
//! use std::sync::Arc;
//!
//! // Create adapters
//! let git_cas = Arc::new(GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates")));
//! let acp_runtime = Arc::new(AcpRuntime::default());
//! let cns_emitter = Arc::new(CnsEmitterAdapter::new(hkask_types::WebID::new()));
//! let mcp_runtime = Arc::new(McpRuntimeAdapter::new());
//! let memory_adapter = Arc::new(MemoryStorageAdapter::in_memory()?);
//! let episodic_storage: Arc<dyn EpisodicStoragePort> = memory_adapter.clone();
//! let semantic_storage: Arc<dyn SemanticStoragePort> = memory_adapter.clone();
//!
//! // Create pod manager
//! let manager = PodManager::new(git_cas, acp_runtime, cns_emitter, mcp_runtime, episodic_storage, semantic_storage);
//! # Ok(())
//! # }
//! ```

pub mod acp;
pub mod adapters;
pub mod bot;
pub mod capabilities;
pub mod consent;
pub mod curator;
pub mod error;
pub mod ocap;
pub mod pod;
pub mod ports;
pub mod registry_loader;
pub mod replicant;
pub mod revocation_store;
pub mod security;
pub mod sovereignty;

pub use acp::{A2AMessage, AcpAgent, AcpError, AcpRuntime, TemplateDispatchHandler};
pub use adapters::{LoopbackHttpTransport, StdioTransport};
pub use bot::Bot;
pub use capabilities::{AgentCapabilities, MemoryAccess};
pub use consent::{ConsentError, ConsentManager};
pub use curator::escalation::{
    EscalationEntry, EscalationQueue, EscalationStats, EscalationStatus,
};
pub use curator::metacognition::HealthSnapshot;
#[allow(deprecated)]
pub use curator::metacognition::SystemHealthSnapshot;
pub use error::{GitError, McpError, MemoryError, RegistryError};
pub use hkask_types::{BotCapabilities, CapabilityChecker, CapabilityToken, SovereigntyPort};
pub use pod::{
    AgentKind, AgentPersona, AgentPod, AgentPodError, AgentPodResult, PodID, PodLifecycleState,
    PodManager, PodStatus, TemplateCrate, TemplateFile,
};
#[allow(deprecated)]
pub use ports::{
    AcpPort, AcpTransport, AcpWireMessage, AcpWireResponse, EpisodicStoragePort, GitCASPort,
    MCPRuntimePort, MemoryStoragePort, SemanticStoragePort, SovereigntyCheckResult,
    SovereigntyOperation,
};
pub use registry_loader::{BotRegistryLoader, RegistryLoaderError};
pub use replicant::Replicant;
#[allow(deprecated)]
pub use replicant::ReplicantCapabilities;
pub use sovereignty::SovereigntyChecker;
