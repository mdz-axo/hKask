//! PodContext — Runtime context for an active pod
//!
//! Provides access to all ports (inference, memory, MCP, CNS) for a specific pod.
//! This is the unit of access that enforces the pod invariant: all interactions
//! with memory, inference, and tools must go through a pod context.
//!
//! # OCAP Discipline (Phase 2)
//!
//! Memory is now split into episodic and semantic ports:
//! - `episodic_storage` — private, agent-scoped memory (EpisodicStoragePort)
//! - `semantic_storage` — shared, public knowledge (SemanticStoragePort)
//!
//! The legacy `memory_storage` field (MemoryStoragePort) is deprecated.
//! Use `recall_episodic`/`store_episodic` and `recall_semantic`/`store_semantic`
//! instead of `recall_memory`/`store_memory`.

use hkask_types::{
    CapabilityAction, CapabilityResource, CapabilityToken, ExperienceClassification, WebID,
};
use std::sync::Arc;

use super::AgentPodError;
use super::manager::PodManager;
use super::types::PodID;
#[allow(deprecated)]
use crate::ports::{EpisodicStoragePort, MCPRuntimePort, MemoryStoragePort, SemanticStoragePort};

/// PodContext — Runtime context for an active pod
///
/// Provides access to all ports (inference, memory, MCP, CNS) for a specific pod.
/// This is the unit of access that enforces the pod invariant: all interactions
/// with memory, inference, and tools must go through a pod context.
pub struct PodContext {
    pub pod_id: PodID,
    pub webid: WebID,
    pub capability_token: CapabilityToken,
    inference_port: Option<Arc<dyn hkask_templates::InferencePort>>,
    /// Episodic memory storage — private, agent-scoped (OCAP: EpisodicReadHandle/EpisodicWriteHandle)
    episodic_storage: Arc<dyn EpisodicStoragePort>,
    /// Semantic memory storage — shared, public knowledge (OCAP: SemanticReadHandle/SemanticWriteHandle)
    semantic_storage: Arc<dyn SemanticStoragePort>,
    mcp_runtime: Arc<dyn MCPRuntimePort>,
    /// Legacy memory storage (deprecated — use episodic_storage/semantic_storage)
    #[allow(deprecated)]
    memory_storage: Arc<dyn MemoryStoragePort>,
}

impl PodContext {
    pub async fn from_manager(manager: &PodManager, pod_id: &PodID) -> Result<Self, AgentPodError> {
        let pods = manager.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        if pod.state != super::types::PodLifecycleState::Activated {
            return Err(AgentPodError::ACPRegistrationError(
                "Pod must be activated before creating context".to_string(),
            ));
        }

        Ok(Self {
            pod_id: *pod_id,
            webid: pod.webid,
            capability_token: pod.capability_token.clone(),
            inference_port: manager.inference_port.clone(),
            episodic_storage: Arc::clone(&manager.episodic_storage),
            semantic_storage: Arc::clone(&manager.semantic_storage),
            mcp_runtime: Arc::clone(&manager.mcp_runtime),
            #[allow(deprecated)]
            memory_storage: Arc::clone(&manager.memory_storage),
        })
    }

    fn require_capability(
        &self,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> Result<(), AgentPodError> {
        if !self
            .capability_token
            .is_valid_for(resource, resource_id, action)
        {
            return Err(AgentPodError::CapabilityDenied { resource, action });
        }
        Ok(())
    }

    pub fn inference_port(&self) -> Result<Arc<dyn hkask_templates::InferencePort>, AgentPodError> {
        self.require_capability(
            CapabilityResource::Template,
            "inference",
            CapabilityAction::Render,
        )?;
        self.inference_port.clone().ok_or_else(|| {
            AgentPodError::InferenceUnavailable("No inference port configured".to_string())
        })
    }

    // ========================================================================
    // Episodic memory methods — private, agent-scoped
    // ========================================================================

    /// Store an episodic triple (private, agent-scoped).
    ///
    /// OCAP: Only the owning agent can store episodic triples.
    /// The `perspective` field is automatically set to the agent's WebID.
    pub fn store_episodic(
        &self,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
        confidence: f64,
    ) -> Result<String, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "episodic_memory",
            CapabilityAction::Write,
        )?;
        self.episodic_storage
            .store_episodic(
                self.webid,
                entity,
                attribute,
                value,
                confidence,
                &self.capability_token,
            )
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    /// Recall episodic triples for the agent's own perspective.
    ///
    /// OCAP: Only the owning agent can read their own episodic triples.
    /// Returns only triples matching the agent's perspective.
    pub fn recall_episodic(&self, query: &str) -> Result<Vec<serde_json::Value>, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "episodic_memory",
            CapabilityAction::Read,
        )?;
        self.episodic_storage
            .recall_episodic(query, &self.webid, &self.capability_token)
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    /// Check episodic storage usage for this agent's perspective.
    ///
    /// Returns the number of episodic triples currently stored.
    /// Used by Loop 2a.5 (Storage Budget) to enforce per-agent limits.
    pub fn episodic_storage_usage(&self) -> Result<usize, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "episodic_memory",
            CapabilityAction::Read,
        )?;
        self.episodic_storage
            .episodic_storage_usage(&self.webid)
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    /// Store an episodic experience with classification (Loop 2a.1).
    ///
    /// This is the enhanced store method that accepts an experience
    /// classification. The classification determines the default confidence
    /// if `confidence_override` is `None`. Emits a `cns.memory.encode` span.
    ///
    /// Experience classifications and their default confidences:
    /// - `Success` → 0.9
    /// - `Failure` → 0.3
    /// - `Observation` → 0.7
    /// - `Inference` → 0.5
    /// - `Instruction` → 0.8
    pub fn store_episodic_experience(
        &self,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
        classification: ExperienceClassification,
        confidence_override: Option<f64>,
    ) -> Result<String, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "episodic_memory",
            CapabilityAction::Write,
        )?;

        let confidence = confidence_override.unwrap_or_else(|| classification.default_confidence());

        self.episodic_storage
            .store_episodic_classified(
                self.webid,
                entity,
                attribute,
                value,
                classification,
                confidence_override,
                &self.capability_token,
            )
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    // ========================================================================
    // Semantic memory methods — shared, public knowledge
    // ========================================================================

    /// Store a semantic triple (shared, public knowledge).
    ///
    /// OCAP: Agents with consolidation capability can store semantic triples.
    /// Semantic triples have no perspective (consolidated from episodic).
    pub fn store_semantic(
        &self,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
        confidence: f64,
    ) -> Result<String, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "semantic_memory",
            CapabilityAction::Write,
        )?;
        self.semantic_storage
            .store_semantic(
                self.webid,
                entity,
                attribute,
                value,
                confidence,
                &self.capability_token,
            )
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    /// Recall semantic triples (shared, deduplicated knowledge).
    ///
    /// OCAP: Any agent with a valid capability token can read semantic triples.
    pub fn recall_semantic(&self, query: &str) -> Result<Vec<serde_json::Value>, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "semantic_memory",
            CapabilityAction::Read,
        )?;
        self.semantic_storage
            .recall_semantic(query, &self.capability_token)
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    /// Check semantic storage usage for an entity.
    ///
    /// Returns the number of semantic triples currently stored for the entity.
    /// Used by Loop 6e (Semantic Storage Budget) to enforce per-entity limits.
    pub fn semantic_storage_usage(&self, entity: &str) -> Result<usize, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "semantic_memory",
            CapabilityAction::Read,
        )?;
        self.semantic_storage
            .semantic_storage_usage(entity)
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    // ========================================================================
    // Legacy memory methods (deprecated — use episodic/semantic methods)
    // ========================================================================

    /// Recall memory (deprecated — use `recall_episodic` or `recall_semantic`)
    #[deprecated(note = "Use recall_episodic() or recall_semantic() instead")]
    pub async fn recall_memory(
        &self,
        query: &str,
    ) -> Result<Vec<serde_json::Value>, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "memory",
            CapabilityAction::Read,
        )?;
        #[allow(deprecated)]
        self.memory_storage
            .recall(query, &self.capability_token)
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    /// Store memory (deprecated — use `store_episodic` or `store_semantic`)
    #[deprecated(note = "Use store_episodic() or store_semantic() instead")]
    pub async fn store_memory(
        &self,
        artifact_type: &str,
        content: serde_json::Value,
        visibility: &str,
    ) -> Result<String, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "memory",
            CapabilityAction::Write,
        )?;
        #[allow(deprecated)]
        self.memory_storage
            .store_artifact(
                self.webid,
                artifact_type,
                content,
                visibility,
                &self.capability_token,
            )
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    // ========================================================================
    // Tool invocation and CNS span emission
    // ========================================================================

    pub fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AgentPodError> {
        self.require_capability(
            CapabilityResource::Tool,
            tool_name,
            CapabilityAction::Execute,
        )?;
        self.mcp_runtime
            .invoke_tool(tool_name, input, &self.capability_token)
            .map_err(|e| AgentPodError::ToolError(e.to_string()))
    }
}
