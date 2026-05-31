//! PodManager, PodStatus, PodManagerBuilder — Pod lifecycle management

use hkask_keystore::keychain::Keychain;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::types::{AgentKind, AgentPersona, PodID, PodLifecycleState};
use super::{AgentPod, AgentPodError, AgentPodResult};
use crate::adapters::git_cas::GitCasAdapter;
use crate::adapters::mcp_runtime::McpRuntimeAdapter;
use crate::adapters::memory_storage::MemoryStorageAdapter;
#[allow(deprecated)]
use crate::ports::{
    EpisodicStoragePort, GitCASPort, MCPRuntimePort, MemoryStoragePort, SemanticStoragePort,
};
#[allow(deprecated)]
use crate::security::{AgentPersonaInput, SecurityContext};

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
    _keystore: Keychain,
    git_cas: Arc<dyn GitCASPort>,
    acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
    pub(crate) mcp_runtime: Arc<dyn MCPRuntimePort>,
    /// Episodic memory storage — private, agent-scoped (OCAP: EpisodicReadHandle/EpisodicWriteHandle)
    pub(crate) episodic_storage: Arc<dyn EpisodicStoragePort>,
    /// Semantic memory storage — shared, public knowledge (OCAP: SemanticReadHandle/SemanticWriteHandle)
    pub(crate) semantic_storage: Arc<dyn SemanticStoragePort>,
    /// Legacy memory storage (deprecated — use episodic_storage/semantic_storage)
    #[allow(deprecated)]
    pub(crate) memory_storage: Arc<dyn MemoryStoragePort>,
    pub(crate) security_context: SecurityContext,
    pub(crate) inference_port: Option<Arc<dyn hkask_templates::InferencePort>>,
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
    /// Create a new pod manager with trait-object adapters
    pub fn new(
        git_cas: Arc<dyn GitCASPort>,
        acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        episodic_storage: Arc<dyn EpisodicStoragePort>,
        semantic_storage: Arc<dyn SemanticStoragePort>,
    ) -> Self {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            _keystore: Keychain::default(),
            git_cas,
            acp_runtime,
            mcp_runtime,
            episodic_storage,
            semantic_storage,
            #[allow(deprecated)]
            memory_storage: Arc::new(
                MemoryStorageAdapter::in_memory()
                    .expect("In-memory storage initialization should never fail"),
            ),
            security_context: SecurityContext::default(),
            inference_port: None,
        }
    }

    /// Create a new pod manager with inference port
    pub fn with_inference(
        git_cas: Arc<dyn GitCASPort>,
        acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        episodic_storage: Arc<dyn EpisodicStoragePort>,
        semantic_storage: Arc<dyn SemanticStoragePort>,
        inference_port: Arc<dyn hkask_templates::InferencePort>,
    ) -> Self {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            _keystore: Keychain::default(),
            git_cas,
            acp_runtime,
            mcp_runtime,
            episodic_storage,
            semantic_storage,
            #[allow(deprecated)]
            memory_storage: Arc::new(
                MemoryStorageAdapter::in_memory()
                    .expect("In-memory storage initialization should never fail"),
            ),
            security_context: SecurityContext::default(),
            inference_port: Some(inference_port),
        }
    }

    /// Get the inference port if available
    pub fn inference_port(&self) -> Option<Arc<dyn hkask_templates::InferencePort>> {
        self.inference_port.clone()
    }

    /// Create a new pod manager with mock adapters for testing
    pub fn new_mock() -> Self {
        let adapter = Arc::new(
            MemoryStorageAdapter::in_memory()
                .expect("In-memory storage initialization should never fail"),
        );
        let episodic_storage: Arc<dyn EpisodicStoragePort> = adapter.clone();
        let semantic_storage: Arc<dyn SemanticStoragePort> = adapter.clone();

        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            _keystore: Keychain::default(),
            git_cas: Arc::new(GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-mock"))),
            acp_runtime: Arc::new(crate::acp::AcpRuntime::default()),
            mcp_runtime: Arc::new(McpRuntimeAdapter::new()),
            episodic_storage,
            semantic_storage,
            #[allow(deprecated)]
            memory_storage: Arc::new(
                MemoryStorageAdapter::in_memory()
                    .expect("In-memory storage initialization should never fail"),
            ),
            security_context: SecurityContext::default(),
            inference_port: None,
        }
    }
}

/// Builder for constructing [`PodManager`] with explicit adapter configuration
///
/// # Example
///
/// ```rust,no_run
/// use hkask_agents::pod::PodManagerBuilder;
/// use hkask_agents::adapters::git_cas::GitCasAdapter;
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// let pod_manager = PodManagerBuilder::new()
///     .git_cas(Arc::new(GitCasAdapter::from_path(PathBuf::from("./registry/templates"))))
///     .with_in_memory_storage()
///     .build();
/// ```
pub struct PodManagerBuilder {
    git_cas: Option<Arc<dyn GitCASPort>>,
    acp_runtime: Option<Arc<dyn crate::ports::AcpPort + Send + Sync>>,
    mcp_runtime: Option<Arc<dyn MCPRuntimePort>>,
    episodic_storage: Option<Arc<dyn EpisodicStoragePort>>,
    semantic_storage: Option<Arc<dyn SemanticStoragePort>>,
    security_context: Option<SecurityContext>,
    inference_port: Option<Arc<dyn hkask_templates::InferencePort>>,
}

impl PodManagerBuilder {
    pub fn new() -> Self {
        Self {
            git_cas: None,
            acp_runtime: None,
            mcp_runtime: None,
            episodic_storage: None,
            semantic_storage: None,
            security_context: None,
            inference_port: None,
        }
    }

    pub fn git_cas(mut self, adapter: Arc<dyn GitCASPort>) -> Self {
        self.git_cas = Some(adapter);
        self
    }

    pub fn git_cas_from_path<P: Into<PathBuf>>(self, path: P) -> Self {
        self.git_cas(Arc::new(GitCasAdapter::from_path(path.into())))
    }

    pub fn acp_runtime(mut self, adapter: Arc<dyn crate::ports::AcpPort + Send + Sync>) -> Self {
        self.acp_runtime = Some(adapter);
        self
    }

    pub fn mcp_runtime(mut self, adapter: Arc<dyn MCPRuntimePort>) -> Self {
        self.mcp_runtime = Some(adapter);
        self
    }

    pub fn episodic_storage(mut self, adapter: Arc<dyn EpisodicStoragePort>) -> Self {
        self.episodic_storage = Some(adapter);
        self
    }

    pub fn semantic_storage(mut self, adapter: Arc<dyn SemanticStoragePort>) -> Self {
        self.semantic_storage = Some(adapter);
        self
    }

    pub fn inference_port(mut self, adapter: Arc<dyn hkask_templates::InferencePort>) -> Self {
        self.inference_port = Some(adapter);
        self
    }

    /// Configure with in-memory storage (episodic and semantic)
    pub fn with_in_memory_storage(self) -> Self {
        let adapter = Arc::new(
            MemoryStorageAdapter::in_memory()
                .expect("In-memory storage initialization should never fail"),
        );
        let episodic: Arc<dyn EpisodicStoragePort> = adapter.clone();
        let semantic: Arc<dyn SemanticStoragePort> = adapter.clone();
        self.episodic_storage(episodic).semantic_storage(semantic)
    }

    /// Configure with encrypted storage (episodic and semantic)
    pub fn with_encrypted_storage<P: AsRef<std::path::Path>>(
        self,
        path: P,
        passphrase: &str,
    ) -> Self {
        let path_str = path
            .as_ref()
            .to_str()
            .expect("Storage path must be valid UTF-8");
        let adapter = Arc::new(
            MemoryStorageAdapter::from_path(path_str, passphrase)
                .expect("Encrypted storage initialization should succeed"),
        );
        let episodic: Arc<dyn EpisodicStoragePort> = adapter.clone();
        let semantic: Arc<dyn SemanticStoragePort> = adapter.clone();
        self.episodic_storage(episodic).semantic_storage(semantic)
    }

    pub fn security_context(mut self, context: SecurityContext) -> Self {
        self.security_context = Some(context);
        self
    }

    pub fn build(self) -> PodManager {
        let adapter = Arc::new(
            MemoryStorageAdapter::in_memory()
                .expect("In-memory storage initialization should never fail"),
        );
        let default_episodic: Arc<dyn EpisodicStoragePort> = adapter.clone();
        let default_semantic: Arc<dyn SemanticStoragePort> = adapter.clone();
        let episodic_storage = self.episodic_storage.unwrap_or(default_episodic);
        let semantic_storage = self.semantic_storage.unwrap_or(default_semantic);

        let mut manager = PodManager::new(
            self.git_cas.unwrap_or_else(|| {
                Arc::new(GitCasAdapter::from_path(PathBuf::from(
                    "./registry/templates",
                )))
            }),
            self.acp_runtime
                .unwrap_or_else(|| Arc::new(crate::acp::AcpRuntime::default())),
            self.mcp_runtime
                .unwrap_or_else(|| Arc::new(McpRuntimeAdapter::new())),
            episodic_storage,
            semantic_storage,
        );
        manager.inference_port = self.inference_port;
        if let Some(ctx) = self.security_context {
            manager.security_context = ctx;
        }
        manager
    }
}

impl Default for PodManagerBuilder {
    fn default() -> Self {
        Self::new()
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
        // Validate persona input
        // TODO: Migrate to AgentPersona::validate_fields() directly
        #[allow(deprecated)]
        let input = AgentPersonaInput {
            name: persona.agent.name.clone(),
            agent_type: persona.agent.agent_type.to_string().to_lowercase(),
            version: persona.agent.version.clone(),
            description: persona.charter.description.clone(),
            editor: persona.charter.editor.clone(),
            capabilities: persona.capabilities.clone(),
        };

        #[allow(deprecated)]
        input
            .validate(&input)
            .map_err(|e| AgentPodError::PersonaParseError(e.to_string()))?;

        let pod = AgentPod::new_with_memory(
            template_name,
            persona,
            self.git_cas.as_ref(),
            Some(Arc::clone(&self.memory_storage)),
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
                .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

            if pod.state() == PodLifecycleState::Populated {
                Some((
                    pod.webid,
                    pod.agent_type.to_string(),
                    pod.persona.capabilities.clone(),
                ))
            } else {
                None
            }
        }; // Guard dropped here

        // Phase 2: Async ACP registration without holding the lock
        let token = if let Some((webid, agent_type, capabilities)) = registration_data {
            Some(
                self.acp_runtime
                    .register_agent(webid, &agent_type, capabilities)
                    .await
                    .map_err(|e| AgentPodError::ACPRegistrationError(e.to_string()))?,
            )
        } else {
            None
        };

        // Phase 3: Apply result and activate MCP while holding write guard
        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

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
        }

        pod.activate(self.mcp_runtime.as_ref())?;

        // Persist activation event to memory storage
        let event = serde_json::json!({
            "entity": pod.webid.to_string(),
            "attribute": "lifecycle_event",
            "value": {
                "event": "activated",
                "pod_id": pod.id.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        });

        #[allow(deprecated)]
        let _ = self.memory_storage.store_artifact(
            pod.webid,
            "episodic_triple",
            event,
            "private",
            &pod.capability_token,
        );

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
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        let token_id = pod.capability_token.id.clone();
        let webid = pod.webid;

        pod.deactivate()?;

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

        // Persist deactivation event to memory storage
        let event = serde_json::json!({
            "entity": pod.webid.to_string(),
            "attribute": "lifecycle_event",
            "value": {
                "event": "deactivated",
                "pod_id": pod.id.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        });

        #[allow(deprecated)]
        let _ = self.memory_storage.store_artifact(
            pod.webid,
            "episodic_triple",
            event,
            "private",
            &pod.capability_token,
        );

        info!(
            target: "hkask.pod",
            pod_id = %pod_id,
            "Pod deactivated"
        );

        Ok(())
    }

    /// Recall lifecycle events for a pod
    pub async fn recall_pod_events(
        &self,
        pod_id: &PodID,
    ) -> AgentPodResult<Vec<serde_json::Value>> {
        let pods = self.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        #[allow(deprecated)]
        let results = self
            .memory_storage
            .recall(&pod.webid.to_string(), &pod.capability_token)
            .map_err(|e| AgentPodError::StorageError(e.to_string()))?;

        Ok(results)
    }

    /// Get pod status
    pub async fn get_pod_status(&self, pod_id: &PodID) -> AgentPodResult<PodStatus> {
        let pods = self.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

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
