---
title: "MDS Documentation Scaffold"
audience: [architects, documentation maintainers, agents]
last_updated: 2026-06-10
version: "2.5.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# MDS Documentation Scaffold

**Purpose:** Maps the MDS 5-category goal-group taxonomy to directory locations and enforces the lifecycle policy.

**Role:** This document is a generation guideline. It tells you what documentation to produce and where to put it. Verification that what you produced is correct and complete is governed by [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md).

**MDS Reference:** [`architecture/MDS.md`](../architecture/MDS.md)

---

## 1. MDS Category → Document Mapping

The current directory layout (grouped by artifact type) serves the MDS category topology because each authoritative spec document is named by its category cluster and the portal (`docs/README.md`) provides category-based navigation. The mapping below is the authoritative category→document→directory assignment.

| # | MDS Category | Authoritative Document | Directory | Supporting References |
|---|----------------|----------------------|----------------------|
| 1 | **Domain** | [`MDS.md`](../architecture/MDS.md) | `architecture/` | [`reference/hKask-hLexicon.md`](../architecture/reference/hKask-hLexicon.md), [`reference/hKask-Curator-persona.md`](../architecture/reference/hKask-Curator-persona.md) |
| 2 | **Composition** | [`MDS.md`](../architecture/MDS.md) | `architecture/` | [`reference/template-header-standard.md`](../architecture/reference/template-header-standard.md) |
| 3 | **Trust** | [`MDS.md`](../architecture/MDS.md) | `architecture/` | [`magna-carta.md`](../architecture/magna-carta.md) |
| 4 | **Lifecycle** | [`MDS.md`](../architecture/MDS.md) | `architecture/` | [`CI-CD-GUIDE.md`](../specifications/CI-CD-GUIDE.md), [`DEPLOYMENT.md`](../specifications/DEPLOYMENT.md) |
| 5 | **Curation** | [`MDS.md`](../architecture/MDS.md) + [`WRITING_EXCELLENCE.md`](../specifications/WRITING_EXCELLENCE.md) | `architecture/` + `specifications/` | — |

[^evans-ddd]: Evans, Eric. *Domain-Driven Design: Tackling Complexity in the Heart of Software.* Addison-Wesley, 2003. — Bounded contexts and the domain model that MDS categories map to document locations.

---

## 2. Document Structure

```
docs/
├── README.md                              # PORTAL (indexes all active docs by MDS category)
├── architecture/
│   ├── hKask-architecture-master.md       # INDEX (thin pointer)
│   ├── MDS.md                          # FRAMEWORK (taxonomy + methodology)
│   ├── PRINCIPLES.md                      # FRAMEWORK (P1-P9)
│   ├── magna-carta.md                     # FRAMEWORK (user sovereignty)
│   ├── refactoring-plan-services-2026-06-09.md  # PLAN (service layer refactoring)
│   ├── semantic-condensation-analysis.md        # ANALYSIS (condensation algorithms)
│   ├── ADR-024-*.md                       # DECISION RECORD
│   ├── ADR-025-*.md                       # DECISION RECORD
│   ├── ADR-026-*.md                       # DECISION RECORD
│   ├── ADR-027-*.md                       # DECISION RECORD
│   ├── ADR-030-*.md                       # DECISION RECORD (proposed)
│   ├── ADR-031-*.md                       # DECISION RECORD
│   ├── ADR-032-*.md                       # DECISION RECORD (draft)
│   ├── ADR-033-*.md                       # DECISION RECORD (draft)
│   ├── loop-architecture.md              # FRAMEWORK (4-loop authority model)
│   └── reference/
│       ├── hKask-hLexicon.md              # Vocabulary catalog
│       ├── ports-inventory.md             # Port reference
│       ├── utoipa-implementation.md       # API guide
│       ├── template-header-standard.md    # Format reference
│       ├── hKask-Curator-persona.md       # Persona spec
│       └── okapi-integration.md           # Okapi API contract
├── specifications/
│   ├── MDS_SCAFFOLD.md                 # THIS FILE
│   ├── REQUIREMENTS.md                    # Goal specs
│   ├── TRACEABILITY_MATRIX.md             # Code→test traceability
│   ├── DOCUMENTATION_STANDARDS.md         # Documentation standards
│   ├── WRITING_EXCELLENCE.md              # Writing quality protocol
│   ├── DEPENDENCY_POLICY.md               # Dependency policy
│   ├── ADR_TEMPLATE.md                    # ADR template
│   ├── CI-CD-GUIDE.md                     # CI/CD guide
│   ├── DEPLOYMENT.md                      # Deployment guide
│   ├── TESTING_STANDARDS.md               # Testing protocol
│   ├── test-program.md                    # Test program spec
│   ├── REPL-specification.md              # REPL specification
│   └── hhh-alignment-research.md          # HHH alignment research
├── plans/
│   ├── TODO.md                            # Open work
├── status/                                 # Status files (7 active: PROJECT_STATUS, test-inventory, mcp-tools-inventory, spec-code-drift, curation-decisions, fowler-audit-status, adversarial-simplification-inventory)
├── user-guides/                           # User-facing guides
│   ├── AGENT-POD-CREATION-GUIDE.md       # Pod creation guide
│   └── COMMON-AGENT-PATTERNS.md          # Agent patterns reference
├── archive/                                # Archived documents (gitignored)
├── ci/                                     # CI verification scripts
│   ├── check-links.sh                     # Link integrity checker
│   └── check-metadata.sh                  # Metadata compliance checker
├── DIAGRAMS_INDEX.md                       # Diagram index
├── OPEN_QUESTIONS.md                        # Unresolved aspects
└── generated/                             # Auto-generated artifacts
    ├── cli-reference.md                    # Auto-generated CLI reference
    └── openapi.json                        # OpenAPI specification
```

[^cockburn-hexagonal]: Cockburn, A. (2005). *Hexagonal Architecture.* https://alistair.cockburn.us/hexagonal-architecture/ — Ports and adapters pattern that informs the document structure's separation of architecture, specifications, and generated artifacts.

---

## 3. Lifecycle Enforcement

Per [`DOCUMENTATION_STANDARDS.md`](../specifications/DOCUMENTATION_STANDARDS.md) §3:

```
Draft → Active → Deprecated → Superseded → Removed
```

- **Active** documents must map to ≥1 MDS category via `mds_categories` metadata
- **Deprecated/Superseded** documents moved to `docs/archive/YYYY-MM-DD-<label>/`
- **Removed** documents deleted; git history is archive of record
- `docs/archive/` is gitignored

[^nygard-adr]: Nygard, M. (2011). *Documenting Architecture Decisions.* Relevance. http://thinkrelevance.com/blog/2011/11/15/documenting-architecture-decisions — ADR lifecycle states (Draft → Active → Deprecated → Superseded) that MDS_SCAFFOLD enforces.

---

## 4. Spec-Code Completeness Predicate

Per [`MDS.md`](../architecture/MDS.md) §3.2 and the axiom `Spec-document completeness ⊥ Code-implementation completeness`:

**This table evaluates both spec-document completeness AND code-implementation completeness.** Spec-code drift items are tracked in [`spec-code-drift.yaml`](../status/spec-code-drift.yaml) and curation decisions in [`curation-decisions.yaml`](../status/curation-decisions.yaml).

| Category | Authoritative Document | Spec-Document Complete? | Code-Implementation Complete? | Curation Decision | Drift Items |
|----------|----------------------|-------------------------|-------------------------------|-------------------|-------------|
| Domain | `MDS.md` | ✅ | ⚠️ | Merge + Revise | P2-06-D6 (TemplateInvocation stub), DRIFT-001 (CapabilityToken alias), DRIFT-002 (TemplateInvocation in ERD) |
| Composition | `MDS.md` | ✅ | ⚠️ | Merge + Revise | P2-06-D9 (derivation stubs) |
| Trust | `MDS.md` | ✅ | ⚠️ | Merge + Revise | P2-06-D2 (Caveat visibility), D3 (CapabilityToken alias), D5 (CapabilityAwareValidator stub), D7 (SecurityGateway — superseded by GovernedTool), P2-06-D1 (5 hierarchical CNS spans — now registered) |
| Lifecycle | `MDS.md` | ✅ | ✅ | Merge | — |
| Curation | `MDS.md` + `WRITING_EXCELLENCE.md` | ✅ | ⚠️ | Merge + Revise | DA-4-code_ahead (SpecStore method names), DA-5-code_ahead (DefaultSpecCurator exists), DRIFT-004 (self-application matrix labels) |

**Result:** 5/5 categories have authoritative spec documents. 3/5 categories have code-implementation gaps (marked ⚠️). All drift items have curation decisions recorded in [`curation-decisions.yaml`](../status/curation-decisions.yaml). Code-implementation gaps are tracked in [`spec-code-drift.yaml`](../status/spec-code-drift.yaml) and [`plans/TODO.md`](../plans/TODO.md).

[^principles]: hKask Team. (2026). *Architecture Principles.* `docs/architecture/PRINCIPLES.md` — P1-P9 principles and constraint forces that govern spec-code completeness.

---

## 5. Metadata Requirements

Per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §2.

[^doc-standards]: hKask Team. (2026). *Documentation Standards.* `docs/specifications/DOCUMENTATION_STANDARDS.md` — Metadata requirements for all hKask documentation.

---

## 6. Verification Commands

Per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §10.

### 6.1 Self-Application Precedent

The `hkask-mcp-spec` server provides 5 MDS tools (`spec/goal/capture`, `spec/goal/decompose`, `spec/require/writing-quality`, `spec/graph/query`, `spec/graph/coherence`) per MDS that can in principle be used to capture and curate the specification corpus itself. Three curation tools (evaluate, reconcile, cultivate) and bind were deleted per MDS — curation is external to the spec server. This self-application is a future opportunity, not blocked by any circularity concern — the server's process is defined by its own spec and code; using it on the spec corpus is no more circular than using a compiler to compile itself. For v0.27.0, the spec tools are validated against the existing corpus; meta-curation (using spec tools on spec documents) is deferred to a future cycle.

---

## References

[^ddmvss]: hKask Team. (2026). *MDS — Minimal Domain Specification*. `docs/architecture/MDS.md`.
