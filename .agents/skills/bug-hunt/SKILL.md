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

The expedition runs as a phased PDCA pipeline — each phase is a separate KnowAct template with its own contract:

1. **Charter** (`bug-hunt-charter.j2`): Generates a focused exploration mission using Hendrickson format and Bach's HTSM
2. **Probe** (`bug-hunt-probe.j2`): Agent-coordinated MCP tool execution — reads code, searches for bug patterns, runs tests
3. **Oracle** (`bug-hunt-oracle.j2`): Evaluates findings against user-defined quality criteria (Weinberg) with pragmatic-semantics IS/OUGHT classification
4. **Taxonomize** (`bug-hunt-taxonomize.j2`): Classifies bugs into Beizer taxonomy with severity ratings
5. **Report** (`bug-hunt-report.j2`): Produces structured JSON bug report with summary statistics
6. **Convergence** (`bug-hunt-convergence-check.j2`): Saturation detection — severity-weighted unresolved findings + stability check
7. **Loop**: Re-enters at charter if not converged

The previous monolithic `bug-hunt-expedition.j2` is retained for backward compatibility.

## Input

- `target`: crate name, module, or function to hunt in
- `quality_criteria`: what "quality" means for this target (Weinberg: value to some person who matters)

## Output

JSON report with findings, classifications, confidence scores, and pattern signatures.

## Boundary: Bug-Hunt vs Adversarial Red-Team

| | `bug-hunt` | `adversarial-red-team` |
|---|---|---|
| **What it tests** | **Source code** for quality defects | Agent **runtime behavior** under adversarial input |
| **Target** | Crates, modules, functions, data flows | Prompt defenses, instruction boundaries, tool access |
| **Taxonomy** | Beizer (logic errors, boundary bugs, race conditions) | ATLAS/GARAK (injection, hijacking, exfiltration) |
| **Method** | Read code → pattern-search → run tests | Generate adversarial inputs → classify agent responses |
| **Output** | Structured bug reports with severity | Resistance rates per attack category |
| **When to use** | "Does this code have bugs?" | "Is this agent exploitable?" |

These skills **compose**, they do not merge. Bug-hunt embeds adversarial-red-team as a lens for boundary-value probing and unexpected state transitions — adversarial thinking applied to code, not agent prompts. If you need to test an agent's *runtime* resistance to prompt injection, use `adversarial-red-team` directly — not bug-hunt.

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
- **Templates:**
  - `bug-hunt-charter.j2` — Phase 1: Charter generation
  - `bug-hunt-probe.j2` — Phase 2: Agent-coordinated probe
  - `bug-hunt-oracle.j2` — Phase 3: Weinberg oracle + pragmatic-semantics
  - `bug-hunt-taxonomize.j2` — Phase 4: Beizer taxonomy classification
  - `bug-hunt-report.j2` — Phase 5: Structured JSON report
  - `bug-hunt-convergence-check.j2` — Phase 6: Saturation detection
  - `bug-hunt-expedition.j2` — Legacy monolithic template (retained for compatibility)

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
- **rJoule (inference energy):** cap 2 (manifest `rjoule.cap` — see `registry/manifests/bug-hunt.yaml` for canonical value)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
