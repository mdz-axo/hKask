# ARL v0.1.0 Implementation Continuation Prompt

**Purpose:** Complete context for continuing the ARL implementation in `hkask-cns`.  
**Date:** 2026-06-02  
**Status:** Ready for implementation — all architectural decisions resolved

---

## What You're Building

**Allosteric Regulation Logic (ARL)** — the MWC (Monod-Wyman-Changeux) equation as a native regulation primitive inside `hkask-cns`, giving the Cybernetics and Curation loops the ability to autonomously self-regulate and maintain homeostasis.

ARL replaces the deleted `hkask-mcp-gml` MCP server. The regulation kernel now lives in the CNS where it belongs — it is NOT an external tool, it is the core decision-making substrate of the meta loops.

## Architecture Decision Record (All Confirmed)

### Q1: Where does ARL live?
**Option C: CNS-native.** The MWC kernel, `Distribution<T>` type, `AllostericGate`, and RBM all live in `hkask-cns`. Other loops call into CNS when they need regulation. The conceptual-analysis KnowAct (Allosteric Thinking) is separate from the regulation primitive, though it shares the same mathematical kernel.

### Q2: Which insertion points first?
**IP-1 (Algedonic Alert Escalation) + IP-3 (Curation Confidence Gate).**

IP-1 is the safest first insertion — it replaces a binary threshold (`hkask-cns/src/algedonic.rs`) with a smooth MWC sigmoid. Backward-compatible (existing behavior = limit case L→∞). Parameters are directly measurable from system logs.

IP-3 is the most impactful — it gives the Curation Loop genuine metacognition. Without confidence-gated decisions, curation is just observation. With it, curation becomes metacognition. Evidence channels for v0.1.0: `LlmConfidence` (from Okapi), `TemplateMatch` (from registry), `ValidationResult` (from schema). `HumanConfirmation` and `HistoricalPrecedent` are v0.2.

### Q3: Algedonic cooperativity?
**Tunable.** Default to OR-gate (any domain in deficit triggers escalation) for safety — you want to catch single-point failures. AND-gate is available as configuration. The cooperativity dimensionality `n` is the number of domains tracked by the variety tracker.

### Q4: Curation evidence channels (v0.1.0)?
1. `LlmConfidence { c: f64 }` — LLM's self-assessed confidence (from Okapi inference results)
2. `TemplateMatch { c: f64 }` — Template relevance score (from registry)
3. `ValidationResult { c: f64 }` — Schema/validation pass result

### Q5: "Ask what would increase confidence" routing?
As a `CuratorDirective` to Cybernetics, routed through the Communication Loop to the Inference Loop. This respects the authority hierarchy (Curation → Cybernetics → domain loops).

### Q6: Communication exclusion?
**Confirmed — excluded.** IP-9 (message priority routing) is NOT implemented. Communication remains a dumb pipe. The issuing loop sets priority, not the transport.

### Q7: Framework name?
**Allosteric Regulation Logic (ARL).** Confirmed. The deleted MCP server was `hkask-mcp-gml`. No new MCP server replaces it — ARL is built into `hkask-cns`.

### Q8: Probability monad?
**Implement `Distribution<T>` + `bind` in v0.1.0.** Every ARL gate produces a `Distribution<Decision>`, not a scalar `f64`. Gate chains propagate uncertainty. The `act` phase collapses distributions to concrete decisions. **Do NOT claim monadic structure** in documentation or comments until the monad laws are formally verified (v0.2.0). Use the composition pattern without the categorical claim. See `docs/gml/gml-probability-monad-exploration.md` for the full technical rationale.

### Q9: RBM?
**Build it now in v0.1.0.** The Restricted Boltzmann Machine is the statistical mechanics kernel of ARL. It provides the learning algorithm (contrastive divergence) that calibrates coupling weights and biases from observed system behavior. The MWC equation constrains the RBM structure (two-state, concerted transitions); the RBM provides inference and learning.

## Current State of the Codebase

### Deleted
- `mcp-servers/hkask-mcp-gml/` — entirely removed
- All references to `hkask-mcp-gml` cleaned from workspace Cargo.toml, CLI bootstrap, docs, architecture files
- MCP server count: 14 (was 15/16)

### Existing CNS code (where ARL gets inserted)
- `crates/hkask-cns/src/lib.rs` — module declarations and re-exports
- `crates/hkask-cns/src/algedonic.rs` — algedonic alerts (IP-1 target)
- `crates/hkask-cns/src/cybernetics_loop.rs` — Cybernetics Loop with SetPoints (IP-2 target)
- `crates/hkask-cns/src/dampener.rs` — directive dampening (IP-5 target)
- `crates/hkask-cns/src/energy.rs` — energy budget enforcement (IP-4 target)
- `crates/hkask-cns/src/variety.rs` — variety tracking
- `crates/hkask-cns/src/circuit_breaker.rs` — circuit breaking
- `crates/hkask-cns/src/runtime.rs` — CNS runtime

### Existing types code
- `crates/hkask-types/src/loops/mod.rs` — LoopId, Signal, Deviation, LoopAction, ActionType, HkaskLoop trait
- `crates/hkask-types/src/loops/curation.rs` — CuratorDirective, CuratorHandle
- `crates/hkask-types/src/loops/cybernetics.rs` — CyberneticsHandle
- `crates/hkask-types/src/cns.rs` — CnsHealth, CircuitState
- `crates/hkask-types/src/id.rs` — WebID and other ID types

### Documentation (in docs/gml/)
- `gml-mwc-formal-structure.md` — Task 0: RDF triples, kernel ERD, equations
- `gml-kernel-erd.md` — Task 1: 7-entity kernel ERD, allosteric gate table
- `gml-mwc-insertion-audit.md` — 10 insertion points with decision matrix
- `gml-review-response.md` — Full response to internal technical review
- `gml-probability-monad-exploration.md` — Q8 deep dive on probability monad
- `gml-research-paper.md` — original paper (needs ARL revision)
- `gml-user-guide.md` — original user guide (needs ARL revision)
- `gml-allosteric-thinking-v2.md` — original task spec (needs ARL revision)

## Technical Specifications — What to Build

### 1. `crates/hkask-cns/src/allosteric/mod.rs` — New ARL module

Register in `lib.rs`:
```rust
pub mod allosteric;
```

### 2. `Distribution<T>` — Probability distribution type

```rust
/// A probability distribution over values of type T.
/// Every ARL gate produces a Distribution<Decision> instead of a scalar f64.
#[derive(Debug, Clone)]
pub enum Distribution<T> {
    /// Deterministic: single outcome, probability 1. (This is monadic return.)
    Deterministic(T),
    /// Bernoulli: two outcomes, R̄ probability of r_outcome. (This is what MWC produces.)
    Bernoulli { r_outcome: T, t_outcome: T, r_bar: f64 },
    /// Mixture: weighted combination. (This is what bind produces when chaining.)
    Mixture(Vec<(f64, Distribution<T>)>),
}
```

Implement:
- `Distribution::return_(value)` — wrap a certain value
- `Distribution::bind(self, f)` — compose this distribution with a transition kernel
- `Distribution::expected_r_bar(&self)` — collapse to point estimate (for `act` phase)
- `Distribution::sensitivity(&self, channel)` — compute which input contributes most to output uncertainty

**Do NOT call this a "monad" in code comments or docs.** Call it "distribution composition" or "uncertainty propagation." The monad laws are not yet verified.

### 3. `AllostericGate` — MWC-regulated decision point

```rust
/// An allosteric gate — an MWC-regulated decision point in the 6-loop system.
///
/// Parameters are MEASURABLE OPERATIONAL QUANTITIES (not analyst encodings):
/// - L: ratio of T/R decisions in neutral conditions (countable from logs)
/// - c: sensitivity ratio under R vs T state (observable from response curve)
/// - n: number of evidence channels (determined by architecture)
/// - α: normalized deficit/deviation (read from Signal values)
/// - τ: relaxation time (how fast the gate settles)
/// - hysteresis: L adjustment from previous R̄
pub struct AllostericGate {
    pub name: String,
    pub base_l: f64,
    pub c: f64,
    pub n: usize,
    pub alpha: f64,
    pub threshold: f64,
    pub tau: std::time::Duration,
    pub hysteresis: f64,
    pub prev_r_bar: f64,
}

impl AllostericGate {
    pub fn effective_l(&self) -> f64 { /* hysteresis from previous state */ }
    pub fn r_bar_eq(&self) -> f64 { /* MWC at equilibrium */ }
    pub fn r_bar_at(&self, dt: Duration) -> f64 { /* relaxation toward equilibrium */ }
    pub fn decide(&self) -> Distribution<Decision> { /* Bernoulli from MWC */ }
}
```

### 4. MWC computation engine

Move the verified MWC math from the deleted `hkask-mcp-gml/src/engine.rs` into `hkask-cns/src/allosteric/mwc.rs`:

```rust
/// MWC state function: R̄ = (1+α)ⁿ / ((1+α)ⁿ + L·(1+cα)ⁿ)
pub fn mwc_state_function(l: f64, c: f64, n: u32, alpha: f64) -> Result<f64, AllostericError>

/// Hill coefficient (cooperativity measure)
pub fn hill_coefficient(l: f64, c: f64, n: u32, alpha: f64) -> f64

/// Free energy difference: ΔG = -RT·ln(R̄/(1-R̄))
pub fn delta_g(r_bar: f64, temperature: f64) -> f64

/// Sensitivity: ∂R̄/∂α at current α (for sensitivity analysis)
pub fn mwc_sensitivity(l: f64, c: f64, n: u32, alpha: f64) -> f64
```

### 5. Coupling coefficients (replaces `cooperate`)

Per the internal review: multiplying Hill coefficients was semantically undefined. Replace with coupling coefficients in a directed network graph:

```rust
/// Coupling between two gates in the allosteric network.
/// w_AB: how much gate A's state feeds into gate B's α.
pub struct Coupling {
    pub source: String,
    pub target: String,
    pub weight: f64,
}

/// Cooperativity as coupling: cooperate(A, B) = w_AB · (∂R̄_B/∂α_B) · influence
pub fn cooperate(a: &AllostericGate, b: &AllostericGate, coupling: &Coupling) -> f64
```

### 6. RBM — Restricted Boltzmann Machine

The RBM is the statistical mechanics kernel. It provides:
- **Inference:** P(hidden | visible) = P(T/R | evidence)
- **Learning:** Contrastive divergence to learn coupling weights from observed system decisions
- **MWC constraint:** Hidden units are binary (T=0, R=1), transitions are concerted

```rust
/// Restricted Boltzmann Machine with MWC constraints.
///
/// Visible units = evidence channels (effectors, signals, metrics)
/// Hidden units = decision states (T/R binary)
/// Energy: E(v,h) = -Σᵢ aᵢvᵢ - Σⱼ bⱼhⱼ - Σᵢⱼ vᵢWᵢⱼhⱼ
///
/// Mapping:
///   visible biases a = effector baseline strengths
///   hidden biases b = interpretive biases (related to L)
///   weights W = coupling between evidence and decisions (related to c)
pub struct AllostericRbm {
    pub visible_bias: Vec<f64>,     // a parameters
    pub hidden_bias: Vec<f64>,      // b parameters (b[0]=T, b[1]=R)
    pub weights: Vec<Vec<f64>>,      // W matrix (visible × hidden)
    pub visible_count: usize,
    pub hidden_count: usize,        // always 2 for MWC (T, R)
}

impl AllostericRbm {
    /// Compute P(hidden | visible) — the MWC inference
    pub fn infer(&self, visible: &[f64]) -> Distribution<Decision>;

    /// Contrastive divergence learning step
    /// Updates weights based on observed (visible, hidden) pairs
    pub fn learn_step(&mut self, visible: &[f64], learning_rate: f64);

    /// Extract MWC parameters from learned RBM weights
    pub fn to_mwc_parameters(&self) -> MwcParameters;

    /// Construct RBM from MWC parameters
    pub fn from_mwc_parameters(params: &MwcParameters) -> Self;
}
```

### 7. Homeostasis with rebalance

Per the internal review: `homeostasis` must include a feedback mechanism, not just monitoring.

```rust
/// Homeostasis assessment with rebalance capability.
///
/// Descriptive coherence: are gates in mutually consistent states?
/// Normative coherence: are gates in states aligned with agent goals?
pub struct HomeostasisAssessor {
    pub targets: std::collections::HashMap<String, f64>,  // target R̄ per gate
    pub coherence_threshold: f64,
}

impl HomeostasisAssessor {
    pub fn descriptive_coherence(&self, network: &AllostericNetwork) -> f64;
    pub fn normative_coherence(&self, network: &AllostericNetwork) -> f64;
    pub fn rebalance(&self, network: &AllostericNetwork) -> Vec<EffectorAdjustment>;
}
```

### 8. Curation confidence gate (IP-3)

```rust
/// Curation confidence gate — metacognitive decision point.
///
/// R̄ = confidence that the Curator should proceed with a decision.
/// If R̄ is in the transition zone (0.3 < R̄ < 0.8), the regulated
/// response is to ask what would increase confidence.
pub struct CurationConfidenceGate {
    pub l: f64,                // Default skepticism
    pub ports: Vec<CurationPort>,
    pub threshold: f64,
    pub tau: std::time::Duration,
    pub hysteresis: f64,
    pub prev_r_bar: f64,
}

pub enum CurationPort {
    LlmConfidence { c: f64 },
    TemplateMatch { c: f64 },
    ValidationResult { c: f64 },
}
```

### 9. Wire into the Cybernetics Loop

The `CyberneticsLoop::compute()` method currently uses binary deviation detection. Wire the algedonic gate (IP-1) in to replace the hard threshold in `algedonic.rs`:

**Current** (`algedonic.rs` line 53):
```rust
let severity = if deficit > threshold {
    AlertSeverity::Critical
} else if deficit > threshold / 2 {
    AlertSeverity::Warning
} else {
    AlertSeverity::Info
};
```

**Replace with** AllostericGate that computes `Distribution<AlertSeverity>` and collapses in the `act` phase.

## Critical Constraints (From AGENTS.md and PRINCIPLES.md)

1. **Headless** — No visual UI. CLI/MCP/API only.
2. **Idiomatic Rust** — Capability tokens are types, not strings. Loop membership is encoded in the module tree. Deletion is the only deprecation.
3. **6-loop model** — Authority flows downward: Curation → Cybernetics → {Inference, Episodic, Semantic, Communication}. No sideways edges.
4. **OCAP** — No ambient authority. Least privilege. Capability attenuation. End-to-end enforcement.
5. **Communication is a dumb pipe** — Does NOT dampen, throttle, or circuit-break.
6. **Scope separation** — ARL has two use cases: (a) process flow regulation (measurable quantities, primary), (b) conceptual analysis (qualitative heuristic only, no quantitative claims).
7. **Category error does not apply** — In the regulation use case, MWC parameters map to measurable operational quantities (decision frequencies, Signal values, energy budgets). This is explicitly documented.
8. **Do NOT claim monadic structure** — Use "distribution composition" / "uncertainty propagation" language. Monad law verification is v0.2.0.
9. **No `#[deprecated]`** — If something needs to go, delete it.
10. **No `todo!` or `unimplemented!`** — Only ship complete code.

## Review Critique Status

The internal technical review identified 9 issues. All addressed:

| # | Issue | Status | Resolution |
|---|-------|--------|------------|
| 1 | "Monad" claim | ✅ Resolved | Renamed to ARL; no monadic claim until v0.2.0 |
| 2 | Category error | ✅ Resolved | Scope separation: regulation ≠ conceptual analysis |
| 3 | Parameter estimation | ✅ Resolved | Regulation parameters are measurable; calibration protocol specified |
| 4 | cooperate semantics | ✅ Resolved | Replaced with coupling coefficients in network graph |
| 5 | OCAP not specified | ✅ Resolved | Reference existing hKask capability lattice |
| 6 | Temporal dynamics | ✅ Resolved | τ + hysteresis + turn granularity in every gate |
| 7 | homeostasis feedback | ✅ Resolved | Added rebalance; descriptive/normative coherence distinction |
| 8 | Worked examples robustness | ⏳ In progress | Need sensitivity tables for existing docs |
| 9 | Boltzmann connection | ✅ Resolved | Formalized as RBM with MWC constraints |

## Build Order

1. **`crates/hkask-cns/src/allosteric/mod.rs`** — Module declaration
2. **`crates/hkask-cns/src/allosteric/distribution.rs`** — `Distribution<T>` type with `bind`, `expected_r_bar`, `sensitivity`
3. **`crates/hkask-cns/src/allosteric/mwc.rs`** — MWC computation engine (port from deleted gml server)
4. **`crates/hkask-cns/src/allosteric/gate.rs`** — `AllostericGate` with temporal dynamics
5. **`crates/hkask-cns/src/allosteric/coupling.rs`** — Coupling coefficients (replaces `cooperate`)
6. **`crates/hkask-cns/src/allosteric/rbm.rs`** — `AllostericRbm` with inference + contrastive divergence
7. **`crates/hkask-cns/src/allosteric/homeostasis.rs`** — HomeostasisAssessor with rebalance
8. **`crates/hkask-cns/src/allosteric/curation.rs`** — CurationConfidenceGate (IP-3)
9. **Wire IP-1** — Update `algedonic.rs` to use AllostericGate
10. **Wire IP-3** — Update curation types in `hkask-types` for confidence-gated decisions
11. **Update `lib.rs`** — Register allosteric module and re-exports
12. **Tests** — Cybernetic unit tests for each component
13. **`cargo check --workspace`** — Verify clean build
14. **`cargo clippy -p hkask-cns -- -D warnings`** — Verify lint-clean
15. **`cargo test -p hkask-cns`** — Verify all tests pass

## Key Files to Read First

- `crates/hkask-cns/src/lib.rs` — current module structure
- `crates/hkask-cns/src/algedonic.rs` — IP-1 target (algedonic alerts)
- `crates/hkask-cns/src/cybernetics_loop.rs` — Cybernetics Loop with SetPoints
- `crates/hkask-cns/src/dampener.rs` — existing dampening mechanism
- `crates/hkask-cns/src/energy.rs` — energy budget enforcement
- `crates/hkask-types/src/loops/mod.rs` — LoopId, Signal, Deviation, LoopAction
- `crates/hkask-types/src/loops/curation.rs` — CuratorDirective
- `docs/gml/gml-probability-monad-exploration.md` — full Distribution<T> rationale
- `docs/gml/gml-mwc-insertion-audit.md` — all 10 insertion points with analysis
- `docs/gml/gml-review-response.md` — how each review critique was addressed
- `docs/gml/gml-mwc-formal-structure.md` — MWC equations reference
- `docs/gml/gml-kernel-erd.md` — 7-entity kernel ERD

## MWC Equations Reference

```
R̄ = (1 + α)ⁿ / ((1 + α)ⁿ + L·(1 + cα)ⁿ)

L = exp(-(E_T - E_R) / kT)    [allosteric constant]
c = K_R / K_T                  [affinity ratio]
n = number of binding sites    [cooperativity dimensionality]
α = [X] / K_R                  [normalized ligand concentration]

n_H = d·ln(R̄/(1-R̄)) / d·ln(α)  [Hill coefficient at EC50]

Z = (1 + α)ⁿ + L·(1 + cα)ⁿ    [partition function]
P(R) = exp(-E_R/kT) / Z       [Boltzmann interpretation]
```

## RBM Mapping Reference

```
E(v, h) = -Σᵢ aᵢvᵢ - Σⱼ bⱼhⱼ - Σᵢⱼ vᵢWᵢⱼhⱼ

visible units v  = evidence channels (effectors, signals, metrics)
hidden units h   = decision states (T=0, R=1)
visible bias a  = effector baseline strengths
hidden bias b   = interpretive biases (related to L)
weight matrix W = coupling evidence→decisions (related to c)

MWC constraint: hidden_count = 2, transitions are concerted
```

---

*ℏKask — Planck's Constant of Agent Systems — ARL v0.1.0*