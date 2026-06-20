# Handoff: kindle-zip — 2026-06-19

## Original Request

Compose a YAML manifest that opens a book from Kindle Cloud Reader by title, captures each page, and saves as PDF. The user's Chrome session is already authenticated at read.amazon.com.

## Architecture (Verified)

```
kask mcp invoke --server docproc --tool docproc_kindle_zip
  │
  ▼
hkask-mcp-docproc / docproc_kindle_zip()
  │
  ├── ChromeCdpClient (WebSocket CDP to local Chrome)
  ├── kindle::discovery (ASIN lookup via JS DOM walk)
  ├── kindle::capture (blob interception + .kr-chevron-container-right page nav)
  └── kindle::assembly (PNG→JPEG→hand-rolled PDF-1.4)
```

Chrome CDP is the only viable path — verified across 3 independent repos. Amazon's API returns glyph-DRM data, not rendered images. Page images exist only in the browser after WebGL rendering.

## Files

| File | Status |
|------|--------|
| `registry/manifests/kindle-zip.yaml` | Ready |
| `mcp-servers/hkask-mcp-docproc/src/kindle/mod.rs` | Ready |
| `mcp-servers/hkask-mcp-docproc/src/kindle/discovery.rs` | Ready |
| `mcp-servers/hkask-mcp-docproc/src/kindle/capture.rs` | Ready |
| `mcp-servers/hkask-mcp-docproc/src/kindle/assembly.rs` | Ready |
| `mcp-servers/hkask-mcp-docproc/src/lib.rs` (ChromeCdpClient + tool) | Ready |
| `docs/plans/kindle-zip-spec.md` | Ready — full spec with verified source references |
| `scripts/kindle-api-test.sh` | Ready — tests API endpoints with user cookies |

**Tests:** 76 pass. Builds clean.

## To Run

On the user's laptop, where Chrome is running with `--remote-debugging-port=9222` and the book is open:

```bash
./target/debug/kask mcp invoke --server docproc --tool docproc_kindle_zip \
  --input '{"book_title":"Find Your Frame","output_pdf":"~/find-your-frame.pdf","max_pages":500,"page_wait_ms":1500}'
```

## Key Decisions

1. **Chrome CDP, not Firecrawl, not API.** Amazon's TLS fingerprinting blocks direct API calls. The API returns glyph data, not images. Firecrawl cannot access the user's Amazon session. Only local Chrome CDP works.

2. **Blob interception over CDP screenshots.** Override `URL.createObjectURL` in page context to capture the renderer's original PNG blobs at full resolution without Chrome UI. Fallback to `Page.captureScreenshot`.

3. **`.kr-chevron-container-right` over ArrowRight for page nav.** Validated by kindle-ai-export. More reliable across readers.

4. **`docproc_kindle_zip` in docproc, not research.** Book → PDF is document processing. Docproc owns document workflows.

5. **Hand-rolled PDF over external tool.** Uses workspace `image` crate. No ImageMagick dependency.

## Not Yet Done

- Collaborative debugging (trace logging, console co-debugging on failure)
- End-to-end run on user's laptop
