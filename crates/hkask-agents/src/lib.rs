//! hKask Agents — Agent Pod Lifecycle and ACP Integration
//!
//! This crate provides:
//! - **Agent Pod**: Runtime container for ACP agents (bots and replicants)
//! - **Lifecycle Management**: Populated → Registered → Activated → Deactivated
//! - **Capability Tokens**: OCAP-based access control with attenuation
//! - **ACP Runtime**: Agent registration, A2A messaging, capability verification
//! - **Hexagonal Ports**: ACPRuntimePort, MCPRuntimePort, CNSSpanPort, GitCASPort, MemoryStoragePort
//!
//! # Example
//!
//! ```rust,no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use hkask_agents::pod::PodManager;
//! use hkask_agents::adapters::git_cas::GitCasAdapter;
//! use std::path::PathBuf;
//!
//! // Create Git CAS adapter
//! let git_cas = GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates"));
//!
//! // Create pod manager
//! let manager = PodManager::new(git_cas);
//! # Ok(())
//! # }
//! ```

pub mod acp;
pub mod adapters;
pub mod bot;
pub mod capability;
pub mod curator;
pub mod ocap;
pub mod pod;
pub mod replicant;

pub use acp::{A2AMessage, AcpAgent, AcpRuntime, TemplateDispatchHandler};
pub use capability::{BotCapabilities, CapabilityChecker, CapabilityToken};
pub use pod::{
    AgentPersona, AgentPod, AgentPodError, AgentPodResult, AgentType, CNSSpanPort, GitCASPort,
    MCPRuntimePort, MemoryStoragePort, PodID, PodLifecycleState, PodManager, PodStatus,
    TemplateCrate,
};
