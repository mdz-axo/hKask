---
title: "MDS Documentation Scaffold"
audience: [architects, documentation maintainers, agents]
last_updated: 2026-06-08
version: "2.5.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# MDS Documentation Scaffold

**Purpose:** Maps the MDS 9-category goal-group taxonomy to directory locations and enforces the lifecycle policy.

**Role:** This document is a generation guideline. It tells you what documentation to produce and where to put it. Verification that what you produced is correct and complete is governed by [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md).

**MDS Reference:** [`architecture/MDS.md`](../architecture/MDS.md) §3

---

## 1. MDS Category → Document Mapping

The current directory layout (grouped by artifact type) serves the MDS category topology because each authoritative spec document is named by its category cluster and the portal (`docs/README.md`) provides category-based navigation. The mapping below is the authoritative category→document→directory assignment.

| # | MDS Category | Authoritative Document | Directory | Supporting References |
|---|----------------|----------------------|----------------------|
| 1 | **Domain** | [`MDS.md §7.1-7.2`](../architecture/MDS.md §7.1-7.2) | `architecture/` | [`reference/hKask-hLexicon.md`](../architecture/reference/hKask-hLexicon.md), [`reference/hKask-Curator-persona.md`](../architecture/reference/hKask-Curator-persona.md) |
| 2 | **Capability** | [`MDS.md §7.1-7.2`](../architecture/MDS.md §7.1-7.2) | `architecture/` | [`reference/ports-inventory.md`](../architecture/reference/ports-inventory.md) |
| 3 | **Interface** | [`MDS.md §7.2`](../architecture/MDS.md §7.2) | `architecture/` | [`reference/utoipa-implementation.md`](../architecture/reference/utoipa-implementation.md), [`reference/ports-inventory.md`](../architecture/reference/ports-inventory.md) |
| 4 | **Composition** | [`MDS.md §7.2`](../architecture/MDS.md §7.2) | `architecture/` | [`reference/template-header-standard.md`](../architecture/reference/template-header-standard.md) |
| 5 | **Trust & Security** | [`MDS.md §7.3`](../architecture/MDS.md §7.3) | `architecture/` | [`magna-carta.md`](../architecture/magna-carta.md) |
| 6 | **Observability** | [`MDS.md §7.3`](../architecture/MDS.md §7.3) | `architecture/` | — |
| 7 | **Persistence** | [`MDS.md §7.4`](../architecture/MDS.md §7.4) | `architecture/` | [`reference/hKask-erd.md`](../architecture/reference/hKask-erd.md), [`reference/registry-erd.md`](../architecture/reference/registry-erd.md), [`reference/subsystem-erds.md`](../architecture/reference/subsystem-erds.md) |
| 8 | **Lifecycle** | [`MDS.md §7.4`](../architecture/MDS.md §7.4) | `architecture/` | [`CI-CD-GUIDE.md`](../specifications/CI-CD-GUIDE.md), [`DEPLOYMENT.md`](../specifications/DEPLOYMENT.md) |
| 9 | **Curation** | [`MDS.md`](../architecture/MDS.md) + [`WRITING_EXCELLENCE.md`](../specifications/WRITING_EXCELLENCE.md) | `architecture/` + `specifications/` | — |

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
│   ├── MDS.md §7.1-7.2           # SPEC (Domain + Capability)
│   ├── MDS.md §7.2       # SPEC (Interface + Composition)
│   ├── MDS.md §7.3    # SPEC (Trust + Observability)
│   ├── MDS.md §7.4       # SPEC (Persistence + Lifecycle)
│   ├── ADR-022-*.md                       # DECISION RECORD
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
│       ├── hKask-erd.md                   # Diagram artifact
│       ├── registry-erd.md                # Diagram artifact
│       ├── subsystem-erds.md              # Diagram artifact
│       ├── hKask-hLexicon.md              # Vocabulary catalog
│       ├── ports-inventory.md             # Port reference
│       ├── utoipa-implementation.md       # API guide
│       ├── template-header-standard.md    # Format reference
│       ├── hKask-Curator-persona.md       # Persona spec
│       └── okapi-integration.md           # Okapi API contract
│   ├── hlexicon-validation-report.md     # hLexicon compliance audit
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
│   └── hhh-alignment-research.md          # HHH alignment research
├── plans/
│   ├── TODO.md                            # Open work
│   └── high-temp-templates.md             # Template design draft
├── status/                                 # Status files (planned, not yet populated)
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

---

## 4. Spec-Code Completeness Predicate

Per [`MDS.md`](../architecture/MDS.md) §3.2 and the axiom `Spec-document completeness ⊥ Code-implementation completeness`:

**This table evaluates both spec-document completeness AND code-implementation completeness.** Spec-code drift items are tracked in [`spec-code-drift.yaml`](../status/spec-code-drift.yaml) and curation decisions in [`curation-decisions.yaml`](../status/curation-decisions.yaml).

| Category | Authoritative Document | Spec-Document Complete? | Code-Implementation Complete? | Curation Decision | Drift Items |
|----------|----------------------|-------------------------|-------------------------------|-------------------|-------------|
| Domain | `MDS.md §7.1-7.2` | ✅ | ⚠️ | Merge + Revise | P2-06-D6 (TemplateInvocation stub), DRIFT-001 (CapabilityToken alias), DRIFT-002 (TemplateInvocation in ERD) |
| Capability | `MDS.md §7.1-7.2` | ✅ | ⚠️ | Merge + Revise | P2-06-D4 (ContractValidator stub) |
| Interface | `MDS.md §7.2` | ✅ | ✅ | Merge | P2-06-D8 (McpTransport — resolved: rmcp handles transport) |
| Composition | `MDS.md §7.2` | ✅ | ⚠️ | Merge + Revise | P2-06-D9 (derivation stubs) |
| Trust & Security | `MDS.md §7.3` | ✅ | ⚠️ | Merge + Revise | P2-06-D2 (Caveat visibility), D3 (CapabilityToken alias), D5 (CapabilityAwareValidator stub), D7 (SecurityGateway — superseded by GovernedTool) |
| Observability | `MDS.md §7.3` | ✅ | ⚠️ | Merge + Revise | P2-06-D1 (5 hierarchical CNS spans — now registered) |
| Persistence | `MDS.md §7.4` | ✅ | ✅ | Merge | — |
| Lifecycle | `MDS.md §7.4` | ✅ | ✅ | Merge | — |
| Curation | `MDS.md` + `WRITING_EXCELLENCE.md` | ✅ | ⚠️ | Merge + Revise | DA-4-code_ahead (SpecStore method names), DA-5-code_ahead (DefaultSpecCurator exists), DRIFT-004 (self-application matrix labels) |

**Result:** 9/9 categories have authoritative spec documents. 5/9 categories have code-implementation gaps (marked ⚠️). All drift items have curation decisions recorded in [`curation-decisions.yaml`](../status/curation-decisions.yaml). Code-implementation gaps are tracked in [`spec-code-drift.yaml`](../status/spec-code-drift.yaml) and [`plans/TODO.md`](../plans/TODO.md).

---

## 5. Metadata Requirements

Per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §2.

---

## 6. Verification Commands

Per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §10.

### 6.1 Self-Application Precedent

The `hkask-mcp-spec` server provides 5 MDS tools (`spec/goal/capture`, `spec/goal/decompose`, `spec/require/writing-quality`, `spec/graph/query`, `spec/graph/coherence`) per MDS §3 that can in principle be used to capture and curate the specification corpus itself. Three curation tools (evaluate, reconcile, cultivate) and bind were deleted per MDS §3 — curation is external to the spec server. This self-application is a future opportunity, not blocked by any circularity concern — the server's process is defined by its own spec and code; using it on the spec corpus is no more circular than using a compiler to compile itself. For v0.27.0, the spec tools are validated against the existing corpus; meta-curation (using spec tools on spec documents) is deferred to a future cycle.

---

## References

[^ddmvss]: hKask Team. (2026). *MDS — Domain-Driven Minimum Viable Specification Set*. `docs/architecture/MDS.md`.
