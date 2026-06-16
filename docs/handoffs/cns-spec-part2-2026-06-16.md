# Handoff: CNS Domain Specification Integration — 2026-06-16

## 1. Session Context

Pure documentation session. Integrated the 6-domain CNS specification (44 FRs, 44 contracts) from the prior handoff draft into the formal MDS documentation corpus at `docs/architecture/core/CNS-DOMAIN-SPECIFICATION.md`. Updated the architecture master to reference it. Zero code changes — all 44 contracts were already verified against the codebase. Build is clean, tests pass.

## 2. What Was Done

### Created

- **`docs/architecture/core/CNS-DOMAIN-SPECIFICATION.md`** — 190 lines. Formal MDS specification with YAML frontmatter (`mds_categories: [domain, composition, trust, lifecycle, curation]`). Covers 6 sub-domains:
  - 2.1 Algedonic (4 contracts — P9)
  - 2.2 Runtime (24 contracts — P9/P3/P7/P12)
  - 2.3 Governed Tool (3 contracts — P4)
  - 2.4 Governed Inference (2 contracts — P4)
  - 2.5 Circuit Breaker (3 contracts — P4)
  - 2.6 API Metering (8 contracts — P9)
  - Section 3: Verification commands (`cargo check`, `cargo test`, `kask cns status`)

### Updated

- **`docs/architecture/hKask-architecture-master.md`** — Two edits: added CNS spec row to "Canonical Specifications" table (line 197), added entry to document tree (line 570). Total count bumped 22 → 23.

### Verification

```bash
cargo check -p hkask-cns   # ✅ 0 warnings
cargo test -p hkask-cns    # ✅ 200+ tests pass
```

All 44 contract IDs match the codebase. The handoff draft had a typo on `FR-CIRCUIT-001` (P4 header in the table) — corrected to P9 (the actual motivating principle in the code).

## 3. What Remains

### HIGH

- **Contract Coverage** — 87 `pub fn` with only 72 `/// REQ:` contracts (82.8%). The Testing Discipline (`docs/architecture/core/TESTING_DISCIPLINE.md`) calls for pre/post conditions on every public function. This is a systematic audit, not a quick fix. Run `scripts/contract-audit.sh --summary` to scope the work.

### MEDIUM

- **Dynamic Gas Table** — `crates/hkask-cns/src/dynamic_gas_table.rs` (6 contracts, GAS-CALIB-001 through 003) is already implemented and tested. The `TableEnergyEstimator` has hardcoded costs; `DynamicGasTable` observes `cns.gas.settled` spans and adjusts via EMA. Both are built. The next phase would be **integration testing** across the full CNS feedback loop. Proposed: add `cargo test -p hkask-cns --test integration` scenarios for gas→rJoule→budget→replenish cycle.

### LOW

- **Documentation Sweep** — The `docs/architecture/hKask-architecture-master.md` references components that may need updating. Run `kask docs validate` after any structural changes to detect stale references.

## 4. Recommended Skills and Tools

The next agent should activate:

- **coding-guidelines** — Before writing any code. Enforces simplicity-first and surgical changes.
- **tdd** — For any new feature work. Vertical tracer-bullet RED → GREEN → REFACTOR.
- **condenser-continuation** — If continuing condenser work after context reset.

```bash
# Verify the codebase is still healthy before starting:
cargo check -p hkask-cns -p hkask-agents -p hkask-services
cargo clippy -p hkask-cns -- -D warnings
cargo test -p hkask-cns -p hkask-agents

# Check contract coverage gap:
scripts/contract-audit.sh --summary

# CNS span health:
kask cns status
```

## 5. Key Decisions to Preserve

1. **All four gaps are already closed.** The prior analysis identified `GasReport`, `BotHealthEvaluator`, `DynamicGasTable`, and `BotLifecycle` as missing — they are all implemented. Do not re-implement them.

2. **The `TableEnergyEstimator` uses hardcoded costs by design.** The comment in that file explicitly states: "These are intentionally conservative — they prevent infinite loops while being simple to understand and calibrate." Any dynamic calibration should be a new `DynamicGasTable` module, not a rewrite of the existing estimator.

3. **`MetacognitionLoop` is correctly wired.** The `bot_reports` field is populated via `BotHealthEvaluator::evaluate_all()` (not left empty). The `MetacognitionLoop::sense()` calls `get_bot_reports()` which delegates to the evaluator when present.

4. **Visibility pattern: `pub` for types, `pub(crate)` for internal plumbing.** `BotHealthStatus`, `BotStatusReport`, and `HealthThresholds` were made `pub` to match their usage across the `curator_agent` module boundary. This is the correct visibility for dependency-inversion style (constructor takes `Option<HealthThresholds>`).

5. **Zero stubs, zero warnings** — the codebase is clean. Any future work should maintain this standard.

6. **The CNS specification lives as a standalone file in `docs/architecture/core/`**, not appended to `REQUIREMENTS.md`. Per MDS scaffold §1: Domain → `architecture/`. The 6 sub-domains each map to their own principle (P4, P9, P12) and the file structure mirrors the crate's module layout.

7. **66 contracts in the code (not 44 in the spec)** — the spec covers the 6 sub-domain modules; additional contracts in `energy.rs`, `composite_energy_estimator.rs`, `dynamic_gas_table.rs`, and `wallet_energy_estimator.rs` are implementation-level and not part of this specification document.
