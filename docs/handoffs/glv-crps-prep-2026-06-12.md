# Handoff — Gentle Lovelace Corpus Preparation

**Session date:** 2026-06-12
**Project:** hKask v0.27.0
**Handoff from:** Full documentation sweep (8 tasks) + Gentle Lovelace replica specification
**Handoff to:** Corpus download, format conversion, and OCR extraction — preparing 11 works for embedding

---

## 1. Session Context

This session executed a complete documentation corpus sweep (8 tasks per the `document-update` skill) and designed the **Gentle Lovelace** mashup replica — a composite embedding space that evaluates technical documentation against four dimensions of excellence, each grounded in a woman who shaped the field: Ada Lovelace (precision), Grace Hopper (accessibility), Karen Schriver (findability), and Anne Gentle (agent-correctness). The configuration layer is complete (manifest, works identified, weighting architecture, formal specification). The implementation layer — corpus preparation, embedding, centroid computation — is the next phase.

**Progress:** Configuration 100% complete. Corpus preparation 0% — this is the handoff.

---

## 2. What Was Done

### Documentation Sweep (Tasks 1-8)

- **47 documents classified** in `docs/status/corpus_inventory.yaml`
- **16 metadata violations fixed** — all documents now use 5-category `mds_categories`, zero `ddmvss_categories` in frontmatter
- **CI scripts updated** — `check-metadata.sh` checks `mds_categories`, `check-links.sh` excludes transient dirs
- **8 cross-reference violations fixed** — removed "Historical" section from architecture master, stripped `§7.1-7.2` references from 5 documents, removed 3 "formerly" annotations
- **14/14 spec-code drift items resolved** — 1 new stub (`CapabilityAwareValidator` in `hkask-templates`), 5 items discovered already-implemented, 5 old-spec references resolved as superseded
- **Portal refreshed** — `docs/README.md` now lists all 54 active documents, zero broken links (218 checked)
- **`document-update` skill composed** — `.agents/skills/document-update/SKILL.md` + `registry/templates/document-update/manifest.yaml` + 7 `.j2` templates
- **5 FUT-DOC items** added to `docs/OPEN_QUESTIONS.md` under new `## Document Automation` section
- **Writing quality report** — `docs/status/writing_quality_report.yaml` — 18 documents assessed, 17 at ≥3/4

### Gentle Lovelace Replica Design

- **Corpus manifest** — `registry/styles/gentle-lovelace/corpus.yaml` (11 works, 7 foundational rules, 4 orthogonal tag sets, weighted centroids)
- **Formal specification** — `docs/specifications/gentle-lovelace-specification.md` (343 lines, MDS-aligned, 5 Mermaid diagrams, 18 citations)
- **Weighting architecture**: Gentle 0.50 / Schriver 0.30 / Hopper 0.10 / Lovelace 0.10 — with per-document-type context sensitivity
- **Exemplar excerpts** — `registry/corpora/technical-documentation-exemplars/` (manifest + 3 annotated excerpt files)
- **FUT-DOC-6** registered in `OPEN_QUESTIONS.md` — semantic documentation embedding space

### Verification State

```
Metadata:  54 documents, 0 violations     ✅
Links:     218 links, 0 broken            ✅
ddmvss:    zero in frontmatter            ✅
formerly:  zero in active tree            ✅
Cargo:     hkask-templates compiles       ✅
Tests:     13 passed, 0 failed            ✅
```

---

## 3. What Remains

### HIGH — Corpus Preparation (this handoff)

The 11 works need to be downloaded/converted to plain text before the embedding pipeline can run. Current state:

| # | Work | Current Format | Target Format | Action Needed |
|---|------|---------------|---------------|---------------|
| 1 | Lovelace Notes | Gutenberg URL (txt) | Plain text | `curl` download → save to corpus dir |
| 2 | Hopper Mark I Manual | PDF (50MB, local) | Plain text | **OCR extraction** — 561 pages, 1946 typeset |
| 3 | Schriver Protocol-Aided Revision | PDF (2.5MB, downloaded) | Plain text | `markitdown` or `lighton` OCR |
| 4 | Schriver Document Design 1990 | PDF (1.3MB, downloaded) | Plain text | `markitdown` or `lighton` OCR |
| 5 | Schriver Info Design 2013 | PDF (6.1MB, downloaded) | Plain text | `markitdown` or `lighton` OCR |
| 6 | Write the Docs Guide | Web (writethedocs.org) | Plain text | `web_extract` → markdown |
| 7 | Gentle OpenStack Contributor Guide | Web (docs.openstack.org) | Plain text | `web_extract` → markdown |
| 8 | Gentle OpenStack API Guidelines | Web (docs.openstack.org) | Plain text | `web_extract` → markdown |
| 9 | Gentle OpenStack Writing Style | Web (docs.openstack.org) | Plain text | `web_extract` → markdown |
| 10 | Gentle Docs Like Code | Web (docslikecode.com) | Plain text | `web_extract` → markdown |
| 11 | Microsoft Style Guide welcome | Web (learn.microsoft.com) | Plain text | `web_extract` → markdown |

**Key files and locations:**

```
registry/styles/gentle-lovelace/corpus.yaml          ← The manifest (defines all works, URLs, paths)
registry/corpora/technical-documentation-exemplars/  ← Downloaded PDFs live here
  schriver-protocol-aided-revision.pdf               ← 2.5MB, already downloaded
  schriver-document-design-1990.pdf                  ← 1.3MB, already downloaded
  schriver-info-design-2013.pdf                      ← 6.1MB, already downloaded
/home/mdz-axolotl/Clones/Library/MarkI_operMan_1946.pdf ← 50MB Hopper PDF
```

**Recommended approach:**

1. **Web sources first** (fastest) — use `hkask-mcp-research` `web_extract` tool or direct `curl` + HTML→markdown conversion for the 5 web sources. These are plain HTML, no OCR needed.

2. **Lovelace Gutenberg** — `curl` the plain text URL from the manifest. Trivial.

3. **Schriver PDFs** — try `hkask-mcp-markitdown` first (it handles PDFs). If markitdown fails on the academic PDF formatting, fall back to `lighton` OCR via the `dynamic_space` tool.

4. **Hopper PDF** — this is the heavy lift. 561 pages, 1946 typeset with mechanical diagrams, operation code tables, and mathematical notation. `markitdown` may struggle with the vintage formatting. `lighton` OCR via Hugging Face Spaces is the likely path. This will take the most time.

**Output target:** One plain text file per work in `registry/corpora/technical-documentation-exemplars/`, named by slug:
```
lovelace-notes.txt
hopper-mark1-manual.txt
schriver-protocol-revision.txt
schriver-document-design-1990.txt
schriver-info-design-2013.txt
writethedocs-guide.txt
gentle-openstack-contrib-guide.txt
gentle-openstack-api-guidelines.txt
gentle-openstack-writing-style.txt
gentle-docs-like-code.txt
microsoft-style-guide-welcome.txt
```

### MEDIUM — Embedding Pipeline Extension

After corpus preparation, the `EmbedService` needs struct extensions to support the new manifest fields. Files to modify:

- `crates/hkask-services/src/embed.rs` — extend `CorpusConfig` (add `dimension_centroids`, `tag_sets`, `tag_weights`), extend `Work` (add `local_path`, `format`, `type`, `dimensions`, `section_types`, `mds_categories`), extend `FoundationalRule` (add `dimensions`, `section_type`)
- `crates/hkask-services/src/embed.rs` — add PDF ingestion path (detect `format: pdf`, route to markitdown or OCR), add web ingestion path (detect `format: web`, route to `web_extract`)
- `crates/hkask-services/src/embed.rs` — add per-dimension centroid computation (currently computes one centroid; needs to compute 4 + weighted composite)

### LOW — Integration

After embedding:
- Register Gentle Lovelace in replica registry (manifest + templates)
- Wire `replica_compare` with document type parameter for weighted comparison
- Extend `spec/require/writing-quality` with `replica_persona` parameter
- Add 5th quality dimension to `document-update` Task 3

---

## 4. Recommended Skills and Tools

For the next session, activate these skills in order:

| Order | Skill | Why |
|-------|-------|-----|
| 1 | **condenser-continuation** | Restore session state from this handoff |
| 2 | **coding-guidelines** | Surgical changes only — touch only corpus files, don't refactor adjacent code |
| 3 | **essentialist** | Keep the corpus preparation minimal — extract text, don't over-process |

**Key commands:**
```bash
# Verify nothing broke since handoff
cargo check -p hkask-templates
bash docs/ci/check-links.sh
bash docs/ci/check-metadata.sh

# Download Lovelace Gutenberg text
curl -o registry/corpora/technical-documentation-exemplars/lovelace-notes.txt \
  "https://www.gutenberg.org/cache/epub/75107/pg75107.txt"

# Test markitdown on a Schriver PDF
# (via hkask-mcp-markitdown if server is running)

# OCR the Hopper PDF
# (via lighton Hugging Face Space: dynamic_space tool)
```

**Infrastructure available:**
- `hkask-mcp-markitdown` — PDF → markdown conversion (try first for Schriver PDFs)
- `hkask-mcp-research` `web_extract` — web page → markdown (for OpenStack, Docs Like Code, Write the Docs, Microsoft)
- `dynamic_space` tool — Hugging Face Spaces including OCR (lighton)
- `firecrawl_scrape` — alternative web extraction if `web_extract` fails

---

## 5. Key Decisions to Preserve

1. **Gentle dominates at 50%.** Rationale: in an agent-native system, markdown specifications ARE the code. Stale documentation is a functional defect, not a quality issue. Gentle anticipated this in 2017. Do not rebalance without understanding this rationale.

2. **All four exemplars are women — this is not incidental.** The specification explicitly credits this lineage. Technical documentation was founded (Hopper), algorithmized (Lovelace), measured (Schriver), and modernized (Gentle) by women. Any future work on this replica should preserve this credit.

3. **Orthogonal tag sets are the generalization hook.** The 4-axis tagging (section_type × mds_category × document_type × dimension) is designed to be reusable for future replicas. Don't collapse them into a single taxonomy.

4. **Per-dimension centroids, not just composite.** The real diagnostic power is in per-dimension distances ("this doc is 0.3 from Hopper but 0.7 from Lovelace"). The composite is a summary; the dimensions are the landscape.

5. **Corpus preparation is surgical.** Extract text, don't edit it. The embedding pipeline needs the original prose, not cleaned-up versions. OCR errors are part of the authentic corpus — the centroid should reflect the actual source material.

6. **The `document-update` skill is the integration point.** Gentle Lovelace feeds into Task 3 (Writing Quality Gate) as a 5th dimension. It also feeds into `spec/require/writing-quality` via a `replica_persona` parameter. Don't create a separate evaluation path.

7. **The Hopper PDF is the bottleneck.** 561 pages of 1946 typeset with mechanical diagrams. This will take the most time. Start it first, let it run while processing the web sources.

---

## Broader Context

This handoff is part of a larger arc:

```
Documentation Sweep (complete)
  └── 8 tasks: inventory, metadata, quality, coherence, drift, archive, portal, skill
        │
Gentle Lovelace Design (complete)
  ├── corpus.yaml manifest
  ├── formal specification (gentle-lovelace-specification.md)
  └── exemplar excerpts
        │
Corpus Preparation ← THIS HANDOFF
  └── Download + OCR 11 works → plain text
        │
Embedding Pipeline (next)
  ├── Extend EmbedService structs
  ├── Run embed_corpus()
  └── Compute 4 centroids (gentle, schriver, hopper, lovelace)
  └── Composite = weighted mean of the 4
        │
Integration (future)
  ├── Register in replica registry
  ├── Wire into document-update Task 3
  └── Wire into spec/require/writing-quality
```

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
