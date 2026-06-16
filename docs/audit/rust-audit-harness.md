# Task 7 — Rust Dual-Layer Audit Harness

**Date:** 2026-06-16  
**Location:** `crates/hkask-services/src/skills.rs`

---

## Status

The harness already existed as a scaffold. I hardened it so it now correctly enforces the runtime type boundary discovered in Task 0.

## Public API

The public surface is intentionally small (deep-module discipline):

| Item | Purpose |
|------|---------|
| `SkillAuditor::new(...)` | Build an auditor from a `RegistryIndex`, `SkillRegistryIndex`, and project root |
| `SkillAuditor::audit_all()` | Audit every skill name found in either layer |
| `SkillAuditor::audit_skill(name)` | Audit a single skill |
| `SkillAuditReport::to_json()` | Serialize the full report |
| `SkillAuditReport::active_count()` | Count active skills |
| `SkillAuditReport::flowdef_on_j2_count()` | Count invalid FlowDef-on-.j2 defects |
| `SkillHealthScore::is_active()` | True iff `health_score >= 0.8` |

## Defects Detected

The harness now flags:

- Missing Zed layer (`SKILL.md`) or registry layer (`registry/templates/<name>/`).
- `SKILL.md` frontmatter issues (missing, name/dir mismatch, short description).
- Missing `manifest.yaml`.
- `.j2` files missing `[inference]` frontmatter.
- **`.j2` files declaring `template_type: FlowDef`** — Prohibition-level, because the runtime type system says FlowDef = `.yaml`.
- DDMVSS aliases (`Cognition`, `Prompt`, `Process`) in `.j2` frontmatter.
- Invalid or missing `visibility`.
- Missing or empty `contract.input` / `contract.output`.
- `energy_cap` outside the allowed range.
- Unknown `hlexicon_terms` against `registry/hlexicon/hlexicon-workspace.yaml`.
- Name mismatch between `SKILL.md` and `manifest.yaml`.

## Key Code Changes

- Added `J2FrontMatter.template_type_raw` so we can distinguish valid runtime types from DDMVSS aliases that `TemplateType::parse_str` rejects.
- Added `J2FileInfo.template_type_raw` and wired alias/FlowDef detection in `audit_j2_file`.
- Added Prohibition-level deductions in `audit_skill_internal` for FlowDef-on-`.j2` and DDMVSS aliases.
- Added `SkillAuditReport::active_count()` and `SkillAuditReport::flowdef_on_j2_count()` for CI gating.
- Added a runnable unit test `complete_skill_is_active` that constructs a temporary dual-layer skill and asserts it scores active.
- Left a `#[ignore]` proptest skeleton for future property-based coverage.

## Validation

```bash
cargo test -p hkask-services complete_skill_is_active
```

Result: **pass** (1 test).

```bash
cargo test -p hkask-services
```

Result: **all 91 tests pass** (83 unit + 7 integration + 1 corpus test), 2 ignored.

## CI Gate Hook

The harness is the engine behind the proposed CI gate. The next step is to expose a `kask skill audit --ci --fail-below 0.8` command in `hkask-cli` that:

1. Loads the project root's skills and registry.
2. Calls `SkillAuditor::audit_all()`.
3. Fails if any skill is below 0.8 or if `flowdef_on_j2_count() > 0`.
4. Emits the JSON report as a build artifact.

See the Future task section below.

## Reuse of Existing Infrastructure

- `hkask_templates::SkillLoader` — Zed layer discovery.
- `hkask_templates::Registry` — in-memory registry index.
- `hkask_types::ports::{RegistryIndex, SkillRegistryIndex, RegistryEntry}` — port traits.
- `hkask_types::lexicon::{HLexicon, LexiconTerm, TemplateType}` — lexicon validation.
- `hkask_types::visibility::Visibility` — visibility parsing.
