---
title: "hKask Skill User Guide — Discovering and Using Agent Skills"
audience: [developers, operators, curators]
last_updated: 2026-06-16
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# hKask Skill User Guide

**Purpose:** How to discover, load, and use skills in hKask. Covers the `kask skill` CLI, skill list, activation triggers, and the dual-layer runtime model.

**Companion doc:** [`skill-designer-guide.md`](../../docs/guides/skill-designer-guide.md) — for creating and maintaining skills.

---

## 1. What Are Skills?

Skills are composable agent capabilities. They teach the Zed coding agent domain knowledge and provide runtime-executable templates for the hKask engine. Skills live in two layers:

| Layer | You interact with it via... | Example |
|-------|---------------------------|---------|
| **Zed agent layer** (SKILL.md) | The `skill` tool in Zed — "Use coding-guidelines" | Teaches the agent Karpathy's four coding principles |
| **Registry template layer** (`.j2` + `manifest.yaml`) | The `kask` CLI and runtime engine | Renders a KnowAct template to assess code quality |

A skill may have one or both layers. You don't need to know which — the system handles routing.

---

## 2. Discovering Skills

### 2.1 List All Skills

```bash
kask skill list
```

Output shows name, visibility, description, and activation trigger for every installed skill:

```
Name                    Visibility   Description
coding-guidelines       Public       Enforce Karpathy's four coding behavioral principles...
caveman                 Public       Compress a draft response into ultra-compact caveman mode...
kata-starter            Public       Toyota Kata Starter practice routines for building...
```

### 2.2 Show Skill Details

```bash
kask skill show coding-guidelines
```

Displays the full SKILL.md content, template inventory, CNS spans, and contract signatures.

### 2.3 Filter by Visibility

```bash
kask skill list --visibility Public     # Shared/discoverable skills
kask skill list --visibility Private    # Your personal/namespace skills
```

### 2.4 Discover via CNS

```bash
kask cns spans --filter skill     # Show all skill-related CNS spans
```

---

## 3. Activating a Skill

### 3.1 In Zed (Agent Layer)

Skills activate when a Zed agent's prompt matches the skill's `description` trigger. You can also explicitly invoke:

```
Use the coding-guidelines skill before reviewing this PR.
```

The Zed agent loads `SKILL.md`, absorbs the procedural knowledge, and applies it to the task. No runtime template is rendered — this is pure agent instruction.

### 3.2 Via the kask CLI (Registry Layer)

Skills with registry templates can be invoked directly:

```bash
kask skill invoke coding-guidelines/guidelines-assess --input task_description="review auth module"
```

This renders the `.j2` KnowAct template with your input, executes it through the inference router, and returns the structured assessment.

### 3.3 Via Bundles

Bundles compose multiple skills into workflows:

```bash
kask bundle run kata-pattern --bot Alice
```

The `kata-pattern` bundle routes to `kata-starter`, `kata-improvement`, or `kata-coaching` based on the bot's current automaticity score.

---

## 4. Skill Categories

### 4.1 Behavioral Guardrails

Skills that constrain HOW an agent works, not WHAT it builds:

| Skill | Purpose |
|-------|---------|
| `coding-guidelines` | Karpathy's four coding principles |
| `essentialist` | Delete-before-add, three-gate challenge loop |
| `deep-module` | Ousterhout's module depth discipline |

### 4.2 Kata System (Capability Development)

Toyota Kata scientific thinking for agent improvement:

| Skill | Type | Purpose |
|-------|------|---------|
| `kata-starter` | Primary | Foundational habit practice (Type 1) |
| `kata-improvement` | Primary | 4-step PDCA scientific pattern (Type 4) |
| `kata-coaching` | Primary | 5-question dialogue (Type 4) |
| `kata` | Bundle | Full system orchestration with CNS monitoring |

### 4.3 Composition & Orchestration

| Skill | Purpose |
|-------|---------|
| `pragmatic-laziness` | Composes 5 sub-skills for least-action pathfinding |
| `skill-bundler` | Orchestrates multiple skills into a cohesive bundle |
| `improv` | Agent interaction grammar (Plussing, Yes And, etc.) |

### 4.4 Domain Expertise

| Skill | Domain |
|-------|--------|
| `rust-expertise` | Idiomatic Rust design (Graydon Hoare philosophy) |
| `diagnose` | Disciplined debugging loop |
| `refactor-service-layer` | Strangler fig service extraction |
| `tdd` | Red-green-refactor with contract grounding |

### 4.5 Meta-Cognition

| Skill | Purpose |
|-------|---------|
| `review` | Self-critique reasoning outputs |
| `self-critique-revision` | Iterative critique-revise cycle |
| `handoff` | Session handoff documentation |
| `condenser-continuation` | Resume work after context reset |

### 4.6 Skill Management

| Skill | Purpose |
|-------|---------|
| `skill-discovery` | Find and install skills |
| `skill-manager` | CRUD for the skill corpus |
| `skill-maintenance` | Audit skills for staleness and quality |
| `skill-translator` | Convert skills between formats |
| `skill-logic-audit` | Audit template logic against stated goals |

### 4.7 Governance

| Skill | Purpose |
|-------|---------|
| `magna-carta-verifier` | Verify Magna Carta principle compliance |
| `constraint-forces` | Classify constraints by enforcement level |
| `pragmatic-semantics` | Epistemic discipline for statement classification |
| `pragmatic-cybernetics` | CNS feedback loop analysis |

---

## 5. Understanding Template Types

When a skill has registry templates, each `.j2` file is typed:

| Type | What It Does | Example |
|------|-------------|---------|
| **WordAct** | Produces text or structured output | `kata-coaching/coaching-q1-target.j2` — asks Question 1 |
| **KnowAct** | Reasons, classifies, evaluates, decides | `coding-guidelines/guidelines-assess.j2` — assesses task against principles |
| **FlowDef** | Orchestrates WordAct/KnowAct templates | `essentialist/essentialist-flow.j2` — iterates G1→G2→G3 gates |

You rarely need to know the template type — the runtime dispatches correctly. But when debugging: WordAct = "what to say", KnowAct = "how to think", FlowDef = "what to do".

---

## 6. Visibility and P11

Every skill has a visibility field:

| Visibility | Meaning |
|------------|---------|
| `Public` | Discoverable and usable by all agents and users |
| `Private` | Only usable by the owning replicant or namespace |

P11 (Digital Public/Private Sphere) governs this. You control what is shared via explicit consent boundaries. No skill is loaded without matching visibility scope.

---

## 7. Checking Skill Health

### 7.1 CNS Health

```bash
kask cns health
```

Shows CNS span health for all active skills. Red spans indicate missing or malformed declarations.

### 7.2 Schema Validation

```bash
cargo test -p hkask-templates yaml_schema_validation
```

Validates all manifest YAML files. Run this after installing new skills.

### 7.3 Contract Audit

```bash
scripts/contract-audit.sh --summary
```

Checks that template contracts are complete and consistent with their manifests.

---

## 8. Troubleshooting

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| Skill doesn't appear in `kask skill list` | Not registered in bootstrap | Check `registry/templates/bootstrap-registry.yaml` |
| "CNS span not found" | SKILL.md references a non-canonical span | Use only spans from `crates/hkask-types/src/cns.rs` |
| "Template render failed" | `.j2` has prohibited `template_type` | Must be `WordAct` or `KnowAct` (not `FlowDef`) |
| "Permission denied" | Visibility mismatch (P11) | Check `visibility` field in skill SKILL.md frontmatter |
| Bundle doesn't compose | Constituent skill score < 0.8 | Calibrate constituent skills first |

---

## References

- [Skill Designer Guide](../../docs/guides/skill-designer-guide.md) — Creating and maintaining skills
- [Dual-Layer Skill Model](../architecture/core/skill-dual-layer-model.md) — Architecture specification
- [PRINCIPLES.md](../architecture/core/PRINCIPLES.md) — P1–P12 principles
- [AGENTS.md](../../AGENTS.md) — Agent operating guide
