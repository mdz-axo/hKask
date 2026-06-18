//! PodManager, PodStatus — Pod lifecycle management

use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_types::{CapabilityChecker, InferencePort, NuEventSink, WebID};
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

/// Callback invoked after a pod is successfully activated.
pub type ActivationHook = Box<dyn Fn(WebID, String) + Send + Sync>;

pub struct PodManager {
    pub(crate) pods: Arc<RwLock<HashMap<PodID, AgentPod>>>,
    git_cas: Arc<GitCasAdapter>,
    a2a_runtime: Arc<dyn crate::ports::A2APort + Send + Sync>,
    pub(crate) mcp_runtime: Arc<dyn MCPRuntimePort>,
    pub(crate) episodic_storage: Arc<dyn EpisodicStoragePort>,
    pub(crate) semantic_storage: Arc<dyn SemanticStoragePort>,
    pub(crate) inference_port: Option<Arc<dyn InferencePort>>,
    pub(crate) capability_checker: Option<Arc<CapabilityChecker>>,
    pub(crate) governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
    nu_event_sink: Option<Arc<dyn NuEventSink>>,
    consent: Arc<dyn crate::SovereigntyConsent>,
    /// Hooks called after a pod is successfully activated.
    /// Each hook receives the pod's WebID and display name.
    activation_hooks: RwLock<Vec<ActivationHook>>,
}

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

impl PodStatus {
    fn from_pod(pod: &AgentPod) -> Self {
        Self {
            pod_id: pod.id.to_string(),
            name: Some(pod.persona.agent.name.clone()),
            state: pod.state,
            webid: pod.webid.to_string(),
            agent_type: pod.agent_type,
            template: pod.template_crate.name.clone(),
            created_at: pod.created_at,
        }
    }
}

impl PodManager {
    #[allow(clippy::too_many_arguments)]
    /// REQ: P1-agt-pod-manager-new
    /// \[P1\] Motivating: User Sovereignty — PodManager orchestrates user agent pods
    /// \[P4\] Constraining: Clear Boundaries — default DenyAllConsent
    /// pre:  All `Option` arguments may be `Some` or `None`; defaults are
    ///       provided for absent ports.
    /// post: Returns a `PodManager` with all ports set (or defaulted),
    ///       `DenyAllConsent` as the default consent policy, and empty
    ///       pod registry and activation hooks.
    pub fn new(
        git_cas: Option<Arc<GitCasAdapter>>,
        a2a_runtime: Option<Arc<dyn crate::ports::A2APort + Send + Sync>>,
        mcp_runtime: Option<Arc<dyn MCPRuntimePort>>,
        episodic_storage: Option<Arc<dyn EpisodicStoragePort>>,
        semantic_storage: Option<Arc<dyn SemanticStoragePort>>,
        inference_port: Option<Arc<dyn InferencePort>>,
        capability_checker: Option<Arc<CapabilityChecker>>,
        governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
        nu_event_sink: Option<Arc<dyn NuEventSink>>,
    ) -> Self {
        if episodic_storage.is_none() || semantic_storage.is_none() {
            tracing::warn!(target: "hkask.agents.pod",
                "No persistent storage configured — episodic and semantic memory are in-memory and will be lost on restart. \
                 Use PodManager::new() with explicit storage ports for sovereign persistence.");
        }
        let default_adapter = Arc::new(MemoryLoopAdapter::in_memory_unchecked());
        let capability_checker =
            capability_checker.or_else(|| resolve_a2a_secret_for_checker().map(Arc::new));
        if capability_checker.is_none() {
            tracing::info!(target: "hkask.ocap",
                "No capability checker configured — PodContext::require_capability() will deny capabilities");
        }
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            git_cas: git_cas.unwrap_or_else(|| {
                Arc::new(GitCasAdapter::from_path(PathBuf::from(
                    "./registry/templates",
                )))
            }),
            a2a_runtime: a2a_runtime.unwrap_or_else(|| Arc::new(crate::a2a::A2ARuntime::default())),
            mcp_runtime: mcp_runtime.unwrap_or_else(|| {
                Arc::new(CapabilityOnlyAdapter::new(Arc::new(
                    CapabilityChecker::new(&[]),
                )))
            }),
            episodic_storage: episodic_storage.unwrap_or(default_adapter.clone()),
            semantic_storage: semantic_storage.unwrap_or(default_adapter),
            inference_port,
            capability_checker,
            governed_tool,
            nu_event_sink,
            consent: Arc::new(crate::DenyAllConsent),
            activation_hooks: RwLock::new(Vec::new()),
        }
    }

    /// REQ: P1-agt-pod-manager-with-consent
    /// \[P2\] Constraining: Affirmative Consent — replace default consent policy
    /// pre:  `consent` is a valid `Arc<dyn SovereigntyConsent>`.
    /// post: Returns `self` with `consent` updated.
    pub fn with_consent_port(mut self, consent: Arc<dyn crate::SovereigntyConsent>) -> Self {
        self.consent = consent;
        self
    }

    /// Register a hook to be called after every successful pod activation.
    ///
    /// The hook receives the pod's WebID and display name. Use this for
    /// \[NORMATIVE\] cross-cutting concerns like Matrix registration that should happen (P6 — Space for Replicants).
    /// whenever a pod becomes active.
    ///
    /// REQ: P1-agt-pod-manager-activation-hook
    /// \[P3\] Motivating: Generative Space — hook runs when pod becomes active
    /// pre:  `hook` is a valid boxed closure.
    /// post: The hook is appended to the activation hooks list.
    pub async fn register_activation_hook(&self, hook: Box<dyn Fn(WebID, String) + Send + Sync>) {
        self.activation_hooks.write().await.push(hook);
    }
    /// REQ: P1-agt-pod-manager-with-checker
    /// \[P4\] Constraining: Clear Boundaries — set capability checker
    /// pre:  `checker` is a valid `CapabilityChecker`.
    /// post: Returns `self` with `capability_checker` set to
    ///       `Some(Arc::new(checker))`.
    pub fn with_capability_checker(mut self, checker: CapabilityChecker) -> Self {
        self.capability_checker = Some(Arc::new(checker));
        self
    }
    /// REQ: P1-agt-pod-manager-with-sink
    /// \[P9\] Motivating: Homeostatic Self-Regulation — attach CNS event sink
    /// pre:  `sink` is a valid `Arc<dyn NuEventSink>`.
    /// post: Returns `self` with `nu_event_sink` set to `Some(sink)`.
    pub fn with_nu_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.nu_event_sink = Some(sink);
        self
    }
    /// REQ: P1-agt-pod-manager-with-governed-tool
    /// \[P4\] Constraining: Clear Boundaries — wire governed tool for capability gating
    /// pre:  `tool` is a valid `Arc<GovernedTool<RawMcpToolPort>>`.
    /// post: Returns `self` with `governed_tool` set to `Some(tool)`.
    pub fn with_governed_tool(mut self, tool: Arc<GovernedTool<RawMcpToolPort>>) -> Self {
        self.governed_tool = Some(tool);
        self
    }

    /// REQ: P1-agt-pod-manager-with-ports
    /// \[P1\] Motivating: User Sovereignty — configure runtime ports for pods
    /// pre:  All arguments are valid, non-null `Arc`s.
    /// post: Returns a `PodManager` with all ports set (no optional
    ///       defaults), `DenyAllConsent`, and empty pod registry.
    pub fn with_inference(
        git_cas: Arc<GitCasAdapter>,
        a2a_runtime: Arc<dyn crate::ports::A2APort + Send + Sync>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        episodic_storage: Arc<dyn EpisodicStoragePort>,
        semantic_storage: Arc<dyn SemanticStoragePort>,
        inference_port: Arc<dyn InferencePort>,
    ) -> Self {
        Self::new(
            Some(git_cas),
            Some(a2a_runtime),
            Some(mcp_runtime),
            Some(episodic_storage),
            Some(semantic_storage),
            Some(inference_port),
            None,
            None,
            None,
        )
    }

    /// REQ: P1-agt-pod-manager-inference-port
    /// \[P8\] Motivating: Semantic Grounding — accessor for inference port
    /// pre:  (none — accessor).
    /// post: Returns a clone of the inner `Option<Arc<dyn InferencePort>>`.
    pub fn inference_port(&self) -> Option<Arc<dyn InferencePort>> {
        self.inference_port.clone()
    }

    /// REQ: P1-agt-pod-manager-sovereignty-checker
    /// \[P1\] Motivating: User Sovereignty — get per-pod sovereignty checker
    /// pre:  `pod_id` is a valid `PodID`.
    /// post: Returns `Some(Arc<SovereigntyChecker>)` if the pod exists;
    ///       `None` otherwise.
    pub async fn sovereignty_checker_for(
        &self,
        pod_id: &PodID,
    ) -> Option<Arc<crate::SovereigntyChecker>> {
        self.pods
            .read()
            .await
            .get(pod_id)
            .map(|pod| Arc::new(pod.sovereignty_checker.clone()))
    }

    /// Create a mock PodManager for testing.
    ///
    /// If `inference_port` is provided, agent pods can perform inference
    /// (e.g., improv interactions). Default: no inference (pods are orchestration-only).
    ///
    /// REQ: P1-agt-pod-manager-default
    /// \[P5\] Motivating: Essentialism — default manager with in-memory mocks
    /// pre:  `inference_port` is `Some` or `None`.
    /// post: Returns a `PodManager` with in-memory storage, a mock A2A
    ///       runtime, and `DenyAllConsent`.
    pub fn new_mock(inference_port: Option<Arc<dyn InferencePort>>) -> Self {
        const MOCK_A2A_SECRET: &[u8] = b"xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX";
        let adapter = Arc::new(MemoryLoopAdapter::in_memory_unchecked());
        let capability_checker = Arc::new(CapabilityChecker::new(MOCK_A2A_SECRET));
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            git_cas: Arc::new(GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-mock"))),
            a2a_runtime: Arc::new(crate::a2a::A2ARuntime::new(MOCK_A2A_SECRET)),
            mcp_runtime: Arc::new(CapabilityOnlyAdapter::new(Arc::clone(&capability_checker))),
            episodic_storage: adapter.clone(),
            semantic_storage: adapter,
            inference_port,
            capability_checker: Some(capability_checker),
            governed_tool: None,
            nu_event_sink: None,
            consent: Arc::new(crate::DenyAllConsent),
            activation_hooks: RwLock::new(Vec::new()),
        }
    }
}

impl PodManager {
    /// REQ: P1-agt-pod-manager-create-pod
    /// \[P1\] Motivating: User Sovereignty — create a new agent pod from template + persona
    /// pre:  `template_name` is a non-empty string; `persona` is a valid
    ///       `AgentPersona` with validated fields; `name` is `Some` or
    ///       `None`.
    /// post: Returns `Ok(PodID)` — the new pod's ID — after creating and
    ///       inserting the pod into the registry. Returns `Err` if
    ///       validation or pod creation fails.
    pub async fn create_pod(
        &self,
        template_name: &str,
        persona: &AgentPersona,
        name: Option<String>,
    ) -> AgentPodResult<PodID> {
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
            Arc::clone(&self.consent),
        )?;
        let pod_id = pod.id;
        self.pods.write().await.insert(pod_id, pod);
        info!(target: "hkask.pod", pod_id = %pod_id, template = %template_name, name = ?name, "Pod created");
        Ok(pod_id)
    }

    /// REQ: P1-agt-pod-manager-activate-pod
    /// \[P1\] Motivating: User Sovereignty — activate pod (register + grant MCP)
    /// pre:  `pod_id` is a valid `PodID` referencing an existing pod.
    /// post: If the pod is `Populated`, registers it with A2A, then
    ///       activates it via MCP. Runs activation hooks on success.
    ///       Returns `Ok(())` or `Err` on failure.
    pub async fn activate_pod(&self, pod_id: &PodID) -> AgentPodResult<()> {
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
        };
        let token = if let Some((webid, agent_type, capabilities)) = registration_data {
            Some(
                self.a2a_runtime
                    .register_agent(webid, agent_type, capabilities)
                    .await
                    .map_err(|e| AgentPodError::A2ARegistrationError(e.to_string()))?,
            )
        } else {
            None
        };

        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;
        if let Some(token) = token {
            pod.capability_token = token;
            pod.state = PodLifecycleState::Registered;
            tracing::debug!(target: "cns.pod", span = "cns.agent_pod.registered", verb = "registered",
                pod_id = %pod.id, webid = %pod.webid, agent_type = %pod.agent_type, confidence = 1.0, "CNS event");
            info!("Agent pod {} registered with A2A", pod.id);
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
        info!(target: "hkask.pod", pod_id = %pod_id, "Pod activated");

        // Run activation hooks (e.g., Matrix registration)
        let pod_name = pod.persona.agent.name.clone();
        let pod_webid = pod.webid;
        let hooks = self.activation_hooks.read().await;
        for hook in hooks.iter() {
            hook(pod_webid, pod_name.clone());
        }

        Ok(())
    }

    /// REQ: P1-agt-pod-manager-deactivate-pod
    /// \[P1\] Motivating: User Sovereignty — deactivate pod and revoke capabilities
    /// pre:  `pod_id` is a valid `PodID` referencing an existing pod.
    /// post: Deactivates the pod and attempts to revoke its capability
    ///       token; logs a warning if revocation fails (pod is still
    ///       deactivated). Returns `Ok(())` or `Err` on failure.
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
        if let Err(e) = self
            .a2a_runtime()
            .revoke_capability(&token_id, &webid)
            .await
        {
            tracing::warn!(target: "hkask.pod", pod_id = %pod_id, token_id = %token_id, error = %e,
                "Failed to revoke capability token on deactivation (pod is still deactivated)");
            tracing::debug!(target: "cns.pod", span = "cns.agent_pod.revocation_warning",
                verb = "revocation_warning", pod_id = %pod_id, token_id = %token_id, error = %e,
                confidence = 0.8, "CNS event");
        }
        info!(target: "hkask.pod", pod_id = %pod_id, "Pod deactivated");
        Ok(())
    }

    /// REQ: P1-agt-pod-manager-recall-lifecycle
    /// \[P3\] Motivating: Generative Space — recall pod lifecycle episodes
    /// pre:  `pod_id` is a valid `PodID` referencing an existing pod.
    /// post: Returns `Ok(Vec<RecalledEpisode>)` with lifecycle events
    ///       for the pod; `Err` if the pod is not found or recall fails.
    pub async fn recall_pod_events(&self, pod_id: &PodID) -> AgentPodResult<Vec<RecalledEpisode>> {
        let pods = self.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;
        self.episodic_storage
            .recall_episodic(&crate::ports::RecallRequest::episodic(
                "lifecycle",
                pod.webid,
                pod.capability_token.clone(),
            ))
            .map_err(AgentPodError::from)
    }

    /// REQ: P1-agt-pod-manager-status
    /// \[P8\] Motivating: Semantic Grounding — get pod status
    /// pre:  `pod_id` is a valid `PodID`.
    /// post: Returns `Ok(PodStatus)` if the pod exists; `Err(PodNotFound)`
    ///       otherwise.
    pub async fn get_pod_status(&self, pod_id: &PodID) -> AgentPodResult<PodStatus> {
        self.pods
            .read()
            .await
            .get(pod_id)
            .map(PodStatus::from_pod)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))
    }

    /// REQ: P1-agt-pod-manager-list-status
    /// \[P8\] Motivating: Semantic Grounding — list all pod statuses
    /// pre:  (none).
    /// post: Returns `Ok(Vec<PodStatus>)` with status for all registered
    ///       pods (may be empty).
    pub async fn list_pods(&self) -> AgentPodResult<Vec<PodStatus>> {
        Ok(self
            .pods
            .read()
            .await
            .values()
            .map(PodStatus::from_pod)
            .collect())
    }

    /// REQ: P1-agt-pod-manager-a2a-port
    /// \[P4\] Constraining: Clear Boundaries — accessor for A2A port
    /// pre:  (none — accessor).
    /// post: Returns a clone of the inner `Arc<dyn A2APort + Send + Sync>`.
    pub fn a2a_runtime(&self) -> Arc<dyn crate::ports::A2APort + Send + Sync> {
        Arc::clone(&self.a2a_runtime)
    }

    // ── Daemon-oriented accessors ──

    /// Find a pod ID by replicant name (matches persona.agent.name).
    ///
    /// REQ: P1-agt-pod-manager-find-by-name
    /// \[P8\] Motivating: Semantic Grounding — lookup pod by replicant name
    /// pre:  `name` is a non-empty string.
    /// post: Returns `Some(PodID)` if a pod with matching name exists;
    ///       `None` otherwise.
    pub async fn find_pod_by_name(&self, name: &str) -> Option<PodID> {
        let pods = self.pods.read().await;
        for (id, pod) in pods.iter() {
            if pod.persona.agent.name == name {
                return Some(*id);
            }
        }
        None
    }

    /// Get the WebID for a pod.
    ///
    /// REQ: P1-agt-pod-manager-webid
    /// \[P1\] Motivating: User Sovereignty — get pod's WebID
    /// pre:  `pod_id` is a valid `PodID`.
    /// post: Returns `Some(WebID)` if the pod exists; `None` otherwise.
    pub async fn get_pod_webid(&self, pod_id: &PodID) -> Option<WebID> {
        self.pods.read().await.get(pod_id).map(|p| p.webid)
    }

    /// Check if a pod is assigned to a specific MCP role.
    ///
    /// REQ: P1-agt-pod-manager-has-role
    /// \[P4\] Constraining: Clear Boundaries — check MCP role assignment
    /// pre:  `pod_id` is a valid `PodID`; `role` is a non-empty string.
    /// post: Returns `true` if the pod exists and has the role assigned;
    ///       `false` otherwise (including if pod not found).
    pub async fn is_assigned_to_role(&self, pod_id: &PodID, role: &str) -> bool {
        self.pods
            .read()
            .await
            .get(pod_id)
            .map(|p| p.assigned_mcp_roles.iter().any(|r| r == role))
            .unwrap_or(false)
    }

    /// Check if a pod holds a specific capability.
    /// Capabilities are stored as strings like "web_search:execute" or "web_search".
    ///
    /// REQ: P1-agt-pod-manager-has-capability
    /// \[P4\] Constraining: Clear Boundaries — check tool capability
    /// pre:  `pod_id` is a valid `PodID`; `tool` is a non-empty string.
    /// post: Returns `true` if the pod exists and has a capability
    ///       matching `tool` (exact or prefix match with `:` separator);
    ///       `false` otherwise.
    pub async fn has_capability(&self, pod_id: &PodID, tool: &str) -> bool {
        self.pods
            .read()
            .await
            .get(pod_id)
            .map(|p| {
                p.persona
                    .capabilities
                    .iter()
                    .any(|cap| cap == tool || cap.starts_with(&format!("{}:", tool)))
            })
            .unwrap_or(false)
    }

    /// Assign an MCP role to a pod by name.
    ///
    /// REQ: P1-agt-pod-manager-assign-role
    /// \[P1\] Motivating: User Sovereignty — assign MCP role to named pod
    /// pre:  `name` is a non-empty string matching an existing pod;
    ///       `role` is a non-empty string.
    /// post: If the pod exists and doesn't already have the role, it is
    ///       added to `assigned_mcp_roles`. Returns `Ok(())` or `Err` if
    ///       pod not found.
    pub async fn assign_role(&self, name: &str, role: &str) -> AgentPodResult<()> {
        let pod_id = self.find_pod_by_name(name).await.ok_or_else(|| {
            AgentPodError::PersonaParseError(format!("No pod found for replicant '{}'", name))
        })?;
        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(&pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(pod_id))?;
        if !pod.assigned_mcp_roles.iter().any(|r| r == role) {
            pod.assigned_mcp_roles.push(role.to_string());
            tracing::info!(target: "hkask.pod", pod_id = %pod_id, name = %name, role = %role, "MCP role assigned");
        }
        Ok(())
    }

    /// Set the agent mode for a pod by name.
    /// Mode can be "server" (with role), "chat", or "exit".
    ///
    /// REQ: P1-agt-pod-manager-set-mode
    /// \[P1\] Motivating: User Sovereignty — set pod mode (server/chat/exit)
    /// pre:  `name` is a non-empty string matching an existing pod;
    ///       `mode` is "server", "chat", or "exit"; `role` is required
    ///       for "server" mode.
    /// post: Transitions the pod to the requested mode. Returns `Ok(())`
    ///       or `Err` if pod not found or mode transition fails.
    pub async fn set_mode(&self, name: &str, mode: &str, role: Option<&str>) -> AgentPodResult<()> {
        let pod_id = self.find_pod_by_name(name).await.ok_or_else(|| {
            AgentPodError::PersonaParseError(format!("No pod found for replicant '{}'", name))
        })?;
        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(&pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(pod_id))?;
        match mode {
            "server" => {
                let r = role.ok_or_else(|| {
                    AgentPodError::PersonaParseError("role required for server mode".to_string())
                })?;
                pod.enter_server_mode(r)?;
            }
            "chat" => pod.enter_chat_mode()?,
            "exit" => pod.exit_mode()?,
            other => {
                return Err(AgentPodError::PersonaParseError(format!(
                    "Unknown mode: {}",
                    other
                )));
            }
        }
        Ok(())
    }
}

impl Default for PodManager {
    fn default() -> Self {
        Self::new_mock(None)
    }
}

fn resolve_a2a_secret_for_checker() -> Option<CapabilityChecker> {
    hkask_keystore::keychain::resolve_a2a_secret()
        .ok()
        .map(|secret| CapabilityChecker::new(&secret))
}
