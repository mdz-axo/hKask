//! ActivePods — Runtime registry for active pod deployments.
//!
//! Stores PodFactory + port references so the API matches
//! the old PodManager signature exactly. Zero consumer changes needed.

use super::AgentPodError;
use super::context::PodContext;
use super::deployment::{PodDeployment, PodFactory, PodRegistry};
use super::types::{AgentKind, AgentPersona, PodID, PodKind, PodLifecycleState};
use crate::curator::SemanticIndex;
use crate::ports::{A2APort, EpisodicStoragePort, MCPRuntimePort, SemanticStoragePort};
use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_types::{CapabilityChecker, InferencePort, NuEventSink, WebID};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ActivePods {
    deployments: RwLock<HashMap<PodID, PodDeployment>>,
    factory: Option<Arc<PodFactory>>,
    a2a_runtime: Option<Arc<dyn A2APort + Send + Sync>>,
    mcp_runtime: Option<Arc<dyn MCPRuntimePort>>,
    governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
    capability_checker: Option<Arc<CapabilityChecker>>,
    nu_event_sink: Option<Arc<dyn NuEventSink>>,
    episodic_adapter: Option<Arc<dyn EpisodicStoragePort>>,
    semantic_adapter: Option<Arc<dyn SemanticStoragePort>>,
    inference_port: Option<Arc<dyn InferencePort>>,
    /// CuratorPod's SemanticIndex — shared with all pod contexts for
    /// merged-lens semantic recall (Step 5).
    curator_index: RwLock<Option<Arc<std::sync::RwLock<SemanticIndex>>>>,
}

impl ActivePods {
    pub fn new() -> Self {
        Self {
            deployments: RwLock::new(HashMap::new()),
            factory: None,
            a2a_runtime: None,
            mcp_runtime: None,
            governed_tool: None,
            capability_checker: None,
            nu_event_sink: None,
            episodic_adapter: None,
            semantic_adapter: None,
            inference_port: None,
            curator_index: RwLock::new(None),
        }
    }

    /// Create a mock ActivePods for testing — matches old PodManager::new_mock().
    /// Wires in-memory adapters and a test factory so create_pod/activate_pod work.
    /// Full test harness with in-memory adapters, AllowAllConsent,
    /// mock templates, and master key set. One call sets up everything
    /// needed for integration tests.
    pub fn new_test_harness(data_dir: &std::path::Path) -> Self {
        // Set test master key for ADR-027 key derivation
        unsafe {
            std::env::set_var("HKASK_MASTER_KEY",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        }
        // Create mock template directories
        let tmpl = data_dir.join("templates");
        let persona_yaml = "agent:\n  name: test\n  type: Bot\n  version: \"0.1.0\"\ncharter:\n  description: Test\n  editor: test\n";
        for name in &["curator", "replicant", "team", "solo"] {
            let dir = tmpl.join(name);
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("agent_persona.yaml"), persona_yaml);
            let _ = std::fs::write(dir.join("dispatch_manifest.yaml"), "selector: test\n");
        }
        Self::new_test_harness_inner(data_dir)
    }

    /// Inner test harness — shared with new_mock for backward compatibility.
    fn new_test_harness_inner(data_dir: &std::path::Path) -> Self {
        use crate::a2a::A2ARuntime;
        use crate::adapters::mcp_runtime::CapabilityOnlyAdapter;
        use crate::adapters::memory_loop_adapter::MemoryLoopAdapter;
        use crate::pod::PodFactory;
        use crate::AllowAllConsent;
        use hkask_types::CapabilityChecker;

        let adapter = Arc::new(MemoryLoopAdapter::in_memory_unchecked());
        let mcp = Arc::new(CapabilityOnlyAdapter::new(Arc::new(CapabilityChecker::new(b"mock"))));
        let a2a = Arc::new(A2ARuntime::new(b"mock"));
        let factory = Arc::new(PodFactory::new(
            Arc::new(hkask_mcp::GitCasAdapter::from_path(data_dir.join("templates"))),
            Arc::new(AllowAllConsent),
            data_dir.to_path_buf(),
        ));
        Self::new()
            .with_a2a_runtime(a2a)
            .with_factory_and_ports(
                factory, mcp.clone(), None, None, None,
                adapter.clone() as Arc<dyn EpisodicStoragePort>,
                adapter as Arc<dyn SemanticStoragePort>,
            )
    }

    /// Legacy mock — delegates to new_test_harness_inner with /tmp path.
    pub fn new_mock() -> Self {
        Self::new_test_harness_inner(&std::path::PathBuf::from("/tmp/hkask-mock"))
    }

    /// Wire the factory and port adapters so create_pod/activate_pod work
    /// with the simple old PodManager-style signatures.
    pub fn with_factory_and_ports(
        mut self,
        factory: Arc<PodFactory>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
        capability_checker: Option<Arc<CapabilityChecker>>,
        nu_event_sink: Option<Arc<dyn NuEventSink>>,
        episodic_adapter: Arc<dyn EpisodicStoragePort>,
        semantic_adapter: Arc<dyn SemanticStoragePort>,
    ) -> Self {
        self.factory = Some(factory);
        self.mcp_runtime = Some(mcp_runtime);
        self.governed_tool = governed_tool;
        self.capability_checker = capability_checker;
        self.nu_event_sink = nu_event_sink;
        self.episodic_adapter = Some(episodic_adapter);
        self.semantic_adapter = Some(semantic_adapter);
        self
    }

    /// Wire the A2A runtime for pod registration.
    pub fn with_a2a_runtime(mut self, a2a: Arc<dyn A2APort + Send + Sync>) -> Self {
        self.a2a_runtime = Some(a2a);
        self
    }

    /// Set the inference port for pods to use.
    #[must_use = "builder methods return self for chaining"]
    pub fn with_inference_port(mut self, port: Arc<dyn InferencePort>) -> Self {
        self.inference_port = Some(port);
        self
    }

    /// Get the inference port, if one is wired.
    pub fn inference_port(&self) -> Option<Arc<dyn InferencePort>> {
        self.inference_port.clone()
    }

    /// Get a clone of the Curator's shared SemanticIndex, if a CuratorPod
    /// has been deployed. Used to construct CuratorSync at startup.
    pub async fn curator_index(&self) -> Option<Arc<std::sync::RwLock<SemanticIndex>>> {
        self.curator_index.read().await.clone()
    }

    pub async fn insert(&self, deployment: PodDeployment) {
        self.deployments
            .write()
            .await
            .insert(deployment.pod_id, deployment);
    }

    pub async fn remove(&self, pod_id: &PodID) -> Option<PodDeployment> {
        self.deployments.write().await.remove(pod_id)
    }

    /// Get a PodContext for an active pod.
    /// Wires the Curator's SemanticIndex for merged-lens semantic recall (Step 5).
    pub async fn context(&self, pod_id: &PodID) -> Result<PodContext, AgentPodError> {
        let deployments = self.deployments.read().await;
        let deployment = deployments
            .get(pod_id)
            .ok_or(AgentPodError::PodNotFound(*pod_id))?;
        let mut ctx = PodContext::from_deployment(deployment)?;
        // Wire curator index for merged-lens semantic recall (Step 5)
        if let Some(ref curator) = *self.curator_index.read().await {
            ctx = ctx.with_curator_index(Arc::clone(curator));
        }
        Ok(ctx)
    }

    /// Find a pod by replicant name — matches old PodManager::find_pod_by_name.
    pub async fn find_by_name(&self, name: &str) -> Option<PodID> {
        let deployments = self.deployments.read().await;
        for (id, d) in deployments.iter() {
            if d.pod.persona.agent.name == name {
                return Some(*id);
            }
        }
        None
    }

    /// Alias: matches old PodManager API.
    pub async fn find_pod_by_name(&self, name: &str) -> Option<PodID> {
        self.find_by_name(name).await
    }

    /// Get a pod's WebID — matches old PodManager::get_pod_webid.
    pub async fn get_pod_webid(&self, pod_id: &PodID) -> Option<WebID> {
        self.deployments
            .read()
            .await
            .get(pod_id)
            .map(|d| d.pod.webid)
    }

    /// Alias for get_pod_webid.
    pub async fn webid(&self, pod_id: &PodID) -> Option<WebID> {
        self.get_pod_webid(pod_id).await
    }

    /// Alias for has_role — matches old PodManager::is_assigned_to_role.
    pub async fn is_assigned_to_role(&self, pod_id: &PodID, role: &str) -> bool {
        self.deployments
            .read()
            .await
            .get(pod_id)
            .map(|d| d.pod.assigned_mcp_roles.iter().any(|r| r == role))
            .unwrap_or(false)
    }

    /// Alias for is_assigned_to_role.
    pub async fn has_role(&self, pod_id: &PodID, role: &str) -> bool {
        self.is_assigned_to_role(pod_id, role).await
    }

    pub async fn has_capability(&self, pod_id: &PodID, tool: &str) -> bool {
        self.deployments
            .read()
            .await
            .get(pod_id)
            .map(|d| {
                d.pod
                    .persona
                    .capabilities
                    .iter()
                    .any(|cap| cap == tool || cap.starts_with(&format!("{}:", tool)))
            })
            .unwrap_or(false)
    }

    /// Create a pod — matches old PodManager::create_pod(template, persona, name).
    /// Uses internally stored PodFactory and port adapters.
    pub async fn create_pod(
        &self,
        template_name: &str,
        persona: &AgentPersona,
        _name: Option<String>,
        pod_kind: PodKind,
    ) -> Result<PodID, AgentPodError> {
        let factory = self.factory.as_ref().ok_or_else(|| {
            AgentPodError::PersonaParseError("ActivePods not wired with PodFactory".into())
        })?;
        let mcp = Arc::clone(self.mcp_runtime.as_ref().ok_or_else(|| {
            AgentPodError::PersonaParseError("ActivePods not wired with MCP runtime".into())
        })?);
        // Enforce CuratorPod singleton (P5 Essentialism)
        if pod_kind == PodKind::Curator {
            let ci = self.curator_index.read().await;
            if ci.is_some() {
                return Err(AgentPodError::PersonaParseError(
                    "CuratorPod already exists — only one CuratorPod per system".into(),
                ));
            }
        }
        let deployment = factory
            .deploy(
                template_name,
                persona,
                pod_kind,
                mcp,
                self.governed_tool.clone(),
                self.capability_checker.clone(),
                self.nu_event_sink.clone(),
                self.inference_port.clone(),
            )
            .await
            .map_err(|e| AgentPodError::PersonaParseError(e.to_string()))?;
        let pod_id = deployment.pod_id;

        // Step 5: If this is a CuratorPod, wire its SemanticIndex as the
        // shared curator_index that all PodContexts use for merged-lens recall.
        // The sync loop (Step 4) writes to the same Arc<RwLock<>> that PodContext reads from.
        if pod_kind == PodKind::Curator {
            if let Some(ref index) = deployment.semantic_index {
                let mut ci = self.curator_index.write().await;
                *ci = Some(Arc::clone(index));
            }
        }

        self.insert(deployment).await;
        Ok(pod_id)
    }

    /// Ensure a CuratorPod exists, is activated, and has CuratorSync running.
    /// Idempotent — if a Curator already exists, returns its SemanticIndex.
    /// If not, creates one, activates it, spawns the sync loop.
    ///
    /// Returns the shared SemanticIndex Arc for consumers that need it.
    /// The sync loop runs as a background task until the cancellation token fires.
    pub async fn ensure_curator(
        &self,
        data_dir: std::path::PathBuf,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Result<Option<Arc<std::sync::RwLock<SemanticIndex>>>, AgentPodError> {
        // Check if curator already exists
        {
            let ci = self.curator_index.read().await;
            if let Some(ref index) = *ci {
                return Ok(Some(Arc::clone(index)));
            }
        }

        // Create CuratorPod
        let curator_persona =
            super::types::AgentPersona::system("curator", hkask_types::AgentKind::Bot);
        let pod_id = self
            .create_pod("curator", &curator_persona, None, PodKind::Curator)
            .await?;

        // Activate it
        self.activate_pod(&pod_id).await?;

        // Extract the SemanticIndex (set by create_pod when PodKind::Curator)
        let index = {
            let ci = self.curator_index.read().await;
            ci.clone().ok_or_else(|| {
                AgentPodError::PersonaParseError(
                    "CuratorPod created but SemanticIndex missing".into(),
                )
            })?
        };

        // Spawn CuratorSync background loop
        let registry = Arc::new(PodRegistry::new(&data_dir));
        let sync = crate::curator::CuratorSync::new(Arc::clone(&index), data_dir, registry);
        tokio::spawn(async move {
            sync.run(cancel).await;
        });
        // Keep handle alive so the task isn't cancelled by drop
        tracing::info!("CuratorSync spawned — polling semantic triples from all pods");

        Ok(Some(index))
    }

    /// Activate a pod — matches old PodManager::activate_pod(id).
    /// Handles full lifecycle: Populated → Registered → Activated.
    pub async fn activate_pod(&self, pod_id: &PodID) -> Result<(), AgentPodError> {
        let mcp = self.mcp_runtime.as_ref().ok_or_else(|| {
            AgentPodError::PersonaParseError("ActivePods not wired with MCP runtime".into())
        })?;
        let a2a = self.a2a_runtime.as_ref().ok_or_else(|| {
            AgentPodError::PersonaParseError("ActivePods not wired with A2A runtime".into())
        })?;

        // Register with A2A if still Populated
        let registration_data = {
            let d = self.deployments.read().await;
            let d = d.get(pod_id).ok_or(AgentPodError::PodNotFound(*pod_id))?;
            if d.pod.state == PodLifecycleState::Populated {
                Some((
                    d.pod.webid,
                    d.pod.agent_type,
                    d.pod.persona.capabilities.clone(),
                ))
            } else {
                None
            }
        };

        // Perform A2A registration outside the write lock
        let token = if let Some((webid, agent_type, capabilities)) = registration_data {
            Some(
                a2a.register_agent(webid, agent_type, capabilities)
                    .await
                    .map_err(|e| AgentPodError::A2ARegistrationError(e.to_string()))?,
            )
        } else {
            None
        };

        // Apply registration token + activate
        let mut d = self.deployments.write().await;
        let d = d
            .get_mut(pod_id)
            .ok_or(AgentPodError::PodNotFound(*pod_id))?;
        if let Some(token) = token {
            d.pod.capability_token = token;
            d.pod.state = PodLifecycleState::Registered;
        }
        d.pod.activate(mcp.as_ref())
    }

    /// Deactivate a pod — matches old PodManager::deactivate_pod(id).
    pub async fn deactivate_pod(&self, pod_id: &PodID) -> Result<(), AgentPodError> {
        let mut d = self.deployments.write().await;
        d.get_mut(pod_id)
            .ok_or(AgentPodError::PodNotFound(*pod_id))?
            .pod
            .deactivate()
    }

    /// Get pod status — matches old PodManager::get_pod_status(id).
    pub async fn get_pod_status(&self, pod_id: &PodID) -> Result<PodStatusInfo, AgentPodError> {
        let d = self.deployments.read().await;
        let d = d.get(pod_id).ok_or(AgentPodError::PodNotFound(*pod_id))?;
        Ok(PodStatusInfo {
            pod_id: d.pod_id.to_string(),
            name: Some(d.pod.persona.agent.name.clone()),
            state: d.pod.state,
            webid: d.pod.webid.to_string(),
            agent_type: d.pod.agent_type,
            template: d.pod.template_crate.name.clone(),
            pod_kind: d.pod_kind,
            created_at: d.pod.created_at,
        })
    }

    /// List all pods — matches old PodManager::list_pods().
    pub async fn list_pods(&self) -> Result<Vec<PodStatusInfo>, AgentPodError> {
        self.deployments
            .read()
            .await
            .values()
            .map(|d| {
                Ok(PodStatusInfo {
                    pod_id: d.pod_id.to_string(),
                    name: Some(d.pod.persona.agent.name.clone()),
                    state: d.pod.state,
                    webid: d.pod.webid.to_string(),
                    agent_type: d.pod.agent_type,
                    template: d.pod.template_crate.name.clone(),
                    pod_kind: d.pod_kind,
                    created_at: d.pod.created_at,
                })
            })
            .collect()
    }

    pub async fn assign_role(&self, name: &str, role: &str) -> Result<(), AgentPodError> {
        let pod_id = self.find_by_name(name).await.ok_or_else(|| {
            AgentPodError::PersonaParseError(format!("No pod found for replicant '{}'", name))
        })?;
        let mut d = self.deployments.write().await;
        let d = d
            .get_mut(&pod_id)
            .ok_or(AgentPodError::PodNotFound(pod_id))?;
        if !d.pod.assigned_mcp_roles.iter().any(|r| r == role) {
            d.pod.assigned_mcp_roles.push(role.to_string());
        }
        Ok(())
    }

    pub async fn set_mode(
        &self,
        name: &str,
        mode: &str,
        role: Option<&str>,
    ) -> Result<(), AgentPodError> {
        let pod_id = self.find_by_name(name).await.ok_or_else(|| {
            AgentPodError::PersonaParseError(format!("No pod found for replicant '{}'", name))
        })?;
        let mut d = self.deployments.write().await;
        let d = d
            .get_mut(&pod_id)
            .ok_or(AgentPodError::PodNotFound(pod_id))?;
        match mode {
            "server" => d.pod.enter_server_mode(role.ok_or_else(|| {
                AgentPodError::PersonaParseError("role required for server mode".to_string())
            })?),
            "chat" => d.pod.enter_chat_mode(),
            "exit" => d.pod.exit_mode(),
            other => Err(AgentPodError::PersonaParseError(format!(
                "Unknown mode: {}",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PodStatusInfo {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: PodLifecycleState,
    pub webid: String,
    pub agent_type: AgentKind,
    pub template: String,
    pub pod_kind: PodKind,
    pub created_at: i64,
}

impl Default for ActivePods {
    fn default() -> Self {
        Self::new()
    }
}
