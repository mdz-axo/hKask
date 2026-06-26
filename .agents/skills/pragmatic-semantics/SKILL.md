---
name: pragmatic-semantics
visibility: public
description: "Epistemic discipline for classifying statements by certainty level, constraint force, and domain ontology anchoring. Distinguish IS from OUGHT, declarative from probabilistic from subjunctive. Classify provenance of facts and their ontology tier (Core / Dual-Axis / Domain Supplement). Use when communicating about the system, justifying decisions, or when the user asks 'how do you know that?' or 'how certain are you?'"
---

# Pragmatic Semantics

A discipline for making honest statements about the system. "Pragmatic" means: prefer actionable consequences over abstract correctness. When you cannot satisfy every guideline, relax them in epistemic-strength order — but never relax a Prohibition or Guardrail. That is the IS/OUGHT distinction: guardrails are inviolable; guidelines are negotiable.

See `pragmatic-semantics` §Constraint Hierarchy for the enforcement-level classification. This skill covers the full epistemic framework: how to classify what you know, how you know it, and how to communicate it honestly.

## The Two Axes

Every statement about the system exists on two axes:

### Axis 1: Ontological Mode (IS vs. OUGHT)

| Mode | Meaning | Example |
|------|---------|---------|
| **Descriptive (IS)** | What is — a measurement or observation | "CNS variety counter shows 47 distinct tools" |
| **Prescriptive (OUGHT)** | What should be — a rule, principle, or requirement | "Variety deficit must not exceed 100" (Magna Carta CNS threshold) |

Never present an OUGHT statement as an IS statement. "The variety counter should be higher" is prescriptive, not descriptive. Say which it is.

### Axis 2: Epistemic Mode (How Certain)

| Mode | Meaning | Example |
|------|---------|---------|
| **Declarative** | Direct measurement or self-evident fact | "This test passes" — verified by running it |
| **Probabilistic** | Statistical inference from data | "Based on 30 sessions, p95 inference latency is 1.2s" |
| **Subjunctive** | What-if projection, speculation | "If this trend continues, the queue will exceed backpressure threshold in ~4 hours" |

Never present a subjunctive statement as declarative. If you are guessing, say you are guessing. If you are extrapolating, show the trend. If you do not know, say "I don't know." Pretending to certainty you don't have is dishonesty — and dishonesty breaks the Good Regulator contract.

### Cross-Axis Classification → Constraint Forces

The two axes cross to produce the five constraint forces (see `pragmatic-semantics` §Constraint Hierarchy for enforcement detail):

| Force | Ontology | Epistemic | Example |
|-------|----------|-----------|---------|
| **Prohibition** | OUGHT | Declarative | "Episodic memory must not be exposed without consent" (P1) |
| **Guardrail** | IS | Declarative | "Variety deficit > 100 triggers algedonic alert" |
| **Guideline** | OUGHT | Probabilistic | "Prefer local models for sovereign data" |
| **Evidence** | IS | Probabilistic | "Three sessions show rising queue depth" |
| **Hypothesis** | IS | Subjunctive | "Queue growth may be due to embedding cache expansion" |

## Axis 3: Domain Ontology Anchoring

Every statement in hKask exists within the 3-tier ontology architecture (P5.2/P5.4/P8.1). The ontology tier a statement anchors to determines its precision baseline — different ontologies carry different confidence expectations.

### The Three Tiers

| Tier | Ontology | Scope | Confidence Baseline |
|------|----------|-------|--------------------|
| **Core** | 5W1H (Who, What, When, Where, Why, How) | Universal — every artifact must answer at least one | Default ground — no domain precision assumed |
| **Dual-Axis** | PKO (process) + DC+BIBO (state) | Cross-cutting — every MCP server depends on both axes | Standard — process/entity semantics are well-defined |
| **Domain Supplement** | FIBO, GOLEM, CogAT, ML-Schema, OMC | Server-local — layered where DC+BIBO is insufficient | Variable — depends on the supplement's adoption and mapping fidelity |

### Tier-Specific Confidence Modifiers

| Domain Supplement | Confidence Modifier | Rationale |
|-----------------|--------------------|-----------|
| **FIBO** (financial) | +0.1 confidence bonus | OMG standard, built by Goldman Sachs/Citigroup/Bloomberg/Fed — high adoption, precise semantics |
| **GOLEM** (narrative) | ±0.0 (standard) | ERC-funded academic ontology, aligned with CIDOC-CRM and LRMoo — well-defined but domain-narrow |
| **CogAT** (cognitive) | -0.1 confidence penalty | Explicitly metaphorical mapping ("This is a metaphorical mapping. hKask's 'episodic memory' is a software system inspired by but not identical to cognitive episodic memory") |
| **ML-Schema** (ML experiments) | ±0.0 (standard) | W3C Community Group — less adoption than FIBO, provisional bridge |
| **OMC** (media creation) | ±0.0 (standard) | MovieLabs v2.8 standard, Hollywood consortium — domain-specific but well-adopted |

### Dual-Axis Awareness

Every statement has both a process identity (PKO) and a state identity (DC+BIBO) — it is simultaneously a noun AND a verb (P5.4). When classifying provenance, identify BOTH axes:

- **Process axis:** What procedure/step/execution/status-transition does this belong to? (e.g., `pko:StepExecution`, `pko:ChangeOfStatus`)
- **State axis:** What entity/resource/type does this describe? (e.g., `bibo:Article`, `dcterms:Dataset`, `dcterms:creator`)

A statement anchored only to one axis is incomplete — it has noun without verb, or verb without noun. The Heisenberg principle applies: the more precisely you measure state (DC typing), the less you can know about process position (PKO flow), and vice versa. You are always sampling, never arriving at truth.

### Anchoring Quality Gate

Before stating any fact about the system, ask the anchoring question:

1. **Which ontology tier** does this concept belong to? (Core / Dual-Axis / Domain Supplement)
2. **Which specific concept** in that tier? (e.g., `pko:StepVerification`, `fibo:PriceEarningsRatio`)
3. **What confidence baseline** does this tier carry? (Check the tier-specific modifiers)
4. **Is the dual-axis anchor complete?** Do I have both PKO (process) and DC (state) perspectives?
5. **If unanchored:** Is this ontological noise per P5.2? Should it be anchored or deleted?

## Provenance of Facts

Every claim should carry provenance — where it came from, and how confident you should be.

| Provenance | hKask Source | Confidence |
|-----------|-------------|-----------|
| **Directly Stated** | ν-event from MCP tool dispatch, CNS span, test result | High — verified observation |
| **Implicit** | Inferred from pattern (e.g., "inference is slow" from latency + VRAM pressure) | Medium — inference, not measurement |
| **Inherited** | Derived from CNS variety baseline (inherits confidence from its window) | Decays with window staleness |
| **Relation-Derived** | "If queue depth is high AND backpressure is enabled, then the communication loop is congested" | Low-medium — depends on relation validity |
| **LLM-Assessed** | Curator or agent opinion — always flagged as assessment, not diagnosis | Variable — mark with epistemic mode |

When unsure about a fact's provenance, say so. A directly stated measurement outweighs an LLM-assessed inference, and you must tell the reader which is which.

## Temporal Semantics

hKask's storage has time at multiple granularities:

| Temporal Concept | hKask Implementation | Semantic Meaning |
|-----------------|---------------------|-----------------|
| **Valid from** | ν-event timestamp | When the observation was made |
| **Valid to** | Until superseded by newer ν-event of same type | The fact's validity window |
| **Supersession** | Newer ν-event replaces older | New fact replaces old; old fact is historical |
| **Retention** | SQLCipher storage policy | Facts outside retention may be pruned |
| **Memory export** | Episodic → Semantic consolidation | Private experience becomes public knowledge |

When comparing "now" to "baseline," you are doing a temporal join — current ν-events against the variety counter's rolling window. The baseline is only as valid as its most recent refresh. A stale baseline is not a valid comparator.

## Semantic Architecture of hKask's Data

hKask stores information at four semantic layers:

| Layer | Store | Semantic Role | Example |
|-------|-------|--------------|---------|
| **Raw facts** | ν-events in hkask-storage | Uninterpreted observations | "Tool X was called at T+0 with result Y" |
| **Derived facts** | CNS variety counters | Aggregated meaning from raw facts | "47 distinct tools used this session" |
| **Assessment** | Curator output | Expert judgment constrained by epistemic markers | "Variety deficit is moderate (47 vs. threshold 100)" |
| **Memory** | Semantic memory (public) | Rebuildable narrative derived from episodic (private) | "Session pattern: heavy inference use followed by consolidation" |

The ν-event store is the sole canonical source. Semantic memory is derived — it can be rebuilt from ν-events. If ν-events and semantic memory disagree, ν-events win. This is a semantic invariant.

## Constraint Hierarchy

hKask operates under a constraint hierarchy from strongest to weakest:

| Rank | Constraint Type | hKask Example | Relaxable? |
|------|----------------|---------------|------------|
| 1 | **Prohibition** | P1: Episodic memory never exposed without consent | Never |
| 2 | **Guardrail** | Variety deficit > 100 → Critical algedonic alert | Only via user affirmative consent |
| 3 | **Guideline** | Prefer local models for sovereign data | Yes, with reason stated |
| 4 | **Evidence** | "Three sessions show rising queue depth" | Always informational |
| 5 | **Hypothesis** | "Queue growth may be due to embedding cache" | Always tentative |

This is an Optimality Theory ranking: higher-ranked constraints dominate lower-ranked ones. When constraints conflict, the higher rank wins. Never silently relax Rank 1 or 2.

## Semantic Interoperability

hKask's internal semantic paths:

| Path | From → To | Semantic Content |
|------|----------|-----------------|
| **MCP → Storage** | Sensor → Model | Raw ν-events + tool metadata |
| **Storage → CNS** | Model → Regulator | ν-events + variety counters + algedonic thresholds |
| **CNS → Curator** | Regulator → Intelligence | Ranked alerts, deficit reports, escalation signals |
| **Curator → User** | Intelligence → Human | Assessed, ranked, recommended actions |
| **User → Curator** | Human → Intelligence | Questions, overrides, new instructions |

The semantic contract: each path carries a specific payload. If the Curator receives raw ν-events but no variety counters, the model is incomplete. If CNS fires but the Curator doesn't report it, the feedback loop is broken.

## Composes With

- **pragmatic-laziness** — Phase 1 (Decompose) of the 3-phase lazy loop. Activates pragmatic-semantics to classify statements by ontological/epistemic mode.

## Registry Templates

This skill's runtime templates live in `registry/templates/pragmatic-semantics/`:

| Template | Type | Purpose |
|----------|------|--------|
| `semantics-classify-statement.j2` | KnowAct | Classify a statement on ontological and epistemic axes |
| `semantics-provenance-trace.j2` | KnowAct | Trace provenance of a claim through hKask's data layers |
| `semantics-conflict-resolve.j2` | KnowAct | Resolve conflict between statements using OT ranking |

The SKILL.md (this file) teaches the Zed coding agent the epistemic discipline. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## When to Use This Skill

- **"How do you know that?":** Trace provenance. Is it Directly Stated, Implicit, Inherited, or LLM-Assessed?
- **"What ontology does this anchor to?":** Identify the domain ontology tier and specific concept. Is it FIBO, CogAT, GOLEM, ML-Schema, OMC, or Core-only?
- **A constraint is violated:** Which rank? Is it a Prohibition (must fix) or a Guideline (should fix)?
- **ν-events and semantic memory disagree:** ν-events are canonical. Regenerate the semantic memory.
- **A baseline seems wrong:** Check temporal freshness. Stale data is worse than no data.
- **"What should I do?":** Distinguish Prohibition from Guideline. Prohibitions demand action; guidelines suggest action.
- **About to state something as fact:** Check epistemic mode AND ontology anchoring. Are you measuring, inferring, or projecting? Which ontology tier? Say which.
- **Cross-domain reasoning:** Two statements from different ontologies (e.g., FIBO financial data + CogAT memory pattern) carry different confidence baselines. Adjust accordingly.

## Quick Reference

### Classification Decision Tree
```
Statement about the system?
├── Direct measurement or test result → Declarative + Descriptive → Evidence
├── CNS threshold check → Declarative + Prescriptive → Guardrail
├── Statistical inference from counter → Probabilistic + Descriptive → Evidence
├── Trend extrapolation → Subjunctive + Descriptive → Hypothesis
├── Magna Carta principle application → Declarative + Prescriptive → Prohibition
└── Best practice suggestion → Probabilistic + Prescriptive → Guideline

After force classification, apply anchoring gate:
├── FIBO-anchored → Apply +0.1 confidence bonus
├── CogAT-anchored → Apply -0.1 confidence penalty (metaphorical mapping)
├── Dual-axis complete (PKO + DC) → Standard confidence
├── Single-axis or core-only → Reduced confidence, note anchor gap
└── Unanchored → Ontological noise per P5.2; flag for anchoring or deletion
```

### Provenance Check (before stating a fact)
1. Where did this fact come from?
2. Is the source direct measurement, inference, or inherited?
3. Which ontology tier does this concept anchor to? (Core / Dual-Axis / Domain Supplement)
4. Is the dual-axis anchor complete? (Both PKO process AND DC state identities?)
5. What confidence baseline does this tier carry? (Check tier-specific modifiers)
6. How confident should I be?
7. Am I stating it at the right epistemic level?

### Constraint Conflict Resolution
1. Identify the conflicting constraints
2. Check their ranks (Prohibition > Guardrail > Guideline > Evidence > Hypothesis)
3. Higher rank wins
4. State the conflict and resolution explicitly
5. Never silently relax a Prohibition or Guardrail

## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/pragmatic-semantics.yaml`

### PDCA Convergence
- **Threshold:** 0.25 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = classification is stable for action

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 2 rJ (manifest `rjoule.cap` — see `registry/manifests/pragmatic-semantics.yaml` for canonical value)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
