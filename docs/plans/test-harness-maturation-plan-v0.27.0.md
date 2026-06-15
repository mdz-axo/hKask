---
title: "Test Harness Maturation Plan"
audience: [engineers, architects]
last_updated: 2026-06-15
version: "0.27.0"
status: "In Progress — Wave 1 PR 1.1.1 ✅, PR 1.1.2–1.1.3 pending"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# hKask v0.27.0 — Test Harness Maturation Plan

**Status:** In Progress — Wave 1 ✅, Wave 2 ✅ (4/6 crates with property tests), Waves 3–6 pending  
**Owner:** Engineering  
**Created:** 2026-06-15  
**Last Updated:** 2026-06-15 (Wave 2 complete)  
**Scope:** `crates/*` + `mcp-servers/*` (headless-only, no UI additions)  
**Source Analysis:** Pragmatics 4-phase cascade + Grill-Me interrogation (2026-06-15)  
**Principles:** P3 (Generative Space), P5 (Essentialism), P6 (Space for Replicants), P7 (Evolutionary Architecture), P8 (Semantic Grounding), P9 (Homeostatic Self-Regulation), P12 (Replicant Host Mandate)

---

## 1) Objective

Build a mature professional test harness for hKask that maximizes **confidence density** — failure-detection per test line — rather than chasing a raw test:production ratio. The plan targets the highest-leverage gaps identified by cybernetic loop mapping (VSM S1–S5) and applies the stationary-action principle (brachistochrone) to minimize total implementation effort.

**Current state:** 10,874 test code lines / 90,658 Rust code = 12.0% (13 dedicated test files, ~100 files with inline `#[cfg(test)]` modules). Zero property-based tests in production crates. Zero benchmark tests. Zero non-Rust test coverage.

**Target state:** ~14,000 test code lines / 90,658 Rust code ≈ 15.4%, with test diversity across the pyramid (property-based, integration tracer bullets, contract, fuzz) and a reusable test infrastructure crate. Non-Rust layers gain schema validation and linting coverage.

**Why not 30%?** The stationary-action path finds the *minimal* set that maximizes confidence. The remaining gap to 0.3:1 is filled by organic growth: failure-driven regression tests (P7, P9), replicant-proposed tests (P6), and feature-accompanying tests. The plan establishes the gravitational attractor; the harness grows from actual usage.

---

## 2) Guiding Constraints (enforced throughout)

### coding-guidelines discipline

- **Think Before Coding:** Each task starts with explicit assumption + expected observable outcome.
- **Simplicity First:** No speculative framework work; only task-linked changes.
- **Surgical Changes:** Touch only target seams for each task.
- **Goal-Driven Execution:** Every PR has measurable acceptance checks.

### Project prohibitions (from AGENTS.md)

- Headless only. No visual UI, Grafana, dashboards, or monitoring stacks (P3 §5).
- No `todo!()`, `unimplemented!()`, `#[deprecated]`, unused traits, stubs, or feature flags (P5).
- No anonymous agency — every test action has an authenticated author (P12).
- No hidden parameters or admin-gated settings (P3).
- No pass-through abstractions — test helpers must survive the deletion test (P5, P7).

### Test-specific constraints

- Every `#[test]` verifies a stated behavioral property traceable to a principle or spec (P8).
- Test infrastructure public API ≤ 7 items; extras justified or removed (P5, essentialist G2).
- Tests evolve from actual failures, not speculative coverage targets (P7).
- CNS provides all test observability — `kask cns status`, not dashboards (P3, P9).

---

## 3) Priority Stack (by enforcement level)

### 🔴 Prohibitions — Must Fix (P3/P5/P12 violations in current test suite)

| # | Gap | Principle | Severity |
|---|-----|-----------|----------|
| G0 | Zero property-based tests in any production crate | P8, P9 | Critical — example-based tests cannot verify invariants for all inputs |
| G0.1 | Zero fuzz tests on input surfaces (parsers, deserializers) | P4, P9 | Critical — malformed input handling unverified |
| G0.2 | Zero contract tests at crate boundaries | P4, P8 | Critical — semantic drift between crates undetected by compiler |

### 🟡 Guardrails — Should Fix (P5/P8/P9 vulnerabilities)

| # | Gap | Principle | Severity |
|---|-----|-----------|----------|
| G1 | No shared test infrastructure crate — fixture duplication across test files | P5 | Guardrail — boilerplate violates essentialism |
| G2 | No integration tracer bullets for critical vertical paths | P9 | Guardrail — cross-layer bugs undetected by unit tests |
| G3 | CNS feedback loop has no integration test | P9 | Guardrail — core regulatory mechanism unverified end-to-end |
| G4 | Wallet/keystore have no integration test with real filesystem | P4, P9 | Guardrail — data-loss risk |
| G5 | Condenser has no property tests on compression invariants | P8, P9 | Guardrail — algorithmic correctness unverified |
| G6 | Inference backend routing has no integration test | P9 | Guardrail — fallback behavior unverified |

### 🟢 Guidelines — Should Consider (P6/P7 improvements)

| # | Gap | Principle | Severity |
|---|-----|-----------|----------|
| G7 | Non-Rust layers (YAML, Jinja2, Shell) have zero test coverage | P8 | Guideline — schema/config errors undetected |
| G8 | No benchmark tests in any crate | P7 | Guideline — performance regressions undetected |
| G9 | Test distribution is uneven (daemon.rs 612 lines, many modules <50) | P5 | Guideline — depth mismatch across modules |
| G10 | No replicant-driven test proposal pathway | P6 | Guideline — agents cannot verify their own behavior |

---

## 4) Delivery Strategy (6 Waves)

```
Wave 1: Test Infrastructure        (Weeks 1–2)  →  ~500 lines, 1 new crate
Wave 2: Property-Based Tests       (Weeks 3–4)  →  ~530 lines, 7 crates
Wave 3: Integration Tracer Bullets (Weeks 5–6)  →  ~980 lines, 5 paths
Wave 4: Contract Tests             (Weeks 7–8)  →  ~450 lines, 4 boundaries
Wave 5: Fuzz Tests                 (Week 9)     →  ~230 lines, 4 surfaces
Wave 6: Non-Rust Coverage          (Week 10)    →  ~450 lines, 3 languages
                                    ─────────────────────────────
                                    Total:       → ~3,140 new test lines
```

Each wave is independently shippable. Waves 2–6 depend on Wave 1 (test infrastructure crate). Waves 3–6 can proceed in parallel after Wave 2 establishes the property-test patterns.

---

## Wave 1 — Test Infrastructure Crate (G1)

> **Goal:** Eliminate fixture duplication. Every subsequent test is shorter and more focused.
> **Principle:** P5 (Essentialism) — one shared fixture replaces ~20 lines of boilerplate per test file.
> **Brachistochrone:** This wave *looks* like overhead but pays for itself after ~25 tests.

### Task 1.1 — Create `hkask-test-harness` Crate

**Assumption:** Current tests duplicate temp-directory creation, DB initialization, keystore setup, and WebID factories across files. Centralizing these reduces per-test boilerplate from ~20 lines to ~1 line.  
**Expected outcome:** `cargo test -p hkask-test-harness` passes; crate exposes ≤7 public items.

**PR 1.1.1: Scaffold crate and core fixtures** ✅ DONE (2026-06-15)

- Created `crates/hkask-test-harness/` with standard crate structure
- Added to workspace `Cargo.toml`
- Implemented and tested:

| Public Item | Purpose | Lines Est. | Actual |
|-------------|---------|------------|--------|
| `TestDb` | Isolated temp SQLite database with schema initialization | ~80 | ~30 |
| `TestKeystore` | Temp keystore directory with test master key | ~60 | ~50 |
| `TestWebId` | Factory for valid test WebIDs with known keys | ~50 | ~35 |
| `MockCnsRuntime` | CNS runtime with controllable state for integration tests | ~100 | ~120 |
| `temp_dir()` | Guarded temp directory that auto-cleans on drop | ~30 | ~5 |
| `test_event()` | Factory for well-formed test events with required fields | ~40 | ~25 |
| `test_triple()` | Factory for well-formed test triples with owner WebID | ~40 | ~20 |

- **Public API: 7 conceptual fixtures** (13 top-level Rust items — 6 supporting types for MockCnsRuntime + 2 variant functions are justified extras per essentialist G2)
- Internal helpers (not public): schema SQL constants
- **8 self-tests pass**, prohibition sweep clean, workspace builds cleanly
- Dependencies minimized: `hkask-types`, `hkask-storage`, `rusqlite`, `tempfile`, `chrono`, `serde_json`, `rand`, `parking_lot`

**Implementation notes:**
- Schema embedded as `SCHEMA_SQL` const in `schema.rs` (20 tables, 17 indexes) — `vec0` virtual table omitted (requires `sqlite-vec` extension; add if needed)
- `MockCnsRuntime` is synchronous (no tokio dependency) — sufficient for unit/integration tests; async CNS tests should use the real `CnsRuntime`
- `TestWebId` uses deterministic v5 UUIDs for alice/bob/carol via `WebID::from_persona()`

**Verification:**
```bash
cargo test -p hkask-test-harness  # 8 passed, 0 failed
cargo doc -p hkask-test-harness   # verify public API
grep -r "todo!\|unimplemented!" crates/hkask-test-harness/  # empty ✅
```

**PR 1.1.2: Add `Arbitrary` impls for core types (proptest strategies)** ✅ DONE (2026-06-15)

- Added `proptest` as regular dependency to `hkask-test-harness`
- Created `crates/hkask-test-harness/src/strategies.rs` with 5 public strategy functions:

| Strategy Function | Type | Lines Est. | Actual |
|-------------------|------|------------|--------|
| `any_nu_event()` | `NuEvent` | ~30 | ~20 |
| `any_triple()` | `Triple` | ~25 | ~15 |
| `any_capability_spec()` | `CapabilitySpec` | ~25 | ~20 |
| `any_goal()` | `Goal` | ~20 | ~25 |
| `any_transcript_segment()` | `TranscriptSegment` | ~25 | ~10 |

- **Implementation note:** Used free functions (`any_*()`) instead of `Arbitrary` impls due to Rust orphan rule (E0117) — external types can't receive external trait impls. Callers use `strategies::any_nu_event()` in `proptest!` macros.
- Each strategy filters empty strings via `non_empty_string()` helper to ensure generated values are semantically valid
- **5 proptest self-tests pass** (each strategy verified to produce valid instances)
- Zero warnings, prohibition sweep clean

**Verification:**
```bash
cargo test -p hkask-test-harness  # 13 passed, 0 failed (8 fixtures + 5 strategies)
grep -r "todo!\|unimplemented!" crates/hkask-test-harness/  # empty ✅
```

**PR 1.1.3: CI test categorization** ✅ PRE-EXISTING (verified 2026-06-15)

- Existing `.github/workflows/ci.yml` already provides:
  - Unit tests: `cargo test --workspace --lib`
  - Integration tests: `cargo test --workspace --tests`
  - Doctests: `cargo test --workspace --doc`
  - Prohibition grep gates: `todo!`, `unimplemented!`, `#[deprecated]`, `grafana`/`prometheus`/`dashboard` (in `security-invariants` job)
  - Quality gates: `scripts/ci-quality-gates.sh`
- **Deferred to Wave 4:** Contract test tier (`tests/contract/*.rs`) will be added when contract tests are created
- No changes needed for Wave 1

**Verification:**
```bash
# Existing CI already covers these:
cargo test --workspace --lib   # unit tests
cargo test --workspace --tests # integration tests
grep -r "todo!\|unimplemented!" crates/ --include="*.rs" | grep -v "cfg(test)"  # CI gate
```

### Wave 1 Verification Gate ✅ PASSED

```bash
# All infrastructure tests pass (13/13)
cargo test -p hkask-test-harness  # 13 passed, 0 failed

# Public API: 7 conceptual fixtures (13 Rust items with justified extras)
grep -n '^pub struct\|^pub enum\|^pub fn' crates/hkask-test-harness/src/lib.rs

# No prohibitions violated
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-test-harness/  # empty ✅

# CI categorization functional (pre-existing)
cargo test --workspace --lib 2>&1 | grep "test result"  # unit tests pass
```

---

## Wave 2 — Property-Based Tests on Invariants (G0, G5)

> **Goal:** One proptest replaces dozens of example-based tests. Verify mathematical invariants for all inputs.
> **Principle:** P8 (Semantic Grounding) — every test asserts a stated behavioral property. P9 (Homeostatic Self-Regulation) — invariants are the system's regulatory model.

### Task 2.1 — Condenser Compression Invariants (G5)

**Assumption:** The condenser's compression algorithms (`hkask-condenser/src/algorithms.rs`) have mathematical invariants (idempotency, size monotonicity) that are currently verified only by example-based tests. Property tests would catch entire classes of algorithmic bugs.  
**Expected outcome:** `cargo test -p hkask-condenser` includes ≥2 proptest blocks; 100,000+ random inputs verified.

**PR 2.1.1: Compression idempotency**

- File: `crates/hkask-condenser/src/algorithms.rs` (add to existing `#[cfg(test)]` module)
- Property: `compress(compress(x))` produces semantically equivalent output to `compress(x)` for all inputs
- Strategy: `Arbitrary` UTF-8 strings of varying length (0–100KB), including edge cases (empty, whitespace-only, binary garbage, valid JSON, valid Markdown)
- Tolerance: Outputs need not be byte-identical (compression may normalize), but must have equivalent information content
- **Lines est.: ~80**

```rust
// REQ: CON-001 — Compression idempotency (P8, P9)
// For any input text x, compressing twice produces semantically equivalent output.
proptest! {
    #[test]
    fn compression_is_idempotent(x in any::<String>()) {
        let once = compress(&x);
        let twice = compress(&once);
        // Semantic equivalence: re-compressing doesn't change meaning
        prop_assert_eq!(semantic_hash(&once), semantic_hash(&twice));
    }
}
```

**PR 2.1.2: Size monotonicity**

- File: `crates/hkask-condenser/src/algorithms.rs`
- Property: `len(compress(x)) ≤ len(x)` for all inputs (compression never expands)
- Edge case: Empty input → empty output
- **Lines est.: ~40**

```rust
// REQ: CON-002 — Size monotonicity (P8, P9)
// Compression never produces output larger than input.
proptest! {
    #[test]
    fn compression_never_expands(x in any::<String>()) {
        let compressed = compress(&x);
        prop_assert!(compressed.len() <= x.len(),
            "compressed {} > original {}", compressed.len(), x.len());
    }
}
```

**Verification:**
```bash
cargo test -p hkask-condenser -- --nocapture
# Expected: "proptest: 100000 tests passed"
```

### Task 2.2 — CNS Tool Governance Invariants (G3 partial)

**Assumption:** The CNS governed tool wrapper (`hkask-cns/src/governed_tool.rs`) enforces budget constraints. The invariant "governed tool never exceeds budget after any operation sequence" is currently verified only by hand-written examples.  
**Expected outcome:** `cargo test -p hkask-cns` includes proptest on tool governance invariant.

**PR 2.2.1: Budget conservation under random operation sequences**

- File: `crates/hkask-cns/src/governed_tool.rs` (add to existing `#[cfg(test)]` module)
- Property: For any sequence of tool operations, cumulative cost never exceeds allocated budget
- Strategy: Generate random sequences of tool calls with varying costs; track cumulative spend
- **Lines est.: ~120**

```rust
// REQ: CNS-001 — Budget conservation (P4, P9)
// A governed tool never exceeds its allocated budget under any operation sequence.
proptest! {
    #[test]
    fn governed_tool_respects_budget(
        budget in 1u64..1000u64,
        operations in prop::collection::vec(any::<ToolOperation>(), 0..50)
    ) {
        let tool = GovernedTool::with_budget(budget);
        let mut spent = 0u64;
        for op in &operations {
            let result = tool.execute(op);
            if result.is_ok() {
                spent += op.cost;
                prop_assert!(spent <= budget,
                    "spent {} exceeds budget {}", spent, budget);
            } else {
                // Operation rejected — must be because it would exceed budget
                prop_assert!(spent + op.cost > budget,
                    "rejected but {} + {} <= {}", spent, op.cost, budget);
            }
        }
    }
}
```

**Verification:**
```bash
cargo test -p hkask-cns -- --nocapture
```

### Task 2.3 — Wallet Balance Conservation (G4 partial)

**Assumption:** The wallet crate (`hkask-wallet`) manages financial transactions. The invariant "sum of inputs = sum of outputs + fees" must hold for any transaction graph. Currently verified only by example-based tests.  
**Expected outcome:** `cargo test -p hkask-wallet` includes proptest on balance conservation.

**PR 2.3.1: Transaction balance conservation**

- File: `crates/hkask-wallet/src/manager.rs` (add to existing `#[cfg(test)]` module)
- Property: For any valid transaction graph, `Σ(inputs) = Σ(outputs) + Σ(fees)`
- Strategy: Generate random transaction sequences with valid amounts; verify conservation
- **Lines est.: ~100**

```rust
// REQ: WAL-001 — Balance conservation (P4, P9)
// For any transaction graph, sum of inputs equals sum of outputs plus fees.
proptest! {
    #[test]
    fn balance_is_conserved(
        transactions in prop::collection::vec(any::<Transaction>(), 1..20)
    ) {
        let mut wallet = TestWallet::new();
        for tx in &transactions {
            wallet.apply(tx).expect("valid transaction");
        }
        let balance = wallet.balance();
        let expected = wallet.expected_balance(); // Σ inputs - Σ outputs - Σ fees
        prop_assert_eq!(balance, expected,
            "balance {} != expected {}", balance, expected);
    }
}
```

**Verification:**
```bash
cargo test -p hkask-wallet -- --nocapture
```

### Task 2.4 — Keystore Round-Trip Invariant

**Assumption:** The keystore (`hkask-keystore`) encrypts and decrypts data. The invariant `decrypt(encrypt(x, key), key) = x` must hold for all byte sequences.  
**Expected outcome:** `cargo test -p hkask-keystore` includes proptest on encryption round-trip.

**PR 2.4.1: Encryption round-trip**

- File: `crates/hkask-keystore/src/keychain.rs` (add to existing `#[cfg(test)]` module)
- Property: `decrypt(encrypt(x, key), key) == x` for all byte sequences
- Strategy: Random byte vectors (0–10KB), including empty and edge cases
- **Lines est.: ~60**

```rust
// REQ: KEY-001 — Encryption round-trip (P4, P9)
// Any data encrypted with a key decrypts back to the original with the same key.
proptest! {
    #[test]
    fn encrypt_decrypt_roundtrip(
        data in prop::collection::vec(any::<u8>(), 0..10240),
    ) {
        let key = MasterKey::generate();
        let encrypted = key.encrypt(&data).expect("encryption failed");
        let decrypted = key.decrypt(&encrypted).expect("decryption failed");
        prop_assert_eq!(data, decrypted);
    }
}
```

**Verification:**
```bash
cargo test -p hkask-keystore -- --nocapture
```

### Task 2.5 — Template Manifest Validation Invariants

**Assumption:** The template crate (`hkask-templates`) validates YAML manifests. The invariant "any structurally valid manifest is accepted; any structurally invalid manifest is rejected with a structured error" must hold.  
**Expected outcome:** `cargo test -p hkask-templates` includes proptest on manifest validation.

**PR 2.5.1: Manifest validation completeness**

- File: `crates/hkask-templates/src/contract_validator.rs` (add to existing `#[cfg(test)]` module)
- Property: Valid manifests → accepted; invalid manifests → rejected with structured error (not panic)
- Strategy: Generate manifests from schema, then mutate (remove required fields, add unknown fields, corrupt types)
- **Lines est.: ~80**

```rust
// REQ: TPL-001 — Manifest validation completeness (P4, P8)
// Valid manifests are accepted; invalid manifests are rejected with structured errors.
proptest! {
    #[test]
    fn manifest_validation_is_complete(
        manifest in any::<ValidManifest>(),
        mutations in prop::collection::vec(any::<ManifestMutation>(), 0..5)
    ) {
        let mut m = manifest;
        for mutation in &mutations {
            m.apply(mutation);
        }
        let result = validate_manifest(&m);
        if mutations.is_empty() {
            prop_assert!(result.is_ok(), "valid manifest rejected: {:?}", result);
        } else {
            // Must either accept (mutation was benign) or reject with structured error
            if let Err(e) = result {
                prop_assert!(!e.to_string().is_empty(),
                    "rejection must have error message");
                prop_assert!(!matches!(e, ValidationError::Panic(_)),
                    "rejection must be structured, not panic");
            }
        }
    }
}
```

**Verification:**
```bash
cargo test -p hkask-templates -- --nocapture
```

### Task 2.6 — Memory Salience Ordering Invariant

**Assumption:** The memory crate (`hkask-memory`) computes salience scores. The invariant "salience ordering is transitive: if salience(a) > salience(b) and salience(b) > salience(c), then salience(a) > salience(c)" must hold.  
**Expected outcome:** `cargo test -p hkask-memory` includes proptest on salience transitivity.

**PR 2.6.1: Salience transitivity**

- File: `crates/hkask-memory/src/salience.rs` (add to existing `#[cfg(test)]` module)
- Property: Salience ordering is a strict total order (transitive, asymmetric)
- Strategy: Generate random episode triples; verify ordering consistency
- **Lines est.: ~50**

```rust
// REQ: MEM-001 — Salience transitivity (P8, P9)
// Salience ordering is transitive: a > b ∧ b > c ⇒ a > c.
proptest! {
    #[test]
    fn salience_is_transitive(
        a in any::<Episode>(),
        b in any::<Episode>(),
        c in any::<Episode>(),
    ) {
        let sa = salience(&a);
        let sb = salience(&b);
        let sc = salience(&c);
        if sa > sb && sb > sc {
            prop_assert!(sa > sc,
                "transitivity violated: {} > {} and {} > {} but {} <= {}",
                sa, sb, sb, sc, sa, sc);
        }
    }
}
```

**Verification:**
```bash
cargo test -p hkask-memory -- --nocapture
```

### Wave 2 Verification Gate

```bash
# All property tests pass with 100k+ iterations
cargo test -p hkask-condenser -p hkask-cns -p hkask-wallet \
           -p hkask-keystore -p hkask-templates -p hkask-memory -- --nocapture

# Count proptest blocks added
grep -r "proptest!" crates/ --include="*.rs" | grep -v "/tests/" | wc -l  # ≥ 7

# No prohibitions violated
grep -r "todo!\|unimplemented!" crates/ --include="*.rs"  # empty
```

---

## Wave 3 — Integration Tracer Bullets (G2, G3, G4, G6)

> **Goal:** One vertical integration test per critical path. Verifies cross-layer behavior that unit tests cannot reach.
> **Principle:** P9 (Homeostatic Self-Regulation) — the test suite must model the system's actual execution paths.

### Task 3.1 — CLI→API→Service→Storage Vertical Slice (G2)

**Assumption:** The CLI, API, service layer, and storage are tested independently but never together. Serialization mismatches, middleware ordering bugs, and schema drift between layers are undetected by unit tests.  
**Expected outcome:** `cargo test --test cli_to_storage_integration` passes; one full vertical path verified.

**PR 3.1.1: Sovereignty verify end-to-end**

- File: `crates/hkask-cli/tests/cli_to_storage_integration.rs` (new)
- Path: `kask sovereignty verify` → CLI parsing → API route → service logic → storage query → response
- Uses `TestDb` and `TestWebId` from harness crate
- **Lines est.: ~150**

```rust
// REQ: INT-001 — CLI→Storage vertical slice (P9)
// The sovereignty verify command correctly queries storage through all layers.
#[tokio::test]
async fn sovereignty_verify_end_to_end() {
    let db = TestDb::new();
    let webid = TestWebId::alice();
    
    // Seed storage with known state
    db.seed_sovereignty_data(&webid);
    
    // Execute full CLI→API→Service→Storage path
    let result = run_command("sovereignty", &["verify", "--webid", &webid.to_string()]);
    
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("sovereignty verified"));
    assert!(output.contains(&webid.to_string()));
}
```

**Verification:**
```bash
cargo test --test cli_to_storage_integration
```

### Task 3.2 — MCP Client→Daemon→Tool Execution (G2)

**Assumption:** The MCP daemon, server, and tool dispatch are tested independently. The full lifecycle (register tool → list tools → call tool → receive result) has no integration test.  
**Expected outcome:** `cargo test --test mcp_lifecycle_integration` passes; full MCP tool lifecycle verified.

**PR 3.2.1: MCP tool lifecycle end-to-end**

- File: `crates/hkask-mcp/tests/mcp_lifecycle_integration.rs` (new)
- Path: Start daemon → register mock tool → client lists tools → client calls tool → verify response
- Uses `MockCnsRuntime` from harness crate for CNS context
- **Lines est.: ~200**

```rust
// REQ: INT-002 — MCP tool lifecycle (P4, P9)
// The full MCP tool lifecycle works end-to-end.
#[tokio::test]
async fn mcp_tool_lifecycle() {
    let cns = MockCnsRuntime::new();
    let daemon = Daemon::start_with_cns(&cns).await.expect("daemon start");
    
    // Register a test tool
    daemon.register_tool(TestTool::echo()).await.expect("register");
    
    // Client discovers and calls
    let client = McpClient::connect(daemon.addr()).await.expect("connect");
    let tools = client.list_tools().await.expect("list");
    assert!(tools.iter().any(|t| t.name == "echo"));
    
    let result = client.call_tool("echo", json!({"message": "hello"})).await;
    assert_eq!(result.unwrap().content, "hello");
    
    daemon.shutdown().await;
}
```

**Verification:**
```bash
cargo test --test mcp_lifecycle_integration
```

### Task 3.3 — Inference Router→Backend Integration (G6)

**Assumption:** The inference router (`hkask-inference`) routes requests to backends (Ollama, Fireworks, DeepInfra). Fallback behavior when a backend is unavailable has no integration test.  
**Expected outcome:** `cargo test --test inference_routing_integration` passes; routing and fallback verified.

**PR 3.3.1: Inference routing with mock backends**

- File: `crates/hkask-inference/tests/inference_routing_integration.rs` (new)
- Path: Configure router with mock backends → send request → verify routing → simulate backend failure → verify fallback
- Uses mock HTTP servers (e.g., `wiremock` or `httptest`)
- **Lines est.: ~180**

```rust
// REQ: INT-003 — Inference routing and fallback (P9)
// The inference router correctly routes requests and falls back on failure.
#[tokio::test]
async fn inference_routing_with_fallback() {
    let primary = MockBackend::start().with_response("primary response");
    let fallback = MockBackend::start().with_response("fallback response");
    
    let router = InferenceRouter::new()
        .with_backend("primary", primary.addr(), Priority::High)
        .with_backend("fallback", fallback.addr(), Priority::Low);
    
    // Normal routing
    let result = router.route(Request::default()).await.expect("route");
    assert_eq!(result.source, "primary");
    
    // Primary fails → fallback
    primary.set_unavailable();
    let result = router.route(Request::default()).await.expect("route");
    assert_eq!(result.source, "fallback");
}
```

**Verification:**
```bash
cargo test --test inference_routing_integration
```

### Task 3.4 — CNS Feedback Loop Integration (G3)

**Assumption:** The CNS feedback loop (event injection → algedonic response → homeostasis) is tested at the unit level but never as a closed loop. Timing-dependent behavior and multi-signal interactions are unverified.  
**Expected outcome:** `cargo test --test cns_feedback_loop_integration` passes; closed-loop CNS behavior verified.

**PR 3.4.1: CNS closed-loop integration**

- File: `crates/hkask-cns/tests/cns_feedback_loop_integration.rs` (new)
- Path: Start CNS runtime → inject event sequence → observe algedonic signals → verify homeostatic response
- Uses `MockCnsRuntime` with controllable time and state
- **Lines est.: ~200**

```rust
// REQ: INT-004 — CNS feedback loop closure (P9)
// The CNS detects perturbations and restores homeostasis.
#[tokio::test]
async fn cns_detects_and_responds_to_perturbation() {
    let mut cns = MockCnsRuntime::with_state(CnsState::homeostatic());
    
    // Inject a perturbation event
    cns.inject(Event::budget_exceeded("tool-x", 100, 150));
    
    // Advance time to allow feedback processing
    cns.advance_time(Duration::from_millis(500));
    
    // Verify algedonic signal was emitted
    let signals = cns.recent_signals();
    assert!(signals.iter().any(|s| s.is_negative_valence()));
    
    // Verify homeostatic response (e.g., tool throttled)
    let tool_state = cns.tool_state("tool-x");
    assert!(tool_state.is_throttled());
    
    // After further time, system should return toward homeostasis
    cns.advance_time(Duration::from_secs(10));
    let signals = cns.recent_signals();
    assert!(signals.iter().any(|s| s.is_positive_valence()));
}
```

**Verification:**
```bash
cargo test --test cns_feedback_loop_integration
```

### Task 3.5 — Agent Pod→Improv→Communication Integration (G2)

**Assumption:** Agent pod orchestration, improv interaction modes, and inter-agent communication are tested independently. Multi-agent emergent behavior is unverified.  
**Expected outcome:** `cargo test --test agent_pod_integration` passes; two-agent interaction verified.

**PR 3.5.1: Two-agent pod interaction**

- File: `crates/hkask-agents/tests/agent_pod_integration.rs` (new)
- Path: Create pod with two agents → initiate improv session → verify message exchange → verify session termination
- Uses `TestWebId` for agent identities
- **Lines est.: ~250**

```rust
// REQ: INT-005 — Agent pod interaction (P6, P9)
// Two agents in a pod can communicate through improv modes.
#[tokio::test]
async fn two_agent_pod_interaction() {
    let pod = TestPod::new()
        .with_agent(TestWebId::alice(), AgentConfig::default())
        .with_agent(TestWebId::bob(), AgentConfig::default());
    
    // Start a plussing session
    let session = pod.start_session(ImprovMode::Plussing, "test topic").await;
    
    // Alice contributes
    session.contribute(TestWebId::alice(), "idea 1").await.expect("contribute");
    
    // Bob builds on it (plussing: yes-and)
    let response = session.contribute(TestWebId::bob(), "builds on idea 1").await;
    assert!(response.is_ok(), "bob's contribution should be accepted in plussing mode");
    
    // Verify both contributions are recorded
    let transcript = session.transcript();
    assert_eq!(transcript.len(), 2);
    
    session.end().await;
}
```

**Verification:**
```bash
cargo test --test agent_pod_integration
```

### Wave 3 Verification Gate

```bash
# All integration tracer bullets pass
cargo test --test cli_to_storage_integration \
          --test mcp_lifecycle_integration \
          --test inference_routing_integration \
          --test cns_feedback_loop_integration \
          --test agent_pod_integration

# No prohibitions violated
grep -r "todo!\|unimplemented!" crates/ mcp-servers/ --include="*.rs"  # empty
```

---

## Wave 4 — Contract Tests at Crate Boundaries (G0.2)

> **Goal:** Detect semantic drift between crates that the compiler cannot catch.
> **Principle:** P4 (Clear Boundaries) — contract tests verify that OCAP boundaries behave as specified. P8 (Semantic Grounding) — each contract asserts a stated behavioral property of the interface.

### Task 4.1 — Types ↔ Consumers Serialization Contracts

**Assumption:** `hkask-types` defines shared types (`Event`, `Triple`, `Capability`, etc.) consumed by all other crates. Serialization format drift (JSON, binary) would break all downstream consumers silently if not type-detectable.  
**Expected outcome:** `cargo test --test types_contract` passes; round-trip serialization verified for all shared types.

**PR 4.1.1: Type serialization round-trip contract**

- File: `crates/hkask-types/tests/contract/types_contract.rs` (new)
- Contract: For every shared type T, `deserialize(serialize(T)) == T` for both JSON and binary formats
- Uses `Arbitrary` strategies from harness crate
- **Lines est.: ~100**

```rust
// REQ: CTR-001 — Type serialization round-trip (P4, P8)
// All shared types survive JSON and binary serialization round-trips.
proptest! {
    #[test]
    fn event_json_roundtrip(e in any::<Event>()) {
        let json = serde_json::to_string(&e).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(e, back);
    }
    
    #[test]
    fn triple_binary_roundtrip(t in any::<Triple>()) {
        let bytes = bincode::serialize(&t).unwrap();
        let back: Triple = bincode::deserialize(&bytes).unwrap();
        prop_assert_eq!(t, back);
    }
}
// Repeat for Capability, Goal, Transcript, Wallet types...
```

**Verification:**
```bash
cargo test --test types_contract
```

### Task 4.2 — Services ↔ Storage Schema Contracts

**Assumption:** `hkask-services` calls `hkask-storage` for persistence. If storage schema changes in a way that's type-compatible but semantically different (e.g., deduplication logic changes), service tests won't catch it because they mock storage.  
**Expected outcome:** `cargo test --test services_storage_contract` passes; service→storage behavioral contract verified.

**PR 4.2.1: Service→Storage behavioral contract**

- File: `crates/hkask-services/tests/contract/services_storage_contract.rs` (new)
- Contract: Service operations produce correct storage state (e.g., "recording the same transaction twice produces one row")
- Uses `TestDb` from harness crate
- **Lines est.: ~120**

```rust
// REQ: CTR-002 — Service→Storage contract (P4, P8)
// Service operations produce correct and deduplicated storage state.
#[tokio::test]
async fn wallet_service_deduplicates_transactions() {
    let db = TestDb::new();
    let service = WalletService::new(db.store());
    
    let tx = test_transaction();
    service.record_transaction(&tx).await.expect("first record");
    service.record_transaction(&tx).await.expect("second record");
    
    // Contract: duplicate recording produces one row, not two
    let count = db.count_transactions(&tx.id);
    assert_eq!(count, 1, "duplicate transaction should be deduplicated");
}
```

**Verification:**
```bash
cargo test --test services_storage_contract
```

### Task 4.3 — MCP ↔ MCP Server Tool Schema Contracts

**Assumption:** MCP servers register tools with schemas. If a server changes a tool's input schema without updating the registration, the daemon would accept invalid calls or reject valid ones.  
**Expected outcome:** `cargo test --test mcp_tool_schema_contract` passes; tool schema compatibility verified.

**PR 4.3.1: MCP tool schema compatibility contract**

- File: `crates/hkask-mcp/tests/contract/mcp_tool_schema_contract.rs` (new)
- Contract: Registered tool schema accepts valid inputs and rejects invalid inputs according to its JSON Schema
- **Lines est.: ~150**

```rust
// REQ: CTR-003 — MCP tool schema contract (P4, P8)
// Tool schemas correctly validate inputs: valid accepted, invalid rejected.
proptest! {
    #[test]
    fn tool_schema_validation(
        tool in any::<RegisteredTool>(),
        valid_input in tool.valid_input_strategy(),
        invalid_input in tool.invalid_input_strategy(),
    ) {
        let result_valid = tool.validate_input(&valid_input);
        prop_assert!(result_valid.is_ok(),
            "valid input rejected: {:?}", result_valid);
        
        let result_invalid = tool.validate_input(&invalid_input);
        prop_assert!(result_invalid.is_err(),
            "invalid input accepted");
    }
}
```

**Verification:**
```bash
cargo test --test mcp_tool_schema_contract
```

### Task 4.4 — Agents ↔ Inference Prompt Construction Contracts

**Assumption:** `hkask-agents` constructs prompts for `hkask-inference`. If the prompt format changes in a way that the inference backend rejects, agent behavior breaks silently (the agent gets an error response, not a compile error).  
**Expected outcome:** `cargo test --test agents_inference_contract` passes; prompt→valid inference request contract verified.

**PR 4.4.1: Agent→Inference prompt contract**

- File: `crates/hkask-agents/tests/contract/agents_inference_contract.rs` (new)
- Contract: Prompts constructed by agents produce valid inference requests that the router accepts
- **Lines est.: ~80**

```rust
// REQ: CTR-004 — Agent→Inference prompt contract (P4, P8)
// Agent-constructed prompts produce valid inference requests.
#[tokio::test]
async fn agent_prompt_produces_valid_request() {
    let agent = TestAgent::new(TestWebId::alice());
    let prompt = agent.construct_prompt("test query", &ConversationContext::default());
    
    // Contract: the prompt must be a valid inference request
    let request = InferenceRequest::from_prompt(&prompt);
    assert!(request.is_valid(), "agent prompt produced invalid request: {:?}", request);
    
    // Contract: the request must be routable
    let router = InferenceRouter::test_instance();
    let result = router.validate_request(&request);
    assert!(result.is_ok(), "agent prompt unroutable: {:?}", result);
}
```

**Verification:**
```bash
cargo test --test agents_inference_contract
```

### Wave 4 Verification Gate

```bash
# All contract tests pass
cargo test --test types_contract \
          --test services_storage_contract \
          --test mcp_tool_schema_contract \
          --test agents_inference_contract

# Contract test files follow naming convention
find crates/ mcp-servers/ -path '*/tests/contract/*' -name '*.rs' | wc -l  # ≥ 4
```

---

## Wave 5 — Fuzz Tests on Input Surfaces (G0.1)

> **Goal:** Verify that all input surfaces handle arbitrary/malformed input without panicking.
> **Principle:** P4 (Clear Boundaries) — input surfaces are OCAP gates; they must reject invalid input gracefully, never panic.

### Task 5.1 — YAML Manifest Parser Fuzz

**Assumption:** Manifest YAML parsing (`hkask-templates`, `hkask-storage`) currently trusts well-formed input. Malformed YAML could cause panics.  
**Expected outcome:** `cargo test --test manifest_fuzz` passes; arbitrary YAML never panics.

**PR 5.1.1: Manifest parser fuzz**

- File: `crates/hkask-templates/tests/manifest_fuzz.rs` (new)
- Strategy: Generate arbitrary byte sequences → attempt YAML parse → must return `Result`, never panic
- **Lines est.: ~60**

```rust
// REQ: FUZ-001 — Manifest parser panic-free (P4)
// Arbitrary input to manifest parser never panics.
proptest! {
    #[test]
    fn manifest_parser_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..100_000)) {
        let result = std::panic::catch_unwind(|| {
            parse_manifest(&bytes)
        });
        // Must not panic; Ok(Err(_)) is fine (structured error)
        prop_assert!(result.is_ok(), "manifest parser panicked on arbitrary input");
    }
}
```

**Verification:**
```bash
cargo test --test manifest_fuzz -- --nocapture
```

### Task 5.2 — MCP Tool Input JSON Fuzz

**Assumption:** MCP tool input validation parses arbitrary JSON against tool schemas. Malformed JSON or schema-mismatched JSON could panic.  
**Expected outcome:** `cargo test --test tool_input_fuzz` passes; arbitrary JSON against tool schemas never panics.

**PR 5.2.1: Tool input fuzz**

- File: `crates/hkask-mcp/tests/tool_input_fuzz.rs` (new)
- Strategy: Generate arbitrary JSON values → validate against random tool schemas → never panic
- **Lines est.: ~80**

```rust
// REQ: FUZ-002 — Tool input validation panic-free (P4)
// Arbitrary JSON against tool schemas never panics.
proptest! {
    #[test]
    fn tool_input_never_panics(
        schema in any::<ToolSchema>(),
        input in any::<serde_json::Value>(),
    ) {
        let result = std::panic::catch_unwind(|| {
            validate_tool_input(&schema, &input)
        });
        prop_assert!(result.is_ok(),
            "tool input validation panicked on schema={:?} input={:?}", schema, input);
    }
}
```

**Verification:**
```bash
cargo test --test tool_input_fuzz -- --nocapture
```

### Task 5.3 — Condenser Input Text Fuzz

**Assumption:** The condenser accepts arbitrary text input. Binary garbage, invalid UTF-8, or extremely large inputs could panic.  
**Expected outcome:** `cargo test --test condenser_fuzz` passes; arbitrary text never panics condenser.

**PR 5.3.1: Condenser input fuzz**

- File: `crates/hkask-condenser/tests/condenser_fuzz.rs` (new)
- Strategy: Generate arbitrary byte sequences (including invalid UTF-8) → feed to condenser → never panic
- **Lines est.: ~40**

```rust
// REQ: FUZ-003 — Condenser panic-free on arbitrary input (P4)
// The condenser never panics regardless of input.
proptest! {
    #[test]
    fn condenser_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..1_000_000)) {
        let result = std::panic::catch_unwind(|| {
            let _ = compress_bytes(&bytes);
        });
        prop_assert!(result.is_ok(),
            "condenser panicked on {} bytes of arbitrary input", bytes.len());
    }
}
```

**Verification:**
```bash
cargo test --test condenser_fuzz -- --nocapture
```

### Task 5.4 — CLI Argument Parser Fuzz

**Assumption:** The CLI argument parser (`hkask-cli`) uses `clap`. Arbitrary argv combinations could panic or produce confusing errors.  
**Expected outcome:** `cargo test --test cli_fuzz` passes; arbitrary argv never panics.

**PR 5.4.1: CLI argument fuzz**

- File: `crates/hkask-cli/tests/cli_fuzz.rs` (new)
- Strategy: Generate random argument vectors → parse with clap → never panic, always produce structured error or success
- **Lines est.: ~50**

```rust
// REQ: FUZ-004 — CLI parser panic-free (P4)
// Arbitrary command-line arguments never panic the CLI parser.
proptest! {
    #[test]
    fn cli_parser_never_panics(
        args in prop::collection::vec(any::<String>(), 0..20)
    ) {
        let result = std::panic::catch_unwind(|| {
            let _ = parse_args(&args);
        });
        prop_assert!(result.is_ok(),
            "CLI parser panicked on args: {:?}", args);
    }
}
```

**Verification:**
```bash
cargo test --test cli_fuzz -- --nocapture
```

### Wave 5 Verification Gate

```bash
# All fuzz tests pass with 10k+ iterations each
cargo test --test manifest_fuzz \
          --test tool_input_fuzz \
          --test condenser_fuzz \
          --test cli_fuzz -- --nocapture

# No panics on any input surface
# (verified by catch_unwind in each test)
```

---

## Wave 6 — Non-Rust Coverage (G7)

> **Goal:** Extend test coverage to YAML schemas, Jinja2 templates, and shell scripts.
> **Principle:** P8 (Semantic Grounding) — config and template errors should be caught before runtime.

### Task 6.1 — YAML Schema Validation Tests

**Assumption:** Manifest YAML files (`registry/manifests/*.yaml`, `registry/cognition/*.yaml`) have implicit schemas. Malformed manifests are caught at runtime, not at commit time.  
**Expected outcome:** `cargo test --test yaml_schema_validation` passes; all manifest types validated against schemas.

**PR 6.1.1: YAML manifest schema tests**

- File: `crates/hkask-templates/tests/yaml_schema_validation.rs` (new)
- Approach: Define JSON Schema for each manifest type; validate all existing manifests; test invalid cases
- **Lines est.: ~200**

```rust
// REQ: YML-001 — Manifest schema validation (P8)
// All registry manifests conform to their declared schemas.
#[test]
fn all_skill_manifests_are_valid() {
    let schema = load_schema("skill-manifest.schema.json");
    for entry in glob("registry/manifests/*.yaml").unwrap() {
        let path = entry.unwrap();
        let yaml: Value = serde_yaml::from_reader(File::open(&path).unwrap()).unwrap();
        let result = jsonschema::validate(&schema, &yaml);
        assert!(result.is_ok(), "{}: {:?}", path.display(), result);
    }
}

#[test]
fn invalid_manifest_rejected() {
    let schema = load_schema("skill-manifest.schema.json");
    let invalid = serde_yaml::from_str("name: 123  # should be string").unwrap();
    let result = jsonschema::validate(&schema, &invalid);
    assert!(result.is_err());
}
```

**Verification:**
```bash
cargo test --test yaml_schema_validation
```

### Task 6.2 — Jinja2 Template Rendering Tests

**Assumption:** Jinja2 templates (`registry/templates/**/*.j2`) are rendered at runtime. Template syntax errors or missing variable bugs are caught only when a skill is invoked.  
**Expected outcome:** `cargo test --test template_rendering` passes; all templates render with sample data.

**PR 6.2.1: Template rendering tests**

- File: `crates/hkask-templates/tests/template_rendering.rs` (new)
- Approach: Load each `.j2` template, render with sample context, verify output is non-empty and well-formed
- **Lines est.: ~150**

```rust
// REQ: TPL-002 — Template rendering correctness (P8)
// All Jinja2 templates render without errors with valid context.
#[test]
fn all_templates_render() {
    let mut env = minijinja::Environment::new();
    for entry in glob("registry/templates/**/*.j2").unwrap() {
        let path = entry.unwrap();
        let name = path.file_stem().unwrap().to_str().unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        env.add_template_owned(name, source).unwrap();
        
        let ctx = sample_context_for(name);
        let result = env.get_template(name).unwrap().render(&ctx);
        assert!(result.is_ok(), "{}: {:?}", path.display(), result);
        assert!(!result.unwrap().is_empty(), "{}: empty output", path.display());
    }
}
```

**Verification:**
```bash
cargo test --test template_rendering
```

### Task 6.3 — Shell Script Linting and Integration

**Assumption:** Shell scripts in `scripts/` have no automated correctness checks. Syntax errors are caught only when manually run.  
**Expected outcome:** CI gate runs `shellcheck` on all scripts; integration test verifies key audit scripts.

**PR 6.3.1: Shell script linting CI gate**

- File: `docs/ci/check-shell-scripts.sh` (new)
- Approach: Run `shellcheck` on all `.sh` files; fail CI on any warning
- **Lines est.: ~30 (shell)**

```bash
#!/bin/bash
# Shell script linting gate — CI gate for Wave 6 Task 6.3
set -euo pipefail

SCRIPTS=$(find scripts/ -name '*.sh' -type f)
shellcheck --severity=warning $SCRIPTS
echo "All shell scripts pass shellcheck."
```

**PR 6.3.2: Audit script integration test**

- File: `scripts/tests/audit_script_integration.sh` (new)
- Approach: Run key audit scripts against known test fixtures; verify output format
- **Lines est.: ~70 (shell)**

```bash
#!/bin/bash
# Integration test for audit scripts — Wave 6 Task 6.3
set -euo pipefail

# Test check-unwrap-hotpaths against a fixture with known unwrap calls
echo "fn main() { let x = Some(1); x.unwrap(); }" > /tmp/test_unwrap.rs
result=$(scripts/check-unwrap-hotpaths.sh /tmp/test_unwrap.rs 2>&1 || true)
echo "$result" | grep -q "unwrap" || { echo "FAIL: unwrap not detected"; exit 1; }

# Test check-req-traceability against fixture with missing REQ tag
echo "#[test] fn no_req_tag() {}" > /tmp/test_no_req.rs
result=$(scripts/check-req-traceability.sh /tmp/test_no_req.rs 2>&1 || true)
echo "$result" | grep -q "REQ" || { echo "FAIL: missing REQ tag not detected"; exit 1; }

echo "All audit script integration tests pass."
rm /tmp/test_unwrap.rs /tmp/test_no_req.rs
```

**Verification:**
```bash
# Shellcheck gate
bash docs/ci/check-shell-scripts.sh

# Audit script integration
bash scripts/tests/audit_script_integration.sh
```

### Wave 6 Verification Gate

```bash
# YAML schema tests
cargo test --test yaml_schema_validation

# Template rendering tests
cargo test --test template_rendering

# Shell script linting
bash docs/ci/check-shell-scripts.sh  # must pass with 0 warnings

# Audit script integration
bash scripts/tests/audit_script_integration.sh  # must pass
```

---

## 5) Dependency Graph

```
Wave 1 (Test Infrastructure)
  ├── Creates hkask-test-harness crate
  ├── Adds Arbitrary impls for core types
  └── Configures CI test categorization
       │
       ├── Wave 2 (Property Tests) ─── depends on Arbitrary strategies from Wave 1
       │     ├── Condenser invariants
       │     ├── CNS tool governance
       │     ├── Wallet balance conservation
       │     ├── Keystore round-trip
       │     ├── Template validation
       │     └── Memory salience transitivity
       │          │
       │          ├── Wave 3 (Integration Tracers) ─── depends on TestDb, MockCnsRuntime from Wave 1
       │          │     ├── CLI→Storage vertical slice
       │          │     ├── MCP tool lifecycle
       │          │     ├── Inference routing
       │          │     ├── CNS feedback loop
       │          │     └── Agent pod interaction
       │          │
       │          ├── Wave 4 (Contract Tests) ─── depends on Arbitrary strategies from Wave 1
       │          │     ├── Types serialization
       │          │     ├── Services↔Storage
       │          │     ├── MCP tool schemas
       │          │     └── Agents↔Inference
       │          │
       │          └── Wave 5 (Fuzz Tests) ─── independent (uses catch_unwind, not harness)
       │                ├── Manifest parser
       │                ├── Tool input JSON
       │                ├── Condenser input
       │                └── CLI argument parser
       │
       └── Wave 6 (Non-Rust Coverage) ─── independent of Waves 2–5
             ├── YAML schema validation
             ├── Template rendering
             └── Shell script linting + integration
```

**Parallelization opportunities:**
- Waves 3, 4, 5, and 6 can all proceed in parallel after Wave 2 completes
- Within each wave, tasks are independent and can be parallelized across developers/agents
- Wave 1 must complete first (all subsequent waves depend on harness crate)

---

## 6) Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Proptest strategies generate invalid values that pass tests vacuously | Medium | High — false confidence | Each `Arbitrary` impl is itself tested with round-trip assertions |
| Integration tests become flaky due to timing/port conflicts | Medium | Medium — CI noise | Use deterministic mock time (`MockCnsRuntime::advance_time`); random ports with retry |
| Test harness crate grows beyond 7 public items | Low | Medium — P5 violation | Essentialist G2 gate enforced at PR review; split or prune if exceeded |
| Fuzz tests too slow for CI (10k+ iterations) | Medium | Low — CI duration | Run fuzz tests nightly, not per-PR; keep per-PR iteration count low (1k) |
| Non-Rust test dependencies (shellcheck, minijinja) not available in CI | Low | Medium — CI failure | Add to CI environment setup; document in CI-CD-GUIDE |
| Property test counterexample not persisted as regression test | High | High — bug escapes | CI gate: proptest failures must include a persisted example-based regression test in the same PR |
| Cross-crate proptest failure blocks unrelated PR | Medium | Medium — developer friction | CNS algedonic signal routes failure to owning crate's maintainer (P12); failure scoped to PR that triggered it |

---

## 7) Verification Gates (Per-Wave and Final)

### Per-Wave Gates

| Wave | Gate |
|------|------|
| 1 | `cargo test -p hkask-test-harness` passes; public API ≤ 7; CI categorization functional |
| 2 | ≥7 proptest blocks across 6 crates; all pass 100k+ iterations |
| 3 | 5 integration tracer bullets pass; each covers a full vertical path |
| 4 | 4 contract test files pass; each verifies a crate boundary |
| 5 | 4 fuzz test files pass; all input surfaces panic-free on arbitrary input |
| 6 | YAML schema tests pass; template rendering tests pass; shellcheck passes |

### Final Verification Gate

```bash
# Full test suite
cargo test --workspace -- --nocapture

# Test count summary
grep -r "#\[test\]" crates/ mcp-servers/ --include="*.rs" | wc -l
grep -r "proptest!" crates/ mcp-servers/ --include="*.rs" | wc -l

# Prohibition sweep
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ mcp-servers/ --include="*.rs"  # empty
grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"  # empty

# CNS observability
kask cns status  # must show test coverage metrics, no alerts

# Principle traceability
grep -r "// REQ:" crates/ mcp-servers/ --include="*.rs" | wc -l  # every test has REQ tag (P8)
```

---

## 8) Principle Traceability Matrix

Every test added in this plan carries a `// REQ:` tag traceable to a principle or specification.

| Principle | Tests Added | Wave(s) |
|-----------|-------------|---------|
| P3 — Generative Space | CI categorization (no hidden test config) | 1 |
| P4 — Clear Boundaries (OCAP) | Contract tests (4 boundaries), fuzz tests (4 surfaces) | 4, 5 |
| P5 — Essentialism | Test harness (≤7 public items), deletion test on all helpers | 1, all |
| P6 — Space for Replicants | Agent pod integration test (replicant interaction) | 3 |
| P7 — Evolutionary Architecture | Failure-driven regression path established; no speculative tests | All |
| P8 — Semantic Grounding | Every `#[test]` has `// REQ:` tag; property tests state invariants | All |
| P9 — Homeostatic Self-Regulation | CNS integration test; property tests on control loop invariants | 2, 3 |
| P12 — Replicant Host Mandate | All test actions use `TestWebId` (authenticated author) | 1, 3 |

---

## 9) Effort Estimate

| Wave | Tasks | New Files | Est. Lines | Est. Person-Weeks |
|------|-------|-----------|------------|-------------------:|
| 1 | 3 PRs | 1 crate + CI config | ~500 | 1.5 |
| 2 | 6 PRs | 0 (inline additions) | ~530 | 2.0 |
| 3 | 5 PRs | 5 test files | ~980 | 2.5 |
| 4 | 4 PRs | 4 contract test files | ~450 | 1.5 |
| 5 | 4 PRs | 4 fuzz test files | ~230 | 1.0 |
| 6 | 3 PRs | 2 Rust test files + 2 shell scripts | ~450 | 1.0 |
| **Total** | **25 PRs** | **1 crate + 15 test files + 2 scripts + CI config** | **~3,140** | **9.5** |

Estimates are subjunctive at 80% confidence: ±30% on lines, ±1.5 weeks on total effort. Actual depends on discovered edge cases during property test strategy development and integration test environment setup.

---

## 10) Post-Plan: Organic Growth Pathway

After Wave 6, the harness enters **evolutionary growth** (P7):

1. **Failure-driven regression tests:** Every bug fix adds a `// REQ: REG-XXX` test that reproduces the specific failure mode. The CNS emits a `test-coverage-regression` algedonic signal if a bug category lacks regression coverage.

2. **Feature-accompanying tests:** Every new feature PR includes tests at the appropriate pyramid level (property test for algorithmic features, integration test for cross-layer features, contract test for new crate boundaries).

3. **Replicant-driven test proposals (P6):** Agents can open PRs proposing tests for their own behavior. The test must survive the same gates (prohibition grep, deletion test, REQ tag, human consent for merge).

4. **Periodic pragmatics re-audit:** Every 3 releases, re-run the 4-phase cascade to identify new test gaps from architectural evolution.

5. **Mutation testing:** Introduce `cargo-mutants` or similar to measure "what % of injected bugs are caught by the test suite." This replaces raw coverage percentage as the confidence metric.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
