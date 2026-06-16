# Task 2 — Rational Reconstruction of the hKask Skill Model

**Date:** 2026-06-16  
**Classification:** Prescriptive (OUGHT) — derived from runtime code, Magna Carta P1–P4, and Testing Discipline §9.2.

---

## 1. The Recursive Dynamics

hKask skills are a **dual-layer, recursive composition system**:

| Layer | Artifact | Role | Loaded By | Executed By |
|-------|----------|------|-----------|-------------|
| **Zed (companion)** | `.agents/skills/<name>/SKILL.md` | Teaches the coding agent when and how to use the skill | `SkillLoader` (frontmatter only) | **Not executed** at hKask runtime; injected into the LLM conversation in Zed |
| **Registry (runtime)** | `registry/templates/<name>/manifest.yaml` + `*.j2` | Provides executable templates and process manifests | `Registry`/`SqliteRegistry` | `ManifestExecutor` renders `.j2` and executes `.yaml` `FlowDef` |

### Atomic template types

```text
WordAct  = .j2 template that produces speech/action output — "what to say"
KnowAct  = .j2 template that produces reasoning/evaluation output — "how to think"
FlowDef  = .yaml BundleManifest that orchestrates WordAct/KnowAct calls — "what to do"
```

A `.j2` file is **never** a `FlowDef`. A `FlowDef` is **never** a `.j2` file.

### Primary skill

A **primary skill** is a focused capability that has:

1. A `SKILL.md` companion guide (Zed layer).
2. A registry crate with at least one `WordAct` or `KnowAct` `.j2` and an optional `FlowDef` `.yaml`.
3. A valid `manifest.yaml` describing the crate.
4. `template_type` values that match the runtime type system.
5. hLexicon terms that exist in `registry/hlexicon/hlexicon-workspace.yaml`.
6. Typed `contract.input` and `contract.output` blocks that match the template body.

### Bundle

A **bundle** is a **curated composition of calibrated primary skills**. It is expressed as a `BundleManifest` (`.yaml`) and may:

- declare which primary skills it contains (`skills:`)
- specify ordering, gas budgets, OCAP requirements, and convergence
- be loaded into the registry via `Registry::register_bundle()` or SQLite

A bundle is **not** a separate template type. It does not introduce new `.j2` files unless those `.j2` files are legitimate WordAct/KnowAct templates that belong to a primary skill.

---

## 2. Decision Table: Skill or Bundle?

| Proposed artifact | Has SKILL.md body only? | Has .j2 WordAct/KnowAct? | Has .yaml FlowDef? | Composes ≥2 primary skills? | Classification |
|-------------------|-------------------------|--------------------------|--------------------|----------------------------|----------------|
| SKILL.md with no registry crate | Yes | No | No | No | **Incomplete skill** — add registry layer |
| Registry crate with no SKILL.md | No | Yes | Maybe | No | **Incomplete skill** — add Zed layer |
| Single `.j2` prompt/cognition + manifest | Yes | Yes | No | No | **Primary skill** |
| Single `.yaml` process + manifest | Yes | Maybe | Yes | No | **Primary skill** (the FlowDef is the runtime entry) |
| `.yaml` process referencing 2+ skills | Yes | Maybe | Yes | Yes | **Bundle** — only valid if all referenced skills are `active` |
| `.j2` file declaring `FlowDef` | — | No | No (wrong format) | No | **Invalid** — delete or convert to `.yaml` |
| `.j2` wrapper that only chains other skills | Yes | Yes (but no new logic) | No | Yes | **Premature bundle** — delete; re-express as `.yaml` bundle when primaries are active |

---

## 3. File-Format Rules

| File extension | Allowed `template_type` | Allowed role | Example |
|----------------|-------------------------|--------------|---------|
| `.j2` | `WordAct`, `KnowAct` only | Prompt/cognition template | `coding-guidelines/guidelines-assess.j2` |
| `.yaml` (manifest) | `FlowDef` only (by file kind) | Process manifest / bundle | `registry/manifests/tdd.yaml` |
| `.yaml` (bootstrap) | `WordAct`, `KnowAct`, `FlowDef` | `RegistryEntry` records for bootstrap | `registry/templates/bootstrap-registry.yaml` |

---

## 4. Calibration Gate

A skill may be declared `active` only when:

1. Both Zed and registry layers exist.
2. Every `.j2` has `template_type ∈ {WordAct, KnowAct}`.
3. Every `.j2` has valid `visibility`, `energy_cap` in [1024, 16384], and typed contract.
4. Every `template_type` alias from DDMVSS (`Cognition`, `Prompt`, `Process`) has been replaced by runtime names.
5. All `lexicon_terms` and `hlexicon_terms` exist in the workspace lexicon.
6. `manifest.yaml` frontmatter aligns with `SKILL.md` description.
7. Health score ≥ 0.8 per `skill-maintenance`.

A bundle may be registered only when:

1. It is a `.yaml` `BundleManifest`.
2. Every referenced primary skill is `active`.
3. Its `steps` call templates by ID rather than duplicating their logic.
4. It does not add new `.j2` template types.

---

## 5. Migration Path for Current Corpus

1. **Delete** all `.j2` files declaring `FlowDef` and all `.j2` wrappers that only chain other skills (pre-release: direct delete, git is revert).
2. **Delete** bundle manifests in `registry/manifests/` whose constituent skills are not active.
3. **Convert** legitimate process ideas into `.yaml` `BundleManifest` files once the underlying WordAct/KnowAct templates exist and are active.
4. **Calibrate** primary skills to health ≥ 0.8 before reintroducing bundles.
5. **Sync** `bootstrap-registry.yaml` only from calibrated, active registry crates.
