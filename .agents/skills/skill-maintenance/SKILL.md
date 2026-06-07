---
name: skill-maintenance
visibility: public
description: "Audit hKask's dual-layer skill architecture for staleness, coverage gaps, and quality degradation. Detect broken references, contract drift, invalid template types, and cross-layer inconsistencies across Zed agent skills and registry templates. Score health, recommend deprecation. Use when the user says 'audit skills', 'check skills', or periodically to maintain skill hygiene."
---

# Skill Maintenance — Dual-Layer Audit

hKask skills live in TWO layers. Audit both. A skill with only one layer is incomplete.

| Layer | Artifact | Audience |
|-------|----------|----------|
| **Zed Agent** | `.agents/skills/<name>/SKILL.md` | Zed coding agent |
| **Registry** | `registry/templates/<name>/manifest.yaml` + `*.j2` | Runtime (Okapi, cascade) |

## Skill Lifecycle

```
active → stale_warning → deprecated → retired
```

| State | Meaning | Entry |
|-------|---------|-------|
| **active** | Both layers valid, descriptions match reality | Validated after install |
| **stale_warning** | One or more staleness signals detected | Audit found issues |
| **deprecated** | Marked for removal, no longer activated | User or supersession |
| **retired** | Removed from both layers | Deleted |

## Staleness Detection

### Zed Agent Layer Signals (SKILL.md)

| Signal | Detection | Severity |
|--------|-----------|----------|
| Referenced file/crate path does not exist | Read body, check paths | High |
| CNS span name not in canonical set | Compare `PRINCIPLES.md` §1.4 | Medium |
| Magna Carta P1–P4 reference outdated | Compare `docs/architecture/magna-carta.md` | High |
| Description vague or generic | < 20 chars, no specific triggers | Medium |
| Body contradicts current architecture | Read referenced code, check consistency | High |
| Supporting files in body missing | Check paths in skill directory | High |

### Registry Layer Signals (manifest.yaml + .j2)

| Signal | Detection | Severity |
|--------|-----------|----------|
| manifest.yaml version stale | Version doesn't match workspace version | Medium |
| .j2 contract drift — input/output fields no longer match runtime types | Compare contract against actual struct fields in crate code | High |
| `template_type: Cognition` — invalid (Cognition is DDMVSS name, not runtime-valid) | Must be `WordAct`, `KnowAct`, or `FlowDef` | High |
| hLexicon terms not in workspace registry | Compare against `hkask-types::hlexicon` or workspace lexicon source | Medium |
| `energy_cap` out of range | Must be 2048–8192 | Medium |
| `visibility` value invalid | Must be `Private`, `Public`, or `Shared` | High |
| manifest `templates` entry references .j2 that doesn't exist | Check `path` against filesystem | High |
| .j2 frontmatter `template_type` disagrees with manifest entry | Cross-check manifest type vs .j2 `[inference]` type | High |

### Cross-Layer Signals

| Signal | Detection | Severity |
|--------|-----------|----------|
| SKILL.md exists but no `registry/templates/<name>/` | Compare directory listings | High |
| Registry templates exist but no `.agents/skills/<name>/SKILL.md` | Compare directory listings | High |
| SKILL.md methodology doesn't match .j2 template logic | Read both, compare intent | Medium |

## Audit Procedure

For each skill name present in EITHER layer:

1. **Enumerate** both layers: check `.agents/skills/<name>/SKILL.md` AND `registry/templates/<name>/`
2. **Check Zed layer**: frontmatter format, references, consistency (current procedure)
3. **Check Registry layer**: manifest structure, each .j2 frontmatter vs manifest, contract validity, template_type validity, hLexicon coverage, energy_cap range, visibility values
4. **Check cross-layer**: both layers exist, methodology aligns
5. **Score**: compute health score across both layers

## Health Score

```
score = 1.0

Zed layer deductions:
  -0.15 per broken reference (file, crate, span, doc path)
  -0.10 per contradiction with architecture or Magna Carta
  -0.05 per vague description
  -0.10 per missing supporting file

Registry layer deductions:
  -0.15 per invalid template_type (e.g. Cognition → must be KnowAct)
  -0.10 per contract drift (input/output mismatch)
  -0.10 per manifest/.j2 path reference broken
  -0.05 per hLexicon term not in workspace
  -0.05 per energy_cap out of 2048–8192 range
  -0.10 per invalid visibility value

Cross-layer deductions:
  -0.25 per missing layer (SKILL.md without registry, or vice versa)
  -0.10 per methodology/logic mismatch between layers

Floor at 0.0.
```

| Score | Status | Action |
|-------|--------|--------|
| ≥ 0.8 | active | No action |
| 0.5–0.79 | stale_warning | Review within 30 days |
| 0.2–0.49 | stale_warning (critical) | Prioritize revision |
| < 0.2 | recommend deprecation | Deprecate or retire |

### Report Format

```
Dual-Layer Skill Audit — [date]:
  ✓ skill-name   active (0.92) — both layers valid
  ⚠ skill-name   stale (0.61) — [layer]: [issue]
  ✗ skill-name   critical (0.31) — [layer]: [issue]
  ◌ skill-name   deprecated — [reason]
```

## Coverage Gap Analysis

Check BOTH layers for gaps:

| Dimension | What to Check |
|----------|---------------|
| **Task pattern → SKILL.md** | Does a skill description cover each common hKask task pattern? |
| **template_type distribution** | Are `WordAct`, `KnowAct`, `FlowDef` all represented? Over-concentration in one type signals gap. |
| **hLexicon term coverage** | Do template `lexicon_terms` and `hlexicon_terms` cover the workspace lexicon? Missing terms = blind spots. |
| **Cascade depth** | Do FlowDef templates exist for multi-step workflows, or are all skills single-act? Missing FlowDef = no cascade wiring. |

### Gap Report Format

```
Coverage Gaps:
  Zed layer:
    [pattern] — no SKILL.md matches
    [pattern] — partially covered by [name], missing [aspect]
  Registry layer:
    [template_type] — [N]% concentration, [missing_type] absent
    hLexicon: [term] not covered by any template
    Cascade: no FlowDef for [workflow]
  Cross-layer:
    [name] — SKILL.md present, registry absent (or vice versa)
```

## Deprecation

### When to deprecate

- Superseded by a better skill
- Domain no longer relevant
- Health score consistently < 0.2
- User decides
- Violates Magna Carta and fix requires rewrite

### Soft deprecation

| Layer | Action |
|-------|--------|
| Zed Agent | Add `disable-model-invocation: true` to SKILL.md frontmatter |
| Registry | Set `visibility: Private` on all .j2 templates and manifest |

Skill still exists on disk. User can invoke manually.

### Hard retirement

Delete from BOTH:
- `.agents/skills/<name>/` (entire directory)
- `registry/templates/<name>/` (entire directory)

Consider archiving first if content may be adapted later.

### Merge/Replace

When skill A supersedes skill B:

1. Document supersession in A's SKILL.md body and manifest description
2. Transfer unique content from B that A doesn't cover
3. Verify A covers all task patterns B covered
4. Deprecate B (soft or hard)

## Maintenance Triggers

| Trigger | Check |
|---------|-------|
| Architecture changes | All skills — crate names, spans, doc paths, contracts |
| Magna Carta updates | All skills — P1–P4 references |
| Workspace version bump | All manifests — version freshness |
| New skill installed | Coverage gaps, cross-layer consistency |
| Monthly | Full dual-layer audit |
| Skill seems broken | Targeted audit of that skill |

## Self-Maintenance

During normal sessions, flag issues when noticed:

- Instructions didn't work → skill may be stale
- Reached for a skill that doesn't exist → coverage gap
- Description didn't match task → description quality issue
- Two skills contradicted → flag for `skill-bundler` conflict resolution
- `template_type: Cognition` seen → flag for correction to `KnowAct`
- Only one layer present → flag cross-layer gap

Don't full-audit every session. Mention issues when relevant.