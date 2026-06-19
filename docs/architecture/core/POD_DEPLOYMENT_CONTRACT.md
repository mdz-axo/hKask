---
title: "PodDeployment Contract — Per-Pod Deployment Unit"
audience: [architects, developers]
last_updated: 2026-06-18
version: "0.29.0"
status: "Implemented — PodDeployment is canonical. PodManager deleted."
domain: "Agent Pod Lifecycle"
principles: [P1, P4, P5, P6, P7, P11]
---

# PodDeployment Contract — Per-Pod Deployment Unit

**Purpose:** Define the deployment contract for a pod as the minimal viable container for one human+replicant pair. A pod IS the deployment unit — not a cache entry in a shared manager.

**Status:** Strangler-Fig Phase 1 (coexistence). `PodDeployment` and `PodFactory` introduced alongside existing `PodManager`. Both paths produce a functioning pod. Default: PodManager. Feature flag `--deployment-mode isolated` uses PodFactory.

---

## β.1 — Deployment Contract

A pod owns five dedicated resources. No sharing across pods. No service collision surface.

### (a) Dedicated SQLCipher Database File

**Property:** Per-pod, not shared. The file IS the pod's data.

- Path: `{data_dir}/pods/{pod_id}.db`
- Encryption: SQLCipher with per-pod key derived from master key (ADR-027)
- Backup: Copy the file. This was already the backup model ("Backup as portable archive. Encrypted SQLCipher file.")
- Migration: `kask pod migrate-storage <pod_id>` exports from shared TripleStore into per-pod file
- **Replaces:** Current `owner_webid`-scoped rows in shared `TripleStore`

### (b) Dedicated Keystore Root

**Property:** Per-pod HKDF-SHA256 derivation from user's master key.

- Deterministic: same master key + same WebID → same pod key material
- Portable: independent of which server the pod runs on
- Already designed: `derive_ocap_secret(webid)` in ADR-027
- **No change required.** Key derivation is already per-WebID.

### (c) Dedicated CNS Runtime

**Property:** Per-pod variety counters, algedonic thresholds, and span emission.

- Currently: `CnsRuntime` is server-global (one per process)
- Target: One `PerPodCnsRuntime` per pod
- Aggregation: Curator (VSM S4) polls or receives push from all pod CNS runtimes
- Span namespace extends: `cns.agent_pod.{pod_id}.tool.*`

### (d) Dedicated MCP Server Binding

**Property:** Each pod connects to its own MCP server instances.

- Stateless servers (inference, research): Virtual instance — same process, different capability token, OCAP-gated at dispatch
- Stateful servers (memory, kanban): Separate instance with pod-scoped storage
- No shared tool dispatch across pods
- `GovernedTool` already gates per-call via capability tokens; pod boundary makes this structural

### (e) Self-Contained Template Registry

**Property:** Pod carries its own FlowDef/KnowAct/WordAct manifests.

- Pattern A (Skills Model) is crate-level; pods inherit the crate's templates
- No change required. Templates are already crate-level, not PodManager-level.

---

## β.2 — Rust Type Definitions (Make Invalid States Unrepresentable)

The type system encodes what was previously a documentation intent: shared PodManager state is impossible to construct because `PodDeployment` owns its storage, CNS, and tools directly.

```rust
/// A pod IS the deployment unit. Constructing a PodDeployment
/// means: a database file exists, a keystore root is derived,
/// a CNS runtime is initialized, and MCP servers are bound.
/// No shared state. No service collision surface.
///
/// [P6] Goal: Space for Replicants — each replicant inhabits its own pod
/// [P11] Constraining: Digital Public/Private Sphere — per-pod SQLCipher boundary
/// [P4] Constraining: Clear Boundaries — OCAP tokens scoped to this pod
pub struct PodDeployment {
    /// Pod identity — WebID is the root of all authority (P1)
    pod_id: PodId,
    /// Dedicated database. The file IS the pod. No shared store.
    storage: PerPodStorage,
    /// Dedicated CNS runtime. Variety counters scoped to this pod.
    cns: PerPodCnsRuntime,
    /// MCP servers bound to this pod. No cross-pod tool dispatch.
    tools: PerPodToolBinding,
    /// Ephemeral state: runtime ports live as long as the pod is activated
    state: PodState,
}

/// PerPodStorage wraps a single SQLCipher connection to a pod-level
/// database file. The file path is deterministic: {data_dir}/pods/{pod_id}.db
/// This type makes "shared store" an invalid state — you cannot
/// accidentally query another pod's data.
///
/// [P11] Goal: Digital Public/Private Sphere — storage isolation
struct PerPodStorage {
    conn: SqlCipherConnection,
    pod_db_path: PathBuf, // {data_dir}/pods/{pod_id}.db
}

/// PerPodToolBinding owns the MCP server instances for this pod.
/// Each pod gets its own server processes (or in-process server
/// instances). No shared dispatch — tool calls go to this pod's
/// servers, governed by this pod's OCAP tokens.
///
/// [P4] Goal: Clear Boundaries — tool access is pod-scoped
struct PerPodToolBinding {
    servers: Vec<McpServerHandle>,
    governed_tool: GovernedTool<RawMcpToolPort>,
}
```

---

## β.3 — PodFactory (Stateless Constructor)

PodManager's `HashMap<PodID, AgentPod>` is deleted. `PodFactory` is a stateless constructor that produces `PodDeployment` instances.

```rust
/// PodFactory constructs PodDeployment instances from templates.
/// It is stateless — it does not cache, pool, or share pods.
/// The factory creates; the caller owns.
///
/// [P5] Goal: Essentialism — factory only; no runtime cache
/// [P7] Constraining: Evolutionary Architecture — seam for future pod types
pub struct PodFactory {
    /// Template resolution: loads crate-level manifests
    template_resolver: Arc<TemplateResolver>,
    /// Master key for deterministic per-pod key derivation (ADR-027)
    key_material: Arc<KeyMaterial>,
    /// Configuration shared across pods (server address, etc.)
    server_config: PodServerConfig,
}

impl PodFactory {
    /// Create a new pod deployment. Returns a fully-initialized pod
    /// with its own storage, CNS, and tool bindings.
    ///
    /// pre:  template_name resolves to a valid template crate
    ///       persona is validated
    ///       data_dir is writable
    /// post: PodDeployment with dedicated SQLCipher file at
    ///       {data_dir}/pods/{pod_id}.db, dedicated CNS runtime,
    ///       and per-pod MCP server bindings
    pub async fn deploy(
        &self,
        template_name: &str,
        persona: &AgentPersona,
    ) -> Result<PodDeployment, PodDeployError> {
        // 1. Derive pod_id deterministically from WebID + template
        // 2. Create {data_dir}/pods/{pod_id}.db (SQLCipher)
        // 3. Derive pod key material via HKDF-SHA256(webid, master_key)
        // 4. Initialize PerPodCnsRuntime (variety counters, thresholds)
        // 5. Bind MCP servers for this pod
        // 6. Return PodDeployment
        todo!("strangler-fig: implement alongside existing PodManager")
    }
}
```

---

## β.4 — Service Collision Elimination

| Current (Shared) | Target (Per-Pod) | Mechanism |
|-----------------|------------------|-----------|
| One `mcp_runtime: Arc<dyn MCPRuntimePort>` on PodManager | `PerPodToolBinding` with pod-scoped handles | Stateless servers: same process, different OCAP token. Stateful: separate instance. |
| One `episodic_storage: Arc<dyn EpisodicStoragePort>` | `PerPodStorage` with pod-level SQLCipher file | One database file per pod at `{data_dir}/pods/{pod_id}.db` |
| One `semantic_storage: Arc<dyn SemanticStoragePort>` | Consolidated into per-pod storage | Semantic triples stored in pod's own database |
| One `inference_port: Option<Arc<dyn InferencePort>>` | Per-pod inference port (API key scoped to user) | PodDeployment holds its own inference handle |
| `CnsRuntime` server-global | `PerPodCnsRuntime` per pod | CNS span namespace: `cns.agent_pod.{pod_id}.*`. Curator aggregates across pods. |

---

## β.5 — Essentialist Deletion Test on PodManager's HashMap

### Delete `pods: Arc<RwLock<HashMap<PodID, AgentPod>>>` from PodManager

**Question:** Does behavior vanish?
- Yes — pods can't be looked up centrally.
- But lookup was already the wrong pattern: the pod should be a deployment unit owned by the user, not a cache entry looked up by the server.

**Replacement:** Pod metadata in a lightweight index (`pod_id → disk path, activation status`). Not a cache. Not shared state. Just a directory listing of deployed pods.

**Verdict:** PodManager's HashMap is a **pass-through cache** — delete it. PodFactory is the replacement.

### Deletion Test Outcomes

| Artifact | G1 (Exist) | G2 (Surface) | G3 (Contract) | Verdict |
|----------|-----------|-------------|---------------|---------|
| `PodManager::pods: HashMap` | **FAIL** — behavior reappears as directory listing, not cache | N/A (already deleted) | N/A | **DELETE**. Replace with lightweight index. |
| `PodManager` (entire struct) | **PASS** during strangler-fig — existing code depends on it | 19 public methods — **FAIL**. Many are pass-through accessors. | **FAIL** — `create_pod` delegates to `AgentPod::new` then inserts into HashMap. | **STRANGLER-FIG**. Migrate to PodFactory, then delete. |
| `PodFactory` (proposed) | **PASS** — behavior (pod construction) would reappear in callers if deleted | 1 public method (`deploy`) — **PASS** | **PASS** — encapsulates database creation, key derivation, CNS init, MCP binding | **CREATE**. Earns its existence. |

---

## Next: Task Group γ — Strangler-Fig Migration Plan

See [`STRANGLER_FIG_MIGRATION.md`](STRANGLER_FIG_MIGRATION.md) for the phased migration plan.
