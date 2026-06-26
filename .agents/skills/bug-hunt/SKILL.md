---
name: bug-hunt
version: "0.30.0"
visibility: Public
namespace: bug-hunt
description: >
  Bug hunting skill. Runs expeditions against target crates to find threats
  to user-defined quality. Applies Weinberg's quality definition, Beizer's
  bug taxonomy, Bach's Heuristic Test Strategy Model, and Hendrickson's
  exploratory testing charters. Uses MCP tools (file read, code search,
  terminal) to probe code and produces structured bug reports.
trigger: >
  User says "hunt bugs in X", "find bugs", "bug hunt", "explore for bugs",
  "what bugs are in this crate", or specifies a target with quality criteria.
---

# Bug Hunt Skill

A bug hunting skill that explores target crates for threats to user-defined quality.

## When to Use

- "Hunt bugs in hkask-wallet"
- "Find bugs in hkask-cns — quality criteria: no energy budget violations"
- "Explore hkask-types for data boundary bugs"
- "What bugs exist in hkask-capability?"

## How It Works

1. **Charter:** Generates a focused exploration mission using Hendrickson format
2. **Probe:** Reads code, searches for bug patterns, runs tests via MCP tools
3. **Oracle:** Evaluates findings against user-defined quality criteria (Weinberg)
4. **Taxonomize:** Classifies bugs into Beizer taxonomy with severity
5. **Report:** Produces structured JSON bug report with fix suggestions

## Input

- `target`: crate name, module, or function to hunt in
- `quality_criteria`: what "quality" means for this target (Weinberg: value to some person who matters)

## Output

JSON report with findings, classifications, confidence scores, and pattern signatures.

## Composition

The expedition template embeds reasoning patterns from five skills as inline prompt instructions:

| Skill | Embedded in template | Role |
|-------|---------------------|------|
| pragmatic-semantics | ✓ (v0.30.0) | IS/OUGHT classification, epistemic mode, provenance tracing |
| pragmatic-cybernetics | ✓ (v0.30.0) | Feedback loop analysis, Good Regulator checks, variety engineering |
| diagnose | ✓ (v0.30.0) | Reproduce before diagnosing, single-variable isolation |
| adversarial-red-team | ✓ (v0.30.0) | Boundary-value probes, unexpected state transitions |
| grill-me | ✓ (v0.30.0) | Self-challenge verdicts, intentional-vs-bug discrimination |

Additionally referenced (not embedded): TDD (contract verification) and kata (PDCA learning) inform the expedition methodology but are not inline in the template.

**Note:** These are inline prompt instructions, not delegated inference calls. Versions are annotated above. If the referenced skills change methodology, the expedition template should be updated.

## Registry

- **Canonical source:** `registry/manifests/bug-hunt.yaml`
- **Template:** `registry/templates/bug-hunt/bug-hunt-expedition.j2`

### Convergence Calibration

The convergence threshold is 0.25 — the most permissive of all hKask skills. This is intentional: bug hunting is **exploratory**, not exhaustive. A 0.25 threshold means findings are directionally stable and critical quality threats are identified, but not every bug class has been exhaustively searched. See Registry Manifest below for chaining guidance.


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/bug-hunt.yaml`

### PDCA Convergence
- **Threshold:** 0.25 — highest of all skills; intentional for exploratory hunting (not exhaustive elimination)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = findings stable, no critical bugs remain unresolved. For exhaustive elimination, chain with `diagnose` on specific findings.

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 18000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
