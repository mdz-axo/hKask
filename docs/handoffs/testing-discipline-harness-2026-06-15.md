# Handoff — Testing Discipline & Harness Implementation

**Date:** 2026-06-15  
**Session scope:** Test harness maturation (Waves 1–6), testing discipline establishment (DbC+PBT), documentation consolidation, strategic migration planning  
**Completion:** ~80% of feasible work done. Infrastructure-heavy integration tests and contract-first migration remain.

---

## 1. Session Context

This session built hKask's testing program from three angles simultaneously: (1) a reusable test harness crate with fixtures and proptest strategies, (2) 45 new tests across 7 crates spanning property-based, integration, contract, fuzz, and non-Rust coverage, and (3) a formal Testing Discipline document anchoring the entire program on Design by Contract (Meyer, 1986) verified through Property-Based Testing (QuickCheck, 2000). The session also consolidated three redundant testing documents into one, registered new CNS spans for contract monitoring, updated the TDD and coding-guidelines skills to reflect contract-first ordering, and created a strategic plan for migrating the codebase to 100% contract coverage with replicant-assisted proposals.

---

## 2. What Was Done

### 2.1 Test Harness Crate (Wave 1 — Complete)

**New crate:** `crates/hkask-test-harness/`
- `Cargo.toml`, `src/lib.rs` (479 lines), `src/schema.rs` (217 lines), `src/strategies.rs` (276 lines)
- Added to workspace `Cargo.toml`
- **7 fixtures:** `TestDb` (in-memory SQLite with full schema, `Arc<Mutex<Connection>>`), `TestKeystore` (temp dir + master key), `TestWebId` (deterministic alice/bob/carol + random), `MockCnsRuntime` (controllable CNS with event injection, time advancement, signal tracking, variety counters), `temp_dir()`, `test_event()`, `test_triple()`
- **5 proptest strategies:** `any_nu_event()`, `any_triple()`, `any_capability_spec()`, `any_goal()`, `any_transcript_segment()` — free functions (not `Arbitrary` impls) due to Rust orphan rule (E0117)
- **13 self-tests pass.** Zero warnings. Prohibition sweep clean.
- **Key decision:** Schema embedded as `SCHEMA_SQL` const (20 tables, 17 indexes). `vec0` virtual table omitted (requires `sqlite-vec` extension loading). `MockCnsRuntime` is synchronous (no tokio) — sufficient for unit/integration tests.
- **Key decision:** `TestDb` stores `Arc<Mutex<Connection>>` (not bare `Connection`) to be compatible with `TripleStore::new()` and other Store types that require `Arc<Mutex<Connection>>`.

### 2.2 Property-Based Tests (Wave 2 — 4/6 crates)

| Crate | File | Tests Added | Invariants |
|-------|------|-------------|------------|
| `hkask-condenser` | `src/algorithms.rs` | 2 proptest | Idempotency: `compress(compress(x)) == compress(x)`; Size monotonicity: `len(compress(x)) ≤ len(x)` |
| `hkask-cns` | `src/energy.rs` | 3 proptest | Budget cap: `remaining + reserved ≤ cap`; Available ≥ 0; Replenish ≤ cap |
| `hkask-templates` | `src/contract_validator.rs` | 2 proptest | Validator never panics; Known terms always accepted |
| `hkask-memory` | `src/salience.rs` | 2 proptest | Salience scores in [0,1]; Empty tags → zero |

**Skipped:** Wallet (needs async mock for `WalletManager`), Keystore (Argon2id too slow for proptest).

Each crate got `hkask-test-harness` + `proptest` as dev-dependencies.

### 2.3 Integration Tracer Bullets (Wave 3 — 1/5 tasks)

**New file:** `crates/hkask-cns/tests/cns_feedback_loop_integration.rs` (100 lines)
- 5 tests: perturbation detection, homeostasis restoration, tool throttling, variety tracking, signal accumulation
- Uses `MockCnsRuntime` from harness crate

**Skipped:** CLI→Storage (needs full `AgentService` stack), MCP lifecycle (needs daemon startup), Inference routing (needs mock HTTP), Agent pod (needs agent infrastructure).

### 2.4 Contract Tests (Wave 4 — 2/4 tasks)

**New file:** `crates/hkask-types/tests/contract/types_contract.rs` (52 lines)
- 3 proptest: NuEvent, Goal, CapabilitySpec JSON round-trips
- Required `[[test]]` entry in `Cargo.toml` for `tests/contract/` subdirectory
- **Key decision:** `Triple` doesn't implement `Serialize`/`Deserialize` — excluded from contract tests

**New file:** `crates/hkask-storage/tests/contract/services_storage_contract.rs` (95 lines)
- 5 tests: TripleStore insert/query, query-by-attribute, count accuracy, delete, owner preservation
- Uses `TestDb::conn_arc()` for `TripleStore::new()`

### 2.5 Fuzz Tests (Wave 5 — Complete)

| Crate | File | Tests |
|-------|------|-------|
| `hkask-condenser` | `tests/condenser_fuzz.rs` | 2: arbitrary text + large input |
| `hkask-templates` | `tests/manifest_fuzz.rs` | 2: arbitrary bytes + strings → `serde_yaml::from_slice/from_str` |
| `hkask-mcp` | `tests/tool_input_fuzz.rs` | 2: arbitrary bytes + strings → `serde_json::from_slice/from_str` |
| `hkask-cli` | `tests/cli_fuzz.rs` | 1: random argv → `Cli::try_parse_from` |

All use `catch_unwind` + proptest. Each crate got `[[test]]` entries in `Cargo.toml`.

### 2.6 Non-Rust Coverage (Wave 6 — Complete)

| Artifact | File | Details |
|----------|------|---------|
| YAML validation | `crates/hkask-templates/tests/yaml_schema_validation.rs` | Validates all 81 registry manifests have required fields |
| Template rendering | `crates/hkask-templates/tests/template_rendering.rs` | Renders all 244 `.j2` templates with sample context |
| Shell lint gate | `docs/ci/check-shell-scripts.sh` | `shellcheck --severity=warning` on all `scripts/*.sh` |

### 2.7 Testing Discipline Document

**New file:** `docs/architecture/core/TESTING_DISCIPLINE.md` (565 lines, 10 sections)
- **External anchor:** Design by Contract (Meyer, 1986) — the single established discipline, not a hKask invention
- **Verification method:** Property-Based Testing (QuickCheck, 2000)
- **Internal bridge:** TDD skill — contract-first ordering (CONTRACT → RED → GREEN → REFACTOR)
- **Consolidated from:** `docs/specifications/specs/test-program.md` and `docs/specifications/standards/TESTING_STANDARDS.md` (both archived to `docs/archive/2026-06-15-testing-consolidation/`)
- **Key content absorbed:** Ontology (§3), Test Classification (§4), MDS Category→Test Strategy (§5), Rust Conventions (merged into §8), Verification Gates (§9)
- **New content added:** Contract syntax (§1.2), Contract→PBT pipeline (§2.3), Test Pyramid under DbC (§2.4), Partial Contract Coverage (§2.5), Probabilistic Contracts for LLM Agents (§7.6), Contract Conflict Resolution (§7.4)
- **Audited via:** Pragmatics (4-phase cascade), Essentialist (3-gate deletion test), Grill-Me (5-level Socratic interrogation). 6 gaps found and fixed.

### 2.8 CNS Span Registration

**Files modified:**
- canonical CNS span registry: `crates/hkask-types/src/cns.rs` (`CnsSpan`) — includes `cns.contract.violated`, `cns.contract.coverage`
- `crates/hkask-types/src/event.rs` `CANONICAL_NAMESPACES` — added same spans to code-level source of truth
- **Status:** Spans registered. Emission code NOT YET IMPLEMENTED. Documented as OUGHT with pointer to migration plan §5.4.

### 2.9 Skill Updates

| Skill | File | Changes |
|-------|------|---------|
| `tdd` | `.agents/skills/tdd/SKILL.md` | Philosophy: contract-first ordering, good tests = property-based. Tracer Bullet: CONTRACT→RED→GREEN. Checklists: contract checks added. |
| `coding-guidelines` | `.agents/skills/coding-guidelines/SKILL.md` | Goal-Driven Execution: contract verification added to success criteria. Anchoring discipline reference added. |

### 2.10 CI & Infrastructure

| File | Change |
|------|--------|
| `.github/workflows/ci.yml` | Contract coverage trend gate added to `security-invariants` job — warning-only, exit 0 always |
| `AGENTS.md` | TDD skill description updated; Key Docs now points to single Testing Discipline; Constraint Verification includes contract audit; archived test-program.md reference removed |

### 2.11 Strategic Plan

**New file:** `docs/plans/contract-first-migration-plan-v0.27.0.md` (342 lines)
- **Capability A:** Contract-first migration — 4 phases (Seed→Expand→Complete→Sustain), 17 crates prioritized by risk, bug-driven migration engine
- **Capability B:** Replicant-driven contract proposals — 4 phases (Discovery→Proposal→PR Flow→CNS), PR template with P2 consent gate, 5 new CNS spans specified

### 2.12 Build & Test Status

All crates compile cleanly. All 45 new tests pass. Zero warnings. Prohibition sweep clean.

---

## 3. What Remains

### HIGH PRIORITY

**H1 — Inference Routing Integration Test (Test Harness Wave 3, Task 3.3)**
- **What:** Create `crates/hkask-inference/tests/inference_routing_integration.rs` — test that the inference router correctly routes requests and falls back when a backend is unavailable
- **Blocker:** Needs a mock HTTP server. Options: `wiremock` (not in workspace deps), `httptest` (not in workspace deps), or a simple `tokio::test` with a real local HTTP server thread
- **Strategy:** Add `wiremock` or `httptest` as dev-dependency to `hkask-inference`. Create mock backends that return known responses. Test: normal routing → primary backend; primary fails → fallback backend. See plan §Task 3.3 for code sketch.
- **Files to create:** `crates/hkask-inference/tests/inference_routing_integration.rs`
- **Files to modify:** `crates/hkask-inference/Cargo.toml` (add dev-deps + `[[test]]` entry)
- **Verification:** `cargo test -p hkask-inference --test inference_routing_integration`

**H2 — Contract-First Migration Phase A1 (Seed)**
- **What:** Add `// REQ: pre/post` contracts to ≥50% of `pub fn` in `hkask-cns`, `hkask-wallet`, `hkask-keystore`
- **Current baseline:** 1,720 `pub fn` total, 0 contracted
- **Strategy:** Focus on highest-risk functions first. For each function: read existing tests/docs → write contract as doc-comment → verify with proptest (or note why PBT inappropriate) → add `// REQ:` tag to existing tests. Bug-driven: if contract reveals a bug, fix implementation.
- **Files to modify:** `crates/hkask-cns/src/*.rs`, `crates/hkask-wallet/src/*.rs`, `crates/hkask-keystore/src/*.rs`
- **Reference:** `docs/plans/contract-first-migration-plan-v0.27.0.md` §4.4
- **Verification:** Contract coverage audit bash one-liner (Testing Discipline §9.2)

**H3 — Replicant Contract Discovery Tool (Phase B1)**
- **What:** Create an MCP tool or CLI command that lists uncontracted `pub fn` in a given crate
- **Strategy:** Simplest path: a shell script or Rust binary that runs the contract audit grep and formats output. Can be wrapped as an MCP tool later.
- **Reference:** `docs/plans/contract-first-migration-plan-v0.27.0.md` §5.1

### MEDIUM PRIORITY

**M1 — MCP Lifecycle Integration Test (Test Harness Wave 3, Task 3.2)**
- **What:** Create `crates/hkask-mcp/tests/mcp_lifecycle_integration.rs` — test full MCP tool lifecycle (register → list → call → result)
- **Blocker:** Needs MCP daemon startup with mock CNS context. `MockCnsRuntime` exists but daemon startup requires `McpRuntime` which has complex dependencies.
- **Strategy:** Use `MockCnsRuntime` from harness. Start daemon in test mode. Register a simple echo tool. Verify client can discover and call it.
- **Files to create:** `crates/hkask-mcp/tests/mcp_lifecycle_integration.rs`
- **Reference:** `docs/plans/test-harness-maturation-plan-v0.27.0.md` §Task 3.2

**M2 — Agent Pod Integration Test (Test Harness Wave 3, Task 3.5)**
- **What:** Create `crates/hkask-agents/tests/agent_pod_integration.rs` — test two-agent pod interaction through improv modes
- **Blocker:** Needs agent creation, pod orchestration, improv session infrastructure
- **Strategy:** Use `TestWebId` for agent identities. Create minimal pod with two agents. Start plussing session. Verify message exchange.
- **Reference:** `docs/plans/test-harness-maturation-plan-v0.27.0.md` §Task 3.5

**M3 — Wallet Property Tests (Test Harness Wave 2, Task 2.3)**
- **What:** Add proptest on wallet balance conservation invariant
- **Blocker:** `WalletManager` requires `ChainPort` and `PrivacyPort` trait objects — need mock implementations
- **Strategy:** Create mock `ChainPort` and `PrivacyPort` in `hkask-wallet/src/manager.rs` `#[cfg(test)]` module. Test: for any transaction graph, `Σ(inputs) = Σ(outputs) + Σ(fees)`
- **Reference:** `docs/plans/test-harness-maturation-plan-v0.27.0.md` §Task 2.3

### LOW PRIORITY

**L1 — CNS Signal Emission Implementation**
- **What:** Wire `cns.contract.violated` and `cns.contract.coverage` spans to actually emit when contract tests fail
- **Reference:** `docs/plans/contract-first-migration-plan-v0.27.0.md` §5.4

**L2 — CLI→Storage Vertical Slice (Test Harness Wave 3, Task 3.1)**
- **What:** Full CLI→API→Service→Storage integration test
- **Blocker:** Requires full `AgentService` stack with real DB, keystore, config, tokio runtime

**L3 — MCP Tool Schema Contract (Test Harness Wave 4, Task 4.3)**
- **What:** Contract test verifying tool schema validation
- **Blocker:** Requires tool registration + JSON Schema validation infrastructure

**L4 — Agents↔Inference Contract (Test Harness Wave 4, Task 4.4)**
- **What:** Contract test verifying agent prompt → valid inference request
- **Blocker:** Requires agent prompt construction + inference router

---

## 4. Recommended Skills and Tools

### Skills to Load (in order)

1. **`condenser-continuation`** — Restores session state from this handoff. Use first.
2. **`coding-guidelines`** — Enforces think-before-coding, simplicity, surgical changes, goal-driven execution. Use before any code change.
3. **`tdd`** — Contract-first RED→GREEN→REFACTOR. Use for H1 (inference test) and H2 (contract migration). The skill has been updated to reflect contract-first ordering.
4. **`pragmatics`** — For architecture analysis if any task reveals design issues. Use if H2 (contract migration) surfaces functions whose behavior is unclear.

### Key Commands

```bash
# Verify harness crate
cargo test -p hkask-test-harness

# Contract coverage audit (run before and after H2 work)
pub_fns=$(grep -rn "pub fn\|pub async fn" crates/ mcp-servers/ --include="*.rs" | grep -v "cfg(test)" | grep -v "/tests/" | wc -l)
contracted=$(grep -rn "// REQ:.*pre:" crates/ mcp-servers/ --include="*.rs" | wc -l)
echo "Coverage: $contracted / $pub_fns"

# Prohibition sweep (run after any code change)
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"
grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"

# Full workspace check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

## 5. Key Decisions to Preserve

1. **Design by Contract is the external anchor, not a hKask invention.** The testing program is anchored on Meyer (1986), verified through QuickCheck (2000). Do not replace with a custom methodology. The combination DbC+PBT is established in the literature (Hillel Wayne 2017, `icontract-hypothesis` 2020, GUMBOX 2025).

2. **Contract-first ordering: CONTRACT → TEST → IMPLEMENTATION.** The TDD skill now says "write the contract before the test." The contract IS the specification of the test. Do not revert to test-first without contract.

3. **Contracts are `// REQ:` doc-comments on function signatures, not separate documents.** Format: `/// REQ: <spec_id> /// pre: ... /// post: ... /// inv: ...`. There is no separate "spec document" for function behavior — the contract serves as documentation, specification, and test oracle simultaneously (Meyer's rule 5).

4. **Proptest strategies are free functions, not `Arbitrary` impls.** Due to Rust orphan rule (E0117), we cannot implement `proptest::arbitrary::Arbitrary` for external types. Use `hkask_test_harness::strategies::any_nu_event()` etc. in `proptest!` macros.

5. **`TestDb` stores `Arc<Mutex<Connection>>`** — this was changed mid-session to be compatible with `TripleStore::new()` and other Store types. Do not revert to bare `Connection`.

6. **`cns.contract.violated` and `cns.contract.coverage` are registered but NOT YET IMPLEMENTED.** The spans exist in `CANONICAL_NAMESPACES` and the canonical CNS span registry (`crates/hkask-types/src/cns.rs`, `CnsSpan`). Emission code is pending (migration plan §5.4). Until implemented, contract violations are detected through CI test failures.

7. **Contract coverage CI gate is warning-only (exit 0).** Baseline is 0/1,720. A hard gate would block all PRs. The gate becomes hard when baseline exceeds 50%.

8. **Probabilistic contracts use `(p, δ, k)`-satisfaction from Agent Behavioral Contracts (2025).** For LLM agents, contracts cannot be binary pass/fail. The `prob:` field in the contract syntax specifies probability threshold, tolerance bound, and recovery window. See Testing Discipline §7.6.

9. **Contract conflict resolution for replicant proposals:** human identifies conflict → replicants reconcile via improv plussing → if fails, human selects one → rejected contract archived as curation decision. See Testing Discipline §7.4.

10. **The three testing documents are consolidated.** `docs/architecture/core/TESTING_DISCIPLINE.md` is the single authoritative reference. `test-program.md` and `TESTING_STANDARDS.md` are archived. Do not create new testing documents — extend the existing one.

---

## 6. Quick-Start for Next Agent

```
1. Load condenser-continuation skill to restore context
2. Load coding-guidelines skill
3. Run: cargo check --workspace (verify clean state)
4. Run: contract coverage audit (establish baseline)
5. Pick highest-priority task (H1, H2, or H3)
6. For H1: add wiremock/httptest, create inference routing test
7. For H2: start with hkask-cns governed_tool.rs, add contracts
8. For H3: create contract audit script/tool
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
