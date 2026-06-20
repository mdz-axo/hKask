# Kindle Zip — Specification

## Original Request

> Create something to flip through a book in my Kindle library (using the open browser session at https://read.amazon.com/kindle-library). (1) Open a book given the title, (2) take a picture of each page, (3) save as a PDF. Compose this as a YAML manifest.

## Architecture

### Dispatch

```
kask dispatch kindle-zip --param book_title="Find Your Frame"
  │
  ▼
registry/manifests/kindle-zip.yaml   ← YAML flow definition
  │
  ▼
hkask-mcp-docproc                    ← MCP server
  └─ docproc_kindle_zip()            ← tool
       │
       ├── kindle::discovery          ← ASIN lookup
       ├── kindle::capture            ← reader control + page capture
       └── kindle::assembly           ← PNG → PDF
```

### Module structure (`mcp-servers/hkask-mcp-docproc/src/kindle/`)

| Module | Purpose | Key types/functions |
|--------|---------|-------------------|
| `mod.rs` | Shared types, CSS selectors | `CapturedPage`, `selectors` |
| `discovery.rs` | Find book ASIN by title in library DOM | `find_asin_by_title(chrome, title) → Option<ASIN>` |
| `capture.rs` | Open reader, inject blob intercept, page nav, capture | `open_reader()`, `inject_blob_intercept()`, `next_page()`, `capture_page()` |
| `assembly.rs` | PNG screenshots → PDF | `assemble(pages, output_path) → Result<u64>` |

### Flow at runtime

```
1. ChromeCdpClient::connect()
   └─ GET localhost:9222/json → find read.amazon.com tab
   └─ WebSocket to tab's debugger URL

2. Navigate to Kindle library (if not already there)
   └─ Runtime.evaluate("window.location.href = '...'")

3. kindle::discovery::find_asin_by_title()
   └─ Runtime.evaluate(JS DOM walk) → extract ASIN from book link href

4. kindle::capture::open_reader()
   └─ Navigate to https://read.amazon.com/?asin={ASIN}
   └─ Wait for reader to load

5. kindle::capture::inject_blob_intercept()
   └─ Runtime.addBinding("captureKindleBlob")
   └─ Page.addScriptToEvaluateOnNewDocument (override URL.createObjectURL)

6. Loop per page:
   ├─ kindle::capture::capture_page()
   │   ├─ Try blob interception (full resolution, no Chrome UI)
   │   └─ Fallback: Page.captureScreenshot (CDP viewport screenshot)
   └─ kindle::capture::next_page()
       └─ Click .kr-chevron-container-right
       └─ Poll #kr-renderer .kg-full-page-img img src until change (timeout 10s → end of book)

7. kindle::assembly::assemble()
   └─ PNG → JPEG re-encode (image crate)
   └─ Hand-rolled PDF-1.4 (DCTDecode, one image per page)
```

## Research — Prior Art

| Repo | Stars | Approach | What we use |
|------|-------|----------|-------------|
| `transitive-bullshit/kindle-ai-export` | 307 | Playwright + blob interception | Blob interception technique, CSS selectors, ASIN URL pattern, page nav via `.kr-chevron-container-right` |
| `Xetera/kindle-api` | 223 | Kindle private API (cookies + TLS proxy) | ASIN URL pattern (`?asin=X`), book metadata structure |
| `dmilin1/dekindled` | 33 | Chrome extension, blob intercept + auto-scan + EPUB | Blob interception validation (same technique), auto-page-turn concept |
| `d10r/kindle-fetch` | 59 | Chrome WebSQL extraction | Alternative: extract from offline storage without browser automation |
| `lazykern/kindle-cloud-reader-rpc` | 5 | Chrome extension, Discord RPC | DOM state detection patterns |

### Validated CSS Selectors (from kindle-ai-export)

```
Rendered page image:  #kr-renderer .kg-full-page-img img
Next page chevron:    .kr-chevron-container-right
Settings button:      ion-button[aria-label="Reader settings"]
Page footer:          ion-footer ion-title
Font selector:        #AmazonEmber
Single column:        [role="radiogroup"][aria-label$=" columns"]
```

### Page Navigation (from kindle-ai-export)

- **Method**: Click `.kr-chevron-container-right` (not ArrowRight key — more reliable per the author's testing)
- **Confirmation**: Poll `#kr-renderer .kg-full-page-img img[src]` until the `src` attribute changes
- **Timeout**: 10 seconds (50 × 200ms polls)
- **End of book**: Image src stops changing

### Blob Interception (from kindle-ai-export + dekindled)

Override `URL.createObjectURL` in page context:
1. Intercept the blob before Kindle's renderer revokes it
2. Read blob bytes via `arrayBuffer()`
3. Convert to base64
4. Send back via CDP `Runtime.bindingCalled`

This gives full-resolution rendered images without viewport limitations or Chrome UI.

## Collaborative Debugging Strategy

The user requires that I can help debug when things fail. The compiled Rust binary is opaque to me from this server. Mitigations:

### Approach A: Structured logging
- Set `RUST_LOG=hkask.mcp.docproc=trace` for per-step CDP visibility
- Each module logs entry/exit with timing
- Failed CDP commands log the full response

### Approach B: Step-by-step diagnostics
- The tool emits JSON diagnostics at each step
- The user pastes the diagnostics, I analyze
- Each step is independently retryable

### Approach C: Chrome console co-debugging
- When a step fails, print the exact JS that was attempted
- User runs it in Chrome DevTools console and pastes the result
- I iterate on the JS until it works, then update the tool

### Recommended: Hybrid (A + C)
- Build with detailed trace logging enabled by default for kindle operations
- On failure, print the failing CDP command and expected outcome
- User can run equivalent commands in Chrome DevTools to diagnose

## Implementation Status

| Component | Status |
|-----------|--------|
| `registry/manifests/kindle-zip.yaml` | ✅ Done |
| `kindle/mod.rs` (types, selectors) | ✅ Done |
| `kindle/discovery.rs` (ASIN lookup) | ✅ Done |
| `kindle/capture.rs` (reader + blob + nav) | ✅ Done |
| `kindle/assembly.rs` (PDF) | ✅ Done |
| `ChromeCdpClient` in lib.rs | ✅ Done (pub(crate)) |
| `KindleZipRequest` struct | ✅ Done |
| `docproc_kindle_zip` tool wiring | ❌ Needs update — still uses old inline code, needs to delegate to kindle module |
| Tests (PDF assembly) | ❌ Need update — reference `assemble_kindle_pdf` not `kindle::assembly::assemble` |

## Dependencies

Added to workspace:
```
tokio-tungstenite = "0.26"  # WebSocket for Chrome CDP
```

Added to `hkask-mcp-docproc`:
```
tokio-tungstenite.workspace = true
futures-util.workspace = true
```

Existing (no new deps):
```
image.workspace = true       # PNG → JPEG for PDF
base64.workspace = true      # Screenshot decode
reqwest.workspace = true     # CDP tab discovery (localhost:9222/json)
```

## Remaining Work (in priority order)

1. **Wire `docproc_kindle_zip` to kindle module** — replace inline code with kindle::discovery + kindle::capture + kindle::assembly calls
2. **Update tests** — reference `kindle::assembly::assemble` instead of `assemble_kindle_pdf`
3. **Remove old `assemble_kindle_pdf`** from lib.rs — now in `kindle::assembly::assemble`
4. **Add trace logging** — `tracing::debug!` at each CDP step for collaborative debugging
5. **Add Chrome console co-debugging output** — on failure, emit the JS that was attempted
6. **Test end-to-end on user's laptop** — with `RUST_LOG=hkask.mcp.docproc=trace`
