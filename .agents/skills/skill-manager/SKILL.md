---
name: skill-manager
visibility: public
description: "Dual-layer CRUD for the skill corpus. List, validate, build, install, and prune skills across both Zed agent layer (SKILL.md) and registry template layer (manifest.yaml + *.j2). Pairs with skill-discovery, skill-maintenance, and skill-translator."
---

# Skill Manager

Manage hKask's dual-layer skill architecture. Every skill has two artifacts that must stay consistent:

| Layer | Location | Purpose | Loaded By |
|-------|----------|---------|-----------|
| **Zed Agent** | `.agents/skills/<name>/SKILL.md` | Agent companion guide | `SkillLoader` → `SkillRegistryIndex` |
| **Registry Template** | `registry/templates/<name>/manifest.yaml` + `*.j2` | Primary runtime artifact | `SqliteRegistry` |

A skill is **complete** when both layers exist and are consistent. A skill with only one layer is **incomplete** — flag it.

**Skill struct** (`hkask-types/src/ports/mod.rs`): `id`, `domain` (TemplateType), `word_act`, `flow_def`, `know_act`, `polarity`, `content_hash`, `visibility` (Private/Public/Shared), `zone` (Private/Public), `namespace`.

**TemplateType** (`hkask-types/src/lexicon.rs`): `WordAct` | `KnowAct` | `FlowDef` only. DDMVSS names (Cognition, Prompt, Process) are spec aliases — **never** use them in .j2 frontmatter.

**Visibility**: `Private` | `Public` | `Shared` (in .j2 `[inference]` frontmatter and `Skill.visibility`).

**SkillZone**: `Private` → `.agents/skills/`, `Public` → `skills/`.

## Operations

### List Skills

Scan both layers. For each skill name, report:

```
Skills (N total, C complete, I incomplete):
  [name]  [SKILL.md]  [Registry]  [status]  [description excerpt]
  ...
```

Steps:
1. Scan `.agents/skills/*/SKILL.md` → collect Zed layer names
2. Scan `registry/templates/*/manifest.yaml` → collect registry layer names
3. Union both sets → complete list
4. For each name: mark SKILL.md ✓/✗, registry ✓/✗, status = `complete` / `incomplete`
5. Flag skills with invalid frontmatter or missing manifest

### Validate Skills

Check both layers per skill. Three validation categories:

**Zed Agent layer (SKILL.md):**

| ID | Check | Pass Criteria |
|----|-------|---------------|
| Z1 | SKILL.md exists | File present in skill directory |
| Z2 | Frontmatter valid | `---` delimiters with `name` and `description` fields |
| Z3 | Name matches directory | `name` field == directory basename, lowercase-hyphenated `^[a-z0-9]+(-[a-z0-9]+)*$` |
| Z4 | Description valid | Present, 1–1024 chars, specific and actionable |
| Z5 | Body non-empty | Instructional content present |
| Z6 | Imperative voice | Steps are commands, not passive descriptions |
| Z7 | No placeholders | No `TODO`, `FIXME`, `<insert>`, `TBD` |
| Z8 | No Magna Carta violations | Skill does not act without user direction; dangerous actions gated |
| Z9 | Headless compliance | No visual UI, Grafana, dashboards, web frontends |
| Z10 | No deprecated markers | No `todo!`, `unimplemented!`, `#[deprecated]` |

**Registry layer (manifest.yaml + *.j2):**

| ID | Check | Pass Criteria |
|----|-------|---------------|
| R1 | manifest.yaml exists | File present in `registry/templates/<name>/` |
| R2 | Crate metadata valid | `crate.name`, `crate.version`, `crate.description` all present |
| R3 | Templates list non-empty | At least one entry in `templates` array |
| R4 | Template entry valid | Each entry has `id`, `path`, `type`, `description` |
| R5 | template_type is valid | Each `.j2` frontmatter `template_type` ∈ {`WordAct`, `KnowAct`, `FlowDef`} — **reject** `Cognition`, `Prompt`, `Process` |
| R6 | Visibility is valid | Each `.j2` frontmatter `visibility` ∈ {`Private`, `Public`, `Shared`} |
| R7 | Contract valid | Each `.j2` has `contract.input` and `contract.output` with typed fields |
| R8 | energy_cap in range | Each `.j2` `energy_cap` is an integer in [1024, 16384] |
| R9 | .j2 file exists | Each template entry's `path` resolves to an actual `.j2` file |
| R10 | [inference] frontmatter valid | Each `.j2` starts with `[inference]` block containing `template_type`, `lexicon_terms`, `contract` |
| R11 | hLexicon terms exist | `hlexicon_terms` in manifest + `lexicon_terms` in each .j2 reference terms in `registry/registries/hlexicon-workspace.yaml` |
| R12 | Jinja2 body present | Each `.j2` has template body after `---` separator |

**Cross-layer consistency:**

| ID | Check | Pass Criteria |
|----|-------|---------------|
| X1 | Name consistency | SKILL.md `name` == manifest.yaml `crate.name` == directory name |
| X2 | Description aligns | SKILL.md `description` and manifest `crate.description` describe the same capability |
| X3 | No orphan Zed layer | SKILL.md exists without registry → incomplete (flag, don't fail) |
| X4 | No orphan registry layer | Registry exists without SKILL.md → incomplete (flag, don't fail) |
| X5 | Template type coverage | Manifest's `templates` include at least one `KnowAct` .j2 (agent companion skills always reason) |

Report: total skills, complete/incomplete counts, pass/fail per check category, specific failures with fix suggestions. Safety check failures (Z8, Z9, Z10) are always `critical` priority. R5 (invalid template_type) is `high` priority.

### Build a Skill

Scaffold both layers from a user description:

1. **Confirm scope**: Project-local (default) or global
2. **Choose name**: User confirms. Lowercase-hyphenated, 2–40 chars, verb-noun or noun-noun pattern. No `hkask-`, `cns-`, `mcp-` prefixes.
3. **Derive hLexicon terms**: From description, pick 3–8 terms from `registry/registries/hlexicon-workspace.yaml`
4. **Create Zed layer** — `.agents/skills/<name>/SKILL.md`:
   ```
   ---
   name: <name>
   visibility: public
   description: "<specific, actionable, 1–1024 chars>"
   ---

   # <Name Title>

   <2–3 sentence description. Imperative voice.>

   ## When to Use
   <Specific trigger conditions — at least 2>

   ## Instructions
   1. <Step one — imperative, concrete>
   2. <Step two>
   ...

   ## Constraints
   - <What the skill must NOT do>
   ...

   ## Related Skills
   - <Optional: pairing skills>
   ```
5. **Create registry layer** — `registry/templates/<name>/`:
   - `manifest.yaml`:
     ```yaml
     crate:
       name: <name>
       version: "0.24.0"
       description: >
         <one-paragraph description aligned with SKILL.md>

     templates:
       - id: <name>/<name>-<verb>
         path: <name>-<verb>.j2
         type: KnowAct
         lexicon_terms: [<term1>, <term2>, ...]
         description: >
           <what this template produces>

     hlexicon_terms:
       - <term1>
       - <term2>
       ...
     ```
   - At least one `.j2` template:
     ```
     [inference]
     template_type: KnowAct
     lexicon_terms: [<term1>, <term2>]
     contract:
       input:
         <field>: <type>
       output:
         <field>: <type>
       energy_cap: 4096
       visibility: Shared

     ---
     {# Template: <name>/<name>-<verb>.j2 #}
     {# KnowAct — <one-line purpose> #}
     {# ℏKask v0.27.0 — A Minimal Viable Container for Agents #}

     [inference]
     temperature = 0.2
     reasoning_effort = "high"
     verbosity = "standard"
     max_tokens = 4096
     thinking_budget = "full"

     You are a <role>. Your job is to <task>. <Instructions in imperative voice.>

     ## Input

     {{ <input_field> }}

     ## Output Requirements

     Respond with a JSON object:

     ```json
     {
       "<output_field>": "<type and description>"
     }
     ```

     ## Constraints

     - <Constraint 1>
     - <Constraint 2>
     - Do not execute arbitrary Python code in Jinja2 expressions.
     - Handle missing variables gracefully.
     ```
6. **Validate**: Run full validation (Z1–Z10, R1–R12, X1–X5)
7. **Confirm**: Show both layers to user for review

### Install a Skill

Install both layers from an external source:

1. **Source is hKask dual-layer format**: Copy both `.agents/skills/<name>/` and `registry/templates/<name>/`, then validate
2. **Source has only one layer**: Use `skill-translator` to generate the missing layer, then install both
3. **Source is different format**: Use `skill-translator` to convert to dual-layer, then install
4. **Validate** after installation (Z1–Z10, R1–R12, X1–X5)
5. **Verify**: Skill is discoverable by description matching in both layers

### Prune a Skill

**Soft prune** (deprecate without deleting):

| Layer | Action |
|-------|--------|
| Zed Agent | Add `disable-model-invocation: true` to SKILL.md frontmatter |
| Registry | Set `visibility: Private` on all .j2 templates; add `deprecated: true` to manifest.yaml `crate` section |

Skill remains on disk but is not auto-loaded or rendered.

**Hard prune** (delete):

1. Confirm with user — **irreversible**
2. Delete `.agents/skills/<name>/` (entire directory)
3. Delete `registry/templates/<name>/` (entire directory)
4. If version-controlled, recovery via git is possible

### Stats

Report dual-layer corpus statistics:

| Metric | What to Report |
|--------|---------------|
| Total skills | Count across both layers |
| Complete skills | Both layers present |
| Incomplete skills | Only one layer — break down: Zed-only vs registry-only |
| Template type distribution | Count of WordAct / KnowAct / FlowDef .j2 templates |
| Visibility distribution | Private / Public / Shared across .j2 templates |
| hLexicon coverage | Unique terms used vs total workspace terms |
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
| "Find a skill for X" | Delegate to `skill-discovery` |
| "Is this skill stale?" | Delegate to `skill-maintenance` |
| "Translate this skill" | Delegate to `skill-translator` |
| "Bundle skills" | Delegate to `skill-bundler` |

## Safety

- **Never** delete a skill without user confirmation
- **Never** modify a skill's instructions without telling the user what changed
- **Always** validate after build or install
- **Always** ensure both layers are consistent before declaring a skill complete
- **Always** back up (git tracks changes — commit before major modifications)

## When to Use This Skill

- **"List skills" / "Show skills"**: Report the dual-layer skill corpus
- **"Validate skills"**: Check format, quality, and safety across both layers
- **"Create a skill"**: Scaffold both SKILL.md and registry templates
- **"Install a skill"**: Add an external skill to both layers
- **"Remove a skill"**: Prune or deprecate across both layers
- **"Skill stats"**: Dual-layer corpus health overview