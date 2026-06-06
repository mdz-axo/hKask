# hKask Test Program Evolution — Continuation Prompt v3

## Session Purpose

Evolve hKask's test program from ad-hoc tests to a DDMVSS-governed behavioral testing discipline. Integrate development skills (TDD, coding-guidelines, diagnose, etc.) into DDMVSS curation protocols. Extend `hkask-mcp-spec` with test/skill governance tools. Write behavioral tracer-bullet tests at deep seams.

## Progress Summary

### Completed ✅

1. **Test inventory** (`docs/status/test-inventory.md`) — Full seam inventory across 11 crates + 20 MCP servers with depth analysis, DDMVSS invariant mapping, and priority-ordered test writing plan.

2. **Test program specification** (`docs/specifications/test-program.md`) — DDMVSS self-applying spec covering all 9 categories.

3. **Testing Standards** (`docs/specifications/TESTING_STANDARDS.md`) — 401-line doc with test classification, DDMVSS category→strategy mapping, traceability matrix, skill integration table, P0–P3 gap priority.

4. **DDMVSS §12 Testing Protocol** — Added to `docs/architecture/DDMVSS.md`. TP-1 through TP-7 principles.

5. **PRINCIPLES.md** — P8 (no test without an invariant) and C8 (test depth matches module depth).

6. **AGENTS.md** — Test Program section updated.

7. **OPEN_QUESTIONS.md** — TQ-1 through TQ-9.

8. **Behavioral tests for `spec_types.rs`** — 38 tracer-bullet tests in `crates/hkask-storage/src/spec_types.rs`.

9. **hLexicon terms** — 7 terms added (diagnose, verify, trace, deepen, register, handoff).

10. **P0: hkask-keystore behavioral tests** — 31 tests across 4 modules:
    - `encryption.rs`: 10 tests (roundtrip, empty passphrase, wrong key, truncated ciphertext, derive_key determinism/differentiation/length, salt non-zero)
    - `master_key.rs`: 6 tests (HKDF determinism, domain separation, output length, InternalSecrets determinism/field independence/hex encoding)
    - `error.rs`: 7 tests (From conversions, Display format)
    - `keychain.rs`: 8 tests (construction, default, error display, From\<KeyringError\>, resolve env/derived)

11. **P1: hkask-mcp-spec type tests** — 20 tests for request/response types, TestClassification, TestTraceability, TestVerifyResponse.

12. **P2: SpecCurator behavioral tests** — 12 tests in `crates/hkask-agents/src/curator_agent/spec_curator.rs`:
    - evaluate: Merge/Discard/Revise decisions, coherence match, spec_id match
    - reconcile: one record per spec, decisions match evaluate
    - cultivate: coherent collection, discard removal, depth exceeded
    - Construction: threshold clamping, default threshold

13. **P4: CNS test variety spans** — Added `cns.test` to CANONICAL_NAMESPACES. 14 behavioral tests in `crates/hkask-types/src/event.rs`:
    - SpanNamespace: all namespaces valid, invalid panics, parse short/full form, from_str roundtrip, Display, short_name, cns.test valid
    - Phase: as_str roundtrip, backward compat, case insensitive, unknown→Sense
    - Span: new constructs full path

14. **P5: GoalState behavioral tests** — 23 tests in `crates/hkask-types/src/goal.rs`:
    - GoalState: 5 variants, roundtrip, case-insensitive, invalid→None, is_terminal, can_transition_to (Pending/Active/Blocked/Completed/Abandoned), IllegalGoalTransition Display+Error
    - Goal: new starts Pending/no parent/depth 0, transition legal/illegal, terminal rejection, completed_at, self-transition noop, can_have_subgoals

### All tests passing ✅
- `cargo test -p hkask-types --lib` — 76 passed
- `cargo test -p hkask-storage --lib` — 50 passed
- `cargo test -p hkask-cns --lib` — 88 passed
- `cargo test -p hkask-keystore --lib` — 31 passed
- `cargo test -p hkask-mcp-spec` — 20 passed
- `cargo test -p hkask-agents --lib` — 50 passed
- `cargo clippy -p hkask-types -p hkask-storage -p hkask-keystore -p hkask-agents -p hkask-mcp-spec -- -D warnings` — clean
- `cargo check --workspace` — clean

## Key Decisions Made

1. **InMemorySpecStore** uses `RefCell<HashMap>` for interior mutability — matches the `&self` signature of `SpecStore` trait, which `SqliteSpecStore` implements with SQL transactions.

2. **CurationDecision has 3 variants** (Merge/Discard/Revise), not the 4 from DDMVSS §5.9. `Defer` was removed in v0.22.0.

3. **SpecCategory has 4 live variants** (Domain/Capability/Interface/Composition). Trust/Observability/Persistence/Lifecycle/Curation are DDMVSS categories applied to all specs, not separate storage buckets.

4. **Keychain OS operations** — store/retrieve/delete depend on OS keyring and are not tested directly. The `From<KeyringError>` conversion for `NoEntry` currently maps to `Platform` (not `NotFound`), which is documented in the test.

5. **GoalState state machine** — Completed and Abandoned are terminal states that only allow self-transitions. Blocked can resume to Active. Pending can only transition to Active or Abandoned.

6. **cns.test** is the 16th canonical namespace, added for DDMVSS test observability.

7. **Pre-existing clippy warning** in `crates/hkask-types/src/loops/cybernetics.rs` — doc comment `//` changed to `//!` to separate list from paragraph. This was a pre-existing issue, not introduced by test changes.

## Architecture Context

### Deep Seams (Test Priority Order)

1. **`CompletenessCheck`** — `GoalSpec::is_complete()` and `Spec::is_complete()` — ✅ Tested
2. **`SpecCategory`** — enum roundtrip — ✅ Tested
3. **`CurationDecision`** — enum display — ✅ Tested
4. **`GoalSpec`** — completeness, coherence, depth limit — ✅ Tested
5. **`Spec`** — completeness, coherence, collection_coherence, drift — ✅ Tested
6. **`SpecStore`** — save/load/delete/list roundtrip — ✅ Tested
7. **`SpecCurator`** — evaluate/reconcile/cultivate — ✅ Tested (DefaultSpecCurator in hkask-agents)
8. **`CNS` algedonic thresholds** — ✅ Already has good test coverage
9. **`hkask-keystore`** — ✅ 31 behavioral tests (encryption, master key, error, keychain)
10. **`hkask-mcp-spec`** — ✅ 20 type tests (handlers are P3)
11. **`GoalState`** — ✅ 23 tests (state machine, transition, terminal)
12. **`SpanNamespace`/`Phase`** — ✅ 14 tests (validation, roundtrip, cns.test)

### Remaining Deep Seams (Not Yet Tested)

| Seam | Crate | Priority | Notes |
|------|-------|----------|-------|
| `GoalState` + `Goal` | hkask-storage/goals.rs | P2 | SQL persistence layer for goals — 5 existing tests |
| `NuEventStore` | hkask-storage/nu_event_store.rs | P2 | Bitemporal queries, event persistence — 2 existing tests |
| `CapabilityChecker` | hkask-types/capability | P2 | Token verification, attenuation — tests exist in capability module |
| `OCAPBoundary` | hkask-types/curation | P2 | Boundary enforcement — no tests yet |
| `DelegationToken` | hkask-types/capability | P2 | Token creation, verification, attenuation |
| `AllostericGate` | hkask-types/allosteric | P2 | MWC state function, gate config — tests exist |
| `TemplateType` | hkask-templates | P2 | Registry, cascade depth ≤ 7 — tests exist |
| `McpServer` tool dispatch | hkask-mcp | P3 | Core MCP runtime — 0 test modules, CRITICAL |
| `SpecServer` tool handlers | hkask-mcp-spec | P3 | 8 tool methods — need InMemorySpecStore + CapabilityChecker |
| `WebSearchPort` | hkask-mcp-web | P3 | External API integration — 0 test modules |
| `OcapPolicy` | hkask-mcp-ocap | P3 | Security boundary — 0 test modules |

### File Locations (Key)

| Artifact | Path |
|----------|------|
| Encryption + derive_key tests | `crates/hkask-keystore/src/encryption.rs` |
| Master key + HKDF tests | `crates/hkask-keystore/src/master_key.rs` |
| KeystoreError conversion tests | `crates/hkask-keystore/src/error.rs` |
| Keychain + resolve tests | `crates/hkask-keystore/src/keychain.rs` |
| SpecServer + types tests | `mcp-servers/hkask-mcp-spec/src/types.rs` |
| DefaultSpecCurator tests | `crates/hkask-agents/src/curator_agent/spec_curator.rs` |
| Spec types + InMemorySpecStore | `crates/hkask-storage/src/spec_types.rs` |
| CurationDecision + OCAPBoundary | `crates/hkask-types/src/curation.rs` |
| GoalState + Goal tests | `crates/hkask-types/src/goal.rs` |
| SpanNamespace + Phase tests | `crates/hkask-types/src/event.rs` |
| CapabilityChecker + DelegationToken | `crates/hkask-types/src/capability/verification.rs` |
| DDMVSS spec | `docs/architecture/DDMVSS.md` |
| Principles | `docs/architecture/PRINCIPLES.md` |
| Test inventory | `docs/status/test-inventory.md` |
| Test program spec | `docs/specifications/test-program.md` |
| Testing standards | `docs/specifications/TESTING_STANDARDS.md` |
| Open questions | `docs/OPEN_QUESTIONS.md` |
| CANONICAL_NAMESPACES (now includes cns.test) | `crates/hkask-types/src/event.rs` |

## Next Steps (Prioritized)

### P3: SpecServer tool handler tests (HIGH — DDMVSS governance surface)

The MCP spec server has 8 tool methods that depend on `Arc<dyn SpecStore + Send + Sync>`, `Arc<CapabilityChecker>`, and `WebID`. The `InMemorySpecStore` adapter from `spec_types.rs` can be reused. The `CapabilityChecker` is easy to construct with a known secret.

Key seams to test:
- `spec_goal_capture` — creates spec with goals, validates category/anchor parsing
- `spec_curate_evaluate` — evaluates spec completeness (complete→Merge, empty→Discard, partial→Revise)
- `spec_curate_cultivate` — cultivates collection coherence (threshold, categories_covered/missing)
- `spec_graph_validate` — validates collection completeness (violations, suggestions)
- Capability verification gate on each tool

**Challenge:** Tool methods use `rmcp` `Parameters<T>` wrapper. To call them directly in tests, you need to construct `Parameters<GoalCaptureRequest>` etc. Alternatively, extract the business logic into testable functions that don't depend on rmcp types.

**Recommended approach:** Extract the curation decision logic (already tested via SpecCurator) and focus on the capability verification gate and JSON response structure. The `SpecServer` method bodies are thin wrappers; the deep logic is already tested.

### P3: hkask-mcp runtime tests (CRITICAL — all MCP servers depend on this)

`hkask-mcp` has 0 test modules. The `McpServer` and `McpToolError` types are the foundation for all 20 MCP servers.

Key seams:
- `McpToolError` variants and JSON serialization
- `ServerContext` construction and credential extraction
- `validate_field!` macro behavior
- `ToolSpanGuard` span creation and error formatting

### P3: Tool handler implementations for spec/test/* and spec/skill/*

The other agent added `TestClassification`, `TestTraceability`, `TestVerifyResponse` types but no handler implementations. Need:
- `spec/test/invariant` tool handler in `SpecServer`
- `spec/test/verify` tool handler
- `spec/skill/register` request/response types + handler
- `spec/skill/evaluate` request/response types + handler
- CLI equivalents (`kask test invariant`, `kask test verify`, `kask skill register`, `kask skill evaluate`)
- API route equivalents

### P6: Capability verification tests (P2)

`OCAPBoundary` and `DelegationToken` are security-critical seams with no dedicated test module. The `capability` module has tests for HMAC operations and capability specs, but:
- `OCAPBoundary::explicit()`, `OCAPBoundary::denied()` — boundary construction
- `DelegationToken::new()`, `verify()`, `attenuate()` — token lifecycle
- `verify_delegation_token()` — the unified verification function used by all MCP servers

### P7: Remaining zero-test MCP servers

16 MCP servers still have 0 test modules. Priority order by security/surface:
1. `hkask-mcp-web` (HIGH — external API integration)
2. `hkask-mcp-ocap` (HIGH — security boundary)
3. `hkask-mcp-cns` (MEDIUM)
4. `hkask-mcp-keystore` (MEDIUM)
5. Others (LOW until they gain surface area)

## Constraints

- **No `todo!()` or `unimplemented!()`** — P6 enforced
- **P8**: Every test must verify a stated behavioral property of a public seam
- **C8**: Test depth matches module depth — deepen before testing shallow seams
- **No visual UI** — headless only (CLI/MCP/API)
- **DDMVSS categories** govern all spec artifacts
- **Curation gradient**: Merge/Revise/Discard (3 variants, not 4 — Defer removed v0.22.0)
- **SpecCategory**: 4 variants (Domain/Capability/Interface/Composition), not 9
- **GoalState**: 5 variants (Pending/Active/Completed/Blocked/Abandoned), Completed/Abandoned are terminal
- **CANONICAL_NAMESPACES**: Now has 16 entries including `cns.test`
- **Argon2id is intentionally slow** (~100ms per call) — minimize `derive_key` calls in keystore tests; use helpers that reuse results

## Verification Commands

```bash
cargo test -p hkask-storage --lib          # 50 tests (spec_types)
cargo test -p hkask-types --lib             # 76 tests (including goal + event)
cargo test -p hkask-cns --lib               # 88 tests (including gas budget)
cargo test -p hkask-keystore --lib          # 31 tests (encryption, master_key, error, keychain)
cargo test -p hkask-mcp-spec                # 20 tests (types)
cargo test -p hkask-agents --lib             # 50 tests (including spec_curator)
cargo clippy -p hkask-types -p hkask-storage -p hkask-keystore -p hkask-agents -p hkask-mcp-spec -- -D warnings
cargo check --workspace
```

## Recommended Skills

- **coding-guidelines** — Enforce P8 (no test without invariant), C8 (test depth matches module depth), surgical changes, simplicity first
- **TDD** — Red-green-refactor discipline for new test writing
- **diagnose** — If tests fail, use the diagnose skill to reproduce → minimize → hypothesize → instrument → fix → regression-test
- **zoom-out** — When unfamiliar with a crate's architecture, zoom out to understand how it fits into the bigger picture before writing tests

## Open Questions to Resolve

- **TQ-1**: Mechanical vs LLM completeness — **Resolved**: `is_complete()` is mechanical for criteria-based goals. Natural-language goals need LLM evaluation via `CurateEvaluate`.
- **TQ-8**: hkask-keystore zero tests — **Resolved**: 31 behavioral tests now cover all 4 modules.
- **TQ-9**: hkask-mcp-spec zero tests — **Resolved**: 20 type tests cover request/response types. Tool handler tests are P3.
- **TQ-2, TQ-3, TQ-4, TQ-5, TQ-6, TQ-7**: Still open. See OPEN_QUESTIONS.md for details.