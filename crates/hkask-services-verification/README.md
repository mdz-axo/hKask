# hkask-services-verification — Magna Carta Verification

Loads YAML assertion manifests from `.agents/skills/magna-carta-verifier/manifests/` and runs structural audits (grep-based) against the codebase. Behavioral probes and other assertion methods requiring runtime execution are reported as "gap" — assertions defined but not yet automatically verified. Extracted from `hkask-services-core` (see `tasks/plan-core-scope-contraction.md`, Task 1.2).

**Version:** v0.31.0 | **Crate:** `hkask-services-verification`

## Exports

| Type | Purpose |
|------|---------|
| `VerificationService` | Entry point — `verify(filter)` returns a `VerificationReport`; `verify_json(filter)` returns JSON |
| `VerificationReport` | Aggregate result (per-principle results + totals: pass/fail/gap/skip) |
| `PrincipleResult` | One Magna Carta principle + its `AssertionResult` list |
| `AssertionResult` | Per-assertion outcome (id, name, status, findings, recommendations) |
| `Manifest` / `Assertion` | Public manifest types (principle, display_name, assertions) |

## Manifest location

Resolved at runtime via `manifest_dir()` / `find_crate_dir()`, trying in order:
1. `.agents/skills/magna-carta-verifier/manifests` (CWD-relative)
2. `../.agents/skills/magna-carta-verifier/manifests` (one level up)
3. `CARGO_MANIFEST_DIR/../../.agents/skills/magna-carta-verifier/manifests` (workspace-root fallback)

The `../../` fallback makes discovery relocation-safe — it resolves to the workspace root from any `crates/hkask-services-*` location.

## Dependencies

- `serde` / `serde_json` / `serde_yaml_neo` — manifest (de)serialization + JSON output
- No path dependencies; no coupling back to `hkask-services-core`