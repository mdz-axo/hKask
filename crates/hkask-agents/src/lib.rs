#![allow(unused_imports)]
//! hKask Agents — Agent Pod Lifecycle and A2A Integration
//!
//! This crate provides:
//! - **Agent Pod**: Runtime container for A2A agents (bots and replicants)
//! - **Lifecycle Management**: Populated → Registered → Activated → Deactivated
//! - **Capability Tokens**: OCAP-based access control with attenuation
//! - **A2A Runtime**: Agent registration, A2A messaging, capability verification
//! - **Hexagonal Ports**: A2APort, MCPRuntimePort, CnsEmit, GitCasAdapter
//!
//! # Example
//!
//! ```rust,ignore
//! use hkask_agents::pod::PodManager;
//! ```

pub mod a2a; // Loop 6 (Cybernetics: A2A is access control)
pub mod adapters;
pub mod consent; // Loop 6
pub mod curator; // Loop 5
pub mod curator_agent; // Loop 5
pub mod error;

pub mod inference_loop; // Loop 1 (domain logic; governance applied externally via GovernedTool in hkask-cns)
pub mod loop_system;
pub mod pod; // Loop 5 (agent pod lifecycle is Curation)
pub mod ports;
pub mod prompt_analysis; // Loop 1 (inference variety sensing — relocated from hkask-cns)
pub mod registry_loader;
pub mod sovereignty; // Loop 6 (sovereignty enforcement)
pub mod types;
pub mod yaml_parser;
pub mod yaml_types;

// Re-export rich agent domain types from types/ (these are the canonical versions
// that extend the hkask-types foundation types with additional fields).
pub use types::agent::definition::{AgentDefinition, Charter, PersonaConstraints, RegisteredAgent};
pub use types::agent::profile::{Contact, Responsibility, Right, ScheduledTask, UserProfile};

pub use a2a::{A2AAgent, A2AError, A2AMessage, A2ARuntime};

pub use consent::{ConsentError, ConsentManager};
pub use curator::context::CuratorContext;
pub use curator::curation_loop::CurationLoop;
pub use curator::{CuratorSync, SemanticIndex};
pub use curator_agent::{CuratorAgent, DefaultSpecCurator};

pub use error::{CoreError, MemoryError};
pub use inference_loop::InferenceLoop;
pub use loop_system::{CyberneticsLoopHandle, LoopSystem};
pub use pod::{
    ActivePods, AgentMode, AgentPersona, PodDeployment, PodFactory, PodID, PodKind, PodRegistry,
};
pub use ports::{
    A2APort, EpisodicStoragePort, GitCasAdapter, RecallRequest, RecalledEpisode, RecalledSemantic,
    SemanticStoragePort, StorageRequest,
};
pub use prompt_analysis::{PromptAnalysis, SentenceDecomposition, decompose_prompt};
pub use registry_loader::AgentRegistryLoader;
pub use sovereignty::{AllowAllConsent, DenyAllConsent, SovereigntyChecker, SovereigntyConsent};

// Agent types remain in hkask-types (canonical location for SQL impls).
pub use types::voice::VoiceDesign;
