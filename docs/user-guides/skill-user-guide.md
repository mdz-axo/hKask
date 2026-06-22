---
title: "hKask Skill User Guide — Discovering and Using Agent Skills"
audience: [developers, operators, curators]
last_updated: 2026-06-19
version: "0.30.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# hKask Skill User Guide

**Purpose:** How to discover, load, and use skills in hKask. Covers the `kask skill` CLI, skill list, activation triggers, the dual-layer runtime model, and a catalog of all 45 available skills organized by functional category.

**Companion doc:** [`skill-designer-guide.md`](../../docs/guides/skill-designer-guide.md) — for creating and maintaining skills.

---

## 1. What Are Skills?

Skills are composable agent capabilities. They teach the Zed coding agent domain knowledge and provide runtime-executable templates for the hKask engine. Skills live in two layers:

| Layer | You interact with it via... | Example |
|-------|---------------------------|---------|
| **Zed agent layer** (SKILL.md) | The `skill` tool in Zed — "Use coding-guidelines" | Teaches the agent Karpathy's four coding principles |
| **Registry template layer** (`.j2` + `manifest.yaml`) | The `kask` CLI and runtime engine | Renders a KnowAct template to assess code quality |

A skill may have one or both layers. You don't need to know which — the system handles routing.

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

Of the ~74 registry crates in `registry/templates/`, 45 have SKILL.md companions in `.agents/skills/`. The remaining ~29 are **infrastructure crates** — runtime dispatch, storage, monitoring, and orchestration templates that run the system rather than perform user-facing tasks. A skill with only a registry crate (`.j2` + `manifest.yaml`) is runtime-complete — the cascade can execute it. The SKILL.md is a **generated companion** for the Zed coding agent during development. When both exist, the registry crate is authoritative. See `hKask-architecture-master.md` Pattern A for the full derivation rule.

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

### Pattern 2: Forecast → Decide → Record → Verify

```
superforecasting → mcda → decision-journal → goal-analysis
```

**When:** You're making a consequential decision under uncertainty. Superforecasting produces calibrated probabilities. MCDA ranks alternatives on weighted criteria. The decision journal records the reasoning and schedules a revisit. Goal analysis tracks whether the outcome matches the prediction.

### Pattern 3: Diagnose → Extract → Fix → Harden

```
diagnose → structured-extraction → refactor-service-layer → adversarial-red-team
```

**When:** Something broke. Diagnose finds the bug. Structured extraction maps the incident to a root cause schema. Refactor-service-layer fixes it systematically. Adversarial red-team tests whether the fix holds under attack.

### Pattern 4: Explore → Summarize → Compress

```
zoom-out → chain-of-density → caveman
```

**When:** You need to understand a large unfamiliar codebase and communicate it concisely. Zoom out for context. Chain-of-density for maximum factual density. Caveman for final stylistic compression.

---

## 6. Skill Summary — All 45 Skills

| # | Skill | Category | Type | What it does |
|---|-------|----------|------|-------------|
| 1 | `adversarial-red-team` | Security | KnowAct | Red-team agent outputs with ATLAS/GARAK taxonomy |
| 2 | `caveman` | Extraction/Summarization | WordAct | Ultra-compact prose compression |
| 3 | `chain-of-density` | Extraction/Summarization | KnowAct | Iterative density-increase summarization (Gao et al.) |
| 4 | `coding-guidelines` | Behavioral Guardrails | KnowAct | Karpathy's four coding principles |
| 5 | `condenser-continuation` | Meta-Cognition | KnowAct | Resume work after context reset |
| 6 | `constraint-forces` | Regulative | KnowAct | Classify constraints by enforcement level |
| 7 | `decision-journal` | Decision & Strategy | KnowAct | Kahneman decision journal with Brier scoring |
| 8 | `deep-module` | Structural Analysis | KnowAct | Ousterhout module depth with deletion test |
| 9 | `diagnose` | Diagnostics | KnowAct | Disciplined diagnosis loop |
| 10 | `document-update` | Documentation | KnowAct | 7-task doc maintenance workflow |
| 11 | `dokkodo-mindset` | Perceptual | KnowAct | Musashi's 21 precepts as perceptual filter |
| 12 | `essentialist` | Structural Analysis | KnowAct | 3-gate eliminative interrogation |
| 13 | `falstaffian-perspective` | Perceptual | KnowAct | Semantic shape transforms for perspective-taking |
| 14 | `gentle-lovelace` | Meta-Cognition | KnowAct | 4D writing quality evaluation |
| 15 | `goal-analysis` | Meta-Cognition | KnowAct | Goal specification and completion verification |
| 16 | `grill-me` | Structural Analysis | KnowAct | Socratic interrogation |
| 17 | `handoff` | Meta-Cognition | KnowAct | Session handoff documentation |
| 18 | `improv` | Creativity | FlowDef | Agent interaction grammar |
| 19 | `improve-codebase-architecture` | Diagnostics | KnowAct | Find deepening opportunities |
| 20 | `kata` | Kata System | Bundle | Full Toyota Kata orchestration |
| 21 | `kata-coaching` | Kata System | KnowAct | 5-question coaching dialogue |
| 22 | `kata-improvement` | Kata System | KnowAct | 4-step PDCA scientific pattern |
| 23 | `kata-starter` | Kata System | KnowAct | Foundational scientific thinking habits |
| 24 | `logo-builder` | Creativity | FlowDef | Logo design with Bokhua's five gates |
| 25 | `magna-carta-verifier` | Regulative | KnowAct | Verify Magna Carta compliance |
| 26 | `mcda` | Decision & Strategy | KnowAct | Multi-criteria decision analysis with masking detection |
| 27 | `pragmatic-cybernetics` | Structural Analysis | KnowAct | CNS feedback loop analysis |
| 28 | `pragmatic-laziness` | Structural Analysis | KnowAct | 3-phase lazy loop for least-action pathfinding |
| 29 | `pragmatic-semantics` | Structural Analysis | KnowAct | Epistemic statement classification |
| 30 | `refactor-service-layer` | Diagnostics | KnowAct | Strangler fig service extraction |
| 31 | `review` | Meta-Cognition | KnowAct | Self-critique reasoning outputs |
| 32 | `rust-expertise` | Behavioral Guardrails | KnowAct | Idiomatic Rust design principles |
| 33 | `scenario-builder` | Decision & Strategy | KnowAct | Schwartz method scenario planning |
| 34 | `self-critique-revision` | Meta-Cognition | KnowAct | Iterative draft → critique → revise |
| 35 | `skill-bundler` | Skill Management | KnowAct | Orchestrate skills into bundles |
| 36 | `skill-discovery` | Skill Management | KnowAct | Find and install skills |
| 37 | `skill-logic-audit` | Skill Management | KnowAct | Audit template logic |
| 38 | `skill-maintenance` | Skill Management | KnowAct | Audit skills for staleness and quality |
| 39 | `skill-manager` | Skill Management | KnowAct | CRUD for the skill corpus |
| 40 | `skill-translator` | Skill Management | KnowAct | Convert skills between formats |
| 41 | `strangler-fig` | Diagnostics | KnowAct | Incremental architectural migration |
| 42 | `structured-extraction` | Extraction/Summarization | KnowAct | Schema-driven entity and relation extraction |
| 43 | `superforecasting` | Decision & Strategy | WordAct | Tetlock 8-stage calibrated forecasting |
| 44 | `tdd` | Behavioral Guardrails | KnowAct | Red-green-refactor with contract grounding |
| 45 | `zoom-out` | Extraction/Summarization | KnowAct | Broader context on unfamiliar code |

---

## 7. Understanding Template Types

When a skill has registry templates, each `.j2` file is typed:

| Type | What It Does | Example |
|------|-------------|---------|
| **WordAct** | Produces text or structured output | `superforecasting/stage_7_record.j2` — creates forecast record |
| **KnowAct** | Reasons, classifies, evaluates, decides | `dokkodo-mindset/dokkodo-perceive.j2` — applies perceptual filter |
| **FlowDef** | Orchestrates WordAct/KnowAct in a pipeline | `essentialist/essentialist-flow.j2` — iterates G1→G2→G3 gates |

You rarely need to know the template type — the runtime dispatches correctly. But when debugging: WordAct = "what to say", KnowAct = "how to think", FlowDef = "what to do".

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

---

## References

- [Skill Designer Guide](../../docs/guides/skill-designer-guide.md) — Creating and maintaining skills
- [PRINCIPLES.md](../architecture/core/PRINCIPLES.md) — P1–P12 principles
- [AGENTS.md](../../AGENTS.md) — Agent operating guide
- [dokkodo-user-guide.md](dokkodo-user-guide.md) — Using the Dokkodo mindset skill
