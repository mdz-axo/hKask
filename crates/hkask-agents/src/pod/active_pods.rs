//! ActivePods — Runtime registry for active pod deployments.
//!
//! Stores PodFactory + port references so the API matches
//! the old PodManager signature exactly. Zero consumer changes needed.

use super::AgentPodError;
use super::context::PodContext;
use super::deployment::{PodDeployment, PodFactory, PodRegistry};
use super::types::{AgentKind, AgentPersona, PodID, PodKind, PodLifecycleState};
use crate::a2a::A2ARuntime;
use crate::curator::SemanticIndex;
use crate::ports::{EpisodicStoragePort, MCPRuntimePort, SemanticStoragePort};
use hkask_capability::CapabilityChecker;
use hkask_cns::GovernedTool;
use hkask_database::sqlite::SqliteDriver;
use hkask_mcp::RawMcpToolPort;
use hkask_ports::InferencePort;
use hkask_types::{NuEventSink, WebID};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ActivePods {
    deployments: RwLock<HashMap<PodID, PodDeployment>>,
    factory: Option<Arc<PodFactory>>,
    a2a_runtime: Option<Arc<A2ARuntime>>,
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
    /// Matrix homeserver URL for automatic pod Matrix registration.
    matrix_homeserver_url: Option<String>,
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
            matrix_homeserver_url: None,
        }
    }

    /// Set the Matrix homeserver URL for automatic pod Matrix registration.
    pub fn with_matrix_homeserver(mut self, url: String) -> Self {
        self.matrix_homeserver_url = Some(url);
        self
    }

    /// Access the Matrix homeserver URL (for daemon retry loop).
    pub fn matrix_homeserver_url(&self) -> Option<&str> {
        self.matrix_homeserver_url.as_deref()
    }

    /// Create a mock ActivePods for testing — matches old PodManager::new_mock().
    /// Wires in-memory adapters and a test factory so create_pod/activate_pod work.
    /// Full test harness with in-memory adapters, AllowAllConsent,
    /// mock templates, and master key set. One call sets up everything
    /// needed for integration tests.
    pub fn new_test_harness(data_dir: &std::path::Path) -> Self {
        // Set test secrets for OCAP signing and canonical SQLCipher encryption.
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            );
            std::env::set_var("HKASK_DB_PASSPHRASE", "hkask-test-db-passphrase");
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

    /// Inner test harness.
    fn new_test_harness_inner(data_dir: &std::path::Path) -> Self {
        use crate::AllowAllConsent;
        use crate::a2a::A2ARuntime;
        use crate::adapters::mcp_runtime::CapabilityOnlyAdapter;
        use crate::adapters::memory_loop_adapter::MemoryLoopForwarder;
        use crate::pod::{PodFactory, system_capability_checker};
        use hkask_database::types::DbProvider;

        // A deterministic master key so token issuance and the capability checker
        // derive the SAME system OCAP key. SAFETY: test-only, single-threaded setup.
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            );
        }

        let driver: Arc<dyn hkask_database::driver::DatabaseDriver> = Arc::new(SqliteDriver::new(
            SqliteDriver::in_memory_pool().expect("in-memory pool"),
        )) as _;
        let adapter =
            Arc::new(MemoryLoopForwarder::from_driver(driver).expect("in-memory adapter"));
        let a2a = Arc::new(A2ARuntime::new(b"mock"));
        // Anchor the checker to BOTH the system OCAP authority (pre-registration
        // tokens) and the A2A root (post-registration tokens), so legitimate pod
        // tokens verify while forged tokens are rejected (matches production).
        let checker = Arc::new(
            system_capability_checker()
                .expect("system capability checker (test master key set)")
                .trust_root(a2a.root_public_key()),
        );
        let mcp = Arc::new(CapabilityOnlyAdapter::new(Arc::clone(&checker)));
        let factory = Arc::new(PodFactory::new(
            Arc::new(hkask_templates::TemplateCrateLoader::from_path(
                data_dir.join("templates"),
            )),
            Arc::new(AllowAllConsent),
            data_dir.to_path_buf(),
            DbProvider::Sqlite,
        ));
        Self::new().with_a2a_runtime(a2a).with_factory_and_ports(
            factory,
            mcp.clone(),
            None,
            Some(checker),
            None,
            adapter.clone() as Arc<dyn EpisodicStoragePort>,
            adapter as Arc<dyn SemanticStoragePort>,
        )
    }

    /// Wire the factory and port adapters so create_pod/activate_pod work
    /// with the simple old PodManager-style signatures.
    #[allow(clippy::too_many_arguments)]
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
    pub fn with_a2a_runtime(mut self, a2a: Arc<A2ARuntime>) -> Self {
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

    /// List all active pod names (for daemon self-healing loops).
    pub async fn pod_names(&self) -> Vec<String> {
        self.deployments
            .read()
            .await
            .values()
            .map(|d| d.pod.persona.agent.name.clone())
            .collect()
    }

    /// Retry Matrix registration for a pod (delegates to `register_pod_matrix`).
    ///
    /// Called by the background retry loop when a pending registration is found.
    pub async fn retry_pod_matrix_registration(
        homeserver_url: &str,
        pod_name: &str,
    ) -> anyhow::Result<()> {
        register_pod_matrix(homeserver_url, pod_name).await
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

    /// Get the persona for a pod by ID.
    pub async fn persona(&self, pod_id: &PodID) -> Option<AgentPersona> {
        self.deployments
            .read()
            .await
            .get(pod_id)
            .map(|d| d.pod.persona.clone())
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
        // Enforce CuratorPod singleton (P5 Essentialism).
        // Check BEFORE deployment to avoid SQLCipher HMAC mismatch on
        // already-encrypted curator.db from a prior pod.
        if pod_kind == PodKind::Curator {
            let ci = self.curator_index.read().await;
            if ci.is_some() {
                return Err(AgentPodError::PersonaParseError(
                    "CuratorPod already exists — only one CuratorPod per system".into(),
                ));
            }
            drop(ci);
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
        if pod_kind == PodKind::Curator
            && let Some(ref index) = deployment.semantic_index
        {
            let mut ci = self.curator_index.write().await;
            if ci.is_some() {
                return Err(AgentPodError::PersonaParseError(
                    "CuratorPod already exists — only one CuratorPod per system".into(),
                ));
            }
            *ci = Some(Arc::clone(index));
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
        // Idempotent — if curator already exists, don't spawn a second sync
        {
            let ci = self.curator_index.read().await;
            if let Some(ref index) = *ci {
                return Ok(Some(Arc::clone(index)));
            }
        }

        let (index, registry) = self.create_curator_pod(&data_dir).await?;
        let sync = crate::curator::CuratorSync::new(Arc::clone(&index), registry);
        tokio::spawn(async move {
            sync.run(cancel).await;
        });
        tracing::info!("CuratorSync spawned — polling semantic h_mems from all pods");

        Ok(Some(index))
    }

    /// Like `ensure_curator` but returns the `CuratorSync` without spawning
    /// the background loop. Integration tests call `sync.tick()` directly
    /// after storing an h_mem — deterministic, no polling, no timeout.
    pub async fn ensure_curator_for_test(
        &self,
        data_dir: std::path::PathBuf,
    ) -> Result<
        (
            Arc<std::sync::RwLock<SemanticIndex>>,
            crate::curator::CuratorSync,
        ),
        AgentPodError,
    > {
        let (index, registry) = self.create_curator_pod(&data_dir).await?;
        let sync = crate::curator::CuratorSync::new(Arc::clone(&index), registry);
        Ok((index, sync))
    }

    /// Create the CuratorPod, activate it, and build the PodRegistry.
    /// Shared by `ensure_curator` (production) and `ensure_curator_for_test` (tests).
    async fn create_curator_pod(
        &self,
        data_dir: &std::path::Path,
    ) -> Result<(Arc<std::sync::RwLock<SemanticIndex>>, Arc<PodRegistry>), AgentPodError> {
        // Check if curator already exists
        {
            let ci = self.curator_index.read().await;
            if let Some(ref index) = *ci {
                // Already exists — build registry from data_dir, return existing index
                let registry = Arc::new(PodRegistry::new(data_dir));
                return Ok((Arc::clone(index), registry));
            }
        }

        // Create CuratorPod
        let curator_persona = super::types::AgentPersona::system("curator", AgentKind::Bot);
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

        let registry = Arc::new(PodRegistry::new(data_dir));
        Ok((index, registry))
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

            // Matrix registration — register pod on Conduit synchronously.
            // Must complete before activation so the pod can authenticate
            // immediately when it starts. Failed registrations are retried
            // on the next activation attempt.
            if let Some(ref homeserver_url) = self.matrix_homeserver_url {
                let pod_name = d.pod.persona.agent.name.clone();

                // Check for pending retry from a previous failed attempt.
                let url = if let Ok(saved_url) = hkask_keystore::Keychain::default()
                    .retrieve_by_key(&format!(
                        "{}-{}",
                        hkask_types::keychain_keys::KEY_MATRIX_POD_PENDING_PREFIX,
                        pod_name
                    )) {
                    tracing::info!(
                        target: "hkask.communication.matrix.pod_registration",
                        pod = %pod_name,
                        "Retrying deferred Matrix pod registration"
                    );
                    saved_url
                } else {
                    homeserver_url.clone()
                };

                match register_pod_matrix(&url, &pod_name).await {
                    Ok(()) => {
                        let _ = hkask_keystore::Keychain::default().delete_by_key(&format!(
                            "{}-{}",
                            hkask_types::keychain_keys::KEY_MATRIX_POD_PENDING_PREFIX,
                            pod_name
                        ));
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "hkask.communication.matrix.pod_registration",
                            pod = %pod_name,
                            error = %e,
                            "Failed to register pod on Matrix — storing for retry"
                        );
                        let _ = hkask_keystore::Keychain::default().store_by_key(
                            &format!(
                                "{}-{}",
                                hkask_types::keychain_keys::KEY_MATRIX_POD_PENDING_PREFIX,
                                pod_name
                            ),
                            &url,
                        );
                        // Continue activation — pod can operate without Matrix.
                        // The daemon's self-healing loop will retry registration.
                    }
                }
            }
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

    /// Return (pod_name, pod_db_path) for all activated pods with existing databases.
    /// Used by the backup system to snapshot pod state.
    pub async fn pod_db_paths(&self) -> Vec<(String, std::path::PathBuf)> {
        self.deployments
            .read()
            .await
            .values()
            .filter(|d| d.storage.db_path.exists())
            .map(|d| (d.pod.persona.agent.name.clone(), d.storage.db_path.clone()))
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

    /// Export a pod as a container build context (delegates to PodFactory).
    pub fn export_container(
        &self,
        pod_id: PodID,
        output_dir: &std::path::Path,
    ) -> Result<(), super::AgentPodError> {
        let factory = self.factory.as_ref().ok_or_else(|| {
            super::AgentPodError::PersonaParseError("ActivePods not wired with PodFactory".into())
        })?;
        factory
            .export_container(pod_id, output_dir)
            .map_err(|e| super::AgentPodError::PersonaParseError(e.to_string()))
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

/// Register a pod on the Matrix homeserver (Conduit).
///
/// Uses m.login.dummy auth — the pod's Matrix identity is daemon-managed.
/// Credentials are stored in the OS keychain.
pub(crate) async fn register_pod_matrix(
    homeserver_url: &str,
    pod_name: &str,
) -> anyhow::Result<()> {
    let localpart = pod_name.to_lowercase().replace(' ', "-");
    let username = format!("{}-bot", localpart);
    let password = uuid::Uuid::new_v4().to_string();
    let full_id = format!("@{username}:localhost");

    let url = format!(
        "{}/_matrix/client/v3/register",
        homeserver_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "username": &username,
        "password": &password,
        "initial_device_display_name": format!("hKask Pod: {}", pod_name),
        "auth": {"type": "m.login.dummy"}
    });

    let response = reqwest::Client::new()
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Matrix registration request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Matrix registration HTTP {}",
            response.status().as_u16()
        ));
    }

    let keychain = hkask_keystore::Keychain::default();
    let _ = keychain.store_by_key(
        &format!(
            "{}{}",
            hkask_types::keychain_keys::KEY_MATRIX_POD_PREFIX,
            pod_name
        ),
        &password,
    );

    tracing::info!(
        target: "hkask.communication.matrix.pod_registered",
        pod = %pod_name,
        matrix_id = %full_id,
        "Pod registered on Matrix"
    );
    Ok(())
}
