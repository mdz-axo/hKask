---
name: skill-maintenance
visibility: public
description: "Audit hKask's skill architecture for staleness, coverage gaps, and quality degradation. Registry crate (manifest.yaml + *.j2) is the canonical source of truth. Detect broken references, contract drift, invalid template types. Score health, recommend deprecation. Use when the user says 'audit skills', 'check skills', or periodically to maintain skill hygiene."
---

# Skill Maintenance — Registry-First Audit

hKask skills live in the registry crate as the canonical source of truth per P5.1. The SKILL.md is a generated companion. Audit the registry first; SKILL.md is secondary.

| Artifact | Audience | Authoritative? |
|----------|----------|---------------|
| Registry crate (`manifest.yaml` + `*.j2`) | Runtime (Inference Router, cascade) | **Yes** |
| SKILL.md (`.agents/skills/<name>/SKILL.md`) | Zed coding agent | **No** — derived from registry |

## Skill Lifecycle

```
active → stale_warning → deprecated → retired
```

| State | Meaning | Entry |
|-------|---------|-------|
| **active** | Registry crate valid, contract accurate | Validated after install |
| **stale_warning** | One or more staleness signals detected | Audit found issues |
| **deprecated** | Marked for removal, no longer activated | User or supersession |
| **retired** | Registry crate deleted | Deleted |

## Staleness Detection

### Registry Crate Signals (canonical — manifest.yaml + .j2 + .yaml)

| Signal | Detection | Severity |
|--------|-----------|----------|
| manifest.yaml version stale | Version doesn't match workspace version | Medium |
| .j2 contract drift — input/output fields no longer match runtime types | Compare contract against actual struct fields in crate code | High |
| `template_type: Cognition` — invalid | Must be `WordAct`, `KnowAct`, or `FlowDef` | High |
| vocabulary terms not in known vocabulary | Compare against known vocabulary | Medium |
| `energy_cap` out of range | Must be 2048–8192 | Medium |
| `visibility` value invalid | Must be `Private`, `Public`, or `Shared` | High |
| manifest `templates` entry references .j2 or .yaml that doesn't exist | Check `path` against filesystem; resolve relative paths (`../`) from manifest directory | High |
| .j2 frontmatter `template_type` disagrees with manifest entry | Cross-check manifest type vs .j2 frontmatter type | High |

### FlowDef-Specific Signals (.yaml files)

FlowDef YAML files describe multi-step workflows. Each step references MCP tools
and/or registry templates. These references must be validated against actual
registrations — the logo-builder audit revealed that stale tool names and
stale template references are the most common runtime-breaking bugs.

| Signal | Detection | Severity |
|--------|-----------|----------|
| **FlowDef `tool:` references nonexistent MCP tool** | Cross-check every step's `tool:` field against the MCP server tool registry. Known tools include: `generate_image`, `describe_image`, `image_remove_background`, `upscale_image`, `transform_image`, `generate_video`, `generate_speech`, `transcribe`, `gallery_organize`, `gallery_search`, `gallery_status`, `gallery_refresh`, `gallery_analyze`, `video_clip`, `video_to_gif`, `image_to_video`, `apply_style`, `create_collage`, `face_register`, `face_validate`, `face_list`, `object_extract`, `gallery_by_time`. Unknown tool names → flag. | **Critical** — causes runtime failure |
| **FlowDef references `tool: internal`** | `tool: internal` means no MCP implementation exists — the step is agent-coordinated. This is valid but must be documented as aspirational. Flag for human review. | Medium |
| **FlowDef `template:` references a template that doesn't exist or was deleted** | Resolve `template:` values against the manifest's known templates. A FlowDef referencing `media/logo-single-shot` when that file was deleted → flag. | **Critical** — causes runtime failure |
| **FlowDef step references tool with mismatched parameter names** | Compare FlowDef step parameters against the MCP tool's declared `Parameters` struct fields. Mismatched names mean the tool won't recognize the input. | High |
| **FlowDef YAML present but no corresponding manifest entry** | Every FlowDef `.yaml` file should have a `templates` entry in the directory's `manifest.yaml`. Orphaned FlowDefs can't be discovered by the cascade. | High |

### SKILL.md Companion Signals (generated)

| Signal | Detection | Severity |
|--------|-----------|----------|
| Referenced file/crate path does not exist | Read body, check paths | Medium |
| CNS span name not in canonical set | Compare `crates/hkask-types/src/cns.rs` (`CnsSpan`) | Medium |
| Magna Carta P1–P4 reference outdated | Compare `docs/architecture/core/PRINCIPLES.md` | Medium |
| Description vague or generic | < 20 chars, no specific triggers | Low |
| Body contradicts current architecture | Read referenced code, check consistency | Medium |
| Drift from registry — claims behaviors registry templates do not support | Compare SKILL.md methodology against .j2 template contents | Medium |
| **SKILL.md references tool name that doesn't match MCP registration** | Cross-check tool names in SKILL.md descriptions and frontmatter against MCP tool registry. Example: `remove_background` in description but actual tool is `image_remove_background`. | High — agent will call wrong tool |
| **SKILL.md references stale template paths or deleted templates** | Verify every `registry/templates/` path reference in SKILL.md resolves to an existing file. | High |

### Cross-Artifact Signals

| Signal | Detection | Severity |
|--------|-----------|----------|
| SKILL.md exists but no registry crate | Compare directory listings | **Critical** — not executable |
| Registry crate exists but no SKILL.md | Compare directory listings | Info — optional companion |
| SKILL.md methodology doesn't match .j2 template logic | Read both, compare intent | Medium |
| **Template deletion cascading — stale references after file removal** | When a `.j2` or `.yaml` template is deleted, re-audit all manifests and FlowDefs that reference it by `path:` or `template:`. The audit must detect that the reference target no longer exists. | **Critical** — cascading breakage |
| **Manifest `path:` uses `../` cross-directory reference that doesn't resolve** | Verify relative paths from the manifest's directory. Example: `path: ../media/logo-formal-prompt.j2` from `logo-builder/manifest.yaml` must resolve to `media/logo-formal-prompt.j2`. | High |

## Audit Procedure

For each skill name present in the registry:

1. **Check registry**: manifest structure, each .j2 frontmatter vs manifest, contract validity, template_type validity, vocabulary coverage, energy_cap range, visibility values
2. **Check FlowDef .yaml files**: for every `steps` entry, validate `tool:` against MCP registry; validate `template:` references against manifest; check for `tool: internal`; validate parameter names
3. **Check SKILL.md** (if present): frontmatter format, references, consistency, drift from registry, tool name accuracy
4. **Check cross-artifact**: registry exists (required), SKILL.md aligns with registry when present, template deletion cascading, cross-directory path resolution
5. **Score**: compute health score with registry as primary

## Health Score

```
score = 1.0

Registry deductions (canonical):
  -0.50 per missing registry crate (critical — not executable)
  -0.15 per invalid template_type (e.g. Cognition → must be KnowAct)
  -0.15 per FlowDef `tool:` referencing nonexistent MCP tool (causes runtime failure)
  -0.15 per FlowDef `template:` referencing deleted/nonexistent template
  -0.10 per contract drift (input/output mismatch)
  -0.10 per manifest/.j2/.yaml path reference broken (including ../ cross-directory)
  -0.10 per FlowDef step parameter name mismatch with MCP tool
  -0.10 per invalid visibility value
  -0.05 per vocabulary term not in known vocabulary
  -0.05 per energy_cap out of 2048–8192 range
  -0.05 per FlowDef `tool: internal` (agent-coordinated — valid but aspirational)
  -0.05 per FlowDef YAML present with no manifest entry

SKILL.md deductions (companion):
  -0.10 per broken reference (file, crate, span, doc path)
  -0.10 per contradiction with architecture or Magna Carta
  -0.10 per drift from registry (claims unsupported behaviors)
  -0.10 per tool name mismatch with MCP registration (agent will call wrong tool)
  -0.10 per stale template path reference
  -0.05 per vague description

Cross-artifact deductions:
  -0.15 per template deletion cascading (stale reference after file removal)
  -0.10 per cross-directory path that doesn't resolve

Floor at 0.0.
```

| Score | Status | Action |
|-------|--------|--------|
| >= 0.8 | active | No action |
| 0.5–0.79 | stale_warning | Review within 30 days |
| 0.2–0.49 | stale_warning (critical) | Prioritize revision |
| < 0.2 | recommend deprecation | Deprecate or retire |

### Report Format

```
Skill Audit — [date]:
  + skill-name   active (0.92) — registry valid
  ! skill-name   stale (0.61) — registry: [issue]
  X skill-name   critical (0.31) — registry: [issue]
  - skill-name   deprecated — [reason]
  i skill-name   registry-only (0.85) — no SKILL.md (optional companion)
```

## Coverage Gap Analysis

| Dimension | What to Check |
|----------|---------------|
| **Task pattern → registry crate** | Does a registry crate cover each common hKask task pattern? |
| **template_type distribution** | Are `WordAct`, `KnowAct`, `FlowDef` all represented? Over-concentration signals gap. |
| **Vocabulary term coverage** | Do template `lexicon_terms` cover the known vocabulary? Missing terms = blind spots. |
| **Cascade depth** | Do FlowDef templates exist for multi-step workflows? Missing FlowDef = no cascade wiring. |
| **FlowDef tool coverage** | Do FlowDef steps reference tools that exist? Unmapped `tool:` entries = dead execution paths. |
| **FlowDef template coverage** | Do FlowDef `template:` references resolve to existing templates? Stale refs = broken cascades. |

### Gap Report Format

```
Coverage Gaps:
  Registry:
    [template_type] — [N]% concentration, [missing_type] absent
    vocabulary: [term] not covered by any template
    Cascade: no FlowDef for [workflow]
  Companion:
    [name] — registry crate present, SKILL.md absent (info only)
```

## Deprecation

### When to deprecate

- Superseded by a better skill
- Domain no longer relevant
- Health score consistently < 0.2
- User decides
- Violates Magna Carta and fix requires rewrite

### Soft deprecation

| Artifact | Action |
|----------|--------|
| Registry | Set `visibility: Private` on all .j2 templates; add `deprecated: true` to manifest.yaml |
| SKILL.md | Add `disable-model-invocation: true` to frontmatter; add deprecation notice |

### Hard retirement

Delete:
- `registry/templates/<name>/` (canonical source)
- `.agents/skills/<name>/` (generated companion, if present)

### Merge/Replace

When skill A supersedes skill B:

1. Document supersession in A's manifest description
2. Transfer unique content from B's templates that A doesn't cover
3. Verify A's templates cover all task patterns B covered
4. Deprecate B (soft or hard)

## Maintenance Triggers

| Trigger | Check |
|---------|-------|
| Architecture changes | All registry crates — crate names, spans, contracts |
| Magna Carta updates | All skills — P1–P4 references |
| Workspace version bump | All manifests — version freshness |
| New skill installed | Coverage gaps, template_type validity |
| Monthly | Full registry audit |
| Skill seems broken | Targeted audit of that registry crate |

## Self-Maintenance

During normal sessions, flag issues when noticed:

- Instructions didn't work → skill may be stale
- Reached for a skill that doesn't exist → coverage gap
- `template_type: Cognition` seen → flag for correction to `KnowAct`
- SKILL.md exists without registry crate → critical gap, needs registry
- FlowDef `tool:` references a name that doesn't match any MCP tool → flag for correction
- FlowDef `template:` references a template that was deleted → cascading breakage
- SKILL.md description mentions a tool by wrong name → agent will call wrong tool
- Manifest `path:` uses `../` that doesn't resolve → cross-directory path broken
- Template file deleted but references remain in manifests/FlowDefs → stale references

## When to Use This Skill

- "Audit skills" / "Check skills": Full registry-first audit
- "Is this skill stale?": Targeted audit of one registry crate
- Monthly: Full registry audit
- After architecture changes: Audit affected registry crates

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `skill-maintenance-audit.j2` | KnowAct | Audit registry crates for staleness and produce health scores |
| `skill-maintenance-coverage.j2` | KnowAct | Analyze coverage gaps across the registry corpus |
