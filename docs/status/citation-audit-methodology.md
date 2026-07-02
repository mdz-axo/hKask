---
title: "Citation Audit Methodology"
audience: [curator, contributors]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Curation"
mds_categories: [curation, lifecycle]
---

# Citation Audit Methodology

How to audit the documentation corpus for PS-07 ("Sourced Ideas") compliance and systematically close citation gaps.

## 1. The Rule

Per `DOCUMENTATION_STANDARDS.md` §5 (Sourced-Ideas Mandate):

> "No design choice, best practice, pattern, or process is treated as originating from the team. Every significant design choice, structure, abstraction, or convention must carry footnotes, source citations, and web links."

Enforcement: every `##`-level section SHOULD contain at least one `[^key]` footnote citation.

**Scope exemption** (§5.1): purely referential sections (indexes, table-of-contents, phase-alignment tables, internal index-of-index sections) are exempt. An exempt section SHOULD include a one-line annotation naming the authoritative sections it indexes.

## 2. Mechanical Audit

Run `docs/ci/check-citations.sh` to identify which documents and sections lack citations:

```bash
# Report only (non-failing)
bash docs/ci/check-citations.sh

# Verbose — shows which specific sections lack citations
bash docs/ci/check-citations.sh --verbose

# Strict — fails CI if any document has uncited sections
bash docs/ci/check-citations.sh --strict
```

The script counts `##`-level sections and `[^...]` footnote references per document, flagging sections that lack at least one citation. It produces a `PASS|GAP|EXEMPT` verdict per document.

## 3. Closing Gaps — Process

Closing a citation gap requires domain knowledge. The process per document:

### 3.1 Triage

For each GAP document, classify every uncited `##` section:

| Classification | Action |
|---------------|--------|
| **Scope-exempt** | Add a `<!-- scope-exempt: indexes §X.Y -->` annotation. No citation needed. |
| **Needs citation** | Identify the prior art the section's design choice draws from. |
| **Trivial convention** | The section describes a mechanical project convention (naming, directory layout). Mark `<!-- scope-exempt: project convention -->`. |

### 3.2 Source Assignment

For sections needing citations, assign sources using the preference order from §5.2:

1. Peer-reviewed publications, standards documents (RFCs, ISO, IETF, W3C)
2. Primary-source books and academic papers with DOIs or stable URLs
3. Reference implementations and specifications of named open-source projects
4. Official documentation sites of named open-source projects
5. Named authors' technical blog posts (for pattern-naming conventions)

### 3.3 Citation Format

```markdown
[^key]: Author, A. (Year). *Title* (edition). Publisher. https://url/path.
    Optional annotation explaining why this source is cited here.
```

Place all `[^...]:` definitions at the bottom of the document in a `## References` or `## Citations` section.

## 4. The 23-File Gap (PS-07, 2026-06-11)

The citation audit of 2026-06-11 identified 23 files with fewer footnotes than `##` sections:

| Gap Size | Files |
|----------|-------|
| 3 | `TESTING_STANDARDS.md` |
| 4 | `ADR-024`, `ADR-026`, `MDS.md` |
| 5 | `ADR-031`, `ADR-032`, `ADR-033`, `ADR-034` |
| 6 | `AGENTSERVICE-IMPLEMENTATION.md`, `MDS_SCAFFOLD.md` |
| 7 | `hKask-architecture-master.md`, `ADR_TEMPLATE.md`, `MDS-agent-service.md` |
| 8 | `refactoring-plan-services-2026-06-09.md` |
| 9 | `agatha-eliot-moe-plan.md`, `semantic-condensation-analysis.md` |
| 10 | `REQUIREMENTS.md`, `TRACEABILITY_MATRIX.md` |
| 11 | `CI-CD-GUIDE.md` |
| 12 | `test-program.md` |
| 13 | `DEPLOYMENT.md` |
| 23 | `REPL-specification.md` |

**Estimated effort:** 2–4 hours of domain-expert time per document for the large-gap files (>6 gaps). The small-gap files (3–5 gaps) are ~30 minutes each. Total: ~20–30 curator-hours to close all gaps.

### 4.1 Prioritized Attack Plan

| Wave | Documents | Gap Sum | Rationale |
|------|-----------|---------|-----------|
| **Wave 1** | `MDS.md`, `hKask-architecture-master.md`, `REQUIREMENTS.md`, `TRACEABILITY_MATRIX.md` | 31 | Core specs — highest impact |
| **Wave 2** | `ADR-024`, `ADR-026`, `ADR-031`, `ADR-032`, `ADR-033`, `ADR-034`, `ADR_TEMPLATE.md` | 36 | ADRs — each cites specific prior art |
| **Wave 3** | `REPL-specification.md` | 23 | Large file but many sections are scope-exempt |
| **Wave 4** | `CI-CD-GUIDE.md`, `DEPLOYMENT.md`, `test-program.md` | 36 | Operational docs — cite tooling docs |
| **Wave 5** | Remaining 8 small-gap documents | ~55 | Cleanup pass |

## 5. Automation Strategy

### 5.1 CI Gate (Recommended)

Add `check-citations.sh` to `.github/workflows/ci.yml` as a non-blocking warning step (not strict mode). This surfaces new gaps without blocking merges.

### 5.2 LLM-Assisted Citation Discovery (Experimental)

For large-gap documents, an LLM can suggest candidate citations per section by reading the section content and searching for prior art. The process:

1. Feed each uncited section to an LLM with the source preference order
2. LLM proposes `[^key]` citations with URLs
3. Human curator verifies each proposed citation (LLM may hallucinate URLs)
4. Verified citations are committed

This is an augmentation, not a replacement — human verification remains mandatory.

### 5.3 Progress Tracking

Track citation gap closure in `docs/status/citation-audit-progress.yaml`:

```yaml
- file: docs/architecture/core/MDS.md
  total_sections: 22
  uncited: 4
  wave: 1
  assigned_to: curator
  status: open
  last_checked: 2026-07-01
```

## 6. Verification

After closing gaps, verify:

```bash
# Mechanical check
bash docs/ci/check-citations.sh --strict

# Manual spot-check: are the citations actually relevant?
# Pick 3 random documents, read 2 [^...] citations each, verify:
#   - URL resolves (not a 404)
#   - Citation actually supports the claim it annotates
#   - Source class matches preference order
```
