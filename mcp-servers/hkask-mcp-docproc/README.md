# hkask-mcp-docproc

Unified document processing MCP server — format conversion, OCR, chunking, parsing, and QA generation.

## Tools (17)

| Tool | Description |
|------|-------------|
| `docproc_convert` | Convert document formats |
| `docproc_ocr` | OCR document images |
| `docproc_chunk` | Chunk document text |
| `docproc_embed` | Embed document chunks |
| `docproc_query` | Query document embeddings |
| `docproc_generate_qa` | Generate QA pairs from documents |
| `docproc_extract_triples` | Extract knowledge triples |
| `docproc_cache` | Cache document processing results |
| `docproc_clear_index` | Clear document index |
| `do_ocr` | Run OCR on image |
| `pdf_to_images` | Convert PDF to images |
| `run_pipeline` | Run full document processing pipeline |
| `index_passages` | Index text passages |
| `enrich_with_semantic` | Enrich with semantic data |
| `resolve_ocr_model` | Resolve OCR model |
| `persist_pipeline_outcome` | Persist pipeline results |
| `run` | Main run loop |

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_DB_PATH` | SQLite database path |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase |
