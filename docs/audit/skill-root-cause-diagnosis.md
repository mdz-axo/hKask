# Task 1 — Root-Cause Diagnosis of the Skill Corpus

**Date:** 2026-06-16  
**Scope:** `.agents/skills/*`, `registry/templates/*`, `registry/manifests/*`, `registry/templates/bootstrap-registry.yaml`, and canonical hKask type/loader/executor code.

---

## 1. Executive Summary

Prior agents produced a registry layer that is **structurally incompatible** with the hKask runtime type system. The dominant failure mode is treating the Zed-style prompt-injection model (where `SKILL.md` text is runtime behavior) as if it were the hKask runtime model (where `.j2` templates are rendered inside YAML `FlowDef` manifests executed by `ManifestExecutor`).

The corpus currently has **71 registry entries**, of which:

- 21 score below 0.2 and are recommended for deprecation
- 13 are flagged critical
- 20 are stale warnings
- only 17 are active

The single largest defect class is `.j2` files declaring `template_type: FlowDef`.

---

## 2. Defect Classification by Pragmatic Force

| # | Failure | Force | Evidence | Count |
|---|---------|-------|----------|-------|
| F1 | `.j2` files declare `template_type: FlowDef` | **Prohibition** | Runtime type system says FlowDef → `.yaml` (`crates/hkask-types/src/lexicon.rs:64-70`); `Registry::bootstrap()` loads `RegistryEntry` records, not `.j2` as FlowDef (`crates/hkask-templates/src/registry.rs:337-357`) | ≥ 30 `.j2` files |
| F2 | DDMVSS aliases (`Cognition`, `Prompt`, `Process`) used as runtime `template_type` | **Prohibition** | `TemplateType::parse_str()` accepts only `WordAct`/`KnowAct`/`FlowDef` (`crates/hkask-types/src/lexicon.rs:52-59`) | Found in `registry/templates/flowdef/*.j2` and others |
| F3 | hLexicon terms in `.j2` frontmatter do not exist in `registry/hlexicon/hlexicon-workspace.yaml` | **Guardrail** | `Registry::register()` logs warnings for unknown terms when hLexicon is set (`crates/hkask-templates/src/registry.rs:141-169`) | Hundreds of flags |
| F4 | Skills exist in only one layer (Zed-only or registry-only) | **Guideline** | A skill is complete only when both layers exist (`skill-maintenance` dual-layer audit) | 42 incomplete skills |
| F5 | Bundles / FlowDef manifests created before constituent primary skills were calibrated | **Guideline** | `BundleManifest` references `skills:`; many referenced skills are deprecated/critical (`registry/manifests/*.yaml`) | Multiple manifests |
| F6 | `SKILL.md` methodology contradicts `.j2` contract | **Evidence** | Some `.j2` contracts specify JSON output but the body emits free text; some `FlowDef` `.j2` bodies duplicate prompt logic that should live in `KnowAct` | Observed in `adversarial-red-team`, `decision-journal`, `self-critique-revision` |
| F7 | Missing `manifest.yaml` in registry directories | **Evidence** | `skill-manager` R1 requires manifest; `Registry::bootstrap()` uses `bootstrap-registry.yaml`, but the directory manifest is the source-of-truth crate descriptor | 40 directories lack manifest |

---

## 3. Representative Evidence

### F1 — FlowDef declared on `.j2` file (Prohibition)

`registry/templates/adversarial-red-team/generate-adversarial.j2:1-2`:

```yaml
[inference]
template_type: FlowDef
```

Same defect in:
- `registry/templates/adversarial-red-team/multi-turn-attack.j2:2`
- `registry/templates/adversarial-red-team/select-target.j2:2`
- `registry/templates/adversarial-red-team/test-against-target.j2:2`
- `registry/templates/flowdef/*.j2` (by directory naming)
- `registry/templates/decision-journal/*.j2`
- `registry/templates/dct-pipeline/*.j2`

Runtime consequence: if this `.j2` were registered as a `RegistryEntry` with `template_type: FlowDef`, the executor would expect a YAML `BundleManifest` but receive a Jinja2 prompt. `ManifestExecutor::execute_manifest` takes `BundleManifest`, not a rendered `.j2` string.

### F2 — DDMVSS alias used as runtime type

`registry/templates/flowdef/` contains files whose frontmatter likely uses `Cognition`, `Prompt`, or `Process` instead of `WordAct`/`KnowAct`. The `TemplateType` enum maps DDMVSS aliases only through `as_spec_name()`:

```rust
pub fn as_spec_name(&self) -> &'static str {
    match self {
        TemplateType::WordAct => "Prompt",
        TemplateType::KnowAct => "Cognition",
        TemplateType::FlowDef => "Process",
    }
}
```

(`crates/hkask-types/src/lexicon.rs:75-81`)

This is a one-way mapping for documentation, not a parseable runtime type.

### F3 — Unknown hLexicon terms

`registry/templates/adversarial-red-team/generate-adversarial.j2:3-13` lists terms such as `generate`, `adversarial`, `inject`, `hijack`, `tool_misuse`, `exfiltrate`. These are absent from the canonical workspace lexicon, as flagged by the audit script.

### F4 — Single-layer skills

`registry/templates/caveman/manifest.yaml` exists but `.agents/skills/caveman/SKILL.md` does not (audit row: `caveman | ✓ | ✗ | 0.75`).

Conversely, `.agents/skills/pragmatics/SKILL.md` exists but `registry/templates/pragmatics/` does not (audit row: not listed because only registry entries are in the report).

### F5 — Bundles before calibrated primaries

`registry/manifests/pragmatic-composition/process_manifest.yaml:5-116` defines a multi-stage process but references `template_ref: prompt/selector` and `mcp: hkask-mcp-memory` with placeholder validation. The underlying `prompt/selector` template is not robustly calibrated, and the manifest itself carries an `editor: curator-or-human-admin` admin-gated field — a P3 / Clear Boundaries concern.

`registry/manifests/ensemble-orchestration.yaml` and `registry/manifests/standing-ensemble-session.yaml` similarly orchestrate multiple skills before each skill reaches active health.

### F6 — Layer contradiction

`registry/templates/adversarial-red-team/test-against-target.j2:14-38` declares:

```yaml
contract:
  output:
    test_results: array
    resistance_rate: number
    critical_failures: array
```

but the template body is a narrative prompt, not a JSON-emitting structure, and the audit flags `contract input 'intensity_level' not obviously used in template body`.

---

## 4. Root-Cause Narrative

1. **Model confusion:** Prior agents came from a Zed-style mental model where `SKILL.md` text drives runtime behavior. They wrote `.j2` files as if they were SKILL.md bodies and tagged them `FlowDef` because the DDMVSS spec calls a process template "Process".
2. **No boundary guard:** The current `SkillLoader` silently assigns `TemplateType::FlowDef` to every `Skill` it registers from `SKILL.md` frontmatter, without linking it to registry templates. This made the mismatch invisible.
3. **Bootstrap drift:** The runtime registry loads from `bootstrap-registry.yaml`, but many template crates were added to `registry/templates/` without being registered there. The directory layer and the bootstrap layer diverged.
4. **Premature composition:** Agents created `BundleManifest` YAML files in `registry/manifests/` and `FlowDef`-declared `.j2` wrappers before the primary skills they compose were calibrated, producing cascading dependency on broken templates.

---

## 5. Prioritized Fix Order

1. Stop authoring `FlowDef` on `.j2` files; convert or delete these templates.
2. Delete premature bundles whose primary skills are not yet active.
3. Fix hLexicon grounding on remaining active candidates.
4. Add missing manifests and align cross-layer descriptions.
5. Bootstrap `bootstrap-registry.yaml` only after calibration.
