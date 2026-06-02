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
//! ```rust,ignore
//! use hkask_agents::pod::PodManager;
//! ```

pub mod acp;
pub mod adapters;
pub mod communication;

pub mod consent;
pub mod curator;
pub mod error;
pub mod pod;
pub mod ports;
pub mod registry_loader;

pub mod sovereignty;

pub use acp::{A2AMessage, AcpAgent, AcpError, AcpRuntime};

pub use communication::{CommunicationLoop, MessageDispatch};
pub use consent::{ConsentError, ConsentManager};
pub use curator::context::CuratorContext;
pub use curator::curation_loop::CurationLoop;
pub use curator::escalation::{EscalationEntry, EscalationQueue};
pub use error::{GitError, McpError, MemoryError, RegistryError};
pub use pod::{
    AgentKind, AgentPersona, AgentPod, AgentPodError, AgentPodResult, PodID, PodLifecycleState,
    PodManager, PodStatus,
};
pub use ports::{AcpPort, EpisodicStoragePort, GitCASPort, MCPRuntimePort, SemanticStoragePort};
pub use registry_loader::{BotRegistryLoader, RegistryLoaderError};
pub use sovereignty::SovereigntyChecker;
