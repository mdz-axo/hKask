# hkask-templates Compilation Issues

## Status: **PRE-EXISTING BLOCKER**

## Errors (9 total)

| Error | Type | Location |
|-------|------|----------|
| `E0405` | Cannot find trait `RegistryIndex` | `registry_git.rs:9` |
| `E0412` | Cannot find type `ProcessManifest` | `registry_git.rs:168` |
| `E0422` | Cannot find struct `ProcessManifest` | `registry_git.rs:168` |
| `E0422` | Cannot find struct `ManifestStep` | Multiple |
| `E0433` | Undeclared type `Action` | Multiple |

## Root Cause

The `hkask-templates/src/ports.rs` file defines these types, but they are not being exported properly from `lib.rs`, OR there is a circular dependency preventing compilation.

## Impact

| Crate | Status |
|-------|--------|
| `hkask-templates` | ❌ Cannot compile |
| `hkask-agents` | ❌ Blocked (depends on templates) |
| `hkask-mcp` | ❌ Blocked (depends on agents) |
| `hkask-cli` | ❌ Blocked (depends on templates) |
| `hkask-api` | ❌ Blocked (depends on templates) |

## Workaround

For this session, verified:
- ✅ `hkask-storage` compiles with new similarity search
- ✅ `hkask-types` compiles
- ✅ `hkask-cns` compiles
- ✅ `hkask-testing` migrated tests compile

## Resolution Path

1. **Immediate**: Fix `lib.rs` exports for `ports` module
2. **Short-term**: Verify no circular dependencies
3. **Long-term**: Add CI gate for crate compilation

## Files Requiring Attention

- `crates/hkask-templates/src/lib.rs` (line 59-63)
- `crates/hkask-templates/src/ports.rs` (lines 164-170)
- `crates/hkask-templates/src/registry_git.rs` (line 9)

---
*Issue documented: 2026-05-22*
*Part of hKask Technical Debt Resolution (Phase 3)*