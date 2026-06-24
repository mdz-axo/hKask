# hkask-mcp-docproc

Unified document processing MCP server — format conversion, OCR, chunking, parsing, and QA generation.

## Tools (9)

| Tool | Description |
|------|-------------|
| `docproc_convert` | Extract text from a document. Detects format, extracts text with automatic OCR fallback for scanned/image-based PDFs. For PDF: tries text extraction first, falls back to vision OCR if result is near-empty. For other supported formats (TXT, MD, HTML): extracts plain text. Requires HKASK_OCR_MODEL for OCR fallback. |
| `docproc_ocr` | OCR a document using a local vision model. Requires HKASK_OCR_MODEL env var or explicit model parameter. The model must be a vision-capable model available in the inference catalog. |
| `docproc_chunk` | Chunk text into passages at configurable token granularity. Accepts raw text or a file path (extracts text from PDF/MD/HTML/TXT with OCR fallback for scanned PDFs). Supports single-tier or multi-tier (coarse/medium/fine) output. |
| `docproc_generate_qa` | Generate QA pairs from a text chunk by calling the inference engine. Returns structured question-answer pairs at specified Bloom's taxonomy levels. |
| `docproc_extract_triples` | Extract RDF triples (subject, predicate, object) from text using the inference engine. Returns structured knowledge triples with confidence scores. |
| `docproc_embed` | Generate embedding vectors for a list of texts (passages or triples). Uses the configured embedding model via the inference router. |
| `docproc_cache` | Cache processed document text for reference. Stores content keyed by label in the docproc cache directory (~/.config/hkask/docproc-cache/). |
| `docproc_query` | Query the in-memory vector index for passages relevant to a natural language question. Embeds the query, computes cosine similarity against indexed passages, and returns top-k results. Optionally generates an LLM-augmented answer from retrieved context. |
| `docproc_clear_index` | Clear the in-memory vector index. Call this when starting a new document set to avoid cross-document contamination in query results. |

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_DB_PATH` | SQLite database path |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase |

## Quick Start

```bash
# The server starts automatically with kask
kask chat
# Or standalone:
hkask-mcp-docproc
```
