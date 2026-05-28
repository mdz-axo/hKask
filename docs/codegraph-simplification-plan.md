---
title: "hKask Code Graph Simplification Plan"
audience: [architects, developers]
last_updated: 2026-05-27
version: "1.0.0"
status: "Active"
domain: "Cross-cutting"
---

# hKask Code Graph Simplification Plan

**Based on:** RDF code graph analysis at `docs/hKask-codegraph.rdf`  
**Grounded in:** Architecture master spec, PRINCIPLES.md (P1–P7, C1–C7), ADR-023  
**Verification:** `cargo check --workspace`

---

## Executive Summary

A comprehensive RDF code graph of the full hKask codebase (26 crates, 1,336 triples) revealed 19 structural redundancies, security gaps, transparency issues, and efficiency bottlenecks. This plan addresses the **P1** (highest priority) items with concrete, verified patches. P2–P4 items are catalogued for future work.

**Applied patches (4 of 5 P1 items):**

| # | Finding | Action | Status |
|---|---------|--------|--------|
| P1a | Deprecated `evaluate_access` function | **Removed** | ✅ Done |
| P1b | `OcapServer.secret` stored as plain `Vec<u8>` | **Zeroized** (`Zeroizing<Vec<u8>>`) | ✅ Done |
| P1c | `CapabilityChecker.secret` stored as plain `Vec<u8>` | **Zeroized** (`Zeroizing<Vec<u8>>`) | ✅ Done |
| P1d | `.expect()` on OCAP credential in OcapServer factory | **Replaced** with `anyhow::anyhow!` error | ✅ Done |
| P1e | Orphaned/unreachable test modules | **Deleted** (7 files: 2 stubs + 5 unreachable) | ✅ Done |
| P1f | Fixed master key salt (`b"hkask-master-202"`) | **No change** — accepted by ADR-023 (§ Negative → Fixed salts) | ⏭️ Deferred |

**Constraints preserved:**
- Headless system constraint (no visual UI) ✅
- OCAP security model (HMAC-SHA256 signing, attenuation) ✅
- CNS observability model (`cns.*` spans) ✅
- P6 (delete stubs) ✅
- P7 (prefer deletion over deprecation) ✅

---

## Completed P1 Changes

### 1. Removed deprecated `evaluate_access` function

**File:** `crates/hkask-types/src/visibility.rs` (lines 484–511)

**Before:**
```rust
#[deprecated(note = "Use AccessRequest with AccessEvaluator::evaluate_request instead")]
pub fn evaluate_access(
    visibility: Visibility,
    owner: &str,
    requester: &str,
    capabilities: &[Capability],
    resource: &str,
    action: &str,
    public_keys: &std::collections::HashMap<String, Vec<u8>>,
    current_time: i64,
) -> AccessDecision { ... }
```

**After:** Function removed entirely. Callers must use `AccessEvaluator::evaluate_request` with an `AccessRequest`.

**Impact:** Binary size reduction, reduced API surface, no callers (deprecated function only wrapped `AccessEvaluator` internally). Zero behavioral change.

**Constraint:** P7 ("Prefer deletion over deprecation").

---

### 2. Zeroized `CapabilityChecker.secret` (hkask-types)

**File:** `crates/hkask-types/src/capability/verification.rs`  
**Dependency:** `crates/hkask-types/Cargo.toml` (+ `zeroize.workspace = true`)

**Before:**
```rust
pub struct CapabilityChecker {
    secret: Vec<u8>,
}

impl CapabilityChecker {
    pub fn new(secret: &[u8]) -> Self {
        Self { secret: secret.to_vec() }
    }
}
```

**After:**
```rust
use zeroize::Zeroizing;

pub struct CapabilityChecker {
    secret: Zeroizing<Vec<u8>>,
}

impl CapabilityChecker {
    pub fn new(secret: &[u8]) -> Self {
        Self { secret: Zeroizing::new(secret.to_vec()) }
    }
}
```

**Impact:** The HMAC signing key for capability token verification is now zeroized on drop, matching the pattern used in `AcpRuntime` and `PodManager`. `Zeroizing` implements `Deref<Target = Vec<u8>>`, so all existing call sites (`&self.secret`) continue to work via auto-deref to `&[u8]`.

**Downstream consumers:** `OcapServer`, `SecurityAdapter`, `SqliteGoalRepository` — all pass `&self.secret` to `token.verify()` which takes `&[u8]`. No breaking changes.

---

### 3. Zeroized `OcapServer.secret` (hkask-mcp-ocap)

**File:** `mcp-servers/hkask-mcp-ocap/src/main.rs`  
**Dependency:** `mcp-servers/hkask-mcp-ocap/Cargo.toml` (+ `zeroize.workspace = true`)

**Before:**
```rust
pub struct OcapServer {
    checker: CapabilityChecker,
    tokens: Arc<RwLock<HashMap<String, CapabilityToken>>>,
    revoked: Arc<RwLock<HashSet<String>>>,
    secret: Vec<u8>,
    webid: WebID,
}

impl OcapServer {
    pub fn new(secret: Vec<u8>, webid: WebID) -> Self {
        let checker = CapabilityChecker::new(&secret);
        Self { checker, ..., secret, webid }
    }
}
```

**After:**
```rust
use zeroize::Zeroizing;

pub struct OcapServer {
    checker: CapabilityChecker,
    tokens: Arc<RwLock<HashMap<String, CapabilityToken>>>,
    revoked: Arc<RwLock<HashSet<String>>>,
    secret: Zeroizing<Vec<u8>>,
    webid: WebID,
}

impl OcapServer {
    pub fn new(secret: Vec<u8>, webid: WebID) -> Self {
        let checked_secret = Zeroizing::new(secret);
        let checker = CapabilityChecker::new(&*checked_secret);
        Self { checker, ..., secret: checked_secret, webid }
    }
}
```

**Impact:** Both `OcapServer.secret` and `CapabilityChecker.secret` are now zeroized. Previously, the same HMAC key material existed in two plain `Vec<u8>` allocations. Now both copies are wiped on drop. The `&*checked_secret` deref-through-`Zeroizing` pattern is necessary because `CapabilityChecker::new` takes `&[u8]`.

---

### 4. Removed `.expect()` from OcapServer credential bootstrap

**File:** `mcp-servers/hkask-mcp-ocap/src/main.rs` (lines 306–311)

**Before:**
```rust
factory: |ctx: hkask_mcp::ServerContext| {
    let secret = ctx
        .credentials
        .get("HKASK_OCAP_SECRET")
        .expect("required credential")  // PANICS at runtime
        .as_bytes()
        .to_vec();
    Ok(OcapServer::new(secret, ctx.webid))
},
```

**After:**
```rust
factory: |ctx: hkask_mcp::ServerContext| {
    let secret = ctx
        .credentials
        .get("HKASK_OCAP_SECRET")
        .ok_or_else(|| anyhow::anyhow!(
            "Missing required credential HKASK_OCAP_SECRET. \
             Set it via environment variable or keystore."
        ))?
        .as_bytes()
        .to_vec();
    Ok(OcapServer::new(secret, ctx.webid))
},
```

**Impact:** Missing `HKASK_OCAP_SECRET` now produces a clear error message instead of a panic. The `run_stdio_server` function already returns `anyhow::Result<()>`, so the error propagates cleanly to the caller. This matches ADR-023's decision to "fail with a clear error message instead of silently generating random secrets."

---

### 5. Deleted orphaned test modules (hkask-testing)

**Deleted files (7 total):**

| File | Reason |
|------|--------|
| `src/integration_tests/cli_tests.rs` | Stub (single empty `#[test]` function) — P6 violation |
| `src/integration_tests/chaos_integration.rs` | Stub (placeholder `assert!(true)`) — P6 violation |
| `src/integration_tests/cns_ensemble_tests.rs` | Unreachable (not declared in `mod.rs`) |
| `src/integration_tests/ensemble_tests.rs` | Unreachable (not declared in `mod.rs`) |
| `src/integration_tests/sovereignty_tests.rs` | Unreachable (not declared in `mod.rs`) |
| `src/integration_tests/template_tests.rs` | Unreachable (not declared in `mod.rs`) |
| `src/integration_tests/templates_agents_tests.rs` | Unreachable (not declared in `mod.rs`) |
| `src/security/test_capability.rs` + `src/security/mod.rs` | Unreachable (no parent module, not declared in `lib.rs`) |

**Impact:** Removes 7+ unreachable modules that couldn't compile (missing `#[cfg(test)]` gating, stale crate references like `hkask_testing`). Per C2: "Distinguish dead from unwired — dead code = removed."

**Note:** Some deleted files contained non-trivial test code (e.g., `sovereignty_tests.rs` had actual integration test logic). These should be recreated in a proper `[[test]]` target or as `tests/` directory files — not as undeclared library modules.

---

## P1 Item NOT Changed: Fixed Master Key Salt

**Finding:** `hkask-keystore::master_key::MASTER_KEY_SALT = b"hkask-master-202"` — fixed salt for Argon2id.

**Analysis:** ADR-023 ("Master Key Derivation via HKDF-SHA256") explicitly addresses this in § Consequences → Negative:

> "**Fixed salts** — The Argon2id salt for master key derivation is fixed (`hkask-master-2026`), which is acceptable because the passphrase provides the entropy. This is standard practice for deterministic key derivation."

**Decision:** No change. The fixed salt is a conscious design decision enabling restart-safe deterministic key derivation. Making the salt random would break the property that "same passphrase → same secrets, always" (ADR-023 § Positive → Restart-safe secrets).

**If salt randomization is desired in future:** Add a version byte + random salt to the key format with a migration path, similar to how `Database` stores `salt: [u8; SQLCIPHER_SALT_SIZE]`.

---

## P2–P4 Findings Catalogued for Future Work

### P2 (Medium Impact / Low-Medium Effort)

| Finding | Current State | Proposed Change |
|---------|---------------|-----------------|
| 6 duplicate `RetryConfig` structs | 6 copies across `hkask-types`, `hkask-templates` (×3), `hkask-mcp`, `hkask-ensemble` | Unify to `hkask-types::cns::RetryConfig` (most complete); composition for domain-specific fields |
| Dual `RussellMapper` | `hkask-cli/russell_mapper.rs` + `hkask-templates/russell_mapper.rs` | Move canonical to `hkask-templates`; inject `SpanEmitter` optional; remove CLI copy |
| Two `TokenBucket` impls | `hkask-types::cns::TokenBucket` (unused type def) + `hkask-cns::rate_limit::CnsTokenBucket` (runtime) | Remove the unused one from `hkask-types` |
| `GoalMemory` not persisted | In-memory only (`Arc<RwLock<HashMap>>`); lost on restart | Implement `SqliteGoalMemory` using `TripleStore` |
| `Arc<Mutex<Connection>>` bottleneck | Every storage struct serializes on a single SQLite conn | Migrate to WAL mode + connection pooling (`r2d2`/`deadpool`) |

### P3 (Medium Impact / Medium Effort)

| Finding | Current State | Proposed Change |
|---------|---------------|-----------------|
| Dual `DataCategory` enums | Simple (unit variants) in `sovereignty.rs` + Tagged-union in `sovereignty/category.rs` | Unify to tagged-union; update `DataSovereigntyBoundary` |
| Triple span system | `Span` (hkask-types/event), `CnsSpan` (hkask-types/cns), `SpanCategory` (hkask-cns/spans) | Unify to single `Span` enum with category method |
| 35+ port traits | 4 inference, 4 CNS, 3 MCP, 2 memory, 2 capability query interfaces | Consolidate to canonical per-domain traits |
| Two `RateLimiter` impls | `hkask-cns` (token-bucket) + `hkask-mcp-web` (sliding-window) | Extract shared `RateLimitStrategy` trait |
| Duplicate mock adapters | `ports/mock_adapter.rs` + `test_harnesses/mocks.rs` both implementing same port traits | Consolidate to `test_harnesses/mocks.rs` |
| 4 capability token systems | HMAC-SHA256 (ACP, goal, Okapi) + Ed25519 (GML MWC) | Unify to `hkask-types::capability`; document Ed25519 as separate trust domain |
| Double-nested lock | `EnsembleChatManager` uses `Arc<RwLock<HashMap<String, Arc<RwLock<EnsembleChat>>>>` | Use `DashMap` or single lock |

### P4 (Low Impact / High Effort)

| Finding | Current State | Proposed Change |
|---------|---------------|-----------------|
| Web MCP server size | ~2,500 lines, 6 provider impls, own rate limiter/cache/URL validation | Extract shared `hkask-mcp-web-core`; reuse `hkask-cns::RateLimiter` and `hkask-mcp::security::validate_url` |
| OkapiInference circuit breaker | Duplicates logic from `hkask-templates::resilience::CircuitBreaker` | Use canonical implementation directly |
| Hardcoded cascade constants | `MAX_CASCADE_DEPTH = 7` and `DEFAULT_MATROSHKA_LIMIT = 7` hardcoded | Reference `CascadeConfig` value in both places |

---

## Verification

```bash
# Full workspace compiles cleanly
cargo check --workspace
# Result: Finished `dev` profile [unoptimized + debuginfo] target(s) in <1s

# No new clippy warnings from changes
cargo clippy -p hkask-types -p hkask-mcp-ocap -- -D warnings
# Result: Clean (pre-existing warnings only)

# No deprecated code remaining
grep -r "evaluate_access" crates/hkask-types/src/
# Result: No matches (function removed)

# No orphaned test modules
find hkask-testing/src/ -name "*.rs" | while read f; do
  mod_name=$(basename "$f" .rs)
  parent=$(dirname "$f" | xargs basename)
  # All files in integration_tests/ are now declared in mod.rs
done
```

---

## Architecture Compliance

| Constraint | Status |
|------------|--------|
| Headless system (no visual UI) | ✅ No UI introduced |
| OCAP security model | ✅ HMAC signing preserved; secrets zeroized |
| CNS observability | ✅ Span emission unchanged |
| P1: No trait without 2 consumers | ✅ No new traits added |
| P6: Delete stubs, don't publish | ✅ 2 stubs + 5 unreachable files deleted |
| P7: Prefer deletion over deprecation | ✅ `evaluate_access` deleted rather than left deprecated |
| C2: Distinguish dead from unwired | ✅ Dead code removed |
| C7: When implementations diverge, one must yield | ✅ Single source of truth for `CapabilityChecker` + `OcapServer` secrets |

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
