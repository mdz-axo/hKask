---
title: "Bug Hunting Skill — Implementation Plan"
version: "0.1.0"
date: 2026-06-21
status: "Draft"
anchored_to: "docs/research/bug-hunting-as-autopoietic-skill-unified.md"
principles: [P1, P2, P3, P4, P5, P8, P9, P12]
---

# Bug Hunting Skill — Implementation Plan

**Anchored to:** [Autopoietic Bug Hunting: A Unified Model](docs/research/bug-hunting-as-autopoietic-skill-unified.md)  
**Governing principle:** P5 (Essentialism) — ≤7 public templates, every artifact earns existence  
**Falsifiability criterion:** `CnsHealth.overall_deficit` must monotonically decrease across sessions


## 0. Assumptions & Risks (Think Before Coding)

### 0.1 Explicit assumptions

1. **The LLM triage pipeline (`kask qa triage`, Gemma 4 26B) is operational.** The skill depends on it for oracle classification and repair proposals. If the triage pipeline is unavailable, the skill degrades to detection + localization only.

2. **The `CnsSpan` enum can be extended.** Adding `BugHunt*` variants requires modifying `crates/hkask-types/src/cns.rs` — a surgical change to a foundational type. This is the highest-risk change in the plan.

3. **CNS span persistence is functional.** The `NuEventSink` must be able to persist `cns.bughunt.*` spans for the `cns_health_check` function to compute `CnsHealth.overall_deficit` from bug-hunting activity.

4. **Existing QA spans are sufficient for bootstrapping.** `QaBoleroFailure`, `QaRepairAttempted`, `QaRepairVerified`, `QaRepairExhausted`, `QaMutantSurvived`, `ContractViolated`, `CiInvariantViolation` already exist and will serve as the initial witness surface. BugHunt-specific spans are additive, not prerequisite.

5. **The skill operates under user sovereignty (P1).** The user defines quality criteria. The skill hunts threats to those criteria. The skill never defines quality independently.

6. **The skill requires affirmative consent for fixes (P2).** Autonomous repair proposals at ≥0.95 confidence are made as git branches; human merges them. Below 0.95, the skill reports findings without attempting repair.

7. **P1–P12 are *constraints*, not aspirational.** The plan documents how each principle constrains each implementation step. Any step that violates a principle must be redesigned.

### 0.2 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| `CnsSpan` enum extension breaks exhaustive match arms | High | Compile errors in all CNS consumers | Use compiler-driven refactoring; add `BugHunt*` variants in a single commit with all match arms updated |
| LLM heuristic regeneration produces low-quality charters | Medium | Skill finds no novel bugs, falsifies autopoietic claim early | Accept as empirical result; this IS the falsifiability mechanism |
| Cross-skill composition creates circular dependencies | Low | Deadlock in FlowDef execution | Skills compose via delegate, not import; each skill is independently executable |
| Skill grows beyond 7 templates | Medium | P5 violation | Freeze at 7; merge or justify any additional |

### 0.3 Simpler alternatives considered

| Alternative | Why rejected |
|-------------|-------------|
| Extend `diagnose` skill instead of new skill | `diagnose` addresses single bugs reactively; bug-hunt is proactive, exploratory, and autopoietic. Different functional role. |
| Extend `qa-script-builder` instead of new skill | QA builder produces static manifests; bug-hunt produces evolving heuristics. Different lifecycle. |
| Implement as Rust library code, not a skill | P3 (Generative Space): selection intelligence lives in Jinja2/LLM, not Rust. Skills are the correct abstraction. |
| Skip CNS span extension, use stringly-typed spans only | P8 violation: CNS spans must be typed. Stringly-typed is acceptable for performative spans but not for correctness-sensitive semantic ground. |


## 1. Implementation Phases

### Phase 1: CNS Span Extension (Foundation)

**Why first:** The skill's registry crate references CNS span types. Spans must exist before templates reference them.

#### Step 1.1 — Add `BugHunt*` variants to `CnsSpan` enum

**File:** `crates/hkask-types/src/cns.rs`  
**Principle:** P8 (Semantic Grounding) — typed, not stringly-typed  
**Constraint:** P5 (Essentialism) — add only spans with clear semantic role

Add these variants to `pub enum CnsSpan`:

```rust
/// Bug hunting charter generated (WordAct output).
BugHuntCharter,
/// Bug hunting probe executed — what was explored, what was found.
BugHuntProbe,
/// Bug hunting oracle evaluation — "is this behavior a bug?"
BugHuntOracle,
/// Bug hunting taxonomy classification — Beizer category, severity, confidence.
BugHuntTaxonomize,
/// Bug hunting heuristic model updated — delta between H_t and H_{t+1}.
/// THIS IS THE AUTOPOIETIC MARKER. If this span consistently shows
/// zero heuristic change, the autopoietic claim is falsified.
BugHuntLearn,
/// Bug hunting report generated (WordAct output).
BugHuntReport,
```

**REQ: P8-BUGHUNT-001** — pre: `CnsSpan` enum exists with match arms in `as_str`, `from_str`, and test; post: six `BugHunt*` variants added with canonical namespace strings `cns.bughunt.{charter,probe,oracle,taxonomize,learn,report}`

**Verification:**
- `cargo build --workspace` passes (all match arms updated)
- `cargo test -p hkask-types` — `cnsspan_exhaustive_match_covers_all_canonical` test updated and passes
- `grep -r "BugHunt" crates/` shows variants only in `cns.rs` and consumers (no orphan references)

#### Step 1.2 — Add canonical namespace strings

In `impl CnsSpan { pub fn as_str }`, add:

```rust
CnsSpan::BugHuntCharter => "cns.bughunt.charter",
CnsSpan::BugHuntProbe => "cns.bughunt.probe",
CnsSpan::BugHuntOracle => "cns.bughunt.oracle",
CnsSpan::BugHuntTaxonomize => "cns.bughunt.taxonomize",
CnsSpan::BugHuntLearn => "cns.bughunt.learn",
CnsSpan::BugHuntReport => "cns.bughunt.report",
```

In `impl FromStr for CnsSpan`, add the reverse mappings.

**Verification:**
- `CnsSpan::BugHuntLearn.as_str() == "cns.bughunt.learn"`
- `"cns.bughunt.learn".parse::<CnsSpan>() == Ok(CnsSpan::BugHuntLearn)`

#### Step 1.3 — Update exhaustive match arms

Every `match` on `CnsSpan` in the codebase must be updated. This is compiler-enforced — the Rust compiler will flag every non-exhaustive match. Use `cargo check --workspace 2>&1 | grep "non-exhaustive"` to enumerate all sites, then add `BugHunt*` arms with `unreachable!()` for sites that should never receive bug-hunt spans (e.g., `WalletBalance` handler).

**Verification:**
- `cargo check --workspace` passes with zero warnings
- `cargo clippy --workspace` passes

---

### Phase 2: Registry Crate (The Skill)

**Why second:** Templates are the canonical source of truth (P5.1). Everything else serves the skill; the skill does not serve infrastructure.

**Constraint:** ≤7 public templates (P5 + deep-module discipline). If additional functionality is needed, it MUST be merged into existing templates or explicitly justified in a P5 exception document.

#### Step 2.1 — Create crate directory and manifest.yaml

**Directory:** `registry/templates/bug-hunt/`  
**File:** `manifest.yaml`  
**Pattern:** Follow `registry/templates/tdd/manifest.yaml` and `registry/templates/qa-script-builder/manifest.yaml`

##### `manifest.yaml`

```yaml
# Template crate manifest for bug-hunt skill
# hKask v0.30.0

crate:
  name: bug-hunt
  version: "0.30.0"
  description: >
    Autopoietic bug hunting: generates charters, probes code for threats to
    user-defined quality, classifies findings into Beizer taxonomy, updates
    heuristic model from findings, and produces structured bug reports.
    The recursive charter→probe→oracle→taxonomize→learn loop is observable
    via CNS spans (cns.bughunt.*) and converges when CnsHealth.healthy is true.
    Falsifiable: CnsHealth.overall_deficit must monotonically decrease across
    sessions for the autopoietic claim to hold.
  contract_energy_budget:
    description: >
      Calibrated inference cost for autopoietic loop phases.
      Charter generation and heuristic learning are the most expensive
      KnowAct operations. Probe execution delegates to tools (lower cost).
    baseline_energy_cap: 8192
    per_check_costs:
      charter_generation: 2048
      oracle_evaluation: 1024
      taxonomy_classification: 1024
      heuristic_update: 2048
      report_generation: 1024

templates:
  - id: bug-hunt/bug-hunt-charter
    path: bug-hunt-charter.j2
    type: WordAct
    lexicon_terms: [charter, explore, hunt, scope, survey, target, mission]
    description: >
      Generate a testing charter from current heuristic model (H_t) and prior
      findings (F_{<t}). A charter is a mission statement: "Explore [target]
      using [strategy] to discover [quality threat], given what we now know."
      Incorporates Hendrickson-style touring patterns and Bach's Heuristic
      Test Strategy Model. Bounded by user-specified quality criteria (P1).
      Output: charter document with scope, strategy, oracle criteria, and
      expected finding categories.
    generates_spans:
      - cns.bughunt.charter

  - id: bug-hunt/bug-hunt-probe
    path: bug-hunt-probe.j2
    type: FlowDef
    lexicon_terms: [probe, execute, explore, instrument, invoke, test, fuzz]
    description: >
      Execute a testing expedition defined by the charter. Delegates to
      available tools: file reads, code search, terminal commands, prop-test
      generators, bolero fuzz targets, cargo-mutants, and adversarial-red-team
      attack patterns. Records observations, logs, and crash traces.
      Each probe emits a cns.bughunt.probe span.
      OCAP-gated: requires Tool:test:Execute and Tool:cns:Read tokens.
    generates_spans:
      - cns.bughunt.probe

  - id: bug-hunt/bug-hunt-oracle
    path: bug-hunt-oracle.j2
    type: KnowAct
    lexicon_terms: [evaluate, judge, classify, verdict, oracle, assess, detect]
    description: >
      Heuristic oracle: "Is this observed behavior a bug?" Three confidence
      tiers: (1) Contract violation → bug, high confidence (uses TDD contracts);
      (2) Satisfies contracts but pattern-suspicious → potential bug, medium
      confidence (uses heuristic pattern matching); (3) Novel behavior, no
      contracts → observation, low confidence (flags contract gap).
      Each evaluation emits a cns.bughunt.oracle span.
      Anchored on Weinberg: a bug is a threat to user-defined quality.
    generates_spans:
      - cns.bughunt.oracle

  - id: bug-hunt/bug-hunt-taxonomize
    path: bug-hunt-taxonomize.j2
    type: KnowAct
    lexicon_terms: [classify, categorize, taxonomize, pattern, severity, assign]
    description: >
      Classify confirmed bug findings into Beizer-derived taxonomy:
      requirements, structural, data, coding, interface, integration,
      timing, configuration. Assign severity and confidence. Generate
      pattern signature for future oracle matching. If pattern doesn't
      fit existing taxonomy categories, flag as novel pattern for
      heuristic model extension (the autopoietic taxonomy evolution step).
      Each classification emits a cns.bughunt.taxonomize span.
    generates_spans:
      - cns.bughunt.taxonomize

  - id: bug-hunt/bug-hunt-learn
    path: bug-hunt-learn.j2
    type: KnowAct
    lexicon_terms: [learn, update, refine, evolve, strengthen, weaken, synthesize]
    description: >
      THE AUTOPOIETIC CORE. Update heuristic model H_t → H_{t+1} from
      findings F_{<t}. Three operations: (1) Strengthen heuristic patterns
      that found bugs (increase weight/priority); (2) Weaken patterns that
      found nothing (decrease weight, candidate for deprecation); (3) Generate
      new heuristic categories if patterns don't fit existing taxonomy.
      Output: updated heuristic model with delta documentation.
      CNS span cns.bughunt.learn records: heuristics_before, heuristics_after,
      heuristics_added, heuristics_strengthened, heuristics_weakened,
      novel_pattern. If added + strengthened remain zero across cycles,
      the autopoietic claim is FALSIFIED.
      Equivalent to kata improvement cycle (PDCA: Act phase).
    generates_spans:
      - cns.bughunt.learn

  - id: bug-hunt/bug-hunt-report
    path: bug-hunt-report.j2
    type: WordAct
    lexicon_terms: [report, summarize, document, evidence, reproduce, trace]
    description: >
      Generate structured bug report from oracle verdict + taxonomy
      classification. Include: summary, reproduction steps (minimal from
      delta debugging), evidence (CNS span references), severity, Beizer
      category, confidence, and suggested fix (if LLM triage confidence
      ≥ 0.95: git branch created; if < 0.95: suggestion only, human gate).
      P12: every report carries replicant WebID attribution.
    generates_spans:
      - cns.bughunt.report
```

**REQ: P5-BUGHUNT-001** — pre: `registry/templates/bug-hunt/` directory exists; post: `manifest.yaml` created with exactly 7 templates, each with id, path, type, lexicon_terms, description, and generates_spans

**Verification:**
- `kask skill validate registry/templates/bug-hunt/manifest.yaml` (if available)
- Manual review: 7 templates, no template lacks `generates_spans`, all span references match `CnsSpan` variants
- Manifest parses as valid YAML

#### Step 2.2 — Create Jinja2 templates

**Files to create:**

| Template | Type | Core Responsibility |
|----------|------|-------------------|
| `bug-hunt-charter.j2` | WordAct | Charter generation from H_t + F_{<t} |
| `bug-hunt-probe.j2` | FlowDef | Expedition execution with tool dispatch |
| `bug-hunt-oracle.j2` | KnowAct | Heuristic oracle evaluation |
| `bug-hunt-taxonomize.j2` | KnowAct | Beizer taxonomy classification |
| `bug-hunt-learn.j2` | KnowAct | Heuristic model update (autopoietic core) |
| `bug-hunt-report.j2` | WordAct | Structured bug report generation |

**Template: `bug-hunt-charter.j2`** (WordAct)

```jinja2
{# Template: bug-hunt/bug-hunt-charter.j2 #}
{# Functional Role: WordAct (charter generation) #}
{# Implementation: Jinja2 prompt #}
{# Produces: Testing charter document #}
{# Bug Hunt Charter Generator #}

You are a bug-hunting charter generator operating within the hKask autopoietic testing loop. Your mission is to produce a testing charter — a focused exploration mission — informed by the current heuristic model and prior findings.

## Context

### Current Heuristic Model (H_t)
{{ heuristics | tojson }}

### Prior Findings (F_{<t})
{{ findings | tojson }}

### User-Defined Quality Criteria (P1 boundary)
{{ quality_criteria }}

### Available Testing Tools
{{ available_tools | tojson }}

### Beizer Taxonomy Categories
- requirements: bugs originating from incorrect/missing requirements
- structural: bugs in control flow, data flow, or architectural structure
- data: bugs in data types, boundaries, initialization, or persistence
- coding: bugs in implementation logic, error handling, or edge cases
- interface: bugs in API contracts, parameter passing, or protocol compliance
- integration: bugs in component interaction, ordering, or dependency management
- timing: bugs in concurrency, race conditions, or ordering assumptions
- configuration: bugs in environment, settings, or deployment state

## Task

Generate a testing charter in the Hendrickson format: "Explore [target] using [strategy] to discover [quality threat]."

Your charter MUST:
1. Target a specific code area informed by prior findings (prioritize areas where past bugs were found)
2. Use a strategy from the Heuristic Test Strategy Model (Bach) or a Hendrickson testing tour
3. Specify which Beizer taxonomy category you expect to find bugs in
4. Define oracle criteria: what constitutes a bug vs. expected behavior
5. State the confidence level of this charter finding bugs (low/medium/high) with rationale

## Output Format

```json
{
  "charter_id": "charter-{timestamp}-{target}",
  "statement": "Explore [target] using [strategy] to discover [quality threat]",
  "target": { "crate": "...", "module": "...", "function": "..." },
  "strategy": { "name": "...", "heuristic_source": "Bach HTSM | Hendrickson Tour", "description": "..." },
  "expected_taxonomy_category": "requirements | structural | data | coding | interface | integration | timing | configuration",
  "oracle_criteria": {
    "contract_anchors": ["list of REQ tags if available"],
    "heuristic_rules": ["consistency check 1", "boundary check 2"],
    "user_quality_requirement": "..."
  },
  "confidence": { "level": "low | medium | high", "rationale": "..." },
  "probe_strategy": { "tools": ["..."], "sequence": ["..."], "stop_conditions": ["..."] }
}
```

Emit CNS: cns.bughunt.charter with charter_id, target, strategy, confidence.
```

**Template: `bug-hunt-probe.j2`** (FlowDef)

```jinja2
{# Template: bug-hunt/bug-hunt-probe.j2 #}
{# Functional Role: FlowDef (expedition execution) #}
{# Implementation: Jinja2 prompt #}
{# Produces: Probe execution results #}
{# Bug Hunt Probe Executor #}

You are a bug-hunting probe executor operating within the hKask autopoietic testing loop. Your mission is to execute the testing charter by systematically probing the target.

## Charter
{{ charter | tojson }}

## Available Tools
{{ available_tools | tojson }}

## OCAP Delegation Tokens
You hold the following capability tokens:
{{ delegation_tokens | tojson }}

## Task

Execute the probe defined by the charter. Follow the probe_strategy sequence. For each step:

1. **Read** the target code (if Tool:file:Read token held)
2. **Search** for patterns matching the expected taxonomy category (if Tool:search:Execute token held)
3. **Test** using property-based generators, bolero fuzz targets, or cargo-mutants (if Tool:test:Execute token held)
4. **Record** every observation — what was probed, what happened, what was unexpected
5. **Stop** when stop_conditions are met or when the probe strategy is exhausted

## Output Format

```json
{
  "probe_id": "probe-{timestamp}-{charter_id}",
  "charter_id": "...",
  "steps_executed": [
    {
      "step": 1,
      "tool_used": "...",
      "action": "...",
      "observation": "...",
      "unexpected": true | false,
      "cns_span_ref": "cns.bughunt.probe.{step}"
    }
  ],
  "findings": [
    {
      "finding_id": "...",
      "description": "What was observed that may be a bug",
      "evidence": { "span_refs": ["..."], "logs": ["..."], "repro_input": "..." },
      "severity_initial": "low | medium | high | critical"
    }
  ],
  "coverage": { "target_explored_pct": 0.0-1.0, "patterns_probed": 0, "stop_reason": "..." }
}
```

Emit CNS: cns.bughunt.probe for each step with charter_id, tool_used, observation_count, duration_ms, replicant WebID.
```

**Template: `bug-hunt-oracle.j2`** (KnowAct)

```jinja2
{# Template: bug-hunt/bug-hunt-oracle.j2 #}
{# Functional Role: KnowAct (heuristic oracle evaluation) #}
{# Implementation: Jinja2 prompt #}
{# Produces: Oracle verdict with confidence #}
{# Bug Hunt Oracle #}

You are a heuristic oracle for bug hunting. Your job is to evaluate observations from a probe and determine: "Is this behavior a bug?" You apply Weinberg's definition: a bug is a threat to user-defined quality.

## Observation
{{ observation | tojson }}

## User-Defined Quality Criteria (P1 boundary)
{{ quality_criteria }}

## Available Contracts (from TDD)
{{ contracts | tojson }}

## Known Bug Patterns (from prior findings)
{{ known_patterns | tojson }}

## Beizer Taxonomy Reference
{{ taxonomy_reference }}

## Task

Evaluate the observation against three tiers:

### Tier 1 — Contract Violation (High Confidence)
Does the observed behavior violate any existing contract (REQ tag)?
- If YES → VERDICT: BUG, confidence: HIGH (0.90–1.00). Cite specific contract violated.
- If NO → proceed to Tier 2.

### Tier 2 — Heuristic Pattern Match (Medium Confidence)
Does the observed behavior match a known bug pattern from prior findings, or violate heuristic rules (boundary conditions, consistency checks, invariants)?
- If YES → VERDICT: POTENTIAL BUG, confidence: MEDIUM (0.60–0.89). Cite pattern matched.
- If NO → proceed to Tier 3.

### Tier 3 — Novel Observation (Low Confidence)
Is this behavior simply not covered by any existing contract or pattern?
- VERDICT: OBSERVATION, confidence: LOW (<0.60). Flag as contract gap — recommend new contract.

## Output Format

```json
{
  "oracle_id": "oracle-{timestamp}-{finding_id}",
  "finding_id": "...",
  "verdict": "BUG | POTENTIAL_BUG | OBSERVATION | NOT_A_BUG",
  "confidence": 0.0-1.0,
  "tier": 1 | 2 | 3,
  "rationale": "...",
  "evidence": { "contract_violated": "REQ-XXX | null", "pattern_matched": "pattern_id | null", "quality_threat": "..." },
  "recommendation": "fix | investigate | contract_needed | ignore"
}
```

Emit CNS: cns.bughunt.oracle with oracle_id, verdict, confidence, tier, rationale.
```

**Template: `bug-hunt-taxonomize.j2`** (KnowAct)

```jinja2
{# Template: bug-hunt/bug-hunt-taxonomize.j2 #}
{# Functional Role: KnowAct (taxonomy classification) #}
{# Implementation: Jinja2 prompt #}
{# Produces: Bug classification into Beizer taxonomy #}
{# Bug Hunt Taxonomizer #}

You are a bug taxonomist. Your job is to classify confirmed bug findings into Boris Beizer's bug taxonomy, assign severity, and generate a pattern signature for future oracle matching.

## Finding (confirmed bug)
{{ finding | tojson }}

## Oracle Verdict
{{ oracle_verdict | tojson }}

## Beizer Taxonomy (from Software Testing Techniques, 2nd ed., Ch. 2)
- **requirements**: The bug originates from incorrect, missing, ambiguous, or contradictory requirements. Specification problems (Beizer: ~30% of all bugs).
- **structural**: The bug is in control flow (wrong branch, missing case, unreachable code) or data flow (uninitialized variable, wrong reference, memory error).
- **data**: The bug is in data types (overflow, underflow, truncation, precision), boundaries (off-by-one, fencepost), initialization (uninitialized, wrong default), or persistence (corrupted state, stale cache).
- **coding**: The bug is in implementation logic (wrong algorithm, incorrect condition, missing error check) or edge cases not covered by requirements.
- **interface**: The bug is in API contracts (wrong parameter, missing validation, protocol violation), module boundaries, or type mismatches.
- **integration**: The bug emerges from component interaction (wrong ordering, missing synchronization, dependency conflict) — not visible in any single component.
- **timing**: The bug is in concurrency (race condition, deadlock, livelock), ordering assumptions, or timing dependencies.
- **configuration**: The bug is in environment settings, deployment state, feature flags, or build configuration.

## Task

1. Classify the finding into the most specific Beizer category.
2. If the finding spans multiple categories, identify the PRIMARY category (where the root cause lives) and SECONDARY categories (affected areas).
3. Assign severity: CRITICAL (blocks release, data loss, security breach), HIGH (user-visible failure, no workaround), MEDIUM (workaround exists, edge case), LOW (cosmetic, unlikely to occur).
4. Generate a pattern signature: a concise description of the bug pattern that can be used by the oracle for future matching. Include: preconditions, trigger, failure mode.
5. If the pattern does NOT fit any existing Beizer category, flag as NOVEL and describe the new category. This is the autopoietic taxonomy evolution path.

## Output Format

```json
{
  "taxonomy_id": "tax-{timestamp}-{finding_id}",
  "finding_id": "...",
  "primary_category": "requirements | structural | data | coding | interface | integration | timing | configuration",
  "secondary_categories": ["..."],
  "severity": "CRITICAL | HIGH | MEDIUM | LOW",
  "pattern_signature": {
    "preconditions": ["..."],
    "trigger": "...",
    "failure_mode": "...",
    "example_code_snippet": "...",
    "detection_heuristic": "How to detect this pattern in the future"
  },
  "novel_category": null | { "name": "...", "description": "...", "differentiation": "Why this doesn't fit existing categories" }
}
```

Emit CNS: cns.bughunt.taxonomize with taxonomy_id, primary_category, severity, pattern_signature.
```

**Template: `bug-hunt-learn.j2`** (KnowAct — THE AUTOPOIETIC CORE)

```jinja2
{# Template: bug-hunt/bug-hunt-learn.j2 #}
{# Functional Role: KnowAct (heuristic model update — autopoietic core) #}
{# Implementation: Jinja2 prompt #}
{# Produces: Updated heuristic model H_{t+1} with delta from H_t #}
{# Bug Hunt Learner — Autopoietic Core #}

You are the learning function of the autopoietic bug-hunting loop. Your job is to update the heuristic model from the session's findings. This IS the autopoietic claim: H_{t+1} = f(F_{<t}, H_t). If you produce zero heuristic change across cycles, the autopoietic hypothesis is FALSIFIED.

## Current Heuristic Model (H_t)
{{ heuristics | tojson }}

## Session Findings (F_{<t})
{{ findings | tojson }}

## CNS Health Baseline
CnsHealth before session: {{ cns_health_before | tojson }}

## Kata PDCA Context
- Plan: {{ pdca_plan }}
- Do: {{ pdca_do }}
- Check: {{ pdca_check }}
- Act: (you are producing this)

## Task

Update the heuristic model through three operations:

### Operation 1 — STRENGTHEN patterns that found bugs
For each heuristic pattern that successfully detected a bug this session:
- Increase its weight/priority
- Narrow its preconditions if the finding reveals more specific triggers
- Add the specific code location to the pattern's target list

### Operation 2 — WEAKEN patterns that found nothing
For each heuristic pattern deployed this session that found nothing:
- Decrease its weight/priority
- If weight falls below deprecation threshold (3 consecutive zero-yield sessions), mark for deprecation
- Do NOT delete — deprecated patterns are retained as negative heuristics ("don't look here again soon")

### Operation 3 — GENERATE new heuristic categories
For each finding classified as NOVEL (doesn't fit existing Beizer taxonomy):
- Create a new heuristic category with initial weight
- Define preconditions, trigger, and failure mode from the pattern signature
- Link to the finding for provenance

### Additional: Update Beizer Taxonomy (if NOVEL)
If novel patterns were found, propose taxonomy extensions. These are user-reviewable (P2: taxonomy changes are structural — user must approve).

## Output Format

```json
{
  "learn_id": "learn-{timestamp}",
  "heuristics_before": { "total_patterns": N, "active_patterns": N, "deprecated_patterns": N },
  "heuristics_after": { "total_patterns": N, "active_patterns": N, "deprecated_patterns": N },
  "changes": {
    "strengthened": [
      { "pattern_id": "...", "old_weight": N, "new_weight": N, "rationale": "..." }
    ],
    "weakened": [
      { "pattern_id": "...", "old_weight": N, "new_weight": N, "rationale": "...", "consecutive_zero_yield": N }
    ],
    "deprecated": [
      { "pattern_id": "...", "reason": "3+ consecutive zero-yield sessions" }
    ],
    "novel": [
      { "category_name": "...", "description": "...", "initial_weight": N, "source_finding": "..." }
    ]
  },
  "taxonomy_updates_proposed": [
    { "new_category": "...", "justification": "...", "requires_user_approval": true }
  ],
  "autopoietic_marker": {
    "heuristics_added": N,
    "heuristics_strengthened": N,
    "heuristics_weakened": N,
    "novel_pattern_detected": true | false,
    "heuristic_delta_nonzero": true | false
  },
  "next_session_recommendations": {
    "priority_targets": ["..."],
    "strategies_to_retry": ["..."],
    "strategies_to_avoid": ["..."]
  }
}
```

Emit CNS: cns.bughunt.learn with heuristics_before, heuristics_after, heuristics_added, heuristics_strengthened, heuristics_weakened, novel_pattern, replicant WebID.
**CRITICAL:** The `heuristic_delta_nonzero` field IS the autopoietic marker. If false across consecutive sessions, the model is FALSIFIED.
```

**Template: `bug-hunt-report.j2`** (WordAct)

```jinja2
{# Template: bug-hunt/bug-hunt-report.j2 #}
{# Functional Role: WordAct (bug report generation) #}
{# Implementation: Jinja2 prompt #}
{# Produces: Structured bug report #}
{# Bug Hunt Report Generator #}

You are a bug report generator. Your job is to produce a structured, actionable bug report from oracle verdicts and taxonomy classifications. Every report carries replicant attribution (P12).

## Confirmed Bugs (oracle verdict = BUG, confidence >= HIGH)
{{ confirmed_bugs | tojson }}

## Potential Bugs (oracle verdict = POTENTIAL_BUG)
{{ potential_bugs | tojson }}

## Observations (oracle verdict = OBSERVATION)
{{ observations | tojson }}

## User-Defined Quality Criteria (P1)
{{ quality_criteria }}

## LLM Triage Results (from kask qa triage)
{{ triage_results | tojson }}

## Task

For each confirmed bug, generate a structured report:

1. **Summary**: One-sentence description of the bug and its impact on user-defined quality.
2. **Reproduction**: Minimal steps to reproduce, leveraging delta debugging results.
3. **Evidence**: CNS span references, logs, crash traces.
4. **Classification**: Beizer taxonomy category, severity.
5. **Fix proposal**: 
   - If LLM triage confidence ≥ 0.95: "Autonomous repair proposed — git branch `bug-hunt/fix-{bug_id}` created. Requires human merge (P2)."
   - If < 0.95: "Suggested fix: [description]. Human investigation needed. Escalated via QaRepairExhausted CNS span."

For potential bugs, generate investigation recommendations.
For observations, flag contract gaps.

## Output Format

```json
{
  "report_id": "report-{timestamp}",
  "session_id": "...",
  "replicant": "{{ replicant_webid }}",
  "summary": {
    "confirmed_bugs": N,
    "potential_bugs": N,
    "observations": N,
    "contract_gaps_identified": N
  },
  "bugs": [
    {
      "bug_id": "...",
      "summary": "...",
      "reproduction": { "steps": ["..."], "minimal_input": "...", "environment": "..." },
      "evidence": { "cns_spans": ["cns.bughunt.oracle.{id}", "cns.contract.violated.{id}"], "logs": ["..."], "crash_trace": "..." },
      "classification": { "beizer_category": "...", "severity": "...", "pattern_signature": "..." },
      "fix": {
        "autonomous": true | false,
        "confidence": 0.0-1.0,
        "branch": "bug-hunt/fix-{bug_id} | null",
        "description": "...",
        "requires_human_merge": true,
        "cns_span": "cns.qa.repair_attempted.{id} | cns.qa.repair_exhausted.{id}"
      }
    }
  ],
  "investigations_recommended": [...],
  "contract_gaps": [...],
  "cns_health": { "before_session": {...}, "after_session": {...}, "deficit_delta": N }
}
```

Emit CNS: cns.bughunt.report with report_id, confirmed_bugs_count, potential_bugs_count, contract_gaps_count, replicant WebID.
```

**REQ: P8-BUGHUNT-002** — pre: manifest.yaml exists with 7 template ids; post: all 7 .j2 files exist, each with template header identifying functional role, and each referencing at least one `cns.bughunt.*` span

**Verification:**
- All 7 `.j2` files exist in `registry/templates/bug-hunt/`
- Each `.j2` file has a valid Jinja2 template header
- `grep -l "cns\\.bughunt" registry/templates/bug-hunt/*.j2 | wc -l` == 7
- Templates parse as valid Jinja2 (syntax check)

---

### Phase 3: CNS Span Emission Wiring

**Why third:** Templates reference CNS spans. The spans must actually be emitted when templates execute.

#### Step 3.1 — Add CNS span emitter functions

**File:** `crates/hkask-cns/src/bug_hunt_events.rs` (new file)  
**Pattern:** Follow `crates/hkask-cns/src/contract_events.rs`

```rust
//! Bug hunt CNS event emitters.
//!
//! Emits CNS spans for the autopoietic bug-hunting loop:
//! charter → probe → oracle → taxonomize → learn → report.
//! The learn span is the autopoietic marker — its delta indicates
//! whether the heuristic model is self-producing.

use hkask_types::WebID;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};

pub fn emit_bughunt_charter(
    sink: &dyn NuEventSink,
    replicant: &WebID,
    charter_id: &str,
    target: &str,
    strategy: &str,
    confidence: &str,
) {
    let namespace = SpanNamespace::from(CnsSpan::BugHuntCharter);
    let span = Span::new(namespace, charter_id);
    let observation = serde_json::json!({
        "replicant": replicant.to_string(),
        "target": target,
        "strategy": strategy,
        "confidence": confidence,
    });
    let event = NuEvent::new(replicant.clone(), span, Phase::Act, observation, 0);
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(target: "cns.bughunt", error = %e, "Failed to persist bughunt_charter event");
    }
}

pub fn emit_bughunt_probe(
    sink: &dyn NuEventSink,
    replicant: &WebID,
    charter_id: &str,
    tool_used: &str,
    observation_count: u64,
    duration_ms: u64,
) {
    let namespace = SpanNamespace::from(CnsSpan::BugHuntProbe);
    let span = Span::new(namespace, "probe_executed");
    let observation = serde_json::json!({
        "replicant": replicant.to_string(),
        "charter_id": charter_id,
        "tool": tool_used,
        "observations": observation_count,
        "duration_ms": duration_ms,
    });
    let event = NuEvent::new(replicant.clone(), span, Phase::Act, observation, 0);
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(target: "cns.bughunt", error = %e, "Failed to persist bughunt_probe event");
    }
}

// ... (emit_bughunt_oracle, emit_bughunt_taxonomize, emit_bughunt_report follow same pattern)

/// The AUTOPOIETIC MARKER.
/// This function emits the CNS span that makes the autopoietic claim falsifiable.
/// If heuristics_added + heuristics_strengthened remain zero across sessions,
/// the system is not autopoietic — it is merely executing static tests.
pub fn emit_bughunt_learn(
    sink: &dyn NuEventSink,
    replicant: &WebID,
    heuristics_before: u64,
    heuristics_after: u64,
    heuristics_added: u64,
    heuristics_strengthened: u64,
    heuristics_weakened: u64,
    novel_pattern: bool,
) {
    let namespace = SpanNamespace::from(CnsSpan::BugHuntLearn);
    let span = Span::new(namespace, "heuristic_updated");
    let observation = serde_json::json!({
        "replicant": replicant.to_string(),
        "heuristics_before": heuristics_before,
        "heuristics_after": heuristics_after,
        "heuristics_added": heuristics_added,
        "heuristics_strengthened": heuristics_strengthened,
        "heuristics_weakened": heuristics_weakened,
        "novel_pattern": novel_pattern,
        "heuristic_delta_nonzero": (heuristics_added + heuristics_strengthened) > 0,
    });
    let event = NuEvent::new(replicant.clone(), span, Phase::Act, observation, 0);
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(target: "cns.bughunt", error = %e, "Failed to persist bughunt_learn event");
    }
}
```

**REQ: P9-BUGHUNT-001** — pre: `CnsSpan::BugHunt*` variants exist; post: emitter functions exist in `bug_hunt_events.rs`, each emitting typed CNS spans through `NuEventSink`

**Verification:**
- `cargo build -p hkask-cns` passes
- `cargo test -p hkask-cns` — new unit tests verify span emission succeeds
- `grep -r "emit_bughunt" crates/hkask-cns/src/` shows all 6 functions

#### Step 3.2 — Register bug-hunt domain in CNS runtime

**File:** `crates/hkask-cns/src/runtime.rs`  
**Principle:** P9 (Homeostatic Self-Regulation)  
**Constraint:** Surgical change — add bug-hunt domain to existing domain initialization, don't restructure the runtime.

Add `"cns.bughunt.charter"`, `"cns.bughunt.probe"`, etc. to the CNS domain initialization list so that `VarietyTracker` monitors bug-hunt spans.

**Verification:**
- CNS health check returns bug-hunt domain in variety report
- `kask cns health` shows `cns.bughunt.*` domains

---

### Phase 4: Skill Registration

**Why fourth:** The registry crate exists but isn't discoverable until registered.

#### Step 4.1 — Add bug-hunt to bootstrap registry

**File:** `registry/templates/bootstrap-registry.yaml`  
**Principle:** P5.1 (Single Source of Truth) — registry is canonical  
**Constraint:** Surgical change — add one entry block

Add after the existing entries:

```yaml
- id: bug-hunt/bug-hunt-charter
  template_type: WordAct
  name: "Bug Hunt Charter Generator"
  lexicon_terms: [charter, explore, hunt, scope, survey, target, mission]
  description: "Generates testing charters informed by heuristic model and prior findings"
  source_path: registry/templates/bug-hunt/bug-hunt-charter.j2
  required_capabilities: []
  cascade_level: 0
  matroshka_limit: 7

# ... (repeat for all 7 templates)
```

**Verification:**
- `kask skill list` includes `bug-hunt` entries
- Bootstrap registry parses without YAML errors

#### Step 4.2 — Generate SKILL.md companion

**File:** `.agents/skills/bug-hunt/SKILL.md`  
**Principle:** P5.1 (SKILL.md is derived from registry, not co-equal)  
**Tool:** `skill-translator` skill (reverse-generate from registry crate)

**Verification:**
- SKILL.md references all 7 templates
- SKILL.md does not contain content not derivable from manifest.yaml + .j2

---

### Phase 5: Integration Tests (Falsifiability Instrumentation)

**Why last:** The entire model is falsifiable. The integration tests ARE the falsifiability instrumentation. Without them, the paper's central claim cannot be empirically evaluated.

#### Step 5.1 — CNS health deficit tracking test

**REQ: P9-BUGHUNT-002** — pre: `CnsHealth` struct exists with `overall_deficit` field; post: integration test verifies that running the bug-hunt skill across simulated sessions produces monotonically decreasing `overall_deficit`

```rust
// File: crates/hkask-cns/tests/bug_hunt_autopoiesis.rs

#[cfg(test)]
mod bug_hunt_autopoiesis_tests {
    use hkask_cns::algedonic::cns_health_check;
    use hkask_cns::bug_hunt_events::*;
    use hkask_types::cns::CnsHealth;
    use hkask_types::event::MockNuEventSink;
    use hkask_types::id::WebID;

    /// Falsifiability test: after N sessions of simulated bug hunting,
    /// CnsHealth.overall_deficit should be lower than at session 0.
    /// If this test fails, the autopoietic model IS FALSIFIED.
    #[test]
    fn autopoietic_loop_reduces_cns_deficit_over_sessions() {
        let sink = MockNuEventSink::new();
        let replicant = WebID::new("test-replicant");

        // Session 0: initial state — simulate bugs being found
        for i in 0..5 {
            emit_bughunt_charter(&sink, &replicant, &format!("charter-{}", i), "test_crate", "boundary_tour", "medium");
            emit_bughunt_probe(&sink, &replicant, &format!("charter-{}", i), "proptest", 3, 150);
            // Simulate finding a bug (high confidence oracle verdict)
            emit_bughunt_learn(&sink, &replicant, 10, 12, 2, 3, 1, false);
        }

        let health_session_0 = /* compute CnsHealth from sink events */;

        // Sessions 1-N: simulate continued hunting with heuristic refinement
        // After learning, fewer bugs should be found (heuristics improved)
        for session in 1..5 {
            for i in 0..3 {
                emit_bughunt_charter(&sink, &replicant, &format!("charter-s{}-{}", session, i), "test_crate", "learned_pattern", "high");
                emit_bughunt_probe(&sink, &replicant, &format!("charter-s{}-{}", session, i), "proptest", 1, 80);
                // Fewer bugs found as heuristics improve
                if i == 0 {
                    emit_bughunt_learn(&sink, &replicant, 12, 13, 1, 1, 0, false);
                } else {
                    emit_bughunt_learn(&sink, &replicant, 13, 13, 0, 0, 0, false);
                }
            }
        }

        let health_session_4 = /* compute CnsHealth from sink events */;

        // THE FALSIFIABLE CLAIM: deficit should decrease
        assert!(
            health_session_4.overall_deficit < health_session_0.overall_deficit,
            "AUTOPOIETIC MODEL FALSIFIED: CnsHealth.overall_deficit did not decrease across sessions.\n\
             Session 0 deficit: {}\n\
             Session 4 deficit: {}\n\
             This means the heuristic model is not self-producing — the loop does not learn.",
            health_session_0.overall_deficit,
            health_session_4.overall_deficit,
        );
    }

    /// Counter-test: if learn spans show zero heuristic change, the model
    /// should NOT claim autopoiesis — deficit should remain flat.
    #[test]
    fn zero_heuristic_delta_is_not_autopoietic() {
        let sink = MockNuEventSink::new();
        let replicant = WebID::new("test-replicant");

        // Simulate sessions with zero heuristic change
        for session in 0..5 {
            emit_bughunt_charter(&sink, &replicant, &format!("charter-s{}", session), "test_crate", "static_tour", "low");
            emit_bughunt_probe(&sink, &replicant, &format!("charter-s{}", session), "proptest", 1, 100);
            // Zero heuristic change — not autopoietic
            emit_bughunt_learn(&sink, &replicant, 10, 10, 0, 0, 0, false);
        }

        // ...assert deficit does NOT decrease meaningfully
    }
}
```

**Verification:**
- `cargo test -p hkask-cns --test bug_hunt_autopoiesis` — both tests pass
- Test failure indicates either (a) the implementation is broken or (b) the autopoietic model itself is wrong

#### Step 5.2 — Template rendering test

**REQ: P8-BUGHUNT-003** — pre: all 7 .j2 templates exist; post: each template renders without error when given minimal valid context

**Verification:**
- Integration test renders each template with `{"heuristics": {}, "findings": [], "quality_criteria": "correctness", "charter": {}, ...}` minimal context
- No template panics on render
- Output is valid JSON (for JSON-producing templates)

#### Step 5.3 — CNS span exhaustiveness test

**REQ: P8-BUGHUNT-004** — pre: `CnsSpan` enum includes `BugHunt*` variants; post: every `BugHunt*` variant appears in at least one template's `generates_spans` field and has an emitter function

**Verification:**
- `grep -r "cns.bughunt" registry/templates/bug-hunt/manifest.yaml` returns exactly 6 span references
- `grep -r "BugHunt" crates/hkask-cns/src/bug_hunt_events.rs` returns exactly 6 function definitions

---

### Phase 6: Composition with Existing Skills

**Why deferred:** Composition requires the base skill to exist and be tested first.

#### Step 6.1 — TDD composition (contract oracles)

The `bug-hunt-oracle.j2` template already references `contracts` as Tier 1 oracle input. The TDD skill produces contracts tagged with REQ identifiers. The composition is **data flow**: TDD contracts → oracle context.

**No code changes needed.** The FlowDef in `bug-hunt-probe.j2` should include a delegate step to gather contracts from the MDS specification before probing.

**Verification:**
- Manual test: run TDD cycle → run bug-hunt session → oracle references TDD contract names in verdict

#### Step 6.2 — Adversarial red-team composition (exploration probes)

The adversarial-red-team skill generates attack inputs. The `bug-hunt-probe.j2` template should include adversarial probes as one of its available strategies.

If a red-team probe finds a vulnerability that survives defenses, the bug-hunt loop classifies it, learns from it, and generates a regression test. This closes the exploitation→exploration gap.

**Integration point:** `bug-hunt-probe.j2` FlowDef delegates to `adversarial-red-team/test-against-target` as one probe strategy.

**Verification:**
- Adversarial finding → oracle classifies as bug → learn phase strengthens heuristic → next charter includes adversarial pattern

#### Step 6.3 — Kata composition (PDCA cycles)

The autopoietic loop IS a PDCA cycle. The kata framework's `kata-improvement` skill can wrap the bug-hunt skill as a coached improvement cycle, with the 5 coaching questions mapping to charter phases.

**Integration point:** `kata-improvement` FlowDef delegates to `bug-hunt/` templates for the "Do" and "Check" phases.

**Verification:**
- Kata session with bug-hunt as the improvement target produces CNS `cns.kata` spans interleaved with `cns.bughunt.*` spans

#### Step 6.4 — QA script builder composition (initial charters)

The QA script builder produces static QA pipeline manifests. These serve as **bootstrap charters** — initial exploration structure before the learning loop begins to specialize.

**Integration point:** On first session, `bug-hunt-charter.j2` can accept a QA script manifest as seed input.

**Verification:**
- QA script builder produces manifest → bug-hunt session 1 uses manifest as initial charter → session 2+ generates novel charters from learned heuristics

---

## 2. What We Are NOT Building (Anti-Scope)

Per coding-guidelines principle 2 (Simplicity First), these items are explicitly excluded:

| Excluded | Why |
|----------|-----|
| A standalone "bug hunter" binary or CLI command | Skills run through the existing `kask` cascade — no new binary |
| A dashboard or UI for bug findings | P3 §5: headless system. Findings reported through CNS spans and structured reports, not visual dashboards |
| Autonomous code merging (no human gate) | P2: fixes require affirmative consent. Skill creates branches; humans merge |
| Cross-pod span correlation | Future work — acknowledged in unified paper §5.1 |
| Causal graph inference from CNS telemetry | Speculative — acknowledged in unified paper §5.2 |
| A "confidence threshold tuner" configuration system | P5: no configurability that wasn't requested. 0.95 is the threshold; it's in the template, not a config parameter |
| Integration with external CI systems | Surgical changes only. If CI integration is needed later, it's a separate feature |
| Real-time fuzzing or continuous background hunting | Phase 1 does session-based hunting, not always-on background processes |

---

## 3. Success Criteria (Goal-Driven Execution)

| Criterion | Measurement | Pass Condition |
|-----------|------------|---------------|
| **SC-1: Skill executes** | `kask skill invoke bug-hunt/bug-hunt-charter` produces valid charter JSON | Charter parses as JSON with all required fields |
| **SC-2: CNS spans emitted** | After one session, `cns.bughunt.*` spans appear in CNS health report | ≥1 span per template phase emitted |
| **SC-3: Autopoietic marker nonzero** | After multi-session test, `cns.bughunt.learn` shows `heuristics_added > 0` or `heuristics_strengthened > 0` | Learn span delta is nonzero for at least one session |
| **SC-4: Deficit decreases** | `CnsHealth.overall_deficit` slope across 5 simulated sessions is negative | Slope < 0 supports autopoietic claim; slope ≥ 0 falsifies |
| **SC-5: ≤7 public templates** | `ls registry/templates/bug-hunt/*.j2 | wc -l` | Exactly 7 (manifest.yaml not counted as template) |
| **SC-6: All references verified** | `cargo build --workspace && cargo test --workspace` | Zero compilation errors, zero test failures |
| **SC-7: No principle violations** | Manual principle audit against P1–P12 | Zero Prohibition or Guardrail violations |

---

## 4. Files Changed (Surgical Changes Audit)

Every changed file traces to a specific implementation step:

| File | Phase | Change | Principle |
|------|-------|--------|-----------|
| `crates/hkask-types/src/cns.rs` | 1.1–1.3 | Add 6 `BugHunt*` variants to `CnsSpan` enum | P8 |
| `crates/hkask-cns/src/bug_hunt_events.rs` | 3.1 | New file: CNS span emitter functions | P9 |
| `crates/hkask-cns/src/runtime.rs` | 3.2 | Register bug-hunt domains | P9 |
| `crates/hkask-cns/tests/bug_hunt_autopoiesis.rs` | 5.1 | New file: falsifiability integration tests | P9 |
| `registry/templates/bug-hunt/manifest.yaml` | 2.1 | New file: skill manifest | P5 |
| `registry/templates/bug-hunt/bug-hunt-charter.j2` | 2.2 | New file: WordAct template | P5 |
| `registry/templates/bug-hunt/bug-hunt-probe.j2` | 2.2 | New file: FlowDef template | P5 |
| `registry/templates/bug-hunt/bug-hunt-oracle.j2` | 2.2 | New file: KnowAct template | P5 |
| `registry/templates/bug-hunt/bug-hunt-taxonomize.j2` | 2.2 | New file: KnowAct template | P5 |
| `registry/templates/bug-hunt/bug-hunt-learn.j2` | 2.2 | New file: KnowAct template (autopoietic core) | P5 |
| `registry/templates/bug-hunt/bug-hunt-report.j2` | 2.2 | New file: WordAct template | P5 |
| `registry/templates/bootstrap-registry.yaml` | 4.1 | Add 7 bug-hunt template entries | P5.1 |
| `.agents/skills/bug-hunt/SKILL.md` | 4.2 | New file: generated companion (derived from registry) | P5.1 |

**Total: 13 files.** 9 new files, 3 modified files. No adjacent code touched. No existing skills modified.

---

## 5. Execution Order Summary

```
Phase 1: CNS Span Extension (Foundation)
  └─ Step 1.1: Add BugHunt* variants to CnsSpan enum
  └─ Step 1.2: Add canonical namespace strings
  └─ Step 1.3: Update exhaustive match arms

Phase 2: Registry Crate (The Skill)
  └─ Step 2.1: Create manifest.yaml
  └─ Step 2.2: Create 7 Jinja2 templates

Phase 3: CNS Span Emission Wiring
  └─ Step 3.1: Add CNS span emitter functions
  └─ Step 3.2: Register bug-hunt domain in CNS runtime

Phase 4: Skill Registration
  └─ Step 4.1: Add bug-hunt to bootstrap registry
  └─ Step 4.2: Generate SKILL.md companion

Phase 5: Integration Tests (Falsifiability Instrumentation)
  └─ Step 5.1: CNS health deficit tracking test
  └─ Step 5.2: Template rendering test
  └─ Step 5.3: CNS span exhaustiveness test

Phase 6: Composition with Existing Skills
  └─ Step 6.1: TDD composition (contract oracles)
  └─ Step 6.2: Adversarial red-team composition
  └─ Step 6.3: Kata composition (PDCA cycles)
  └─ Step 6.4: QA script builder composition (initial charters)
```

Phases 1–4 must execute sequentially (each depends on the prior). Phase 5 can begin after Phase 3 is complete (tests can be written against the emitter API without waiting for registration). Phase 6 is deferred — skill is fully functional for single-skill execution before composition.
