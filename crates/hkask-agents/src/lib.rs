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
//! use hkask_agents::pod::{AgentPod, AgentPersona};
//! use hkask_agents::acp::{AcpRuntime, TemplateDispatchHandler};
//! use std::sync::Arc;
//!
//! // Create ACP runtime
//! let acp = Arc::new(AcpRuntime::new(b"secret-key"));
//!
//! // Create dispatch handler
//! let handler = TemplateDispatchHandler::new(acp.clone());
//! # Ok(())
//! # }
//! ```

pub mod acp;
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
    MCPRuntimePort, MemoryStoragePort, PodID, PodLifecycleState, TemplateCrate,
};
