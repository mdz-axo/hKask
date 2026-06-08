---
title: "DDMVSS Documentation Scaffold"
audience: [architects, documentation maintainers, agents]
last_updated: 2026-06-07
version: "2.5.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# DDMVSS Documentation Scaffold

**Purpose:** Maps the DDMVSS 9-category goal-group taxonomy to directory locations and enforces the lifecycle policy.

**Role:** This document is a generation guideline. It tells you what documentation to produce and where to put it. Verification that what you produced is correct and complete is governed by [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md).

**DDMVSS Reference:** [`architecture/DDMVSS.md`](../architecture/DDMVSS.md) §3

---

## 1. DDMVSS Category → Document Mapping

The current directory layout (grouped by artifact type) serves the DDMVSS category topology because each authoritative spec document is named by its category cluster and the portal (`docs/README.md`) provides category-based navigation. The mapping below is the authoritative category→document→directory assignment.

| # | DDMVSS Category | Authoritative Document | Directory | Supporting References |
|---|----------------|----------------------|----------------------|
| 1 | **Domain** | [`domain-and-capability.md`](../architecture/domain-and-capability.md) | `architecture/` | [`reference/hKask-hLexicon.md`](../architecture/reference/hKask-hLexicon.md), [`reference/hKask-Curator-persona.md`](../architecture/reference/hKask-Curator-persona.md) |
| 2 | **Capability** | [`domain-and-capability.md`](../architecture/domain-and-capability.md) | `architecture/` | [`reference/ports-inventory.md`](../architecture/reference/ports-inventory.md) |
| 3 | **Interface** | [`interface-and-composition.md`](../architecture/interface-and-composition.md) | `architecture/` | [`reference/utoipa-implementation.md`](../architecture/reference/utoipa-implementation.md), [`reference/ports-inventory.md`](../architecture/reference/ports-inventory.md) |
| 4 | **Composition** | [`interface-and-composition.md`](../architecture/interface-and-composition.md) | `architecture/` | [`reference/template-header-standard.md`](../architecture/reference/template-header-standard.md) |
| 5 | **Trust & Security** | [`trust-security-observability.md`](../architecture/trust-security-observability.md) | `architecture/` | [`magna-carta.md`](../architecture/magna-carta.md) |
| 6 | **Observability** | [`trust-security-observability.md`](../architecture/trust-security-observability.md) | `architecture/` | — |
| 7 | **Persistence** | [`persistence-and-lifecycle.md`](../architecture/persistence-and-lifecycle.md) | `architecture/` | [`reference/hKask-erd.md`](../architecture/reference/hKask-erd.md), [`reference/registry-erd.md`](../architecture/reference/registry-erd.md), [`reference/subsystem-erds.md`](../architecture/reference/subsystem-erds.md) |
| 8 | **Lifecycle** | [`persistence-and-lifecycle.md`](../architecture/persistence-and-lifecycle.md) | `architecture/` | [`CI-CD-GUIDE.md`](../specifications/CI-CD-GUIDE.md), [`DEPLOYMENT.md`](../specifications/DEPLOYMENT.md) |
| 9 | **Curation** | [`DDMVSS.md`](../architecture/DDMVSS.md) + [`WRITING_EXCELLENCE.md`](../specifications/WRITING_EXCELLENCE.md) | `architecture/` + `specifications/` | — |

---

## 2. Document Structure

```
docs/
├── README.md                              # PORTAL (indexes all active docs by DDMVSS category)
├── architecture/
│   ├── hKask-architecture-master.md       # INDEX (thin pointer)
│   ├── DDMVSS.md                          # FRAMEWORK (taxonomy + methodology)
│   ├── PRINCIPLES.md                      # FRAMEWORK (P1-P7, C1-C7)
│   ├── magna-carta.md                     # FRAMEWORK (user sovereignty)
│   ├── domain-and-capability.md           # SPEC (Domain + Capability)
│   ├── interface-and-composition.md       # SPEC (Interface + Composition)
│   ├── trust-security-observability.md    # SPEC (Trust + Observability)
│   ├── persistence-and-lifecycle.md       # SPEC (Persistence + Lifecycle)
│   ├── ADR-022-*.md                       # DECISION RECORD
│   ├── ADR-024-*.md                       # DECISION RECORD
│   ├── ADR-025-*.md                       # DECISION RECORD
│   ├── ADR-026-*.md                       # DECISION RECORD
│   ├── ADR-027-*.md                       # DECISION RECORD
│   ├── ADR-030-*.md                       # DECISION RECORD (proposed)
│   ├── ADR-031-*.md                       # DECISION RECORD
│   ├── ADR-032-*.md                       # DECISION RECORD (draft)
│   ├── ADR-033-*.md                       # DECISION RECORD (draft)
│   ├── loop-architecture.md              # FRAMEWORK (6-loop authority model)
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
│   ├── DDMVSS_SCAFFOLD.md                 # THIS FILE
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

- **Active** documents must map to ≥1 DDMVSS category via `ddmvss_categories` metadata
- **Deprecated/Superseded** documents moved to `docs/archive/YYYY-MM-DD-<label>/`
- **Removed** documents deleted; git history is archive of record
- `docs/archive/` is gitignored

---

## 4. Spec-Document Completeness Predicate

Per [`DDMVSS.md`](../architecture/DDMVSS.md) §3.2 and the axiom `Spec-document completeness ⊥ Code-implementation completeness`:

**This table evaluates spec-document completeness only** — whether each category has an authoritative specification document that is internally consistent and properly cross-referenced. It does not evaluate whether the code implementing those specifications is complete. Code-implementation gaps are tracked in [`plans/TODO.md`](../plans/TODO.md) and [`OPEN_QUESTIONS.md`](../OPEN_QUESTIONS.md).

| Category | Authoritative Document | Spec-Document Complete? | Curated? |
|----------|----------------------|-------------------------|----------|
| Domain | `domain-and-capability.md` | ✅ | ✅ Merge |
| Capability | `domain-and-capability.md` | ✅ | ✅ Merge |
| Interface | `interface-and-composition.md` | ✅ | ✅ Merge |
| Composition | `interface-and-composition.md` | ✅ | ✅ Merge |
| Trust & Security | `trust-security-observability.md` | ✅ | ✅ Merge |
| Observability | `trust-security-observability.md` | ✅ | ✅ Merge |
| Persistence | `persistence-and-lifecycle.md` | ✅ | ✅ Merge |
| Lifecycle | `persistence-and-lifecycle.md` | ✅ | ✅ Merge |
| Curation | `DDMVSS.md` + `WRITING_EXCELLENCE.md` | ✅ | ✅ Merge |

**Result:** 9/9 categories have authoritative spec documents that are internally consistent and cross-referenced. Code-implementation gaps (MCP≡CLI≡API equivalence verification, SpecStore bitemporal query methods, curation record persistence wiring, coherence threshold calibration) are code tasks, not spec-document gaps — tracked in [`OPEN_QUESTIONS.md`](../OPEN_QUESTIONS.md).

---

## 5. Metadata Requirements

Per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §2.

---

## 6. Verification Commands

Per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §10.

### 6.1 Self-Application Precedent

The `hkask-mcp-spec` server provides 11 DDMVSS tools (`spec/goal/capture`, `spec/goal/decompose`, `spec/require/bind`, `spec/curate/evaluate`, `spec/curate/reconcile`, `spec/curate/cultivate`, `spec/curate/writing-excellence`, `spec/graph/query`, `spec/graph/validate`, `spec/test/invariant`, `spec/test/verify`) that can in principle be used to capture and curate the specification corpus itself. This self-application is a future opportunity, not blocked by any circularity concern — the server's process is defined by its own spec and code; using it on the spec corpus is no more circular than using a compiler to compile itself. For v0.23.0, the spec tools are validated against the existing corpus; meta-curation (using spec tools on spec documents) is deferred to a future cycle.

---

## References

[^ddmvss]: hKask Team. (2026). *DDMVSS — Domain-Driven Minimum Viable Specification Set*. `docs/architecture/DDMVSS.md`.
