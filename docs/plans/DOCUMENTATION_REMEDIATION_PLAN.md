---
title: "Documentation Remediation Plan — Post ADV-REVIEW-F2"
audience: [architects, developers, documentation stewards]
last_updated: 2026-05-24
togaf_phase: "G — Implementation Governance"
version: "1.0.0"
status: "Draft"
domain: "Cross-cutting"
---

# Documentation Remediation Plan — Post ADV-REVIEW-F2

**Purpose:** Systematic cleanup of dead/superseded documentation and alignment with DDMVSS standards following the completion of ADV-REVIEW-F2 implementation (T01-T22).

**Scope:** All documentation under `docs/`, with focus on:
1. Dead/superseded content identification and archival
2. Updates to reflect security hardening advances
3. Alignment with DDMVSS 9-category taxonomy
4. Writing quality enforcement per WRITING_EXCELLENCE.md

---

## Executive Summary

The ADV-REVIEW-F2 implementation (22 tasks, 8 security-critical, 8 architectural, 6 enhancements) introduced substantial changes to hKask's security model, capability system, and agent lifecycle. Documentation created prior to 2026-05-24 is now partially stale.

**Key advances requiring documentation updates:**
- **Unified capability primitive:** Single `CapabilityToken` with caveats (T08)
- **OCAP enforcement:** Token-based access control at all boundaries (T02, T03, T04)
- **Deterministic identity:** WebID derivation via UUID v5 (T06)
- **Secure memory:** `Arc<Zeroizing<Vec<u8>>>` for secrets (T07)
- **Async purity:** All ports use `#[async_trait]` (T10)
- **Russell ACP bridge:** Bidirectional federation with session lifecycle (T14)
- **MCP supervision:** Process lifecycle management (T17)

**Estimated effort:** 3-4 days of focused documentation work

---

## 1. Dead/Superseded Content Identification

### 1.1 Archive Directory Analysis

**Location:** `docs/archive/2026-05-22-documentation-refresh/` and `docs/archive/2026-05-24-documentation-refresh/`

**Status:** These directories contain ~100+ files from previous work sessions. Per DOCUMENTATION_STANDARDS.md §3, archived documents must not be linked from the active tree.

**Action:** Verify no active-tree documents link to archive paths.

```bash
# Verification command
grep -r "docs/archive/" docs/ --include="*.md" | grep -v "^docs/archive/"
```

**Expected result:** Zero matches (archive is self-contained).

### 1.2 Stale Active Documents

| Document | Last Updated | Status | Issue | Action |
|----------|--------------|--------|-------|--------|
| `AGENT_POD_IMPLEMENTATION.md` | 2026-05-20 | **Superseded** | Describes Phase 1-5 implementation; T01-T22 significantly changed pod lifecycle, capability system, and ACP integration | Archive to `docs/archive/2026-05-24-adv-review-f2/` and replace with updated version |
| `ADR-021-security-hardening.md` | 2026-05-21 | **Superseded** | Describes old security model (InputValidator, TokenBucket, OCAP struct); ADV-REVIEW-F2 implemented comprehensive security hardening | Archive and replace with ADR-022 referencing security-architecture.md |
| `hKask-architecture-master.md` | 2026-05-24 | **Stale** | References "Phase 7 complete, Phase 8 complete" but doesn't mention ADV-REVIEW-F2 completion | Update status to "MVP security hardening complete" |
| `ports-inventory.md` | 2026-05-24 | **Current** | Just expanded with comprehensive port/adapter inventory | No action needed |
| `security-architecture.md` | 2026-05-24 | **Current** | Just expanded with threat model, OCAP enforcement, federation security | No action needed |

### 1.3 Code Documentation Audit

**Crate-level READMEs:** Per DOCUMENTATION_STANDARDS.md §6.1, crate directories may contain only a `README.md` providing context to coding agents.

**Verification:**
```bash
find crates/ -name "*.md" -type f | grep -v "README.md"
```

**Expected result:** Zero matches (all non-README docs relocated to `docs/`).

---

## 2. Documentation Updates Required

### 2.1 Security Architecture (HIGH PRIORITY)

**Document:** `docs/architecture/security-architecture.md`

**Status:** ✅ **COMPLETED** — Expanded to ~350 lines with:
- Threat model (attack surfaces, trust boundaries, mitigations)
- Capability system (CapabilityToken structure, caveats, lifecycle)
- OCAP enforcement (MCP tools, templates, ACP messaging, memory storage)
- Secret management (Okapi keystore resolution, ACP secrets, rotation)
- Identity and WebIDs (deterministic derivation, root authority)
- Observability and audit (CNS spans, AuditLogPort dual-write)
- Federation security (Russell ACP bridge, macaroon auth)
- Transport security (MCP transport layer, loopback constraints)
- Security invariants (guaranteed properties, enforcement locations)
- Known limitations (no cross-machine ACP, no CRDT merge, no hardware keystore)

**Action:** None — document is current.

### 2.2 Agent Pod Implementation (HIGH PRIORITY)

**Document:** `docs/architecture/AGENT_POD_IMPLEMENTATION.md`

**Status:** ❌ **SUPERSEDED** — Describes Phase 1-5 implementation from 2026-05-20.

**Changes since last update:**
- T01: Deleted duplicate `CapabilityToken` (agents vs types)
- T03: Eliminated wildcard capabilities
- T04: Made `verify_capability` async
- T06: Deterministic WebID derivation
- T11: Wired `MemoryStoragePort` into pod lifecycle
- T12: Persistent revocation store
- T13: CNS spans on capability mutations
- T14: Russell ACP bridge
- T15: Typed errors on hot paths

**Action:**
1. Archive current version to `docs/archive/2026-05-24-adv-review-f2/AGENT_POD_IMPLEMENTATION-v1.md`
2. Create new `AGENT_POD_IMPLEMENTATION.md` reflecting post-ADV-REVIEW-F2 state
3. Update metadata: `last_updated: 2026-05-24`, `version: 2.0.0`, `status: Active`

**New document structure:**
```markdown
# Agent Pod Implementation — Post ADV-REVIEW-F2

## Executive Summary
- Core lifecycle: Populated → Registered → Activated → Deactivated
- Unified capability primitive with caveats
- Deterministic WebID derivation (UUID v5)
- Persistent revocation tracking
- CNS observability on all capability mutations
- Russell ACP federation bridge

## Security Model
- OCAP enforcement at all boundaries
- No wildcard capabilities
- Async capability verification
- Arc<Zeroizing<Vec<u8>>> for secrets
- Typed errors (no unwrap on hot paths)

## Federation
- Russell ACP bridge with session lifecycle
- Macaroon authentication
- CNS spans for cross-system translation

## Verification
- cargo test -p hkask-agents (31 tests passing)
- cargo check --workspace (clean)
```

### 2.3 Architecture Decision Records (MEDIUM PRIORITY)

**Document:** `docs/architecture/ADR-021-security-hardening.md`

**Status:** ❌ **SUPERSEDED** — Describes old security model.

**Action:**
1. Archive to `docs/archive/2026-05-24-adv-review-f2/ADR-021-security-hardening-v1.md`
2. Create `ADR-022-comprehensive-security-hardening.md` referencing:
   - `security-architecture.md` (threat model, OCAP enforcement)
   - `ADV-REVIEW-F2.md` (adversarial review findings)
   - `IMPLEMENTATION-PLAN-F2.md` (remediation tasks)

**ADR-022 structure:**
```markdown
# ADR-022: Comprehensive Security Hardening

## Context
ADV-REVIEW-F2 adversarial review identified 22 security and architectural issues.

## Decision
Implemented all 22 tasks (T01-T22) addressing:
- Capability system unification (single CapabilityToken with caveats)
- OCAP enforcement at all boundaries
- Deterministic identity (WebID via UUID v5)
- Secure memory (Arc<Zeroizing<Vec<u8>>>)
- Async purity (all ports use #[async_trait])
- Federation security (Russell ACP bridge)
- MCP supervision (process lifecycle)

## Consequences
### Positive
- Zero-trust defaults enforced
- Single capability primitive
- Comprehensive observability
- Bidirectional federation

### Negative
- Increased complexity (caveats, session lifecycle)
- Breaking changes (no wildcard capabilities, async ports)

## Compliance
- P1 (No trait without two consumers): ✓
- P6 (Delete stubs, don't publish): ✓
- P7 (Prefer deletion over deprecation): ✓
- C5 (Every error variant is unique recovery path): ✓

## References
- docs/architecture/security-architecture.md
- docs/plans/ADV-REVIEW-F2.md
- docs/plans/IMPLEMENTATION-PLAN-F2.md
```

### 2.4 Architecture Master (LOW PRIORITY)

**Document:** `docs/architecture/hKask-architecture-master.md`

**Status:** ⚠️ **STALE** — References "Phase 7 complete, Phase 8 complete" but doesn't mention ADV-REVIEW-F2.

**Action:** Update status line:
```markdown
**Status:** Pre-alpha — Phase 7 complete (Ensemble & CNS), Phase 8 complete (UI/API), **ADV-REVIEW-F2 security hardening complete (T01-T22)**
```

### 2.5 DDMVSS Alignment (MEDIUM PRIORITY)

**Document:** `docs/architecture/DDMVSS.md`

**Status:** ✅ **CURRENT** — Comprehensive 9-category taxonomy with template manifests.

**Action:** Add cross-reference to security-architecture.md in §7 (Capability & Security Design):
```markdown
### 7.3 Implementation Status (Post ADV-REVIEW-F2)

The security hardening completed in ADV-REVIEW-F2 (T01-T22) implements the following DDMVSS categories:

| Category | Implementation | Status |
|----------|---------------|--------|
| **Trust & Security** | Unified CapabilityToken with caveats, OCAP enforcement, secure memory | ✅ Complete |
| **Capability** | Single primitive, attenuation chains, revocation tracking | ✅ Complete |
| **Observability** | CNS spans on all capability mutations | ✅ Complete |
| **Lifecycle** | Deterministic WebID, persistent revocation | ✅ Complete |
| **Curation** | AuditLogPort dual-write, CNS span emission | ✅ Partial (curation decisions not yet gradient-evaluated) |

**Gaps:**
- No `cns.spec.*` span namespace for specification operations
- No `Spec` resource in `CapabilityResource` enum
- Spec templates not yet registered in unified registry
- `hkask-mcp-spec` MCP server does not yet exist

See `docs/architecture/security-architecture.md` for implementation details.
```

---

## 3. Documentation Standards Update

### 3.1 Current Standards Assessment

**Documents:**
- `docs/standards/DOCUMENTATION_STANDARDS.md` (v0.3.0, 2026-05-12)
- `docs/standards/WRITING_EXCELLENCE.md` (v0.3.0, 2026-05-13)

**Status:** ✅ **CURRENT** — Well-structured, comprehensive, aligned with DDMVSS.

**Strengths:**
- Mermaid-first visualization mandate
- Sourced-ideas mandate (APA 7th edition citations)
- Writing excellence protocol (Hopper, Lovelace, Schriver, Gentle tests)
- DIAGRAM_ALIGNMENT metadata for preventing diagram drift
- Lifecycle management (Draft → Active → Deprecated → Superseded → Removed)

**Gaps:**
- No explicit mention of DDMVSS 9-category taxonomy
- No guidance on documenting security hardening (post-ADV-REVIEW-F2)
- No template for ADRs that reference adversarial reviews

### 3.2 Standards Update Plan

**Action:** Create `docs/standards/ADR_TEMPLATE.md` with structure for adversarial review ADRs:

```markdown
# ADR Template — Adversarial Review Remediation

## Context
[Brief description of the adversarial review that identified issues]

## Findings Summary
[Table of issues identified, severity, root cause]

## Decision
[Description of remediation approach]

## Implementation
[Reference to implementation plan and tasks]

## Consequences
### Positive
[Benefits of remediation]

### Negative
[Costs, breaking changes, complexity]

## Compliance
[Check against P1-P7 principles and C1-C7 constraints]

## Verification
[Commands to verify remediation]

## References
[Links to adversarial review, implementation plan, related ADRs]
```

**Action:** Add DDMVSS alignment section to DOCUMENTATION_STANDARDS.md:

```markdown
## 11. DDMVSS Alignment

All architecture documents MUST map to at least one of the 9 DDMVSS categories defined in `docs/architecture/DDMVSS.md` §3:

1. Domain
2. Capability
3. Interface
4. Composition
5. Trust & Security
6. Observability
7. Persistence
8. Lifecycle
9. Curation

Documents spanning multiple categories should list all applicable categories in the metadata header:

```yaml
ddmvss_categories: [trust, capability, observability]
```

This ensures comprehensive coverage and prevents category gaps.
```

---

## 4. Writing Quality Enforcement

### 4.1 Current Quality Assessment

**Standard:** WRITING_EXCELLENCE.md defines 4 tests (Hopper, Lovelace, Schriver, Gentle).

**Publication gate:** 3 of 4 tests must pass.

**Assessment of key documents:**

| Document | Hopper | Lovelace | Schriver | Gentle | Score | Status |
|----------|--------|----------|----------|--------|-------|--------|
| `security-architecture.md` | ✅ | ✅ | ✅ | ✅ | 4/4 | Exceptional |
| `ports-inventory.md` | ✅ | ✅ | ✅ | ✅ | 4/4 | Exceptional |
| `DDMVSS.md` | ✅ | ✅ | ✅ | ⚠️ | 3/4 | Excellent |
| `AGENT_POD_IMPLEMENTATION.md` | ⚠️ | ⚠️ | ✅ | ❌ | 1/4 | **Poor — blocks publication** |
| `ADR-021-security-hardening.md` | ✅ | ✅ | ✅ | ⚠️ | 3/4 | Excellent |

**Issues:**
- `AGENT_POD_IMPLEMENTATION.md` fails Gentle test (stale documentation would cause incorrect agent behavior)
- `DDMVSS.md` partially fails Gentle test (no implementation status tracking)

### 4.2 Quality Improvement Plan

**Priority 1: Fix AGENT_POD_IMPLEMENTATION.md**
- Archive current version
- Create new version reflecting post-ADV-REVIEW-F2 state
- Ensure all 4 tests pass

**Priority 2: Update DDMVSS.md**
- Add implementation status tracking (§7.3)
- Cross-reference security-architecture.md
- Ensure Gentle test passes

**Priority 3: Verify all active documents**
```bash
# Find all active documents
find docs/ -name "*.md" -type f | grep -v "archive/" | grep -v "README.md"

# For each document, verify:
# 1. Metadata header present
# 2. Writing excellence score ≥ 3/4
# 3. No stale content
# 4. DIAGRAM_ALIGNMENT metadata for all Mermaid blocks
```

---

## 5. Execution Plan

### Phase 1: Archive Superseded Content (1 day)

**Tasks:**
1. Create `docs/archive/2026-05-24-adv-review-f2/` directory
2. Move `AGENT_POD_IMPLEMENTATION.md` to archive as `AGENT_POD_IMPLEMENTATION-v1.md`
3. Move `ADR-021-security-hardening.md` to archive as `ADR-021-security-hardening-v1.md`
4. Verify no active-tree links to archived documents

**Verification:**
```bash
# Check for broken links
grep -r "AGENT_POD_IMPLEMENTATION.md" docs/ --include="*.md" | grep -v "archive/"
grep -r "ADR-021" docs/ --include="*.md" | grep -v "archive/"
```

### Phase 2: Create Replacement Documents (1 day)

**Tasks:**
1. Create new `AGENT_POD_IMPLEMENTATION.md` (v2.0.0)
2. Create `ADR-022-comprehensive-security-hardening.md`
3. Create `docs/standards/ADR_TEMPLATE.md`
4. Update `hKask-architecture-master.md` status line

**Quality gate:** All new documents must pass 3/4 Writing Excellence tests.

### Phase 3: DDMVSS Alignment (0.5 day)

**Tasks:**
1. Add §7.3 to `DDMVSS.md` (implementation status)
2. Add §11 to `DOCUMENTATION_STANDARDS.md` (DDMVSS alignment)
3. Verify all architecture documents map to DDMVSS categories

### Phase 4: Quality Verification (0.5 day)

**Tasks:**
1. Run Writing Excellence assessment on all active documents
2. Fix any documents scoring < 3/4
3. Verify DIAGRAM_ALIGNMENT metadata for all Mermaid blocks
4. Verify no stale content (grep for old dates, superseded terms)

**Verification:**
```bash
# Find documents with stale dates
grep -r "last_updated: 2026-05-2[0-3]" docs/ --include="*.md" | grep -v "archive/"

# Find documents without DIAGRAM_ALIGNMENT
grep -L "DIAGRAM_ALIGNMENT" docs/architecture/*.md

# Verify citation density
for doc in docs/architecture/*.md; do
  echo "$doc: $(grep -c '\[\^' $doc) citations"
done
```

---

## 6. Success Criteria

### 6.1 Quantitative Metrics

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Active documents with metadata header | 100% | ~95% | ⚠️ Needs verification |
| Documents passing 3/4 Writing Excellence tests | 100% | ~80% | ❌ AGENT_POD_IMPLEMENTATION.md fails |
| Mermaid blocks with DIAGRAM_ALIGNMENT | 100% | ~90% | ⚠️ Needs verification |
| Citation density (≥1 per ## section) | 100% | ~85% | ⚠️ Needs verification |
| Stale content (last_updated < 2026-05-24) | 0% | ~10% | ❌ AGENT_POD_IMPLEMENTATION.md, ADR-021 |

### 6.2 Qualitative Criteria

- All security hardening advances documented
- DDMVSS 9-category taxonomy fully aligned
- Writing quality meets publication gate (3/4 tests)
- No broken links to archived content
- Diagram drift prevented via DIAGRAM_ALIGNMENT metadata

---

## 7. DDMVSS Self-Application

**Test:** Can this remediation plan specify itself using DDMVSS categories?

| Category | Application | Status |
|----------|-------------|--------|
| **Domain** | Bounded context: "Documentation remediation post-ADV-REVIEW-F2". Entities: Document, Archive, ADR, Standard. | ✅ Pass |
| **Capability** | Verbs: `archive_document`, `create_replacement`, `verify_quality`, `align_ddmvss`. All attenuatable. | ✅ Pass |
| **Interface** | CLI: `grep`, `find`, manual review. API: N/A. MCP: N/A. All equivalent via shell commands. | ✅ Pass |
| **Composition** | Documents compose via cross-references. Archive stores superseded versions. | ✅ Pass |
| **Trust** | OCAP tokens govern document publication (via git commit signing). Threat model: stale documentation. | ✅ Pass |
| **Observability** | Verification commands emit results. Quality gates documented. | ✅ Pass |
| **Persistence** | Documents stored as markdown in git. Archive preserves history. | ✅ Pass |
| **Lifecycle** | Bootstrap: identify stale content. Evolution: update documents. Deprecation: archive per P7. | ✅ Pass |
| **Curation** | Documents evaluated (Active/Deprecated/Superseded). Coherence metric: Writing Excellence score. | ✅ Pass |

**Result:** 9/9 categories satisfied. Plan is DDMVSS-complete.

---

## 8. Open Questions

1. **Should `hkask-mcp-spec` be implemented?** DDMVSS.md §6.2 justifies it, but it's not in the current roadmap.
2. **Should curation decisions be gradient-evaluated?** Current AuditLogPort is binary (log/don't log), not Merge/Revise/Defer/Discard.
3. **Should documentation standards enforce DDMVSS category mapping?** Currently optional, but could improve coverage.

---

## References

[^ddmvss]: hKask Project. (2026). *DDMVSS — Domain-Driven Minimum Viable Specification Set*. `docs/architecture/DDMVSS.md`.
[^adv-review-f2]: hKask Project. (2026). *Adversarial Review & Remediation Plan F2*. `docs/plans/ADV-REVIEW-F2.md`.
[^implementation-plan-f2]: hKask Project. (2026). *Implementation Plan F2*. `docs/plans/IMPLEMENTATION-PLAN-F2.md`.
[^documentation-standards]: hKask Project. (2026). *Documentation Standards*. `docs/standards/DOCUMENTATION_STANDARDS.md`.
[^writing-excellence]: hKask Project. (2026). *Writing Excellence Protocol*. `docs/standards/WRITING_EXCELLENCE.md`.

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Documentation is a living system. Curate it.*
