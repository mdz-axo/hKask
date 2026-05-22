//! hKask Agents — Agent Pod Lifecycle and ACP Integration
//!
//! This crate provides:
//! - **Agent Pod**: Runtime container for ACP agents (bots and replicants)
//! - **Lifecycle Management**: Populated → Registered → Activated → Deactivated
//! - **Capability Tokens**: OCAP-based access control with attenuation
//! - **ACP Runtime**: Agent registration, A2A messaging, capability verification
//! - **Hexagonal Ports**: ACPRuntimePort, MCPRuntimePort, CNSSpanPort, GitCASPort
//!
//! # Example
//!
//! ```rust,no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use hkask_agents::pod::PodManager;
//! use hkask_agents::adapters::git_cas::GitCasAdapter;
//! use hkask_agents::adapters::acp_runtime::AcpRuntimeAdapter;
//! use hkask_agents::adapters::cns_emitter::CnsEmitterAdapter;
//! use hkask_agents::adapters::mcp_runtime::McpRuntimeAdapter;
//! use hkask_agents::adapters::memory_storage::MemoryStorageAdapter;
//! use std::path::PathBuf;
//!
//! // Create adapters
//! let git_cas = GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates"));
//! let acp_runtime = AcpRuntimeAdapter::new();
//! let cns_emitter = CnsEmitterAdapter::new(hkask_types::WebID::new());
//! let mcp_runtime = McpRuntimeAdapter::new();
//! let memory_storage = MemoryStorageAdapter::in_memory()?;
//!
//! // Create pod manager
//! let manager = PodManager::new(git_cas, acp_runtime, cns_emitter, mcp_runtime, memory_storage);
//! # Ok(())
//! # }
//! ```

pub mod acp;
pub mod adapters;
pub mod bot;
pub mod capability;
pub mod consent;
pub mod curator;
pub mod ocap;
pub mod pod;
pub mod ports;
pub mod replicant;
pub mod security;
pub mod sovereignty;

pub use acp::{A2AMessage, AcpAgent, AcpRuntime, TemplateDispatchHandler};
pub use capability::{BotCapabilities, CapabilityChecker, CapabilityToken};
pub use consent::ConsentManager;
pub use pod::{
    AgentPersona, AgentPod, AgentPodError, AgentPodResult, AgentType, CNSSpanPort, GitCASPort,
    MCPRuntimePort, MemoryStoragePort, PodID, PodLifecycleState, PodManager, PodStatus,
    TemplateCrate,
};
pub use sovereignty::SovereigntyChecker;
