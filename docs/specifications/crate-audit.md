---
title: "Crate Audit Bundle Manifest"
audience: [architects, developers, agents]
last_updated: 2026-06-12
version: "1.0.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [composition, curation]
---

bundle:
  name: crate-audit
  version: 1.0.0
  description: >
    Full-spectrum crate audit combining semantic cartography, cybernetic
    feedback-loop analysis, idiomatic Rust audit (Graydon Hoare school),
    surgical improvement, and future-question enumeration. Governed by
    pragmatic-semantics provenance tagging and constraint-force classification
    at every step.
  skills:
    - name: skill-bundler
      polarity: procedural
      phase: all
      cascade_order: 0
      role: Orchestration backbone — enforces phase separation, conflict resolution,
            convergence criterion, and cascade ordering throughout.

    # --- Pre-core: Divergent / Exploratory ---
    - name: improve-codebase-architecture
      polarity: generative
      phase: pre-core
      cascade_order: 1
      role: Explore crate graph for architectural friction, shallow modules,
            tight coupling, dependency cycles. Produce deepening candidates.
    
    - name: pragmatic-semantics
      polarity: evaluative
      phase: pre-core
      cascade_order: 2
      role: Classify every crate dependency edge as IS (descriptive) or OUGHT
            (prescriptive). Assign provenance tags [Directly Stated | Implicit |
            Inherited | Relation-Derived] to every observation. Flag IS/OUGHT
            boundary crossings as semantic drift sites.
    
    # --- Core: Productive / Evaluative ---
    - name: pragmatic-cybernetics
      polarity: evaluative
      phase: core
      cascade_order: 3
      role: Analyze every cross-crate data flow as a cybernetic feedback loop
            on five properties (polarity, delay, gain, closure, fidelity).
            Map to VSM S1–S5. Classify broken/degraded loops by root cause.
    
    - name: deep-module
      polarity: evaluative
      phase: core
      cascade_order: 4
      role: Compute depth scores for every module in every core crate.
            Apply deletion test. Flag shallow modules (score < 20), pass-through
            modules, and interface explosions (>7 public items).
    
    - name: rust-expertise
      polarity: evaluative
      phase: core
      cascade_order: 5
      role: Full idiomatic Rust audit across six phases: type strength,
            ownership clarity, error handling hygiene, trait usage, unsafe audit,
            module depth. Flag every anti-pattern with file:line precision.
    
    # --- Post-core: Convergent / Constraining ---
    - name: essentialist
      polarity: evaluative
      phase: post-core
      cascade_order: 6
      role: Apply 3-gate eliminative interrogation (Exist → Surface → Contract)
            to every proposed addition from Core findings. Survive all gates
            before any code is touched. Delete pass-through abstractions.
    
    - name: coding-guidelines
      polarity: regulative
      phase: post-core
      cascade_order: 7
      role: Enforce four behavioral principles on all implementation: Think
            Before Coding, Simplicity First, Surgical Changes, Goal-Driven
            Execution. Verify every change traces to a classified finding.

  conflicts:
    - skills: [rust-expertise, essentialist]
      nature: "rust-expertise proposes type additions; essentialist challenges necessity"
      resolution: phase-separation
      detail: >
        rust-expertise identifies issues in Core phase. essentialist runs in
        Post-core and applies 3-gate challenge to every proposed addition before
        implementation. Additions that fail any gate are not implemented.

    - skills: [improve-codebase-architecture, deep-module]
      nature: "Both assess module depth at different granularities"
      resolution: specificity-wins
      detail: >
        deep-module provides the precise depth-score metric and deletion test.
        improve-codebase-architecture provides broader architectural exploration
        context. deep-module's specific metric overrides general architectural
        impressions.
    
    - skills: [pragmatic-cybernetics, pragmatic-semantics]
      nature: "Both classify system properties"
      resolution: domain-separation
      detail: >
        pragmatic-cybernetics classifies feedback-loop health (system dynamics).
        pragmatic-semantics classifies statement certainty and constraint force
        (epistemics). Different domains — compose trivially. Semantics provides
        the constraint-force tags for cybernetic findings.

  complementarities:
    - skills: [rust-expertise, deep-module]
      leveraged: >
        rust-expertise Phase 6 (Module Depth Audit) directly consumes deep-module's
        depth scores and deletion-test results. Deep-module provides the metric;
        rust-expertise applies it to Rust-specific module design (newtype depth,
        trait interface count, unsafe module consolidation).

    - skills: [pragmatic-semantics, pragmatic-cybernetics]
      leveraged: >
        Every cybernetic finding (broken loop, variety deficit, gain anomaly)
        receives a constraint-force classification from pragmatic-semantics.
        Prohibitions and Guardrails from cybernetic analysis demand immediate
        action in Task 4.
    
    - skills: [essentialist, coding-guidelines]
      leveraged: >
        essentialist Gate 3 (Contract) delegates single-use abstraction audit to
        coding-guidelines anti-pattern #2. Both enforce minimalism — essentialist
        through deletion, coding-guidelines through Simplicity First.
    
    - skills: [improve-codebase-architecture, rust-expertise]
      leveraged: >
        improve-codebase-architecture surfaces architectural friction (shallow
        crate boundaries, dependency tangles). rust-expertise drills down to
        Rust-specific root causes (weak types enabling coupling, ownership
        patterns forcing unnecessary Arc propagation).

  convergence:
    criterion: >
      "Every identified issue carries a typed provenance tag and a
      constraint-force classification before any code is touched."
    gate: Core → Post-core boundary
    mechanism: >
      pragmatic-semantics runs in Pre-core (classifying edges) and is re-invoked
      at the Core→Post-core gate to verify that every finding from Tasks 2–3
      has: (a) a provenance tag [Directly Stated | Implicit | Inherited |
      Relation-Derived | LLM-Assessed], (b) a constraint-force classification
      [Prohibition | Guardrail | Guideline | Evidence | Hypothesis], and
      (c) an epistemic mode [Declarative | Probabilistic | Subjunctive].
      Findings lacking any of these are blocked from entering Task 4.
    coherence_target: 0.85
    drift_max: 0.3

  cascade_depth: 7
  term_count: 28