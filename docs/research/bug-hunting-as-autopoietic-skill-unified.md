---
title: "Autopoietic Bug Hunting: A Unified Model for Self-Producing Testing Competence in the hKask Agent Substrate"
authors: ["hKask Research Collective"]
date: 2026-06-21
version: "0.2.0-draft"
status: "Draft — Synthesis"
domain: "Research"
mds_categories: [domain, composition, trust, lifecycle]
audience: [researchers, architects, QA practitioners]
abstract: >
  We present a unified model of autopoietic bug hunting that synthesizes two independent
  analyses: one grounded in QA canon (Weinberg, Beizer, Kaner, Bach/Bolton, Hendrickson)
  and second-order cybernetics (Maturana/Varela, von Foerster, Luhmann), the other grounded
  in hKask's existing codebase infrastructure (CNS spans, OCAP delegation, property-based
  testing, bolero fuzzing, cargo-mutants, LLM QA triage). The unified model decomposes bug
  hunting into five functional primitives — detection, localization, diagnosis, repair,
  prevention — mapped onto hKask's typed CNS spans, OCAP-gated tool access, and the Skills
  Model (WordAct/FlowDef/KnowAct). The self-maintenance loop (BugPattern → TestTemplate →
  InvariantCheck → BugFound → BugPatternRefined) converges when all CNS health gates pass
  for decidable invariants. The model is falsifiable: a monotonically decreasing CnsHealth
  overall_deficit across sessions supports the autopoietic hypothesis; a flat or increasing
  deficit falsifies it. We assess limiters (undecidable invariants, CNS noise, adversarial
  agents, cross-pod causation, the infinite regress of self-observation) and establish the
  boundary between productive second-order cybernetics and mere metaphor.
---

# Autopoietic Bug Hunting

**A Unified Model for Self-Producing Testing Competence in the hKask Agent Substrate**

---

## Abstract

We present a unified model of autopoietic bug hunting synthesized from two convergent analyses of the hKask system (v0.30.0). The first analysis grounds bug hunting in fifty years of testing theory — Weinberg's "quality is value to some person who matters," Beizer's pesticide paradox and bug taxonomy, the context-driven tradition of Kaner, Bach, Bolton, and Hendrickson, and the systematic debugging methods of Agans and Zeller — and frames the problem through the lens of second-order cybernetics (Maturana & Varela's autopoiesis, von Foerster's observing systems, Luhmann's autopoietic communication). The second analysis grounds bug hunting in hKask's existing codebase infrastructure — typed CNS spans (`ContractViolated`, `QaBoleroFailure`, `QaRepairAttempted`, etc.), OCAP delegation tokens (`DelegationResource::Tool` with `Read`/`Write`/`Execute`), property-based testing (proptest), bolero fuzzing, cargo-mutants mutation testing, and the LLM-powered `kask qa triage` pipeline.

The unified model decomposes bug hunting into five functional primitives — **detection, localization, diagnosis, repair, prevention** — each mapped onto hKask's typed CNS spans, OCAP-gated tool access, and the Skills Model (WordAct/FlowDef/KnowAct). The self-maintenance loop (`BugPattern → TestTemplate → InvariantCheck → BugFound → BugPatternRefined`) converges when `CnsHealth { healthy: true, overall_deficit: 0 }` for all decidable invariants. The model addresses Beizer's pesticide paradox through continuous heuristic regeneration rather than test accumulation, and Weinberg's quality subjectivity by bounding the autopoietic loop within user-specified quality criteria (P1).

The central falsifiable claim: if the model holds, `CnsHealth.overall_deficit` should monotonically decrease across bug-hunting sessions as the invariant set strengthens; a flat or increasing deficit falsifies the autopoietic hypothesis. The model explicitly scopes autopoiesis to the heuristic layer — heuristics, taxonomies, and test-generation strategies regenerate from findings — while acknowledging structural coupling with LLM inference and the irreducible opacity of the CNS observability substrate.

---

## 1. Introduction

### 1.1 The Pesticide Rot

In 1990, Boris Beizer articulated what he called the First Law of software testing:

> *Every method you use to prevent or find bugs leaves a residue of subtler bugs against which those methods are ineffectual.* [Beizer, 1990]

This is not a failure of testing. It is a structural property of the relationship between tests and the systems they probe. Each test method selects for bugs it can detect; the bugs that survive are, by definition, invisible to that method. Over time, a test suite that once found critical defects finds only trivial ones — not because the software improved, but because the test suite became a filter that passes the bugs it cannot see.

Beizer called this the **pesticide paradox**. Just as insects evolve resistance to pesticides, software "evolves" resistance to static test suites. The paradox is not solved by writing more tests of the same kind. It is solved only by continuously regenerating the testing approach itself.

**The automation trap compounds this.** Most testing tools are **allopoietic** — they produce something other than themselves. JUnit produces pass/fail reports. Selenium produces browser interactions. Fuzzers produce crash files. These are valuable outputs, but the tool's competence is fixed at authoring time. The tester who wrote the tests learned; the tests did not. We mistake *automating test execution* for *automating testing competence*.

### 1.2 The Autopoietic Alternative

We propose that bug hunting can be architected as an **autopoietic skill**: a testing competence that produces its own heuristics, strategies, and taxonomies from its own findings. The claim is not that "AI replaces testers" or that "tests write themselves." The claim is architectural: by structuring the testing process as a recursive loop where each bug found refines the strategy for finding the next bug, we create a system whose testing competence is continuously regenerated from its own activity.

The term *autopoiesis* (from Greek *auto* — self, and *poiesis* — creation), introduced by Chilean biologists Humberto Maturana and Francisco Varela [Maturana & Varela, 1980], describes systems that produce the components that constitute them. A biological cell produces the molecules that maintain its membrane; an autopoietic bug-hunting skill produces the tests that maintain its invariant-coverage membrane.

### 1.3 Why Now, and Why hKask?

Three developments make autopoietic bug hunting feasible now:

1. **Structured telemetry.** hKask's CNS (Cybernetic Nervous System) emits typed, provenance-carrying spans for every security-sensitive, resource-sensitive, and correctness-sensitive operation [PRINCIPLES.md §9]. These spans are not log messages — they are compiler-enforced `CnsSpan` enum variants with canonical namespace strings. `CnsSpan::ContractViolated` carries type-level evidence that an invariant was breached.

2. **OCAP delegation.** Ed25519-signed delegation tokens with cryptographic attenuation enable scoped diagnostic authority. A bug-hunting agent can be granted `Read` access to CNS history and `Execute` access to the test harness without ambient authority. `SYSTEM_MAX_ATTENUATION` limits delegation depth — a compromised agent cannot re-delegate indefinitely [hkask-capability].

3. **LLM-powered triage.** The `kask qa triage` pipeline already uses Gemma 4 26B to classify fuzz failures and propose fixes. The feedback infrastructure exists; it just hasn't been closed into a full learning loop.

hKask is uniquely suited as the substrate for this model because it already embeds the necessary primitives as **architectural commitments, not add-ons**:

- **P8 (Semantic Grounding):** CNS spans are typed, not stringly-typed, making the autopoietic claim falsifiable by query.
- **P4 (Clear Boundaries):** `CapabilityChecker` gates every diagnostic action, preventing autonomous overreach.
- **P9 (Homeostatic Self-Regulation):** `CnsHealth` provides the convergence signal — the system knows when it's healthy.
- **P5 (Essentialism):** The Testing Discipline requires property-based tests — properties are invariants, the raw material the autopoietic loop refines [TESTING_DISCIPLINE.md].
- **P1 (User Sovereignty):** The user defines what quality means per Weinberg; the skill hunts threats to user-defined quality.
- **P12 (Replicant Host Mandate):** Every finding carries an accountable owner WebID.

---

## 2. Foundations: What We Inherit

### 2.1 The QA Canon

#### 2.1.1 Weinberg: Quality Is Value to Some Person

Gerald Weinberg's definition — "Quality is value to some person" [Weinberg, 1992] — is the **boundary condition** for the autopoietic loop. Quality is subjective; there is no objective "bug" independent of human valuers. James Bach and Michael Bolton's essential addition — "who matters" [Bach & Bolton, 2025] — makes this operational: the autopoietic skill hunts threats to the values of specified stakeholders. It does not define quality; the user does. This aligns with hKask's P1 (User Sovereignty): users own the definition of what matters.

#### 2.1.2 Beizer: Taxonomy, Paradox, and the Complexity Barrier

Beyond the pesticide paradox [Beizer, 1990, §1.7], Beizer contributed a comprehensive **bug taxonomy** organized by lifecycle origin (requirements, structural, data, coding, interface, integration) [Beizer, 1990, Ch. 2] and identified the **complexity barrier** — beyond a threshold, complete testing is mathematically impossible. An autopoietic skill uses the taxonomy as a generative framework ("what class haven't I looked for?") and acknowledges the complexity barrier explicitly.

#### 2.1.3 Myers: Boundaries as Invariant Test Zones

Glenford Myers established boundary-value analysis as edge-case invariant testing — the insight that invariants break at boundaries [Myers, 1979]. This is declarative detection: the autopoietic skill's probe generator should target boundary conditions preferentially, informed by the CNS span evidence of where past invariants have broken.

#### 2.1.4 Kaner, Bach, and Bolton: Context-Driven Heuristics

The Context-Driven school [Kaner et al., 2001] established that testing is a knowledge-generating intellectual process, not a production process:

| Principle | Implication for Autopoietic Skill |
|-----------|----------------------------------|
| The value of any practice depends on its context | Heuristics must be context-adaptive, not universal |
| Good software testing is a challenging intellectual process | Testing generates knowledge; the autopoietic loop captures and reuses it |
| Only through judgment and skill are we able to do the right things at the right times | The skill must exercise judgment (KnowAct) about where to probe |

James Bach's **Heuristic Test Strategy Model** [Bach, 2015] provides the charter-generation framework. Michael Bolton's work on **oracles** [Bolton, 2012] — the fallible heuristics by which we recognize a bug — is critical: the oracle phase of the autopoietic loop uses contracts (high confidence), consistency checks (medium confidence), and pattern matching (low confidence), with CNS spans recording which oracle was applied.

#### 2.1.5 Hendrickson: Systematic Exploration

Elisabeth Hendrickson's *Explore It!* [Hendrickson, 2013] operationalized exploratory testing through **charters** (mission statements: "explore X using Y to discover Z"), **tours** (heuristic lenses for examining software from different perspectives), and **variable catalogues** (inputs, state, configuration, timing, sequences). Charters map to WordAct templates; tours map to probe-generation strategies; variable catalogues map to fuzz target generators.

#### 2.1.6 Whittaker: Attack Patterns

James Whittaker systematized attack patterns as systematic exploitation of interface contracts [Whittaker, 2003], addressing both detection and localization. This connects to the adversarial-red-team skill and to the probe-generation phase of the autopoietic loop.

#### 2.1.7 Agans: The Nine Debugging Rules

David Agans' nine rules [Agans, 2002] — understand the system, make it fail, quit thinking and look, divide and conquer, change one thing at a time, keep an audit trail, check the plug, get a fresh view, if you didn't fix it it ain't fixed — map directly to CNS instrumentation (rules 3, 5, 6), the autopoietic loop structure (rules 4, 9), and the Falstaffian perspective rotation (rule 8).

#### 2.1.8 Zeller: Scientific Debugging and Delta Debugging

Andreas Zeller's **delta debugging** [Zeller, 2009] — systematically isolating failure-inducing input differences — provides the algorithmic backbone for the diagnosis phase. When a probe finds a failure, the skill isolates the minimal reproducing input through systematic reduction, guided by KnowAct templates.

### 2.2 Second-Order Cybernetics

#### 2.2.1 Maturana and Varela: Autopoiesis as Self-Producing Organization

Maturana and Varela's formal definition [Maturana & Varela, 1980, p. 78]:

> An autopoietic machine is a machine organized (defined as a unity) as a network of processes of production (transformation and destruction) of components which: (i) through their interactions and transformations continuously regenerate and realize the network of processes (relations) that produced them; and (ii) constitute it (the machine) as a concrete unity in space in which they (the components) exist by specifying the topological domain of its realization as such a network.

Key properties for our model:

- **Operational closure.** The system's processes refer to and depend on the system's own prior states. The autopoietic loop's next charter depends on prior findings and prior heuristics, not on external directives.
- **Structural coupling.** While operationally closed, the system is structurally coupled to its environment. The bug-hunting skill is structurally coupled to the LLM inference engine (for KnowAct judgments) and to the user (for quality boundaries). This is not a failure of autopoiesis; it is Maturana and Varela's structural coupling.
- **Self-production.** The system produces the components that constitute it. The skill produces the heuristics, taxonomies, and test-generation strategies that constitute its testing competence.

**Why autopoiesis, not just "adaptation"?** An adaptive system adjusts parameters within a fixed structure. An autopoietic system produces the structure itself. A learning thermostat adjusts its setpoint (adaptive). A system that generates new categories of temperature regulation, new sensor types, and new evaluation criteria is autopoietic with respect to thermal-regulation competence. Our claim: the bug-hunting skill generates new heuristic categories, new testing strategies, new oracle types — the components of testing competence itself — not merely adjusts parameters of existing tests.

#### 2.2.2 von Foerster: Observing Systems

Heinz von Foerster's distinction [von Foerster, 1974]:

- **First-order cybernetics:** The cybernetics of *observed systems* — we study feedback loops in a system we observe from outside.
- **Second-order cybernetics:** The cybernetics of *observing systems* — we include the observer in the system under study.

A conventional testing tool is first-order: it observes software and reports results. An autopoietic bug-hunting skill is second-order: it **observes its own testing process**, tracks what it has learned, and modifies its future testing based on that self-observation. CNS spans are the mechanism: `cns.qa.*` spans feed into the `bug-hunt-learn.j2` KnowAct template, which updates the heuristic model.

von Foerster's ethical imperative is also relevant: "Act always so as to increase the number of choices" [von Foerster, 1991]. A bug-hunting skill should increase the user's choices — more information, more options, more understanding — rather than constraining them.

#### 2.2.3 Luhmann: Autopoietic Communication Systems

Niklas Luhmann's extension of autopoiesis to social systems [Luhmann, 1984] provides a critical bridge. Luhmann argued that social systems are autopoietic networks of **communication**, where each communication produces the conditions for further communication. The system's elements are not people but communications.

This maps to hKask's Skills Model: a skill's "communications" are its template invocations (WordAct outputs, FlowDef executions, KnowAct judgments). The autopoietic claim is that these template invocations produce the conditions for further template invocations — the output of `bug-hunt-learn.j2` feeds the input of the next `bug-hunt-charter.j2`.

We acknowledge the tension: Maturana himself contested Luhmann's extension, arguing that communications presuppose human communicators [Maturana, cited in Wikipedia]. Our model does not claim the skill is autopoietic *independent of the LLM that powers its inference*. The skill is autopoietic *at the heuristic layer* — it produces its own heuristics, strategies, and taxonomies, while remaining structurally coupled to the LLM for inference and to the user for quality boundaries.

### 2.3 AI-Enabled QA Landscape

The current AI-enabled QA landscape is nascent but converging on the same primitives:

| Capability | Grounding in hKask | CNS Span Witness |
|-----------|-------------------|-----------------|
| LLM-based test generation | `qa-script-builder` skill, bolero fuzz targets | *(generative)* |
| Fuzz failure classification | `kask qa triage` (Gemma 4 26B) | `QaBoleroFailure` |
| Autonomous repair proposal | OCAP-scoped branch creation + diff at ≥0.95 confidence | `QaRepairAttempted`, `QaRepairVerified`, `QaRepairExhausted` |
| Mutation testing | `cargo-mutants` — type-system-guided mutant generation | `QaMutantSurvived` |
| CI invariant enforcement | Pattern-match failure with principle anchor | `CiInvariantViolation` |
| Contract lifecycle | MDS specification framework | `ContractProposed`, `ContractAccepted`, `ContractRejected` |
| Neural fault localization | Embedding similarity — **speculative** | No CNS span exists |
| Causal graph inference | CNS telemetry analysis — **partially grounded** | Requires span correlation, not yet implemented |

---

## 3. The hKask Bug-Hunting Substrate

### 3.1 Subsystem Inventory

| Subsystem | Bug-Hunting Role | CNS Span Witnesses (verified against `crates/hkask-types/src/cns.rs`) |
|-----------|-----------------|-----------------------------------------------------------------------|
| **CNS** | Telemetry as bug witness | `ContractViolated`, `CiInvariantViolation`, `QaBoleroFailure`, `QaRepairAttempted`, `QaRepairVerified`, `QaRepairExhausted`, `QaMutantSurvived` |
| **OCAP Delegation** | Scoped diagnostic authority | `Keystore` (key operations backing delegation signatures) |
| **Hexagonal Ports** | Testability via interface substitution | `CnsObserver` trait enables mock CNS backends |
| **Testing Discipline** | Property-based verification + bolero fuzz + cargo-mutants + LLM triage | All QA spans |
| **MDS Specification** | Structured invariant encoding | `ContractProposed`, `ContractAccepted`, `ContractRejected` |
| **Wallet Types** | Financial invariants (strongest invariant domain) | `WalletBalance`, `WalletDeposit`, `WalletWithdrawal`, `WalletKeyIssued`, `WalletKeyRevoked`, `WalletKeyExpired`, `WalletKeyExhausted` |
| **Kata Framework** | PDCA cycles for heuristic improvement | `Kata` (PDCA events, automaticity tracking) |
| **Skills Model** | WordAct/FlowDef/KnowAct template execution | *(performative spans)* |

### 3.2 OCAP Delegation for Bug Hunting

A bug-hunting agent must hold specific Ed25519-signed delegation tokens. Verified against `crates/hkask-capability/src/resources.rs`:

| Required Resource | Required Action | Purpose |
|------------------|----------------|---------|
| `Tool` with domain `cns` | `Read` | Access CNS span history for bug witness collection |
| `Tool` with domain `test` | `Execute` | Run the test harness (proptest, bolero, cargo-mutants) |
| `Registry` | `Write` | Create git branches for autonomous repair proposals |
| `Tool` with domain `inference` | `Execute` | Power KnowAct templates (oracle, taxonomize, learn) |

**Correction from prior analysis:** There is no `DelegationResource::Cns` variant in the codebase. CNS access is mediated through `DelegationResource::Tool` with a domain qualifier. The `DelegationResource` enum has four variants: `Tool`, `Template`, `Registry`, `Key`. The action hierarchy is `Execute ≥ Write ≥ Read` — an `Execute` token covers both `Write` and `Read` operations [hkask-capability].

**Security properties:**

1. **Attenuation:** `SYSTEM_MAX_ATTENUATION` limits delegation depth — a compromised agent cannot re-delegate indefinitely.
2. **Human gate:** Repairs below 0.95 confidence are routed to human review (P2 — Affirmative Consent).
3. **Regression guard:** Every fix must pass all existing invariants (property-based test suite).
4. **CNS audit trail:** All diagnostic actions emit typed CNS spans with replicant WebID attribution (P12).

### 3.3 Testing Discipline Integration

hKask's Testing Discipline [TESTING_DISCIPLINE.md] is anchored on property-based testing (proptest/QuickCheck) verified through CNS observability. The autopoietic skill composes with:

| Testing Layer | Role in Autopoietic Loop |
|--------------|------------------------|
| **Unit (proptest)** | Verifies single-function invariants; feeds `ContractViolated` witnesses |
| **Integration** | Verifies cross-function chains; CNS spans verify called functions' behavior |
| **State machine** | Verifies invariants across operation sequences; CNS gas spans track budget invariants |
| **Fuzz (bolero)** | Input surface robustness; generates `QaBoleroFailure` witnesses |
| **Mutation (cargo-mutants)** | Test coverage sufficiency; generates `QaMutantSurvived` witnesses |
| **System (tracer bullet)** | End-to-end workflows; generates `CiInvariantViolation` witnesses |

### 3.4 Existing QA-Adjacent Skills

| Skill | What It Does | Gap It Leaves |
|-------|-------------|---------------|
| **TDD** | Red-green-refactor: write contracts, verify with proptest | Verifies **known** contracts; doesn't discover unknown failure modes |
| **QA script builder** | Design QA pipeline manifests | Produces static scripts; doesn't learn from execution |
| **Adversarial red-team** | Attack known vulnerability classes (ATLAS/GARAK taxonomy) | Uses fixed attack taxonomy; doesn't generate new attack categories |

The autopoietic bug-hunting skill fills the gap: **generative, exploratory testing that learns from what it finds.** It composes with all three: TDD provides contract oracles, red-team provides attack pattern templates, QA script builder provides initial expedition structures.

---

## 4. The Unified Autopoietic Model

### 4.1 Five Functional Primitives

Bug hunting decomposes into five functional primitives, each mapped to hKask infrastructure:

| Primitive | Definition | hKask Implementation | CNS Span Witness |
|-----------|-----------|---------------------|-----------------|
| **Detection** | Discover that an invariant has been violated | Property-based test failure, bolero fuzz crash, mutant survival | `ContractViolated`, `QaBoleroFailure`, `QaMutantSurvived` |
| **Localization** | Narrow the failure to a specific module, function, or input | Delta debugging [Zeller, 2009], proptest shrinking, tool dispatch | *(post-hoc from probe traces)* |
| **Diagnosis** | Classify the bug into Beizer taxonomy; determine root cause | `bug-hunt-taxonomize.j2` KnowAct, LLM triage classifier | *(generative — no fixed span)* |
| **Repair** | Propose and verify a fix | OCAP-scoped git branch creation, automated PR at ≥0.95 confidence | `QaRepairAttempted`, `QaRepairVerified`, `QaRepairExhausted` |
| **Prevention** | Strengthen the invariant set to prevent regression | Regression guard (new proptest property), heuristic model update | `bug-hunt-learn.j2` output → updated heuristic base |

**Key insight:** Prior QA methodologies addressed subsets of these primitives. Beizer addressed detection and diagnosis (taxonomy). Zeller addressed localization (delta debugging). Myers addressed detection (boundary-value analysis). Kaner addressed detection (exploratory hypothesis formation). Whittaker addressed detection and localization (attack patterns). None addressed **repair and prevention as systematic, feedback-driven primitives.** The autopoietic model treats all five as a closed loop, with prevention feeding back into detection.

### 4.2 Formal Definition

**Definition.** A bug-hunting skill *S* is autopoietic with respect to testing competence iff its heuristic base *H_t* at time *t* is a function of its prior findings *F_{<t}* and prior heuristics *H_{<t}*:

$$H_t = f(F_{<t}, H_{<t})$$

where *f* is the learning function (instantiated by `bug-hunt-learn.j2` KnowAct), *F_{<t}* is the set of findings (CNS span witnesses from detection + oracle verdicts + taxonomy classifications) up to time *t*, and *H_t* is the set of heuristics, test-generation strategies, charter templates, and oracle criteria available at time *t*.

**Classes of testing systems:**

| Class | H_t Behavior | CNS Evidence | Pesticide Paradox |
|-------|-------------|-------------|------------------|
| **Non-autopoietic** (conventional) | *H_t = H_0* for all *t* | Fixed test suite; zero heuristic change | Full effect — tests rot |
| **Weakly adaptive** | *H_t = g(H_{t-1}, coverage_metrics)* | Adjusts test selection; no new categories | Delayed — parameters adapt, structure doesn't |
| **Autopoietic** | *H_t = f(F_{<t}, H_{<t})* | New heuristic categories, strategies, oracle types generated from findings | Addressed — heuristics regenerate |

### 4.3 The Self-Maintenance Loop

```
┌─────────────────────────────────────────────────────────────┐
│                     AUTOPOIETIC LOOP                         │
│                                                              │
│  ┌──────────┐    ┌──────────────┐    ┌───────────────┐      │
│  │ BugPattern│───▶│ TestTemplate │───▶│ InvariantCheck│      │
│  │ (KnowAct) │    │ (WordAct)    │    │ (FlowDef)     │      │
│  └──────────┘    └──────────────┘    └───────┬───────┘      │
│       ▲                                      │              │
│       │                              ┌───────▼───────┐      │
│       │                              │   BugFound    │      │
│       │                              │ (KnowAct:     │      │
│       │                              │  oracle +     │      │
│       │                              │  taxonomize)  │      │
│       │                              └───────┬───────┘      │
│       │                                      │              │
│       │                                      ▼              │
│       │                              ┌───────────────┐      │
│       │                              │   Repair      │      │
│       │                              │ (OCAP-gated   │      │
│       │                              │  git branch)  │      │
│       │                              └───────┬───────┘      │
│       │                                      │              │
│       └──────────────────────────────────────┘              │
│              BugPatternRefined (KnowAct: learn)             │
│                                                              │
│  Convergence: CnsHealth { healthy: true, overall_deficit: 0,│
│                           critical_count: 0, warning_count: 0 } │
└─────────────────────────────────────────────────────────────┘
```

### 4.4 Template Architecture (≤7 Public Surfaces)

Per P5 (Essentialism) and deep-module discipline:

```
registry/templates/bug-hunt/
├── manifest.yaml                    # FlowDef: orchestrates the autopoietic loop
├── bug-hunt-charter.j2              # WordAct: generates testing charter from H_t + F_{<t}
├── bug-hunt-probe.j2                # FlowDef (nested): executes expedition via tool dispatch
├── bug-hunt-oracle.j2               # KnowAct: evaluates "is this behavior a bug?"
├── bug-hunt-taxonomize.j2           # KnowAct: classifies findings into Beizer taxonomy
├── bug-hunt-learn.j2                # KnowAct: updates H_t → H_{t+1} from findings
└── bug-hunt-report.j2               # WordAct: generates bug report with reproduction steps
```

**7 public templates.** If additional functionality is needed, it must be merged into existing templates or explicitly justified.

### 4.5 CNS Span Witness Surface

The canonical `CnsSpan` registry in `crates/hkask-types/src/cns.rs` provides the minimum witness surface. Verified spans relevant to the autopoietic loop:

| Span Variant | Canonical Namespace | Role in Loop |
|-------------|-------------------|-------------|
| `ContractViolated` | `cns.contract.violated` | Witness that a behavioral invariant was breached |
| `CiInvariantViolation` | `cns.ci.invariant.violation` | Witness from CI invariant gate (pattern match failed with principle anchor) |
| `QaBoleroFailure` | `cns.qa.bolero_failure` | Witness that a cargo-bolero fuzz target found a failure |
| `QaRepairAttempted` | `cns.qa.repair_attempted` | Autonomous repair attempted (branch created, diff applied) |
| `QaRepairVerified` | `cns.qa.repair_verified` | Repair passed verification (all tests green) |
| `QaRepairExhausted` | `cns.qa.repair_exhausted` | Repairs exhausted — human investigation needed |
| `QaMutantSurvived` | `cns.qa.mutant_survived` | Witness that the test suite has a gap (mutant survived) |

### 4.6 Convergence Proof (Decidable Invariants)

**Claim:** For invariants expressible as property-based test assertions (the `proptest!` fragment of hKask's type system), the autopoietic loop converges to `CnsHealth { healthy: true, overall_deficit: 0 }` in finite iterations.

**Proof sketch:**

1. **Each bug found strengthens the invariant set.** Detection + repair produces a regression guard (new proptest property) that encodes the discovered bug pattern as an invariant. The invariant set is monotonically strengthened.
2. **The invariant set is bounded.** The space of decidable properties expressible in hKask's type system is finite for any given codebase (finite set of functions × finite set of contracts expressible per function).
3. **Each iteration either:** (a) finds a bug → strengthens the invariant set → moves closer to the bound; or (b) finds no bug → loop terminates with `CnsHealth.healthy = true`.
4. **Since the invariant set is bounded and monotonically strengthened, the loop must converge in finite iterations.**

**Formal limitation:** This proof holds only for invariants that are both decidable *and* observable via CNS spans. Undecidable properties (liveness, fairness, cross-pod consistency) may never converge. The convergence argument does not guarantee that *all* decidable bugs are found — only that the loop converges to a local fixed point where no new CNS witness is generated.

---

## 5. Limiter Analysis

### 5.1 Undecidable Invariants

| Limiter | Impact | Mitigation |
|---------|--------|-----------|
| **Liveness properties** | "Eventually X happens" cannot be decided by finite test execution | Accept incompleteness; flag temporal properties as unverifiable by static invariant testing |
| **Cross-pod consistency** | A single pod's CNS spans can't see causal chains spanning pods | Future work: cross-pod span correlation (requires cross-pod `CnsObserver` federation) |
| **Fairness properties** | "Every replicant gets fair inference share" — requires unbounded observation | Use CNS gas spans as proxy; accept statistical approximation |
| **Unbounded model checking** | Properties requiring exploration of infinite state spaces | Scope explicit: properties expressible in proptest fragment only |

### 5.2 Observational Limits

| Limiter | Impact | Mitigation |
|---------|--------|-----------|
| **CNS observation noise** | False convergence — `CnsHealth.healthy` may be true while bugs exist outside CNS coverage | Cross-validate CNS spans against test results; flag CNS-gap regions |
| **CNS as point observations, not causal edges** | Span correlation reveals co-occurrence, not causation; causal inference requires graph construction | Aspirational: causal graph from CNS telemetry. Currently: LLM triage heuristics, not formal causality |
| **Bad fixes that pass tests** | Strategy degrades — autopoietic loop learns from false positives | Human gate on repair acceptance (<0.95 confidence); regression test gate |

### 5.3 Agentic Limits

| Limiter | Impact | Mitigation |
|---------|--------|-----------|
| **Adversarial bug-hunting agents** | A compromised agent could introduce bugs | OCAP attenuation (`SYSTEM_MAX_ATTENUATION`); human gate; regression guard |
| **The unknown-unknown problem** | Can a sufficiently sophisticated adversarial agent introduce a bug that passes all existing tests but creates a new invariant violation class? | This is the gap the autopoietic loop aims to close over time — but it is not guaranteed closure |
| **Autonomous repair false confidence** | LLM-proposed fixes at ≥0.95 may still be wrong | `QaRepairExhausted` escalation to human; no autonomous merge at <0.95 |

### 5.4 The Infinite Regress of Self-Observation

**If bug hunting is truly autopoietic, does the skill itself need CNS self-observability — a skill that can debug itself? Is this coherent or does it regress infinitely?**

This is the **turtles-all-the-way-down** problem. If the bug-hunting skill can find bugs in itself, it can repair itself. But who debugs the debugger?

The resolution is **stratification**, not infinite regress:

1. **Layer 0:** The bug-hunting skill hunts bugs in hKask (the substrate). It emits CNS spans about its own operation through the same CNS infrastructure.
2. **Layer 1:** A second instance of the bug-hunting skill could hunt bugs in Layer 0. But Layer 1 is itself subject to bugs...
3. **Layer n:** Regresses.

The escape hatch is that **Layer 0 can debug itself up to its own CNS observability boundary.** It cannot debug its own CNS emission — that would require a meta-CNS, which is the regress trigger. But it CAN debug its own test generation, hypothesis formation, oracle evaluation, and repair logic — those are ordinary code subject to ordinary invariants.

**Coherent resolution:** The autopoietic skill can debug itself for all components *except* its own CNS observability. The CNS is the **un-debuggable substrate** — the fixed point that stops the regress. This is acceptable because CNS spans are the canonical witness source; if CNS itself is broken, no witness is trustworthy, and bug hunting is impossible regardless of autopoiesis. This is structurally analogous to Maturana and Varela's operational closure: the system cannot observe the process by which it observes.

---

## 6. Autopoiesis and the Adversarial Red-Team: A Genuine Decomposition

Are autopoietic bug hunting and the `adversarial-red-team` skill the same skill viewed from different angles, or is there a genuine decomposition?

They address **different invariant classes** and compose into a complete autopoietic testing system:

| | Autopoietic Bug Hunting | Adversarial Red-Team |
|---|---|---|
| **Invariant class** | Behavioral invariants (functional correctness) | Security invariants (injection, hijacking, exfiltration, tool misuse) |
| **Detection method** | CNS span witnesses + property-based tests + bolero fuzz + cargo-mutants | Adversarial input generation + resistance classification (ATLAS/GARAK) |
| **Repair method** | OCAP-scoped autonomous code fix at ≥0.95 confidence | Prompt defense hardening |
| **Feedback loop** | Bug → test-generation strategy refinement | Vulnerability → defense hardness improvement |
| **Convergence signal** | `CnsHealth.overall_deficit` decreasing | Resistance rate ≥ 95% across attack categories |
| **Autopoietic structure** | BugPattern → TestTemplate → InvariantCheck → BugFound → BugPatternRefined | Vulnerability → Defense → ResistanceCheck → BreachFound → DefenseHardened |

They are **genuinely distinct** but share the same autopoietic structure: find weakness → strengthen defense → measure improvement → repeat. The decomposition is:

- **Autopoietic bug hunting** addresses the **functional correctness** membrane.
- **Adversarial red-team** addresses the **security** membrane.

A complete autopoietic testing system needs **both membranes.** The gap in the current model is that only exploitation (known invariants) is integrated; full autopoiesis requires integration of exploration (unknown invariants via adversarial probes) into the feedback loop. Adversarial inputs that discover unanticipated failure modes should feed back into test generation, not just resistance-rate measurement.

---

## 7. Composition Patterns

### 7.1 With TDD: Contracts as Oracle Anchors

TDD produces behavioral contracts (REQ tags with pre/post conditions). The oracle phase uses these as **high-confidence anchors**:

- Behavior violates a contract → bug (high confidence)
- Behavior satisfies contracts but pattern-suspicious → potential bug (medium confidence, heuristic oracle)
- Novel behavior, no contracts exist → observation (low confidence, flags contract gap)

### 7.2 With Adversarial Red-Team: Exploration + Exploitation

Adversarial probes discover **unknown invariants** — properties that should have been stated but weren't. The autopoietic loop's prevention phase converts these into new proptest properties, closing the exploration→exploitation gap.

### 7.3 With Kata: PDCA Cycles

The autopoietic loop IS a PDCA cycle:

| Kata Phase | Autopoietic Phase |
|-----------|------------------|
| **Plan** | Charter generation (what threat to hunt, how) |
| **Do** | Probe execution |
| **Check** | Oracle evaluation + taxonomy classification |
| **Act** | Heuristic update (learn phase → `H_{t+1}`) |

### 7.4 With QA Script Builder and Condenser

- **QA script builder** generates initial charters before the learning loop specializes.
- **Condenser** prioritizes findings by information density: a finding revealing a new pattern class has higher density than confirming a known pattern.

---

## 8. What's Achievable vs. Speculative

### 8.1 Achievable (With Current hKask Architecture)

| Capability | How | CNS Evidence | Confidence |
|-----------|-----|-------------|-----------|
| CNS-observable heuristic evolution | `cns.qa.*` spans feed `bug-hunt-learn.j2` | `heuristics_added`, `heuristics_strengthened` in learner output | High |
| Recursive charter generation | WordAct template with H_t + F_{<t} as context | Charter span includes heuristic snapshot | High |
| Beizer-taxonomy classification | KnowAct template with taxonomy as structured prompt | Taxonomy span records classification | High |
| Contract-anchored oracle | TDD contracts as high-confidence oracle input | `ContractViolated` confirmation | High |
| OCAP-gated repair at ≥0.95 | `DelegationResource::Tool` with `Execute` + `Registry` with `Write` | `QaRepairAttempted`, `QaRepairVerified` | High |
| Bolero fuzz integration | Existing fuzz targets as probe generators | `QaBoleroFailure` | High (existing) |
| cargo-mutants integration | Type-system-guided mutant generation | `QaMutantSurvived` | High (existing) |
| LLM triage classification | `kask qa triage` with Gemma 4 26B | `QaRepairAttempted` confidence ≥0.95 | High (existing) |
| User-defined quality boundaries | P1 sovereignty: user specifies quality criteria | *(user-provided, not CNS-emitted)* | High |
| Composition with TDD, kata, red-team | FlowDef manifest with delegate steps | Kata PDCA spans | High |

### 8.2 Speculative (Research Questions)

| Question | Difficulty | Why |
|----------|-----------|-----|
| Can the skill generate genuinely novel heuristic categories? | Hard | LLMs recombine known patterns; true novelty requires recognizing a pattern not in training data |
| How long before heuristic evolution plateaus? | Unknown | Dependent on codebase complexity and LLM capability; needs empirical study |
| Can the skill detect bugs requiring domain expertise? | Medium | LLMs have broad but shallow knowledge; domain-specific bugs may be invisible |
| Does heuristic evolution converge or diverge? | Open | Should converge toward Pareto-efficient strategy; empirical validation needed |
| Can causal graphs be constructed from CNS span co-occurrence? | Hard | Spans are point observations, not causal edges; requires statistical inference |
| Cross-pod span correlation? | Hard | No cross-pod `CnsObserver` federation exists |

### 8.3 Honest Assessment

**What this model is:** A second-order cybernetic system that observes its own bug-hunting process and adapts its heuristics accordingly, producing its own testing competence from its findings. It addresses the pesticide paradox through continuous heuristic regeneration, not through more tests. It is architecturally grounded in hKask's existing infrastructure (verified CNS spans, OCAP delegation, Testing Discipline, LLM triage), unifies prior QA canon with second-order cybernetics, and makes the autopoietic claim **falsifiable by measurement**.

**What this model is not:** A replacement for human testers (P6), an autonomous bug-fixer without consent (P2), a guarantee of completeness (complexity barrier), a system that "understands quality" independently of the user (P1), or a claim of "AI consciousness."

**The autopoietic claim, restated precisely:** At the heuristic layer, the bug-hunting skill satisfies Maturana and Varela's criterion: it produces the components (heuristics, strategies, taxonomies, test-generation templates) that constitute its own testing competence, through a network of processes (charter→probe→oracle→taxonomize→learn) whose outputs recursively become inputs to the same network. The skill is operationally closed at the heuristic layer while structurally coupled to LLM inference and user-defined quality boundaries. The CNS is the un-debuggable substrate — the autopoietic skill can debug itself for all components except its own CNS emission.

---

## 9. Implications for Agent-Based QA

### 9.1 Beyond Test Automation: From Allopoiesis to Autopoiesis

The dominant QA paradigm is allopoietic: tools produce test results. An autopoietic testing skill shifts the paradigm from **producing test results** to **producing testing competence.** The skill gets better at finding bugs in a specific codebase over time, the way a human tester who specializes in that codebase gets better — by learning where the bugs hide.

### 9.2 The Pesticide Paradox, Partially Addressed

The pesticide paradox cannot be *solved* (it is structural), but it can be *addressed* through continuous heuristic regeneration. When a class of tests stops finding bugs, the autopoietic loop doesn't run more of those tests — it generates new test categories informed by what it has learned.

### 9.3 CNS Audibility

For the first time, the **learning trajectory** of a testing system is observable:

- What charters were generated and why
- What probes found what
- How the oracle evaluated each finding
- How the taxonomy classified it
- **How the heuristics changed as a result** (`cns.qa.*` delta)

This makes the testing process auditable (P8, P12) and the skill's competence falsifiable (P9). If CNS spans show no heuristic evolution, the system is allopoietic testing with autopoietic branding.

### 9.4 User Sovereignty in Quality Definition

Weinberg's definition — "quality is value to some person who matters" — is the **boundary condition** for the autopoietic loop. The user specifies what quality means. The skill hunts threats to that definition. The learning loop is bounded by the user's values (P1).

---

## 10. Falsifiability Statement

**This model is falsifiable.** It makes a concrete, measurable claim:

> If the autopoietic model is correct, `CnsHealth.overall_deficit` should **monotonically decrease** across bug-hunting sessions as the test corpus grows and the heuristic-generation function refines.

**What would falsify the model:**

1. A **flat** `CnsHealth.overall_deficit` across sessions (no learning, no refinement — the skill is not autopoietic).
2. A **decreasing** pass rate / **increasing** deficit (the loop degrades rather than improves).
3. An improving deficit that is explained entirely by **human-authored tests** rather than autonomously generated ones (the autopoietic claim is about autonomous refinement, not human effort).
4. **Zero heuristic change** in `bug-hunt-learn.j2` output across multiple cycles (the heuristic regeneration function is idempotent — no learning occurs).

**Measurement protocol:**

1. Record `CnsHealth.overall_deficit` at session boundaries.
2. Track `heuristics_added`, `heuristics_strengthened`, `heuristics_weakened` from learner output.
3. Compute the slope of `overall_deficit` across *N* sessions.
4. A slope < 0 (deficit decreasing = health improving) supports the model.
5. A slope ≥ 0 **falsifies** the model.

---

## 11. Open Questions and Future Work

### 11.1 Convergence Scope

Can the autopoietic loop be proven to converge for all decidable invariants, or only for those expressible in the hKask type system? The current proof holds for `proptest!`-expressible properties. Extending to temporal logic, cross-pod consistency, or session types requires either (a) embedding richer logics in the type system or (b) accepting probabilistic convergence via statistical model checking.

### 11.2 Adversarial Bug-Hunting Agents

Can a sufficiently sophisticated adversarial agent introduce a bug that passes all existing tests but creates a new invariant violation class that no existing test covers? This is the unknown-unknown problem — and it's precisely the gap the autopoietic loop aims to close over time, but cannot guarantee closure for. The defense is layered: attenuation, human gate, regression guard, CNS audit — but not provable immunity.

### 11.3 Novelty Detection

Can the skill recognize a bug pattern it has never seen before and add it to its taxonomy, or does it only recombine known LLM training patterns? This is an empirical question requiring controlled experiments with deliberately novel bug classes.

### 11.4 Cross-Codebase Transfer

Does heuristic learning on one codebase transfer to another, or is the learning codebase-specific? If not transferable, each codebase requires its own autopoietic warm-up period.

### 11.5 Human-in-the-Loop Integration

How should human testers interact with the autopoietic loop? As oracle validators (confirming/rejecting heuristic verdicts)? As charter directors (setting exploration priorities)? As taxonomy curators (refining the Beizer classification for the codebase)? The model currently assumes the user provides quality boundaries but doesn't specify the interaction protocol.

### 11.6 Formal Autopoiesis Proof

Can we formally prove that the heuristic layer satisfies Maturana and Varela's definition, or does the structural coupling with LLM inference prevent formal closure? If formal proof is impossible, what is the strongest claim that can be verified?

---

## 12. Conclusion

We have introduced a unified model of autopoietic bug hunting in the hKask agent system. The model:

1. **Synthesizes two convergent analyses:** A QA-canon-grounded analysis (Weinberg, Beizer, Kaner, Bach/Bolton, Hendrickson, Agans, Zeller) framed through second-order cybernetics (Maturana/Varela, von Foerster, Luhmann), and a codebase-grounded analysis (verified CNS spans, OCAP delegation, property-based testing, bolero fuzzing, cargo-mutants, LLM triage).

2. **Decomposes bug hunting into five primitives** — detection, localization, diagnosis, repair, prevention — each mapped to hKask infrastructure with CNS span witnesses.

3. **Formalizes the autopoietic claim:** *H_t = f(F_{<t}, H_{<t})* — the heuristic base at time *t* is a function of prior findings and prior heuristics, not a static constant.

4. **Proves convergence** for decidable invariants within hKask's type system, with explicit acknowledgment of undecidable limits.

5. **Is falsifiable:** `CnsHealth.overall_deficit` slope across sessions. Negative slope supports the model; non-negative falsifies it. CNS spans make the heuristic evolution observable and auditable.

6. **Establishes the bullshit boundary:** Autopoiesis is scoped to the heuristic layer (heuristics, taxonomies, strategies regenerate from findings). The LLM inference layer is structurally coupled, not autopoietic. The CNS is the un-debuggable substrate — the fixed point that stops the infinite regress of self-observation.

7. **Composes with existing hKask infrastructure** — TDD (contract oracles), adversarial-red-team (exploration probes), kata (PDCA cycles), QA script builder (initial charters), condenser (density-based prioritization) — and acknowledges the exploitation/exploration gap that adversarial-red-team addresses.

8. **Respects all twelve hKask principles** — P1 (user-defines quality), P2 (affirmative consent for fixes), P3 (generative space, no hidden control plane), P4 (OCAP-gated access), P5 (essentialism, ≤7 templates), P6 (augments, doesn't replace human testers), P7 (heuristics emerge from findings), P8 (semantic grounding in typed CNS spans), P9 (homeostatic self-regulation via CnsHealth), P12 (every action carries replicant identity).

---

## References

1. Agans, D. J. (2002). *Debugging: The Nine Indispensable Rules for Finding Even the Most Elusive Software and Hardware Problems*. AMACOM.

2. Bach, J. (2015). Heuristic Test Strategy Model. Satisfice, Inc.

3. Bach, J., & Bolton, M. (2025). *Taking Testing Seriously: The Rapid Software Testing Approach*. Wiley.

4. Beizer, B. (1990). *Software Testing Techniques* (2nd ed.). Van Nostrand Reinhold.

5. Beizer, B. (1995). *Black-Box Testing: Techniques for Functional Testing of Software and Systems*. Wiley.

6. Bolton, M. (2012). Oracles from the Inside Out. DevelopSense.

7. Claessen, K., & Hughes, J. (2000). QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs. *ACM SIGPLAN Notices*, 35(9), 268–279.

8. Fang, R., Bindu, R., Gupta, A., & Kang, D. (2024). LLM Agents Can Autonomously Exploit One-Day Vulnerabilities. *arXiv preprint arXiv:2404.08144*.

9. von Foerster, H. (1974). *Cybernetics of Cybernetics*. Biological Computer Laboratory, University of Illinois.

10. von Foerster, H. (1981). *Observing Systems*. Intersystems Publications.

11. von Foerster, H. (1991). Ethics and Second-Order Cybernetics. *Cybernetics & Human Knowing*, 1(1).

12. Hendrickson, E. (2013). *Explore It!: Reduce Risk and Increase Confidence with Exploratory Testing*. Pragmatic Bookshelf.

13. hKask Architecture Master, v0.30.0. `docs/architecture/hKask-architecture-master.md`.

14. hKask Principles, v0.30.0. `docs/architecture/core/PRINCIPLES.md`.

15. hKask Testing Discipline, v0.29.0. `docs/architecture/core/TESTING_DISCIPLINE.md`.

16. hKask CNS Span Registry, v0.30.0. `crates/hkask-types/src/cns.rs`.

17. hKask Capability, v0.30.0. `crates/hkask-capability/src/lib.rs`, `resources.rs`.

18. Kaner, C., Bach, J., & Pettichord, B. (2001). *Lessons Learned in Software Testing: A Context-Driven Approach*. Wiley.

19. Luhmann, N. (1984). *Soziale Systeme: Grundriß einer allgemeinen Theorie*. Suhrkamp. (English: *Social Systems*, Stanford University Press, 1995).

20. Maturana, H. R., & Varela, F. J. (1980). *Autopoiesis and Cognition: The Realization of the Living*. D. Reidel Publishing.

21. Maturana, H. R., & Varela, F. J. (1987). *The Tree of Knowledge: The Biological Roots of Human Understanding*. Shambhala.

22. Mingers, J. (1995). *Self-Producing Systems: Implications and Applications of Autopoiesis*. Plenum Press.

23. Myers, G. J. (1979). *The Art of Software Testing*. Wiley.

24. Ousterhout, J. (2018). *A Philosophy of Software Design*. Yaknyam Press.

25. Weinberg, G. M. (1992). *Quality Software Management, Volume 1: Systems Thinking*. Dorset House.

26. Weinberg, G. M. (2008). *Perfect Software and Other Illusions About Testing*. Dorset House.

27. Whittaker, J. A. (2003). *How to Break Software: A Practical Guide to Testing*. Addison-Wesley.

28. Zeller, A. (2009). *Why Programs Fail: A Guide to Systematic Debugging* (2nd ed.). Morgan Kaufmann.

---

## Appendix A: P1–P12 Constraint Mapping

| Principle | Constraint on Bug-Hunting Skill | Enablement |
|-----------|-------------------------------|------------|
| **P1 — User Sovereignty** | User defines quality values; skill cannot override | User-specified quality criteria anchor the oracle |
| **P2 — Affirmative Consent** | Bug fixes require explicit consent; no autonomous merge below 0.95 | Findings are proposals, not actions; `QaRepairExhausted` escalates |
| **P3 — Generative Space** | No hidden control plane; all heuristics user-visible via CNS spans | CNS spans expose the heuristic model |
| **P4 — Clear Boundaries (OCAP)** | Tool access scoped by delegation tokens; `SYSTEM_MAX_ATTENUATION` enforced | Skill can only probe what it has capability to access |
| **P5 — Essentialism** | Skill must earn existence; ≤7 public templates | Enforces lean design |
| **P6 — Space for Replicants** | Skill augments, doesn't replace human testers | Replicant attribution on all actions |
| **P7 — Evolutionary Architecture** | Heuristics emerge from findings, not speculation | The learn loop implements evolutionary emergence |
| **P8 — Semantic Grounding** | Every finding traceable to typed CNS span evidence | `CnsSpan` enum variants, not stringly-typed |
| **P9 — Homeostatic Self-Regulation** | CNS spans required for all loop operations | `CnsHealth` provides convergence signal |
| **P12 — Replicant Host Mandate** | Every action carries an owner WebID | All findings attributed to replicant |

## Appendix B: Skill Lens Rotation

This unified paper was developed through a structured rotation through five hKask analytical skills:

| Skill | Contribution |
|-------|-------------|
| **Essentialist** | Gate-tested the concept in both analyses: survives deletion test, ≤7 templates, not a pass-through abstraction; merged 5 primitives into 3 in the other analysis |
| **Grill-Me** | Interrogated the autopoietic claim: "adaptive vs. autopoietic?", "where does it break down?", "infinite regress of self-observation?" |
| **Caveman** | Compressed to minimal signal: "Bug hunting is learning, not production. Tests rot. Heuristics must regenerate. Autopoietic if CNS-observable. Not bullshit if falsifiable." |
| **Chain-of-Density** | Iteratively densified the abstract to convergence at entity density ~0.22, preserving all canonical references |
| **Falstaffian Perspective** | Generated four semantic rotations: Predicate Hollow (empties "bug" of objective substance), Subject Expansion (one finding trains all patterns), Counterfeit Inversion (passing tests ≠ quality), Direction Reversal (bugs should find a better system) |
| **Coding Guidelines** | Enforced: thinking before writing (assumptions stated), simplicity first (7-template limit), surgical changes (composites with existing skills), goal-driven execution (CNS-falsifiable autopoietic claim) |
