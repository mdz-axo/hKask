---
title: "DDMVSS Documentation Scaffold"
audience: [architects, documentation maintainers, agents]
last_updated: 2026-06-08
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

## 4. Spec-Code Completeness Predicate

Per [`DDMVSS.md`](../architecture/DDMVSS.md) §3.2 and the axiom `Spec-document completeness ⊥ Code-implementation completeness`:

**This table evaluates both spec-document completeness AND code-implementation completeness.** Spec-code drift items are tracked in [`spec-code-drift.yaml`](../status/spec-code-drift.yaml) and curation decisions in [`curation-decisions.yaml`](../status/curation-decisions.yaml).

| Category | Authoritative Document | Spec-Document Complete? | Code-Implementation Complete? | Curation Decision | Drift Items |
|----------|----------------------|-------------------------|-------------------------------|-------------------|-------------|
| Domain | `domain-and-capability.md` | ✅ | ⚠️ | Merge + Revise | P2-06-D6 (TemplateInvocation stub), DRIFT-001 (CapabilityToken alias), DRIFT-002 (TemplateInvocation in ERD) |
| Capability | `domain-and-capability.md` | ✅ | ⚠️ | Merge + Revise | P2-06-D4 (ContractValidator stub) |
| Interface | `interface-and-composition.md` | ✅ | ✅ | Merge | P2-06-D8 (McpTransport — resolved: rmcp handles transport) |
| Composition | `interface-and-composition.md` | ✅ | ⚠️ | Merge + Revise | P2-06-D9 (derivation stubs) |
| Trust & Security | `trust-security-observability.md` | ✅ | ⚠️ | Merge + Revise | P2-06-D2 (Caveat visibility), D3 (CapabilityToken alias), D5 (CapabilityAwareValidator stub), D7 (SecurityGateway — superseded by GovernedTool) |
| Observability | `trust-security-observability.md` | ✅ | ⚠️ | Merge + Revise | P2-06-D1 (5 hierarchical CNS spans — now registered) |
| Persistence | `persistence-and-lifecycle.md` | ✅ | ✅ | Merge | — |
| Lifecycle | `persistence-and-lifecycle.md` | ✅ | ✅ | Merge | — |
| Curation | `DDMVSS.md` + `WRITING_EXCELLENCE.md` | ✅ | ⚠️ | Merge + Revise | DA-4-code_ahead (SpecStore method names), DA-5-code_ahead (DefaultSpecCurator exists), DRIFT-004 (self-application matrix labels) |

**Result:** 9/9 categories have authoritative spec documents. 5/9 categories have code-implementation gaps (marked ⚠️). All drift items have curation decisions recorded in [`curation-decisions.yaml`](../status/curation-decisions.yaml). Code-implementation gaps are tracked in [`spec-code-drift.yaml`](../status/spec-code-drift.yaml) and [`plans/TODO.md`](../plans/TODO.md).

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
