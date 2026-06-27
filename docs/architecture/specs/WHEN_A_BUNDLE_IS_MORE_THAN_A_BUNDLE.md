---
title: "When a Bundle Is More Than a Bundle — PKO Graph Synthesis for Skill Composition"
audience: [architects, developers, agents, researchers]
last_updated: 2026-06-26
version: "0.32.0"
status: "Draft"
domain: "Composition"
mds_categories: [composition, domain]
references:
  - Carriero et al., 2025, arXiv:2503.20634 — PKO ontology
  - PROV-O, W3C Recommendation, 2013
  - Van der Aalst, 2016 — Process Mining
  - Rozenberg, 1997 — Graph Grammars
  - Rother, 2009 — Toyota Kata
  - Bunke & Allermann, 1983 — Graph edit distance
---

# When a Bundle Is More Than a Bundle

**PKO Graph Synthesis for Skill Composition in hKask**

## 1. The Question

A bundle is a list of skills to activate in sequence. A composite is a genuinely new skill — a single multi-loop PDCA procedure where the original loops are structurally integrated, redundancies are eliminated, shared patterns are fused, and novel self-reinforcing feedback edges connect previously independent processes.

**When is a bundle more than a bundle?** When the constituent skills' PDCA graphs are merged through graph-theoretic operations into a single `pko:CompositeProcedure` that executes with its own convergence threshold, energy budget, and CNS instrumentation.

## 2. Grounding

### 2.1 Skills Are Process Loops

A skill in hKask is a PDCA improvement loop. Its FlowDef manifest declares a sequence of steps, a convergence threshold, an energy budget, and a loop target. Mastery is continuous improvement — through kata practice (Rother, 2009), recursive self-improvement, or learning and discovery.

### 2.2 PKO Is the Scaffold

The Procedural Knowledge Ontology (Carriero et al., 2025) decomposes process specifications into RDF triples:

```
pko:Procedure          — the skill
pko:hasStep            — links procedure to steps
pko:Step               — a step within the procedure
pko:nextStep            — sequential ordering between steps
pko:StepVerification   — a quality check with a threshold
pko:ProcedureTarget    — convergence threshold value
pko:CompositeProcedure — a procedure composed of sub-procedures
pko:precedes           — temporal precedence
pko:consumes           — what a step takes as input
pko:produces           — what a step generates as output
pko:feedsBack           — cross-skill feedback edge
```

PKO is the scaffold — it links every subgraph, provides shared vocabulary that eliminates the ontology alignment problem (Euzenat & Shvaiko, 2013), and enables graph operations impossible on raw YAML.

### 2.3 Graph Theory Provides the Operations

| Operation | Technique | Reference |
|-----------|-----------|-----------|
| Pattern detection | Subgraph isomorphism (VF2) | Cordella et al., 2004 |
| Similarity measurement | Graph edit distance | Bunke & Allermann, 1983 |
| Edge minimization | Transitive reduction | Aho et al., 1972 |
| Model merging | Process model merging, behavioral profiles | Van der Aalst, 2016 |
| Graph transformation | DPO rewriting rules | Rozenberg, 1997 |

### 2.4 RSI Instruments the Result

The recomposed graph is serialized to a FlowDef manifest and instrumented with **R**ecursive **S**elf-**I**mprovement: convergence thresholds, dual energy budgets (gas + rJoule), CNS span emission, and algedonic alerting.

## 3. The Synthesis Pipeline

```
[N Skill Manifests]
       │
       ▼
┌──────────────────────────────────────┐
│ 1. DECOMPOSE                         │
│    FlowDef → PKO RDF triples         │
│    Each skill: nodes typed, edges    │
│    labeled with PKO vocabulary       │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│ 2. ANCHOR                            │
│    Link subgraphs via PKO edges      │
│    pko:precedes (ordering),          │
│    pko:consumes/produces (data flow) │
│    prov:wasDerivedFrom (provenance)  │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│ 3. COMPARE                           │
│    Subgraph isomorphism for overlaps │
│    Edit distance for similarity      │
│    Collision detection (contradictory│
│      outputs, incompatible targets)  │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│ 4. DECIMATE (delete redundancies)    │
│    Pass-throughs: in=1,out=1,no prod │
│    Duplicate nodes: same type+inputs │
│    Orphans: in=0, out=0              │
│    Transitive edges: A→C where A→B→C │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│ 5. FUSE (merge isomorphic patterns)  │
│    Verification nodes → composite    │
│    Shared step patterns → unified    │
│    Identical chains → single chain   │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│ 6. RECOMPOSE (add novel edges)       │
│    pko:feedsBack where B→A input     │
│    Conditional loop branching        │
│    Cross-skill feedback cycles       │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│ 7. INSTRUMENT (RSI)                  │
│    Serialize → FlowDef manifest      │
│    Set composite convergence         │
│    Allocate gas/rJoule budgets       │
│    Wire CNS spans + algedonic alerts │
└──────────────┬───────────────────────┘
               ▼
     [Composite Skill Manifest]
     registry/manifests/<id>.yaml
```

## 4. Graph Rewriting Rules (DPO)

### D1: Eliminate Pass-Through
```
LHS: A → B → C where B has in=1, out=1, no produces, no consumes
RHS: A → C (delete B)
```

### D2: Fuse Verification Nodes
```
LHS: V1, V2 both pko:StepVerification, edit_distance(V1,V2) < δ
RHS: Single Vc: pko:StepVerification, pko:verifies [V1.source,V2.source],
     pko:ProcedureTarget max(V1.target, V2.target)
```

### D3: Eliminate Duplicate Steps
```
LHS: S1, S2 both pko:Step, identical consumes+produces+template_ref
RHS: Delete S2, rewire S2 edges to S1
```

### R1: Generate Feedback Edge
```
LHS: Skill A produces X, Skill B consumes X (prov:wasDerivedFrom)
RHS: Add B.loop pko:feedsBack A.entry
     Condition: when B.convergence > A.convergence
```

### R2: Conditional Loop Branching
```
LHS: Composite has V1...Vn each with pko:ProcedureTarget Ti
RHS: Loop has conditional targets: L→step_i iff Vi.metric > Ti
     L→exit iff all Vi.metric ≤ Ti
```

## 5. When a Bundle IS More Than a Bundle

| Condition | How Verified |
|-----------|-------------|
| At least one decimation applied | `|decimations| > 0` |
| At least one fusion applied | `|fusions| > 0` |
| At least one novel edge added | `|novel_edges| > 0` |
| Composite has fewer steps than sum | `composite_steps < Σ(original_steps)` |
| Composite has its own convergence threshold | `composite.threshold ≠ any individual` |
| Composite has conditional loop branching | `loop.targets` is array of {step, condition} |
| Composite is PKO + DC anchored | `pko:CompositeProcedure` AND `dcterms:creator` |
| Composite is RSI instrumented | CNS spans, gas/rJoule, algedonic alerts |

If none hold, the bundle is just a bundle.

## 6. Integration

The synthesis engine is the `skill-bundler` v0.32.0 FlowDef (`registry/manifests/skill-bundler.yaml`):

| Step | Template | Phase |
|------|----------|-------|
| 1 | `goal-analysis/create` | Goal extraction |
| 2 | `bundler-compose` | Select, order, PKO-anchor skills |
| 3 | `bundler-synthesize` | Graph, anchor, compare, decimate, fuse, recompose |
| 4 | `bundler-validate` | Validate composite (V1-V15) |
| 5 | `bundler-convergence-check` | Structural + goal convergence |
| 6 | `bundler-evolve` | Goal-delta recomposition |
| 7 | `loop → 2` | Self-improvement cycle |

Output: either `"composite"` (a new skill manifest) or `"bundle_only"` (no synthesis opportunities — skills run in sequence).
