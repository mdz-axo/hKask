# TODO — Corpus Pipeline MCP Migration + Architecture Improvement

## Phase 0: Delete custom CLI code ✅ DONE

- [x] 0.1 Delete custom embed/classify/decimate code
- [x] 0.2 Remove minijinja dependency

## Phase 1: Fix existing MCP tools ✅ DONE

- [x] 1.1 Add `extract_json_from_response` thinking-mode utility
- [x] 1.2 Fix `docproc_tag_chunks` (model, thinking-mode, owner)
- [x] 1.3 Fix `docproc_embed` (DB storage)
- [x] 1.4 Fix `docproc_extract_triples` (DB, thinking-mode, retry, entity, template name)
- [x] 1.5 Fix `docproc_generate_qa` + batch (thinking-mode)
- [x] 1.6 Cache compiled templates

## Phase 2: Create missing MCP tools

- [ ] **2.1** Fix `docproc_build_prompts` build errors
  - [ ] Remove unused `webid` variable
  - [ ] Verify `query_deduped` compiles
- [ ] **2.2** Verify `docproc_ingest_qa` builds
- [ ] **2.3** Improve `build-prompts.j2` template quality
  - [ ] Add `<system>/<task>/<output_contract>` structure
  - [ ] Add examples
  - [ ] Add quality checklist
- [ ] **2.4** Clean up template manifest
  - [ ] Remove `classify-chunks.j2` from manifest
  - [ ] Add `build-prompts.j2` to manifest

**Checkpoint 2:** All pipeline steps have MCP tools, build succeeds, templates registered

## Phase 3: Architecture improvement

- [ ] **3.1** Move request structs to co-located tool modules
- [ ] **3.2** Fix template cache memory leak (check before leak)
- [ ] **3.3** Replace `println!`/`eprintln!` with `tracing` in library code
- [ ] **3.4** Consolidate owner defaults to single const
- [ ] **3.5** Extract shared helpers (`embedding_dim`, `normalize_in_place`, `owner_webid`)
- [ ] **3.6** Remove dead `corpus_salience` from replica server
- [ ] **3.7** Update replica corpus tools to call docproc MCP tools (no subprocess)
- [ ] **3.8** Remove pipeline logic from CLI binary (keep purge-qa, ocr only)

**Checkpoint 3:** Clean architecture, no dead code, no subprocess wrappers

## Phase 4: Update pipeline YAML + build

- [ ] **4.1** Rewrite pipeline YAML (docproc_* tools, approved models, owner)
- [ ] **4.2** Build and verify all components
- [ ] **4.3** Verify .env models
- [ ] **4.4** Delete ad-hoc scripts

**Checkpoint 4:** Everything builds, tests pass, pipeline YAML valid

## Phase 5: Run pipeline

- [ ] 5.1 Purge existing data (keep chunks.jsonl)
- [ ] 5.2 `docproc_embed` — 33K chunks → vectors + h_mems
- [ ] 5.3 `docproc_tag_chunks` — ontology tagging (Qwen3.6)
- [ ] 5.4 `docproc_extract_triples` — h_mem decimation (Qwen3.6)
- [ ] 5.5 `docproc_dedup_chunks`
- [ ] 5.6 `docproc_consolidate_chunks`
- [ ] 5.7 `docproc_build_prompts` — with h_mem knowledge graph
- [ ] 5.8 `docproc_generate_qa_batch` — GLM-5.2
- [ ] 5.9 `docproc_ingest_qa` — SemDeDup + h_mem storage

**Checkpoint 5:** Full pipeline complete through MCP tools

## Phase 6: Train

- [ ] 6.1 Convert to ChatML
- [ ] 6.2 Verify Axolotl config
- [ ] 6.3 Train on RunPod H100 with PiSSA

**Checkpoint 6:** LoRA adapter trained