---
title: "GML Architecture (Aspirational)"
audience: [architects, developers]
last_updated: 2026-05-24
version: "0.1.0"
status: "Draft"
domain: "Application"
ddmvss_categories: [domain]
---

# GML Architecture

**Status:** Draft — This document describes a planned multi-crate GML decomposition. The current implementation lives entirely in `hkask-mcp-gml` (1,022 LOC). The crate structure below is aspirational for v1.1+.

---

## Contents

| Section | Description |
|---------|-------------|
| [Overview](#overview) | GML as a KnowAct applying MWC allosteric model |
| [Crate Structure](#crate-structure) | Planned multi-crate decomposition |
| [Domain Types](#domain-types) | Core type definitions and relationships |
| [GML Algebra](#gml-algebra) | Mathematical operations on concept distributions |
| [Cascade Structure](#cascade-structure) | Multi-level cascade selection mechanism |
| [Hexagonal Architecture](#hexagonal-architecture) | Ports and adapters for GML |
| [CNS Integration](#cns-integration) | GML-specific CNS spans and metrics |
| [Security Model (OCAP)](#security-model-ocap) | Capability-gated GML operations |
| [Boltzmann Machine Integration](#boltzmann-machine-integration) | Boltzmann sampling for concept recombination |
| [See Also](#see-also) | Related documentation |

---

## Overview

Generalized Monad Logic (GML) is a KnowAct that applies the Monod-Wyman-Changeux (MWC) allosteric model [^mwc1965] to abstract concept recombination and regulation [^beer1972].

---

## Crate Structure

```
hkask-workspace/
├── hkask-gml-types/          # Domain types (ID types, structures)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── ids.rs            # ConceptId, PortId, EffectorId, NetworkId
│       ├── concept.rs        # ConceptualSystem, Interpretation, Port
│       ├── effector.rs       # Effector, EffectType, EffectorShape
│       ├── distribution.rs   # Distribution, Coherence, Stability
│       ├── network.rs        # Network, ConceptEdge, EdgeType
│       ├── parameters.rs     # MwcParameters
│       └── capability.rs     # GmlCapability, CapabilityScope, GmlOperation
│
├── hkask-gml-core/           # GML algebra implementation
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── algebra.rs        # GmlAlgebra trait + implementation
│       ├── mwc.rs            # MWC state function, Hill coefficient, Z
│       ├── boltzmann.rs      # Boltzmann distribution, energy computations
│       └── error.rs          # GmlError types
│
├── hkask-gml-cascade/        # Cascade execution (KnowAct/FlowDef)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── recognize.rs      # recognize-ensemble.j2 execution
│       ├── bind.rs           # bind-effector.j2 execution
│       ├── equilibrate.rs    # compute-equilibrium.j2 execution
│       ├── assess.rs         # assess-coherence.j2 execution
│       └── reframe.rs        # reframe-concept.j2 execution
│
├── hkask-gml-storage/        # Storage adapter (SQLite + SQLCipher)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── adapter.rs        # GmlStorage trait implementation
│       ├── schema.rs         # SQL schema definitions
│       └── rls.rs            # Row-level security (OCAP)
│
├── hkask-gml-template/       # Template adapter (Jinja2 via hkask-templates)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── adapter.rs        # GmlTemplateEngine trait implementation
│       └── namespace.rs      # gml/ namespace registration
│
├── hkask-gml-embedding/      # Embedding adapter (Okapi via hkask-mcp-inference)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── adapter.rs        # ConceptEmbedding trait implementation
│       └── similarity.rs     # State vector projection, similarity
│
├── hkask-mcp-gml/            # MCP server exposing GML operations
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── server.rs         # MCP server definition
│       ├── tools.rs          # MCP tool definitions
│       └── handlers.rs       # Tool request handlers
│
└── hkask-testing/            # Test crate (excluded from line budget)
    └── gml/
        ├── unit/             # Unit tests
        ├── integration/      # Cross-crate integration tests
        └── fixtures/         # Test fixtures, mocks
```

---

## Domain Types

Core domain types derive from the MWC allosteric formalism [^mwc1965] and cooperative binding theory [^hill1910].

### MwcParameters

```rust
pub struct MwcParameters {
    pub l: f64,        // allosteric constant [T]_0/[R]_0
    pub c: f64,        // affinity ratio K_R/K_T (selectivity)
    pub n: usize,      // number of binding sites
    pub alpha: f64,    // normalized ligand concentration
}
```

### ConceptualSystem

```rust
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

### Effector

```rust
pub struct Effector {
    pub id: EffectorId,
    pub name: String,
    pub concentration: f64,
    pub effect_type: EffectType,
    pub shape: EffectorShape,
}

pub enum EffectType {
    Activator,   // stabilizes R-state (c < 1)
    Inhibitor,   // stabilizes T-state (c > 1)
    Neutral,     // shifts without preference (c = 1)
}
```

### Distribution

```rust
pub struct Distribution {
    pub p_r: f64,    // probability of R-state
    pub p_t: f64,    // probability of T-state
    pub n_h: f64,    // Hill coefficient
}
```

### Network

```rust
pub struct Network {
    pub id: NetworkId,
    pub concepts: Vec<ConceptualSystem>,
    pub edges: Vec<ConceptEdge>,
}

pub struct ConceptEdge {
    pub from: ConceptId,
    pub to: ConceptId,
    pub edge_type: EdgeType,
    pub weight: f64,
}

pub enum EdgeType {
    Cooperative,
    Competitive,
    Neutral,
}
```

### Coherence

```rust
pub struct Coherence {
    pub score: f64,
    pub stability: Stability,
}

pub enum Stability {
    Stable,
    Transitioning,
    Unstable,
}
```

---

## GML Algebra

The algebra implements the MWC state function, Hill cooperativity coefficient, and Boltzmann energy partition [^mwc1965][^hill1910].

### Trait Definition

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
```

### Mathematical Implementations

```rust
// MWC state function: R̄ = (1 + α)ⁿ / ((1 + α)ⁿ + L·(1 + cα)ⁿ)
pub fn mwc_state_function(l: f64, c: f64, n: usize, alpha: f64) -> f64 {
    let one_plus_alpha = 1.0 + alpha;
    let one_plus_c_alpha = 1.0 + c * alpha;
    let numerator = one_plus_alpha.powi(n as i32);
    let denominator = numerator + l * one_plus_c_alpha.powi(n as i32);
    numerator / denominator
}

// Hill coefficient: n_H = n · (1-c)/(1+c) · √(α/(1+α))
pub fn hill_coefficient(n: usize, c: f64, alpha: f64) -> f64 {
    let cooperativity_factor = (1.0 - c) / (1.0 + c);
    let concentration_factor = (alpha / (1.0 + alpha)).sqrt();
    n as f64 * cooperativity_factor * concentration_factor
}

// Partition function: Z = (1 + α)ⁿ + L·(1 + cα)ⁿ
pub fn partition_function(l: f64, c: f64, n: usize, alpha: f64) -> f64 {
    let one_plus_alpha = 1.0 + alpha;
    let one_plus_c_alpha = 1.0 + c * alpha;
    one_plus_alpha.powi(n as i32) + l * one_plus_c_alpha.powi(n as i32)
}

// Boltzmann factor: L = exp(-(E_T - E_R)/kT)
pub fn boltzmann_factor(e_t: f64, e_r: f64, kt: f64) -> f64 {
    ((e_t - e_r) / kt).exp()
}
```

---

## Cascade Structure

Cascade execution follows a system-dynamics-inspired flow of pre/core/post stages [^forrester1961].

### YAML Configuration

```yaml
name: "gml-allosteric-reasoning"
type: KnowAct
lexicon_terms: [recognize, analogy, infer, sequence, probe, assert, bind, shift, equilibrate]

cascade:
  pre:
    - template: gml/recognize-ensemble.j2
      type: Cognition
      knowact: [recognize, discriminate, parse]
      
  core:
    - template: gml/bind-effector.j2
      type: Cognition
      knowact: [analogy, infer, abduct]
      
    - template: gml/compute-equilibrium.j2
      type: Cognition
      knowact: [calculate, compare, evaluate]
      
  post:
    - template: gml/assess-coherence.j2
      type: Cognition
      knowact: [evaluate, reflect, calibrate]
```

### FlowDef

```yaml
process:
  name: "allosteric-reframing"
  type: FlowDef
  
  inputs:
    - name: target_concept
      type: Concept
    - name: contextual_effectors
      type: List[Effector]
    - name: mwc_parameters
      type: MwcParameters
  
  steps:
    - id: recognize
      action: template.render
      template: gml/recognize-ensemble.j2
      
    - id: bind
      action: template.render
      template: gml/bind-effector.j2
      
    - id: equilibrate
      action: template.render
      template: gml/compute-equilibrium.j2
      
    - id: assess
      action: template.render
      template: gml/assess-coherence.j2
      
  outputs:
    - name: coherence_analysis
      type: CoherenceReport
    - name: shifted_concept
      type: Concept
```

---

## Hexagonal Architecture

GML follows the ports-and-adapters (hexagonal) pattern to isolate domain logic from infrastructure [^cockburn2005].

### Inbound Ports (Domain Interface)

```rust
pub trait GmlAlgebra { ... }  // Core algebra
pub trait GmlCascade { ... }  // Cascade execution
```

### Outbound Ports (Infrastructure)

```rust
pub trait GmlStorage {
    fn save_concept(&self, concept: &ConceptualSystem) -> Result<()>;
    fn load_concept(&self, id: &ConceptId) -> Result<ConceptualSystem>;
    fn query_by_cooperativity(&self, min_n_h: f64) -> Result<Vec<ConceptualSystem>>;
    fn save_network(&self, network: &Network) -> Result<()>;
}

pub trait GmlTemplateEngine {
    fn render_recognize(&self, concept: &ConceptualSystem) -> Result<String>;
    fn render_bind(&self, concept: &ConceptualSystem, effector: &Effector) -> Result<String>;
    fn render_equilibrium(&self, before: &ConceptualSystem, after: &ConceptualSystem) -> Result<String>;
    fn render_coherence(&self, report: &CoherenceReport) -> Result<String>;
}

pub trait ConceptEmbedding {
    fn compute_similarity(&self, a: &ConceptualSystem, b: &ConceptualSystem) -> f64;
    fn project_to_state_space(&self, concept: &ConceptualSystem) -> StateVector;
}
```

### Adapters

| Adapter | Implementation |
|---------|----------------|
| Storage | SQLite + SQLCipher with row-level security |
| Template | Jinja2 via hkask-templates with gml/ namespace |
| Embedding | Okapi-backed via hkask-mcp-inference |
| MCP Server | hkask-mcp-gml exposing GML operations |

---

## CNS Integration

The Cybernetic Nervous System (CNS) provides observability and algedonic alerting inspired by Beer's viable system model [^beer1972].

### Spans

```rust
pub const CNS_GML_RECOGNIZE: &str = "cns.gml.recognize";
pub const CNS_GML_BIND: &str = "cns.gml.bind";
pub const CNS_GML_EQUILIBRATE: &str = "cns.gml.equilibrate";
pub const CNS_GML_ASSESS: &str = "cns.gml.assess";
pub const CNS_GML_REFRAME: &str = "cns.gml.reframe";
```

### Algedonic Alert

```rust
pub const VARIETY_DEFICIT_THRESHOLD: u64 = 100;
pub const CNS_ALGEDONIC_VARIETY: &str = "cns.algedonic.variety_deficit";
```

---

## Security Model (OCAP)

Access control follows the object-capability model, where capabilities are unforgeable tokens that confer authority [^dennis1966].

### Capability Types

```rust
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

pub enum CapabilityScope {
    Private,
    SharedRead,
    SharedWrite,
    Public,
}

pub enum GmlOperation {
    Recognize,
    Bind,
    Equilibrate,
    Cooperate,
    Inhibit,
    Activate,
    Homeostasis,
}
```

### Audit Logging

```rust
pub struct GmlAuditLog {
    pub id: AuditLogId,
    pub timestamp: i64,
    pub operation: GmlOperation,
    pub concept_id: ConceptId,
    pub capability_hash: Vec<u8>,
    pub effector_id: Option<EffectorId>,
    pub before_r_bar: Option<f64>,
    pub after_r_bar: Option<f64>,
    pub actor: HkaskId,
    pub result: OperationResult,
}
```

---

## Boltzmann Machine Integration

GML extends the MWC framework with energy-based computation drawn from Hopfield networks [^hopfield1982].

### Energy-Based Concept

```rust
pub struct EnergyBasedConcept {
    pub id: ConceptId,
    pub name: String,
    pub e_t: f64,
    pub e_r: f64,
    pub ligand_binding_energies: Vec<f64>,
    pub interaction_weights: Vec<f64>,
}

impl EnergyBasedConcept {
    pub fn probability_active(&self, temperature: f64) -> f64 {
        let boltzmann_t = (-self.e_t / temperature).exp();
        let boltzmann_r = (-self.e_r / temperature).exp();
        boltzmann_r / (boltzmann_r + boltzmann_t)
    }
    
    pub fn allosteric_constant(&self, temperature: f64) -> f64 {
        ((self.e_t - self.e_r) / temperature).exp()
    }
}
```

### Hybrid Model

```rust
pub struct HybridConcept {
    pub mwc_params: MwcParameters,
    pub e_t: f64,
    pub e_r: f64,
    pub temperature: f64,
    pub external_field: f64,
}

impl HybridConcept {
    pub fn verify_consistency(&self) -> Result<bool> {
        let l_from_mwc = self.mwc_params.l;
        let l_from_boltzmann = boltzmann_factor(self.e_t, self.e_r, self.temperature);
        let tolerance = 0.1 * l_from_mwc.max(l_from_boltzmann);
        Ok((l_from_mwc - l_from_boltzmann).abs() < tolerance)
    }
}
```

---

## See Also

- [API Reference](./gml-api.md)

---

[^mwc1965]: Monod, J., Wyman, J., & Changeux, J.-P. (1965). On the nature of allosteric transitions: A plausible model. *Journal of Molecular Biology*, 12(1), 88–118. https://doi.org/10.1016/S0022-2836(65)80285-6

[^hill1910]: Hill, A. V. (1910). The possible effects of the aggregation of the molecules of haemoglobin on its dissociation curves. *Journal of Physiology*, 40(Suppl), iv–vii.

[^beer1972]: Beer, S. (1972). *Brain of the Firm: The Managerial Cybernetics of Organization*. Allen Lane.

[^forrester1961]: Forrester, J. W. (1961). *Industrial Dynamics*. MIT Press.

[^cockburn2005]: Cockburn, A. (2005). Hexagonal architecture (Ports and Adapters). https://alistair.cockburn.us/hexagonal-architecture/

[^dennis1966]: Dennis, J. B., & Van Horn, E. C. (1966). Programming semantics for multiprogrammed computations. *Communications of the ACM*, 9(3), 143–155. https://doi.org/10.1145/365230.365252

[^hopfield1982]: Hopfield, J. J. (1982). Neural networks and physical systems with emergent collective computational abilities. *Proceedings of the National Academy of Sciences*, 79(8), 2554–2558. https://doi.org/10.1073/pnas.79.8.2554

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
