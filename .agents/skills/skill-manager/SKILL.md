---
name: skill-manager
visibility: public
description: "CRUD for the skill corpus. Registry crate (manifest.yaml + *.j2) is the canonical source of truth; SKILL.md is a generated companion. List, validate, build, install, and prune skills."
---

# Skill Manager

Manage hKask's skill architecture per **P5.1 — Single Source of Truth for Skills** (see `docs/architecture/core/PRINCIPLES.md`). Every skill has one canonical source: its **registry crate**. The SKILL.md is a generated companion, not a co-equal artifact.

| Artifact | Location | Role | Authoritative? |
|----------|----------|------|---------------|
| Registry crate | `registry/templates/<name>/manifest.yaml` + `*.j2` | Canonical source of truth — what `ManifestExecutor` drives | **Yes** |
| SKILL.md | `.agents/skills/<name>/SKILL.md` | Generated companion for Zed coding agent during development | **No** — derived from registry |

A skill is **complete** when its registry crate exists and is valid. The SKILL.md is optional for runtime correctness. A skill with only a SKILL.md (no registry) is **incomplete** — it cannot execute in the cascade.

**Skill struct** (`hkask-types/src/ports/mod.rs`): `id`, `domain` (TemplateType), `word_act`, `flow_def`, `know_act`, `polarity`, `content_hash`, `visibility` (Private/Public/Shared), `zone` (Private/Public), `namespace`.

**TemplateType** (`crates/hkask-types/src/template_type.rs`): `WordAct` | `KnowAct` | `FlowDef` only. DDMVSS names (Cognition, Prompt, Process) are spec aliases — **never** use them in .j2 frontmatter.

**Visibility**: `Private` | `Public` | `Shared` (in .j2 `[inference]` frontmatter and `Skill.visibility`).

**SkillZone**: `Private` → `.agents/skills/`, `Public` → `skills/`.

## Operations

### List Skills

Scan the registry (canonical source). For each skill name, report:

```
Skills (N total, R with registry, G with generated SKILL.md):
  [name]  [Registry]  [SKILL.md]  [status]  [description excerpt]
  ...
```

Steps:
1. Scan `registry/templates/*/manifest.yaml` → collect canonical skill names
2. Scan `.agents/skills/*/SKILL.md` → collect companion names
3. For each registry skill: mark registry ✓, SKILL.md ✓/✗, status = `complete` (registry exists)
4. Flag skills with SKILL.md but no registry as `incomplete — needs registry crate`

### Validate Skills

Check registry first (authoritative), then SKILL.md (companion).

**Registry layer (canonical — manifest.yaml + *.j2):**

| ID | Check | Pass Criteria |
|----|-------|---------------|
| R1 | manifest.yaml exists | File present in `registry/templates/<name>/` |
| R2 | Crate metadata valid | `crate.name`, `crate.version`, `crate.description` all present |
| R3 | Templates list non-empty | At least one entry in `templates` array |
| R4 | Template entry valid | Each entry has `id`, `path`, `type`, `description` |
| R5 | template_type is valid | Each `.j2` frontmatter `template_type` in {WordAct, KnowAct, FlowDef} |
| R6 | Visibility is valid | Each `.j2` frontmatter `visibility` in {Private, Public, Shared} |
| R7 | Contract valid | Each `.j2` has `contract.input` and `contract.output` with typed fields |
| R8 | energy_cap in range | Each `.j2` `energy_cap` is an integer in [2048, 8192] |
| R9 | .j2 file exists | Each template entry's `path` resolves to an actual `.j2` file |
| R10 | [inference] frontmatter valid | Each `.j2` starts with `[inference]` block |
| R11 | vocabulary terms exist | All `lexicon_terms` reference terms in known vocabulary |
| R12 | Jinja2 body present | Each `.j2` has template body after `---` separator |

**SKILL.md layer (generated companion):**

| ID | Check | Pass Criteria |
|----|-------|---------------|
| Z1 | SKILL.md exists | File present in `.agents/skills/<name>/` — informational |
| Z2 | Frontmatter valid | `---` delimiters with `name` and `description` fields |
| Z3 | Name matches directory | `name` field == directory basename, lowercase-hyphenated |
| Z4 | Description valid | Present, 1–1024 chars |
| Z5 | Body non-empty | Instructional content present |
| Z6 | No Magna Carta violations | No instructions that bypass user sovereignty or consent |
| Z7 | Headless compliance | No visual UI, Grafana, dashboards |
| Z8 | No deprecated markers | No `todo!`, `unimplemented!`, `#[deprecated]` |
| Z9 | Derived from registry | SKILL.md methodology aligns with registry templates; registry is authoritative when they disagree |

**Cross-artifact consistency:**

| ID | Check | Pass Criteria | Severity |
|----|-------|---------------|----------|
| X1 | Registry exists | manifest.yaml present | **Critical** — cannot execute without registry |
| X2 | Name consistency | SKILL.md name == manifest crate.name == directory name | Medium |
| X3 | No orphan SKILL.md | SKILL.md without registry → incomplete | High |
| X4 | No drift | SKILL.md does not claim behaviors registry templates do not support | Medium |

Severity levels: **Critical** (registry missing, invalid template_type), **High** (orphan SKILL.md, contract invalidity), **Medium** (drift, name mismatch, vocabulary gaps), **Info** (missing SKILL.md, description brevity).

### Build a Skill

Build the registry crate first (canonical), then generate SKILL.md from it.

1. **Confirm scope**: Project-local (default) or global
2. **Choose name**: User confirms. Lowercase-hyphenated, 2–40 chars. No `hkask-`, `cns-`, `mcp-` prefixes.
3. **Derive vocabulary terms**: From description, pick 3–8 terms from the known vocabulary (`crates/hkask-templates/src/vocabulary.rs` `KNOWN_TERMS`)
4. **Create registry crate** — `registry/templates/<name>/`:
   - `manifest.yaml`:
     ```yaml
     crate:
       name: <name>
       version: "0.28.0"
       description: >
         <one-paragraph description of what this skill does at runtime>

     templates:
       - id: <name>/<name>-<verb>
         path: <name>-<verb>.j2
         type: KnowAct
         lexicon_terms: [<term1>, <term2>, ...]
         description: >
           <what this template produces>

     vocabulary_terms:
       - <term1>
       - <term2>
       ...
     ```
   - At least one `.j2` template with valid `[inference]` frontmatter and contract.
5. **Generate SKILL.md** — `.agents/skills/<name>/SKILL.md`:
   - Derived from `manifest.yaml` (`crate.description`, template entries) and `.j2` body content
   - Frontmatter: `name` from manifest `crate.name`, `description` from manifest `crate.description`
   - Body: `## When to Use` from template descriptions, `## Instructions` from `.j2` system prompt body
6. **Validate**: Run full validation (R1–R12, Z1–Z9, X1–X4)
7. **Confirm**: Show registry crate to user for review; note that SKILL.md is generated

### Install a Skill

Install from an external source:

1. **Source is a registry crate**: Copy `registry/templates/<name>/`, then generate SKILL.md from it
2. **Source is dual-layer (old format)**: Copy both, treat registry as authoritative, regenerate SKILL.md to eliminate drift
3. **Source is SKILL.md only**: Use `skill-manager-translate` template to create registry crate, then install
4. **Validate** after installation (R1–R12, X1–X4)
5. **Verify**: Skill is discoverable via registry index

### Prune a Skill

**Soft prune** (deprecate without deleting):

| Artifact | Action |
|----------|--------|
| Registry | Set `visibility: Private` on all .j2 templates; add `deprecated: true` to manifest.yaml |
| SKILL.md | Add `disable-model-invocation: true` to frontmatter; add deprecation notice |

**Hard prune** (delete):

1. Confirm with user — **irreversible**
2. Delete `registry/templates/<name>/` (canonical source)
3. Delete `.agents/skills/<name>/` (generated companion)
4. Recovery via git is possible if version-controlled

### Stats

Report skill corpus statistics:

| Metric | What to Report |
|--------|---------------|
| Total skills | Count of registry crates (canonical count) |
| SKILL.md coverage | Registry skills with / without generated SKILL.md |
| Orphan SKILL.md | SKILL.md files with no registry crate |
| Template type distribution | Count of WordAct / KnowAct / FlowDef .j2 templates |
| Visibility distribution | Private / Public / Shared across .j2 templates |
| Vocabulary coverage | Unique terms used vs total known terms |
| Staleness | Broken references, missing .j2 files, invalid template_types |

## Decision Guide

| User Request | Operation |
|-------------|-----------|
| "List skills" / "Show skills" | List |
| "Validate skills" / "Check skills" | Validate |
| "Create a skill" / "New skill" | Build |
| "Install a skill" / "Add a skill" | Install |
| "Remove a skill" / "Delete a skill" | Hard prune (confirm) |
| "Deprecate a skill" | Soft prune |
| "Skill stats" / "How many skills" | Stats |
| "Generate SKILL.md for X" | Reverse-translate registry → SKILL.md |
| "Find a skill for X" | Delegate to `skill-discovery` |
| "Is this skill stale?" | Delegate to `skill-maintenance` |
| "Translate this skill" | Use `skill-manager-translate` template |
| "Bundle skills" | Delegate to `skill-bundler` |

## Safety

- **Never** delete a skill without user confirmation
- **Never** modify a skill's registry crate without telling the user what changed
- **Always** validate after build or install
- **Registry is authoritative** — when registry and SKILL.md disagree, fix SKILL.md to match registry
- **Always** back up (git tracks changes — commit before major modifications)

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `skill-manager-validate.j2` | KnowAct | Validate skills against registry format and quality checks |
| `skill-manager-build.j2` | KnowAct | Scaffold a new registry crate from a user description |

## When to Use This Skill

- "List skills" / "Show skills": Report the skill corpus from the registry
- "Validate skills": Check registry format, quality, and safety; SKILL.md is secondary
- "Create a skill": Scaffold registry crate first, generate SKILL.md from it
- "Install a skill": Add registry crate to the project, generate companion SKILL.md
- "Remove a skill": Prune or deprecate registry crate and generated SKILL.md
- "Skill stats": Registry-first corpus health overview


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/skill-manager.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = no critical blockers remain

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 18000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
