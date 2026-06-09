---
title: "Test Program Specification — DDMVSS Self-Applying Specification"
audience: [architects, developers, agents]
last_updated: 2026-06-07
version: "0.1.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# Test Program Specification

**Purpose:** The test program is specified using DDMVSS — it is self-applying (Task 9 pattern from DDMVSS §9). This document classifies every aspect of the test program under DDMVSS categories.

**Self-application test:** "Given a `GoalSpec` with one criterion `satisfied: true`, `is_complete()` returns `true`." This is both the first tracer-bullet test and the proof that the specification framework can specify itself.

---

## 1. Domain — Test Program Bounded Context

### Ontology

| Term | Definition | hLexicon Domain |
|------|-----------|-----------------|
| **Seam** | A public interface (`pub` trait, `pub` fn, `pub` struct with `pub` methods) that is the test surface | FlowDef |
| **Invariant** | A behavioral property that a seam must satisfy, expressed as a compilable, runnable `#[test]` | KnowAct |
| **Tracer-bullet** | A vertical RED→GREEN cycle: one invariant, one test, one implementation. Never horizontal slices | FlowDef |
| **Behavioral test** | A test that exercises the public seam and verifies *what* the system does, not *how*. Survives refactors | KnowAct |
| **Structural test** | A test coupled to implementation detail. Must be rewritten per P8 or documented as a gap in OPEN_QUESTIONS.md | KnowAct |
| **Deep seam** | Small interface, high leverage (few methods, many behaviors). Prefer testing at deep seams | WordAct |
| **Shallow seam** | Interface as complex as implementation. Deepening candidate, not a testing target | WordAct |
| **Seam depth** | Ratio of behaviors to interface surface area. Higher ratio = deeper = better test leverage | KnowAct |
| **Test cycle** | `tracer-bullet` (first test for new behavior), `regression` (test after fix), `property` (proptest/fuzz) | FlowDef |
| **DDMVSS category** | One of 9 categories that governs test invariants: Domain, Capability, Interface, Composition, Trust, Observability, Persistence, Lifecycle, Curation | KnowAct |

### hLexicon Allocation

| Term | Domain | Definition |
|------|--------|-----------|
| `trace` | WordAct | Execute a tracer-bullet test cycle (RED→GREEN for one invariant) |
| `deepen` | FlowDef | Extract a smaller interface from a shallow module to create a deeper testable seam |
| `diagnose` | KnowAct | Construct a feedback loop to identify root causes of test failures |
| `curate` | FlowDef | Evaluate test invariants via Merge/Revise/Defer/Discard curation gradient |
| `verify` | KnowAct | Mechanically check whether behavioral tests exist for a seam and verify stated invariants |
| `register` | FlowDef | Record a skill-to-DDMVSS mapping as a SpecArtifact |
| `evaluate` | KnowAct | Assess whether a skill's constraints were upheld in a diff or session |
| `handoff` | FlowDef | Transfer session context to a fresh agent for continuity |

### Bounded-Context Map

```
┌─────────────────────────────────────────────┐
│  Test Program Bounded Context              │
│                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │ hkask-   │  │ hkask-   │  │ hkask-   │ │
│  │ types    │──│ storage  │──│ mcp-spec │ │
│  │ (types)  │  │ (specs)  │  │ (tools)  │ │
│  └──────────┘  └──────────┘  └──────────┘ │
│        │              │            │        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │ hkask-   │  │ hkask-   │  │ hkask-   │ │
│  │ cns      │  │ agents   │  │ templates│ │
│  │ (spans)  │  │ (curator)│  │ (lexicon)│ │
│  └──────────┘  └──────────┘  └──────────┘ │
└─────────────────────────────────────────────┘
```

---

## 2. Capability — Test Verbs

Every test verb is an attenuatable capability:

| Verb | Resource | Interface | Attenuatable |
|------|----------|-----------|---------------|
| `verify_invariant` | `test:{seam_id}` | MCP, CLI, API | Yes |
| `trace_seam` | `test:{seam_id}` | MCP, CLI, API | Yes |
| `curate_test` | `test:{invariant_id}` | MCP, CLI, API | Yes (Curator only for Discard) |
| `deepen_seam` | `test:{seam_id}` | MCP, CLI, API | Yes |

Capability tokens for test operations follow the same `DelegationToken` pattern as spec operations (see DDMVSS §7.2). The `spec:validate` action covers test verification.

---

## 3. Interface — MCP, CLI, API Equivalence

### MCP Tool Surface

| Tool | Input | Output | hLexicon |
|------|-------|--------|----------|
| `spec/test/invariant` | `{spec_id, seam, invariant, category, cycle?, capability_token?}` | `{invariant_id, status}` | `trace`, `verify` |
| `spec/test/verify` | `{seam?, category?, capability_token?}` | `{total_requirements, tested, gaps, debt, traceability: [{requirement_id, classification?, test_path?, has_gap, test_debt_location?}], complete}` | `verify`, `diagnose` |
| `spec/skill/register` | `{skill_name, root_principle, ddmvss_categories, hlexicon_terms}` | `{artifact_id, status}` | `register`, `curate` |
| `spec/skill/evaluate` | `{skill_name, context}` | `{skill, constraints_evaluated, violations, curation_decision}` | `evaluate`, `diagnose` |

### CLI Equivalence

| MCP Tool | CLI Command |
|----------|-------------|
| `spec/test/invariant` | `kask spec test-invariant --spec-id <id> --seam <seam> --invariant <text> --category <cat> [--cycle <cycle>]` |
| `spec/test/verify` | `kask spec test-verify [--seam <seam>] [--category <cat>]` |
| `spec/skill/register` | `kask skill register --name <name> --principle <text> --categories <cats>` |
| `spec/skill/evaluate` | `kask skill evaluate --name <name> --context <context>` |

### API Equivalence

| MCP Tool | API Route |
|----------|-----------|
| `spec/test/invariant` | `POST /api/v1/tests/invariants` |
| `spec/test/verify` | `POST /api/v1/tests/verify` |
| `spec/skill/register` | `POST /api/v1/skills/register` |
| `spec/skill/evaluate` | `POST /api/v1/skills/evaluate` |

---

## 4. Composition — Test Invariants Compose

Test invariants compose via goal decomposition (DDMVSS §5.11 `mvss-compose` pattern):

- A `GoalSpec` with sub-goals decomposes into individual invariants per sub-goal.
- Registry stores test templates as `template_type: FlowDef`.
- The TDD workflow is itself a `SpecArtifact` with `template_type: FlowDef`:

```yaml
# registry/manifests/tdd-tracer-bullet.yaml
manifest:
  name: tdd-tracer-bullet
  description: Vertical tracer-bullet TDD cycle governed by DDMVSS Curation

steps:
  - ordinal: 1
    action: select
    description: "Identify seam and invariant"
    template_ref: test/templates/seam-selector.j2
  - ordinal: 2
    action: populate
    description: "Write RED test for invariant"
    template_ref: test/templates/red-test.j2
  - ordinal: 3
    action: execute
    description: "Write minimal GREEN implementation"
    template_ref: test/templates/green-impl.j2
```

---

## 5. Trust & Security

Test capability tokens are attenuatable. Threat model additions:

| Asset | Adversary | Vector | Mitigation |
|-------|-----------|--------|-----------|
| Test invariants | Malicious contributor | Invariant injection | Capability check on `spec/test/invariant` |
| Test results | Compromised runner | Result tampering | CNS span emission on test outcomes |
| Skill mappings | Malicious contributor | Skill registration with wrong DDMVSS category | `CurateEvaluate` on skill registration |

---

## 6. Observability — CNS Spans for Test Operations

Every test run emits `cns.test.*` spans:

| Span | What It Covers |
|------|---------------|
| `cns.test.trace` | Tracer-bullet test execution (RED→GREEN cycle) |
| `cns.test.verify` | Invariant verification result |
| `cns.test.curate` | Curation decision on test invariant |
| `cns.test.deepen` | Seam deepening operation |

Variety counters track test diversity per DDMVSS category. Algedonic alert when test variety drops below threshold/2.

---

## 7. Persistence — Test Results as Bitemporal Triples

Test results are stored as bitemporal triples in `hkask-storage`:

- `valid_from`: When the test invariant was introduced
- `valid_to`: When the test invariant was removed (NULL if current)
- `recorded_at`: When the test result was recorded

This enables regression history queries: "What invariants held at commit SHA X?"

---

## 8. Lifecycle — Version with Git SHA

- Test invariants version with Git SHA (no SemVer per P7)
- Deprecation: delete, don't deprecate
- Bootstrap: `CompletenessCheck::is_complete()` self-test as the atomic spec

---

## 9. Curation — Test Invariants Are Curated

Every test invariant is evaluated via `CurationDecision` gradient:

| Decision | Meaning | Action |
|----------|---------|--------|
| **Merge** | Invariant is well-specified and tests a real behavioral property | Add to test suite |
| **Revise** | Invariant is unclear or tests implementation detail | Rewrite to target public seam |
| **Defer** | Invariant may be valid but seam doesn't exist yet | Document in OPEN_QUESTIONS.md |
| **Discard** | Invariant is redundant or tests wrong thing | Remove from consideration |

A test invariant that fails Curation is **documented**, not silently dropped. This follows the C6 constraint: "A stub is a debt receipt."

---

## 10. DDMVSS Completeness Predicates — Self-Application

| # | Category | Completeness Predicate | Mechanically Verifiable? |
|---|----------|----------------------|--------------------------|
| 1 | **Domain** | Every test seam has a named term in hLexicon | ✅ `lexicon.contains(seam_name)` |
| 2 | **Capability** | Every test verb has a granted capability with attenuatable token | ✅ Capability table entry exists |
| 3 | **Interface** | MCP, CLI, API all exercise the same test capability set via one functional core | ✅ Equivalence matrix |
| 4 | **Composition** | Test invariants compose via goal decomposition | ✅ `GoalSpec::is_complete()` recursive |
| 5 | **Trust** | Every test operation has a threat-model entry | ✅ Threat model table complete |
| 6 | **Observability** | Every test run emits `cns.test.*` span | ⚠️ Partial — requires CNS integration |
| 7 | **Persistence** | Test results stored as bitemporal triples | ⚠️ Partial — schema defined, not yet implemented |
| 8 | **Lifecycle** | Test invariants version with Git SHA | ✅ Git-based versioning |
| 9 | **Curation** | Every test invariant curated via CurationDecision gradient | ✅ `SpecCurator::evaluate()` applies |

---

## 11. Skill-to-DDMVSS Mapping

| Skill | Root Principle | DDMVSS Categories | hLexicon Terms |
|-------|---------------|-------------------|----------------|
| **TDD** | Vertical tracer-bullet discipline (RED→GREEN per behavior) | Domain (goal specification), Capability (tracer-bullet cycle), Curation (evaluate invariants) | `trace` (KnowAct), `verify` (KnowAct) |
| **coding-guidelines** | Minimum code that solves the problem; no speculative features | Capability (minimal code constraint), Trust (no speculative features), Composition (surgical changes preserve seams) | `constrain` (WordAct), `require` (WordAct) |
| **improve-codebase-architecture** | Deepen modules to create testable seams | Domain (module depth assessment), Composition (deepening refactor), Curation (deletion test as completeness) | `deepen` (KnowAct), `curate` (FlowDef) |
| **diagnose** | Root cause over symptoms; feedback loop construction; spec-anchored bug-to-requirement mapping | Domain (spec-anchored bug classification), Observability (feedback loop), Trust (root cause), Lifecycle (regression test as evolution gate), Curation (spec gap identification) | `diagnose` (KnowAct), `recognize` (KnowAct) |
| **zoom-out** | Bounded context mapping for unfamiliar code | Domain (bounded context), Interface (seam identification) | `contextualise` (KnowAct), `elicit` (WordAct) |
| **caveman** | Token-efficient communication | Composition (token-efficient encoding), Interface (minimal message surface) | `select` (FlowDef) |
| **handoff** | Session continuity across agents | Lifecycle (session transfer), Curation (knowledge preservation) | `handoff` (KnowAct), `cultivate` (KnowAct) |
| **skill-bundler** | Compose multiple skills coherently | Composition (practice composition), Curation (coherence across skills) | `compose` (FlowDef), `reconcile` (FlowDef) |
| **magna-carta-verifier** | Sovereignty compliance audit via YAML assertion manifests and Jinja2 verification templates | Trust (P1–P4 enforcement verification), Curation (assertion lifecycle) | `verify` (KnowAct), `audit` (KnowAct) |
| **constraint-forces** | Classify constraints by enforcement level (Prohibition→Hypothesis); map to Magna Carta P1–P4 | Trust (constraint enforcement), Curation (conflict resolution) | `classify` (KnowAct), `resolve` (KnowAct) |
| **pragmatic-cybernetics** | CNS feedback loop analysis, VSM mapping, variety engineering, Good Regulator check | Observability (CNS span analysis), Domain (VSM component mapping), Curation (variety deficit remediation) | `detect` (KnowAct), `regulate` (KnowAct) |
| **pragmatic-semantics** | Epistemic discipline: IS/OUGHT classification, provenance tracing, constraint force hierarchy | Domain (semantic architecture), Trust (constraint enforcement), Curation (fact classification) | `classify` (KnowAct), `trace` (KnowAct) |
| **skill-translator** | Translate agent skills into hKask's dual-layer architecture (registry crate + SKILL.md companion) | Composition (cross-format composition), Curation (methodology preservation) | `translate` (KnowAct), `adapt` (KnowAct) |
| **skill-discovery** | Find, evaluate, and install dual-layer skills (SKILL.md + registry templates); gap detection | Capability (skill lifecycle), Curation (quality evaluation) | `detect` (KnowAct), `evaluate` (KnowAct) |
| **skill-maintenance** | Audit hKask's dual-layer skill architecture for staleness, coverage gaps, and quality degradation | Lifecycle (skill lifecycle states), Curation (health scoring, deprecation) | `audit` (KnowAct), `classify` (KnowAct) |
| **skill-manager** | Dual-layer CRUD for the skill corpus across Zed agent and registry layers | Capability (skill operations), Curation (corpus management) | `validate` (WordAct), `create` (WordAct) |
| **condenser-continuation** | Resume condenser implementation after context reset; inference-agnostic restoration and verification | Domain (session state restoration), Capability (build verification), Lifecycle (continuity across sessions) | `compact` (KnowAct), `restore` (KnowAct) |

---

## 12. Open Questions

See `docs/OPEN_QUESTIONS.md` for the full list. Test-program-specific questions:

1. **Mechanical vs. LLM completeness evaluation:** Can `CompletenessCheck::is_complete()` be evaluated mechanically, or does it require LLM-assisted judgment for natural-language goals? If mechanical, implement as `#[test]`. If LLM-assisted, delegate to `CurateEvaluate`.

2. **Coherence threshold calibration:** The 0.7 threshold is a starting guess. Calibrate after operational data from at least one full crate's test rewrite.

3. **Skill enforcement vs. guidance:** Should skills be enforced (pre-commit hooks, CI checks, `spec/skill/evaluate` blocking merge) or treated as guidance (curation decisions overridable per sovereignty)?

4. **Property-based testing:** Where do `proptest` and `cargo fuzz` fit? DDMVSS invariants are natural property candidates, but property testing is a different cycle than tracer-bullet. Separate skill or specialized TDD cycle?

5. **Integration test isolation:** MCP server tests require `rmcp` transport. Extract `hkask-test-utils` when 3+ servers need shared fixtures (currently 2 — below C4 threshold).

6. **CNS variety counters for test diversity:** Should `cns.test.*` spans track test variety per DDMVSS category and emit algedonic alerts when below threshold?

7. **Skill-bundler composition with TDD:** When multiple skills are active, does TDD apply per-skill or per-task? Curation decides per `CurateReconcile`.

8. **Self-application bootstrapping:** The first tracer-bullet test is: "Given a `GoalSpec` with one criterion `satisfied: true`, `is_complete()` returns `true`." This must pass before any other test is written.

---

*Test Program Specification v0.1.0 — DDMVSS self-applying, tracer-bullet disciplined, curated not governed.*