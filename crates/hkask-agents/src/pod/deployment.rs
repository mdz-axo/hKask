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
//! - \[P6\] Goal: Space for Replicants — each replicant inhabits its own pod
//!   - Constraining: Digital Public/Private Sphere — per-pod SQLCipher boundary
//!   - Constraining: Clear Boundaries — OCAP tokens scoped to this pod
//! - \[P5\] Goal: Essentialism — factory only; no runtime cache
//! - \[P9\] Goal: Homeostatic Self-Regulation — per-pod variety tracking

use hkask_capability::CapabilityChecker;
use hkask_cns::CnsRuntime;
use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_ports::InferencePort;
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_types::event::SpanNamespace;
use hkask_types::{NuEventSink, WebID};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info};

use super::types::{AgentPersona, PodID, PodKind};
use super::{AgentPod, AgentPodError};
use crate::SovereigntyChecker;
use crate::curator::SemanticIndex;
use crate::ports::{EpisodicStoragePort, MCPRuntimePort, SemanticStoragePort};
use hkask_templates::TemplateCrateLoader;

// ── PodDeployment — The pod IS the deployment unit ──────────────────────────

/// A pod IS the deployment unit. Constructing a PodDeployment
/// means: a SQLCipher database file exists at {data_dir}/pods/{pod_id}.db,
/// a CNS runtime is initialized with namespace cns.agent_pod.{pod_id}.*,
/// and MCP servers are bound. No shared state. No service collision surface.
///
///Constraining: Digital Public/Private Sphere — per-pod SQLCipher boundary
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
    /// Inference port for LLM generation (None if inference unavailable)
    pub inference_port: Option<Arc<dyn InferencePort>>,
    /// Pod tier — determines isolation model
    pub pod_kind: PodKind,
    /// Semantic index — only set on CuratorPod
    pub semantic_index: Option<Arc<std::sync::RwLock<SemanticIndex>>>,
}

/// PerPodStorage owns a SQLCipher database file for a single pod.
/// The file IS the pod's data. Backup IS copying the file.
/// This type makes "shared store" an invalid state — you cannot
/// accidentally query another pod's data because you have no
/// connection handle to its file.
///
/// \[P11\] Goal: Digital Public/Private Sphere — storage isolation
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
///Constraining: Digital Public/Private Sphere — CNS isolation
#[derive(Clone)]
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

    pub async fn register_gas_budget(&self, agent: WebID, budget: hkask_cns::GasBudget) {
        self.inner.register_gas_budget(agent, budget).await;
    }

    pub async fn agent_energy_status(&self, agent: &WebID) -> Option<hkask_cns::AgentGasStatus> {
        self.inner.agent_gas_status(agent).await
    }
}

// ── PodFactory — Stateless pod constructor ──────────────────────────────────

/// PodFactory constructs PodDeployment instances from templates.
/// Stateless — does not cache, pool, or share pods.
///
pub struct PodFactory {
    template_loader: Arc<TemplateCrateLoader>,
    consent: Arc<dyn crate::SovereigntyConsent>,
    data_dir: PathBuf,
}

impl PodFactory {
    pub fn new(
        template_loader: Arc<TemplateCrateLoader>,
        consent: Arc<dyn crate::SovereigntyConsent>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            template_loader,
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
    ///Constraining: Digital Public/Private Sphere — per-pod SQLCipher
    #[allow(clippy::too_many_arguments)]
    pub async fn deploy(
        &self,
        template_name: &str,
        persona: &AgentPersona,
        pod_kind: PodKind,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
        capability_checker: Option<Arc<CapabilityChecker>>,
        nu_event_sink: Option<Arc<dyn NuEventSink>>,
        inference_port: Option<Arc<dyn InferencePort>>,
    ) -> Result<PodDeployment, PodDeployError> {
        // 1. Create the underlying AgentPod
        let mut pod = AgentPod::new(
            template_name,
            persona,
            self.template_loader.as_ref(),
            Arc::clone(&self.consent),
        )?;
        // Deterministic PodID from pod_kind + persona name (Solid Pod principle).
        // Same persona + same pod_kind → same PodID on any server, portable identity.
        let pod_id = PodID::from_name(&format!("{}:{}", pod_kind, persona.agent.name));
        pod.id = pod_id;

        // 2. Create per-pod SQLCipher database + storage adapters
        let (storage, memory_adapter) = self.create_pod_storage(pod_id, persona, pod_kind)?;

        // 3. Initialize per-pod CNS runtime
        let cns = PerPodCnsRuntime::scoped(pod_id);

        let tools = PerPodToolBinding {
            mcp_runtime: Arc::clone(&mcp_runtime),
            governed_tool,
        };
        let sovereignty_checker = pod.sovereignty_checker.clone();

        let adapter: Arc<crate::adapters::memory_loop_adapter::MemoryLoopForwarder> =
            Arc::new(memory_adapter);
        let episodic: Arc<dyn EpisodicStoragePort> = adapter.clone();
        let semantic: Arc<dyn SemanticStoragePort> = adapter;

        // Create SemanticIndex if this is a CuratorPod
        let semantic_index = if pod_kind == PodKind::Curator {
            let conn = storage.db.conn_arc();
            let index_store = TripleStore::new(conn);
            Some(Arc::new(std::sync::RwLock::new(SemanticIndex::new(
                index_store,
            ))))
        } else {
            None
        };

        info!(
            target: "hkask.pod.deployment",
            pod_id = %pod_id, template = %template_name,
            db_path = %storage.db_path.display(),
            cns_namespace = %cns.span_namespace,
            "Pod deployed (self-contained SQLCipher storage)"
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
            episodic_storage: episodic,
            semantic_storage: semantic,
            inference_port,
            pod_kind,
            semantic_index,
        })
    }

    /// Create the per-pod SQLCipher database file, stores, and memory adapter.
    /// Returns both the storage struct and a MemoryLoopForwarder that wraps
    /// the pod's own TripleStore + EmbeddingStore for episodic/semantic I/O.
    fn create_pod_storage(
        &self,
        _pod_id: PodID,
        persona: &AgentPersona,
        pod_kind: PodKind,
    ) -> Result<
        (
            PerPodStorage,
            crate::adapters::memory_loop_adapter::MemoryLoopForwarder,
        ),
        PodDeployError,
    > {
        // Standard agent directory: agents/{name}/
        let agent_name = &persona.agent.name;
        let agent_dir = self
            .data_dir
            .join(hkask_types::agent_paths::AGENTS_DIR)
            .join(hkask_types::agent_paths::sanitize_name(agent_name));
        std::fs::create_dir_all(&agent_dir).map_err(|e| PodDeployError::StorageInitFailed {
            path: agent_dir.clone(),
            reason: e.to_string(),
        })?;

        // Pod database at agents/{name}/pod.db
        let db_path = agent_dir.join("pod.db");
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

        // Build stores from the pod's own database connection
        let conn = db.conn_arc();
        let triples = TripleStore::new(Arc::clone(&conn));
        let embeddings = EmbeddingStore::new(Arc::clone(&conn));

        // Create memory adapter from the pod's own database — this is what
        // PodContext uses for episodic/semantic I/O. All data goes into
        // the pod's own SQLCipher file.
        let memory = crate::adapters::memory_loop_adapter::MemoryLoopForwarder::from_connection(
            db.conn_arc(),
        )
        .map_err(|e| PodDeployError::StorageInitFailed {
            path: db_path.clone(),
            reason: e.to_string(),
        })?;

        // Write webid sidecar for CuratorSync passphrase bootstrapping.
        // The .webid file is read BEFORE the database is opened (chicken-and-egg
        // resolution: need webid to derive passphrase, need passphrase to open DB).
        let webid_path = db_path.with_extension("webid");
        std::fs::write(&webid_path, persona.webid().to_string()).map_err(|e| {
            PodDeployError::StorageInitFailed {
                path: webid_path.clone(),
                reason: format!("Failed to write webid sidecar: {e}. CuratorSync depends on this file to sync the pod."),
            }
        })?;
        // Write pod kind sidecar for PodRegistry classification.
        let kind_path = db_path.with_extension("kind");
        let kind_str = match pod_kind {
            PodKind::Curator => "curator",
            PodKind::Team => "team",
            PodKind::Replicant => "replicant",
        };
        std::fs::write(&kind_path, kind_str).map_err(|e| PodDeployError::StorageInitFailed {
            path: kind_path.clone(),
            reason: format!("Failed to write pod.kind sidecar: {e}"),
        })?;
        // Write pod.name sidecar with the original (unsanitized) agent name.
        // The directory name is sanitized for filesystem safety, but PodID
        // derivation and cross-pod references use the original name.
        let name_path = db_path.with_extension("name");
        std::fs::write(&name_path, &persona.agent.name).map_err(|e| {
            PodDeployError::StorageInitFailed {
                path: name_path.clone(),
                reason: format!("Failed to write pod.name sidecar: {e}"),
            }
        })?;
        // Also write pod metadata into the database for backup/portability.
        {
            let conn = db.conn_arc();
            let conn = conn.lock().map_err(|e| PodDeployError::StorageInitFailed {
                path: db_path.clone(),
                reason: format!("Lock failed: {e}"),
            })?;
            let now = chrono::Utc::now().to_rfc3339();
            for (key, value) in &[
                ("webid", persona.webid().to_string()),
                ("pod_kind", format!("{:?}", pod_kind)),
                ("created_at", now),
            ] {
                conn.execute(
                    "INSERT OR REPLACE INTO pod_meta (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, value],
                )
                .map_err(|e| PodDeployError::StorageInitFailed {
                    path: db_path.clone(),
                    reason: format!("Failed to write pod_meta.{}: {e}", key),
                })?;
            }
        }

        Ok((
            PerPodStorage {
                db,
                triples,
                embeddings,
                db_path,
            },
            memory,
        ))
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Export a pod as a container image build context.
    ///
    /// Produces a Containerfile + pod files in `output_dir`:
    /// - `Containerfile` — inlined at runtime via `format!()` (no external template)
    /// - `pods/{pod_id}.db` — copy of the pod's SQLCipher file
    /// - `pods/{pod_id}.webid` — copy of the webid sidecar
    ///
    /// After export: `docker build -t hkask-pod-{pod_id} {output_dir}`
    pub fn export_container(
        &self,
        pod_id: PodID,
        output_dir: &std::path::Path,
    ) -> Result<(), PodDeployError> {
        let registry = PodRegistry::new(&self.data_dir);
        let db_path = registry.db_path(&pod_id);
        if !db_path.exists() {
            return Err(PodDeployError::PodNotFound(pod_id));
        }

        let webid_path = db_path.with_extension("webid");
        let webid = std::fs::read_to_string(&webid_path).map_err(|_| {
            PodDeployError::StorageInitFailed {
                path: webid_path.clone(),
                reason: "Missing webid sidecar — pod identity cannot be resolved for export".into(),
            }
        })?;

        // Create output directory structure
        let pods_out = output_dir.join("pods");
        std::fs::create_dir_all(&pods_out).map_err(PodDeployError::Io)?;

        // Copy the pod's database, webid, and salt files
        std::fs::copy(&db_path, pods_out.join(format!("{}.db", pod_id)))
            .map_err(PodDeployError::Io)?;
        if webid_path.exists() {
            std::fs::copy(&webid_path, pods_out.join(format!("{}.webid", pod_id)))
                .map_err(PodDeployError::Io)?;
        }
        let salt_path = db_path.with_extension("db.salt");
        if salt_path.exists() {
            std::fs::copy(&salt_path, pods_out.join(format!("{}.db.salt", pod_id)))
                .map_err(PodDeployError::Io)?;
        }

        // Inline Containerfile via format!()
        let containerfile = format!(
            "# Pod Container — Generated by hKask v0.30.0\n\
             FROM hkask-runtime:0.30.0\n\
             LABEL hkask.pod.id=\"{pod_id}\"\n\
             LABEL hkask.pod.webid=\"{webid}\"\n\
             LABEL hkask.exported.at=\"{}\"\n\n\
             COPY pods/{pod_id}.db /data/pod.db\n\
             COPY pods/{pod_id}.webid /data/pod.webid\n\n\
             ENV HKASK_POD_ID={pod_id}\n\
             ENV HKASK_POD_MODE=standalone\n\n\
             ENTRYPOINT [\"kask\", \"pod\", \"serve\", \"--pod-id\", \"{pod_id}\"]\n",
            chrono::Utc::now().to_rfc3339()
        );
        std::fs::write(output_dir.join("Containerfile"), &containerfile)
            .map_err(PodDeployError::Io)?;

        info!(target: "hkask.pod.export", pod_id = %pod_id, output = %output_dir.display(),
            "Pod exported as container build context");
        Ok(())
    }
}

// ── PodRegistry — Filesystem-based pod discovery ────────────────────────────

/// Lightweight pod index — scans {data_dir}/agents/*/pod.db for deployed pods.
pub struct PodRegistry {
    agents_dir: PathBuf,
}

impl PodRegistry {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            agents_dir: data_dir.join(hkask_types::agent_paths::AGENTS_DIR),
        }
    }

    /// List all deployed pod IDs by scanning agent directories.
    pub fn list_pod_ids(&self) -> Result<Vec<PodID>, PodDeployError> {
        if !self.agents_dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&self.agents_dir)? {
            let entry = entry?;
            let agent_dir = entry.path();
            if !agent_dir.is_dir() {
                continue;
            }
            let pod_db = agent_dir.join("pod.db");
            if !pod_db.exists() {
                continue;
            }
            // Derive PodID from agent directory name (matches PodFactory naming)
            if let Some(agent_name) = agent_dir.file_name().and_then(|n| n.to_str()) {
                ids.push(PodID::from_name(agent_name));
            }
        }
        Ok(ids)
    }

    /// Check if a pod exists on disk.
    /// Scans agent directories for matching name-based PodID.
    pub fn pod_exists(&self, pod_id: &PodID) -> bool {
        self.db_path(pod_id).exists()
    }

    /// Get the database path for a pod by PodID.
    /// The PodID is derived from the agent name, so we reverse-lookup
    /// by scanning agent directories.
    pub fn db_path(&self, pod_id: &PodID) -> PathBuf {
        // Try to find the agent directory matching this PodID.
        // PodID is name-derived, so we iterate to find the match.
        if let Ok(entries) = std::fs::read_dir(&self.agents_dir) {
            for entry in entries.flatten() {
                let agent_dir = entry.path();
                if !agent_dir.is_dir() {
                    continue;
                }
                if let Some(agent_name) = agent_dir.file_name().and_then(|n| n.to_str())
                    && PodID::from_name(agent_name) == *pod_id
                {
                    return agent_dir.join("pod.db");
                }
            }
        }
        // Fallback: construct path from PodID string representation
        self.agents_dir.join(pod_id.to_string()).join("pod.db")
    }

    /// Scan agent directories for pod databases, read kind from pod.kind sidecar.
    /// Returns (PodKind, original_agent_name, db_path).
    pub fn scan_by_kind(&self) -> Result<Vec<(PodKind, String, PathBuf)>, PodDeployError> {
        if !self.agents_dir.exists() {
            return Ok(Vec::new());
        }
        let mut results = Vec::new();
        for entry in std::fs::read_dir(&self.agents_dir)? {
            let entry = entry?;
            let agent_dir = entry.path();
            if !agent_dir.is_dir() {
                continue;
            }
            let pod_db = agent_dir.join("pod.db");
            if !pod_db.exists() {
                continue;
            }
            // Read pod_kind from pod.kind sidecar
            let kind = read_pod_kind(&pod_db).unwrap_or_default();
            // Read original agent name from pod.name sidecar.
            // Falls back to directory name if sidecar is missing.
            let agent_name = read_pod_name(&pod_db).unwrap_or_else(|| {
                agent_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            });
            results.push((kind, agent_name, pod_db));
        }
        Ok(results)
    }

    /// Find the CuratorPod. Returns None if none found, first if multiple.
    pub fn find_curator(&self) -> Option<PathBuf> {
        let path = self.agents_dir.join("curator").join("pod.db");
        if path.exists() { Some(path) } else { None }
    }

    /// Find all TeamPods.
    pub fn find_teams(&self) -> Result<Vec<(String, PathBuf)>, PodDeployError> {
        let entries = self.scan_by_kind()?;
        Ok(entries
            .into_iter()
            .filter(|(k, _, _)| *k == PodKind::Team)
            .map(|(_, stem, path)| (stem, path))
            .collect())
    }
}

/// Read the pod kind from the pod.kind sidecar file.
fn read_pod_kind(db_path: &std::path::Path) -> Option<PodKind> {
    let kind_path = db_path.with_extension("kind");
    let content = std::fs::read_to_string(&kind_path).ok()?;
    match content.trim() {
        "curator" => Some(PodKind::Curator),
        "team" => Some(PodKind::Team),
        "replicant" => Some(PodKind::Replicant),
        _ => None,
    }
}

/// Read the original (unsanitized) agent name from the pod.name sidecar.
fn read_pod_name(db_path: &std::path::Path) -> Option<String> {
    let name_path = db_path.with_extension("name");
    let content = std::fs::read_to_string(&name_path).ok()?;
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
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
        let template_loader = Arc::new(TemplateCrateLoader::from_path(data_dir.clone()));
        let consent = Arc::new(crate::DenyAllConsent);
        let factory = PodFactory::new(template_loader, consent, data_dir.clone());
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
        let registry = PodRegistry::new(temp.path());
        // Empty directory — no pods
        let ids = registry.list_pod_ids().expect("list");
        assert!(ids.is_empty());

        // Create a pod directory under agents/ (matching PodRegistry layout)
        let agents_dir = temp.path().join("agents");
        let pod_id = PodID::new();
        let pod_dir = agents_dir.join(pod_id.to_string());
        std::fs::create_dir_all(&pod_dir).expect("create pod dir");
        std::fs::write(pod_dir.join("pod.db"), b"").expect("write db file");

        let ids = registry.list_pod_ids().expect("list");
        assert_eq!(ids.len(), 1);
        assert!(registry.pod_exists(&pod_id));
        assert!(!registry.pod_exists(&PodID::new()));
    }

    #[test]
    fn pod_registry_scan_by_kind_classifies_files() {
        let temp = tempfile::TempDir::new().expect("tempdir");

        let agents_dir = temp.path().join("agents");

        let curator_dir = agents_dir.join("curator");
        std::fs::create_dir_all(&curator_dir).unwrap();
        std::fs::write(curator_dir.join("pod.db"), b"").unwrap();
        std::fs::write(curator_dir.join("pod.kind"), b"curator").unwrap();

        let team_dir = agents_dir.join("team.7r7");
        std::fs::create_dir_all(&team_dir).unwrap();
        std::fs::write(team_dir.join("pod.db"), b"").unwrap();
        std::fs::write(team_dir.join("pod.kind"), b"team").unwrap();

        let repl_dir = agents_dir.join("replicant.alice");
        std::fs::create_dir_all(&repl_dir).unwrap();
        std::fs::write(repl_dir.join("pod.db"), b"").unwrap();
        std::fs::write(repl_dir.join("pod.kind"), b"replicant").unwrap();

        let registry = PodRegistry::new(temp.path());
        let results = registry.scan_by_kind().expect("scan");
        assert_eq!(results.len(), 3);

        let curator: Vec<_> = results
            .iter()
            .filter(|(k, _, _)| *k == PodKind::Curator)
            .collect();
        assert_eq!(curator.len(), 1);

        let teams: Vec<_> = results
            .iter()
            .filter(|(k, _, _)| *k == PodKind::Team)
            .collect();
        assert_eq!(teams.len(), 1);
    }

    #[test]
    fn pod_registry_find_curator_returns_path() {
        let temp = tempfile::TempDir::new().expect("tempdir");

        let registry = PodRegistry::new(temp.path());
        assert!(registry.find_curator().is_none());

        let curator_dir = temp.path().join("agents").join("curator");
        std::fs::create_dir_all(&curator_dir).unwrap();
        std::fs::write(curator_dir.join("pod.db"), b"").unwrap();
        assert!(registry.find_curator().is_some());
    }

    #[test]
    fn pod_kind_defaults_to_replicant() {
        assert_eq!(PodKind::default(), PodKind::Replicant);
    }

    #[test]
    fn pod_kind_filenames_follow_convention() {
        // Curator: curator.db
        // Team: team.{name}.db
        // Replicant: replicant.{name}.db
        assert_eq!(format!("curator.db"), "curator.db");
        assert_eq!(format!("team.{}.db", "7r7"), "team.7r7.db");
        assert_eq!(format!("replicant.{}.db", "alice"), "replicant.alice.db");
    }
}
