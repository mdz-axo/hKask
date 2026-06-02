# ARL System Audit: MWC Allosteric Gate Insertion Points in hKask

**Version:** 0.1.0  
**Status:** Decision Document — Requires Human Input  
**Date:** 2026-06-02

---

## Purpose

This document audits the hKask codebase for decision points where the MWC equation could serve as a **universal process flow regulator** (monad logic gate). Each insertion point is analyzed against the 6-loop model, the existing code, and the authority hierarchy.

**The core hypothesis under evaluation:** The MWC equation is not just a conceptual analysis tool — it is the fundamental **regulation primitive** that gives the Cybernetics and Curation loops the ability to natively and autonomously self-regulate and maintain homeostasis.

**Critical scope distinction (per internal review):** The MWC equation serves two distinct use cases with different validity requirements:
1. **Conceptual analysis** (KnowAct) — qualitative heuristic framework; MWC variables are analyst encodings, NOT measurable quantities. Quantitative claims are limited to illustrative values with required robustness analysis.
2. **Process flow regulation** (Allosteric Gate) — MWC variables map to measurable operational quantities (decision frequencies, signal values, energy budgets). This is the primary use case for insertion into the 6-loop architecture.

The category error identified in the review (MWC variables undefined for concepts) applies to use case 1 but NOT to use case 2. In regulation, L = ratio of T/R decisions in neutral conditions (countable), α = normalized deficit (from Signal), c = sensitivity ratio (observable), n = number of evidence channels (architectural).

**Decision required:** Which insertion points should be implemented, in what order, and with what parameter semantics?

---

## The Allosteric Gate Pattern

Before auditing specific insertion points, here is the generalized pattern:

```rust
/// An allosteric gate — an MWC-regulated decision point.
///
/// Every gate has:
/// - L (skepticism): default bias toward NOT proceeding
///   [regulation]: count(T-decisions)/count(R-decisions) in neutral conditions
/// - c (signal quality): how much evidence shifts confidence
///   [regulation]: ratio of gate sensitivity under R vs T state
/// - n (ports): number of independent input channels
///   [regulation]: number of evidence channels (architectural)
/// - α (evidence): current contextual pressure
///   [regulation]: normalized deficit/deviation from Signal values
/// - τ (relaxation): settling time for the gate
///   [regulation]: how fast the gate reaches equilibrium after input change
///
/// The gate computes:
///   R̄ = (1+α)ⁿ / ((1+α)ⁿ + L_eff·(1+cα)ⁿ)
///   where L_eff includes hysteresis from previous state
///
/// Where R̄ is the confidence that the system should proceed.
/// - R̄ > threshold → proceed (R-state: express, activate, escalate)
/// - R̄ < threshold → don't proceed (T-state: suppress, inhibit, wait)
///
/// The regulated response at low confidence:
/// - What evidence would increase R̄?
/// - What uncertainty could be reduced?
/// - What other gates need to activate first?
struct AllostericGate {
    name: String,
    base_l: f64,        // skepticism (L in neutral conditions)
    c: f64,             // signal quality (affinity ratio)
    n: usize,           // number of input ports
    alpha: f64,         // current evidence (normalized)
    threshold: f64,    // confidence threshold for proceeding
    tau: Duration,      // relaxation time
    hysteresis: f64,   // L adjustment from previous R̄ (0.0-1.0)
    prev_r_bar: f64,   // previous tick's confidence
}

impl AllostericGate {
    /// Effective L includes hysteresis: recent R-state activity
    /// lowers effective L, making it easier to return to R-state.
    fn effective_l(&self) -> f64 {
        self.base_l * (1.0 - self.hysteresis * self.prev_r_bar)
    }

    /// R̄ at equilibrium (current tick)
    fn r_bar_eq(&self) -> f64 {
        mwc_state_function(self.effective_l(), self.c, self.n, self.alpha)
    }

    /// R̄ at time dt after input change (relaxation toward equilibrium)
    fn r_bar_at(&self, dt: Duration) -> f64 {
        let eq = self.r_bar_eq();
        let t = dt.as_secs_f64();
        let tau = self.tau.as_secs_f64();
        eq + (self.prev_r_bar - eq) * (-t / tau).exp()
    }
}
```

---

## Insertion Point Audit

### IP-1: Algedonic Alert Escalation (CNS → Curation)

**Current code:** `hkask-cns/src/algedonic.rs`
**Current behavior:** Hard threshold comparison — `if deficit > threshold → escalate`
**6-Loop:** Cybernetics (sense) → Curation (escalate)

**Current implementation:**
```rust
// Current: binary escalation
let severity = if deficit > threshold {
    AlertSeverity::Critical  // Hard switch
} else if deficit > threshold / 2 {
    AlertSeverity::Warning
} else {
    AlertSeverity::Info
};
```

**MWC gate proposal:**
```rust
// Proposed: MWC-regulated escalation
// L = high (conservative: don't escalate by default)
// α = deficit (contextual pressure from variety tracker)
// c = selectivity (how much deficit shifts escalation confidence)
// n = number of domains contributing to deficit
// R̄ = confidence that escalation is warranted

let r_bar = mwc_state_function(L_ALERT, c_alert, n_domains, normalized_deficit);
let severity = if r_bar > 0.8 {
    AlertSeverity::Critical   // High confidence: escalate
} else if r_bar > 0.3 {
    AlertSeverity::Warning    // Moderate confidence: warn
} else {
    AlertSeverity::Info       // Low confidence: note
};
```

**What changes:**
- Hard threshold → smooth sigmoid transition
- Cooperativity: multiple small deficits across domains can collectively trigger escalation (n > 1, AND-gate behavior)
- Leakiness: always some baseline alerting even below threshold (1/(1+L) > 0)
- **This IS the "first insertion point" you suggested**

**Key question:** Should the algedonic system have AND-gate cooperativity (multiple domains must be in deficit before escalation), OR-gate (any domain in deficit triggers escalation), or something tunable?

---

### IP-2: Cybernetics Loop Set-Point Deviation → Regulatory Action

**Current code:** `hkask-cns/src/cybernetics_loop.rs` (sense → compute → act)
**Current behavior:** `if deviation > set_point → produce LoopAction`
**6-Loop:** Cybernetics (sense+compute) → {Inference, Communication} (act)

**Current implementation:**
```rust
// Current: binary deviation detection
"energy_remaining" if dev.direction == DeviationDirection::BelowSetPoint => {
    Some(LoopAction::new(LoopId::Inference, ActionType::Throttle, ...))
}
"variety_deficit" if dev.direction == DeviationDirection::AboveSetPoint => {
    Some(LoopAction::new(LoopId::Curation, ActionType::Escalate, ...))
}
```

**MWC gate proposal:**
```rust
// Proposed: MWC-regulated action gating
// Each set-point comparison IS already an allosteric gate
// Currently implemented as binary threshold
// MWC would give: smooth regulation curve, cooperativity between deviations

// Energy gate: L = high (don't throttle by default)
// α = (1 - energy_remaining/set_point) — how far below set-point
// c = selectivity of energy signal
// R̄ = confidence that throttling is warranted
let r_bar_energy = mwc_state_function(L_THROTTLE, c_energy, 1, energy_deviation);
if r_bar_energy > THROTTLE_THRESHOLD {
    actions.push(LoopAction::new(LoopId::Inference, ActionType::Throttle, ...));
}
```

**What changes:**
- Binary deviation → graded confidence in regulatory action
- Cooperativity: multiple concurrent deviations (energy + variety + error_rate) produce amplified response (cascade detection is currently a warning — MWC makes it quantitative)
- Dynamic range: the system can produce proportional responses, not just on/off actions

**Key question:** Should the Cybernetics Loop use a single multi-ligand MWC gate (all deviations feed into one equation) or separate gates per metric? Multi-ligand gives natural AND/OR behavior; separate gates give independent tuning.

---

### IP-3: Curation Confidence Gate (R̄ = confidence of Curator/LLM)

**Current code:** `hkask-types/src/loops/curation.rs`
**Current behavior:** Curator issues directives directly; no confidence assessment
**6-Loop:** Curation (metacognition) → Cybernetics (regulation)

**This is your key example:**
> "What if R̄ is the confidence level of the Curator or of an LLM in their answer, and the MWC managed confidence in templates or the Curator's decisions — where if confidence is too low the MWC-regulated response is to ask what could be done to increase confidence or reduce uncertainty, and if it's high enough we move to the next step in the process."

**MWC gate proposal:**
```rust
/// Curation confidence gate
/// L = high (skeptical by default: don't act on low confidence)
/// α = evidence strength (LLM confidence, template match score, validation result)
/// c = evidence quality (how much this evidence type shifts confidence)
/// n = number of evidence channels
/// R̄ = confidence that the Curator should proceed with this decision

struct CurationConfidenceGate {
    l: f64,           // Default skepticism (e.g., L = 100 → very skeptical)
    ports: Vec<CurationPort>,  // Evidence channels
    threshold: f64,    // Confidence threshold for action
}

enum CurationPort {
    LlmConfidence { c: f64 },        // LLM's self-assessed confidence
    TemplateMatch { c: f64 },         // Template relevance score
    ValidationResult { c: f64 },      // Schema/validation pass
    HumanConfirmation { c: f64 },     // Human-in-the-loop signal
    HistoricalPrecedent { c: f64 },   // Past success rate
}
```

**The regulated response:**
- R̄ > 0.8: Proceed with decision (high confidence)
- 0.3 < R̄ < 0.8: **Ask what would increase confidence** (the MWC-managed response)
  - "What evidence is missing?"
  - "What uncertainty could be reduced?"
  - "Which port has the weakest signal?"
- R̄ < 0.3: Don't proceed (low confidence), escalate to human

**What changes:**
- Curator decisions become confidence-gated instead of direct
- The "ask what would increase confidence" response IS the allosteric shape of the gate
- Multi-channel confidence (LLM + template + validation + human) gives AND/OR gate behavior
- **This is the metacognitive loop: Curation evaluating its own confidence IS metacognition**

**Key question:** What are the evidence channels (ports) for curation confidence? Should the Curator have configurable L (skepticism) per domain? How does the "ask what would increase confidence" response actually get routed — is it a CuratorDirective, a loop message, or a direct query to the inference loop?

---

### IP-4: Energy Budget Allocation (Soft Limit → MWC Gate)

**Current code:** `hkask-cns/src/energy.rs`
**Current behavior:** `if cost > remaining && hard_limit → reject`
**6-Loop:** Cybernetics (energy) → Inference (budget)

**Current implementation:**
```rust
// Current: hard limit binary gate
pub fn can_proceed(&self, estimated_tokens: u64) -> bool {
    let cost = self.calculate_cost(estimated_tokens);
    cost <= self.remaining || !self.hard_limit
}
```

**MWC gate proposal:**
```rust
// Proposed: MWC-regulated energy gate
// L = moderate (conservative but not absolute)
// α = usage_ratio (how much of budget is consumed)
// c = selectivity (how much usage shifts rejection probability)
// R̄ = confidence that the operation should be allowed
// Leakiness: even near budget limit, some small operations pass (1/(1+L))

let r_bar = mwc_state_function(L_ENERGY, c_energy, 1, usage_ratio);
if r_bar > ENERGY_THRESHOLD {
    allow_operation()
} else {
    reject_or_throttle()
}
```

**What changes:**
- Hard limit → soft sigmoid (operations near the boundary have proportional rejection)
- Leakiness: the system always has some residual capacity (1/(1+L) > 0)
- Cooperativity: concurrent operations can exhibit AND-gate behavior (if multiple agents compete for budget, the MWC equation naturally prioritizes)

**Key question:** Is the hard_limit behavior desirable for energy (preventing overconsumption is critical), or should the MWC soft gate replace it? Perhaps MWC applies only to the soft_limit path?

---

### IP-5: Dampener as Allosteric Suppressor

**Current code:** `hkask-cns/src/dampener.rs`
**Current behavior:** Time-window binary suppress/pass
**6-Loop:** Cybernetics (regulation) → Curation (feedback)

**Current implementation:**
```rust
// Current: binary dampening
if now.duration_since(*last_seen) < self.window {
    return true; // Dampen: suppress
}
```

**MWC gate proposal:**
```rust
// Proposed: MWC-regulated dampening
// L = moderate (allow some repetition through)
// α = repeat_count / window (normalized repetition frequency)
// c = selectivity (how much repetition strengthens suppression)
// R̄ = confidence that this directive should be suppressed
// With n > 1: multiple directive types co-occurring → stronger suppression

let r_bar = mwc_state_function(L_DAMPEN, c_repeat, n_directive_types, repeat_frequency);
if r_bar > DAMPEN_THRESHOLD {
    suppress()  // High confidence that this is oscillation
} else {
    pass()      // Low confidence, allow through
}
```

**What changes:**
- Binary suppress/pass → graded dampening
- Repetition count replaces time window (α = how much repetition, not just "was it seen?")
- Cooperativity: multiple directive types within the window → AND-gate dampening (suppress only when multiple signals oscillate together)
- **The dampener IS already an allosteric mechanism** — MWC formalizes what it already does

**Key question:** Should the dampener use pure MWC (replacing the time window), or should MWC supplement the existing mechanism? The time window provides a clear operational guarantee (after N seconds, always pass) that MWC's smooth curve doesn't.

---

### IP-6: Inference Loop — Model Selection Gate

**Current code:** `hkask-cns/src/inference_loop.rs`, CLI `/model` command
**Current behavior:** User explicitly selects model; no automatic selection
**6-Loop:** Inference (cognition) ← Cybernetics (regulation)

**MWC gate proposal:**
```rust
// Proposed: MWC-gated model selection
// L = high (conservative: don't switch models without evidence)
// α = task complexity / evidence strength
// c = selectivity of the switching signal
// R̄ = confidence that a different model would be better

// Ports: task_complexity, context_length, cost_constraint, quality_requirement
// Each port provides an evidence channel
// Cooperativity: complex task + high quality requirement → switch to larger model (AND gate)
```

**Key question:** Should model selection be automatic (MWC-gated) or remain manual? The `/model` command works well for user control; automatic selection could conflict with user sovereignty.

---

### IP-7: Template Cascade Step Gating

**Current code:** `hkask-templates/src/registry.rs` (cascade: pre → core → post)
**Current behavior:** Sequential execution; no confidence-gated step transitions
**6-Loop:** Inference (template execution) ← Cybernetics (regulation)

**MWC gate proposal:**
```rust
// Proposed: MWC-gated cascade steps
// Each step produces an outcome with confidence
// The next step's α = previous step's R̄
// Low confidence in recognize → inhibit bind
// High confidence in recognize → activate bind

// This IS the cascade flow regulator:
// recognize → [MWC gate] → bind → [MWC gate] → equilibrate → [MWC gate] → assess
```

**Key question:** Should cascade steps be MWC-gated? This would make the template cascade self-regulating — if an early step has low confidence, later steps are automatically inhibited. But it also adds latency and complexity.

---

### IP-8: Episodic → Semantic Consolidation Bridge

**Current code:** Bridge exists in architecture but no MWC regulation
**Current behavior:** Consolidation is one-way, not gated
**6-Loop:** Episodic → Semantic (consolidation bridge)

**MWC gate proposal:**
```rust
// Proposed: MWC-gated consolidation
// L = high (conservative: don't consolidate episodic into semantic without evidence)
// α = episodic_strength (recency, frequency, emotional weight)
// c = consolidation quality (how much episodic evidence supports semantic extraction)
// R̄ = confidence that consolidation is warranted

// This gate prevents premature consolidation:
// Private experience becomes public knowledge only when R̄ > threshold
```

**Key question:** Should consolidation be MWC-gated? The current architecture specifies it as one-way but doesn't regulate when it happens. MWC could ensure only well-validated episodic experiences become semantic knowledge.

---

### IP-9: Communication Loop — Message Priority Routing

**Current code:** `hkask-types/src/loops/dispatch.rs`
**Current behavior:** Priority is set by action type (Critical/Warning/Info)
**6-Loop:** Communication (dumb pipe) ← Cybernetics (regulation)

**MWC gate proposal:**
```rust
// Proposed: MWC-regulated priority assignment
// L = moderate (default priority)
// α = urgency signals (error rate, latency, deficit)
// c = selectivity per signal type
// R̄ = confidence that high-priority routing is warranted
```

**Key question:** Communication is supposed to be a dumb pipe. Adding MWC regulation to priority routing may violate the "communication does NOT dampen, throttle, or circuit-break" principle. Should MWC apply here, or should priority remain set by the issuing loop?

---

### IP-10: GML Self-Regulation (Recursive Application)

**Current code:** `mcp-servers/hkask-mcp-gml/`
**Current behavior:** GML operates on external concepts; no self-analysis
**6-Loop:** Curation (metacognition) → Cybernetics (self-regulation)

**MWC gate proposal:**
```rust
// GML analyzing its own operations — the recursive application
// Each GML operation has a confidence gate:
// - bind: confidence that the effector-concept match is valid
// - equilibrium: confidence that the computed distribution is meaningful
// - cooperate: confidence that the cooperativity measure is significant
// - homeostasis: confidence that the coherence score reflects reality

// This IS metacognition: GML evaluating its own confidence
// R̄ = confidence in GML's own analysis
```

**Key question:** Is GML self-regulation essential for v0.1.0, or is it a v0.2.0 concern?

---

## Decision Summary Matrix

| ID | Insertion Point | Loop | Priority | Risk | Key Decision | Parameters Measurable? |
|----|----------------|------|----------|------|-------------|----------------------|
| **IP-1** | Algedonic Alert | Cybernetics→Curation | **P1** | Low | AND/OR/tunable cooperativity | ✅ Yes (decision logs, deficit counters) |
| **IP-2** | Set-Point Deviation | Cybernetics→Domain | **P1** | Medium | Single multi-ligand vs. separate gates | ✅ Yes (Signal values, decision frequencies) |
| **IP-3** | Curation Confidence | Curation→Cybernetics | **P0** | Medium | Evidence channels, L per domain, routing | ✅ Yes (LLM scores, template matches, validations) |
| **IP-4** | Energy Budget | Cybernetics→Inference | **P2** | Medium | Soft vs. hard limit | ✅ Yes (budget remaining, usage ratio) |
| **IP-5** | Dampener | Cybernetics→Curation | **P2** | Low | MWC supplement vs. replacement | ✅ Yes (directive counts, time windows) |
| **IP-6** | Model Selection | Inference←Cybernetics | **P3** | High | Auto vs. manual (sovereignty) | ✅ Yes (model metrics, task features) |
| **IP-7** | Cascade Steps | Inference←Cybernetics | **P2** | Low | Latency vs. self-regulation | ✅ Yes (step confidence scores) |
| **IP-8** | Consolidation Bridge | Episodic→Semantic | **P2** | Low | Gate the bridge? | ✅ Yes (episodic frequency, validation scores) |
| **IP-9** | Message Priority | Communication | **⚠️** | **High** | May violate "dumb pipe" principle | N/A (excluded per principle) |
| **IP-10** | ARL Self-Regulation | Curation | **P3** | Low | v0.1 vs. v0.2 | ✅ Yes (operation outcomes, confidence deltas) |

**Note:** The "Parameters Measurable?" column addresses the internal review's category error objection. For ALL regulation insertion points, the MWC parameters map to measurable operational quantities — the category error does not apply.

---

## The Architectural Question

**Is the MWC equation THE fundamental primitive for all decision gates in hKask?**

Option A: **MWC as universal regulation primitive** — Every decision point in the 6-loop system is an AllostericGate. The type is defined once in `hkask-cns` and used everywhere. The Cybernetics Loop's entire compute phase becomes "evaluate allosteric gates, route actions based on R̄."

Option B: **MWC as meta-loop primitive only** — MWC gates apply only to the meta loops (Cybernetics, Curation). Domain loops (Inference, Episodic, Semantic) use simpler, direct logic. Communication remains a dumb pipe with no MWC.

Option C: **MWC as CNS-native primitive** — MWC lives inside `hkask-cns` as the regulation kernel. The Cybernetics Loop uses allosteric gates for all its sense→compare→compute→act decisions. Other loops call into CNS when they need regulation. The conceptual-analysis KnowAct (ARL/Allosteric Thinking) is separate from the regulation primitive, though it shares the same mathematical kernel.

Option D: **Something else I haven't considered**

**My recommendation:** Option C, starting with IP-1 (algedonic) and IP-3 (curation confidence) as the first two insertion points. Here's why:

1. **IP-1 (algedonic) is the safest first insertion** — it replaces a binary threshold with a smooth curve, and the existing behavior is preserved as the limit case (L → ∞, n_H → ∞). It's backward-compatible. The parameters are directly measurable from system logs.

2. **IP-3 (curation confidence) is the most impactful** — it gives the Curation Loop its metacognitive capability. Without confidence-gated decisions, curation is just observation; with it, curation becomes genuine metacognition. The evidence channels are real operational signals (LLM confidence scores, template match scores, validation results).

3. **IP-9 (communication priority) should be EXCLUDED** — adding MWC regulation to communication would violate the "dumb pipe" principle. The issuing loop should set priority, not the transport.

4. **IP-4 (energy budget) needs careful design** — hard limits exist for safety. MWC soft gates can supplement but should not replace the hard limit.

5. **All regulation gates include temporal dynamics** — τ (relaxation time) and hysteresis are required, not optional, per the internal review's finding that agent interactions are inherently sequential.

6. **The `cooperate` operation is replaced with coupling coefficients** — multiplying Hill coefficients was semantically undefined (per internal review). Network-graph coupling weights connect to the RBM formulation.

7. **The `homeostasis` operation includes `rebalance`** — monitoring alone is insufficient (per internal review). The Cybernetics Loop's `compute()` phase IS the rebalance mechanism; making this explicit in the MWC formulation gives it a mathematical basis.

But this is your decision. What do you think?

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*