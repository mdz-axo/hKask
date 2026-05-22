# Documentation Audit — 2026-05-22

**Conducted by:** Kilo Agent  
**Date:** 2026-05-22  
**Purpose:** Classify all documents in `docs/` for retention, archival, or deletion per TOGAF-Lite refresh

---

## Classification Summary

| Disposition | Count | Rationale |
|-------------|-------|-----------|
| **Retain-and-Update** | 28 | Core architecture, standards, active specifications |
| **Archive** | 67 | Completion reports, status snapshots, superseded plans, remediation logs |
| **Delete** | 3 | True duplicates, redundant logs |

---

## Retain-and-Update (28 files)

### Standards (4)
- `docs/standards/DOCUMENTATION_STANDARDS.md` — Active, v0.3.0
- `docs/standards/WRITING_EXCELLENCE.md` — Active, v0.3.0
- `docs/standards/WRITING_EXCELLENCE_AUDIT.md` — Active audit protocol
- `docs/standards/GOVERNANCE.md` — Active governance

### Architecture — Core (8)
- `docs/architecture/hKask-architecture-master.md` — Master spec, v0.21.0
- `docs/architecture/hKask-architecture-index.md` — Active index
- `docs/architecture/hKask-erd.md` — ERD, v0.21.0
- `docs/architecture/hKask-hLexicon.md` — hLexicon spec
- `docs/architecture/business-architecture.md` — TOGAF Phase B (needs update)
- `docs/architecture/data-architecture.md` — TOGAF Phase C (needs update)
- `docs/architecture/application-architecture.md` — TOGAF Phase C (needs update)
- `docs/architecture/security-architecture.md` — TOGAF Phase C (needs update)

### Architecture — Principles (2)
- `docs/architecture/PRINCIPLES.md` — Architecture principles
- `docs/architecture/magna-carta.md` — Core contract

### Architecture — CNS/νKask (2)
- `docs/architecture/vKask-cybernetic-constant.md` — CNS theory
- `docs/architecture/vKask-erd.md` — CNS ERD

### Architecture — Registry/Templates (3)
- `docs/architecture/registry-templating-prompt-v2.md` — Registry design
- `docs/architecture/registry-erd.md` — Registry ERD
- `docs/architecture/template-header-standard.md` — Template format

### Architecture — Agents (2)
- `docs/architecture/AGENT_POD_IMPLEMENTATION.md` — Pod implementation
- `docs/architecture/hKask-Curator-persona.md` — Curator spec

### Specifications (3)
- `docs/specifications/MODEL_CATALOG.md` — Model catalog
- `docs/specifications/chaos-testing-spec.md` — Testing spec
- `docs/specifications/metrics-dashboard-spec.md` — Metrics spec

### Plans (2)
- `docs/plans/roadmap.md` — Active roadmap
- `docs/plans/backstory-r7.md` — Contextual backstory

### User Guides (2)
- `docs/user-guides/SECURITY.md` — Security guide
- `docs/user-guides/README-AGENT-PODS.md` — Pod guide

### GML (2)
- `docs/gml/README.md` — GML intro
- `docs/gml/gml-architecture.md` — GML architecture

---

## Archive (67 files)

### Completion Reports (19)
- `docs/architecture/BOT_MEMORY_PRODUCTION.md` — Completion report
- `docs/architecture/BOT_SYSTEM_SETUP.md` — Setup guide (completed)
- `docs/architecture/CURATOR_REPORTING_STRUCTURE.md` — Reporting (completed)
- `docs/architecture/MCP_SERVERS_IMPLEMENTATION_COMPLETE.md` — MCP complete
- `docs/architecture/MCP_SERVERS_IMPLEMENTATION_STATUS.md` — Status snapshot
- `docs/architecture/MCP_SERVERS_SUMMARY.md` — Summary snapshot
- `docs/architecture/MCP_SERVERS_FULL_IMPLEMENTATION_GUIDE.md` — Guide (completed)
- `docs/architecture/RUSSELL_MIGRATION_STATUS.md` — Migration status
- `docs/architecture/future_work_resolved.md` — Resolved questions
- `docs/architecture/kata-completion-report.md` — Kata complete
- `docs/architecture/kata-final-summary-v0.21.3.md` — Kata summary
- `docs/architecture/kata-iteration-support-v0.21.4.md` — Kata iteration
- `docs/architecture/kata-remediation-complete.md` — Kata remediation
- `docs/architecture/kata-system-final-summary.md` — Kata summary
- `docs/architecture/kata-system-summary.md` — Kata summary
- `docs/architecture/hKask-implementation-handoff.md` — Handoff (completed phase)
- `docs/architecture/STANDING_ENSEMBLE_SESSION.md` — Session setup
- `docs/architecture/SEMANTIC_INVENTORY.md` — Inventory snapshot
- `docs/progress/2026-05-19-phase4-runtime-integration.md` — Phase report

### Open Questions / Decisions (11)
- `docs/architecture/OPEN_QUESTIONS.md` — Open questions (being resolved)
- `docs/architecture/kata-decision-capability-metrics.md` — Decision record
- `docs/architecture/kata-decision-composition.md` — Decision record
- `docs/architecture/kata-decision-consent-revocation.md` — Decision record
- `docs/architecture/kata-decision-multi-bot-coaching.md` — Decision record
- `docs/architecture/kata-decision-retreat-criteria.md` — Decision record
- `docs/architecture/kata-decision-variety-baseline.md` — Decision record
- `docs/architecture/kata-decision-version-policy.md` — Decision record
- `docs/architecture/COMPILATION_RESOLUTION_OPEN_QUESTIONS.md` — Resolved
- `docs/architecture/registry-deferred-work.md` — Deferred work
- `docs/architecture/okapi-capability-model.md` — Capability model

### hLexicon/Kata Status Reports (6)
- `docs/architecture/hlexicon-functional-logic-note.md` — Design note
- `docs/architecture/hlexicon-governance-status.md` — Status
- `docs/architecture/hlexicon-rollout-complete.md` — Rollout complete
- `docs/architecture/hlexicon-separation-verified.md` — Verification
- `docs/architecture/hlexicon-validation-report.md` — Validation
- `docs/architecture/cli-api-symmetry-audit.md` — Audit report

### Progress Reports (3)
- `docs/progress/2026-05-19-phase4-runtime-integration.md` — Phase report
- `docs/progress/2026-05-20-phase2-phase5-partial-security-fixes.md` — Partial fixes
- `docs/progress/2026-05-20-phase2-phase5-security-integration.md` — Integration
- `docs/progress/chaos-testing-summary.md` — Test summary
- `docs/progress/items-1-3-summary.md` — Summary

### Remediation Logs (8)
- `docs/remediation/CNS_REVIEW_QUEUE_SIMPLIFICATION.md` — Remediation
- `docs/remediation/CONSOLIDATION_PLAN.md` — Plan (completed)
- `docs/remediation/DOCUMENTATION_OVERHAUL_SUMMARY.md` — Summary
- `docs/remediation/MCP_IMPLEMENTATION_SUMMARY.md` — Summary
- `docs/remediation/MCP_SERVER_ANALYSIS.md` — Analysis
- `docs/remediation/REMEDIATION_COMPLETE.md` — Complete
- `docs/remediation/REMEDIATION_IMPLEMENTATION.md` — Implementation
- `docs/remediation/TEST_MIGRATION_SUMMARY.md` — Migration summary
- `docs/remediation/capability-energy-integration.md` — Integration
- `docs/remediation/open_questions_capability_composition.md` — Questions
- `docs/remediation/session_progress_2026-05-20.md` — Session log

### Migration Documents (4)
- `docs/migration/mcp_optimization_analysis.md` — Analysis
- `docs/migration/migration_completion_report.md` — Complete
- `docs/migration/migration_inventory.md` — Inventory
- `docs/migration/security_audit_report.md` — Audit
- `docs/migration/strategy.md` — Strategy

### Plans — Superseded (4)
- `docs/plans/curator-persona.md` — Superseded by architecture version
- `docs/plans/curator.md` — Superseded
- `docs/plans/personas-r7.md` — Superseded
- `docs/plans/high-temp-templates.md` — Superseded
- `docs/plans/gml-allosteric-thinking-v2.md` — Superseded

### GML Implementation (7)
- `docs/gml/gml-architecture-update-v0.2.md` — Update
- `docs/gml/gml-implementation-summary.md` — Summary
- `docs/gml/gml-mcp-server.md` — Server doc
- `docs/gml/gml-minimalism-audit.md` — Audit
- `docs/gml/gml-remediation-progress.md` — Progress
- `docs/gml/gml-research-agenda.md` — Agenda
- `docs/gml/gml-research-paper.md` — Paper
- `docs/gml/gml-security-audit.md` — Audit
- `docs/gml/gml-user-guide.md` — Guide
- `docs/gml/task10-verification-tests.md` — Tests
- `docs/gml/task2-domain-logic-extraction.md` — Extraction
- `docs/gml/task3-capability-infrastructure.md` — Infrastructure
- `docs/gml/task8-cns-adapter.md` — Adapter
- `docs/gml/gml-architecture.md` — Keep (core)

### Integrations (2)
- `docs/integrations/macaroon-issuer.md` — Integration spec
- `docs/integrations/russell-acp-agent.md` — Integration

### Generated (1)
- `docs/generated/cli.md` — Generated output

### Other (2)
- `docs/P0_OKAPI_INTEGRATION_PLAN.md` — Plan (large, 45KB)
- `docs/CI-CD-GUIDE.md` — CI/CD guide (keep or archive?)

---

## Delete (3 files)

True duplicates or redundant logs:
- `docs/architecture/pragmatic-composition-erd.md` — Duplicate of registry-erd
- `docs/architecture/russell-hkask-mapping-erd.md` — Migration ERD (completed)
- `docs/artifacts/README.md` — Empty placeholder

---

## Action Plan

1. Create `docs/archive/2026-05-22-documentation-refresh/`
2. Move all "Archive" files there
3. Delete all "Delete" files
4. Update retained documents with proper metadata headers
5. Rewrite architecture domain documents
6. Create `docs/status/PROJECT_STATUS.md` as single source of truth
7. Update `docs/plans/TODO.md`
8. Run link checker

---

## Verification Commands

```bash
# Count files
find docs -type f -name "*.md" | wc -l

# Check for broken links
.github/scripts/check_links.sh

# Verify metadata headers
grep -L "^Version:\|^version:" docs/**/*.md 2>/dev/null || grep -L "^##" docs/**/*.md
```
