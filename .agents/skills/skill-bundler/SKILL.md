---
name: skill-bundler
visibility: public
description: "Goal-anchored, PKO-grounded skill composition. Extracts a structured goal from user intent, composes skills into a self-improvement PKO knowledge graph with DC provenance and PROV-O artifact linkage, validates structural AND ontological rules (P5.4 dual-axis mandate), and evolves the bundle via goal-delta-driven recomposition. Use when the user says 'bundle skills', 'compose skills', 'activate a skill bundle', or wants multiple skills active simultaneously in a session."
activation: "compose skills into a bundle"
---

# Skill Bundler — Goal-Anchored PKO Composition

You are a skill composition orchestrator operating within hKask's dual-axis ontological framework (P5.4). Your job is to take a set of skills AND explicit user intent/goal, extract a structured goal, compose the skills into a **PKO-grounded bundle knowledge graph**, validate it, and evolve it in a self-improvement loop.

## Core Innovation: Goal-Anchored Self-Improvement Loop

The key to any self-improvement loop is the goal. Without a goal, composition is just ordering — it has no direction, no success criterion, no feedback. With a goal:

1. **Goal Extraction** — User intent becomes a structured goal with explicit completion criteria via `goal-analysis/create`
2. **Goal-Anchored Composition** — Skills are selected and ordered to ACHIEVE the goal, not just to avoid structural conflicts
3. **Goal-Verified Convergence** — The bundle converges when it is both structurally valid AND goal-aligned
4. **Goal-Delta-Driven Evolution** — When the bundle fails to achieve its goal, `bundler-evolve` re-composes based on the gap between output and criteria

This is the **Improvement Kata at the composition level**: the bundle is the experiment, the goal is the target condition, and each recomposition is a PDCA cycle.

## Registry Templates

This skill's runtime templates live in `registry/templates/skill-bundler/`:

| Template | Type | Purpose |
|----------|------|--------|
| `bundler-compose.j2` | KnowAct | Analyze skills for conflicts, complementarities, and optimal ordering; produce PKO-anchored bundle manifest with DC provenance and PROV-O linkage |
| `bundler-validate.j2` | KnowAct | Validate a bundle manifest against composition rules AND ontological anchoring requirements (P5.4) |
| `bundler-convergence-check.j2` | KnowAct | Compute convergence metric accounting for structural validity AND goal achievement |
| `bundler-evolve.j2` | KnowAct | Goal-delta-driven recomposition — the "Act → Plan" bridge in the bundle's self-improvement kata |

The SKILL.md (this file) teaches the Zed coding agent the composition methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

### FlowDef PDCA Structure

The bundler's FlowDef manifest now implements the full goal-anchored PDCA loop:

| Step | Phase | Template | Role |
|------|-------|----------|------|
| 1 | Goal Extraction | `goal-analysis/create` | Extract structured goal from user intent |
| 2 | Plan | `bundler-compose` | Compose bundle to achieve goal, PKO/DC/PROV-O anchored |
| 3 | Do | `bundler-validate` | Validate structural AND ontological rules |
| 4 | Check | `bundler-convergence-check` | Check convergence (structural + goal-aligned) |
| 5 | Act | `bundler-evolve` | Goal-delta-driven recomposition |
| 6 | Loop | `loop → step 2` | Re-enter composition cycle |

## Ontological Anchoring (P5.4 Dual-Axis Mandate)

Every bundle must carry both axes:

| Axis | Ontology | Bundle Binding |
|------|----------|---------------|
| **Process (verb)** | PKO | Bundle IS `pko:CompositeProcedure`. Skills ARE `pko:Procedure` with `pko:Step` substeps (Plan, Do, Check, Act). Sequence IS `pko:precedes`. |
| **State (noun)** | DC+BIBO | Bundle manifest IS `bibo:Document` with `dcterms:creator`, `dcterms:created`. |
| **Provenance** | PROV-O | Skill outputs link via `prov:wasDerivedFrom`. Artifacts carry `prov:wasGeneratedBy`. |

Failure to anchor in both axes is a P5.4 violation (V11/V12).

## Composition Principles

These principles govern how skills compose. They are derived from workflow research (van der Aalst), creative/critical pairing research (Double Diamond, Six Thinking Hats), homeostatic regulation models, and PKO procedural semantics.

### 0. Goal Anchoring (NEW — v0.31.0)

**Every bundle composition begins with a goal.** User intent is extracted into a structured goal with explicit completion criteria via `goal-analysis/create`. Skills are selected and ordered to ACHIEVE the goal. A bundle without a goal is a recipe, not a self-improvement system.

### 1. Phase Separation

**Never place divergent and convergent skills in the same cascade phase.**

| Mode | Characteristic | Cascade Phase |
|------|---------------|---------------|
| Divergent (creative/generative) | Explores possibilities, generates options | Pre-core |
| Productive (execution) | Implements, sequences, produces output | Core |
| Convergent (critical/evaluative) | Narrows, validates, constrains | Post-core |

Skills that expand the solution space (generative) must run BEFORE skills that narrow it (evaluative). Mixing them in the same phase produces contradiction or cancellation.

### 2. Ordering Principle

**Default cascade order: Recognize → Act → Reflect.**

Deviations from this order must be explicitly justified in the manifest.

### 3. Domain Complementarity

**Skills from different domains compose more safely than skills from the same domain.**

When two skills share a domain, check for polarity:
- Same polarity (both divergent or both convergent) → may amplify → compose as parallel streams or sequential refinement
- Opposite polarity (one divergent, one convergent) → conflict risk → separate into different cascade phases

### 4. Conflict Resolution Hierarchy

When skills conflict, resolve in this order:

1. **Domain separation**: Different domains → compose trivially
2. **Phase separation**: Same domain, different phase → compose sequentially
3. **Specificity wins**: Same domain, same phase, different specificity → more specific overrides
4. **Manifest override**: Same domain, same phase, same specificity → manifest `conflicts` section decides
5. **User intent wins**: All else equal → follow the user's current request

### 5. Convergence Criteria

Every iterative or recursive composition must declare a convergence criterion. Default: coherence ≥ 0.7, drift < 0.5.

### 6. Depth and Term Limits

- Cascade depth must not exceed 7
- Each skill declares ≤ 10 key terms
- Bundles exceeding ~30 unique terms should be decomposed into sub-bundles

### 7. Skill Polarity Classification

When composing, classify each skill by polarity:

| Polarity | Characteristic | Role in Bundle |
|----------|---------------|----------------|
| **Generative** | Explores possibilities, proposes alternatives | Expands solution space. Goes early. |
| **Evaluative** | Tests, critiques, validates | Narrows solution space. Goes late. |
| **Regulative** | Constrains, requires, monitors | Governs process. Applied across phases. |
| **Procedural** | Sequences, synchronizes, routes | Orchestrates flow. Applied as backbone. |

## Workflow

### Step 0: Extract Goal

The user provides intent. The bundler extracts a structured goal with explicit completion criteria via `goal-analysis/create`. This goal anchors the entire composition — every skill is evaluated for its contribution to goal achievement.

### Step 1: User Submits Skill List

The user specifies which skills to bundle. This is the input that triggers composition. The user may also ask for suggestions about which skills would complement each other.

### Step 2: Smart Match

Check for existing bundles that match the specified skills:
- Exact match → offer to apply, evolve, or compose new
- Partial/similar match → show similar bundles and offer to apply or compose
- No match → proceed to compose

### Step 3: Compose Bundle

Analyze the skills for conflicts, complementarities, and optimal ordering, then produce a structured manifest containing:

```
bundle:
  name: <descriptive-name>
  version: 1.0.0
  skills:
    - name: <skill-name>
      polarity: <generative|evaluative|regulative|procedural>
      phase: <pre-core|core|post-core>
      cascade_order: <ordinal>
  conflicts:
    - skills: [<skill-a>, <skill-b>]
      resolution: <domain-separation|phase-separation|specificity-wins|manifest-override>
  complementarities:
    - skills: [<skill-a>, <skill-b>]
      leveraged: <how>
  convergence:
    criterion: <description>
```

### Step 4: Validate Bundle

Check the composed manifest:
- No contradictory directives in the same cascade phase
- Cascade depth ≤ 7
- Each skill appears exactly once
- Every conflict has a resolution
- At least one productive skill present
- Convergence criterion declared

### Step 5: User Review

Present the composed manifest to the user for review and approval. Show:
- Skills and their polarity classifications
- Phase assignments
- Conflicts identified and resolutions
- Complementarities leveraged

Ask about visibility:
- **Private**: Bound to the current session
- **Shared**: Available to all sessions

### Step 6: Apply Bundle (when active)

When a bundle is active for a session, follow the cascade order, respect phase separation, and honor conflict resolutions. Each skill in the bundle activates at its designated phase.

### Step 7: Evolve Bundle

When the user says "evolve bundle [name]":
- Re-assess all skills in the bundle
- If any skill has changed, re-compose the manifest
- Preserve what hasn't changed and update what has
- Present the updated manifest for user review

## Anti-Patterns to Flag When Composing

| Anti-Pattern | Detection | Resolution |
|-------------|-----------|------------|
| **Cancel-out** | Divergent + convergent in same phase | Move divergent to pre, convergent to post |
| **Contradictory directives** | Multiple constraints for same target | Specificity wins; add reconciliation step |
| **Ordering collision** | Same domain, same phase, same specificity | Explicit cascade order in manifest |
| **Runaway feedback** | Skill A triggers Skill B triggers Skill A | Convergence criterion + depth limit |
| **Scope creep** | Too many terms per skill or per bundle | Decompose into sub-bundles |
| **Dead letter** | No productive skill in the bundle | Require at least one productive skill |

## Common Bundles (Goal-Anchored)

| Bundle | Skills | Goal | Use Case |
|--------|--------|------|----------|
| **coding-session** | coding-guidelines + grill-me | Write correct, simple code with self-critique | Behavioral guardrails + self-check |
| **debug-session** | diagnose + coding-guidelines | Find and fix the root cause of a specific bug | Systematic debugging |
| **architecture-review** | improve-codebase-architecture + zoom-out + grill-me | Identify and prioritize architectural improvements | Big-picture refactoring |
| **tdd-session** | tdd + coding-guidelines + diagnose | Build a feature with test-first discipline | Red-green-refactor |
| **self-improvement** | pragmatic-laziness → essentialist → grill-me → semantic-graph-audit | Reduce total system action through iterative elimination | Architecture simplification kata |

## Commands

When the user says:

- **"bundle skills [skill1, skill2, ...]"** or **"compose skills [...]"** — Start Step 2-3 (smart match, then compose).
- **"activate bundle [name]"** — Load a saved manifest and enter apply mode.
- **"list bundles"** — Show known bundles.
- **"evolve bundle [name]"** — Start Step 7 (evolve).
- **"show bundle [name]"** — Display the manifest.
- **"deactivate bundle"** — Stop applying the current bundle.
- **"bundle skills"** — List available skills with polarity classifications.

## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/skill-bundler.yaml` | **Version:** 0.31.0

### FlowDef Structure (6 steps)

| Step | Template | Phase |
|------|----------|-------|
| 1 | `goal-analysis/create` | Goal Extraction |
| 2 | `bundler-compose` | Plan (goal-anchored) |
| 3 | `bundler-validate` | Do (structural + ontological) |
| 4 | `bundler-convergence-check` | Check (structural + goal) |
| 5 | `bundler-evolve` | Act (goal-delta recomposition) |
| 6 | `loop → step 2` | Loop (self-improvement cycle) |

### PDCA Convergence
- **Threshold:** 0.10 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = no blocking bundle violations remain AND goal criteria are met
- **Convergence field:** `step_4_result.convergence_metric`

### Ontological Anchoring (P5.4)
- **PKO:** Bundle = `pko:CompositeProcedure`, skills = `pko:Procedure`, steps = `pko:Step`, execution = `pko:ProcedureExecution`
- **DC+BIBO:** Manifest = `bibo:Document`, `dcterms:creator`, `dcterms:created`
- **PROV-O:** Skill edges = `prov:wasDerivedFrom`, artifacts = `prov:wasGeneratedBy`

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 2 rJ
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)

### Inputs
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `skill_names` | array | yes | Skills to compose |
| `user_intent` | string | yes | What the bundle should achieve |
| `existing_bundles` | array | no | For smart matching |
| `goal_verdict` | object | no | Previous verdict (for recomposition) |

### OCAP Delegation
Delegates to `goal-analysis/create` (step 1) and 4 internal templates (steps 2–5). Requires `template_scoped: true` with `ed25519` signatures.

## Commands

When the user says:

- **"bundle skills [skill1, skill2, ...]"** or **"compose skills [...]"** — Start goal extraction → smart match → compose.
- **"activate bundle [name]"** — Load a saved manifest and enter apply mode.
- **"list bundles"** — Show known bundles.
- **"evolve bundle [name]"** — Re-compose based on goal delta (bundler-evolve).
- **"show bundle [name]"** — Display the manifest and its PKO/DC/PROV-O anchors.
- **"deactivate bundle"** — Stop applying the current bundle.
- **"bundle skills"** — List available skills with polarity classifications.
