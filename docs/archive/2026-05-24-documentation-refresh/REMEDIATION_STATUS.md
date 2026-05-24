# Remediation Status

**Version:** 0.21.0
**Last Updated:** 2026-05-24

---

## Action Plan Tracking

### W11: Document Reconciliation

| Task | Status |
|------|--------|
| Identify architecture master version (0.21.0) | done |
| Check magna-carta.md existence (does not exist) | done |
| Create docs/integrations/okapi-integration.md with canonical Okapi API contract | done |
| Create docs/REMEDIATION_STATUS.md | done |

### W12: Broken Import Cleanup

| Task | Status |
|------|--------|
| Audit broken/unused imports across workspace | remaining |
| Fix import paths after module moves | remaining |
| Remove stale re-exports | remaining |

### W13: Test Harness Synchronization

| Task | Status |
|------|--------|
| Fix MockMcpPort to match real McpPort trait (invoke, discover_tools, get_tool_info) | done |
| Remove MockSkillRegistryPort (SkillRegistryPort trait does not exist) | done |
| Remove duplicate TestMocks structs | done |
| Fix stale unit test imports in templates_unit_tests.rs | done |
| Add mocks.rs to module tree | done |
| Verify with cargo check | remaining |

### W14: Future Open Questions Documentation

| Task | Status |
|------|--------|
| Create docs/FUTURE_OPEN_QUESTIONS.md | done |

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
