# Diataxis Quality Review — Dual Classification + Guard

Evaluated against diataxis quality gates and the hKask `diataxis-diagram` skill registry.

## Diagrams Reviewed

| Diagram | Type | File | Quadrant |
|---|---|---|---|
| Dual-Model Classification Flow | flowchart | `flowchart-dual-classification.md` | Reference |
| Guard Pipeline | flowchart | `flowchart-guard-pipeline.md` | Reference |
| Classification-to-Memory Sequence | sequence | `sequence-classify-to-memory.md` | Explanation |
| Guard Violation Lifecycle | state | `state-guard-violations.md` | Reference |

---

## Quality Gate Results

### Functional Quality (accurate, complete, consistent with source)

| Gate | Dual Flowchart | Guard Flowchart | Sequence | State |
|---|---|---|---|---|
| Entity count matches source | ✅ 15 nodes, 12 edges — covers classify_one, extract_triples_one, integrate_dual_triples | ✅ 9 nodes, 9 edges — covers all 4 active scanners (TokenLimit, RoleOverride, Deobfuscate, Secrets) | ✅ 7 participants, 6 alt/par blocks — covers full path | ✅ 4 states, 6 transitions — covers all violation types |
| Mermaid syntax valid | ✅ flowchart TD, correct node shapes | ✅ flowchart TD, correct node shapes | ✅ sequenceDiagram, correct `par`/`alt`/`loop` | ✅ stateDiagram-v2, correct `note` placement |
| Plain-English labels | ✅ No raw identifiers | ✅ OWASP risk numbers on notes | ✅ Participant aliases used | ✅ OWASP categories in notes |
| Description paragraph | ✅ Above diagram | ✅ Above diagram | ✅ Above diagram | ✅ Above diagram |
| Cross-link to code | ✅ Source file paths | ✅ Source + OWASP ref | ✅ Source file paths | ✅ Source + OWASP ref |

### Deep Quality (flows naturally, fits reader's need)

| Gate | Assessment |
|---|---|
| **Dual Flowchart** | ✅ Clear top-down flow: source → guard → models → integrate → store. The parallel model subgraph and epistemic integration subgraph correctly separate concerns. Readable without knowing Rust. |
| **Guard Flowchart** | ✅ First-hit pipeline on input, all-hits on output — the subgraph division makes this explicit. The distinction between "Input Pipeline" and "Output Pipeline" is visually clear. |
| **Sequence Diagram** | ✅ Shows temporal ordering that the flowchart can't: parallel model calls, sequential guard checks, CNS emission interleaved with processing. The `alt` blocks precisely capture the decision points. |
| **State Diagram** | ✅ The composite state `Scanning` with nested transitions correctly models the guard as a state machine. Notes connect each violation to its OWASP category. |

### Diataxis Quadrant Fit

| Diagram | Quadrant | Assessment |
|---|---|---|
| Dual Flowchart | Reference | ✅ Austere, complete, mirrors code structure. No alternatives shown — just the actual path. |
| Guard Flowchart | Reference | ✅ Neutral, descriptive. Lists what happens, not why. |
| Sequence | Explanation | ✅ Shows context and relationships. The flow from source to memory with guard interleaving explains WHY the architecture is structured this way. |
| State | Reference | ✅ Maps violation types to OWASP categories. Consultable, not tutorial. |

---

## Gaps Found

### Gap 1: No Remember Template Diagram ✅ Closed

Added `flowchart-memory-remember.md` — shows the 3-step FlowDef cascade
(operation-selector → remember-episodic → remember-semantic) with parallel
dual-model rendering and `merge_json_values` integration on each step.

### Gap 2: No Drift Detection Diagram ✅ Closed

Added `flowchart-drift-detection.md` — shows the two-threshold decision tree
(30% divergence rate, 2.0 entity asymmetry) with CNS span triggers and a
threshold reference table below the diagram.

### Gap 3: No Architecture Overview ✅ Closed

Added `flowchart-architecture-overview.md` — shows how all four subsystems compose
under P3.1 governance. Includes OWASP alignment table and subsystem-to-crate mapping.

---

## Summary

| Criterion | Score |
|---|---|
| Functional quality | ✅ All diagrams accurate and complete |
| Deep quality | ✅ Natural flow, quadrant-appropriate |
| Cross-linking | ✅ Each diagram references source files |
| OWASP anchoring | ✅ State + guard diagrams map to specific OWASP risks |
| Gaps | 3 low/medium gaps identified |

**Recommendation:** Add the architecture overview diagram to close gap 3. The other two gaps can be deferred.
