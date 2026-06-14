---
title: "Bundler Completion Plan — Remaining Tasks"
audience: [project maintainers, contributors]
last_updated: 2026-06-11
version: "0.27.0"
status: "Active"
domain: "Skill Bundling"
mds_categories: [domain, composition, lifecycle]
---

# Bundler Completion Plan — Remaining Tasks

This plan tracks the remaining work after the initial bundler service extraction
(2026-06-11). The architecture is now correct — `BundleService` in `hkask-services`
with CLI and API surfaces delegating to it — but three integration points remain.

---

## P0 — Essential (Must Complete)

### 1. Wire REPL `/bundle` slash command to real composition

**Status:** Not started  
**Files:** `crates/hkask-cli/src/repl/commands.rs`, `crates/hkask-cli/src/repl/mod.rs`

The REPL `/bundle SKILL1 SKILL2` slash command currently prints informational
text ("Composing bundle from: SKILL1 SKILL2 — use `kask bundle compose`").
It should call `BundleService::compose()` using the REPL's shared inference port
(`ReplState::inference_port`) and display the result.

**Changes needed:**
- In `handle_slash_command()`, the `"bundle" | "b"` arm's `skills_arg` branch
  should call `BundleService::compose()` via `rt.block_on()`
- Needs access to `state.service_context` (already available) and
  `state.inference_port` (already available)
- Display the composed manifest inline (same format as CLI `run_bundle`)

**Verify:** `/bundle coding-guidelines tdd` in REPL produces a valid manifest.

### 2. Wire REPL `/bundle apply <id>` to activate a bundle

**Status:** Not started  
**Files:** `crates/hkask-cli/src/repl/commands.rs`, `crates/hkask-cli/src/repl/mod.rs`

When a bundle is applied, it should become the active `process_manifest` for the
current agent, replacing any existing manifest. This means:
- Load the `BundleManifest` from the registry via `BundleService::apply()`
- Set `state.process_manifest = Some(bundle)`
- Rebuild `state.manifest_executor` with the new manifest
- The next chat turn will run the new cascade via `ChatService::execute_turn()`

**Changes needed:**
- In `handle_slash_command()`, the `"bundle" | "b"` arm's `"apply"` branch
  should call `BundleService::apply()` and update `ReplState`
- `ReplState` needs a method or the handler needs direct field access to
  rebuild `ManifestExecutor` (requires `McpDispatcher` and `acp_secret`)

**Verify:** `/bundle apply <id>` then chat — manifest cascade runs at turn time.

### 3. Wire REPL `/bundle off` to deactivate the current bundle

**Status:** Not started  
**Files:** `crates/hkask-cli/src/repl/commands.rs`

Clear `state.process_manifest` and `state.manifest_executor` back to `None`.
Subsequent turns run without a manifest cascade.

**Verify:** `/bundle off` then chat — no `[Manifest Context]` block in input.

### 4. Wire REPL `/bundle list` and `/bundle skills` to real data

**Status:** Not started  
**Files:** `crates/hkask-cli/src/repl/commands.rs`

Currently prints "use `kask bundle list` for full details". Should call
`BundleService::list()` and `BundleService::list_skills()` and display results
inline in the REPL.

**Verify:** `/bundle list` shows composed bundles. `/bundle skills` shows
loaded skills with polarity, domain, visibility.

---

## P1 — Important (Should Complete)

### 5. Add `// REQ:` tests for BundleService

**Status:** Not started  
**Files:** `crates/hkask-services/src/bundle.rs` (add `#[cfg(test)]` module)

Per P8 (Semantic Grounding), every test must carry a `// REQ:` tag from spec.
Testable behaviors:
- `compose()` rejects < 2 skills → `// REQ: bundle-min-skills`
- `compose()` returns existing bundle on exact match → `// REQ: bundle-smart-match`
- `compose()` errors on missing skill IDs → `// REQ: bundle-skill-not-found`
- `list()` returns empty vec when no bundles → `// REQ: bundle-list-empty`
- `apply()` errors on unknown ID → `// REQ: bundle-apply-not-found`
- `evolve()` errors on unknown ID → `// REQ: bundle-evolve-not-found`

The inference-dependent `compose()` path needs a mock `InferencePort` that
returns a pre-built `BundleManifest` JSON. This is the standard pattern for
testing LLM-dependent services.

### 6. Add `// REQ:` tests for CLI bundle commands

**Status:** Not started  
**Files:** `crates/hkask-cli/src/commands/bundle.rs` (add `#[cfg(test)]` module)

Test the CLI display formatting — not the business logic (that's tested in
BundleService). Verify that each `BundleAction` variant produces the expected
output format.

### 7. Add `// REQ:` tests for API bundle routes

**Status:** Not started  
**Files:** `crates/hkask-api/src/routes/bundles.rs` (add `#[cfg(test)]` module)

Test HTTP response shapes — status codes, JSON structure, error responses.
Use `axum::test` helpers with a mock `ApiState`.

---

## P2 — Optional (Nice to Have)

### 8. Template-based composition (bundler-compose.j2)

**Status:** Not started  
**Files:** `registry/templates/skill-bundler/` (create directory + templates)

Currently `BundleService::compose()` uses an inline prompt string. The
skill-bundler SKILL.md references `bundler-compose.j2`, `bundler-validate.j2`,
and `bundler-evolve.j2` templates. Creating these as proper Jinja2 templates
would:
- Allow the `ManifestExecutor` to run composition as a cascade step
- Enable template versioning and evolution independent of Rust code
- Match the dual-layer architecture (SKILL.md + registry templates)

### 9. Partial/similar match in `find_bundle_by_skills`

**Status:** Not started  
**Files:** `crates/hkask-templates/src/registry.rs`, `crates/hkask-templates/src/registry_sqlite.rs`

Currently `find_bundle_by_skills()` does exact set matching only. The
skill-bundler SKILL.md describes "smart matching" with three tiers:
- Exact match → offer to apply/evolve
- Partial/similar match → show similar bundles
- No match → compose new

Implementing partial matching (e.g., Jaccard similarity on skill ID sets)
would improve the user experience.

### 10. Indexed bundle lookup

**Status:** Not started  
**Files:** `crates/hkask-templates/src/registry_sqlite.rs`

`find_bundle_by_skills()` currently does `list_bundles()` → O(n) scan.
For large skill corpora, an indexed lookup (e.g., a `bundle_skills` join
table query) would be more efficient. The `bundle_skills` table already
exists in the SQLite schema — it's just not used for lookup.

---

## Verification Checklist

```
[ ] REPL /bundle compose produces valid manifest
[ ] REPL /bundle apply activates cascade at turn time
[ ] REPL /bundle off deactivates cascade
[ ] REPL /bundle list shows bundles
[ ] REPL /bundle skills shows loaded skills
[ ] BundleService tests pass with // REQ: tags
[ ] CLI bundle tests pass with // REQ: tags
[ ] API bundle tests pass with // REQ: tags
[ ] cargo check --workspace passes
[ ] cargo test --workspace passes
[ ] cargo clippy --workspace -- -D warnings passes
[ ] No println!() stubs remain in any bundle path
[ ] Dependency direction verified (no circular deps)
```
