# qa-script-builder — Session Handoff

**Date:** 2026-06-20 (session 2)
**Session scope:** Persona-driven scenario generation (Phase 0), rJoule CNS span closure, AGENTS.md registration
**Handoff to:** Next agent session continuing qa-script-builder development

---

## 1. What Was Done

### PERSONA-DRIVEN SCENARIO GENERATION (Phase 0)

Added a new Phase 0 to the qa-script-builder pipeline: persona-driven scenario generation using Falstaffian perspective rotation and grill-me adversarial probing.

**New template:** `registry/templates/qa-script-builder/qa-persona.j2`
- Input: `persona` ("You are an SRE") + `goal` ("monitor flake rates") + `workspace_hint` (optional)
- Process: 4-5 Falstaffian rotations (obvious, shadow, adjacent, inversion, wildcard) + grill-me probing
- Output: `scenario_set` — array of 3-5 hardened scenarios, each with `user_intent`, `testing_angle`, `failure_mode`, `suggested_tool`, `alert_posture`, `gas_environment`, `stress_target`
- Temperature: 0.8 (encourages diversity)
- Energy cap: 4096

**Modified template:** `registry/templates/qa-script-builder/qa-discover.j2`
- Contract now accepts `scenario: object` as alternative input alongside `user_intent: string`
- Body includes `{% if scenario %}` block that renders persona context (persona, goal, rotation, stress target, pre-identified failure mode, etc.)
- Non-persona path unchanged — backwards compatible

**Modified manifest:** `registry/templates/qa-script-builder/manifest.yaml`
- Now registers 5 templates (was 4)
- Description updated: "persona→discover→design→generate→validate"

**Modified SKILL.md:** `.agents/skills/qa-script-builder/SKILL.md`
- New Phase 0 section documenting the persona pipeline
- Template table now shows 5 phases (0→4)
- Workflow section split into "Persona-Driven Path" and "Direct Path"
- Front matter description updated

### rJOULE CNS SPAN CLOSURE (Phase 1 completion)

Two CNS spans were already implemented in the code. Two were added:

| Span | Status |
|------|--------|
| `cns.qa.cost.api_untracked` | Already in code (qa_script.rs:601-610) |
| `cns.qa.cost.missing_token_data` | Already in code (qa_script.rs:591-599) |
| `cns.qa.cost.cap_exceeded` | **Added** in CLI qa.rs:419-425 — emits `tracing::warn!` with `manifest_id`, `total_urj`, `cap_urj` |
| `cns.qa.cost.step_untracked` | **Added** in qa_script.rs:457-466 — captures `gas_before` per step, emits warning if gas counter unchanged after step |

### AGENTS.md REGISTRATION

Added `qa-script-builder` to the Specialized skills table in `hKask/AGENTS.md` (after `logo-builder`).

---

## 2. Build & Test Status

```
cargo check              → Clean, zero warnings
cargo test -p hkask-test-harness → 57 passed, 0 failed
manifest.yaml             → Valid YAML (python3 yaml parse)
```

**⚠ hkask-cli compile error:** Pre-existing, unrelated to our changes. `reqwest::Response::text` ownership issue in `src/commands/qa.rs` (the `runpod_list_machines` function). hkask-cli lib won't compile for tests. Our `cns.qa.cost.cap_exceeded` change in `commands/qa.rs` is in the `run_script` function (line 417+) which is unaffected by the pre-existing error.

---

## 3. File Inventory

| File | Status | Changes |
|------|--------|---------|
| `registry/templates/qa-script-builder/qa-persona.j2` | **NEW** | Phase 0 persona template |
| `registry/templates/qa-script-builder/qa-discover.j2` | Modified | Added `scenario` input contract + persona context block |
| `registry/templates/qa-script-builder/manifest.yaml` | Modified | Registered `qa-persona` template, updated descriptions |
| `.agents/skills/qa-script-builder/SKILL.md` | Modified | Added Phase 0, 5-phase pipeline, persona-driven workflow |
| `hKask/AGENTS.md` | Modified | Added qa-script-builder to Specialized table |
| `crates/hkask-test-harness/src/qa_script.rs` | Modified | Added `gas_before` tracking + `step_untracked` CNS span |
| `crates/hkask-cli/src/commands/qa.rs` | Modified | Added `cap_exceeded` CNS span (tracing::warn!) |
| `docs/handoffs/qa-script-builder-2026-06-20.md` | Modified | This handoff |

---

## 4. What Remains

### HIGH — Test templates in chat runtime
The 5 templates have been structurally validated but not tested with actual variable injection in `kask chat`. Requires inference API keys.

### MEDIUM — Fix hkask-cli pre-existing compile error
`reqwest::Response::text` ownership in `runpod_list_machines`. Blocks `cargo test -p hkask-cli` but not `cargo check`.

### LOW — De-duplicate GasConfig across crates
Three GasConfig structs exist. The kata and bundle ones still have dead `cost_per_token`. Handoff says "separate domains, intentionally not touched."

### LOW — Optional caveman/essentialist modes
The `qa-persona.j2` template doesn't include caveman (compression) or essentialist (pruning) as explicit modes. These would be best added as boolean flags on `qa-generate.j2` (caveman) and `qa-validate.j2` (essentialist).

---

## 5. Architecture Notes

**Pipeline flow (persona-driven):**
```
qa-persona.j2 → scenario_set → qa-discover.j2 → discovery → qa-design.j2 → topology → qa-generate.j2 → manifest → qa-validate.j2 → report
     ↑                                                                                                                       │
     └── one invocation generates 3-5 scenarios ── each feeds independently through phases 1-4 ──────────────────────────────┘
```

**Pipeline flow (direct):**
```
qa-discover.j2 → discovery → qa-design.j2 → topology → qa-generate.j2 → manifest → qa-validate.j2 → report
```

**Persona scenarios are non-deterministic by design.** Temperature 0.8 + Falstaffian rotation + grill-me probing produces different scenarios on each invocation. The `testing_angle` field enforces uniqueness across the set in a single invocation.

**The persona template does NOT hard-code MCP server tool lists or persona definitions.** It lets the user describe their persona in natural language and infers the stress target from context. This avoids maintenance debt.

---

## 6. Key Design Decisions Preserved

1. **Persona is free-form, not an enum.** No predefined role taxonomy. The user writes "You are an SRE" or "You are a security auditor" — the template works with whatever they provide.

2. **Caveman and essentialist are deferred to mode flags, not separate templates.** The persona template uses Falstaffian + grill-me for generation. Compression/minimization would be flags on generate/validate.

3. **No MCP server coupling.** The `stress_target` field is a human-readable label, not a machine-enforced constraint. The persona template doesn't need to know which MCP servers exist.

4. **Backwards compatible.** `qa-discover.j2` still works with bare `user_intent` — the `scenario` input is optional.
