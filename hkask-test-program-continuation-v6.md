# hKask Test Program Evolution — Continuation Prompt v6

## Session Purpose

Evolve hKask's test program from ad-hoc tests to a DDMVSS-governed behavioral testing discipline. Write behavioral tracer-bullet tests at deep seams. Fix pre-existing compile errors blocking test execution.

## Progress Summary

### Completed ✅

1–14. (Same as v4 — see `hkask-test-program-continuation-v4.md` for P0–P5 details)

15. **P6: OCAPBoundary + OcapCapability behavioral tests** — 20 tests in `hkask-types/src/curation.rs`

16. **P6: Capability verification tests** — Pre-existing comprehensive tests in `hkask-types/src/capability/verification.rs`

17. **P6: DelegationToken lifecycle tests** — Pre-existing comprehensive tests in `hkask-types/src/capability/mod.rs`

18. **P3: hkask-mcp runtime tests** — 37 tests in `server.rs` and `security.rs`

19. **Fixed pre-existing compile errors** — 3 bugs in 2 files:
    - `snapshot_writer.rs`: `valid_from` changed from `String` to `DateTime<Utc>`, `perspective()` changed from method to field, `visibility()` changed from method to field
    - `executor.rs`: `render_inline_template` returns `String` not `Result`, `minijinja::Value` doesn't implement `From<serde_json::Value>` (use `from_serialize`)

20. **Fixed snapshot_writer test API drift** — All 5 existing tests now pass with current API

21. **P3: SpecServer tool handler tests** — 19 tests in `hkask-mcp-spec/src/main.rs`:
    - Capability gate (2): missing token rejection, invalid token rejection
    - `spec_goal_capture` (2): creates spec with valid token, criteria populate goal
    - `spec_goal_decompose` (3): rejects empty spec_id, rejects nonexistent spec, adds sub-goals
    - `spec_require_bind` (1): attaches OCAP boundary
    - `spec_curate_evaluate` (3): complete → Merge, empty → Discard, partial → Revise
    - `spec_curate_reconcile` (2): detects tensions, rejects nonexistent specs
    - `spec_curate_cultivate` (2): empty collection below threshold, reports categories
    - `spec_graph_query` (2): filters by category, returns all without filters
    - `spec_graph_validate` (2): violations below threshold, missing categories

22. **P3: OCAP server behavioral tests** — 16 tests in `hkask-mcp-ocap/src/main.rs`:
    - `parse_capability` (6): two-part parsing, three-part parsing, single-part rejection, unknown resource rejection, memory alias, known actions
    - `ocap_delegate` (3): creates valid signed token, rejects empty issuer, rejects invalid capability
    - `ocap_verify` (2): validates valid token, returns not_found for unknown token
    - `ocap_revoke` (2): marks token as revoked, not_found for unknown token
    - `ocap_enumerate` (2): returns tokens for subject, excludes revoked tokens
    - `ocap_list_tokens` (1): returns all tokens with revocation status

23. **P2: Goal repository behavioral tests** — 18 new tests in `hkask-storage/src/goals.rs`:
    - `create_goal_roundtrips_through_sqlite` — creates and retrieves goal with field verification
    - `get_goal_returns_none_for_nonexistent_id`
    - `update_goal_state_pending_to_active` — legal transition persists
    - `update_goal_state_sets_completed_at_on_terminal` — terminal transition sets `completed_at`
    - `update_goal_state_rejects_illegal_transition` — Pending → Completed fails
    - `update_goal_state_rejects_nonexistent_goal`
    - `list_goals_filters_by_webid_and_state` — filters by webid, state, empty for unknown
    - `add_criterion_roundtrips` — persists and retrieves criterion
    - `add_criterion_rejects_mismatched_goal_id` — referential integrity
    - `add_artifact_roundtrips` — persists and retrieves artifact
    - `add_artifact_rejects_mismatched_goal_id` — referential integrity
    - `create_subgoal_sets_parent_and_depth` — depth = parent + 1
    - `create_subgoal_rejects_nonexistent_parent`
    - `get_subgoals_returns_children`
    - `delete_goal_removes_goal`
    - `delete_goal_returns_not_found_for_nonexistent`
    - `goal_criterion_new_starts_unsatisfied` — ID starts with `gc_`
    - `goal_criterion_mark_satisfied_flips_state`
    - `goal_artifact_new_has_correct_prefix` — ID starts with `ga_`

24. **P2: NuEventStore behavioral tests** — 8 new tests in `hkask-storage/src/nu_event_store.rs`:
    - `cursor_roundtrip_persists_and_retrieves`
    - `cursor_load_returns_none_for_absent_key`
    - `cursor_overwrite_replaces_value`
    - `cursor_keys_are_isolated`
    - `query_algedonic_returns_only_algedonic_act_events` — filters by category and phase
    - `query_algedonic_returns_empty_for_no_events`
    - `nu_event_round_trips_through_sqlite` — field-level round-trip verification
    - `decay_config_default_half_lives` — verifies cybernetics=5min, curation=15min, inference=2min, episodic=10min
    - `nu_event_sink_persist_maps_infra_errors`

25. **Fixed pre-existing `InfrastructureError::Other` API drift** — 6 occurrences in 3 files:
    - `agent_registry.rs` (2): `InfrastructureError::Other` → `InfrastructureError::Database`
    - `triples.rs` (2): `InfrastructureError::Other` → `InfrastructureError::Database`
    - `consent_store.rs` (2): `InfrastructureError::Other` → `InfrastructureError::Database`
    - `standing_session.rs` (2): `InfrastructureError::Other` → `InfrastructureError::Database`
    - `goals.rs` (2): `InfrastructureError::Other` → `InfrastructureError::Database`
    - `nu_event_store.rs` (2): `InfrastructureError::Other` → `InfrastructureError::Database`

26. **Fixed pre-existing `bundle.rs` unclosed function** — Missing `}` closing `valid_manifest()` in `hkask-types/src/bundle.rs`

### All passing tests ✅

| Crate | Tests |
|-------|-------|
| hkask-types | 177 |
| hkask-storage | **78** (+28 from v5) |
| hkask-cns | 88 |
| hkask-keystore | 31 |
| hkask-mcp | 62 |
| hkask-mcp-spec | **39** (+19 from v5) |
| hkask-mcp-ocap | **16** (+16 from v5) |
| hkask-templates | 50 |
| hkask-agents | 50 |
| **Total** | **591** |

### Clippy: Clean across all modified crates ✅

### Key Decisions Made

1–11. (Same as v5)

12. **`InMemorySpecStore` for SpecServer tests** — Uses `Mutex<HashMap<SpecId, Spec>>` instead of `RefCell` because `SpecStore: Send + Sync` requires thread-safe interior mutability. This mirrors the production `SqliteSpecStore` pattern but without SQLite dependency.

13. **`Parameters<T>` direct construction for tool handler tests** — The `rmcp::handler::server::wrapper::Parameters` type is `Parameters(pub T)`, enabling direct construction like `Parameters(GoalCaptureRequest { ... })` without MCP protocol overhead.

14. **JSON string matching for test assertions** — Tool handlers return `String` (JSON), so assertions use `result.contains("field")` patterns. Avoid JSON key-value syntax like `"key": value` in Rust string literals (colon is not special in string context).

15. **`InfrastructureError::Other` → `InfrastructureError::Database`** — The `Other` variant was removed from the enum, but 6 call sites still referenced it. Fixed all occurrences to use `Database(String)` which is the appropriate variant for wrapping external errors.

16. **`valid_manifest()` function body closure** — The closing `}` for the function was missing between the struct literal and the first `#[test]` attribute in `bundle.rs`, causing all subsequent tests to be parsed as part of the function body. Added the missing brace.

### Remaining Deep Seams (Not Yet Tested)

| Seam | Crate | Priority | Notes |
|------|-------|----------|-------|
| `WebSearchPort` validation/freshness/ranking/rate_limiter | hkask-mcp-web | P3 | Internal types are testable without external API access |
| `DecayConfig` lambda dispatch | hkask-storage | P2 | `lambda_for_category` is private but critical for decay weighting correctness |
| `row_to_nu_event` round-trip edge cases | hkask-storage | P2 | Unknown `span_category` falls back to `cns.gas`; `Visibility` reconstruction |

### File Locations (Key)

| Artifact | Path |
|----------|------|
| SpecServer tool handler tests | `mcp-servers/hkask-mcp-spec/src/main.rs` (lines ~723–1280) |
| SpecServer request/response type tests | `mcp-servers/hkask-mcp-spec/src/types.rs` (lines ~200–555) |
| OCAP server behavioral tests | `mcp-servers/hkask-mcp-ocap/src/main.rs` (lines ~295–710) |
| Goal repository behavioral tests | `crates/hkask-storage/src/goals.rs` (lines ~592–1079) |
| NuEventStore behavioral tests | `crates/hkask-storage/src/nu_event_store.rs` (lines ~294–614) |
| InfrastructureError::Other fixes | `crates/hkask-storage/src/{agent_registry,triples,consent_store,standing_session,goals,nu_event_store}.rs` |
| bundle.rs unclosed function fix | `crates/hkask-types/src/bundle.rs` (line ~943) |

## Verification Commands

```bash
# Currently passing:
cargo test -p hkask-types --lib          # 177 tests
cargo test -p hkask-storage --lib         # 78 tests
cargo test -p hkask-cns --lib             # 88 tests
cargo test -p hkask-keystore --lib        # 31 tests
cargo test -p hkask-mcp --lib             # 62 tests
cargo test -p hkask-mcp-spec              # 39 tests
cargo test -p hkask-mcp-ocap              # 16 tests
cargo test -p hkask-templates --lib        # 50 tests
cargo test -p hkask-agents --lib          # 50 tests

# Lint:
cargo clippy -p hkask-types -p hkask-storage -p hkask-mcp-spec -p hkask-mcp-ocap -- -D warnings  # clean

# Total: 591 tests across 9 crates
```

## Next Steps (Prioritized)

### P3: WebSearchPort internal types tests (HIGH — DDMVSS governance surface)

The `hkask-mcp-web` crate has four internal modules with testable logic that doesn't require external API access:

1. **`validation.rs`** — `validate_search_request`, `validate_extract_request`, `validate_browse_request`, `sanitize_health_error`
2. **`freshness.rs`** — `Freshness` parsing, `freshness_brave()`, `freshness_serpapi()`, `normalize_freshness()`
3. **`ranking.rs`** — `rrf_score()`, `apply_rerank()`, `dedup_results()`, `normalize_date_bucket()`
4. **`rate_limiter.rs`** — `RateLimiter` window/bucket/count mechanics

Each of these is a pure function or simple struct with well-defined behavioral invariants. Writing tracer-bullet tests for these would cover the P3 seam without needing to mock external API calls.

Key invariants to test:
- `validate_search_request`: rejects empty query, rejects query > 400 chars, accepts valid queries
- `validate_extract_request`: rejects URL > 2048 chars, rejects json_schema > 32KB
- `sanitize_health_error`: strips API key patterns (sk-, pk-, etc.), maps 401/403 → "authentication failed", maps 429 → "rate limited"
- `Freshness` parsing: round-trips all variants, rejects invalid strings
- `rrf_score`: computes `(1 / (k + rank))` correctly, handles empty input
- `dedup_results`: deduplicates by URL, preserves best rank
- `RateLimiter`: allows requests within limit, blocks over-limit, resets on window expiry

### P2: lambda_for_category dispatch tests (MEDIUM)

The `NuEventStore::lambda_for_category` function is a private method that maps span namespace categories to decay constants. It's the core of per-domain decay weighting. The mapping table is:

| Category prefix | Lambda |
|----------------|--------|
| `variety`, `gas`, `killzone` | cybernetics_lambda |
| `curation`, `spec` | curation_lambda |
| `inference` | inference_lambda |
| `agent_pod`, `connector` | episodic_lambda |
| everything else | cybernetics_lambda (safe default) |

This is a private function, so testing it requires either making it `pub(crate)` or testing it indirectly through `replay_weighted`. The existing `replay_weighted` tests already exercise this indirectly, but explicit unit tests would verify the dispatch table more precisely.

**Recommendation:** Add a `#[cfg(test)]` module inside `nu_event_store.rs` that calls `lambda_for_category` directly (it's accessible within the same module). Create a `DecayConfig` with known lambda values and verify each category maps correctly.

### P2: Visibility round-trip and span_category fallback tests (MEDIUM)

The `row_to_nu_event` function has two edge cases that deserve explicit testing:
1. **Unknown `span_category`** — Falls back to `SpanNamespace::new("cns.gas")`. This should be tested by inserting a row with a non-canonical category and verifying the reconstructed event uses the fallback.
2. **`Visibility` reconstruction** — The `visibility` field is stored as a string and reconstructed through `Visibility`'s `FromStr` or `Deserialize` impl. Testing that `Visibility::Public` and `Visibility::Private` survive round-trips through SQLite.

Both of these require direct database manipulation (INSERT rows with non-canonical data), which is straightforward with an in-memory SQLite connection.

## Constraints

(Same as v4 — see `hkask-test-program-continuation-v4.md`)

- **No visual UI** — headless only (CLI/MCP/API)
- **P8/C8** — Every `#[test]` verifies a stated behavioral property of a public seam. Tests without invariants are structural and must be rewritten or removed.
- **TDD practice** — Vertical tracer-bullet discipline (RED→GREEN per behavior, never horizontal slices)
- **No `todo!()` / `unimplemented!()` / `#[deprecated]`** — violations get deleted
- **Test depth matches module depth** — shallow modules get shallow tests; deep modules get deep tests

## Recommended Skills

- **coding-guidelines** — Enforce P8/C8, surgical changes, simplicity first
- **TDD** — Red-green-refactor for new test writing
- **diagnose** — If tests fail, reproduce → minimize → hypothesize → instrument → fix
- **zoom-out** — When unfamiliar with a crate's architecture