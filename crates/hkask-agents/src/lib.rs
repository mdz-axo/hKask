//! hKask Agents — Agent Pod Lifecycle and ACP Integration
//!
//! This crate provides:
//! - **Agent Pod**: Runtime container for ACP agents (bots and replicants)
//! - **Lifecycle Management**: Populated → Registered → Activated → Deactivated
//! - **Capability Tokens**: OCAP-based access control with attenuation
//! - **Hexagonal Ports**: ACPRuntimePort, MCPRuntimePort, CNSSpanPort, GitCASPort, MemoryStoragePort

pub mod bot;
pub mod capability;
pub mod curator;
pub mod ocap;
pub mod pod;
pub mod replicant;

pub use capability::{BotCapabilities, CapabilityChecker, CapabilityToken};
pub use pod::{
    AgentPersona, AgentPod, AgentPodError, AgentPodResult, AgentType, GitCASPort, MCPRuntimePort,
    MemoryStoragePort, CNSSpanPort, PodID, PodLifecycleState, TemplateCrate,
};
