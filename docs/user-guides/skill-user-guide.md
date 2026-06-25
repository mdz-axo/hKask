---
title: "hKask Skill User Guide — Discovering and Using Agent Skills"
audience: [developers, operators, curators]
last_updated: 2026-06-23
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# hKask Skill User Guide

**Purpose:** How to discover, load, and use skills in hKask. Covers the `kask skill` CLI, skill list, activation triggers, the FlowDef+PDCA skill model, and a catalog of all 45 available skills organized by functional category.

**Companion doc:** [`skill-designer-guide.md`](../../docs/guides/skill-designer-guide.md) — for creating and maintaining skills.

---

## 1. What Are Skills?

Skills are composable agent capabilities with explicit PDCA convergence loops. hKask distinguishes three invocation types:

| Type | Behavior | Exit |
|------|----------|------|
| **Template** | One-shot prompt execution | Returns output |
| **Skill** | Iterative PDCA cascade with quality threshold + energy budget | Returns `converged` \| `maxed_out` \| `escalated` |
| **Bundle** | Composition orchestration — delegates to sub-skills | Depends on aggregation method |

In the current hKask model (v0.30.0), **42 Skills** have FlowDef process manifests with `convergence.threshold > 0`, `gas.cap > 0`, `rjoule.cap > 0`, and a `loop` action. **2 Templates** (logo-builder, qa-script-builder) are one-shot without registry manifests. **1 Bundle** (kata) composes skills but is not itself a PDCA loop. **1 Legacy** (kata-pattern pre-upgrade) remains at v0.21.4.

| Layer | You interact with it via... | Purpose |
|-------|---------------------------|---------|
| **FlowDef process layer** (`registry/manifests/<skill>.yaml`) | `kask run <skill>` / runtime engine | Declares convergence rails, gas rails, and explicit `loop` behavior |
| **Template crate layer** (`registry/templates/<skill>/`) | Runtime dispatch via `template_ref` ids | Provides WordAct/KnowAct steps composed by the FlowDef |
| **Zed companion layer** (`.agents/skills/<name>/SKILL.md`) | The `skill` tool in Zed | Teaches procedural guidance to the coding agent |

For user-facing skills, the FlowDef process is authoritative. `SKILL.md` is the companion instruction layer; bundle orchestration (for example `kata`) remains a separate construct.

### 1.1 Skill Architecture Layers

Skills are organized into five composition layers, running top-to-bottom:

```
Perceptual  →  dokkodo-mindset (clears attachment, preference, fear)
Regulative  →  constraint-forces (enforces boundaries across all layers)
Analytic    →  pragmatic-laziness, essentialist, grill-me, mcda
Executive   →  coding-guidelines, domain skills, task-specific skills
Governance  →  magna-carta-verifier, pragmatic-semantics/cybernetics
```

Skills at higher layers feed clarified perception to lower layers. `constraint-forces` runs across all layers — Prohibitions and Guardrails are never relaxed.

### 1.2 The Essential Five — Skills Every Agent Should Know

Start here. These five skills compose together into the most common workflow and teach the foundational patterns:

| # | Skill | Why it matters | First trigger to try |
|---|-------|---------------|---------------------|
| 1 | `dokkodo-mindset` | Clears attachment, preference, and fear before analysis — transforms how you see the problem | "apply the Dokkodo" |
| 2 | `pragmatic-laziness` | Finds the path of least action through any design or decision | "be lazy about this" |
| 3 | `essentialist` | Deletes what doesn't earn its existence — the deletion test | "simplify this" |
| 4 | `coding-guidelines` | Constrains HOW you code — think first, keep it simple, touch only what you must | "use coding-guidelines" |
| 5 | `constraint-forces` | Classifies every constraint so you know what's inviolable and what's negotiable | "what force is this constraint?" |

**The primary chain:** `dokkodo-mindset` → `pragmatic-laziness` → `essentialist` → `coding-guidelines`, with `constraint-forces` running across all stages. Learn these five and you can navigate any hKask task.

### 1.3 Why Some Skills Lack SKILL.md

Of 98 registry manifests, 46 have SKILL.md companions in `.agents/skills/`. The remaining 52 are **infrastructure crates** — runtime dispatch, storage, monitoring, and orchestration templates that run the system rather than perform user-facing tasks. A skill with only a registry crate (`.j2` + `manifest.yaml`) is runtime-complete — the cascade can execute it. The SKILL.md is a **generated companion** for the Zed coding agent during development. When both exist, the registry crate is authoritative. See `hKask-architecture-master.md` Pattern A for the full derivation rule.

---

## 2. Discovering Skills

### 2.1 List All Skills

```bash
kask skill list
```

Output shows name, visibility, description, and activation trigger for every installed skill.

### 2.2 Show Skill Details

```bash
kask skill show coding-guidelines
```

Displays the full SKILL.md content, template inventory, and contract signatures.

### 2.3 Filter by Visibility

```bash
kask skill list --visibility Public     # Shared/discoverable skills
kask skill list --visibility Private    # Your personal/namespace skills
```

---

## 3. Activating a Skill

### 3.1 In Zed (Agent Layer)

Skills activate when a Zed agent's prompt matches the skill's `description` trigger. You can also explicitly invoke:

```
Use the coding-guidelines skill before reviewing this PR.
```

The Zed agent loads `SKILL.md`, absorbs the procedural knowledge, and applies it to the task.

### 3.2 Via the kask CLI (Registry Layer)

```bash
kask skill invoke coding-guidelines/guidelines-assess --input task_description="review auth module"
```

Renders the `.j2` template with your input, executes through the inference router, returns structured output.

### 3.3 Via Bundles

Bundles compose multiple skills into workflows:

```bash
kask bundle run kata-pattern --bot Alice
```

The `kata-pattern` bundle routes to `kata-starter`, `kata-improvement`, or `kata-coaching` based on the bot's automaticity score.

---

## 3.4 Fusion — Multi-Model Deliberation for Skills

Skills are where Fusion has the greatest utility. When a skill invokes inference — for classification, analysis, generation, or critique — it benefits from multiple model perspectives producing higher-quality results.

### How It Works

When Fusion is enabled, skill inference calls automatically route through OpenRouter's Fusion pipeline:

1. The skill's prompt is sent to a **panel** of models in parallel (default: Kimi2.7, Qwen3.7 Max, GLM5.2, Minimax3)
2. Each panel model has `web_search` and `web_fetch` tools for research-backed responses
3. A **judge model** (default: deepseek-v4-pro) compares all responses and produces structured analysis
4. The analysis — consensus, contradictions, blind spots — feeds back into the skill's output

### Enabling Fusion

Fusion is opt-in. Enable it before running skills:

```bash
# Via environment (persists across sessions)
export HKASK_FUSION_JUDGE=deepseek-v4-pro
export HKASK_FUSION_PANEL="Kimi2.7,Qwen3.7 Max,GLM5.2,Minimax3"

# Via REPL (session only)
/fusion on
```

Verify with `/fusion` or `kask doctor`.

### When Fusion Helps Most

| Skill Type | Fusion Benefit |
|-----------|---------------|
| Research (`firecrawl-research`, `bug-hunt`) | Multiple models find different sources; synthesis catches what one model misses |
| Analysis (`mcda`, `superforecasting`, `scenario-builder`) | Diverse perspectives produce more calibrated judgments |
| Critique (`review`, `grill-me`, `skill-logic-audit`) | One model's blind spot is another model's catch |
| Generation (`logo-builder`, `qa-script-builder`) | Panel variety produces more creative and robust outputs |
| Classification (`classify`) | Consensus across panel models reduces misclassification |

### Cost

Expect roughly 4-5× the cost of a single completion. Fusion is for high-stakes skill execution where the cost of being wrong outweighs the cost of extra completions. Short tactical prompts don't trigger it — the model decides whether deliberation is warranted.

### Disabling

```bash
export HKASK_FUSION_OFF=1    # Env var
/fusion off                  # REPL command
```

---

## 4. Skill Catalog

### 4.1 Perceptual

Skills that transform *how the agent sees* — run before analysis.

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `dokkodo-mindset` | Musashi's 21 precepts as perceptual filter — clears attachment, preference, resentment, fear | "apply the Dokkodo", "warrior mindset" |
| `falstaffian-perspective` | Multi-iteration perspective generation through semantic shape transforms | "reframe this", "falstaffian take" |

### 4.2 Regulative

Skills that enforce boundaries and classify constraints.

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `constraint-forces` | Classify constraints as Prohibition/Guardrail/Guideline/Evidence/Hypothesis | "what force is this constraint?" |
| `magna-carta-verifier` | Verify Magna Carta principle compliance | "verify sovereignty" |

### 4.3 Analytic — Structural

Skills that evaluate, decompose, and minimize.

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `pragmatic-laziness` | 3-phase lazy loop for least-action pathfinding | "be lazy about this", "least action" |
| `pragmatic-semantics` | Epistemic discipline — classify statements by certainty | "how do you know that?" |
| `pragmatic-cybernetics` | CNS feedback loop analysis | "analyze feedback loops" |
| `essentialist` | 3-gate eliminative interrogation (Exist → Surface → Contract) | "simplify this", "what can be deleted" |
| `deep-module` | Ousterhout's module depth discipline with deletion test | "deepen this module" |
| `grill-me` | Socratic interrogation — stress-test understanding | "grill me about X" |

### 4.4 Analytic — Decision & Strategy

Skills that support structured choice, prediction, and planning.

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `mcda` | Multi-criteria decision analysis with compensation masking detection | "compare these options", "MCDA" |
| `decision-journal` | Kahneman-style decision recording with Brier score calibration | "journal this decision" |
| `superforecasting` | Tetlock 8-stage calibrated probability forecasting | "forecast this", "superforecast" |
| `scenario-builder` | Schwartz method scenario planning — STEEP, 2×2 matrix, robust strategies | "build scenarios", "explore futures" |
| `hypothesis-framer` | Research framing with FINER criteria + PICO process — question → hypothesis → aims | "frame my research", "write a hypothesis" |

### 4.5 Analytic — Extraction & Summarization

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `structured-extraction` | Schema-driven entity and relation extraction with coverage tracking | "extract structured data", "populate this schema" |
| `chain-of-density` | Gao et al. iterative density-increase summarization | "summarize this densely", "CoD this" |
| `caveman` | Ultra-compact style compression — drop filler, preserve substance | "caveman mode", "compress this" |
| `zoom-out` | Broader context and higher-level perspective on unfamiliar code | "zoom out", "bigger picture" |

### 4.6 Executive — Behavioral Guardrails

Skills that constrain HOW an agent works.

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `coding-guidelines` | Karpathy's four principles — think first, simplicity, surgical, goal-driven | "use coding-guidelines" |
| `tdd` | Red-green-refactor with contract grounding | "write tests first", "red-green-refactor" |
| `rust-expertise` | Idiomatic Rust via Graydon Hoare's design philosophy | "rust expertise", "idiomatic Rust" |

### 4.7 Executive — Diagnostics & Repair

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `diagnose` | Disciplined diagnosis loop — reproduce, anchor, hypothesize, instrument, fix | "diagnose this", "debug this" |
| `improve-codebase-architecture` | Find deepening opportunities, shallow modules, tight coupling | "improve architecture", "ball of mud" |
| `refactor-service-layer` | Strangler fig service extraction from CLI/API/MCP surfaces | "refactor service layer" |
| `strangler-fig` | Incremental architectural migration — new alongside old | "strangler fig", "migrate architecture" |

### 4.8 Executive — Security

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `adversarial-red-team` | Systematic red-teaming with ATLAS/GARAK taxonomy | "red-team this", "adversarial test" |

### 4.9 Executive — Creativity & Media

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `logo-builder` | Pragmatic logo design with Bokhua's five formal gates | "design a logo" |
| `improv` | Agent interaction grammar — Plussing, Yes And, Yes But, Riffing | "/improv" in REPL |

### 4.10 Meta-Cognition

Skills that evaluate the agent's own thinking and process.

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `review` | Self-critique reasoning outputs for contradictions and gaps | "review this" |
| `self-critique-revision` | Iterative draft → critique → revise cycle | "self-critique this" |
| `handoff` | Session handoff documentation for context preservation | "create handoff" |
| `condenser-continuation` | Resume work after context reset | "condenser continuation" |
| `goal-analysis` | Lightweight goal specification and completion verification | "create a goal to...", "goal analysis" |
| `gentle-lovelace` | 4-dimension writing quality evaluation (Hopper/Lovelace/Schriver/Gentle) | "evaluate this document", "gentle lovelace" |

### 4.11 Kata System (Capability Development)

Toyota Kata scientific thinking for agent improvement.

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `kata-starter` | Foundational habit practice — Five Questions Drill, PDCA, Observation | "kata starter" |
| `kata-improvement` | 4-step PDCA scientific pattern for capability gaps | "kata improvement" |
| `kata-coaching` | 5-question coaching dialogue | "kata coaching" |
| `kata` | Full system orchestration with CNS monitoring | "kata" |

### 4.12 Skill Management

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `skill-discovery` | Find and install skills | "find a skill for X" |
| `skill-manager` | CRUD for the skill corpus | "manage skills" |
| `skill-maintenance` | Audit skills for staleness, drift, and quality | "audit skills" |
| `skill-translator` | Convert skills between formats | "translate this skill" |
| `skill-logic-audit` | Audit template logic against stated goals | "audit template logic" |
| `skill-bundler` | Orchestrate multiple skills into a cohesive bundle | "bundle skills" |

### 4.13 Documentation

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `document-update` | 7-task documentation maintenance workflow | "update docs", "consolidate docs" |

### 4.14 Which Skill for What?

Don't know which skill you need? Find your problem:

| I need to... | Use |
|-------------|-----|
| Clear my head before making a decision | `dokkodo-mindset` |
| See a problem from multiple angles | `falstaffian-perspective` |
| Find the simplest path through a design | `pragmatic-laziness` |
| Know what can be deleted | `essentialist` |
| Stress-test my understanding | `grill-me` |
| Know which rules are inviolable | `constraint-forces` |
| Choose between multiple options | `mcda` |
| Predict a future outcome with calibration | `superforecasting` |
| Plan for multiple possible futures | `scenario-builder` |
| Frame a research question into a testable hypothesis | `hypothesis-framer` |
| Record a decision and check if I was right later | `decision-journal` |
| Track whether I achieved a specific goal | `goal-analysis` |
| Extract structured data from prose | `structured-extraction` |
| Summarize text densely without losing facts | `chain-of-density` |
| Compress prose to bare essentials | `caveman` |
| Get the bigger picture on unfamiliar code | `zoom-out` |
| Debug a failure systematically | `diagnose` |
| Hunt bugs in a crate — semantic errors, interaction bugs, contract gaps | `pragmatic-semantics` → `pragmatic-cybernetics` → `bug-hunt` |
| Find the root cause of an incident | `diagnose` → then `improve-codebase-architecture` |
| Refactor a messy codebase | `improve-codebase-architecture` → `refactor-service-layer` |
| Migrate from old code to new incrementally | `strangler-fig` |
| Write tests before code | `tdd` |
| Write idiomatic Rust | `rust-expertise` |
| Enforce coding discipline | `coding-guidelines` |
| Design a logo | `logo-builder` |
| Evaluate document quality | `gentle-lovelace` |
| Review my own reasoning for gaps | `review` |
| Revise a draft through critique cycles | `self-critique-revision` |
| Hand off work between sessions | `handoff` |
| Resume work after context reset | `condenser-continuation` |
| Red-team an agent's security | `adversarial-red-team` |
| Improve agent capability through practice | `kata-starter` → `kata-improvement` → `kata-coaching` |
| Update project documentation | `document-update` |
| Verify Magna Carta compliance | `magna-carta-verifier` |
| Audit or manage skills | `skill-maintenance`, `skill-manager`, `skill-discovery` |
| Verify sovereignty or consent boundaries | `magna-carta-verifier` |

---

## 5. Composition Patterns

Skills don't work in isolation. Here are the three most common chains:

### Pattern 1: Perception → Analysis → Action

```
dokkodo-mindset → pragmatic-laziness → essentialist → coding-guidelines
     ↑                                                    ↑
constraint-forces runs across all stages, never relaxed
```

**When:** You face a design decision, architecture problem, or code review. The Dokkodo clears your perceptual field. Pragmatic laziness finds the least-action path. Essentialist deletes what doesn't earn existence. Coding guidelines enforce discipline throughout.

### Pattern 2: Frame → Plan → Decide

```
hypothesis-framer → scenario-builder → mcda
```

**When:** You have a research idea and need to move from broad topic through hypothesis to strategic decisions. Hypothesis-framer narrows the topic with FINER criteria, structures it with PICO, and produces a testable hypothesis. Scenario-builder explores futures where the hypothesis plays out. MCDA ranks research design alternatives on weighted criteria.

**Example session:**
```
> frame my research on ventilator liberation in children with BPD
> (agent produces FINER scores, PICO-structured question, H₁/H₀, aims)
> build scenarios for this research program over the next 3 years
> mcda: compare cross-sectional vs longitudinal vs RCT designs
```

### Pattern 3: Forecast → Decide → Record → Verify

```
superforecasting → mcda → decision-journal → goal-analysis
```

**When:** You're making a consequential decision under uncertainty. Superforecasting produces calibrated probabilities. MCDA ranks alternatives on weighted criteria. The decision journal records the reasoning and schedules a revisit. Goal analysis tracks whether the outcome matches the prediction.

### Pattern 4: Diagnose → Extract → Fix → Harden

```
diagnose → structured-extraction → refactor-service-layer → adversarial-red-team
```

**When:** Something broke. Diagnose finds the bug. Structured extraction maps the incident to a root cause schema. Refactor-service-layer fixes it systematically. Adversarial red-team tests whether the fix holds under attack.

### Pattern 5: Explore → Summarize → Compress

```
zoom-out → chain-of-density → caveman
```

**When:** You need to understand a large unfamiliar codebase and communicate it concisely. Zoom out for context. Chain-of-density for maximum factual density. Caveman for final stylistic compression.

---

## 6. Skill Summary — All 46 Skills

| # | Skill | Category | Type | What it does |
|---|-------|----------|------|-------------|
| 1 | `adversarial-red-team` | Security | FlowDef | Red-team robustness via convergent adversarial probe loops |
| 2 | `caveman` | Extraction/Summarization | FlowDef | Ultra-compact prose compression via convergent compression loops |
| 3 | `chain-of-density` | Extraction/Summarization | FlowDef | Iterative density-increase summarization via convergent passes |
| 4 | `coding-guidelines` | Behavioral Guardrails | FlowDef | Karpathy's four principles via convergent assess→apply→verify loops |
| 5 | `condenser-continuation` | Meta-Cognition | FlowDef | Resume work after context reset via convergent continuation loops |
| 6 | `constraint-forces` | Regulative | FlowDef | Classify constraints by enforcement level via convergent force loops |
| 7 | `decision-journal` | Decision & Strategy | FlowDef | Kahneman decision journal with convergent calibration loops |
| 8 | `deep-module` | Structural Analysis | FlowDef | Ousterhout module depth with deletion test via convergent depth loops |
| 9 | `diagnose` | Diagnostics | FlowDef | Disciplined diagnosis loop with convergent PDCA exits |
| 10 | `document-update` | Documentation | FlowDef | 7-task doc maintenance workflow with convergent PDCA exits |
| 11 | `dokkodo-mindset` | Perceptual | FlowDef | Musashi's 21 precepts as convergent perceptual filter |
| 12 | `essentialist` | Structural Analysis | FlowDef | 3-gate eliminative interrogation as convergent elimination loop |
| 13 | `falstaffian-perspective` | Perceptual | FlowDef | Perspective-taking via convergent Falstaffian rotation loops |
| 14 | `gentle-lovelace` | Meta-Cognition | FlowDef | 4D writing-quality evaluation via convergent diagnostic loops |
| 15 | `goal-analysis` | Meta-Cognition | FlowDef | Goal lifecycle analysis with convergent completion loops |
| 16 | `grill-me` | Structural Analysis | FlowDef | Socratic interrogation via convergent challenge/assessment loops |
| 17 | `handoff` | Meta-Cognition | FlowDef | Session handoff documentation via convergent transfer loops |
| 18 | `improv` | Creativity | FlowDef | Agent interaction grammar |
| 19 | `improve-codebase-architecture` | Diagnostics | FlowDef | Find deepening opportunities via convergent architecture loops |
| 20 | `kata` | Kata System | Bundle | Full Toyota Kata orchestration |
| 21 | `kata-coaching` | Kata System | FlowDef | 5-question coaching dialogue via convergent coaching loops |
| 22 | `kata-improvement` | Kata System | FlowDef | 4-step improvement kata via convergent experiment loops |
| 23 | `kata-starter` | Kata System | FlowDef | Foundational scientific thinking drills via convergent starter loops |
| 24 | `logo-builder` | Creativity | FlowDef | Logo design with Bokhua's five gates |
| 25 | `magna-carta-verifier` | Regulative | FlowDef | Verify Magna Carta compliance via convergent verification loops |
| 26 | `mcda` | Decision & Strategy | FlowDef | Multi-criteria decision analysis via convergent robustness loops |
| 27 | `pragmatic-cybernetics` | Structural Analysis | FlowDef | CNS feedback loop analysis via convergent diagnostics loops |
| 28 | `pragmatic-laziness` | Structural Analysis | FlowDef | 3-phase lazy loop for least-action pathfinding with PDCA convergence |
| 29 | `pragmatic-semantics` | Structural Analysis | FlowDef | Epistemic statement classification via convergent semantics loops |
| 30 | `refactor-service-layer` | Diagnostics | FlowDef | Strangler fig service extraction via convergent verify loops |
| 31 | `review` | Meta-Cognition | FlowDef | Self-critique reasoning outputs via convergent review loops |
| 32 | `rust-expertise` | Behavioral Guardrails | FlowDef | Idiomatic Rust design principles via convergent audit/refactor loops |
| 33 | `scenario-builder` | Decision & Strategy | FlowDef | Schwartz method scenario planning via convergent scenario loops |
| 34 | `hypothesis-framer` | Decision & Strategy | FlowDef | FINER + PICO research framing via convergent PDCA — question → hypothesis → aims |
| 35 | `self-critique-revision` | Meta-Cognition | FlowDef | Iterative draft → critique → revise via convergent loops |
| 36 | `skill-bundler` | Skill Management | FlowDef | Orchestrate skills into bundles via convergent compose→validate loops |
| 37 | `skill-discovery` | Skill Management | FlowDef | Find and install skills via convergent gap→evaluate loops |
| 38 | `skill-logic-audit` | Skill Management | FlowDef | Audit template logic via bounded critique→proposal loops |
| 39 | `skill-maintenance` | Skill Management | FlowDef | Audit skills for staleness/coverage via convergent maintenance loops |
| 40 | `skill-manager` | Skill Management | FlowDef | Registry-first skill lifecycle management via convergent validation loops |
| 41 | `skill-translator` | Skill Management | FlowDef | Convert skills between formats via convergent translation loops |
| 42 | `strangler-fig` | Diagnostics | FlowDef | Incremental architectural migration with convergent step verification |
| 43 | `structured-extraction` | Extraction/Summarization | FlowDef | Schema-driven extraction via convergent entity/relation/map loops |
| 44 | `superforecasting` | Decision & Strategy | FlowDef | Tetlock 8-stage calibrated forecasting via convergent pipeline loops |
| 45 | `tdd` | Behavioral Guardrails | FlowDef | Contract-anchored red-green-refactor via convergent TDD loops |
| 46 | `zoom-out` | Extraction/Summarization | FlowDef | Broader context on unfamiliar code via convergent context loops |

---

## 7. Understanding Template Types

When a skill has registry templates, each `.j2` file is typed:

| Type | What It Does | Example |
|------|-------------|---------|
| **WordAct** | Produces text or structured output | `superforecasting/stage_7_record.j2` — creates forecast record |
| **KnowAct** | Reasons, classifies, evaluates, decides | `dokkodo-mindset/dokkodo-perceive.j2` — applies perceptual filter |
| **FlowDef** | Orchestrates WordAct/KnowAct in a convergent PDCA process | `registry/manifests/essentialist.yaml` — iterates gates via convergence + `loop` |

You rarely need to know the template type — the runtime dispatches correctly. But when debugging: WordAct = "what to say", KnowAct = "how to think", FlowDef = "what to do".

Note: `.j2` templates use `template_type: WordAct|KnowAct`. FlowDef orchestration is declared in skill process manifests under `registry/manifests/`.

---

## 8. Visibility and P11

Every skill has a visibility field:

| Visibility | Meaning |
|------------|---------|
| `Public` | Discoverable and usable by all agents and users |
| `Private` | Only usable by the owning replicant or namespace |

P11 (Digital Public/Private Sphere) governs this. You control what is shared via explicit consent boundaries.

---

## 9. Checking Skill Health

### 9.1 CNS Health

```bash
kask cns health
```

Shows CNS span health for all active skills.

### 9.2 Schema Validation

```bash
cargo test -p hkask-templates yaml_schema_validation
```

Validates all manifest YAML files. Run after installing new skills.

### 9.3 Contract Audit

Contracts use `expect:` + `[P{N}]` annotations. Verify coverage with:

```bash
# Count functions with principle grounding
grep -rn "/// \[P[0-9]*\]" crates/ --include="*.rs" | wc -l

# Check expect: field presence
grep -rn "/// expect:" crates/ --include="*.rs" | wc -l
```

Run the contract-generator (contract-generator/contract-generator.j2) for any gaps.

---

## 10. Troubleshooting

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| Skill doesn't appear in `kask skill list` | Not registered or visibility mismatch | Check `visibility` field and registry bootstrap |
| "Template render failed" | `.j2` has prohibited `template_type` | Must be `WordAct` or `KnowAct` (not `FlowDef`) |
| "Permission denied" | Visibility mismatch (P11) | Check `visibility` in SKILL.md frontmatter |
| Bundle doesn't compose | Constituent skill score < threshold | Calibrate constituent skills first |
| Two skills with same name | Duplicate installation | Use `kask skill list` to identify and prune |

### 10.1 Migration Assumptions Currently Encoded in Skill Rails

The following assumptions were used during the recent FlowDef migration work and are now reflected in several skill manifests. These are **implementation assumptions**, not immutable design law.

| Assumption | Applied default |
|-----------|-----------------|
| Convergence metric semantics | `0.0 = converged`, `1.0 = far from converged` |
| Convergence gate | `improvement_gate: threshold_only` |
| Improvement ratio | `improvement_ratio: 0.10` in most migrated skills |
| Iteration budget | `max_iterations: 3` default; `4` used for deeper audit flows (for example `skill-logic-audit`) |
| Threshold band | Generally `0.10–0.15` based on skill criticality and migration consistency |
| Gas budgeting | Set from step-level budgets plus modest loop/overhead margin (not empirically calibrated) |
| Wiring discipline | Prefer `step_n_result` / `step_n_populated`; avoid legacy `ordinal_` references |
| Dispatch discipline | Avoid dynamic `template_ref` in skill manifests; prefer stable template ids |

Concrete rails currently visible in core meta-skill manifests:

| Skill | Threshold | Improvement ratio | Gas cap |
|------|----------:|------------------:|--------:|
| `skill-manager` | `0.15` | `0.10` | `18000` |
| `skill-maintenance` | `0.15` | `0.10` | `16000` |
| `skill-discovery` | `0.15` | `0.10` | `16000` |
| `skill-bundler` | `0.10` | `0.10` | `16000` |
| `skill-translator` (`skill-translation` manifest) | `0.15` | `0.10` | `14000` |
| `skill-logic-audit` | `0.10` | `0.10` | `12000` |
| `improv` | `0.12` | `0.10` | `12000` |

Known model exceptions/tensions that still require explicit policy decisions:

- `kata` is currently represented as `Bundle` in this guide, not a single FlowDef skill manifest.
- `logo-builder` is documented as FlowDef capability via media orchestration templates, without a dedicated `registry/manifests/logo-builder.yaml` skill manifest.

---

## References

- [Skill Designer Guide](../../docs/guides/skill-designer-guide.md) — Creating and maintaining skills
- [PRINCIPLES.md](../architecture/core/PRINCIPLES.md) — P1–P12 principles
- [AGENTS.md](../../AGENTS.md) — Agent operating guide
- [dokkodo-user-guide.md](dokkodo-user-guide.md) — Using the Dokkodo mindset skill
