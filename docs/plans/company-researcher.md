# Company Researcher — Corpus Pipeline & LoRA Adapter

**Plan ID:** 2026-07-07-company-researcher  
**Status:** Infrastructure complete, pipeline ready  
**Target:** John Brooks replica persona + Company Researcher LoRA adapter from investment literature corpus at `/Clones/Library/Company_Research`.

---

## Current State

### Extraction — 93/93 Documents Complete

| Source | Count | Method | Words |
|--------|-------|--------|-------|
| Text PDFs | 33 | `docproc_convert` (pdftotext) | ~3.5M |
| Scanned PDFs | 4 | `docproc_convert --force-ocr` via RunPod `RP/allenai/olmocr-2-7b-1025` | ~180K |
| HTML Posts | 55 | `docproc_convert` (HTML→markdown) | ~100K |
| Guidebook | 1 | Pre-cleaned markdown (`MA_Guidebook_July23.md`) | ~123K |

Extracted text at `corpus/extracted/`. Clark and Larson OCR'd via RunPod OLMOCR (412 pages in 123s, 59K words; 343 pages in 182s, 93K words).

### Infrastructure — Built This Session

| Capability | Where |
|-----------|-------|
| RunPod OCR backend (`RP/` prefix) | `crates/hkask-inference/src/runpod_backend.rs` |
| Baseten inference backend (`BT/` prefix) | `crates/hkask-inference/src/baseten_backend.rs` |
| OpenAI multimodal format (all 7 backends) | `crates/hkask-inference/src/chat_protocol.rs` — `build_vision_request()` |
| ContentGuard on QA generation + h_mem extraction | `mcp-servers/hkask-mcp-docproc/src/tools/semantic.rs` |
| Multi-chunk cross-reference QA generation | `docproc_generate_qa` accepts `texts: [String]` for synthesis mode |
| Cross-reference prompt builder | `corpus-ingest build-prompts --cross-reference` |
| QA embedding for KNN retrieval | `corpus-ingest ingest-qa --embed-qas` |
| Pipeline manifest type system | `crates/hkask-ports/src/pipeline_manifest.rs` |
| Pipeline checkpoint/resume | `crates/hkask-ports/src/pipeline_state.rs` |
| Pipeline runner with verification gates | `crates/hkask-ports/src/pipeline_runner.rs` |
| Corpus pipeline FlowDef manifest | `corpus/pipeline-capabilities-researcher.yaml` |
| Replica server corpus tools | `corpus_embed`, `corpus_salience`, `corpus_build_prompts`, `corpus_ingest_qa` |
| OCR progress tracking (CNS spans) | `mcp-servers/hkask-mcp-docproc/src/ocr/pipeline.rs` |
| CNS namespace hierarchical validation | `crates/hkask-types/src/event.rs` |
| `ChatMessage.images` dead field removed | `crates/hkask-inference/src/chat_protocol.rs` |
| Keychain concurrent-access crash fix | `crates/hkask-keystore/src/keychain.rs` + `hkask-inference/src/config.rs` |
| pdftoppm zero-padded naming fix | `mcp-servers/hkask-mcp-docproc/src/ocr/decimation.rs` |

---

## Remaining Pipeline Steps

The generalized Capabilities Researcher pipeline is defined as a `PipelineManifest` at `corpus/pipeline-capabilities-researcher.yaml`. Steps can be executed individually or via `PipelineRunner` with automatic checkpoint/resume.

### Phase 1: Embedding & Knowledge Graph

```
corpus_embed → EmbeddingStore (1024-dim vectors, Qwen3-Embedding-0.6B)
corpus_salience → HMemStore (concept-tagged chunks, graph-centrality scores)
```

Expected: ~20K chunks embedded + tagged. Checkpoint at `corpus/company-researcher-pipeline-state.json`.

### Phase 2: QA Generation

```
corpus_build_prompts --cross-reference → prompts.jsonl
docproc_generate_qa (multi-chunk mode, Bloom's taxonomy) → generated.jsonl
```

**QA type distribution:** 30% diagnostic, 25% causal, 20% comparative, 15% applied, 10% procedural.

**Cross-reference QAs:** Grouped by shared investment concepts (competitive_advantage, DCF, ROIC, etc.). Top-3 salient chunks per group. LLM must cite passages in answers. Targets 7,500–10,000 total QAs.

### Phase 3: Enrichment

```
corpus_ingest_qa --embed-qas → train.jsonl + HMemStore + EmbeddingStore
replica_compare (John Brooks persona) → quality scores
replica_rewrite (Gentle/Schriver/Hopper/Lovelace dimensions) → enriched QAs
replica_build → John Brooks persona
```

### Phase 4: Training

```
inference_call (Qwen 3.6 27B baseline on test set)
training_submit (Baseten Unsloth, P2 consent-gated)
```

**LoRA config:** rank=8, alpha=16, target=q_proj/v_proj/k_proj/o_proj, batch=64, lr=2e-4, epochs=3, bf16.

---

## QA Quality Strategy

### Gates

| Gate | Tool | Threshold |
|------|------|-----------|
| Length filter | `corpus-ingest ingest-qa` | instruction ≥ 30 chars, output ≥ 100 chars |
| Semantic dedup | `corpus-ingest ingest-qa` | Cosine < 0.92 |
| Content safety | ContentGuard (OWASP LLM01/02/04/06) | Input + output scanning |
| Style quality | `replica_compare` | Centroid distance < 0.40 |
| Perspective rotation | `replica_rewrite` | 4 dimensions × low-scoring QAs |

### Research Basis

- **RA-DIT** (Lin et al., 2024): Cross-attention across retrieved passages during QA generation
- **Self-RAG** (Asai et al., 2023): Source citation reduces hallucination
- **GraphRAG** (Microsoft, 2024): Concept-grouped retrieval for multi-hop reasoning
- **Bloom's Taxonomy**: Progressive difficulty from recall → diagnostic synthesis

---

## Execution

### One-shot (PipelineRunner)

```rust
let yaml = std::fs::read_to_string("corpus/pipeline-capabilities-researcher.yaml")?;
let manifest: PipelineManifest = serde_yaml::from_str(&yaml)?;
let mut runner = PipelineRunner::new(manifest)?;
runner.run_all(&executor);
// Checkpoint at corpus/company-researcher-pipeline-state.json
// Resume: rerun same code — completed steps skip automatically
```

### Step-by-step (MCP)

```bash
# Embed chunks
kask mcp invoke --server replica --tool corpus_embed \
  --input '{"chunks_jsonl":"corpus/chunks/chunks.jsonl","db_path":"corpus/memory/corpus_memory.db"}'

# Build prompts with cross-reference
kask mcp invoke --server replica --tool corpus_build_prompts \
  --input '{"tagged_jsonl":"corpus/chunks/tagged_chunks.jsonl","output":"corpus/qa_pairs/prompts.jsonl","cross_reference":true}'

# Generate cross-reference QA
kask mcp invoke --server docproc --tool docproc_generate_qa \
  --input '{"texts":["passage1...","passage2..."],"chunk_id":"corpus:cross-ref:competitive_advantage","bloom_levels":["diagnostic","comparative"]}'

# Ingest with KNN embedding
kask mcp invoke --server replica --tool corpus_ingest_qa \
  --input '{"db_path":"corpus/memory/corpus_memory.db","output":"corpus/qa_pairs/train.jsonl","embed_qas":true}'

# Build John Brooks persona
kask mcp invoke --server replica --tool replica_build \
  --input '{"config_path":"corpus/replica/john-brooks.yaml","db_path":"corpus/memory/corpus_memory.db"}'
```

---

## File Map

```
corpus/
├── extracted/books/*.txt         ← 37 PDF extractions (gitignored)
├── extracted/posts/maia_substack/*.md ← 55 HTML posts (gitignored)
├── extracted/MA_Guidebook_July23.md ← Pre-cleaned guidebook (gitignored)
├── extracted/manifest.json       ← Extraction provenance (gitignored)
├── chunks/chunks.jsonl           ← 20,433 chunks (tracked)
├── memory/corpus_memory.db       ← EmbeddingStore + HMemStore (gitignored)
├── qa_pairs/seed.json            ← 36 hand-crafted seeds (tracked)
├── qa_pairs/train.jsonl          ← Training QAs (tracked)
├── qa_pairs/val.jsonl            ← Validation QAs (tracked)
├── qa_pairs/test.jsonl           ← Test QAs (tracked)
├── replica/john-brooks.yaml      ← Persona config (tracked)
└── .gitignore                    ← Excludes extracted/, memory/, generated.jsonl

corpus/
└── pipeline-capabilities-researcher.yaml  ← authoritative PipelineManifest

crates/
├── hkask-ports/src/pipeline_manifest.rs  ← PipelineManifest type
├── hkask-ports/src/pipeline_state.rs     ← Checkpoint/resume
├── hkask-ports/src/pipeline_runner.rs    ← PipelineRunner + StepExecutor trait
├── hkask-inference/src/runpod_backend.rs ← RunPod OCR backend
├── hkask-inference/src/baseten_backend.rs ← Baseten backend
└── hkask-corpus-ingest/src/main.rs       ← embed, salience, build-prompts, ingest-qa

mcp-servers/
├── hkask-mcp-docproc/src/tools/semantic.rs ← QA gen + h_mem extraction + ContentGuard
└── hkask-mcp-replica/src/lib.rs           ← corpus_embed/salience/build_prompts/ingest_qa

docs/
└── architecture/core/CROSS_REFERENCE_QA.md ← Cross-reference design doc
```
