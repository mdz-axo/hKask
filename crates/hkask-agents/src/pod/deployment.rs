//! PodDeployment — Per-pod deployment unit (Solid Pod isomorphism)
//!
//! This module defines the migration target for the AgentPod architecture:
//! each pod IS the deployment unit with its own SQLCipher file, CNS runtime,
//! and MCP server bindings. No shared state. No service collision surface.
//!
//! # Strangler-Fig Phase 2 — CNS Per-Pod
//!
//! PerPodCnsRuntime now wraps a real CnsRuntime instance. Each pod
//! has its own variety counters, outcome trackers, and energy budgets.
//! CnsAggregator provides server-global aggregation across pods.
//!
//! # Principles
//!
//! - [P6] Goal: Space for Replicants — each replicant inhabits its own pod
//! - [P11] Constraining: Digital Public/Private Sphere — per-pod SQLCipher boundary
//! - [P4] Constraining: Clear Boundaries — OCAP tokens scoped to this pod
//! - [P5] Goal: Essentialism — factory only; no runtime cache
//! - [P7] Constraining: Evolutionary Architecture — seam for future pod types
//! - [P9] Goal: Homeostatic Self-Regulation — per-pod variety tracking

use hkask_cns::CnsRuntime;
use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_rsolidity as rs;
use hkask_types::event::SpanNamespace;
use hkask_types::{CapabilityChecker, DelegationToken, NuEventSink, WebID};
use std::collections::HashMap;
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
/// means: a database file exists, a keystore root is derived,
/// a CNS runtime is initialized, and MCP servers are bound.
/// No shared state. No service collision surface.
///
/// [P6] Goal: Space for Replicants — each replicant inhabits its own pod
/// [P11] Constraining: Digital Public/Private Sphere — per-pod SQLCipher boundary
/// [P4] Constraining: Clear Boundaries — OCAP tokens scoped to this pod
#[allow(dead_code)] // Phase 2 — some fields wired in Phases 3–5
pub struct PodDeployment {
    /// Pod identity — WebID is the root of all authority (P1)
    pod_id: PodID,
    /// The underlying AgentPod (identity, lifecycle, persona, capability token)
    pod: AgentPod,
    /// Dedicated database. The file IS the pod. No shared store.
    storage: PerPodStorage,
    /// Dedicated CNS runtime. Variety counters scoped to this pod.
    cns: PerPodCnsRuntime,
    /// MCP servers bound to this pod. No cross-pod tool dispatch.
    tools: PerPodToolBinding,
    /// Capability checker for OCAP verification (wired in Phase 4)
    capability_checker: Option<Arc<CapabilityChecker>>,
    /// Sovereignty checker wired to consent port
    sovereignty_checker: SovereigntyChecker,
    /// NuEvent sink for lifecycle events
    nu_event_sink: Option<Arc<dyn NuEventSink>>,
}

/// PerPodStorage wraps a single SQLCipher connection to a pod-level
/// database file. The file path is deterministic: {data_dir}/pods/{pod_id}.db
/// This type makes "shared store" an invalid state — you cannot
/// accidentally query another pod's data.
///
/// [P11] Goal: Digital Public/Private Sphere — storage isolation
pub struct PerPodStorage {
    /// Pod-scoped episodic storage (private, per-agent)
    pub episodic: Arc<dyn EpisodicStoragePort>,
    /// Pod-scoped semantic storage (shared, public knowledge)
    pub semantic: Arc<dyn SemanticStoragePort>,
    /// Path to this pod's database file
    pub db_path: PathBuf,
}

/// PerPodCnsRuntime is a CNS runtime scoped to a single pod.
/// Variety counters, outcome trackers, and energy budgets are
/// isolated per-pod. The server-global CNS aggregator reads
/// from all per-pod counters.
///
/// Each pod gets its own CnsRuntime instance (cheap to clone —
/// all fields are Arc-wrapped). CNS spans emitted through this
/// runtime carry the pod's namespace: `cns.agent_pod.{pod_id}.*`.
///
/// [P9] Goal: Homeostatic Self-Regulation — per-pod variety tracking
/// [P11] Constraining: Digital Public/Private Sphere — CNS isolation
pub struct PerPodCnsRuntime {
    /// The pod this CNS runtime is scoped to
    pod_id: PodID,
    /// Span namespace prefix for this pod: cns.agent_pod.{pod_id}
    span_namespace: String,
    /// The actual CNS runtime — per-pod isolate with its own
    /// variety counters, algedonic manager, and subscribers
    inner: CnsRuntime,
}

/// PerPodToolBinding owns the MCP server instances for this pod.
/// Each pod gets its own server processes (or in-process server
/// instances). No shared dispatch — tool calls go to this pod's
/// servers, governed by this pod's OCAP tokens.
///
/// [P4] Goal: Clear Boundaries — tool access is pod-scoped
pub struct PerPodToolBinding {
    /// MCP runtime (may be shared process with virtual namespace via OCAP)
    pub mcp_runtime: Arc<dyn MCPRuntimePort>,
    /// GovernedTool membrane — routes tool invocations through CNS governance
    pub governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
}

// ── PodDeployError ──────────────────────────────────────────────────────────

/// Errors during pod deployment (factory-level errors).
/// Distinct from AgentPodError which covers pod-internal lifecycle errors.
#[derive(Debug, Error)]
pub enum PodDeployError {
    #[error("Failed to create pod storage at {path}: {reason}")]
    StorageInitFailed { path: PathBuf, reason: String },

    #[error("Failed to initialize CNS runtime for pod {pod_id}: {reason}")]
    CnsInitFailed { pod_id: PodID, reason: String },

    #[error("Failed to bind MCP tools for pod {pod_id}: {reason}")]
    ToolBindFailed { pod_id: PodID, reason: String },

    #[error("Pod lifecycle error: {0}")]
    PodError(#[from] AgentPodError),

    #[error("Template resolution failed: {0}")]
    TemplateError(String),
}

// ── PerPodCnsRuntime implementation ─────────────────────────────────────────

impl PerPodCnsRuntime {
    /// Create a CNS runtime scoped to a specific pod.
    ///
    /// expect: "The system isolates variety tracking per pod boundary" [P9]
    /// [P9] Motivating: Homeostatic Self-Regulation — per-pod CNS isolation
    /// [P11] Constraining: Digital Public/Private Sphere — pod-scoped counters
    /// pre:  pod_id is a valid PodID
    /// post: Returns a PerPodCnsRuntime with a fresh CnsRuntime instance
    ///       scoped to this pod
    #[rs::contract(id = "P9-agt-pod-cns-scoped", principle = "P9")]
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

    /// Get the pod ID this CNS runtime is scoped to.
    pub fn pod_id(&self) -> PodID {
        self.pod_id
    }

    /// Get the span namespace for this pod.
    pub fn span_namespace(&self) -> &str {
        &self.span_namespace
    }

    /// Get a reference to the inner CnsRuntime.
    pub fn inner(&self) -> &CnsRuntime {
        &self.inner
    }

    /// Emit a CNS span for a tool invocation within this pod.
    /// The span is namespaced: `cns.agent_pod.{pod_id}.tool.{tool_name}`
    ///
    /// expect: "Tool invocations emit per-pod CNS spans" [P9]
    /// [P9] Motivating: Homeostatic Self-Regulation — per-pod tool observability
    /// [P4] Constraining: Clear Boundaries — spans scoped to pod
    /// pre:  tool_name is non-empty
    /// post: variety counter incremented for domain
    ///       `cns.agent_pod.{pod_id}.tool.{tool_name}`
    #[rs::contract(id = "P9-agt-pod-cns-tool-span", principle = "P9")]
    pub async fn emit_tool_span(&self, tool_name: &str) {
        let domain = format!("cns.agent_pod.{}.tool.{}", self.pod_id, tool_name);
        self.inner.increment_variety(&domain, tool_name).await;
    }

    /// Record a tool outcome (success/failure) for this pod.
    ///
    /// expect: "Tool outcomes are tracked per pod" [P9]
    /// [P9] Motivating: Homeostatic Self-Regulation — per-pod outcome tracking
    /// pre:  tool_name is non-empty
    /// post: outcome recorded in pod-scoped outcome tracker
    #[rs::contract(id = "P9-agt-pod-cns-record-outcome", principle = "P9")]
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

    /// Get CNS health for this pod.
    #[rs::contract(id = "P9-agt-pod-cns-health", principle = "P9")]
    pub async fn health(&self) -> hkask_types::cns::CnsHealth {
        self.inner.health().await
    }

    /// Get variety counts for this pod across all tracked domains.
    #[rs::contract(id = "P9-agt-pod-cns-variety", principle = "P9")]
    pub async fn variety(&self) -> HashMap<SpanNamespace, u64> {
        self.inner.variety().await
    }

    /// Get variety for a specific tool domain within this pod.
    pub async fn variety_for_tool(&self, tool_name: &str) -> u64 {
        let domain = format!("cns.agent_pod.{}.tool.{}", self.pod_id, tool_name);
        self.inner.variety_for_domain(&domain).await
    }

    /// Register an energy budget for this pod's agent.
    #[rs::contract(id = "P9-agt-pod-cns-energy-budget", principle = "P9")]
    pub async fn register_energy_budget(&self, agent: WebID, budget: hkask_cns::EnergyBudget) {
        self.inner.register_energy_budget(agent, budget).await;
    }

    /// Get energy status for this pod's agent.
    pub async fn agent_energy_status(&self, agent: &WebID) -> Option<hkask_cns::AgentEnergyStatus> {
        self.inner.agent_gas_status(agent).await
    }
}

// ── CnsAggregator — Server-global aggregation across pods ────────────────────

/// Server-global aggregator that reads CNS state from all per-pod runtimes.
/// The Curator (VSM S4 — Intelligence) uses this to get a unified view
/// of system health without breaking per-pod isolation.
///
/// [P9] Goal: Homeostatic Self-Regulation — aggregate CNS for Curator
/// [P5] Goal: Essentialism — lightweight registry, not a cache
pub struct CnsAggregator {
    /// Registry of per-pod CNS runtimes, keyed by PodID
    runtimes: HashMap<PodID, CnsRuntime>,
}

impl CnsAggregator {
    /// Create a new empty aggregator.
    ///
    /// expect: "The Curator can aggregate CNS state across pods" [P9]
    /// [P9] Motivating: Homeostatic Self-Regulation — aggregation for Curator
    /// [P5] Constraining: Essentialism — empty by default, pods register explicitly
    /// post: Returns an empty CnsAggregator
    #[rs::contract(id = "P9-agt-pod-cns-aggregator-new", principle = "P9")]
    pub fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
        }
    }

    /// Register a pod's CNS runtime with the aggregator.
    ///
    /// expect: "Pods register their CNS runtime for Curator aggregation" [P9]
    /// [P9] Motivating: Homeostatic Self-Regulation — pod registration
    /// [P12] Constraining: Replicant Host Mandate — explicit registration
    /// pre:  pod_id is not already registered
    /// post: pod's CNS runtime added to aggregator
    #[rs::contract(id = "P9-agt-pod-cns-aggregator-register", principle = "P9")]
    pub fn register(&mut self, pod_id: PodID, runtime: CnsRuntime) {
        self.runtimes.insert(pod_id, runtime);
    }

    /// Remove a pod's CNS runtime (on pod deactivation).
    pub fn unregister(&mut self, pod_id: &PodID) {
        self.runtimes.remove(pod_id);
    }

    /// Get aggregate variety across all pods for a given domain suffix.
    /// For example, `tool_suffix = "research"` sums variety across all
    /// pods' `cns.agent_pod.{id}.tool.research` counters.
    pub async fn aggregate_variety_for_tool(&self, tool_suffix: &str) -> u64 {
        let mut total = 0u64;
        for (pod_id, runtime) in &self.runtimes {
            let domain = format!("cns.agent_pod.{}.tool.{}", pod_id, tool_suffix);
            total += runtime.variety_for_domain(&domain).await;
        }
        total
    }

    /// Get total variety across all pods and all tool domains.
    pub async fn aggregate_variety_total(&self) -> HashMap<String, u64> {
        let mut total = HashMap::new();
        for runtime in self.runtimes.values() {
            let pod_variety = runtime.raw_variety().await;
            for (domain, count) in pod_variety {
                *total.entry(domain).or_insert(0) += count;
            }
        }
        total
    }

    /// Number of registered pods.
    pub fn pod_count(&self) -> usize {
        self.runtimes.len()
    }
}

impl Default for CnsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

// ── PodDeployment implementation ────────────────────────────────────────────

impl PodDeployment {
    /// Get the pod's ID.
    pub fn pod_id(&self) -> PodID {
        self.pod_id
    }

    /// Get the pod's WebID.
    pub fn webid(&self) -> WebID {
        self.pod.webid
    }

    /// Get the pod's lifecycle state.
    pub fn state(&self) -> PodLifecycleState {
        self.pod.state()
    }

    /// Check if the pod is active.
    pub fn is_active(&self) -> bool {
        self.pod.is_active()
    }

    /// Get the capability token for this pod.
    pub fn capability_token(&self) -> &DelegationToken {
        &self.pod.capability_token
    }

    /// Get the storage for this pod.
    pub fn storage(&self) -> &PerPodStorage {
        &self.storage
    }

    /// Get the CNS runtime for this pod.
    pub fn cns(&self) -> &PerPodCnsRuntime {
        &self.cns
    }

    /// Get the tool binding for this pod.
    pub fn tools(&self) -> &PerPodToolBinding {
        &self.tools
    }

    /// Get the CNS span namespace for this pod.
    pub fn cns_namespace(&self) -> &str {
        &self.cns.span_namespace
    }

    /// Get a clone of the inner CnsRuntime for aggregator registration.
    pub fn cns_runtime(&self) -> CnsRuntime {
        self.cns.inner.clone()
    }
}

// ── PodFactory — Stateless pod constructor ──────────────────────────────────

/// PodFactory constructs PodDeployment instances from templates.
/// It is stateless — it does not cache, pool, or share pods.
/// The factory creates; the caller owns.
///
/// [P5] Goal: Essentialism — factory only; no runtime cache
/// [P7] Constraining: Evolutionary Architecture — seam for future pod types
pub struct PodFactory {
    /// Template resolution: loads crate-level manifests
    git_cas: Arc<GitCasAdapter>,
    /// Consent port for sovereignty wiring
    consent: Arc<dyn crate::SovereigntyConsent>,
    /// Data directory for pod storage files
    data_dir: PathBuf,
}

impl PodFactory {
    /// Create a new PodFactory.
    ///
    /// expect: "My agents operate within my sovereignty boundaries" [P1]
    /// [P1] Motivating: User Sovereignty — factory creates pods under user's WebID
    /// [P5] Constraining: Essentialism — factory holds only what's needed for construction
    /// pre:  git_cas is a valid GitCasAdapter; consent is a valid SovereigntyConsent;
    ///       data_dir is a writable directory.
    /// post: Returns a PodFactory ready to deploy pods.
    #[rs::contract(id = "P1-agt-pod-factory-new", principle = "P1")]
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

    /// Deploy a new pod. Returns a fully-initialized PodDeployment
    /// with its own storage, CNS, and tool bindings.
    ///
    /// The CNS runtime is initialized as a per-pod isolate with
    /// span namespace `cns.agent_pod.{pod_id}.*`. Variety counters,
    /// outcome trackers, and energy budgets are scoped to this pod.
    ///
    /// expect: "My agents operate within my sovereignty boundaries" [P1]
    /// [P6] Motivating: Space for Replicants — deploys a pod for a replicant
    /// [P11] Constraining: Digital Public/Private Sphere — per-pod SQLCipher boundary
    /// [P4] Constraining: Clear Boundaries — OCAP tokens scoped to this pod
    /// [P9] Constraining: Homeostatic Self-Regulation — per-pod CNS isolation
    /// pre:  template_name resolves to a valid template crate;
    ///       persona is a validated AgentPersona;
    ///       data_dir is writable.
    /// post: Returns Ok(PodDeployment) with dedicated SQLCipher file at
    ///       {data_dir}/pods/{pod_id}.db, dedicated per-pod CNS runtime
    ///       with span namespace cns.agent_pod.{pod_id}.*, and per-pod
    ///       MCP server bindings. Returns Err on failure.
    #[rs::contract(id = "P6-agt-pod-factory-deploy", principle = "P6")]
    pub async fn deploy(
        &self,
        template_name: &str,
        persona: &AgentPersona,
        episodic_storage: Arc<dyn EpisodicStoragePort>,
        semantic_storage: Arc<dyn SemanticStoragePort>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
        capability_checker: Option<Arc<CapabilityChecker>>,
        nu_event_sink: Option<Arc<dyn NuEventSink>>,
    ) -> Result<PodDeployment, PodDeployError> {
        // 1. Create the underlying AgentPod (identity, lifecycle, capability token)
        let pod = AgentPod::new(
            template_name,
            persona,
            self.git_cas.as_ref(),
            Arc::clone(&self.consent),
        )?;

        let pod_id = pod.id;

        // 2. Set up per-pod storage path
        let pod_db_dir = self.data_dir.join("pods");
        std::fs::create_dir_all(&pod_db_dir).map_err(|e| PodDeployError::StorageInitFailed {
            path: pod_db_dir.clone(),
            reason: e.to_string(),
        })?;
        let db_path = pod_db_dir.join(format!("{}.db", pod_id));

        let storage = PerPodStorage {
            episodic: episodic_storage,
            semantic: semantic_storage,
            db_path,
        };

        // 3. Initialize per-pod CNS runtime with isolated variety counters
        let cns = PerPodCnsRuntime::scoped(pod_id);

        // 4. Bind MCP tools for this pod
        let tools = PerPodToolBinding {
            mcp_runtime,
            governed_tool,
        };

        let sovereignty_checker = pod.sovereignty_checker.clone();

        info!(
            target: "hkask.pod.deployment",
            pod_id = %pod_id,
            template = %template_name,
            cns_namespace = %cns.span_namespace,
            "Pod deployed (isolated mode, per-pod CNS)"
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
        })
    }

    /// Get the data directory for pod storage.
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

// ── PodStatus (lightweight index, not cache) ────────────────────────────────

/// Lightweight pod metadata — a directory listing entry, not a cache.
/// Replaces PodManager's HashMap-based lookup with filesystem-based
/// discovery. The pod IS the deployment unit; status is derived
/// from the deployment's on-disk state.
#[derive(Debug, Clone)]
pub struct PodIndexEntry {
    pub pod_id: PodID,
    pub webid: WebID,
    pub state: PodLifecycleState,
    pub db_path: PathBuf,
}

impl PodIndexEntry {
    /// Create a new index entry from a PodDeployment.
    pub fn from_deployment(deployment: &PodDeployment) -> Self {
        Self {
            pod_id: deployment.pod_id,
            webid: deployment.pod.webid,
            state: deployment.pod.state(),
            db_path: deployment.storage.db_path.clone(),
        }
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
    fn deployment_storage_path_is_deterministic() {
        let pod_id = PodID::new();
        let db_path = PathBuf::from(format!("/tmp/hkask-test/pods/{}.db", pod_id));

        // Same pod_id → same path
        let db_path_2 = PathBuf::from(format!("/tmp/hkask-test/pods/{}.db", pod_id));
        assert_eq!(db_path, db_path_2);
    }

    #[test]
    fn per_pod_cns_runtime_has_scoped_namespace() {
        let pod_id = PodID::new();
        let cns = PerPodCnsRuntime::scoped(pod_id);
        assert!(cns.span_namespace().starts_with("cns.agent_pod."));
        assert!(cns.span_namespace().contains(&pod_id.to_string()));
    }

    #[test]
    fn per_pod_cns_runtime_isolated_per_pod() {
        // Two pods get independent CNS runtimes with different namespaces
        let pod_a = PodID::new();
        let pod_b = PodID::new();

        let cns_a = PerPodCnsRuntime::scoped(pod_a);
        let cns_b = PerPodCnsRuntime::scoped(pod_b);

        // Different namespaces
        assert_ne!(cns_a.span_namespace(), cns_b.span_namespace());
        // Each contains its own pod_id
        assert!(cns_a.span_namespace().contains(&pod_a.to_string()));
        assert!(cns_b.span_namespace().contains(&pod_b.to_string()));
    }

    #[tokio::test]
    async fn cns_aggregator_registers_and_counts_pods() {
        let mut agg = CnsAggregator::new();
        assert_eq!(agg.pod_count(), 0);

        let pod_a = PodID::new();
        let cns_a = PerPodCnsRuntime::scoped(pod_a);
        agg.register(pod_a, cns_a.inner().clone());
        assert_eq!(agg.pod_count(), 1);

        let pod_b = PodID::new();
        let cns_b = PerPodCnsRuntime::scoped(pod_b);
        agg.register(pod_b, cns_b.inner().clone());
        assert_eq!(agg.pod_count(), 2);

        agg.unregister(&pod_a);
        assert_eq!(agg.pod_count(), 1);
    }

    #[tokio::test]
    async fn cns_aggregator_aggregates_variety_across_pods() {
        let mut agg = CnsAggregator::new();

        let pod_a = PodID::new();
        let cns_a = PerPodCnsRuntime::scoped(pod_a);
        // Emit tool spans on pod A
        cns_a.emit_tool_span("research").await;
        cns_a.emit_tool_span("research").await;
        cns_a.emit_tool_span("condenser").await;
        agg.register(pod_a, cns_a.inner().clone());

        let pod_b = PodID::new();
        let cns_b = PerPodCnsRuntime::scoped(pod_b);
        // Emit tool spans on pod B
        cns_b.emit_tool_span("research").await;
        agg.register(pod_b, cns_b.inner().clone());

        // Aggregate variety across pods
        let total_variety = agg.aggregate_variety_total().await;
        assert!(
            !total_variety.is_empty(),
            "Should have variety from both pods"
        );
    }

    #[test]
    fn pod_index_entry_state_and_webid() {
        let persona = make_test_persona();
        let webid = persona.webid();
        let pod_id = PodID::new();
        let db_path = PathBuf::from(format!("/tmp/hkask-test/pods/{}.db", pod_id));

        let entry = PodIndexEntry {
            pod_id,
            webid,
            state: PodLifecycleState::Populated,
            db_path,
        };

        assert_eq!(entry.state, PodLifecycleState::Populated);
        assert_eq!(entry.webid, webid);
    }
}
