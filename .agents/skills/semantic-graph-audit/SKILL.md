---
name: semantic-graph-audit
visibility: public
description: "Domain-agnostic semantic dependency graph analysis. Accepts any directed graph (code modules, crates, skills, ADRs, CNS spans, decision trees, data flows), classifies edges by constraint force, detects cycles/redundancies/gaps/orphans, and evaluates graph health through pragmatic-semantics, pragmatic-cybernetics, essentialist, and grill-me lenses. Use when the user says 'analyze the semantic graph of X', 'audit dependencies', 'is this graph optimally formed', or 'can these connections be condensed or simplified'."
---

# Semantic Graph Audit — Domain-Agnostic Dependency Analysis

A general-purpose meta-skill for analyzing **any** directed semantic graph — how nodes reference and depend on each other — for structural health. The graph can come from any domain: skill dependencies, crate/module call graphs, ADR relationships, CNS span hierarchies, decision trees, data flow pipelines, or goal cascades.

The skill is domain-agnostic. It accepts a graph of nodes and edges and applies the analytical framework. The caller (human or another skill) is responsible for loading and parsing the domain-specific edges. See **Domain Adapters** below for common patterns.

## When to Activate

- "analyze the semantic graph of [anything]"
- "audit the dependency graph of [domain]"
- "is this graph optimally formed?"
- "can these connections be condensed or simplified?"
- "does this graph have cycles or redundancies?"
- "are there missing edges in this dependency structure?"
- After adding or removing nodes from any dependency graph
- During any domain's system-level structural audit

## Do NOT Activate For

- Auditing a single artifact's internal logic → use `skill-logic-audit`
- Designing a new module's interface → use `deep-module`
- General codebase architecture exploration → use `improve-codebase-architecture` (which may then route here)

## Core Concept

Any directed graph of nodes and edges has structural properties — cycles, redundancies, gaps, fan-in/out anomalies — that affect its navigability, maintainability, and correctness. The analysis is independent of what the nodes represent.

### Edge Classification

Every edge carries a **constraint force** (see `pragmatic-semantics`):

| Force | Edge Type | Meaning |
|-------|-----------|---------|
| **Prohibition** | Required connection — graph is incoherent without it | Node A cannot function without Node B |
| **Guardrail** | Required-but-overridable connection | Default path; can skip with explicit override |
| **Guideline** | Recommended routing | Preferred path among alternatives |
| **Evidence** | Cross-reference / "see also" | Informational, not load-bearing |
| **Hypothesis** | Tentative or aspirational connection | "may connect to X if Y condition holds" |

Edges at different force levels between the same pair of nodes do **not** constitute a true cycle. A Prohibition edge A→B with a Guideline edge B→A is asymmetric in force and ontologically different — it's a workflow, not a dependency loop.

## Process

### Phase 0 — Accept the Graph

The caller provides:
- **Nodes:** A set of named entities with optional domain metadata
- **Edges:** A set of directed edges `(source → target)` with optional labels describing their semantics
- **Domain context:** What do the nodes represent? What does an edge mean in this domain?

If the caller hasn't built the graph yet, see **Domain Adapters** below for how to construct one from common sources.

### Phase 1 — Force-Classify Every Edge

Apply `pragmatic-semantics` classification to every edge. For each edge, ask:
- Is this connection load-bearing (removing it breaks the system)? → Prohibition or Guardrail
- Is this connection a preference or routing suggestion? → Guideline
- Is this connection a cross-reference with no behavioral dependency? → Evidence
- Is this connection speculative? → Hypothesis

State the classification explicitly. An unclassified edge is a gap in the analysis.

### Phase 2 — Analyze the Graph

Apply four analytical lenses in sequence:

#### Lens 1: Pragmatic Cybernetics — Feedback Loop Analysis

Analyze the graph as a cybernetic system:

| Property | Question |
|----------|----------|
| **Polarity** | Is the graph stabilizing (negative feedback: convergence toward a decision) or amplifying (positive feedback: runaway dependency)? |
| **Delay** | Where are human-in-the-loop decisions or long dependency chains that create bottlenecks? |
| **Gain** | How sensitive is the graph to changes? If one node changes, how many others are transitively affected? |
| **Closure** | Are all verification/feedback loops closed? Does every "build/extract/create" path have a corresponding "verify" path? |
| **Fidelity** | Does the graph accurately represent actual usage, or are there phantom references? |

#### Lens 2: Essentialist — 3-Gate Elimination

Apply the eliminative gates to the graph itself:

| Gate | Question | Application |
|------|----------|-------------|
| **G1 — Exist** | If this edge is deleted, does any behavior vanish? | Check each edge: is it load-bearing (Prohibition/Guardrail) or informational (Evidence)? |
| **G2 — Surface** | Does any node have > 7 direct out-edges? | Count out-degree per node; flag interface explosion |
| **G3 — Contract** | Is every edge traceable to a concrete behavior or reason? | Trace each edge: what does the target provide that the source needs? |

#### Lens 3: Grill-Me — Interrogation

Cross-examine findings:
- "Is this cycle actually a problem, or are the edges at different force levels?"
- "Could this apparent redundancy be justified by different usage contexts or phases?"
- "Is this missing edge intentional (different concern) or a genuine gap?"
- "Would condensation actually encode new behavior, or just add a pass-through layer?"
- "Is this high in-degree evidence of a shared foundation, or a coupling problem?"

#### Lens 4: Pragmatic Semantics — Provenance & Certainty

For every finding, classify:
- **Ontological mode:** IS (structural observation from the graph) vs. OUGHT (should this edge exist based on domain semantics?)
- **Epistemic mode:** Declarative (verified from source text), Probabilistic (inferred from pattern), Subjunctive (hypothetical fix)
- **Provenance:** Directly Stated (from node content), Implicit (inferred from domain knowledge), Inherited (from transitive closure)

### Phase 3 — Detect Structural Issues

| Issue | Detection | Severity |
|-------|-----------|----------|
| **True cycle** | Bidirectional edges at the same force level (both Prohibition, both Guardrail) | **Critical** — deadlock in dependency resolution |
| **Asymmetric cycle** | Bidirectional edges at different force levels | Low — workflow pattern, not a bug |
| **Duplicate edge** | Same node pair connected through multiple paths with identical semantics | Low — consolidation candidate |
| **Missing reciprocal edge** | A→B exists where domain symmetry is expected, but B→A is absent | Low — informational asymmetry |
| **Orphaned reference** | Edge targets a node that doesn't exist in the graph | **Critical** — broken reference |
| **Redundant path** | Node C is reachable from A through both A→C and A→B→C with no added value in the transitive path | Medium — condensation candidate |
| **Missing closure edge** | A node has "produce/build/create" out-edges but no "verify/validate/test" in-edges | Medium — incomplete feedback loop |
| **Fan-in anomaly** | A leaf node referenced by 5+ sources but not documented as a shared foundation | Medium — undocumented dependency |
| **Fan-out anomaly** | A node with > 7 out-edges (essentialist G2 violation) | Medium — interface explosion |
| **Isolated node** | A node with zero edges in either direction | Low — unused or disconnected |

### Phase 4 — Produce Graph Health Report

```markdown
## Semantic Graph Audit — [domain description]

### Graph Summary
- Nodes: N
- Edges: M
- Domain: [what the nodes represent]

### Dependency Graph
[mermaid diagram with force-classified edges]

### Edge Classification
| Edge | Force | Ontology | Epistemic | Rationale |
|------|-------|----------|-----------|-----------|

### Cybernetic Analysis
| Property | Assessment |
|----------|------------|

### Essentialist Analysis
| Gate | Finding | Verdict |
|------|---------|---------|

### Structural Issues
| Finding | Severity | Force | Status |
|---------|----------|-------|--------|

### Recommendations
1. **Repair:** [specific fix with rationale]
2. **Simplify:** [specific removal with rationale]
3. **Do NOT condense:** [rejected condensation with rationale]
```

## Domain Adapters

The skill itself is graph-agnostic. Use these patterns to construct graphs from common hKask domains:

### Skills Domain

Load target skills + transitive dependencies via the `skill` tool. Parse edges from SKILL.md:
- **Delegation edges** (Prohibition/Guardrail): Phase/section text matching "Delegate to `X`", "Activate `X`"
- **Reference edges** (Evidence): "Related Skills" sections, "see also", "paired with"
- **Conditional edges** (Guideline): "If condition, activate `X`" routing decisions

### Crate/Module Domain

Walk `Cargo.toml` for crate-level edges; parse `use` statements and `pub mod` declarations for module-level edges. Classify:
- **Prohibition:** Direct `use` of a type in a function signature (cannot compile without it)
- **Guideline:** Re-export convenience paths
- **Evidence:** Dev-dependency or optional feature gate

### ADR Domain

Parse `docs/architecture/ADRs/` for "Supersedes", "Depends on", "Related to" references between decision records.

### CNS Span Domain

Map `cns.*` span emission relationships: which spans trigger which algedonic alerts, which alerts escalate to which curator actions.

### Data Flow Domain

Trace data through a pipeline: source → transform → sink. Classify edges by whether data loss at that step is recoverable (Guardrail) or catastrophic (Prohibition).

## Constraints

- Classify every finding by constraint force. Never present a Hypothesis as a Prohibition.
- Do not propose changes without tracing the edge to specific source evidence.
- The graph report is advisory — the caller decides which recommendations to accept.
- If source data for a node is unavailable, note it as a data gap (Evidence) and proceed with available information.
- The analytical framework (4 lenses, structural issue detection) is invariant across domains. Only the input adapter changes.

## Composed Skills

This skill composes four analytical skills as lenses. It does not restate their full methodologies:

| Skill | Role in Graph Audit |
|-------|---------------------|
| `pragmatic-semantics` | Classify edges by ontological/epistemic mode; trace provenance |
| `pragmatic-cybernetics` | Analyze the graph as a cybernetic system (feedback loops, variety, closure) |
| `essentialist` | Apply 3-gate elimination to graph nodes and edges |
| `grill-me` | Cross-examine findings for soundness; challenge assumptions |

Additionally references:
| Skill | Role |
|-------|------|
| `pragmatic-semantics` | Force-rank every edge and finding |

## Paired Skills

- `skill-maintenance` — delegates here for skill dependency graph health (see Coverage Gap Analysis → Dependency graph health)
- `improve-codebase-architecture` — explores the codebase and may route here for cross-module dependency analysis
- `skill-logic-audit` — intra-artifact audit (complementary: this skill is inter-node)
- `skill-bundler` — if graph analysis reveals a condensation opportunity, `skill-bundler` creates the bundle
- `zoom-out` — produces module maps and caller graphs that can be fed directly into this skill

## Quick Reference

Before concluding a graph is malformed, ask:
1. Are the edges at the same force level? (Guideline + Prohibition between same pair ≠ cycle)
2. Does the apparent redundancy encode different usage contexts or phases?
3. Would a proposed condensation encode new behavior, or just add a pass-through layer?
4. Is a missing edge truly a gap, or is it a different concern that doesn't need the reference?
5. Is the finding structural (IS) or prescriptive (OUGHT)? Confusing the two produces bad recommendations.
6. Is the domain adapter correct? A misclassified edge (Prohibition where it should be Guideline) invalidates the analysis.


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/semantic-graph-audit.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = graph issues classified, no structural malformations remain undetected

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 22000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
