# Handoff — Skill Model Migration to Registry-First (P5.1)

**Date:** 2026-06-17
**Session:** Skill model architecture decision formalization and migration
**Status:** ~85% complete — structural migration done, content reconciliation in progress
**Next agent skills to load:** coding-guidelines, skill-translator, skill-maintenance, skill-manager

---

## Session Context

This session formalized and executed the migration from a "dual-layer co-equal" skill model (SKILL.md + registry as peers) to a "registry-first" model (registry crate as canonical source of truth, SKILL.md as generated companion). The decision is codified as **P5.1 — Single Source of Truth for Skills** in `docs/architecture/core/PRINCIPLES.md`. All structural artifacts (manifests, .j2 headers, Registry Templates sections) have been updated. The Rust audit scoring code has been updated. Remaining work: SKILL.md body content reconciliation for non-governance skills, and a reverse-translation demonstration.

---

## What Was Done

### Architecture Documentation (3 files)
- **`docs/architecture/hKask-architecture-master.md`**: Pattern A now includes "Skill Artifact Model: Single Source of Truth" subsection with derivation rule, consequences, and motivation. Version bumped to v0.28.0.
- **`docs/architecture/core/PRINCIPLES.md`**: Added **P5.1 — Single Source of Truth for Skills** under P5 (Essentialism). Version bumped to v0.28.0.
- **`AGENTS.md`**: Version bumped to v0.28.0.

### Governance SKILL.md Companions (4 files, rewritten)
- `skill-manager`: Registry-first CRUD. Registry-only = complete. SKILL.md generated from registry.
- `skill-translator`: Added reverse translation workflow. Removed dual-layer co-equal framing.
- `skill-discovery`: Registry-first evaluation. SKILL.md optional.
- `skill-maintenance`: Registry-first audit. Missing registry = -0.50 critical. Missing SKILL.md = info.

### Governance Registry Crates (4 manifests + 10 .j2 files)
- All 4 manifests: v0.28.0, registry-first descriptions.
- All 10 `.j2` templates: version headers updated, dual-layer language removed, methodologies updated.
- 1 new `.j2`: `skill-translator-reverse.j2` (KnowAct, registry -> SKILL.md reverse translation).

### Batch Mechanical Updates
- **138 `.j2`** v0.27 -> v0.28 version header bumps (batch sed)
- **43 `.j2`** version headers added where missing (awk insertion after `---`)
- **12 manifest** v0.27/v0.23/v0.21 -> v0.28 version bumps (batch sed)
- **27 manifests** generated for registry-only crates that had `.j2` files but no manifest
- **10 SKILL.md** Registry Templates sections appended (6 via cat heredoc, 4 via edit_file)

### Rust Code
- **`crates/hkask-services/src/skills.rs`**: Updated health scoring model. Missing SKILL.md: -0.25 -> -0.05 (info). Missing registry: -0.25 -> -0.50 (critical). Test updated and **passes**.

### Final State
| Metric | Count |
|--------|-------|
| Manifests at v0.28.0 | 62/62 |
| .j2 inference templates at v0.28 | 189/189 |
| SKILL.md with Registry Templates section | 33/33 |
| SKILL.md orphans (no registry) | 0 |
| Stale dual-layer language | 0 |
| `.j2` utility modules (no version header needed) | 44 |

---

## What Remains

### HIGH: SKILL.md Body Content Reconciliation
The 29 non-governance SKILL.md files still have body content written under the old dual-layer model. While no stale "dual-layer" language remains, their "## Instructions" sections may describe workflows that don't match their registry template bodies. Each needs:
1. Read the registry `.j2` template body for the skill
2. Compare against the SKILL.md "## Instructions" section
3. Update SKILL.md to reflect the registry's actual methodology

**Recommended approach:** Start with the most complex/longest SKILL.md files (essentialist, kata, coding-guidelines, tdd, diagnose) since they have the most drift surface. Use `skill-translator-reverse.j2` to generate fresh SKILL.md from registry, then compare.

### MEDIUM: Reverse-Translation Validation
The `skill-translator-reverse.j2` template exists but has not been validated end-to-end. Take a registry-only skill (e.g., `curator` with 6 templates), run it through reverse translation, and verify the generated SKILL.md is usable by the Zed agent.

### MEDIUM: hLexicon Term Population
The 27 auto-generated manifests have placeholder descriptions and empty `lexicon_terms`. Cross-reference template bodies against `registry/hlexicon/hlexicon-workspace.yaml` to populate proper hLexicon terms. This is a documentation-quality task, not blocking.

### LOW: Infrastructure Directory Cleanup
6 directories (`chat-template`, `cns`, `git`, `inference`, `memory`, `registry`) are infrastructure config dirs, not template crates. They contain selectors, YAML configs, and Jinja2 utility files -- no `[inference]` frontmatter. They either need `manifest.yaml` files declaring their non-template role, or a decision that they're intentionally manifest-free infrastructure.

---

## Recommended Skills and Tools

**Load these skills at session start:**
- `coding-guidelines` -- for surgical changes and simplicity-first discipline
- `skill-translator` -- for reverse-translating registry -> SKILL.md
- `skill-maintenance` -- for auditing health scores after reconciliation
- `skill-manager` -- for any CRUD operations on skills

**Key commands:**
```bash
# Check for version drift (should return 0)
grep -rn 'v0\.2[0-7]' registry/templates/*/ --include='*.j2' | wc -l
grep -rn 'version.*0\.2[0-7]' registry/templates/*/manifest.yaml | wc -l

# Health audit
kask skill audit --fail-below 0.8

# Count SKILL.md files needing content reconciliation
ls .agents/skills/*/SKILL.md | wc -l
```

---

## Key Decisions to Preserve

1. **P5.1 -- Single Source of Truth for Skills**: Registry crate (`manifest.yaml` + `*.j2`) is canonical. SKILL.md is a generated companion. This is not negotiable -- it's codified in PRINCIPLES.md and Pattern A. Any drift between SKILL.md and registry is a SKILL.md defect.

2. **Health scoring asymmetry**: Missing registry = -0.50 (critical, not executable). Missing SKILL.md = -0.05 (info, optional companion). The old model had both at -0.25. This is implemented in `crates/hkask-services/src/skills.rs` line 224 and 245.

3. **Registry-only skills are complete**: A skill with only a registry crate (no SKILL.md) is fully operational in the cascade. It does not need a SKILL.md companion. The 27 auto-generated manifests are legitimate complete skills.

4. **Derivation direction is one-way**: manifest.yaml + `.j2` -> SKILL.md (via `skill-translator-reverse.j2`). Never the reverse. The old skill-translator forward path (SKILL.md -> registry) still exists for importing external skills.

5. **All manifests standardized at v0.28.0**: Any new manifest should use `version: "0.28.0"`. Any manifest found at an older version is a defect.

6. **Jinja2 utility modules are not inference templates**: 44 `.j2` files without `[inference]` frontmatter (imports, macros, utility functions in `gml/`, `curator/`) are not versioned. They don't need version headers. Don't add `[inference]` blocks to them.
