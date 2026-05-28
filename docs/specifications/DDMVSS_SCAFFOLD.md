---
title: "DDMVSS Documentation Scaffold"
audience: [architects, documentation maintainers, agents]
last_updated: 2026-05-25
version: "2.0.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# DDMVSS Documentation Scaffold

**Purpose:** Maps the DDMVSS 9-category goal-group taxonomy to directory locations and enforces the lifecycle policy.

**DDMVSS Reference:** [`architecture/DDMVSS.md`](architecture/DDMVSS.md) §3

---

## 1. DDMVSS Category → Document Mapping

| # | DDMVSS Category | Authoritative Document | Supporting References |
|---|----------------|----------------------|----------------------|
| 1 | **Domain** | [`domain-and-capability.md`](architecture/domain-and-capability.md) | [`reference/hKask-hLexicon.md`](architecture/reference/hKask-hLexicon.md), [`reference/hKask-Curator-persona.md`](architecture/reference/hKask-Curator-persona.md) |
| 2 | **Capability** | [`domain-and-capability.md`](architecture/domain-and-capability.md) | [`reference/ports-inventory.md`](architecture/reference/ports-inventory.md) |
| 3 | **Interface** | [`interface-and-composition.md`](architecture/interface-and-composition.md) | [`reference/utoipa-implementation.md`](architecture/reference/utoipa-implementation.md), [`reference/ports-inventory.md`](architecture/reference/ports-inventory.md) |
| 4 | **Composition** | [`interface-and-composition.md`](architecture/interface-and-composition.md) | [`reference/template-header-standard.md`](architecture/reference/template-header-standard.md) |
| 5 | **Trust & Security** | [`trust-security-observability.md`](architecture/trust-security-observability.md) | [`magna-carta.md`](architecture/magna-carta.md) |
| 6 | **Observability** | [`trust-security-observability.md`](architecture/trust-security-observability.md) | — |
| 7 | **Persistence** | [`persistence-and-lifecycle.md`](architecture/persistence-and-lifecycle.md) | [`reference/hKask-erd.md`](architecture/reference/hKask-erd.md), [`reference/registry-erd.md`](architecture/reference/registry-erd.md), [`reference/subsystem-erds.md`](architecture/reference/subsystem-erds.md) |
| 8 | **Lifecycle** | [`persistence-and-lifecycle.md`](architecture/persistence-and-lifecycle.md) | [`CI-CD-GUIDE.md`](CI-CD-GUIDE.md), [`DEPLOYMENT.md`](DEPLOYMENT.md) |
| 9 | **Curation** | [`DDMVSS.md`](architecture/DDMVSS.md) + [`WRITING_EXCELLENCE.md`](standards/WRITING_EXCELLENCE.md) | — |

---

## 2. Document Structure

```
docs/
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
├── specifications/
│   ├── REQUIREMENTS.md                    # Goal specs
│   ├── TRACEABILITY_MATRIX.md             # Code→test traceability
│   └── MODEL_CATALOG.md                   # LLM model catalog
├── standards/
│   ├── DOCUMENTATION_STANDARDS.md         # Documentation standards
│   ├── WRITING_EXCELLENCE.md              # Writing quality protocol
│   ├── DEPENDENCY_POLICY.md               # Dependency policy
│   └── ADR_TEMPLATE.md                    # ADR template
├── plans/
│   └── TODO.md                            # Open work
├── status/
│   └── PROJECT_STATUS.md                  # Single source of truth
├── user-guides/                           # User-facing guides
├── gml/                                   # GML (Allosteric Thinking)
├── DDMVSS_SCAFFOLD.md                     # THIS FILE
├── OPEN_QUESTIONS.md                      # Unresolved aspects
├── CI-CD-GUIDE.md                         # CI/CD guide
└── DEPLOYMENT.md                          # Deployment guide
```

---

## 3. Lifecycle Enforcement

Per [`DOCUMENTATION_STANDARDS.md`](standards/DOCUMENTATION_STANDARDS.md) §3:

```
Draft → Active → Deprecated → Superseded → Removed
```

- **Active** documents must map to ≥1 DDMVSS category via `ddmvss_categories` metadata
- **Deprecated/Superseded** documents moved to `docs/archive/YYYY-MM-DD-<label>/`
- **Removed** documents deleted; git history is archive of record
- `docs/archive/` is gitignored

---

## 4. Completeness Predicate

Per [`DDMVSS.md`](architecture/DDMVSS.md) §3.2:

| Category | Authoritative Document | Complete? | Curated? |
|----------|----------------------|-----------|----------|
| Domain | `domain-and-capability.md` | ✅ | ✅ Merge |
| Capability | `domain-and-capability.md` | ✅ | ✅ Merge |
| Interface | `interface-and-composition.md` | ✅ | ✅ Merge |
| Composition | `interface-and-composition.md` | ✅ | ✅ Merge |
| Trust & Security | `trust-security-observability.md` | ✅ | ✅ Merge |
| Observability | `trust-security-observability.md` | ✅ | ✅ Merge |
| Persistence | `persistence-and-lifecycle.md` | ✅ | ✅ Merge |
| Lifecycle | `persistence-and-lifecycle.md` | ✅ | ✅ Merge |
| Curation | `DDMVSS.md` + `WRITING_EXCELLENCE.md` | ✅ | ✅ Merge |

**Result:** 9/9 categories satisfied. Corpus is DDMVSS-complete.

---

## 5. Metadata Requirements

Every document under `docs/**` (excluding `archive/`) MUST include:

```yaml
---
title: "Document Title"
audience: [role list]
last_updated: YYYY-MM-DD
version: "MAJOR.MINOR.PATCH"
status: "Active | Draft | Deprecated | Superseded"
domain: "Cross-cutting | specific domain"
ddmvss_categories: [category1, category2, ...]
---
```

---

## 6. Verification Commands

| Gate | Command | Expected |
|------|---------|----------|
| Build | `cargo check --workspace` | Pass |
| Tests | `cargo test --workspace` | All pass |
| Lint | `cargo clippy --workspace -- -D warnings` | No warnings |
| Format | `cargo fmt --check` | No diffs |
| Links | `docs/ci/check-links.sh` | Zero broken (excluding intentional placeholders) |
| Metadata | `docs/ci/check-metadata.sh` | All headers present |

---

## References

[^ddmvss]: hKask Team. (2026). *DDMVSS — Domain-Driven Minimum Viable Specification Set*. `docs/architecture/DDMVSS.md`.
