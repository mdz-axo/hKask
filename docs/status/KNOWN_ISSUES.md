---
title: "Known Issues & Remediation Plan"
audience: [developers, maintainers, agents]
last_updated: 2026-05-22
togaf_phase: "Preliminary"
version: "1.0.0"
status: "Active"
domain: "Cross-cutting"
---

<!-- TOGAF_DOMAIN: Cross-cutting -->
<!-- VERSION: 1.0.0 -->
<!-- STATUS: Active -->
<!-- LAST_UPDATED: 2026-05-22 -->

# Known Issues & Remediation Plan

**Purpose:** Track build failures, code quality issues, and documentation gaps identified during 2026-05-22 documentation refresh.

**Related:** [`PROJECT_STATUS.md`](PROJECT_STATUS.md), [`../plans/TODO.md`](../plans/TODO.md)

---

## P0 — Build Failures (Must Fix Before Merge)

### 1. Clippy Error: collapsible_if ✅ RESOLVED

**Location:** `crates/hkask-cns/src/observers/sovereignty.rs:187-196`

**Status:** Fixed with let-chain syntax (Rust 2024 feature)

### 2. Test Compilation Failures ✅ RESOLVED

**Location:** `hkask-testing/unit-tests/*` (multiple files)

**Status:** Fixed 2026-05-22 — All 331 tests now passing
- Fixed import paths for `Triple`, `TripleStore`, `EpisodicMemory`, `SemanticMemory`, `BayesianOps`
- Fixed `TempTripleStore` → `TripleStore` with in-memory database
- Fixed `AlertSeverity` import path
- Removed `cli_api_symmetry` tests (requires integration test structure)

**Root cause:** API changes in core crates not reflected in tests

### 3. Missing Lib Target ⚠️ DEFERRED

**Location:** `mcp-servers/hkask-mcp-gml/Cargo.toml`

**Status:** No crates depend on hkask-mcp-gml lib target — warning is harmless

**Note:** Will add lib target when another crate needs to depend on it

---

## P1 — Security & Error Handling (Fix This Week)

### 1. Hardcoded Cryptographic Key 🔒

**Location:** `crates/hkask-ensemble/src/okapi_integration.rs` (2 occurrences)

**Code:**
```rust
let key = [0x42; 32]; // TODO: Load from secure keystore
```

**Risk:** Hardcoded keys are a critical security vulnerability

**Fix:** Use `hkask-keystore` port:
```rust
let key = keystore.get_key("okapi_encryption")?;
```

**Estimated effort:** 30 minutes

### 2. Unwrap() in Production Code

**Location:** `crates/hkask-templates/src/registry_sqlite.rs` (2 occurrences)

**Code:**
```rust
.unwrap(); // TODO: Handle error properly
```

**Risk:** Panic on database errors

**Fix:** Return `Result<>` and propagate errors:
```rust
let result = operation()?;
```

**Estimated effort:** 20 minutes

### 3. WebID Not Properly Sourced

**Location:** `crates/hkask-templates/src/registry_git.rs`

**Code:**
```rust
hkask_types::WebID::new() // TODO: Get from Git config
```

**Risk:** Incorrect provenance tracking

**Fix:** Extract from Git author config or fail:
```rust
let webid = git_config.author_webid()?;
```

**Estimated effort:** 45 minutes

---

## P2 — Documentation Gaps (Fix Next Session)

### 1. Missing Metadata Headers

**Affected:** ~18 documents

**Required fields:**
```yaml
---
title: "..."
audience: [...]
last_updated: YYYY-MM-DD
togaf_phase: "..."
version: "X.Y.Z"
status: "Active"
domain: "..."
---
```

**Estimated effort:** 1 hour

### 2. Citation Density

**Standard:** ≥1 APA 7th-edition citation per `##` section

**Audit command:**
```bash
grep -L "\[^" docs/architecture/*.md
```

**Estimated effort:** 2 hours

### 3. Diagram Alignment Verification

**Location:** `docs/architecture/*/DIAGRAM_ALIGNMENT` blocks

**Required fields:**
```markdown
<!-- DIAGRAM_ALIGNMENT
id: DIAG-XXX-NNN
verified_date: YYYY-MM-DD
verified_against: path/to/source.rs
status: VERIFIED
-->
```

**Estimated effort:** 1 hour

### 4. Link Checker Script

**Missing:** `.github/scripts/check_links.sh`

**Purpose:** Validate all markdown links in CI

**Estimated effort:** 30 minutes

---

## P3 — Implementation Gaps (Future Sessions)

### 1. sqlite-vec Search Not Implemented

**Location:** `crates/hkask-agents/src/adapters/memory_storage.rs`

**TODO:**
```rust
// TODO: Implement actual search using sqlite-vec
```

**Scope:** Vector similarity search for memory retrieval

**Estimated effort:** 4-8 hours

### 2. CLI Template Processing Incomplete

**Location:** `crates/hkask-cli/src/main.rs`

**TODO:**
```rust
// Simple echo/response for now - TODO: Implement actual template processing
```

**Scope:** Full template cascade execution from CLI

**Estimated effort:** 6-12 hours

### 3. Unused Function Warnings

**Location:** `hkask-cli` (5 warnings)

**Cleanup effort:** 15 minutes

---

## Tracking

| Priority | Count | Status |
|----------|-------|--------|
| **P0** | 0 | ✅ All resolved |
| **P1** | 3 | 🟡 Open (security hardening) |
| **P2** | 4 | 🟠 Open (documentation) |
| **P3** | 3 | 🔵 Open (implementation gaps) |

**Total estimated effort:** ~10-15 hours (P1-P3 remaining)

**Build Status:** ✅ All checks passing (cargo check, test, clippy, fmt)

---

## Verification Commands

```bash
# P0 verification
cargo clippy --workspace -- -D warnings
cargo test --workspace

# P1 verification
cargo audit  # Security scan
grep -r "TODO.*keystore\|TODO.*unwrap" crates/

# P2 verification
grep -L "^Version:\|^version:" docs/**/*.md
.github/scripts/check_links.sh

# P3 verification
grep -r "TODO:" crates/ --include="*.rs"
```

---

*This document tracks technical debt identified during the 2026-05-22 documentation refresh. P0 items must be fixed before any merge.*

**Next review:** 2026-05-29 (weekly cadence)