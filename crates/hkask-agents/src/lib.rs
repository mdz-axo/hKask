//! hKask Agents — Agent Pod Lifecycle and ACP Integration
//!
//! This crate provides:
//! - **Agent Pod**: Runtime container for ACP agents (bots and replicants)
//! - **Lifecycle Management**: Populated → Registered → Activated → Deactivated
//! - **Capability Tokens**: OCAP-based access control with attenuation
//! - **ACP Runtime**: Agent registration, A2A messaging, capability verification
//! - **Hexagonal Ports**: AcpPort, MCPRuntimePort, CnsEmit, GitCasAdapter
//!
//! # Example
//!
//! ```rust,ignore
//! use hkask_agents::pod::PodManager;
//! ```

pub mod acp; // Loop 6 (Cybernetics: ACP is access control)
pub mod adapters;
pub mod consent; // Loop 6
pub mod curator; // Loop 5
pub mod curator_agent; // Loop 5
pub mod ensemble; // Multi-agent chat coordination
pub mod error;

pub mod inference_loop; // Loop 1 (domain logic; governance applied externally via GovernedTool in hkask-cns)
pub mod loop_system;
pub mod pod; // Loop 5 (agent pod lifecycle is Curation)
pub mod ports;
pub mod prompt_analysis; // Loop 1 (inference variety sensing — relocated from hkask-cns)
pub mod registry_loader;
pub mod sovereignty; // Loop 6 (sovereignty enforcement)

pub use acp::{A2AMessage, AcpAgent, AcpError, AcpRuntime};

pub use consent::{ConsentError, ConsentManager};
pub use curator::context::CuratorContext;
pub use curator::curation_loop::CurationLoop;
pub use curator_agent::{CuratorAgent, DefaultSpecCurator};

pub use error::{CoreError, GitError, MemoryError};
pub use hkask_storage::{
    EscalationBatch, EscalationEntry, EscalationError, EscalationQueue, EscalationStats,
    EscalationStatus,
};
pub use inference_loop::InferenceLoop;
pub use loop_system::{CyberneticsLoopHandle, LoopSystem};
pub use pod::{AgentPersona, PodID, PodManager, PodStatus};
pub use ports::{
    AcpPort, EpisodicStoragePort, GitCasAdapter, RecallRequest, RecalledEpisode, RecalledSemantic,
    SemanticStoragePort, StorageRequest,
};
pub use prompt_analysis::{PromptAnalysis, SentenceDecomposition, decompose_prompt};
pub use registry_loader::AgentRegistryLoader;
pub use sovereignty::{AllowAllConsent, DenyAllConsent, SovereigntyChecker, SovereigntyConsent};
