# hKask Test Program Evolution — Continuation Prompt v4

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

15. **P6: OCAPBoundary + OcapCapability behavioral tests** — 20 tests in `crates/hkask-types/src/curation.rs`:
    - OcapTokenKind: all 3 variants, Copy/Clone/Debug/Hash/PartialEq, serde roundtrip (snake_case)
    - OcapCapability: String displays inner, Token displays canonical names, equality, serde roundtrip
    - OCAPBoundary: token() enforced+Token, explicit() enforced+String, denied() unenforced+String, token≠explicit, serde roundtrip
    - CurationThresholdConfig: default 0.7/0.5, custom values, empty YAML defaults, partial override

16. **P6: Capability verification tests** — Pre-existing comprehensive tests in `crates/hkask-types/src/capability/verification.rs`:
    - CapabilityChecker: new, verify same/different secret, verify_with_time, check, check_resource
    - verify_delegation_token: all 5 outcomes (Valid, InvalidSignature, Expired, InsufficientAccess, NoChecker)
    - require_write_access: ok for Write/Execute, err for Read
    - require_read_access: ok for all actions

17. **P6: DelegationToken lifecycle tests** — Pre-existing comprehensive tests in `crates/hkask-types/src/capability/mod.rs` (delegation_token_tests):
    - Construction, verify, is_expired, is_valid_for, grants_resource, holder/issuer
    - allows_write/allows_read, can_attenuate, attenuate produces child, attenuate returns None at max
    - base64 roundtrip, fingerprint, is_compatible_with
    - verify_attenuation_chain, validate_context_nonce, root_context_nonce
    - Serde roundtrip for DelegationResource and DelegationAction

18. **P3: hkask-mcp runtime tests** — 37 tests written (NOT YET RUNNABLE due to pre-existing git_cas compile errors):
    - `security.rs`: 16 tests (URL validation, SSRF protection, private IP classification)
    - `server.rs`: 21 tests (McpErrorKind is_retryable/requires_intervention/Display, McpToolError constructors/JSON/Display, McpToolOutput new/with_metadata/JSON, CredentialRequirement required/optional, validate_identifier accept/reject/boundary, validate_tool_url, classify_http_error status code mapping)

### All previously-passing tests still passing ✅
- `cargo test -p hkask-types --lib` — **169 passed** (was 76, +93 from P4+P5+P6+existing DelegationToken tests)
- `cargo test -p hkask-storage --lib` — 50 passed
- `cargo test -p hkask-cns --lib` — 88 passed
- `cargo test -p hkask-keystore --lib` — 31 passed
- `cargo test -p hkask-mcp-spec` — 20 passed
- `cargo test -p hkask-agents --lib` — 50 passed
- `cargo clippy -p hkask-types -- -D warnings` — clean
- `cargo clippy -p hkask-storage -p hkask-keystore -p hkask-mcp-spec -- -D warnings` — clean (hkask-mcp has pre-existing git_cas errors, not from test changes)

### Blocked ⚠️
- **hkask-mcp tests** — 37 tests written in `server.rs` and `security.rs`, but cannot run because `crates/hkask-mcp/src/git_cas/gix_adapter.rs` has pre-existing compile errors (unresolved `hex` crate, `async_trait`, lifetime mismatches). These errors existed before this session and are unrelated to the test additions.

## Key Decisions Made

1. **InMemorySpecStore** uses `RefCell<HashMap>` for interior mutability — matches the `&self` signature of `SpecStore` trait.

2. **CurationDecision has 3 variants** (Merge/Discard/Revise), not the 4 from DDMVSS §5.9. `Defer` was removed in v0.22.0.

3. **SpecCategory has 4 live variants** (Domain/Capability/Interface/Composition). Trust/Observability/Persistence/Lifecycle/Curation are DDMVSS categories applied to all specs.

4. **Keychain OS operations** — store/retrieve/delete depend on OS keyring and are not tested directly.

5. **GoalState state machine** — Completed and Abandoned are terminal states. Blocked can resume to Active. Pending can only transition to Active or Abandoned.

6. **cns.test** is the 16th canonical namespace, added for DDMVSS test observability.

7. **Pre-existing clippy warning** in `crates/hkask-types/src/loops/cybernetics.rs` — doc comment `//` changed to `//!` to separate list from paragraph.

8. **Pre-existing compile errors** in `crates/hkask-mcp/src/git_cas/gix_adapter.rs` — unresolved `hex` crate, `async_trait` missing, lifetime mismatches. These block `cargo test -p hkask-mcp` but are not related to the test additions.

9. **DelegationToken and verification tests already existed** — `mod delegation_token_tests` (50+ tests) and `mod tests` in verification.rs (15+ tests) were already present in `capability/mod.rs` and `capability/verification.rs`. The P6 gap for these seams was smaller than documented in the continuation v3. The new OCAPBoundary tests (20 tests) filled the actual P6 gap.

## Architecture Context

### Deep Seams (Test Priority Order)

1. **`CompletenessCheck`** — ✅ Tested
2. **`SpecCategory`** — ✅ Tested
3. **`CurationDecision`** — ✅ Tested
4. **`GoalSpec`** — ✅ Tested
5. **`Spec`** — ✅ Tested
6. **`SpecStore`** — ✅ Tested
7. **`SpecCurator`** — ✅ Tested
8. **`CNS` algedonic thresholds** — ✅ Already has good test coverage
9. **`hkask-keystore`** — ✅ 31 behavioral tests
10. **`hkask-mcp-spec`** — ✅ 20 type tests
11. **`GoalState`** — ✅ 23 tests
12. **`SpanNamespace`/`Phase`** — ✅ 14 tests
13. **`OCAPBoundary`/`OcapCapability`/`OcapTokenKind`** — ✅ 20 tests (NEW)
14. **`McpErrorKind`/`McpToolError`/`McpToolOutput`** — ✅ 21 tests (NEW, blocked by git_cas)
15. **`validate_identifier`/`validate_tool_url`** — ✅ 8 tests (NEW, blocked by git_cas)
16. **`security::validate_url`** — ✅ 16 tests (NEW, blocked by git_cas)
17. **`classify_http_error`** — ✅ 8 tests (NEW, blocked by git_cas)
18. **`CapabilityChecker`/`verify_delegation_token`** — ✅ Pre-existing comprehensive tests
19. **`DelegationToken` lifecycle** — ✅ Pre-existing comprehensive tests
20. **`CredentialRequirement`** — ✅ 2 tests (NEW, blocked by git_cas)

### Remaining Deep Seams (Not Yet Tested)

| Seam | Crate | Priority | Notes |
|------|-------|----------|-------|
| `GoalState` + `Goal` | hkask-storage/goals.rs | P2 | SQL persistence layer — 5 existing tests |
| `NuEventStore` | hkask-storage/nu_event_store.rs | P2 | Bitemporal queries, event persistence — 2 existing tests |
| `SpecServer` tool handlers | hkask-mcp-spec | P3 | 8 tool methods — need InMemorySpecStore + CapabilityChecker |
| `McpServer` tool dispatch | hkask-mcp | P3 | 37 tests written but blocked by git_cas compile errors |
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
| CapabilityChecker + verify_delegation_token | `crates/hkask-types/src/capability/verification.rs` |
| DelegationToken lifecycle tests | `crates/hkask-types/src/capability/mod.rs` |
| McpErrorKind + McpToolError + McpToolOutput | `crates/hkask-mcp/src/server.rs` |
| URL validation + SSRF protection | `crates/hkask-mcp/src/security.rs` |
| DDMVSS spec | `docs/architecture/DDMVSS.md` |
| Principles | `docs/architecture/PRINCIPLES.md` |
| Test inventory | `docs/status/test-inventory.md` |
| Test program spec | `docs/specifications/test-program.md` |
| Testing standards | `docs/specifications/TESTING_STANDARDS.md` |
| Open questions | `docs/OPEN_QUESTIONS.md` |
| CANONICAL_NAMESPACES (now includes cns.test) | `crates/hkask-types/src/event.rs` |

## Next Steps (Prioritized)

### P3: Fix git_cas compile errors (BLOCKING — must fix before hkask-mcp tests can run)

The `crates/hkask-mcp/src/git_cas/gix_adapter.rs` has pre-existing compile errors:
- Missing `hex` crate dependency
- Missing `async_trait` import
- Lifetime mismatches on trait methods

Fix these to unblock the 37 hkask-mcp runtime tests that are already written.

### P3: SpecServer tool handler tests (HIGH — DDMVSS governance surface)

The MCP spec server has 8 tool methods that depend on `Arc<dyn SpecStore + Send + Sync>`, `Arc<CapabilityChecker>`, and `WebID`. The `InMemorySpecStore` adapter from `spec_types.rs` can be reused.

Key seams to test:
- `spec_goal_capture` — creates spec with goals, validates category/anchor parsing
- `spec_curate_evaluate` — evaluates spec completeness
- `spec_curate_cultivate` — cultivates collection coherence
- `spec_graph_validate` — validates collection completeness
- Capability verification gate on each tool

**Challenge:** Tool methods use `rmcp` `Parameters<T>` wrapper. Extract business logic into testable functions.

### P3: Tool handler implementations for spec/test/* and spec/skill/*

Need:
- `spec/test/invariant` tool handler in `SpecServer`
- `spec/test/verify` tool handler
- `spec/skill/register` request/response types + handler
- `spec/skill/evaluate` request/response types + handler

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
- **Argon2id is intentionally slow** (~100ms per call) — minimize `derive_key` calls in keystore tests

## Verification Commands

```bash
cargo test -p hkask-types --lib          # 169 tests (including goal, event, curation, capability)
cargo test -p hkask-storage --lib         # 50 tests (spec_types)
cargo test -p hkask-cns --lib             # 88 tests (including gas budget)
cargo test -p hkask-keystore --lib        # 31 tests (encryption, master_key, error, keychain)
cargo test -p hkask-mcp-spec              # 20 tests (types)
cargo test -p hkask-agents --lib           # 50 tests (including spec_curator)
# hkask-mcp is blocked by git_cas compile errors — 37 tests written but not yet runnable
cargo clippy -p hkask-types -- -D warnings  # clean
```

## Recommended Skills

- **coding-guidelines** — Enforce P8/C8, surgical changes, simplicity first
- **TDD** — Red-green-refactor for new test writing
- **diagnose** — If tests fail, reproduce → minimize → hypothesize → instrument → fix
- **zoom-out** — When unfamiliar with a crate's architecture

## Open Questions Updated

- **TQ-1**: Mechanical vs LLM completeness — **Resolved**: `is_complete()` is mechanical for criteria-based goals.
- **TQ-8**: hkask-keystore zero tests — **Resolved**: 31 behavioral tests.
- **TQ-9**: hkask-mcp-spec zero tests — **Resolved**: 20 type tests.
- **TQ-10**: hkask-mcp zero test modules — **Resolved**: 37 tests written (blocked by git_cas compile errors).
- **TQ-11**: OCAPBoundary + OcapCapability zero tests — **Resolved**: 20 behavioral tests.
- **TQ-12**: DelegationToken lifecycle tests — **Resolved**: Pre-existing comprehensive tests in delegation_token_tests (~50 tests) and verification tests (~25 tests). Gap was smaller than documented.
- **TQ-2, TQ-3, TQ-4, TQ-5, TQ-6, TQ-7**: Still open. See OPEN_QUESTIONS.md for details.