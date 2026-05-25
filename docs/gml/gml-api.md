---
title: "GML API Reference (Aspirational)"
audience: [developers]
last_updated: 2026-05-24
version: "0.1.0"
status: "Draft"
domain: "Application"
---

# GML API Reference

**Status:** Draft — This document describes a planned API for separate GML crates. The current implementation lives in `hkask-mcp-gml`. See `mcp-servers/hkask-mcp-gml/src/` for the actual API.

---

## Contents

| Section | Description |
|---------|-------------|
| [hkask-gml-types](#hkask-gml-types) | ID types, parameters, domain structs |
| [hkask-gml-core](#hkask-gml-core) | Concept system, effector, distribution, coherence |
| [hkask-gml-cascade](#hkask-gml-cascade) | Cascade selection and regulation logic |
| [hkask-gml-storage](#hkask-gml-storage) | Persistence, queries, and sqlite-vec |
| [hkask-gml-template](#hkask-gml-template) | GML prompt template integration |
| [hkask-gml-embedding](#hkask-gml-embedding) | Embedding generation and similarity |
| [hkask-mcp-gml](#hkask-mcp-gml) | MCP server implementation for GML |
| [CNS Integration](#cns-integration) | GML-specific CNS spans |
| [See Also](#see-also) | Related documentation |

---

## hkask-gml-types

Domain types implementing the MWC allosteric formalism [^mwc1965].

### Module: `ids`

```rust
pub type ConceptId = HkaskId;
pub type PortId = HkaskId;
pub type EffectorId = HkaskId;
pub type InterpretationId = HkaskId;
pub type NetworkId = HkaskId;
pub type EdgeId = HkaskId;

pub fn new_concept_id(name: &str) -> ConceptId;
pub fn new_port_id(name: &str) -> PortId;
pub fn new_effector_id(name: &str) -> EffectorId;
pub fn new_network_id(name: &str) -> NetworkId;
```

### Module: `parameters`

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MwcParameters {
    pub l: f64,        // allosteric constant [T]_0/[R]_0
    pub c: f64,        // affinity ratio K_R/K_T
    pub n: usize,      // number of binding sites
    pub alpha: f64,    // normalized ligand concentration
}

impl MwcParameters {
    pub fn new(l: f64, c: f64, n: usize, alpha: f64) -> Result<Self, MwcError>;
    pub fn default_bias(l: f64) -> Self;
}

#[derive(Debug, thiserror::Error)]
pub enum MwcError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("State function out of range: {0}")]
    StateFunctionOutOfRange(f64),
}
```

### Module: `concept`

```rust
#[derive(Debug, Clone)]
pub struct Interpretation {
    pub id: InterpretationId,
    pub description: String,
    pub energy: f64,
    pub state_type: StateType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StateType {
    T,  // Tense/conservative/closed
    R,  // Relaxed/progressive/open
}

#[derive(Debug, Clone)]
pub struct AllostericPort {
    pub id: PortId,
    pub name: String,
    pub effector_shape: EffectorShape,
    pub bound_effector: Option<Effector>,
    pub affinity_c: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectorShape {
    SecurityThreat,
    EconomicCondition,
    Evidence,
    Feedback,
    Challenge,
    TechnologyChange,
    SocialNorm,
    Custom(String),
}

impl EffectorShape {
    pub fn compatible(&self, other: &EffectorShape) -> bool;
}

#[derive(Debug, Clone)]
pub struct ConceptualSystem {
    pub id: ConceptId,
    pub name: String,
    pub t_state: Interpretation,
    pub r_state: Interpretation,
    pub l: f64,
    pub ports: Vec<AllostericPort>,
    pub current_alpha: f64,
    pub current_r_bar: f64,
}
```

### Module: `effector`

```rust
#[derive(Debug, Clone)]
pub struct Effector {
    pub id: EffectorId,
    pub name: String,
    pub concentration: f64,
    pub effect_type: EffectType,
    pub shape: EffectorShape,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectType {
    Activator,   // stabilizes R-state (c < 1)
    Inhibitor,   // stabilizes T-state (c > 1)
    Neutral,     // shifts without preference (c = 1)
}
```

### Module: `distribution`

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Distribution {
    pub p_r: f64,    // probability of R-state
    pub p_t: f64,    // probability of T-state
    pub n_h: f64,    // Hill coefficient
}

impl Distribution {
    pub fn new(p_r: f64, p_t: f64, n_h: f64) -> Self;
    pub fn from_mwc(l: f64, c: f64, n: usize, alpha: f64) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coherence {
    pub score: f64,
    pub stability: Stability,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stability {
    Stable,
    Transitioning,
    Unstable,
}

impl Coherence {
    pub fn from_score(score: f64) -> Self;
}
```

### Module: `network`

```rust
#[derive(Debug, Clone)]
pub struct Network {
    pub id: NetworkId,
    pub concepts: Vec<ConceptualSystem>,
    pub edges: Vec<ConceptEdge>,
}

#[derive(Debug, Clone)]
pub struct ConceptEdge {
    pub from: ConceptId,
    pub to: ConceptId,
    pub edge_type: EdgeType,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeType {
    Cooperative,
    Competitive,
    Neutral,
}
```

### Module: `capability`

```rust
#[derive(Debug, Clone)]
pub struct GmlCapability {
    pub id: CapabilityId,
    pub issuer: HkaskId,
    pub subject: HkaskId,
    pub scope: CapabilityScope,
    pub operations: Vec<GmlOperation>,
    pub effector_budget: Option<f64>,
    pub ports_allowed: Vec<PortId>,
    pub concepts_allowed: Vec<ConceptId>,
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityScope {
    Private,
    SharedRead,
    SharedWrite,
    Public,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GmlOperation {
    Recognize,
    Bind,
    Equilibrate,
    Cooperate,
    Inhibit,
    Activate,
    Homeostasis,
}

impl GmlCapability {
    pub fn new_root(
        issuer: HkaskId,
        subject: HkaskId,
        scope: CapabilityScope,
        operations: Vec<GmlOperation>,
    ) -> Self;
    
    pub fn attenuate(&self, new_issuer: HkaskId, new_subject: HkaskId) -> Self;
    pub fn allows_operation(&self, op: GmlOperation) -> bool;
    pub fn allows_port(&self, port_id: &PortId) -> bool;
    pub fn allows_concept(&self, concept_id: &ConceptId) -> bool;
    pub fn check_effector_budget(&self, concentration: f64) -> bool;
    pub fn verify(&self) -> bool;
}

pub struct CapabilityChecker {
    current_time: i64,
}

impl CapabilityChecker {
    pub fn new() -> Self;
    
    pub fn check(
        &self,
        capability: &GmlCapability,
        operation: GmlOperation,
        concept_id: &ConceptId,
        port_id: Option<&PortId>,
        effector_concentration: Option<f64>,
    ) -> CapabilityCheck;
}

pub enum CapabilityCheck {
    Allowed,
    Denied(CapabilityDenialReason),
}

pub enum CapabilityDenialReason {
    OperationNotAllowed(GmlOperation),
    PortNotAllowed(PortId),
    ConceptNotAllowed(ConceptId),
    EffectorBudgetExceeded { requested: f64, allowed: f64 },
    ScopeViolation { required: CapabilityScope, has: CapabilityScope },
    Expired,
    InvalidSignature,
    NoCapability,
}
```

---

## hkask-gml-core

Core algebra implementing the MWC state function and Hill cooperativity coefficient [^mwc1965][^hill1910].

### Module: `mwc`

```rust
/// MWC state function: R̄ = (1 + α)ⁿ / ((1 + α)ⁿ + L·(1 + cα)ⁿ)
pub fn mwc_state_function(l: f64, c: f64, n: usize, alpha: f64) -> f64;

/// Hill coefficient: n_H = n · (1-c)/(1+c) · √(α/(1+α))
pub fn hill_coefficient(n: usize, c: f64, alpha: f64) -> f64;

/// Partition function: Z = (1 + α)ⁿ + L·(1 + cα)ⁿ
pub fn partition_function(l: f64, c: f64, n: usize, alpha: f64) -> f64;

/// Binding function: Ȳ = fractional occupancy
pub fn binding_function(l: f64, c: f64, n: usize, alpha: f64) -> f64;
```

### Module: `boltzmann`

```rust
/// Boltzmann factor: L = exp(-(E_T - E_R)/kT)
pub fn boltzmann_factor(e_t: f64, e_r: f64, kt: f64) -> f64;

/// Probability from Boltzmann: P(state) = exp(-E/kT) / Z
pub fn boltzmann_probability(energy: f64, kt: f64, z: f64) -> f64;

/// Energy difference from L: ΔE = -kT · ln(L)
pub fn energy_from_l(l: f64, kt: f64) -> f64;

/// L from energy difference: L = exp(ΔE/kT)
pub fn l_from_energy(delta_e: f64, kt: f64) -> f64;
```

### Module: `algebra`

```rust
pub trait GmlAlgebra {
    fn bind(&self, concept: &ConceptualSystem, effector: &Effector) -> Result<ConceptualSystem>;
    fn equilibrium(&self, concept: &ConceptualSystem) -> Distribution;
    fn cooperate(&self, a: &ConceptualSystem, b: &ConceptualSystem) -> f64;
    fn inhibit(&self, concept: &ConceptualSystem, inhibitor: &Effector) -> Result<ConceptualSystem>;
    fn activate(&self, concept: &ConceptualSystem, activator: &Effector) -> Result<ConceptualSystem>;
    fn homeostasis(&self, network: &Network) -> Coherence;
    fn hill_coefficient(&self, params: &MwcParameters) -> f64;
}

pub struct GmlEngine {
    pub capability: GmlCapability,
    pub checker: CapabilityChecker,
    pub audit_storage: Box<dyn AuditLogStorage>,
}

impl GmlEngine {
    pub fn new(capability: GmlCapability) -> Self;
    pub fn default() -> Self;
}
```

### Module: `error`

```rust
#[derive(Debug, thiserror::Error)]
pub enum GmlError {
    #[error("Capability denied for operation: {0}")]
    CapabilityDenied(GmlOperation),
    
    #[error("No compatible port for effector")]
    NoCompatiblePort,
    
    #[error("Effector budget exceeded: requested {0}, allowed {1}")]
    EffectorBudgetExceeded(f64, f64),
    
    #[error("Invalid effector type for operation")]
    InvalidEffectorType,
    
    #[error("Invalid MWC parameters: {0}")]
    InvalidParameters(String),
    
    #[error("State function out of range: {0}")]
    StateFunctionOutOfRange(f64),
}

pub type Result<T> = std::result::Result<T, GmlError>;
```

---

## hkask-gml-cascade

Cascade execution follows a system-dynamics flow of recognize → bind → equilibrate → assess [^forrester1961].

### Trait: `GmlCascade`

```rust
pub trait GmlCascade {
    fn recognize(&self, concept: &ConceptualSystem) -> Result<ConceptAnalysis>;
    fn bind_and_shift(&self, analysis: &ConceptAnalysis, effectors: &[Effector]) -> Result<ShiftedConcept>;
    fn assess_coherence(&self, shift: &ShiftedConcept) -> Result<CoherenceReport>;
}

#[derive(Debug, Clone)]
pub struct ConceptAnalysis {
    pub concept: ConceptualSystem,
    pub distribution: Distribution,
    pub interpretation: String,
}

#[derive(Debug, Clone)]
pub struct ShiftedConcept {
    pub before: ConceptAnalysis,
    pub after: ConceptualSystem,
    pub delta_r_bar: f64,
}

#[derive(Debug, Clone)]
pub struct CoherenceReport {
    pub score: f64,
    pub stability: Stability,
    pub assessment: String,
}
```

---

## hkask-gml-storage

Storage with SQLCipher encryption and capability-based row-level security [^dennis1966].

### Trait: `GmlStorage`

```rust
pub trait GmlStorage {
    fn save_concept(&self, concept: &ConceptualSystem) -> Result<()>;
    fn load_concept(&self, id: &ConceptId) -> Result<ConceptualSystem>;
    fn query_by_cooperativity(&self, min_n_h: f64) -> Result<Vec<ConceptualSystem>>;
    fn save_network(&self, network: &Network) -> Result<()>;
    fn load_network(&self, id: &NetworkId) -> Result<Network>;
    fn delete_concept(&self, id: &ConceptId) -> Result<()>;
}

pub struct SqliteGmlStorage {
    conn: Connection,
    current_user_id: HkaskId,
}

impl SqliteGmlStorage {
    pub fn new(path: &str, current_user_id: HkaskId) -> Result<Self>;
    pub fn with_encryption(
        path: &str,
        current_user_id: HkaskId,
        encryption_key: &[u8],
    ) -> Result<Self>;
}
```

### SQL Schema

```sql
CREATE TABLE gml_concepts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    l REAL NOT NULL,
    current_alpha REAL NOT NULL DEFAULT 0.0,
    current_r_bar REAL NOT NULL DEFAULT 0.0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    owner_id TEXT NOT NULL,
    visibility TEXT NOT NULL CHECK (visibility IN ('private', 'shared', 'public'))
);

CREATE TABLE gml_interpretations (
    id TEXT PRIMARY KEY,
    concept_id TEXT NOT NULL REFERENCES gml_concepts(id),
    state_type TEXT NOT NULL CHECK (state_type IN ('T', 'R')),
    description TEXT NOT NULL,
    energy REAL NOT NULL,
    UNIQUE(concept_id, state_type)
);

CREATE TABLE gml_ports (
    id TEXT PRIMARY KEY,
    concept_id TEXT NOT NULL REFERENCES gml_concepts(id),
    name TEXT NOT NULL,
    effector_shape TEXT NOT NULL,
    affinity_c REAL NOT NULL,
    bound_effector_id TEXT
);

CREATE TABLE gml_effectors (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    concentration REAL NOT NULL,
    effect_type TEXT NOT NULL CHECK (effect_type IN ('activator', 'inhibitor', 'neutral')),
    shape TEXT NOT NULL,
    owner_id TEXT NOT NULL
);

CREATE TABLE gml_networks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    coherence_score REAL,
    stability TEXT,
    owner_id TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE gml_network_edges (
    id TEXT PRIMARY KEY,
    network_id TEXT NOT NULL REFERENCES gml_networks(id),
    from_concept_id TEXT NOT NULL REFERENCES gml_concepts(id),
    to_concept_id TEXT NOT NULL REFERENCES gml_concepts(id),
    edge_type TEXT NOT NULL CHECK (edge_type IN ('cooperative', 'competitive', 'neutral')),
    weight REAL NOT NULL DEFAULT 1.0
);

CREATE TABLE gml_audit_log (
    id TEXT PRIMARY KEY,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    operation TEXT NOT NULL,
    concept_id TEXT,
    capability_hash TEXT NOT NULL,
    effector_id TEXT,
    before_r_bar REAL,
    after_r_bar REAL,
    owner_id TEXT NOT NULL
);
```

---

## hkask-gml-template

### Trait: `GmlTemplateEngine`

```rust
pub trait GmlTemplateEngine {
    fn render_recognize(&self, concept: &ConceptualSystem) -> Result<String>;
    fn render_bind(&self, concept: &ConceptualSystem, effector: &Effector) -> Result<String>;
    fn render_equilibrium(&self, before: &ConceptualSystem, after: &ConceptualSystem) -> Result<String>;
    fn render_coherence(&self, report: &CoherenceReport) -> Result<String>;
    fn render_reframe(&self, shift: &ShiftedConcept) -> Result<String>;
}
```

### Templates

| Template | Purpose | KnowAct |
|----------|---------|---------|
| `gml/recognize-ensemble.j2` | Parse concept into states and ports | recognize, discriminate, parse |
| `gml/bind-effector.j2` | Apply effector, infer state-shift | analogy, infer, bind |
| `gml/compute-equilibrium.j2` | Calculate R̄, n_H, distribution | calculate, compare |
| `gml/assess-coherence.j2` | Evaluate network homeostasis | evaluate, reflect, calibrate |
| `gml/reframe-concept.j2` | Generate alternative frames | abduct, generate, synthesize |

---

## hkask-gml-embedding

Embedding projection maps conceptual systems into a state space derived from allosteric binding parameters [^mwc1965].

### Trait: `ConceptEmbedding`

```rust
pub trait ConceptEmbedding {
    fn compute_similarity(&self, a: &ConceptualSystem, b: &ConceptualSystem) -> f64;
    fn project_to_state_space(&self, concept: &ConceptualSystem) -> StateVector;
}

#[derive(Debug, Clone)]
pub struct StateVector {
    pub r_bar: f64,
    pub n_h: f64,
    pub l: f64,
    pub alpha: f64,
}
```

---

## hkask-mcp-gml

MCP server exposing GML operations as tools following the Model Context Protocol specification [^anthropic2024]. API design follows resource-oriented principles of stateless interaction [^fielding2000].

### MCP Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `gml_recognize` | Analyze concept state ensemble | concept_id |
| `gml_bind` | Apply effector to concept | concept_id, effector_name, concentration, effect_type |
| `gml_equilibrium` | Compute state distribution | concept_id |
| `gml_cooperate` | Compute amplification between concepts | concept_a_id, concept_b_id |
| `gml_homeostasis` | Assess network coherence | network_id |

### Tool Parameters

```rust
#[derive(Debug, Deserialize)]
pub struct RecognizeParams {
    pub concept_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BindParams {
    pub concept_id: String,
    pub effector_name: String,
    pub effector_concentration: f64,
    pub effector_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CooperateParams {
    pub concept_a_id: String,
    pub concept_b_id: String,
}

#[derive(Debug, Deserialize)]
pub struct HomeostasisParams {
    pub network_id: String,
}
```

---

## CNS Integration

CNS spans and algedonic alerts provide observability for GML operations following Beer's viable system model [^beer1972].

### Spans

```rust
pub const CNS_GML_RECOGNIZE: &str = "cns.gml.recognize";
pub const CNS_GML_BIND: &str = "cns.gml.bind";
pub const CNS_GML_EQUILIBRATE: &str = "cns.gml.equilibrate";
pub const CNS_GML_ASSESS: &str = "cns.gml.assess";
pub const CNS_GML_REFRAME: &str = "cns.gml.reframe";
pub const CNS_GML_CAPABILITY_CHECK: &str = "cns.gml.security.capability_check";
pub const CNS_GML_CAPABILITY_DENIED: &str = "cns.gml.security.capability_denied";
pub const CNS_GML_AUDIT_WRITE: &str = "cns.gml.security.audit_write";
pub const CNS_GML_RLS_VIOLATION: &str = "cns.gml.security.rls_violation";
```

### Algedonic Alert

```rust
pub const VARIETY_DEFICIT_THRESHOLD: u64 = 100;
pub const CNS_ALGEDONIC_VARIETY: &str = "cns.algedonic.variety_deficit";
```

---

## See Also

- [Architecture](./gml-architecture.md)

---

[^mwc1965]: Monod, J., Wyman, J., & Changeux, J.-P. (1965). On the nature of allosteric transitions: A plausible model. *Journal of Molecular Biology*, 12(1), 88–118. https://doi.org/10.1016/S0022-2836(65)80285-6

[^hill1910]: Hill, A. V. (1910). The possible effects of the aggregation of the molecules of haemoglobin on its dissociation curves. *Journal of Physiology*, 40(Suppl), iv–vii.

[^forrester1961]: Forrester, J. W. (1961). *Industrial Dynamics*. MIT Press.

[^dennis1966]: Dennis, J. B., & Van Horn, E. C. (1966). Programming semantics for multiprogrammed computations. *Communications of the ACM*, 9(3), 143–155. https://doi.org/10.1145/365230.365252

[^anthropic2024]: Anthropic. (2024). *Model Context Protocol specification*. https://modelcontextprotocol.io/

[^fielding2000]: Fielding, R. T. (2000). *Architectural styles and the design of network-based software architectures* [Doctoral dissertation, University of California, Irvine]. https://www.ics.uci.edu/~fielding/pubs/dissertation/top.htm

[^beer1972]: Beer, S. (1972). *Brain of the Firm: The Managerial Cybernetics of Organization*. Allen Lane.

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
