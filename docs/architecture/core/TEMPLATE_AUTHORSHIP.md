---
title: "Template Authorship Policy"
audience: [developers, agents]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, composition]
---

# Template Authorship Policy

**hKask v0.31.0 — How to decide whether a template is skill-bound or infrastructure.**

## Decision Tree

When adding a new `.j2` template, answer these questions in order:

```
Q1: Does the template describe a process with measurable quality improvement?
    └─ YES → Q2
    └─ NO  → Q3

Q2: Does the process benefit from iterative refinement (PDCA loop)?
    └─ YES → SKILL-BOUND
         ├─ Create a FlowDef manifest in registry/manifests/<name>.yaml
         ├─ Add convergence section with threshold, improvement_ratio, max_iterations
         ├─ Add gas and rjoule budget fields
         ├─ Add a terminal `loop` action for PDCA re-entry
         └─ Create a SKILL.md companion in .agents/skills/<name>/ (derived, not canonical)
    └─ NO  → Q3

Q3: Is the template a routing, dispatching, or model-tier selection concern?
    └─ YES → INFRASTRUCTURE
         ├─ Place in registry/manifests/ as <name>.yaml with functional_role: flowdef
         ├─ Do NOT add convergence (no PDCA loop)
         ├─ Do add gas and rjoule budget fields if any step performs inference
         └─ Mark with visibility: Public
    └─ NO  → Q4

Q4: Is the template a base type (WordAct, KnowAct, FlowDef dispatch)?
    └─ YES → INFRASTRUCTURE (base type)
         ├─ Register in bootstrap-registry.yaml
         ├─ No standalone manifest needed
         └─ Document in registry/templates/<name>/README.md
    └─ NO  → Q5

Q5: Is the template a utility that is imported/included by other templates?
    └─ YES → INFRASTRUCTURE (utility)
         ├─ No manifest needed
         ├─ Document the include API in the template header
         └─ Register in bootstrap-registry.yaml only if dispatched directly
    └─ NO  → SKILL-BOUND (default for new cognitive processes)
```

## Budget Requirements

| Manifest Type | gas.cap | rjoule.cap | cost_per_iteration | convergence |
|--------------|---------|------------|-------------------|-------------|
| Skill (PDCA) | Required (standard: 100000) | Required (standard: 2) | Required (100) | Required |
| Infrastructure dispatch | Required (50000) | Required (1) | Required (100) | Not applicable |
| Web tool | Required (2048–8192) | Required (0–1) | Not applicable | Not applicable |
| QA script | Required (10000–50000) | Required (1) | Not applicable | Not applicable |

## SKILL.md ≠ Skill Invariant

**The canonical artifact of a skill is its FlowDef manifest (`.yaml`) and its executable templates (`.j2`).**

The `SKILL.md` file is a derived companion document. It teaches the Zed coding agent the methodology but is **not** what the hKask runtime executes. Editing a `SKILL.md` does not change the skill's behavior in `kask chat` sessions.

All meta-skills (skill-maintenance, skill-logic-audit, etc.) **must** operate on the `.yaml` manifest and `.j2` templates as their primary truth source. They may read `SKILL.md` for methodology context, but any findings or recommendations that reference only `SKILL.md` content without verifying against the manifest and templates are **Epistemically Unsound** and must carry confidence: Hypothesis (Speculative) at maximum.

## Naming Convention

- **Manifest filename:** Must match `manifest.id`. Example: `coding-guidelines.yaml` with `manifest.id: coding-guidelines`
- **Template directory:** Must match `manifest.id`. Example: `registry/templates/coding-guidelines/` for `manifest.id: coding-guidelines`
- **SKILL.md directory:** Should match `manifest.id`. Example: `.agents/skills/coding-guidelines/SKILL.md`
- **Template ref paths:** Use `<manifest.id>/<template-name>` format. Example: `coding-guidelines/guidelines-assess`

Name mismatches (e.g., `scenario-planning` manifest referencing `scenario-builder/` templates) create discoverability failures and must be corrected.

## Audit

Before committing a new manifest or template:

```bash
# Verify manifest.id matches filename
grep "id:" registry/manifests/<name>.yaml | head -1

# Verify template directory exists and matches
ls registry/templates/$(grep "id:" registry/manifests/<name>.yaml | awk '{print $2}')/

# Verify all template_refs resolve
grep "template_ref:" registry/manifests/<name>.yaml | while read -r ref; do
  path="registry/templates/$(echo "$ref" | awk '{print $2}').j2"
  [ -f "$path" ] || echo "MISSING: $path"
done

# Verify budget fields are present
grep -c "gas:" registry/manifests/<name>.yaml
grep -c "rjoule:" registry/manifests/<name>.yaml
```

## rJoule/Gas Budget Calibration

All manifest budgets are currently **uncalibrated** — they are placeholder values set during
authorship without runtime measurement. The `WalletGasCalibrator` in `hkask-cns` provides
the infrastructure for calibration.

### Calibration Procedure

```bash
# 1. Run a representative execution of the skill with CNS span logging enabled
kask run <skill-name> --cns-spans

# 2. Extract the actual gas and rjoule consumption from CNS spans
kask cns alerts

# 3. Compare against the manifest's declared budget
#    If actual > declared: the manifest budget is too low (will cause aborts)
#    If actual < 50% of declared: the manifest budget is too loose (wasteful)

# 4. Update the manifest with calibrated values
#    rjoule.cap = ceil(actual_rjoule * 1.2)    # 20% headroom
#    gas.cap   = ceil(actual_gas * 1.2)
```

### System Constants

| Constant | Value | Location |
|----------|-------|----------|
| `GAS_PER_RJOULE` | 250000 | `crates/hkask-wallet-types/src/lib.rs` |
| `RJOULE_TO_GAS` | 250000 | Used in CSkill files, reverse of above |

### Known Uncalibrated Budgets

All skill manifests currently use uncalibrated placeholder values:

| Budget | Standard Value | Calibrated? |
|--------|---------------|-------------|
| `gas.cap` for skills | 100000 | No — placeholder |
| `gas.cap` for sequential-inquiry | 120000 | No — 20% above standard, not measured |
| `rjoule.cap` for skills | 2 | No — placeholder |
| `rjoule.cap` for superforecasting | 5 | No — justified by complexity but unmeasured |
| `rjoule.cap` for coding-guidelines, tdd | 3 | No — placeholder |

Calibration should be run as part of the v0.32.0 release cycle after the CNS
`WalletGasCalibrator` has been validated against production inference workloads.
