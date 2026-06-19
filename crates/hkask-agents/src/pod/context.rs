//! PodContext — Runtime context for an active pod
//!
//! Provides access to all ports (inference, memory, MCP, CNS) for a specific pod.
//! This is the unit of access that enforces the pod invariant: all interactions
//! with memory, inference, and tools must go through a pod context.
//!
//! # OCAP Discipline
//!
//! Memory is split into episodic and semantic ports:
//! - `episodic_storage` — private, agent-scoped memory (EpisodicStoragePort)
//! - `semantic_storage` — shared, public knowledge (SemanticStoragePort)

use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_types::ports::ToolPort;
use hkask_types::sovereignty::DataCategory;
use hkask_types::{
    CapabilityChecker, Confidence, DelegationAction, DelegationResource, DelegationToken,
    ExperienceClassification, InferencePort, WebID,
};
use std::sync::Arc;

use super::AgentPodError;
use super::deployment::{PerPodCnsRuntime, PodDeployment};
use super::types::PodID;
use crate::SovereigntyChecker;
use crate::curator::SemanticIndex;
use crate::ports::{
    EpisodicStoragePort, MCPRuntimePort, RecallRequest, RecalledEpisode, RecalledSemantic,
    SemanticStoragePort, StorageRequest,
};

/// PodContext — Runtime context for an active pod
///
/// Provides access to all ports (inference, memory, MCP, CNS) for a specific pod.
/// This is the unit of access that enforces the pod invariant: all interactions
/// \[NORMATIVE\] with memory, inference, and tools must go through a pod context. (P4 — Clear Boundaries).
pub struct PodContext {
    pub pod_id: PodID,
    pub webid: WebID,
    pub capability_token: DelegationToken,
    inference_port: Option<Arc<dyn InferencePort>>,
    /// Episodic memory storage — private, agent-scoped (OCAP: DelegationToken)
    episodic_storage: Arc<dyn EpisodicStoragePort>,
    /// Semantic memory storage — shared, public knowledge (OCAP: DelegationToken)
    semantic_storage: Arc<dyn SemanticStoragePort>,
    mcp_runtime: Arc<dyn MCPRuntimePort>,
    /// GovernedTool membrane — routes tool invocations through CNS governance
    /// (energy budget, variety tracking, event spans). When present, `invoke_tool`
    /// routes through this membrane instead of the raw `mcp_runtime`, ensuring
    /// pod-initiated calls are subject to Cybernetics governance.
    governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
    /// Cryptographic capability checker for OCAP verification.
    /// When set, `require_capability()` verifies HMAC signatures.
    /// When absent, falls back to structural `is_valid_for()` check (insecure).
    capability_checker: Option<Arc<CapabilityChecker>>,
    /// Sovereignty checker for this pod — wired to a live `SovereigntyConsent`
    /// port so grants via the API or CLI are observed. `None` means the
    /// manager was constructed without sovereignty wiring; in that case
    /// `require_sovereignty` denies by default.
    sovereignty_checker: Option<Arc<SovereigntyChecker>>,
    /// Per-pod CNS runtime — used to emit `cns.semantic.published` events
    /// on semantic writes. Cloned from PodDeployment (CnsRuntime is Arc-wrapped).
    cns: PerPodCnsRuntime,
    /// CuratorPod's SemanticIndex — available on non-Curator pods for
    /// merged-lens semantic recall. `None` if no CuratorPod is active.
    curator_index: Option<Arc<tokio::sync::RwLock<SemanticIndex>>>,
}

impl PodContext {
    pub fn from_deployment(deployment: &PodDeployment) -> Result<Self, AgentPodError> {
        if deployment.pod.state != super::types::PodLifecycleState::Activated {
            return Err(AgentPodError::PodNotActivated);
        }

        Ok(Self {
            pod_id: deployment.pod_id,
            webid: deployment.pod.webid,
            capability_token: deployment.pod.capability_token.clone(),
            inference_port: deployment.inference_port.clone(),
            episodic_storage: Arc::clone(&deployment.episodic_storage),
            semantic_storage: Arc::clone(&deployment.semantic_storage),
            mcp_runtime: Arc::clone(&deployment.mcp_runtime),
            governed_tool: deployment.tools.governed_tool.clone(),
            capability_checker: deployment.capability_checker.clone(),
            sovereignty_checker: Some(Arc::new(deployment.sovereignty_checker.clone())),
            cns: deployment.cns.clone(),
            curator_index: None,
        })
    }

    /// Access the per-pod CNS runtime for observability queries.
    pub fn cns(&self) -> &PerPodCnsRuntime {
        &self.cns
    }

    /// Wire this context to a CuratorPod's SemanticIndex for merged-lens
    /// semantic recall. Called by ActivePods when a CuratorPod is active.
    pub fn with_curator_index(mut self, index: Arc<tokio::sync::RwLock<SemanticIndex>>) -> Self {
        self.curator_index = Some(index);
        self
    }

    fn require_capability(
        &self,
        resource: DelegationResource,
        resource_id: &str,
        action: DelegationAction,
    ) -> Result<(), AgentPodError> {
        if let Some(ref checker) = self.capability_checker {
            // Full cryptographic verification: HMAC signature + expiry + holder + resource/action
            if !checker.check(
                &self.capability_token,
                &self.webid,
                resource,
                resource_id,
                action,
            ) {
                return Err(AgentPodError::CapabilityDenied { resource, action });
            }
        } else {
            // No checker configured — permissive mode (test/dev).
            // In production, a capability checker is always wired.
            tracing::debug!(
                target: "hkask.ocap",
                webid = ?self.webid,
                resource = ?resource,
                "No capability checker configured — permissive mode (accepting)"
            );
        }
        Ok(())
    }

    /// Require that the pod may access the given data category for the
    /// requesting WebID. Complements `require_capability` by enforcing the
    /// Magna Carta's data-sovereignty policy (sovereign / shared / public
    /// classification with explicit-consent lookup).
    ///
    /// When no sovereignty checker is configured (a misconfiguration),
    /// \[NORMATIVE\] the call denies by default — sovereignty must fail closed. (P1 — User Sovereignty).
    pub fn require_sovereignty(
        &self,
        data_category: &DataCategory,
        requester: &WebID,
    ) -> Result<(), AgentPodError> {
        let checker = match self.sovereignty_checker {
            Some(ref c) => c,
            None => {
                tracing::error!(
                    target: "hkask.sovereignty",
                    webid = ?self.webid,
                    "No sovereignty checker configured — sovereignty check denied"
                );
                return Err(AgentPodError::SovereigntyDenied {
                    category: data_category.clone(),
                    requester: *requester,
                });
            }
        };
        if !checker.can_access(data_category, requester) {
            return Err(AgentPodError::SovereigntyDenied {
                category: data_category.clone(),
                requester: *requester,
            });
        }
        Ok(())
    }

    pub fn inference_port(&self) -> Result<Arc<dyn InferencePort>, AgentPodError> {
        self.require_capability(
            DelegationResource::Template,
            "inference",
            DelegationAction::Execute,
        )?;
        self.inference_port.clone().ok_or_else(|| {
            AgentPodError::InferenceUnavailable("No inference port configured".to_string())
        })
    }

    // Episodic memory methods — private, agent-scoped

    /// Store an episodic triple (private, agent-scoped).
    ///
    /// OCAP: Only the owning agent can store episodic triples.
    /// The `perspective` field is automatically set to the agent's WebID.
    pub fn store_episodic(
        &self,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
        confidence: impl Into<Confidence>,
    ) -> Result<String, AgentPodError> {
        self.require_capability(
            DelegationResource::Registry,
            "episodic_memory",
            DelegationAction::Write,
        )?;
        self.require_sovereignty(&DataCategory::EpisodicMemory, &self.webid)?;
        let request =
            StorageRequest::episodic(entity, attribute, value, confidence.into(), self.webid);
        self.episodic_storage
            .store_episodic(request, &self.capability_token)
            .map_err(AgentPodError::from)
    }

    /// Recall episodic triples for the agent's own perspective.
    ///
    /// OCAP: Only the owning agent can read their own episodic triples.
    /// Returns only triples matching the agent's perspective.
    pub fn recall_episodic(&self, query: &str) -> Result<Vec<RecalledEpisode>, AgentPodError> {
        self.require_capability(
            DelegationResource::Registry,
            "episodic_memory",
            DelegationAction::Read,
        )?;
        self.require_sovereignty(&DataCategory::EpisodicMemory, &self.webid)?;
        let request = RecallRequest::episodic(query, self.webid, self.capability_token.clone());
        self.episodic_storage
            .recall_episodic(&request)
            .map_err(AgentPodError::from)
    }

    /// Check episodic storage usage for this agent's perspective.
    ///
    /// Returns the number of episodic triples currently stored.
    /// Used by Loop 2a.4 (Storage Budget) to enforce per-agent limits.
    pub fn episodic_storage_usage(&self) -> Result<usize, AgentPodError> {
        self.require_capability(
            DelegationResource::Registry,
            "episodic_memory",
            DelegationAction::Read,
        )?;
        self.episodic_storage
            .episodic_storage_usage(&self.webid)
            .map_err(AgentPodError::from)
    }

    /// Get the per-agent storage budget (max episodic triples).
    pub fn episodic_storage_budget(&self) -> usize {
        self.episodic_storage.episodic_storage_budget()
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
    pub fn store_episodic_experience(
        &self,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
        classification: ExperienceClassification,
        confidence_override: Option<Confidence>,
    ) -> Result<String, AgentPodError> {
        self.require_capability(
            DelegationResource::Registry,
            "episodic_memory",
            DelegationAction::Write,
        )?;

        let request = StorageRequest::episodic(
            entity,
            attribute,
            value,
            // Confidence will be resolved from classification in the adapter
            confidence_override
                .unwrap_or_else(|| Confidence::new(classification.default_confidence())),
            self.webid,
        );

        self.episodic_storage
            .store_episodic_classified(
                request,
                classification,
                confidence_override,
                &self.capability_token,
            )
            .map_err(AgentPodError::from)
    }

    // Semantic memory methods — shared, public knowledge

    /// Store a semantic triple (shared, public knowledge).
    ///
    /// OCAP: Agents with consolidation capability can store semantic triples.
    /// Semantic triples have no perspective (consolidated from episodic).
    ///
    /// On success, fires `cns.semantic.published` to trigger the Curator's
    /// sense loop — this is the push-then-pull lazy sync protocol.
    pub fn store_semantic(
        &self,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
        confidence: impl Into<Confidence>,
    ) -> Result<String, AgentPodError> {
        self.require_capability(
            DelegationResource::Registry,
            "semantic_memory",
            DelegationAction::Write,
        )?;
        self.require_sovereignty(&DataCategory::SemanticMemory, &self.webid)?;
        let request =
            StorageRequest::semantic(entity, attribute, value, confidence.into(), self.webid);
        let result = self
            .semantic_storage
            .store_semantic(request, &self.capability_token)
            .map_err(AgentPodError::from)?;

        // Step 3: Emit CNS event to trigger Curator sense loop.
        // Fire-and-forget: spawn on the current runtime so it doesn't block.
        let cns = self.cns.inner().clone();
        let entity = entity.to_string();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                cns.increment_variety("cns.semantic.published", &entity)
                    .await;
            });
        }

        Ok(result)
    }

    /// Recall semantic triples (shared, deduplicated knowledge).
    ///
    /// OCAP: Any agent with a valid capability token can read semantic triples.
    ///
    /// Step 5: When a CuratorPod is wired, routes through the Curator's
    /// SemanticIndex for a merged-lens view across all pods. Falls back
    /// to local semantic storage if no Curator is available.
    pub fn recall_semantic(&self, query: &str) -> Result<Vec<RecalledSemantic>, AgentPodError> {
        self.require_capability(
            DelegationResource::Registry,
            "semantic_memory",
            DelegationAction::Read,
        )?;
        self.require_sovereignty(&DataCategory::SemanticMemory, &self.webid)?;

        // Route through Curator's merged index when available (Step 5)
        if let Some(ref index_lock) = self.curator_index {
            // Spawn the async query into the runtime and use a oneshot channel
            // to receive the result synchronously. Works in both single-threaded
            // and multi-threaded tokio runtimes.
            let q = query.to_string();
            let lock = Arc::clone(index_lock);
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("temp runtime");
                rt.block_on(async {
                    let guard = lock.read().await;
                    let result = guard.query_by_entity(&q);
                    let _ = tx.send(result);
                });
            });
            let triples = match rx.recv() {
                Ok(Ok(t)) => t,
                Ok(Err(e)) => {
                    return Err(AgentPodError::MemoryError(crate::error::MemoryError::Core(
                        crate::error::CoreError::Infra(hkask_types::InfrastructureError::Database(
                            e.to_string(),
                        )),
                    )));
                }
                Err(_) => return self.recall_semantic_local(query),
            };
            return Ok(triples
                .into_iter()
                .map(|t| RecalledSemantic {
                    id: t.id.to_string(),
                    entity: t.entity,
                    attribute: t.attribute,
                    value: t.value,
                    confidence: t.confidence,
                    visibility: t.access.visibility,
                    valid_from: t.temporal.valid_from.to_rfc3339(),
                })
                .collect());
        }

        // Fallback: local semantic store
        self.recall_semantic_local(query)
    }

    /// Fallback semantic recall — queries the pod's own storage.
    fn recall_semantic_local(&self, query: &str) -> Result<Vec<RecalledSemantic>, AgentPodError> {
        let request = RecallRequest::semantic(query, self.capability_token.clone());
        self.semantic_storage
            .recall_semantic(&request)
            .map_err(AgentPodError::from)
    }

    /// Check semantic storage usage for an entity.
    ///
    /// Returns the number of semantic triples currently stored for the entity.
    /// Used by Loop 6e (Semantic Storage Budget) to enforce per-entity limits.
    pub fn semantic_storage_usage(&self, entity: &str) -> Result<usize, AgentPodError> {
        self.require_capability(
            DelegationResource::Registry,
            "semantic_memory",
            DelegationAction::Read,
        )?;
        self.semantic_storage
            .semantic_storage_usage(entity)
            .map_err(AgentPodError::from)
    }

    // Tool invocation and CNS span emission

    /// Invoke an MCP tool by name.
    ///
    /// When a `GovernedTool` membrane is configured, routes through it to get
    /// CNS governance (energy budget enforcement, variety tracking, algedonic spans).
    /// When no membrane is present, falls back to the raw `mcp_runtime` path
    /// which performs OCAP verification but bypasses CNS observability.
    pub fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AgentPodError> {
        self.require_capability(
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
        )?;

        if let Some(ref governed) = self.governed_tool {
            // Route through GovernedTool membrane (CNS governance: gas, variety, spans)
            let server = self
                .mcp_runtime
                .resolve_tool_server(tool_name)
                .unwrap_or_else(|| "pod".to_string());

            let rt = tokio::runtime::Handle::current();
            match rt.block_on(governed.invoke(&server, tool_name, input, &self.capability_token)) {
                Ok(value) => Ok(value),
                Err(e) => Err(AgentPodError::ToolError(e.into())),
            }
        } else {
            // Fallback: raw mcp_runtime path (OCAP verification but no CNS governance)
            self.mcp_runtime
                .invoke_tool(tool_name, input, &self.capability_token)
                .map_err(|e| AgentPodError::ToolError(e.into()))
        }
    }
}
