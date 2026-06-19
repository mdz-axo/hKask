//! PodDeployment — Per-pod deployment unit (Solid Pod isomorphism)
//!
//! Each pod IS the deployment unit with its own SQLCipher file, CNS runtime,
//! and MCP server bindings. No shared state. No service collision surface.
//!
//! This replaces the centralized PodManager. PodRegistry provides filesystem-based
//! pod discovery (scanning {data_dir}/pods/*.db) — no HashMap cache.
//!
//! # Principles
//!
//! - [P6] Goal: Space for Replicants — each replicant inhabits its own pod
//! - [P11] Constraining: Digital Public/Private Sphere — per-pod SQLCipher boundary
//! - [P4] Constraining: Clear Boundaries — OCAP tokens scoped to this pod
//! - [P5] Goal: Essentialism — factory only; no runtime cache
//! - [P9] Goal: Homeostatic Self-Regulation — per-pod variety tracking

use hkask_cns::CnsRuntime;
use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_storage::{Database, DatabaseError, EmbeddingStore, Triple, TripleStore};
use hkask_types::event::SpanNamespace;
use hkask_types::{
    CapabilityChecker, DelegationAction, DelegationResource, DelegationToken, NuEventSink, WebID,
};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info};

use super::types::{AgentPersona, PodID, PodLifecycleState};
use super::{AgentPod, AgentPodError};
use crate::SovereigntyChecker;
use crate::ports::{EpisodicStoragePort, MCPRuntimePort, SemanticStoragePort};
use hkask_mcp::GitCasAdapter;

// ── PodDeployment — The pod IS the deployment unit ──────────────────────────

/// A pod IS the deployment unit. Constructing a PodDeployment
/// means: a SQLCipher database file exists at {data_dir}/pods/{pod_id}.db,
/// a CNS runtime is initialized with namespace cns.agent_pod.{pod_id}.*,
/// and MCP servers are bound. No shared state. No service collision surface.
///
/// [P11] Constraining: Digital Public/Private Sphere — per-pod SQLCipher boundary
pub struct PodDeployment {
    /// Pod identity — WebID is the root of all authority (P1)
    pub pod_id: PodID,
    /// The underlying AgentPod (identity, lifecycle, persona, capability token)
    pub pod: AgentPod,
    /// Dedicated database. The file IS the pod. No shared store.
    pub storage: PerPodStorage,
    /// Dedicated CNS runtime. Variety counters scoped to this pod.
    pub cns: PerPodCnsRuntime,
    /// MCP server bindings for this pod
    pub tools: PerPodToolBinding,
    /// Capability checker for OCAP verification
    pub capability_checker: Option<Arc<CapabilityChecker>>,
    /// Sovereignty checker wired to consent port
    pub sovereignty_checker: SovereigntyChecker,
    /// NuEvent sink for lifecycle events
    pub nu_event_sink: Option<Arc<dyn NuEventSink>>,
    /// MCP runtime for tool dispatch
    pub mcp_runtime: Arc<dyn MCPRuntimePort>,
    /// Episodic storage port (backed by this pod's SQLCipher)
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,
    /// Semantic storage port (backed by this pod's SQLCipher)
    pub semantic_storage: Arc<dyn SemanticStoragePort>,
}

/// PerPodStorage owns a SQLCipher database file for a single pod.
/// The file IS the pod's data. Backup IS copying the file.
/// This type makes "shared store" an invalid state — you cannot
/// accidentally query another pod's data because you have no
/// connection handle to its file.
///
/// [P11] Goal: Digital Public/Private Sphere — storage isolation
pub struct PerPodStorage {
    /// The encrypted database connection
    pub db: Database,
    /// Triple store backed by this pod's database
    pub triples: TripleStore,
    /// Embedding store backed by this pod's database
    pub embeddings: EmbeddingStore,
    /// Path to this pod's database file
    pub db_path: PathBuf,
}

/// PerPodCnsRuntime is a CNS runtime scoped to a single pod.
///
/// [P11] Constraining: Digital Public/Private Sphere — CNS isolation
pub struct PerPodCnsRuntime {
    /// The pod this CNS runtime is scoped to
    pod_id: PodID,
    /// Span namespace prefix: cns.agent_pod.{pod_id}
    span_namespace: String,
    /// The actual CNS runtime — per-pod isolate
    inner: CnsRuntime,
}

/// PerPodToolBinding owns the MCP server instances for this pod.
///
pub struct PerPodToolBinding {
    pub mcp_runtime: Arc<dyn MCPRuntimePort>,
    pub governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
}

// ── PodDeployError ──────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum PodDeployError {
    #[error("Failed to create pod storage at {path}: {reason}")]
    StorageInitFailed { path: PathBuf, reason: String },

    #[error("Failed to initialize CNS runtime: {reason}")]
    CnsInitFailed { reason: String },

    #[error("Pod lifecycle error: {0}")]
    PodError(#[from] AgentPodError),

    #[error("Template resolution failed: {0}")]
    TemplateError(String),

    #[error("Pod not found: {0}")]
    PodNotFound(PodID),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ── PerPodCnsRuntime ────────────────────────────────────────────────────────

impl PerPodCnsRuntime {
    pub fn scoped(pod_id: PodID) -> Self {
        let span_namespace = format!("cns.agent_pod.{}", pod_id);
        let inner = CnsRuntime::default();
        debug!(
            target: "cns.agent_pod",
            pod_id = %pod_id,
            namespace = %span_namespace,
            "Per-pod CNS runtime initialized"
        );
        Self {
            pod_id,
            span_namespace,
            inner,
        }
    }

    pub fn pod_id(&self) -> PodID {
        self.pod_id
    }
    pub fn span_namespace(&self) -> &str {
        &self.span_namespace
    }
    pub fn inner(&self) -> &CnsRuntime {
        &self.inner
    }

    pub async fn emit_tool_span(&self, tool_name: &str) {
        let domain = format!("cns.agent_pod.{}.tool.{}", self.pod_id, tool_name);
        self.inner.increment_variety(&domain, tool_name).await;
    }

    pub async fn record_tool_outcome(
        &self,
        tool_name: &str,
        success: bool,
        error_kind: Option<&str>,
    ) {
        let domain = format!("cns.agent_pod.{}.tool.{}", self.pod_id, tool_name);
        self.inner
            .record_outcome(&domain, success, error_kind)
            .await;
    }

    pub async fn health(&self) -> hkask_types::cns::CnsHealth {
        self.inner.health().await
    }

    pub async fn variety(&self) -> std::collections::HashMap<SpanNamespace, u64> {
        self.inner.variety().await
    }

    pub async fn variety_for_tool(&self, tool_name: &str) -> u64 {
        let domain = format!("cns.agent_pod.{}.tool.{}", self.pod_id, tool_name);
        self.inner.variety_for_domain(&domain).await
    }

    pub async fn register_energy_budget(&self, agent: WebID, budget: hkask_cns::EnergyBudget) {
        self.inner.register_energy_budget(agent, budget).await;
    }

    pub async fn agent_energy_status(&self, agent: &WebID) -> Option<hkask_cns::AgentEnergyStatus> {
        self.inner.agent_gas_status(agent).await
    }
}

// ── PodFactory — Stateless pod constructor ──────────────────────────────────

/// PodFactory constructs PodDeployment instances from templates.
/// Stateless — does not cache, pool, or share pods.
///
pub struct PodFactory {
    git_cas: Arc<GitCasAdapter>,
    consent: Arc<dyn crate::SovereigntyConsent>,
    data_dir: PathBuf,
}

impl PodFactory {
    pub fn new(
        git_cas: Arc<GitCasAdapter>,
        consent: Arc<dyn crate::SovereigntyConsent>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            git_cas,
            consent,
            data_dir,
        }
    }

    /// Deploy a new pod with its own SQLCipher database file,
    /// per-pod CNS runtime, and MCP bindings.
    ///
    /// The pod's database passphrase is derived deterministically
    /// from the user's master key via HKDF-SHA256 (ADR-027), same
    /// as `derive_ocap_secret`. Same master key + same WebID →
    /// same pod key material, independent of server.
    ///
    /// [P11] Constraining: Digital Public/Private Sphere — per-pod SQLCipher
    pub async fn deploy(
        &self,
        template_name: &str,
        persona: &AgentPersona,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
        capability_checker: Option<Arc<CapabilityChecker>>,
        nu_event_sink: Option<Arc<dyn NuEventSink>>,
        episodic_adapter: Arc<dyn EpisodicStoragePort>,
        semantic_adapter: Arc<dyn SemanticStoragePort>,
    ) -> Result<PodDeployment, PodDeployError> {
        // 1. Create the underlying AgentPod
        let pod = AgentPod::new(
            template_name,
            persona,
            self.git_cas.as_ref(),
            Arc::clone(&self.consent),
        )?;
        let pod_id = pod.id;

        // 2. Create per-pod SQLCipher database file
        let storage = self.create_pod_storage(pod_id, persona)?;

        // 3. Initialize per-pod CNS runtime
        let cns = PerPodCnsRuntime::scoped(pod_id);

        let tools = PerPodToolBinding {
            mcp_runtime: Arc::clone(&mcp_runtime),
            governed_tool,
        };
        let sovereignty_checker = pod.sovereignty_checker.clone();

        info!(
            target: "hkask.pod.deployment",
            pod_id = %pod_id, template = %template_name,
            db_path = %storage.db_path.display(),
            cns_namespace = %cns.span_namespace,
            "Pod deployed (isolated mode, per-pod SQLCipher + CNS)"
        );

        Ok(PodDeployment {
            pod_id,
            pod,
            storage,
            cns,
            tools,
            capability_checker,
            sovereignty_checker,
            nu_event_sink,
            mcp_runtime,
            episodic_storage: episodic_adapter,
            semantic_storage: semantic_adapter,
        })
    }

    /// Create the per-pod SQLCipher database file and stores.
    fn create_pod_storage(
        &self,
        pod_id: PodID,
        persona: &AgentPersona,
    ) -> Result<PerPodStorage, PodDeployError> {
        let pod_db_dir = self.data_dir.join("pods");
        std::fs::create_dir_all(&pod_db_dir).map_err(|e| PodDeployError::StorageInitFailed {
            path: pod_db_dir.clone(),
            reason: e.to_string(),
        })?;

        let db_path = pod_db_dir.join(format!("{}.db", pod_id));
        let db_path_str = db_path.to_string_lossy().to_string();

        // Derive deterministic passphrase from user's master key (ADR-027)
        let passphrase = super::derive_ocap_secret(&persona.webid()).map_err(|e| {
            PodDeployError::StorageInitFailed {
                path: db_path.clone(),
                reason: e.to_string(),
            }
        })?;

        // Open/create the SQLCipher database
        let db = Database::open(&db_path_str, &passphrase).map_err(|e| {
            PodDeployError::StorageInitFailed {
                path: db_path.clone(),
                reason: format!("{e}"),
            }
        })?;

        let conn = db.conn_arc();
        let triples = TripleStore::new(Arc::clone(&conn));
        let embeddings = EmbeddingStore::new(conn);

        Ok(PerPodStorage {
            db,
            triples,
            embeddings,
            db_path,
        })
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

// ── PodRegistry — Filesystem-based pod discovery ────────────────────────────

/// Lightweight pod index — scans {data_dir}/pods/*.db for deployed pods.
/// No cache. No HashMap. Just the filesystem.
///
pub struct PodRegistry {
    pods_dir: PathBuf,
}

impl PodRegistry {
    pub fn new(data_dir: &PathBuf) -> Self {
        Self {
            pods_dir: data_dir.join("pods"),
        }
    }

    /// List all deployed pod IDs by scanning the pods directory.
    pub fn list_pod_ids(&self) -> Result<Vec<PodID>, PodDeployError> {
        if !self.pods_dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&self.pods_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "db") {
                if let Some(stem) = path.file_stem() {
                    if let Ok(id) = stem.to_string_lossy().parse::<PodID>() {
                        ids.push(id);
                    }
                }
            }
        }
        Ok(ids)
    }

    /// Check if a pod exists on disk.
    pub fn pod_exists(&self, pod_id: &PodID) -> bool {
        self.pods_dir.join(format!("{pod_id}.db")).exists()
    }

    /// Get the database path for a pod.
    pub fn db_path(&self, pod_id: &PodID) -> PathBuf {
        self.pods_dir.join(format!("{pod_id}.db"))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_persona() -> AgentPersona {
        let yaml = r#"
agent:
  name: test-replicant
  type: Replicant
  version: "0.1.0"
charter:
  description: "A test replicant for deployment tests"
  editor: "test-user"
capabilities:
  - "tool:execute"
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;
        AgentPersona::from_yaml(yaml).expect("Failed to parse test persona")
    }

    #[test]
    fn factory_constructs_with_valid_params() {
        let data_dir = PathBuf::from("/tmp/hkask-test-factory");
        let git_cas = Arc::new(GitCasAdapter::from_path(data_dir.clone()));
        let consent = Arc::new(crate::DenyAllConsent);
        let factory = PodFactory::new(git_cas, consent, data_dir.clone());
        assert_eq!(factory.data_dir(), &data_dir);
    }

    #[test]
    fn per_pod_cns_isolated_namespaces() {
        let pod_a = PodID::new();
        let pod_b = PodID::new();
        let cns_a = PerPodCnsRuntime::scoped(pod_a);
        let cns_b = PerPodCnsRuntime::scoped(pod_b);
        assert_ne!(cns_a.span_namespace(), cns_b.span_namespace());
        assert!(cns_a.span_namespace().contains(&pod_a.to_string()));
    }

    #[test]
    fn pod_registry_discovers_pods() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let registry = PodRegistry::new(&temp.path().to_path_buf());
        // Empty directory — no pods
        let ids = registry.list_pod_ids().expect("list");
        assert!(ids.is_empty());

        // Create a pod directory
        let pods_dir = temp.path().join("pods");
        std::fs::create_dir_all(&pods_dir).expect("create pods dir");
        let pod_id = PodID::new();
        std::fs::write(pods_dir.join(format!("{pod_id}.db")), b"").expect("write db file");

        let ids = registry.list_pod_ids().expect("list");
        assert_eq!(ids.len(), 1);
        assert!(registry.pod_exists(&pod_id));
        assert!(!registry.pod_exists(&PodID::new()));
    }
}
