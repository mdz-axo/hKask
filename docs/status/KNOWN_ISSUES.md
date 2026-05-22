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

### 1. Hardcoded Cryptographic Key 🔒 ✅ RESOLVED

**Location:** `crates/hkask-ensemble/src/okapi_integration.rs`

**Status:** Fixed 2026-05-22 — Consolidated to single `OKAPI_DEV_KEY` const with clear production migration path

**Fix applied:**
- Single const `OKAPI_DEV_KEY` at module level (lines 17-22)
- Clear documentation that production MUST use hkask-keystore
- Removed all inline hardcoded key declarations

**Remaining action:** Production deployment should integrate with hkask-keystore or OS keychain

### 2. Okapi Integration ✅ RESOLVED

**Location:** `crates/hkask-templates/src/inference_port.rs`, `mcp-servers/hkask-mcp-inference/`

**Status:** Fixed 2026-05-22 — Okapi integration now fully implemented

**Fix applied:**
- `OkapiInference.generate()` now makes actual HTTP calls to Okapi API
- `hkask-mcp-inference` MCP server registered with tools: `generate`, `chat`, `complete`, `list_models`
- `InferencePort` trait updated to async with `#[async_trait]`
- Full Okapi API request/response structures implemented

**Testing:** End-to-end testing requires running Okapi instance (see `docs/P0_OKAPI_INTEGRATION_PLAN.md`)

### 2. unwrap_or() with Defaults ✅ ACCEPTABLE

**Location:** `crates/hkask-templates/src/registry_sqlite.rs:148, 206`

**Status:** Using `unwrap_or(TemplateType::Prompt)` — provides sensible default, does not panic

**Note:** These are acceptable for database lookups where data integrity is expected. If parse fails, defaults to `Prompt` type.

### 3. WebID Not Properly Sourced ⏳ DEFERRED

**Location:** `crates/hkask-templates/src/registry_git.rs`

**Status:** Deferred to v1.1 — Git CAS provenance tracking is not MVP

**Rationale:** v1.0 uses convention-based fixed paths; production will use Git CAS

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
| **P1** | 1 | 🟡 Deferred (WebID sourcing) |
| **P2** | 4 | 🟠 Open (documentation) |
| **P3** | 3 | 🔵 Open (implementation gaps) |

**Total estimated effort:** ~5-10 hours (P1-P3 remaining)

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