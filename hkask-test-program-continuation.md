# hKask Test Program Evolution — Continuation Prompt

## Session Purpose

Evolve hKask's test program from ad-hoc tests to a DDMVSS-governed behavioral testing discipline. Integrate development skills (TDD, coding-guidelines, diagnose, etc.) into DDMVSS curation protocols. Extend `hkask-mcp-spec` with test/skill governance tools. Write behavioral tracer-bullet tests at deep seams.

## Progress Summary

### Completed ✅

1. **Test inventory** (`docs/status/test-inventory.md`) — Full seam inventory across 11 crates + 20 MCP servers with depth analysis, DDMVSS invariant mapping, and priority-ordered test writing plan. Identifies 17 crates/MCPs with zero test modules.

2. **Test program specification** (`docs/specifications/test-program.md`) — DDMVSS self-applying spec covering all 9 categories: Domain (seam ontology, hLexicon terms), Capability (test verbs as attenuatable capabilities), Interface (MCP/CLI/API equivalence), Composition (goal decomposition), Trust (threat model), Observability (cns.test.* spans), Persistence (bitemporal results), Lifecycle (Git SHA versioning), Curation (Merge/Revise/Defer/Discard gradient).

3. **Testing Standards** (`docs/specifications/TESTING_STANDARDS.md`) — Written by the other agent. 401-line doc with test classification, DDMVSS category→strategy mapping, traceability matrix (4 tested, 21 gaps), skill integration table, P0–P3 gap priority.

4. **DDMVSS §12 Testing Protocol** — Added by the other agent to `docs/architecture/DDMVSS.md`. TP-1 through TP-7 principles, skill references, category→test strategy, self-application clause.

5. **PRINCIPLES.md** — Added **P8** (no test without an invariant) and **C8** (test depth matches module depth) with verification commands.

6. **AGENTS.md** — Added Test Program section, updated constraint reference to P1–P8/C1–C8, added 6 doc entries.

7. **OPEN_QUESTIONS.md** — Added TQ-1 through TQ-9 covering: mechanical vs LLM completeness, coherence calibration, skill enforcement, property-based testing boundaries, integration test isolation, CNS variety counters, skill-bundler+TDD composition, keystore zero-tests (CRITICAL), mcp-spec zero-tests (HIGH).

8. **Behavioral tests for spec_types.rs** — 38 tracer-bullet tests in `crates/hkask-storage/src/spec_types.rs` covering: SpecCategory/DomainAnchor/SpecId roundtrips, GoalSpec completeness/coherence/depth, Spec completeness/coherence/collection_coherence/drift, CurationDecision display, SpecCurationRecord coherence clamping, Criterion satisfaction, SpecStore roundtrip via InMemorySpecStore adapter.

9. **hLexicon terms** — Added 7 terms to bootstrap: `diagnose`, `verify` (KnowAct), `trace`, `deepen`, `register`, `handoff` (FlowDef). Total bootstrap now 23 terms.

10. **Other agent's work** — Refactored `GasBudgetManager` out of `CyberneticsLoop`, added `TestClassification`/`TestTraceability`/`TestVerifyResponse` to spec MCP types.

### All tests passing ✅
- `cargo test -p hkask-storage --lib` — 59 passed (38 new + 21 existing)
- `cargo test -p hkask-types --lib` — 45 passed
- `cargo test -p hkask-cns --lib` — 93 passed
- `cargo check --workspace` — clean
- `cargo clippy -p hkask-types -p hkask-storage -- -D warnings` — clean

## Key Decisions Made

1. **InMemorySpecStore** uses `RefCell<HashMap>` for interior mutability — matches the `&self` signature of the `SpecStore` trait, which the real `SqliteSpecStore` implements with SQL transactions. This is the correct test adapter pattern.

2. **CurationDecision has 3 variants** (Merge/Discard/Revise), not the 4 from DDMVSS §5.9. `Defer` was removed in v0.22.0. The test `curation_decision_has_exactly_three_variants` documents this.

3. **SpecCategory has 4 live variants** (Domain/Capability/Interface/Composition), not the 9 from DDMVSS §3. Trust/Observability/Persistence/Lifecycle/Curation are not `SpecCategory` enums — they're DDMVSS categories that apply to all specs, not separate storage buckets. Documented in test.

4. **hLexicon term additions** — Only added terms that have ≥2 consumers (spec program doc + skill mapping + tool surfaces). Per P1/P2, no term without two consumers.

5. **Test program doc and testing standards doc are complementary** — `test-program.md` is the *what* (DDMVSS-classified spec, invariants, tool surfaces), `TESTING_STANDARDS.md` is the *how* (procedures, classifications, skill bundles, traceability matrix).

## Architecture Context

### Deep Seams (Test Priority Order)

1. **`CompletenessCheck`** — `GoalSpec::is_complete()` and `Spec::is_complete()` — deepest possible seam (1 method). ✅ Tested.
2. **`SpecCategory`** — enum roundtrip. ✅ Tested.
3. **`CurationDecision`** — enum display. ✅ Tested.
4. **`GoalSpec`** — completeness, coherence, depth limit. ✅ Tested.
5. **`Spec`** — completeness, coherence, collection_coherence, drift. ✅ Tested.
6. **`SpecStore`** — save/load/delete/list roundtrip. ✅ Tested (via InMemorySpecStore adapter).
7. **`SpecCurator`** — evaluate/reconcile/cultivate. ❌ Not yet tested (lives in `hkask-agents`).
8. **`CNS` algedonic thresholds** — ✅ Already has good test coverage (10 modules).
9. **`hkask-keystore`** — ❌ 0 test modules (CRITICAL security gap, TQ-8).
10. **`hkask-mcp-spec`** — ❌ 0 test modules (HIGH gap, TQ-9).

### File Locations (Key)

| Artifact | Path |
|----------|------|
| Spec types + tests | `crates/hkask-storage/src/spec_types.rs` |
| Curation types | `crates/hkask-types/src/curation.rs` |
| Goal types | `crates/hkask-types/src/goal.rs` |
| Event/span types | `crates/hkask-types/src/event.rs` |
| Lexicon | `crates/hkask-types/src/lexicon.rs` |
| Spec MCP types | `mcp-servers/hkask-mcp-spec/src/types.rs` |
| Spec MCP main | `mcp-servers/hkask-mcp-spec/src/main.rs` |
| DefaultSpecCurator | `crates/hkask-agents/src/curator_agent/spec_curator.rs` |
| DDMVSS spec | `docs/architecture/DDMVSS.md` |
| Principles | `docs/architecture/PRINCIPLES.md` |
| Test inventory | `docs/status/test-inventory.md` |
| Test program spec | `docs/specifications/test-program.md` |
| Testing standards | `docs/specifications/TESTING_STANDARDS.md` |
| Open questions | `docs/OPEN_QUESTIONS.md` |
| AGENTS.md | `AGENTS.md` |

## Next Steps (Prioritized)

### P0: hkask-keystore behavioral tests (CRITICAL — security, zero coverage)

`hkask-keystore` has 0 test modules and handles AES-256-GCM encryption, HKDF-SHA256 master key derivation, and OS keychain integration. This is the highest-priority security gap.

Key seams to test:
- `Encryption::encrypt` / `Encryption::decrypt` roundtrip
- `MasterKey::derive` determinism (same inputs → same outputs)
- `Keychain` store/retrieve/delete operations
- Error variants (wrong key, missing key, expired key)

### P1: hkask-mcp-spec behavioral tests (HIGH — DDMVSS governance surface)

The MCP spec server has 8+4 tool surfaces and 0 test modules. The `InMemorySpecStore` adapter from `spec_types.rs` can be reused here.

Key seams to test:
- `SpecServer::spec_goal_capture` — creates spec with goals
- `SpecServer::spec_curate_evaluate` — evaluates spec completeness
- `SpecServer::spec_curate_cultivate` — cultivates collection coherence
- `SpecServer::spec_graph_validate` — validates collection completeness
- Capability verification on each tool

### P2: SpecCurator behavioral tests

`DefaultSpecCurator` lives in `crates/hkask-agents/src/curator_agent/spec_curator.rs`. It depends on `WebID`, `NuEvent`, `LoopMessage` — more complex to instantiate in tests. Need a minimal mock or test adapter.

Key seams:
- `evaluate(spec, verbs)` — complete spec → Merge, empty goals → Discard, partial → Revise
- `reconcile(specs, verbs)` — one record per spec
- `cultivate(specs)` — coherence score ∈ [0.0, 1.0], below threshold → CurationDepthExceeded

### P3: Tool handler implementations (spec/test/*, spec/skill/*)

The other agent added types (`TestClassification`, `TestTraceability`, `TestVerifyResponse`) but no handler implementations. Need:
- `spec/test/invariant` tool handler in `SpecServer`
- `spec/test/verify` tool handler
- `spec/skill/register` request/response types + handler
- `spec/skill/evaluate` request/response types + handler
- CLI equivalents (`kask test invariant`, `kask test verify`, `kask skill register`, `kask skill evaluate`)
- API route equivalents

### P4: CNS test variety spans

Add `cns.test.*` to `CANONICAL_NAMESPACES` in `crates/hkask-types/src/event.rs` and emit spans from test runs.

### P5: GoalState behavioral tests

`crates/hkask-types/src/goal.rs` has `GoalState` with `can_transition_to` state machine and existing tests in `crates/hkask-storage/src/goals.rs`. Review for behavioral alignment and add `is_terminal()` + `can_transition_to()` invariant tests if missing.

## Constraints

- **No `todo!()` or `unimplemented!()`** — P6 enforced
- **P8**: Every test must verify a stated behavioral property of a public seam
- **C8**: Test depth matches module depth — deepen before testing shallow seams
- **No visual UI** — headless only (CLI/MCP/API)
- **DDMVSS categories** govern all spec artifacts
- **Curation gradient**: Merge/Revise/Discard (3 variants, not 4 — Defer removed v0.22.0)
- **SpecCategory**: 4 variants (Domain/Capability/Interface/Composition), not 9

## Verification Commands

```bash
cargo test -p hkask-storage --lib          # 59 tests (38 new spec_types)
cargo test -p hkask-types --lib             # 45 tests (including lexicon)
cargo test -p hkask-cns --lib               # 93 tests (including gas budget)
cargo clippy -p hkask-storage -- -D warnings
cargo clippy -p hkask-types -- -D warnings
cargo check --workspace
```