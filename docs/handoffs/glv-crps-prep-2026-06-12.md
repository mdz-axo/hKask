# Handoff — Gentle Lovelace Corpus Preparation

**Session date:** 2026-06-12
**Project:** hKask v0.27.0
**Handoff from:** Full documentation sweep (8 tasks) + Gentle Lovelace replica design
**Handoff to:** Corpus download, format conversion, OCR extraction — 11 works → plain text for embedding

---

## 1. Session Context

Executed a complete documentation corpus sweep per the `document-update` skill and designed the **Gentle Lovelace** mashup replica — a composite embedding space evaluating technical documentation against four dimensions of excellence: Ada Lovelace (precision), Grace Hopper (accessibility), Karen Schriver (findability), Anne Gentle (agent-correctness). Configuration layer complete. Implementation layer — corpus preparation, embedding, centroid computation — is next.

**Progress:** Configuration 100%. Corpus preparation 0%.

---

## 2. What Was Done

### Documentation Sweep

- 47 documents classified in `docs/status/corpus_inventory.yaml`
- 16 metadata violations fixed — all use 5-category `mds_categories`, zero `ddmvss_categories`
- CI scripts updated — `check-metadata.sh` checks `mds_categories`, flags deprecated `ddmvss_categories`
- 8 cross-reference violations fixed — removed Historical section from architecture master, stripped stale `§7.x` refs from 8 documents, removed 3 "formerly" annotations, fixed "6 tools"→"5 tools" in 3 locations
- 14/14 spec-code drift items resolved — new `CapabilityAwareValidator` stub in `hkask-templates`, 5 items discovered already-implemented, 5 old-spec refs resolved as superseded
- Portal refreshed — `docs/README.md` lists all 54 active documents
- `document-update` skill composed — SKILL.md + manifest.yaml + 7 `.j2` templates
- 6 FUT-DOC items in `OPEN_QUESTIONS.md` under `## Document Automation`
- `docs/status/writing_quality_report.yaml` — 18 documents assessed, 17 at ≥3/4

### Gentle Lovelace Replica

- **Corpus manifest:** `registry/styles/gentle-lovelace/corpus.yaml` — 339 lines, 11 works, 7 foundational rules, 4 orthogonal tag sets
- **Formal spec:** `docs/specifications/gentle-lovelace-specification.md` — 343 lines, 5 Mermaid diagrams, 18 citations
- **Weighting:** Gentle 0.50 / Schriver 0.30 / Hopper 0.10 / Lovelace 0.10 — per-document-type context sensitivity
- **Exemplar excerpts:** `registry/corpora/technical-documentation-exemplars/` — manifest + 3 annotated excerpts + 3 Schriver PDFs

### Verification

```
Metadata:  54 documents, 0 violations     ✅
Links:     224 links, 0 broken            ✅
ddmvss:    zero in frontmatter            ✅
formerly:  zero in active tree            ✅
§7.x refs: zero in active docs            ✅
Cargo:     hkask-templates compiles       ✅
Tests:     13 passed, 0 failed            ✅
```

---

## 3. What Remains

### HIGH — Corpus Preparation

11 works → plain text before embedding can run:

| # | Work | Source | Format | Action |
|---|------|--------|--------|--------|
| 1 | Lovelace Notes | Gutenberg | txt URL | `curl` download |
| 2 | Hopper Mark I Manual | Local PDF (50MB) | PDF | **OCR — 561 pages, 1946 typeset** |
| 3 | Schriver Protocol-Aided Revision | Downloaded (2.5MB) | PDF | markitdown or OCR |
| 4 | Schriver Document Design 1990 | Downloaded (1.3MB) | PDF | markitdown or OCR |
| 5 | Schriver Info Design 2013 | Downloaded (6.1MB) | PDF | markitdown or OCR |
| 6 | Write the Docs Guide | writethedocs.org | web | web_extract → markdown |
| 7 | OpenStack Contributor Guide | docs.openstack.org | web | web_extract → markdown |
| 8 | OpenStack API Guidelines | docs.openstack.org | web | web_extract → markdown |
| 9 | OpenStack Writing Style | docs.openstack.org | web | web_extract → markdown |
| 10 | Docs Like Code | doclikecode.com | web | web_extract → markdown |
| 11 | Microsoft Style Guide | learn.microsoft.com | web | web_extract → markdown |

**Key paths:**
```
registry/styles/gentle-lovelace/corpus.yaml          ← manifest (all URLs/paths)
registry/corpora/technical-documentation-exemplars/  ← downloaded files here
/home/mdz-axolotl/Clones/Library/MarkI_operMan_1946.pdf ← Hopper PDF
```

**Output:** One `.txt` per work, named by slug:
```
lovelace-notes.txt, hopper-mark1-manual.txt,
schriver-protocol-revision.txt, schriver-document-design-1990.txt,
schriver-info-design-2013.txt, writethedocs-guide.txt,
gentle-openstack-contrib-guide.txt, gentle-openstack-api-guidelines.txt,
gentle-openstack-writing-style.txt, gentle-docs-like-code.txt,
microsoft-style-guide-welcome.txt
```

**Order of attack:**
1. Web sources first (fastest) — web_extract or curl
2. Lovelace Gutenberg — trivial curl
3. Schriver PDFs — try markitdown, fall back to lighton OCR
4. Hopper PDF — start first, let run; 561 pages, 1946 typeset with diagrams

**Available tools:** `hkask-mcp-markitdown` (PDF→text), `hkask-mcp-research web_extract` (web→markdown), `dynamic_space` (lighton OCR), `firecrawl_scrape` (web fallback)

### MEDIUM — Embedding Pipeline Extension

Extend `crates/hkask-services/src/embed.rs`:
- `CorpusConfig`: add `dimension_centroids`, `tag_sets`, `tag_weights`
- `Work`: add `local_path`, `format`, `type`, `dimensions`, `section_types`, `mds_categories`
- `FoundationalRule`: add `dimensions`, `section_type`
- Add PDF ingestion path, web ingestion path
- Add per-dimension centroid computation (4 centroids + weighted composite derived at query time)

### LOW — Integration

- Register Gentle Lovelace in replica registry
- Wire `replica_compare` with document type parameter
- Extend `spec/require/writing-quality` with `replica_persona` param
- Add 5th quality dimension to `document-update` Task 3

---

## 4. Skills and Commands

Activate in order: **condenser-continuation** → **coding-guidelines** → **essentialist**

```bash
# Verify nothing broke
cargo check -p hkask-templates
bash docs/ci/check-links.sh
bash docs/ci/check-metadata.sh

# Download Lovelace
curl -o registry/corpora/technical-documentation-exemplars/lovelace-notes.txt \
  "https://www.gutenberg.org/cache/epub/75107/pg75107.txt"
```

---

## 5. Key Decisions

1. **Gentle dominates at 50%.** Docs ARE code in agent-native systems. Stale docs are functional defects. Do not rebalance.
2. **All four exemplars are women.** This lineage is credited in the formal spec. Preserve it.
3. **Orthogonal tag sets** (section_type × mds_category × document_type × dimension) are the generalization hook.
4. **4 centroids, not 5.** Composite is weighted mean of the 4, derived at query time.
5. **Extract text, don't edit it.** OCR errors are authentic corpus.
6. **`document-update` Task 3** is the integration point — not a separate path.
7. **Hopper PDF is the bottleneck.** Start it first.

---

## Broader Arc

```
Documentation Sweep (done) → Gentle Lovelace Design (done) → Corpus Prep ← NOW
  → Embedding Pipeline → Compute 4 Centroids → Integration (Task 3 + spec server)
```

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
