//! PodManager, PodStatus — Pod lifecycle management

use hkask_cns::GovernedTool;
use hkask_mcp::raw_tool_port::RawMcpToolPort;
use hkask_types::{CapabilityChecker, InferencePort, NuEventSink};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::types::{AgentKind, AgentPersona, PodID, PodLifecycleState};
use super::{AgentPod, AgentPodError, AgentPodResult};
use crate::adapters::mcp_runtime::CapabilityOnlyAdapter;
use crate::adapters::memory_loop_adapter::MemoryLoopAdapter;
use crate::ports::{EpisodicStoragePort, MCPRuntimePort, RecalledEpisode, SemanticStoragePort};
use hkask_mcp::GitCasAdapter;

/// Pod Manager — Manages collection of agent pods
///
/// The PodManager provides centralized lifecycle management for all agent pods
/// in the hKask system. It handles:
/// - Pod creation from template crates
/// - Pod activation/deactivation
/// - Status queries
/// - Listing all pods
/// - Inference access via InferencePort
pub struct PodManager {
    pub(crate) pods: Arc<RwLock<HashMap<PodID, AgentPod>>>,
    git_cas: Arc<GitCasAdapter>,
    acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
    pub(crate) mcp_runtime: Arc<dyn MCPRuntimePort>,
    /// Episodic memory storage — private, agent-scoped (OCAP: DelegationToken)
    pub(crate) episodic_storage: Arc<dyn EpisodicStoragePort>,
    /// Semantic memory storage — shared, public knowledge (OCAP: DelegationToken)
    pub(crate) semantic_storage: Arc<dyn SemanticStoragePort>,

    pub(crate) inference_port: Option<Arc<dyn InferencePort>>,
    /// Cryptographic capability checker for OCAP verification.
    /// When set, `PodContext::require_capability()` verifies HMAC signatures.
    /// When absent, falls back to structural `is_valid_for()` check (insecure).
    pub(crate) capability_checker: Option<Arc<CapabilityChecker>>,
    /// GovernedTool membrane for pod tool invocations.
    /// When set, PodContext routes tool calls through CNS governance
    /// (gas budget, variety tracking, event spans).
    pub(crate) governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
    /// NuEvent sink for pod lifecycle observability.
    /// When set, pod lifecycle transitions emit NuEvents through CNS.
    nu_event_sink: Option<Arc<dyn NuEventSink>>,
    /// Sovereignty consent port — read by every pod's SovereigntyChecker to
    /// resolve explicit user consent. Defaults to a deny-all implementation
    /// (sovereignty must fail closed). Production wiring passes a
    /// `ConsentManager`-backed implementation.
    consent: Arc<dyn crate::SovereigntyConsent>,
}

/// Pod status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodStatus {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: PodLifecycleState,
    pub webid: String,
    pub agent_type: AgentKind,
    pub template: String,
    pub created_at: i64,
}

impl PodManager {
    /// Create a new pod manager with trait-object adapters.
    ///
    /// Any port passed as `None` receives a sensible default:
    /// - `git_cas` → `GitCasAdapter` rooted at `./registry/templates`
    /// - `acp_runtime` → `AcpRuntime::default()`
    /// - `mcp_runtime` → `CapabilityOnlyAdapter` with empty-secret checker (no live MCP)
    /// - `episodic_storage` / `semantic_storage` → in-memory `MemoryLoopAdapter`
    /// - `inference_port` → `None` (pods that need inference must supply it)
    /// - `capability_checker` → resolved from ACP secret chain, or `None` (fails-closed)
    /// - `governed_tool` → `None` (pods bypass CNS governance)
    /// - `nu_event_sink` → `None` (no pod lifecycle observability)
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        git_cas: Option<Arc<GitCasAdapter>>,
        acp_runtime: Option<Arc<dyn crate::ports::AcpPort + Send + Sync>>,
        mcp_runtime: Option<Arc<dyn MCPRuntimePort>>,
        episodic_storage: Option<Arc<dyn EpisodicStoragePort>>,
        semantic_storage: Option<Arc<dyn SemanticStoragePort>>,
        inference_port: Option<Arc<dyn InferencePort>>,
        capability_checker: Option<Arc<CapabilityChecker>>,
        governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
        nu_event_sink: Option<Arc<dyn NuEventSink>>,
    ) -> Self {
        // ── storage defaults ────────────────────────────────────────────────
        if episodic_storage.is_none() || semantic_storage.is_none() {
            tracing::warn!(
                target: "hkask.agents.pod",
                "No persistent storage configured — episodic and semantic memory are in-memory and will be lost on restart. \
                 Use PodManager::new() with explicit storage ports for sovereign persistence."
            );
        }
        let default_adapter = Arc::new(MemoryLoopAdapter::in_memory_unchecked());
        let default_episodic: Arc<dyn EpisodicStoragePort> = default_adapter.clone();
        let default_semantic: Arc<dyn SemanticStoragePort> = default_adapter;
        let episodic_storage = episodic_storage.unwrap_or(default_episodic);
        let semantic_storage = semantic_storage.unwrap_or(default_semantic);

        // ── git_cas default ─────────────────────────────────────────────────
        let git_cas = git_cas.unwrap_or_else(|| {
            Arc::new(GitCasAdapter::from_path(PathBuf::from(
                "./registry/templates",
            )))
        });

        // ── acp_runtime default ─────────────────────────────────────────────
        let acp_runtime =
            acp_runtime.unwrap_or_else(|| Arc::new(crate::acp::AcpRuntime::default()));

        // ── mcp_runtime default (CapabilityOnlyAdapter: NoRuntime on tools) ─
        let mcp_runtime = mcp_runtime.unwrap_or_else(|| {
            Arc::new(CapabilityOnlyAdapter::new(Arc::new(
                CapabilityChecker::new(&[]),
            )))
        });

        // ── capability_checker default (resolve from ACP secret chain) ──────
        let capability_checker =
            capability_checker.or_else(|| resolve_acp_secret_for_checker().map(Arc::new));
        if capability_checker.is_none() {
            tracing::info!(
                target: "hkask.ocap",
                "No capability checker configured — PodContext::require_capability() will deny capabilities"
            );
        }

        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            git_cas,
            acp_runtime,
            mcp_runtime,
            episodic_storage,
            semantic_storage,
            inference_port,
            capability_checker,
            governed_tool,
            nu_event_sink,
            consent: Arc::new(crate::DenyAllConsent),
        }
    }

    /// Wire a live `SovereigntyConsent` port into the manager. New pods will
    /// use this port to resolve consent for sovereignty checks. In tests,
    /// pass a `DenyAllConsent` or `AllowAllConsent` to control behavior.
    pub fn with_consent_port(mut self, consent: Arc<dyn crate::SovereigntyConsent>) -> Self {
        self.consent = consent;
        self
    }

    /// Set the capability checker for cryptographic OCAP verification
    pub fn with_capability_checker(mut self, checker: CapabilityChecker) -> Self {
        self.capability_checker = Some(Arc::new(checker));
        self
    }

    /// Set the NuEvent sink for pod lifecycle observability
    pub fn with_nu_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.nu_event_sink = Some(sink);
        self
    }

    /// Set the GovernedTool membrane for pod tool invocations.
    ///
    /// When set, `PodContext::invoke_tool` routes through the membrane,
    /// gaining CNS governance: gas budget enforcement, variety tracking,
    /// and algedonic event spans.
    pub fn with_governed_tool(mut self, tool: Arc<GovernedTool<RawMcpToolPort>>) -> Self {
        self.governed_tool = Some(tool);
        self
    }

    /// Create a new pod manager with inference port
    pub fn with_inference(
        git_cas: Arc<GitCasAdapter>,
        acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        episodic_storage: Arc<dyn EpisodicStoragePort>,
        semantic_storage: Arc<dyn SemanticStoragePort>,
        inference_port: Arc<dyn InferencePort>,
    ) -> Self {
        Self::new(
            Some(git_cas),
            Some(acp_runtime),
            Some(mcp_runtime),
            Some(episodic_storage),
            Some(semantic_storage),
            Some(inference_port),
            None,
            None,
            None,
        )
    }

    /// Get the inference port if available
    pub fn inference_port(&self) -> Option<Arc<dyn InferencePort>> {
        self.inference_port.clone()
    }

    /// Get an `Arc` to the `SovereigntyChecker` for a given pod, if it exists.
    ///
    /// Returns `None` if the pod is not found. Each pod's checker is built
    /// at `AgentPod::new` time, so the same `Arc<dyn SovereigntyConsent>`
    /// is consulted for every pod in the manager.
    pub async fn sovereignty_checker_for(
        &self,
        pod_id: &PodID,
    ) -> Option<Arc<crate::SovereigntyChecker>> {
        let pods = self.pods.read().await;
        pods.get(pod_id)
            .map(|pod| Arc::new(pod.sovereignty_checker.clone()))
    }

    /// Create a new pod manager with mock adapters for testing
    ///
    /// Uses a `CapabilityOnlyAdapter` (no live MCP runtime) so that
    /// capability verification works but tool invocation returns
    /// `McpError::NoRuntime`.
    ///
    /// Uses a deterministic test ACP secret so the mock is self-contained
    /// and does not require `HKASK_ACP_SECRET_KEY` or `HKASK_MASTER_KEY`.
    /// The AcpRuntime and CapabilityChecker share the same secret so tokens
    /// signed by the runtime are verifiable by the checker.
    pub fn new_mock() -> Self {
        // Test-only ACP secret. Never use in production.
        // 32 bytes for HMAC-SHA256 compatibility.
        const MOCK_ACP_SECRET: &[u8] = b"hkask-mock-acp-secret-32-bytes!!";

        let adapter = Arc::new(MemoryLoopAdapter::in_memory_unchecked());
        let episodic_storage: Arc<dyn EpisodicStoragePort> = adapter.clone();
        let semantic_storage: Arc<dyn SemanticStoragePort> = adapter.clone();

        let acp_runtime = Arc::new(crate::acp::AcpRuntime::new(MOCK_ACP_SECRET));
        let capability_checker = Arc::new(CapabilityChecker::new(MOCK_ACP_SECRET));

        // Use CapabilityOnlyAdapter (no live MCP runtime) for the MCP port.
        // Wired with the same test secret so grant_tool_access works.
        let mcp_runtime: Arc<dyn MCPRuntimePort> =
            Arc::new(CapabilityOnlyAdapter::new(Arc::clone(&capability_checker)));

        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            git_cas: Arc::new(GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-mock"))),
            acp_runtime,
            mcp_runtime,
            episodic_storage,
            semantic_storage,
            inference_port: None,
            capability_checker: Some(capability_checker),
            governed_tool: None,
            nu_event_sink: None,
            consent: Arc::new(crate::DenyAllConsent),
        }
    }
}

impl PodManager {
    /// Create a new pod from a template crate
    ///
    /// # Arguments
    /// * `template_name` — Name of the template crate
    /// * `persona` — Agent persona definition
    /// * `name` — Optional pod name (defaults to UUID)
    ///
    /// # Returns
    /// * `Ok(PodID)` — Pod created successfully
    /// * `Err(AgentPodError)` — Failed to create pod
    pub async fn create_pod(
        &self,
        template_name: &str,
        persona: &AgentPersona,
        name: Option<String>,
    ) -> AgentPodResult<PodID> {
        // Validate persona fields
        AgentPersona::validate_fields(
            &persona.agent.name,
            &persona.agent.agent_type.to_string().to_lowercase(),
            &persona.agent.version,
            &persona.charter.description,
            &persona.charter.editor,
            &persona.capabilities,
        )?;

        let pod = AgentPod::new(
            template_name,
            persona,
            self.git_cas.as_ref(),
            // Shared Arc: all pods in the manager consult the same port.
            Arc::clone(&self.consent),
        )?;
        let pod_id = pod.id;

        let mut pods = self.pods.write().await;
        pods.insert(pod_id, pod);

        info!(
            target: "hkask.pod",
            pod_id = %pod_id,
            template = %template_name,
            name = ?name,
            "Pod created"
        );

        Ok(pod_id)
    }

    /// Activate a pod for A2A communication
    pub async fn activate_pod(&self, pod_id: &PodID) -> AgentPodResult<()> {
        // Phase 1: Extract registration data while holding the guard
        let registration_data = {
            let pods = self.pods.read().await;
            let pod = pods
                .get(pod_id)
                .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;

            if pod.state() == PodLifecycleState::Populated {
                Some((pod.webid, pod.agent_type, pod.persona.capabilities.clone()))
            } else {
                None
            }
        }; // Guard dropped here

        // Phase 2: Async ACP registration without holding the lock
        let token = if let Some((webid, agent_type, capabilities)) = registration_data {
            Some(
                self.acp_runtime
                    .register_agent(webid, agent_type, capabilities)
                    .await?,
            )
        } else {
            None
        };

        // Phase 3: Apply result and activate MCP while holding write guard
        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;

        if let Some(token) = token {
            pod.capability_token = token;
            pod.state = PodLifecycleState::Registered;

            tracing::debug!(
                target: "cns.pod",
                span = "cns.agent_pod.registered",
                verb = "registered",
                pod_id = %pod.id,
                webid = %pod.webid,
                agent_type = %pod.agent_type,
                confidence = 1.0,
                "CNS event"
            );

            info!("Agent pod {} registered with ACP", pod.id);

            if let Some(ref sink) = self.nu_event_sink {
                crate::pod::nu_event::emit_pod_registered(
                    sink.as_ref(),
                    pod.webid,
                    &pod.id.to_string(),
                    &pod.agent_type.to_string(),
                );
            }
        }

        pod.activate(self.mcp_runtime.as_ref())?;

        if let Some(ref sink) = self.nu_event_sink {
            crate::pod::nu_event::emit_pod_activated(sink.as_ref(), pod.webid, &pod.id.to_string());
        }

        info!(
            target: "hkask.pod",
            pod_id = %pod_id,
            "Pod activated"
        );

        Ok(())
    }

    /// Deactivate a pod
    pub async fn deactivate_pod(&self, pod_id: &PodID) -> AgentPodResult<()> {
        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;

        let token_id = pod.capability_token.id.clone();
        let webid = pod.webid;

        pod.deactivate()?;

        if let Some(ref sink) = self.nu_event_sink {
            crate::pod::nu_event::emit_pod_deactivated(
                sink.as_ref(),
                pod.webid,
                &pod.id.to_string(),
            );
        }

        // W6: Revoke capability token on deactivation
        if let Err(e) = self.acp_runtime.revoke_capability(&token_id, &webid).await {
            tracing::warn!(
                target: "hkask.pod",
                pod_id = %pod_id,
                token_id = %token_id,
                error = %e,
                "Failed to revoke capability token on deactivation (pod is still deactivated)"
            );
            tracing::debug!(
                target: "cns.pod",
                span = "cns.agent_pod.revocation_warning",
                verb = "revocation_warning",
                pod_id = %pod_id,
                token_id = %token_id,
                error = %e,
                confidence = 0.8,
                "CNS event"
            );
        }

        info!(
            target: "hkask.pod",
            pod_id = %pod_id,
            "Pod deactivated"
        );

        Ok(())
    }

    /// Recall lifecycle events for a pod
    pub async fn recall_pod_events(&self, pod_id: &PodID) -> AgentPodResult<Vec<RecalledEpisode>> {
        let pods = self.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;

        let request = crate::ports::RecallRequest::episodic(
            "lifecycle",
            pod.webid,
            pod.capability_token.clone(),
        );
        self.episodic_storage
            .recall_episodic(&request)
            .map_err(AgentPodError::from)
    }

    /// Get pod status
    pub async fn get_pod_status(&self, pod_id: &PodID) -> AgentPodResult<PodStatus> {
        let pods = self.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;

        Ok(PodStatus {
            pod_id: pod.id.to_string(),
            name: Some(pod.persona.agent.name.clone()),
            state: pod.state,
            webid: pod.webid.to_string(),
            agent_type: pod.agent_type,
            template: pod.template_crate.name.clone(),
            created_at: pod.created_at,
        })
    }

    pub async fn list_pods(&self) -> AgentPodResult<Vec<PodStatus>> {
        let pods = self.pods.read().await;
        let statuses = pods
            .values()
            .map(|pod| PodStatus {
                pod_id: pod.id.to_string(),
                name: Some(pod.persona.agent.name.clone()),
                state: pod.state,
                webid: pod.webid.to_string(),
                agent_type: pod.agent_type,
                template: pod.template_crate.name.clone(),
                created_at: pod.created_at,
            })
            .collect();

        Ok(statuses)
    }

    /// Get a reference to the ACP runtime port
    pub fn acp_runtime(&self) -> Arc<dyn crate::ports::AcpPort + Send + Sync> {
        Arc::clone(&self.acp_runtime)
    }
}

impl Default for PodManager {
    fn default() -> Self {
        Self::new_mock()
    }
}

/// Resolve the ACP secret using the same resolution chain as `AcpRuntime::default()`.
/// Returns `None` if the secret cannot be resolved.
///
/// This allows `PodManager` to construct a `CapabilityChecker` that can verify
/// tokens signed by the default `AcpRuntime`.
fn resolve_acp_secret_for_checker() -> Option<CapabilityChecker> {
    hkask_keystore::resolve_acp_secret()
        .ok()
        .map(|secret| CapabilityChecker::new(&secret))
}
