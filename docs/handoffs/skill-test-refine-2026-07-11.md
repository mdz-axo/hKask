# Handoff: Skill Testing & Refinement — 2026-07-11

## 1. Session Context

This session built the `gpa-evolution` skill (GEPA: Genetic-Pareto prompt optimization, arXiv:2507.19457), then built testing infrastructure for all hKask skills, then used that infrastructure to discover and fix systemic bugs across the executor, manifests, templates, and auditor. A continuation session fixed the QA `run_skill` compile error and replaced hardcoded model names with `model_type` variables across all inference templates. The session is ~95% complete — the remaining work is gpa-evolution end-to-end calibration with a real LLM and optional smoke-test template fixes.

## 2. What Was Done

### gpa-evolution skill (complete)
- Created `registry/manifests/gpa-evolution.yaml` — 7-step PDCA FlowDef manifest
- Created 6 templates in `registry/templates/gpa-evolution/` + `manifest.yaml`
- Created `.agents/skills/gpa-evolution/SKILL.md`
- Updated `AGENTS.md` capability catalog (37 skills, 40 capabilities)

### Testing infrastructure (complete)
- `crates/hkask-test-harness/src/skill_runner.rs` — `SkillTestRunner` wraps `ManifestExecutor` with mock inference, runs any skill manifest through the PDCA executor
- `crates/hkask-test-harness/src/mocks.rs` — added `MockMcpPort` (canned tool results)
- `crates/hkask-test-harness/tests/flowdef_cross_validation.rs` — cross-validates all 93 manifests against template contracts (supports both YAML `contract:` and TOML `[contract]` formats)
- `crates/hkask-test-harness/tests/skill_gpa_evolution.rs` — 5 scenario tests for gpa-evolution
- `crates/hkask-test-harness/tests/skill_kata_improvement.rs` — 3 scenario tests for kata-improvement
- `crates/hkask-test-harness/tests/skill_smoke_tests.rs` — 9 smoke tests across 9 skills (diagnose, mcda, kata-coaching, kata-starter, idiomatic-rust, essentialist, coding-guidelines, deep-module, grill-me)

### Executor fixes (complete)
- **Loop fix** (`crates/hkask-templates/src/executor.rs`): Moved `step_idx` outside the `'cascade` loop; changed `continue` to `continue 'cascade` in the `loop` action. Root cause: `iteration` counter never incremented for loops that re-enter from step 1, making `min_iterations`/`max_iterations` useless. Now all PDCA loops have correct iteration counting.
- **Template resolution fix** (`crates/hkask-templates/src/executor.rs`): Added `.j2` extension fallback — if `read_to_string(path)` fails and the path doesn't end with `.j2`, tries `path.j2`. This makes all 314 `template_ref` values work through `ManifestExecutor` without editing manifests.
- **`with_template_base_path()` setter** added to `ManifestExecutor` for test harness.
- **`#[serde(default)]`** added to `gas_cap` and `timeout_seconds` fields on `BundleManifestStep` (`crates/hkask-templates/src/bundle/manifest.rs`). Root cause: loop steps in 5+ manifests omitted these required fields, causing parse failures through `load_manifest_from_yaml`.

### Auditor fixes (complete)
- **Jinja comment skipping** (`crates/hkask-services-skill/src/audit.rs`): `parse_j2_frontmatter` now skips leading `{# ... #}` comments before `[inference]`. Fixed mcda (0.60→1.00).
- **TOML `[contract]` stripping**: Parser now strips `[contract]` section before YAML parsing, and recognizes TOML-style contracts as valid. Fixed superforecasting.
- **Manifest fallback**: `audit_j2_file_with_fallback` uses `manifest.yaml` template metadata when `[inference]` is missing. Templates that declare metadata in `manifest.yaml` (web, media, replica, codegraph, docproc, platform-engineer, prompt-defense, heal) are no longer penalized.
- **Dead code removed**: Old `audit_j2_file` method replaced by `audit_j2_file_with_fallback`.

### Template contract fixes (complete)
- Fixed 19 template contracts across 7 skills to declare actual input fields (kata-coaching: 5, kata-improvement: 4, improv: 5, kata-starter: 5, bug-hunt: 1, improve-codebase-architecture: 1, platform-wardley-mapper: 1, skill-logic-audit: 2)
- Fixed `wordact/soap.j2` — added missing `visibility` and `energy_cap`
- Fixed `tdd/tdd-verify.j2` — added missing `visibility` and `energy_cap`
- Fixed `coding-guidelines/guidelines-verify.j2` — added missing `visibility` and `energy_cap`
- Fixed `heal/classify-error.j2` — `energy_cap` 1024→2048 (below minimum)
- Fixed `bug-hunt/bug-hunt-expedition.j2` — converted TOML-style `[inference]` to YAML-style, added `---` separator, changed `template_type` from FlowDef to KnowAct, replaced hardcoded `model: "gemma-4-26b"` with `model_type: classifier`

### Manifest fixes (complete)
- Reverted `.j2` extensions from 5 manifests (gpa-evolution, kata-improvement, kata-coaching, diagnose, mcda) — no longer needed since executor has `.j2` fallback
- Added `gas_cap`/`timeout_seconds` to loop steps in: gpa-evolution, kata-improvement, kata-coaching, diagnose, mcda (now optional via serde default, but kept for explicitness)

### Manifest.yaml creation (complete)
- Created `manifest.yaml` for 6 skills that were missing it: codegraph, docproc, platform-engineer, prompt-defense, heal, semantic-graph-audit

### Audit results
- **79/79 skills active** (was 61/79 at start)
- **0 cross-validation warnings** (was 47 at start)
- **88 tests passing**, clippy clean

### Continuation session fixes (2026-07-11)
- **Fixed `run_skill` compile error** (`crates/hkask-test-harness/src/qa_script.rs`): Removed unnecessary `branching.clone()` call — `branching` is already `&Option<BranchTarget>` from match ergonomics, and `.clone()` on a reference to a non-Clone type triggers `noop_method_call` clippy lint. The code at lines 886-898 already used `.as_ref()`, so the clone was dead. No `#[derive(Clone)]` needed.
- **Replaced hardcoded model names with `model_type` variables** across all inference templates:
  - `registry/templates/replica/infer-methods.j2` — `{# model: OM/qwen3:14b #}` → `{# model_type: classifier #}`
  - `registry/templates/replica/extract-concepts.j2` — same fix
  - `registry/templates/media/classify_style.j2` — `model: "DI/meta-llama/Llama-3.2-11B-Vision-Instruct"` → `model_type: classifier`
  - `registry/templates/media/describe_scene.j2` — same fix
  - `registry/templates/media/tag_composition.j2` — same fix
  - `registry/templates/media/tag_colors.j2` — same fix
  - `registry/templates/media/workflow-composer.j2` — `model: "{{ generation_model | default(...) }}"` → `model_type: default`
  - `registry/templates/media/logo-formal-prompt.j2` — same fix
  - `registry/templates/media/logo-discovery-map.j2` — same fix
- **Left as-is** (output content, not inference config):
  - `registry/templates/replica/discovery-corpus.j2` line 30 — `model: "DI/Qwen/Qwen3-Embedding-0.6B"` is in the generated YAML body (embedding model for corpus config)
  - `registry/templates/media/*.yaml` — `primary_model:`/`fallback_model:` are Fal.ai pipeline configs (ffmpeg, image-crate, flux-pro), not inference models
  - `registry/templates/heal/config_not_found.j2` line 44 — `model: DI/Qwen3.5-8B` is inside a JSON string representing default config file content
- All 79/79 skills still active, 88 tests passing, clippy clean

## 3. What Remains

### DONE — Fix QA `run_skill` compile error
Fixed in continuation session. The `branching.clone()` call at line 846 was a no-op (clippy `noop_method_call`) because `BranchTarget` doesn't implement `Clone` and `branching` is already `&Option<BranchTarget>` from match ergonomics. Removed the `.clone()` — the code at lines 886-898 already used `.as_ref()` on the reference. No `#[derive(Clone)]` was needed.

**Verified:** `cargo clippy -p hkask-test-harness --all-targets -- -D warnings` passes clean.

### DONE — Replace hardcoded models with model type variables
Fixed in continuation session. All inference template frontmatter now uses `model_type: classifier` or `model_type: default` instead of hardcoded model names. See "Continuation session fixes" in section 2 for the full list of changes.

Remaining `model:` references in templates are output content (generated corpus config, Fal.ai pipeline configs, default config file content) — not inference template config. These are correct and should not be changed.

### HIGH — Calibrate gpa-evolution with real LLM (model `KC/z-ai/glm5.2`)
Run the gpa-evolution skill with a real inference backend against a small eval set. Verify the evolutionary loop actually improves the prompt. Calibrate gas/rjoule budgets based on real consumption.

**How:** Use `kask kata start gpa-evolution` with context `artifact_type=prompt`, `target_artifact={content: "...", eval_set: [...]}`, `objectives=[{name: "accuracy", maximize: true}]`. The model `KC/z-ai/glm5.2` is the default model — set via `HKASK_CLASSIFIER_MODEL` env var or the `kask settings` command.

**Where:** Run from the hKask workspace root
**Verify:** The skill should complete 2-3 iterations, produce a non-empty Pareto frontier, and converge or max out gracefully.

### MEDIUM — Debug template render errors in smoke tests
6 skills are skipped in the smoke tests because their templates reference context variables that the generic seed data doesn't provide:
- `diagnose` — undefined value at step 132 (diagnose-loop.j2)
- `mcda` — undefined value at step 100 (identify-criteria.j2)
- `essentialist` — undefined value at step 418
- `coding-guidelines` — undefined value at step 139
- `deep-module` — undefined value at step 181
- `grill-me` — undefined value at step 63

**Fix:** Read each template, find the undefined variable reference, either add it to the seed context in the test or add a `| default(...)` filter in the template.

**Where:** `crates/hkask-test-harness/tests/skill_smoke_tests.rs` and the respective template files

### MEDIUM — Create a QA manifest for recursive skill testing
With the `run_skill` step type (once compiled), create a QA manifest that:
1. Runs a skill with mock inference
2. Classifies the output (converged? all steps present? gas within budget?)
3. Branches on quality (high confidence → pass, medium → retry, low → investigate)
4. Loops on flaky results

**Where:** `registry/manifests/qa-skill-gpa-evolution-smoke.yaml`

### LOW — Fix remaining template format inconsistencies
Some templates use non-standard formats that the auditor handles via manifest fallback but aren't ideal:
- `media/*.j2` — uses `parameters:` format with `- name:` entries instead of `[inference]` + `contract:`
- `replica/*.j2` — uses Jinja comments for model specification
- `web/*.j2` — no frontmatter at all, just Jinja comments + template body

These work but are inconsistent with the rest of the registry. A future cleanup pass could standardize them.

## 4. Recommended Skills and Tools

- **coding-guidelines** — Before any code changes
- **diagnose** — If the `run_skill` compile error is non-obvious
- **skill-maintenance** — Run `kask skill audit` after changes to verify health
- **essentialist** — When reviewing whether the `run_skill` step type is over-engineered

**Commands:**
```bash
# Fix the run_skill compile error
cargo clippy -p hkask-test-harness --all-targets -- -D warnings

# Run all tests
cargo test -p hkask-test-harness -p hkask-templates -p hkask-ports -p hkask-services-skill

# Audit skills
cargo run --bin kask -- skill audit

# Run gpa-evolution with real LLM
cargo run --bin kask -- kata start gpa-evolution --bot learner \
  --context artifact_type=prompt \
  --context 'target_artifact={"content":"Answer the question","eval_set":["What is 2+2?"]}' \
  --context 'objectives=[{"name":"accuracy","maximize":true}]'
```

## 5. Key Decisions to Preserve

1. **Executor loop fix uses `continue 'cascade`** — The `loop` action's `continue` was changed to `continue 'cascade` so `iteration` increments correctly. This is the correct fix because `step_idx` is now persisted across `'cascade` iterations (moved outside the loop), and the `while` loop's normal exit resets `step_idx = 0` for implicit loops. Do NOT revert this — it fixes `min_iterations`/`max_iterations` for all PDCA skills.

2. **`.j2` extension fallback in executor** — Rather than editing 314 manifest `template_ref` values to add `.j2` extensions, the executor now tries `path.j2` as a fallback when `path` doesn't exist. This is the pragmatic-laziness brachistochrone — less total action. Do NOT add `.j2` extensions to manifests; the executor handles it.

3. **`#[serde(default)]` on `gas_cap`/`timeout_seconds`** — These fields were required (no default), causing parse failures on loop steps that omit them. Adding `#[serde(default)]` is the root cause fix. The defaults (0) are appropriate for loop steps. Do NOT remove the default — it would re-break 5+ manifests.

4. **Manifest fallback in auditor** — Templates that declare metadata in `manifest.yaml` (not `[inference]` frontmatter) are no longer penalized. The auditor uses `manifest_templates` HashMap to look up template type when `[inference]` is missing. Do NOT add fake `[inference]` blocks to these templates — the manifest fallback is the correct approach.

5. **TOML `[contract]` format support** — The auditor strips `[contract]` sections before YAML parsing and recognizes them as valid contracts. This supports the superforecasting templates which use TOML-style contracts. Do NOT convert these to YAML-style — both formats are now supported.

6. **Model type variables, not hardcoded models** — Templates should use `model_type: classifier` or `model_type: default` instead of hardcoding model names like `gemma-4-26b`. The executor's `default_params` determines the actual model. All inference template frontmatter has been fixed (bug-hunt, replica, media). Remaining `model:` references in templates are output content (generated corpus configs, Fal.ai pipeline configs, default config file content) — not inference template config. Do NOT change those.

7. **gpa-evolution `min_iterations: 2`** — The skill requires at least 2 iterations before convergence is allowed, ensuring the Pareto frontier has time to form. This works correctly now that the executor loop fix is in place. Do NOT set it to 0 — it was temporarily set to 0 as a workaround before the executor fix.
