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

### Registry Crate Signals (canonical — manifest.yaml + .j2)

| Signal | Detection | Severity |
|--------|-----------|----------|
| manifest.yaml version stale | Version doesn't match workspace version | Medium |
| .j2 contract drift — input/output fields no longer match runtime types | Compare contract against actual struct fields in crate code | High |
| `template_type: Cognition` — invalid | Must be `WordAct`, `KnowAct`, or `FlowDef` | High |
| vocabulary terms not in known vocabulary | Compare against known vocabulary | Medium |
| `energy_cap` out of range | Must be 2048–8192 | Medium |
| `visibility` value invalid | Must be `Private`, `Public`, or `Shared` | High |
| manifest `templates` entry references .j2 that doesn't exist | Check `path` against filesystem | High |
| .j2 frontmatter `template_type` disagrees with manifest entry | Cross-check manifest type vs .j2 `[inference]` type | High |

### SKILL.md Companion Signals (generated)

| Signal | Detection | Severity |
|--------|-----------|----------|
| Referenced file/crate path does not exist | Read body, check paths | Medium |
| CNS span name not in canonical set | Compare `crates/hkask-types/src/cns.rs` (`CnsSpan`) | Medium |
| Magna Carta P1–P4 reference outdated | Compare `docs/architecture/core/PRINCIPLES.md` | Medium |
| Description vague or generic | < 20 chars, no specific triggers | Low |
| Body contradicts current architecture | Read referenced code, check consistency | Medium |
| Drift from registry — claims behaviors registry templates do not support | Compare SKILL.md methodology against .j2 template contents | Medium |

### Cross-Artifact Signals

| Signal | Detection | Severity |
|--------|-----------|----------|
| SKILL.md exists but no registry crate | Compare directory listings | **Critical** — not executable |
| Registry crate exists but no SKILL.md | Compare directory listings | Info — optional companion |
| SKILL.md methodology doesn't match .j2 template logic | Read both, compare intent | Medium |

## Audit Procedure

For each skill name present in the registry:

1. **Check registry**: manifest structure, each .j2 frontmatter vs manifest, contract validity, template_type validity, vocabulary coverage, energy_cap range, visibility values
2. **Check SKILL.md** (if present): frontmatter format, references, consistency, drift from registry
3. **Check cross-artifact**: registry exists (required), SKILL.md aligns with registry when present
4. **Score**: compute health score with registry as primary

## Health Score

```
score = 1.0

Registry deductions (canonical):
  -0.50 per missing registry crate (critical — not executable)
  -0.15 per invalid template_type (e.g. Cognition → must be KnowAct)
  -0.10 per contract drift (input/output mismatch)
  -0.10 per manifest/.j2 path reference broken
  -0.05 per vocabulary term not in known vocabulary
  -0.05 per energy_cap out of 2048–8192 range
  -0.10 per invalid visibility value

SKILL.md deductions (companion):
  -0.10 per broken reference (file, crate, span, doc path)
  -0.10 per contradiction with architecture or Magna Carta
  -0.05 per vague description
  -0.10 per drift from registry (claims unsupported behaviors)

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
