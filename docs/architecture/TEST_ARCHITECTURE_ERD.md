# Test Architecture ERD

```mermaid
erDiagram
    PRODUCTION_CRATES ||--o{ HKASK_TESTING : "depends-on"
    
    PRODUCTION_CRATES {
        string name
        int loc_count
        bool budget_excluded FALSE
    }
    
    HKASK_TESTING {
        string name "hkask-testing"
        int test_count "63+ tests"
        bool budget_excluded TRUE
        string[] unit_tests
        string[] integration_tests
    }
    
    UNIT_TESTS ||--|| HKASK_TESTING : "contained-in"
    UNIT_TESTS {
        string[] files "adapter_tests, sovereignty_observer_tests, ..."
        int test_count "49 migrated"
        string imports "Public APIs only"
    }
    
    INTEGRATION_TESTS ||--|| HKASK_TESTING : "contained-in"
    INTEGRATION_TESTS {
        string[] files "sovereignty_tests, chaos_integration, ..."
        int test_count "14+"
        string requires "Database, Okapi instance"
    }
    
    TEST_MIGRATION ||--|| UNIT_TESTS : "populates"
    TEST_MIGRATION {
        string source "Inline #[cfg(test)]"
        string destination "hkask-testing/unit-tests/"
        string benefit "Excluded from LOC budget"
    }
    
    COMPILATION_GATE ||--|| HKASK_TESTING : "validates"
    COMPILATION_GATE {
        string ci_step "cargo test -p hkask-testing --all-targets --no-run"
        string enforcement "Blocks merge on failure"
    }
```

---

## Hexagonal Boundary Analysis

### Ports (Interfaces)

| Port | Direction | Implementation | Test Location |
|------|-----------|----------------|---------------|
| `MemoryStoragePort` | Outbound | `MemoryStorageAdapter` | `adapter_tests.rs` |
| `ACPRuntimePort` | Outbound | `AcpRuntimeAdapter` | `adapter_tests.rs` |
| `MCPRuntimePort` | Outbound | `McpRuntimeAdapter` | `adapter_tests.rs` |
| `CNSSpanPort` | Outbound | `CnsEmitterAdapter` | `adapter_tests.rs` |
| `GitCASPort` | Outbound | `GitCasAdapter` | `adapter_tests.rs` |
| `SovereigntyPort` | Inbound | `SovereigntyChecker` | `sovereignty_observer_tests.rs` |

### Test Boundary Rules

1. **Tests import ports, not adapters** — Tests use public API
2. **No private access** — Tests cannot access `mod tests` internals
3. **Dependency injection** — Tests can swap adapters (mock/real)
4. **Isolation** — Test failures don't block production builds

---

## RDF Triple Graph

```turtle
# Test Architecture
hkask-testing :excludesFromBudget true .
hkask-testing :contains unit-tests .
hkask-testing :contains integration-tests .

# Test Migration
adapter_tests :migratedFrom "hkask-agents/src/adapters/*.rs" .
adapter_tests :testCount 17 .
sovereignty_observer_tests :migratedFrom "hkask-agents/src/sovereignty.rs, hkask-cns/src/observers/sovereignty.rs" .
sovereignty_observer_tests :testCount 14 .
mcp_adapter_tests :migratedFrom "hkask-mcp/src/adapter_container.rs, hkask-mcp/src/archival_service.rs" .
mcp_adapter_tests :testCount 7 .
templates_unit_tests :migratedFrom "hkask-templates/src/curator_pipeline.rs, hkask-templates/src/russell_mapper.rs" .
templates_unit_tests :testCount 11 .

# Hexagonal Architecture
MemoryStoragePort :type OutboundPort .
MemoryStorageAdapter :implements MemoryStoragePort .
adapter_tests :tests MemoryStorageAdapter .

# Security Boundaries
SovereigntyPort :enforces OCAP .
SovereigntyChecker :implements SovereigntyPort .
sovereignty_observer_tests :verifies SovereigntyChecker .

# Compilation Gate
compilation-gate :requires hkask-testing .
compilation-gate :blocksMergeOn failure .
```

---

*Document generated: 2026-05-22*
*Part of hKask Architecture Documentation (Phase 5)*