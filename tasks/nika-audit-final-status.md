# Nika Audit — Final Status

**Date:** 2026-07-21 · **Author:** agent (Zed)

## All Tasks — Final Status

### Phase A — Foundation ✅
| Task | Status | Files |
|---|---|---|
| A1 — `TemplateError::SkillLoad` + `Frontmatter` variants | ✅ Done | `ports.rs` |
| A2 — Migrate `skill_loader.rs` from `anyhow` to typed errors | ✅ Done | `skill_loader.rs` |
| A3 — `SkillFinding` + `ManifestResolveError` types | ✅ Done | `ports.rs`, `lib.rs` |

### Phase B — Core ✅
| Task | Status | Files |
|---|---|---|
| B1 — Inject `SkillReader` trait for purity | ✅ Done | `ports.rs`, `skill_loader.rs` |
| B2 — `parse_front_matter` returns `Err` on missing frontmatter | ✅ Done | `skill_loader.rs` |
| B3 — `infer_domain_from_registry` splits not-found vs malformed | ✅ Done | `skill_loader.rs` |
| B4 — `resolve_manifest` returns typed `Result` | ✅ Done | `manifest_loader.rs` |

### Phase C — Polish ✅ (C1 deferred)
| Task | Status | Files |
|---|---|---|
| C1 — `Skill` fields `pub(crate)` | ❌ Deferred | `Skill` is a DTO; `#[non_exhaustive]` (D1) is sufficient |
| C2 — Replace `.expect()` on pool-get with typed error | ✅ Done | `registry_sqlite.rs`, `registry.rs`, `bundle/mod.rs` |
| C3 — `deny_unknown_fields` on `SkillFrontMatter` | ✅ Done | `skill_loader.rs` |

### Phase D — Nika Verb Follow-Up ✅ (D4 deferred)
| Task | Status | Files |
|---|---|---|
| D1 — `#[non_exhaustive]` on public structs | ✅ Done | `registry.rs`, `bundle/manifest.rs` |
| D2 — `code()` + `is_transient()` on `TemplateError` | ✅ Done | `ports.rs` |
| D3 — `CANCEL SAFETY` docs on `execute_manifest` | ✅ Done | `executor.rs` |
| D4 — Stall guard | ❌ Deferred | Fails essentialist G1 — optimization, not correctness |

### Phase E — Lexicon Rationalization ✅
| Task | Status | Files |
|---|---|---|
| E1 — Audit existing `lexicon_terms` values | ✅ Done | `tasks/lexicon-audit.md` |
| E2 — `LexiconTerm` enum | ❌ Rejected | 347-variant enum is shallow; `validate_entry`→error is the fix |
| Lexicon → error enforcement | ✅ Done | `registry.rs`, `registry_sqlite.rs` |
| Missing terms added | ✅ Done | `vocabulary.rs` (+3: `accommodate`, `engage`, `repair`) |

### Phase F — CNS Namespace Reorganization ✅
| Task | Status | Files |
|---|---|---|
| F1 — Register `cns.skill.*` subdomains explicitly | ✅ Done | `event.rs` (24 entries) |
| F2 — Rename tracing targets to subdomains | ✅ Done | `executor.rs`, `skill_impl.rs` |
| F3 — CI creep gate (`check-cns-creep.sh`) | ✅ Done | `scripts/check-cns-creep.sh` |
| Register all 64 unregistered targets | ✅ Done | `event.rs` (+63 entries across 16 domains) |

### Additional — Together AI Dead Code Removal ✅
| Task | Status | Files |
|---|---|---|
| Delete `together.rs` | ✅ Done | `adapter_router/together.rs` (deleted) |
| Remove backend registration | ✅ Done | `adapter_router/mod.rs` |
| Remove `CostModel::together()` + `ProviderCapability::together()` | ✅ Done | `provider_cost.rs` |
| Fix `CostModel::tinker()` to use `ProviderId::Tinker` | ✅ Done | `provider_cost.rs` |
| Remove `cns.training.provider.together.*` namespaces | ✅ Done | `event.rs` |
| Migrate tests from `ProviderId::Together` to `ProviderId::Runpod` | ✅ Done | `adapter_router/mod.rs`, `adapter_port.rs`, `live_adapter.rs` |
| Clean up doc comments in `hkask-mcp-training` | ✅ Done | `huggingface.rs`, `lib.rs`, `adapters.rs` |

## Open Questions — Resolved

1. **Lexicon (directive 1):** ✅ Resolved — the lexicon is a closed vocabulary (347 terms). Enforcement promoted from warning to error. No `LexiconTerm` enum needed.
2. **CNS exact-match (directive 2):** ✅ Resolved — keep hierarchical `is_canonical` (needed for dynamic namespaces like `cns.agent_pod.{pod_id}`). Defense against creep is the CI gate for static targets.
3. **Stall guard (directive 4, D4):** ✅ Resolved — deferred. Fails essentialist G1 (deletion test): `max_iterations` cap already prevents infinite loops; the stall guard is an optimization, not a correctness fix.
4. **`#[non_exhaustive]` external consumers (D1):** ✅ Resolved — `Skill` is only constructed in-workspace. `#[non_exhaustive]` is safe.

## Validation

- `cargo test -p hkask-templates --lib`: 35 passed, 0 failed
- `cargo test -p hkask-templates --test lexicon_coverage`: 1 passed, 0 failed
- `cargo test -p hkask-adapter --lib`: 47 passed, 0 failed, 7 ignored
- `cargo test -p hkask-mcp-training --lib`: 61 passed, 0 failed, 2 ignored
- `cargo test -p hkask-types --lib cns`: 7 passed, 0 failed
- `cargo test -p hkask-types --lib proptest`: 6 passed, 0 failed
- `scripts/check-cns-creep.sh`: all `cns.*` targets registered
- `cargo check --workspace`: no errors (excluding pre-existing `retry_pod_matrix` issue)
