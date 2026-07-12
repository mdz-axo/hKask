# hkask-corpus-ingest

Company Researcher corpus embedder. Ingests document corpora and generates embeddings for semantic search and retrieval-augmented generation pipelines.

## Purpose

Bridges external document sources (company filings, research reports) into the hKask embedded corpus. Produces vector embeddings compatible with `hkask-storage` embedding stores.

## Dependencies

- `hkask-types` — foundation ID and error types
- `hkask-storage` — embedding store persistence

## See also

- [`hkask-services-corpus`](../../docs/architecture/) — corpus management service
- [`hkask-mcp-docproc`](../../mcp-servers/hkask-mcp-docproc/) — document processing MCP server