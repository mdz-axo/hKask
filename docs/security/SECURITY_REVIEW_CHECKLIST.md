# Security Review Checklist

## S4.1: get_tool API OCAP Boundary

### Current Implementation
- **Endpoint**: `GET /api/mcp/tools/:name`
- **Location**: `crates/hkask-api/src/routes.rs:479`
- **Status**: ⚠️ **REQUIRES CAPABILITY CHECK**

### Required Fix
```rust
async fn get_tool(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    // TODO: Add capability token from headers
    headers: HeaderMap,
) -> Result<Json<ToolResponse>, StatusCode> {
    // 1. Extract capability token from Authorization header
    // 2. Verify token grants tool:read access
    // 3. Check tool name against token scope
    // 4. Only then allow access
}
```

### OCAP Design (Mark Miller Principles)
1. **Principle of Least Authority** — Token must explicitly grant `tool:read`
2. **End-to-End Security** — Capability check at API boundary, not just MCP layer
3. **Defense in Depth** — Multiple validation points (API → MCP → Tool)

---

## S4.2: regex-lite Security Patterns

### Current Patterns
| Pattern | Purpose | Risk Level |
|---------|---------|------------|
| `\|\s*([a-zA-Z_][a-zA-Z0-9_]*)` | Filter extraction | ✅ Low (ASCII-only, fixed) |
| `\bis\s+([a-zA-Z_][a-zA-Z0-9_]*)` | Test extraction | ✅ Low (ASCII-only, fixed) |

### Security Properties
- ✅ **Size limits** — regex-lite has built-in size limits
- ✅ **Fixed patterns** — Not user-provided, no injection risk
- ✅ **O(m*n) worst case** — Protected against ReDoS
- ✅ **ASCII-only** — No Unicode complexity attacks

### Recommendation
**RETAIN** — Patterns are security-appropriate for template filtering.

---

## S4.3: Sovereignty Test Coverage

### Coverage Map

| Boundary | Test Location | Status |
|----------|---------------|--------|
| `SovereigntyPort::check` | `sovereignty_observer_tests.rs` | ✅ Covered |
| `SovereigntyPort::can_access` | `sovereignty_observer_tests.rs` | ✅ Covered |
| `MemoryStoragePort::recall` | `adapter_tests.rs` | ✅ Covered |
| `OCAPBoundary` | `ocap_tests.rs` | ✅ Covered |
| `ConsentManager` | `consent_tests.rs` | ✅ Covered |

### Gaps
- ⚠️ **Integration tests** — End-to-end sovereignty with CNS alerts
- ⚠️ **Algedonic escalation** — Threshold triggering not tested

---

## S4.4: #[allow(dead_code)] Audit

| Location | Code | Justification | Review Date |
|----------|------|---------------|-------------|
| `consent.rs:73` | `store` field | Reserved for persistence | 2026-08-22 |
| `commands.rs:61-78` | MCP CLI functions | Reserved for future CLI | 2026-08-22 |

### Policy
- **90-day rule** — Remove if not implemented within 90 days
- **Documentation** — Must have comment explaining reservation
- **Review** — Quarterly audit of all `#[allow(dead_code)]`

---

*Review completed: 2026-05-22*
*Part of hKask Security Review (Phase 4)*