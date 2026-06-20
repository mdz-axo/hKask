# qa-script-builder — Continuation Handoff

**Date:** 2026-06-20  
**Session scope:** QA Script Builder skill scaffold + rJoule cost system implementation  
**Handoff to:** Next agent session continuing qa-script-builder development

---

## 1. Session Context

This session built the scaffold for a `qa-script-builder` skill and then implemented the rJoule dual-track cost system. The skill scaffold is structurally complete (SKILL.md + 4 Jinja2 templates + manifest.yaml) but not yet registered in the skill registry or tested in the `kask chat` runtime. Mid-session, a significant design shift occurred: CNS integration was reworked from "passive observability" to "active algedonic signalling" (direct QA → Curator escalation). The templates were then updated to align with the rJoule cost system (250,000 gas = 1 rJ, API costs from classifier config not manifest). The skill builder work is ~80% structurally complete; the remaining 20% is registration, testing, and runtime wiring.

## 2. What Was Done

### Skill Scaffold (`.agents/skills/qa-script-builder/`)

| File | Status |
|------|--------|
| `SKILL.md` | Complete — 4-phase pipeline (Discover→Design→Generate→Validate), algedonic signalling section, rJoule-aligned gas config, common script patterns |
| `registry/templates/qa-script-builder/manifest.yaml` | Complete — 4 templates registered |

### Skill Templates (`registry/templates/qa-script-builder/`)

| Template | Status |
|----------|--------|
| `qa-discover.j2` | Updated — rJoule gas fields, alert requirements in output schema |
| `qa-design.j2` | Updated — alert topology in output, alert config on steps |
| `qa-generate.j2` | Updated — `cost_per_token` removed, `gas_per_function` added, `monthly_subscriptions_urj`, rJoule cost estimates in output |
| `qa-validate.j2` | Updated — W3 gas check uses µrJ, 3 alert validation rules (E11/E12/E13), 3 alert warnings (W8/W9/W10) |

### rJoule Cost System (implemented in code)

| File | Changes |
|------|---------|
| `crates/hkask-services-classify/src/classify_impl.rs` | `ClassifyResult` gained `prompt_tokens`, `completion_tokens`, `cost_urj`. `ChatResponse` parses `usage`. `ClassifierDef`/`ClassifierConfig` gained `cost_input_nj_per_token` + `cost_output_nj_per_token`. Error path parses usage from error body. |
| `crates/hkask-test-harness/src/qa_script.rs` | `CostTracker` added. `GasConfig`: `cost_per_token` removed, `gas_per_function` added. `ClassifyResult` gains token/cost fields. `QaScriptReport` gains `cost: CostSummary`. `execute_classify`/`execute_loop` track gas + API cost from classify result. Verification invariant: `gas_used == step_count × 100`. `alert_threshold` emits CNS warning. |
| `crates/hkask-cli/src/commands/qa.rs` | Classify closure propagates token counts + `cost_urj`. Cost summary displayed with µrJ/rJ conversions. |
| `registry/classify/qa-triage.yaml` | Added `cost_input_nj_per_token: 30`, `cost_output_nj_per_token: 60` |
| `registry/classify/qa-feedback.yaml` | Same |

### Specifications & Plans

| File | Status |
|------|--------|
| `docs/architecture/specs/rjoule-cost-system.md` | Complete — unit system, dual-track model, derivation, CostTracker design, verification invariants, discussion of 0.02 kWh |
| `docs/plans/rjoule-cost-tracking-implementation.md` | Complete — 6-phase plan, gap audit (4 missing CNS spans, 4 structural gaps) |

### Build Status

- `cargo check`: clean, zero warnings
- `cargo test -p hkask-test-harness`: 57 passed, 0 failed
- `cargo test -p hkask-services-classify`: all pass
- All changes compile on stable Rust

## 3. What Remains

### HIGH — Register the qa-script-builder skill

The scaffold exists but the skill is not registered in the hKask skill registry. The skill won't appear in `kask skill list` and can't be activated by name until registered.

- **What:** Add `qa-script-builder` to the skill registry (likely `registry/skills/` or a database-backed registry)
- **Dependencies:** The manifest.yaml at `registry/templates/qa-script-builder/manifest.yaml` is the canonical crate manifest
- **Strategy:** Use `kask skill publish --name qa-script-builder` if that command exists, or manually insert into the registry database via `hkask-templates::SqliteRegistry`
- **Verification:** `kask skill list` shows `qa-script-builder` with visibility "Public"

### MEDIUM — Test the templates in chat runtime

The Jinja2 templates have been structurally validated (front matter, contract, inference config) but not tested with actual variable injection in the `kask chat` runtime.

- **What:** Run each template through the chat runtime with test data
- **Files:** `registry/templates/qa-script-builder/qa-*.j2`
- **Strategy:** Execute `kask chat --template qa-script-builder/qa-discover` with test variables, verify JSON output conforms to contract
- **Verification:** Each template produces valid JSON matching its declared contract input/output types

### MEDIUM — Add qa-script-builder to AGENTS.md skill table

The skill isn't listed in the AGENTS.md "Core Development" or "Specialized" tables.

- **File:** `hKask/AGENTS.md`
- **What:** Add a row for `qa-script-builder` under "Specialized" skills with activation conditions

### MEDIUM — Implement Phase 1 of rJoule plan (4 missing CNS spans)

The rJoule spec declares 6 verification invariants but only 2 CNS spans are emitted.

- **File:** `crates/hkask-test-harness/src/qa_script.rs`
- **What:** Emit `cns.qa.cost.api_untracked`, `cns.qa.cost.step_untracked`, `cns.qa.cost.cap_exceeded`, `cns.qa.cost.missing_token_data`
- **Reference:** `docs/plans/rjoule-cost-tracking-implementation.md` Phase 1

### LOW — De-duplicate GasConfig across crates

Three separate `GasConfig` structs exist with different fields:
- `hkask-test-harness/src/qa_script.rs` — rJoule-complete (updated this session)
- `hkask-services-kata/src/kata_impl/manifest.rs` — still has dead `cost_per_token`
- `hkask-types/src/bundle/config.rs` — still has dead `cost_per_token`

## 4. Recommended Skills and Tools

For the next agent continuing qa-script-builder work:

- **coding-guidelines** — Surgical changes only; don't refactor adjacent code
- **skill-manager** — For registering the skill in the registry
- **deep-module** — If consolidating the three GasConfig structs
- **diagnose** — If template rendering fails in chat runtime

Build commands:
```bash
cargo check                               # verify workspace compiles
cargo test -p hkask-test-harness          # verify runner tests pass
cargo test -p hkask-services-classify     # verify classify tests pass
cargo test -p hkask-cli                   # verify CLI compiles
```

## 5. Key Decisions to Preserve

1. **API costs flow from classify service, not manifest.** Provider pricing lives in `registry/classify/*.yaml` (`cost_input_nj_per_token`, `cost_output_nj_per_token`). The manifest's `GasConfig` only has `gas_per_function` for internal software costs. Do not put API pricing back into manifests.

2. **250,000 gas = 1 rJ, not 500,000.** Revised from 0.01 kWh to 0.02 kWh per function call to account for infrastructure overhead (CNS, tracing, registry, YAML parsing) and provisioned-vs-utilized energy per SCI specification. 1 gas = 4 µrJ.

3. **CNS is active signalling, not passive logging.** QA classify steps raise direct algedonic alerts (via `alert:` config) that flow to the Curator. `cns_span` strings are tracing targets for logs — orthogonal to `alert` escalation.

4. **Integer micro-rJ (µrJ) for all internal accounting.** No floating-point. 1 µrJ = 0.000001 rJ = $0.000001. This is future-proofed for v2 tokenization.

5. **No backward compatibility accommodations.** `cost_per_token` is fully removed from `qa_script::GasConfig`. Scripts must use the new rJoule fields. The kata and bundle GasConfigs are separate domains and were intentionally not touched.

6. **Gas tracks hKask-internal only.** Software function calls (CNS, registry, parsing, command execution). API costs track external services. Merged into rJ for unified reporting but tracked separately.
