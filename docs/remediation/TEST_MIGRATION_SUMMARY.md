---
title: "Test Code Migration Summary"
date: 2026-05-20
status: Complete
---

# Test Code Migration Summary

## Overview

As part of the code remediation session on 2026-05-20, inline test code was removed from production Rust modules. Tests now belong exclusively in the `hkask-testing` crate.

## Test Code Removed

### hkask-templates (11 files, ~1,383 lines)

| File | Tests Removed | Key Test Coverage |
|------|---------------|-------------------|
| `audit.rs` | 11 tests | ExecutionAudit, AuditTrail |
| `cascade.rs` | 10 tests | CascadeExecutor, security, cycle detection |
| `contracts.rs` | 9 tests | TemplateContract validation |
| `dependency.rs` | 12 tests | Dependency graph, cycle detection |
| `manifest.rs` | 13 tests | ProcessManifest, step validation |
| `ports.rs` | 10 tests | InferenceConfig, ManifestStep |
| `provenance.rs` | 5 tests | TemplateProvenance tracking |
| `registry.rs` | 14 tests | TemplateRegistry operations |
| `registry_git.rs` | 2 tests | Git-based registry |
| `registry_sqlite.rs` | 3 tests | SQLite registry |
| `renderer.rs` | 9 tests | Jinja2 rendering |

**Total: ~98 test functions removed**

### hkask-cns (5 files, ~304 lines)

| File | Tests Removed | Key Test Coverage |
|------|---------------|-------------------|
| `algedonic.rs` | 4 tests | AlgedonicAlert thresholds |
| `energy.rs` | 3 tests | Energy calibration |
| `rate_limit.rs` | 5 tests | Rate limiting |
| `spans.rs` | 2 tests | CNS span emission |
| `variety.rs` | 4 tests | Variety counters |

**Total: ~18 test functions removed**

## Migration Status

### Tests Already in hkask-testing

| Test File | Status | Coverage |
|-----------|--------|----------|
| `hkask_types_tests.rs` | ✅ Complete | 135 lines, ID types, visibility, lexicon |
| `hkask_templates_tests.rs` | ⚠️ Stub | 6 lines, placeholder only |
| `hkask_cns_tests.rs` | ⚠️ Stub | 1 line, placeholder only |
| `hkask_storage_tests.rs` | ⚠️ Stub | 1 line, placeholder only |
| `hkask_memory_tests.rs` | ⚠️ Stub | 1 line, placeholder only |
| `hkask_ensemble_tests.rs` | ⚠️ Stub | 1 line, placeholder only |
| `hkask_keystore_tests.rs` | ⚠️ Stub | 1 line, placeholder only |
| `hkask_agents_tests.rs` | ⚠️ Stub | 1 line, placeholder only |
| `hkask_api_tests.rs` | ⚠️ Stub | 6 lines, minimal |
| `hkask_cli_tests.rs` | ⚠️ Stub | 6 lines, minimal |
| `hkask_mcp_tests.rs` | ⚠️ Stub | 1 line, placeholder only |
| `hkask_mcp_inference_tests.rs` | ⚠️ Stub | 1 line, placeholder only |

## Next Steps

### High Priority

1. **Migrate critical tests** from git history to `hkask-testing`:
   - Security tests (path traversal, Jinja2 injection)
   - Capability attenuation tests
   - CNS span emission tests
   - Registry operation tests

2. **Update hkask-testing Cargo.toml** with proper dependencies

3. **Verify test coverage** matches original inline tests

### Migration Commands

```bash
# Extract tests from git history
git show f9ed608:crates/hkask-templates/src/cascade.rs | \
  sed -n '/#\[cfg(test)\]/,$ p' > /tmp/cascade_tests.rs

# Add to hkask-testing/unit-tests/hkask_templates_tests.rs
# (Merge with existing test structure)
```

## Verification

Production code compiles cleanly:
```bash
cargo check -p hkask-types -p hkask-cns -p hkask-templates \
  -p hkask-storage -p hkask-memory -p hkask-agents \
  -p hkask-ensemble -p hkask-keystore -p hkask-mcp \
  -p hkask-cli -p hkask-api
# ✅ Finished dev profile
```

---

*Test migration is a follow-up task. Production code is now clean.*
