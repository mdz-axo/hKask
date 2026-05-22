# Test Migration Pattern

## When to Migrate Tests

Migrate inline tests to `hkask-testing` when:
1. Test is in `crates/*/src/**/*.rs` with `#[cfg(test)]`
2. Test does not require private crate internals
3. Test can use public API only

## Migration Steps

### Step 1: Create Test File
```bash
# Create in appropriate directory
hkask-testing/unit-tests/<feature>_tests.rs  # Unit tests
hkask-testing/integration-tests/<feature>_tests.rs  # Integration tests
```

### Step 2: Add Test Entry to Cargo.toml
```toml
# hkask-testing/Cargo.toml
[[test]]
name = "feature_tests"
path = "unit-tests/feature_tests.rs"
```

### Step 3: Migrate Test Code
```rust
// Before: crates/hkask-agents/src/adapters/example.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_example() {
        // ...
    }
}

// After: hkask-testing/unit-tests/adapter_tests.rs
use hkask_agents::adapters::example::ExampleAdapter;

#[test]
fn test_example() {
    // ...
}
```

### Step 4: Remove Inline Tests
Delete `#[cfg(test)] mod tests { ... }` from production file.

### Step 5: Verify
```bash
# Ensure tests pass
cargo test -p hkask-testing --test feature_tests

# Ensure production crate compiles
cargo check -p hkask-agents
```

## Test File Naming

| Pattern | Location | Example |
|---------|----------|---------|
| `adapter_tests.rs` | `unit-tests/` | Tests for adapters |
| `sovereignty_tests.rs` | `unit-tests/` | Sovereignty checks |
| `mcp_adapter_tests.rs` | `unit-tests/` | MCP runtime tests |
| `templates_unit_tests.rs` | `unit-tests/` | Template engine tests |
| `sovereignty_tests.rs` | `integration-tests/` | End-to-end sovereignty |

## Dependencies

Add required dependencies to `hkask-testing/Cargo.toml`:
```toml
[dependencies]
hkask-agents = { path = "../crates/hkask-agents" }
hkask-types = { path = "../crates/hkask-types" }
# ... other crates as needed
```

## Anti-Patterns

❌ **Don't:**
- Keep inline tests "just for this one case"
- Test private functions (refactor to public or extract)
- Create circular dependencies between test and production
- Skip migration for "complex" tests

✅ **Do:**
- Migrate all tests consistently
- Use public APIs only
- Keep test files focused (one feature per file)
- Document test setup requirements

## Benefits

| Aspect | Before | After |
|--------|--------|-------|
| **LOC Budget** | Counts toward 30k | Excluded |
| **Dependencies** | Limited to crate | Full workspace |
| **Compilation** | Slows crate rebuild | Isolated, faster |
| **Maintenance** | Scattered | Centralized |
| **Architecture** | Blurred boundaries | Clear hexagonal ports |

---
*Document generated: 2026-05-22*
*Part of hKask Test Architecture Integrity (Phase 1)*