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
//! use std::path::PathBuf;
//! use std::sync::Arc;
//!
//! // Create adapters
//! let git_cas = Arc::new(GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates")));
//! let acp_runtime = Arc::new(AcpRuntime::default());
//! let cns_emitter = Arc::new(CnsEmitterAdapter::new(hkask_types::WebID::new()));
//! let mcp_runtime = Arc::new(McpRuntimeAdapter::new());
//! let memory_storage = Arc::new(MemoryStorageAdapter::in_memory()?);
//!
//! // Create pod manager
//! let manager = PodManager::new(git_cas, acp_runtime, cns_emitter, mcp_runtime, memory_storage);
//! # Ok(())
//! # }
//! ```

pub mod acp;
pub mod adapters;
pub mod bot;
pub mod consent;
pub mod curator;
pub mod error;
pub mod ocap;
pub mod pod;
pub mod ports;
<<<<<<< HEAD
pub mod registry_loader;
=======
>>>>>>> origin/main
pub mod replicant;
pub mod revocation_store;
pub mod security;
pub mod sovereignty;

<<<<<<< HEAD
pub use acp::{A2AMessage, AcpAgent, AcpError, AcpRuntime, TemplateDispatchHandler};
pub use adapters::{LoopbackHttpTransport, StdioTransport};
pub use bot::Bot;
pub use consent::ConsentManager;
pub use curator::escalation::{
    EscalationEntry, EscalationQueue, EscalationStats, EscalationStatus,
};
pub use error::{GitError, McpError, MemoryError};
pub use hkask_types::{BotCapabilities, CapabilityChecker, CapabilityToken};
pub use pod::{
    AgentPersona, AgentPod, AgentPodError, AgentPodResult, AgentType, PodID, PodLifecycleState,
    PodManager, PodStatus, TemplateCrate,
};
pub use ports::{AcpPort, AcpTransport, AcpWireMessage, AcpWireResponse, GitCASPort, MCPRuntimePort, MemoryStoragePort};
pub use registry_loader::{BotRegistryLoader, RegistryLoaderError};
pub use replicant::{Replicant, ReplicantCapabilities};
=======
pub use crate::acp::{A2AMessage, AcpAgent, AcpRuntime, TemplateDispatchHandler};
pub use capability::{BotCapabilities, CapabilityChecker, CapabilityToken};
pub use pod::{
    AgentPersona, AgentPod, AgentPodError, AgentPodResult, AgentType, CNSSpanPort, GitCASPort,
    MCPRuntimePort, MemoryStoragePort, PodID, PodLifecycleState, PodManager, PodStatus,
    TemplateCrate,
};
pub use ports::{SovereigntyError, SovereigntyPort, SovereigntyResult};
>>>>>>> origin/main
pub use sovereignty::SovereigntyChecker;
