---
name: skill-maintenance
visibility: public
description: "Audit skill corpus for staleness, coverage gaps, and quality degradation. Detect skills that reference moved files, outdated architecture, or contradictory instructions. Score skills and recommend retirement. Use when the user says 'audit skills', 'check skills', or periodically to maintain skill hygiene."
---

# Skill Maintenance

Audit the skill corpus for health. Skills rot — descriptions become inaccurate, instructions reference moved files, patterns contradict updated architecture. This skill teaches you to detect, score, and remediate skill degradation.

## Skill Lifecycle

Every skill moves through four states:

```
active → stale_warning → deprecated → retired
```

| State | Meaning | How It Got There |
|-------|---------|-----------------|
| **active** | Loaded by agent, descriptions match reality | Skill validates after installation |
| **stale_warning** | One or more staleness signals detected | Audit found issues |
| **deprecated** | Marked for removal, no longer activated | User decided or skill superseded |
| **retired** | Removed from skill directory | Deleted or archived |

## Staleness Detection

### Signals that a skill is stale

| Signal | Detection Method | Severity |
|--------|-----------------|----------|
| Description references non-existent files or paths | Read SKILL.md, check referenced paths exist | High — skill instructions are broken |
| Instructions reference renamed or moved crates | Compare against workspace `Cargo.toml` crate map | High — instructions are misleading |
| CNS span names referenced that don't exist in canonical list | Check against `hkask-types::event::CANONICAL_NAMESPACES` or PRINCIPLES.md §1.4 | Medium — observability is wrong |
| Magna Carta principle references outdated | Compare against current P1–P4 in `docs/architecture/magna-carta.md` | High — governance is wrong |
| Description is too vague to match relevant requests | Description < 20 chars or uses generic phrases like "helps with code" | Medium — skill is invisible |
| Instructions contradict current architecture | Read code referenced by instructions, check consistency | High — skill teaches wrong things |
| Supporting files referenced in body don't exist | Check all referenced file paths in skill directory | High — incomplete skill |
| Skill never activated in recent sessions | No user request matched its description (heuristic) | Low — may be unused |

### Staleness Report Format

When auditing, produce a report:

```
Skill Staleness Audit — [date]:
  ✓ skill-name        active — description matches, instructions valid
  ⚠ skill-name        stale_warning — [specific issue]
  ✗ skill-name        stale_warning (critical) — [specific issue]
  ◌ skill-name        deprecated — [reason]
```

### How to Audit a Single Skill

1. **Read** the SKILL.md frontmatter and body
2. **Check format**: `name` matches directory, `description` is 1–1024 chars
3. **Check references**: Every file path, crate name, span name, and doc path referenced in the body exists
4. **Check consistency**: Instructions don't contradict current architecture or Magna Carta
5. **Check description quality**: Description is specific enough to be matched by relevant requests
6. **Score**: Assign a health score (0–1.0)

### Health Score Calculation

```
score = 1.0
- 0.3 per broken reference (file, crate, span, or doc path)
- 0.2 per contradiction with current architecture
- 0.2 per Magna Carta violation or outdated reference
- 0.1 per vague description
- 0.1 per missing supporting file

thresholds:
  ≥ 0.8 → active (healthy)
  0.5–0.79 → stale_warning (review needed)
  0.2–0.49 → stale_warning (critical, prioritize)
  < 0.2 → recommend deprecation
```

## Coverage Gap Analysis

### How to check for gaps

1. List all skills: read `.agents/skills/` directory
2. For each common task pattern in hKask, check if a skill matches:
   - Coding and implementation → `coding-guidelines`, `tdd`
   - Debugging → `diagnose`
   - Architecture → `improve-codebase-architecture`, `zoom-out`
   - Knowledge testing → `grill-me`
   - Sovereignty → `magna-carta-verifier`, `constraint-forces`
   - System reasoning → `pragmatic-cybernetics`, `pragmatic-semantics`
   - Session continuity → `handoff`
   - Skill management → `skill-discovery`, `skill-maintenance`, `skill-manager`, `skill-bundler`, `skill-translator`
3. For any uncovered pattern, report the gap and suggest creating or finding a skill

### Gap Report Format

```
Coverage Gaps:
  [pattern] — no skill matches this task pattern
  [pattern] — partially covered by [skill-name] but [what's missing]
```

## Deprecation Process

### When to deprecate

A skill should be deprecated when:
1. It's superseded by a better skill
2. Its domain is no longer relevant
3. Its health score is consistently below 0.2
4. The user says they don't need it
5. It violates current Magna Carta principles and fixing it would require a rewrite

### Soft deprecation

Add `disable-model-invocation: true` to the SKILL.md frontmatter. The skill still exists on disk but is not auto-loaded. The user can still invoke it manually via slash command.

### Hard retirement

Delete the skill directory from `.agents/skills/`. No uninstall ceremony — but consider archiving first if the skill has useful content that might be adapted later.

### Merge/Replace

When a new skill supersedes an old one:

1. Document the supersession in the new skill's body
2. Deprecate the old skill (soft or hard)
3. Transfer any unique content from the old skill that the new one doesn't cover
4. Verify the new skill covers all task patterns the old one covered

## Maintenance Schedule

| Frequency | What to Check |
|-----------|--------------|
| **After architecture changes** | All skills — check crate names, span references, doc paths |
| **After Magna Carta updates** | All skills — check P1–P4 references |
| **After adding new skills** | Coverage gaps — does the new skill overlap or complement existing ones? |
| **Monthly** | Full audit: staleness, coverage, quality |
| **When a skill seems broken** | Targeted audit of that specific skill |

## Self-Maintenance

During normal sessions, flag skill issues when you notice them:

- A skill's instructions didn't work as expected → the skill may be stale
- You reached for a skill that doesn't exist → coverage gap
- A skill's description didn't match the task → description quality issue
- Two skills gave contradictory instructions → flag for `skill-bundler` conflict resolution

Don't do a full audit in every conversation (token budget). Mention issues when they are relevant.

## When to Use This Skill

- **"Audit skills" / "Check skills":** Run a full staleness + coverage audit
- **"Is this skill still good?":** Audit a single skill, produce health score
- **"Clean up skills":** Identify candidates for deprecation or retirement
- **"What skills am I missing?":** Coverage gap analysis
- **After architecture changes:** Re-validate all skills that reference changed components
- **Periodically:** Monthly hygiene check to catch gradual degradation