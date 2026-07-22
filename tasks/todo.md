# Consolidation TODO

## F1 Baseline (DONE)
- [x] Build workspace — ✅
- [x] Fix clippy lifetime error in hkask-mcp-codegraph — ✅
- [x] Full clippy clean — ✅
- [x] Map dependency graph (regular deps only)
- [x] Identify single-consumer crates
- [x] Zed reference analysis
- [x] Falsifiability admission gate
- [x] Create plan.md

## Phase 1: Safe single-consumer merges

### T1: Merge hkask-codegraph → hkask-mcp-codegraph ✅
- [x] Read both crate structures
- [x] Move source files (src/ → src/codegraph/)
- [x] Update Cargo.toml (merge deps, remove old dep)
- [x] Fix imports (crate:: → crate::codegraph::, hkask_codegraph:: → crate::codegraph::)
- [x] Remove from workspace
- [x] Build + clippy — ✅
- [x] Record delta: 69 → 68 workspace members, S4.1 preserved

### T2: Merge hkask-bridge-eso + fibo + golem → hkask-mcp-docproc ✅
- [x] Read bridge crate structures (single-file, zero-dep constant modules)
- [x] Move source files (→ src/bridge/{eso,fibo,golem}.rs)
- [x] Update Cargo.toml (remove 3 bridge deps)
- [x] Fix imports (hkask_bridge_eso → crate::bridge::eso)
- [x] Remove from workspace
- [x] Build + clippy — ✅
- [x] Record delta: 68 → 65 workspace members, S4.1 preserved

### T3: Merge hkask-storage-guard → hkask-services-context ✅
- [x] Read structure (single file, no crate:: refs)
- [x] Move source (→ src/storage_guard.rs)
- [x] Update Cargo.toml (add parking_lot, remove old dep)
- [x] Fix imports (hkask_storage_guard:: → crate::storage_guard::)
- [x] Remove from workspace + delete
- [x] Build + clippy — ✅
- [x] Record delta: 65 → 64

### T4: Merge hkask-services-verification → hkask-cli ✅
- [x] Read structure (single file, no crate:: refs)
- [x] Move source (→ src/verification.rs)
- [x] Update Cargo.toml (remove old dep, all deps already present)
- [x] Fix imports (hkask_services_verification:: → crate::verification::)
- [x] Remove from workspace + delete
- [x] Build + clippy — ✅
- [x] Record delta: 64 → 63

### T5: Merge hkask-services-research → hkask-mcp-research ✅
- [x] Read structure (21 files, providers/ and types/ subdirs)
- [x] Move source (→ src/research/)
- [x] Update Cargo.toml (merge 9 deps, remove old dep)
- [x] Fix imports (crate:: → crate::research::, hkask_services_research:: → crate::research::)
- [x] Remove from workspace + delete
- [x] Build + clippy — ✅
- [x] Record delta: 64 → 62 (also fixed stale T1 Cargo.toml dep)

**Phase 1 total: 69 → 62 workspace members (-7 nodes)**

## Phase 2: Larger single-consumer merges

### T6: Merge hkask-tui → hkask-repl ✅
- [x] Read structure (16 files, feature-gated optional dep)
- [x] Move source (→ src/tui/)
- [x] Update Cargo.toml (add ratatui+crossterm as optional, update feature)
- [x] Fix imports (crate:: → crate::tui::, hkask_tui:: → crate::tui::)
- [x] Remove from workspace + delete
- [x] Build + clippy (default + tui feature) — ✅
- [x] Also fixed orphaned #[derive] in mcp-codegraph (HEAD commit cleanup)
- [x] Record delta: 62 → 61

### T7: Merge hkask-adapter → hkask-mcp-training ✅
- [x] Read structure (10 files, adapter_router/ subdir)
- [x] Move source (→ src/adapter/)
- [x] Update Cargo.toml (add hkask-storage-core, remove hkask-adapter)
- [x] Fix imports (crate:: → crate::adapter::, hkask_adapter:: → crate::adapter::)
- [x] Move test file (live_adapter.rs)
- [x] Remove from workspace + delete
- [x] Build + clippy — ✅
- [x] Record delta: 61 → 60

**Phase 1+2 total: 69 → 60 workspace members (-9 nodes)**

## Phase 3: Structural evaluation
- [ ] T8: Evaluate 2-consumer crates
- [ ] T9: Evaluate hkask-api → hkask-cli
- [ ] T10: Evaluate hkask-mcp-cloud-gateway

## Phase 4: Convergence
- [ ] T11: Final metrics, S4 proof, PDCA log