# Kindle Zip — Specification

## Original Request

> Create something to flip through a book in my Kindle library (using the open browser session at https://read.amazon.com/kindle-library). (1) Open a book given the title, (2) take a picture of each page, (3) save as a PDF. Compose this as a YAML manifest.

## Verified Architecture

### Why Chrome CDP is the only viable path

**Research conclusion:** Amazon's `renderer/render` API endpoint returns glyph-DRM data (glyph IDs + SVG glyph paths + layout JSON), not rendered page images. Page images exist only in the browser after WebGL rendering. The API alone cannot produce a PDF — a browser is required to render pages.

Verified across 3 independent sources:
- `kindle-ai-export` (307★): "Kindle's web reader uses WebGL... to render the page contents"
- `kindle-api` (223★): Used only for book listing/metadata, explicitly notes "Missing features: Reading book content"
- `dekindled` (33★): Chrome extension that captures rendered images from the browser DOM

Approaches eliminated:
- **API-only**: Returns glyph data, not images. Would require building a full glyph renderer.
- **WebSQL extraction** (`kindle-fetch`): Reads Chrome's offline storage but still requires browser to download the book first, and schema is undocumented.
- **Firecrawl** (cloud headless): Cannot access user's authenticated Amazon session.

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
       ├── ChromeCdpClient           ← WebSocket CDP to local Chrome
       ├── kindle::discovery         ← ASIN lookup via DOM walk
       ├── kindle::capture           ← blob interception + page nav
       └── kindle::assembly          ← PNG → PDF
```

### Module structure (`mcp-servers/hkask-mcp-docproc/src/kindle/`)

| Module | Purpose | Key functions |
|--------|---------|--------------|
| `mod.rs` | Shared types, validated CSS selectors | `CapturedPage`, `selectors` |
| `discovery.rs` | Find book ASIN by title in library DOM | `find_asin_by_title(chrome, title) → Option<ASIN>` |
| `capture.rs` | Open reader, inject blob intercept, page nav, capture | `open_reader()`, `inject_blob_intercept()`, `next_page()`, `capture_page()` |
| `assembly.rs` | PNG screenshots → PDF (hand-rolled, zero external deps) | `assemble(pages, output_path) → Result<u64>` |

### ChromeCdpClient (`lib.rs`)

Raw WebSocket CDP client using `tokio-tungstenite`. Key methods:

| Method | CDP Command | Purpose |
|--------|------------|---------|
| `connect()` | `GET localhost:9222/json` | Find Kindle tab by URL, open WebSocket |
| `evaluate(js)` | `Runtime.evaluate` | Execute JS in page, return result |
| `send_command(method, params)` | Generic CDP | Send any CDP command, await matching response |
| `capture_screenshot()` | `Page.captureScreenshot` | Fallback screenshot as base64 PNG |
| `press_key(key)` | `Input.dispatchKeyEvent` | Key down + key up events |

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

## Research — Verified Source References

All endpoints, selectors, and cookie names verified against source code as of 2026-06-19.

### Kindle Cloud Reader API Endpoints

| Endpoint | Verified in | Purpose | Returns |
|----------|------------|---------|---------|
| `GET /kindle-library/search?query=&libraryType=BOOKS&sortType=recency&querySize=50` | `kindle-api/src/kindle.ts:48` | List books, get ASINs | JSON: `itemsList` with `title`, `asin`, `authors`, `imageUrl` |
| `GET /register/getDeviceToken?serialNumber=X&deviceType=X` | `kindle-api/src/kindle.ts:37` | Get device session token | JSON: `deviceSessionToken` |
| `GET /service/mobile/reader/startReading?asin=X&clientVersion=20000100` | `kindle-ai-export/src/extract-kindle-book.ts:125` | Open book, get render token | JSON: `karamelToken.token` |
| `GET /renderer/render?version=3.0&asin=X&...` | `kindle-ai-export/src/extract-kindle-book.ts:170` | Fetch page glyph data | TAR: page_data, glyphs.json, layout_data, **no images** |

### Required Cookies (from `kindle-api/src/http-client.ts:66-70`)

| Cookie name (browser) | Rust/JS field name |
|----------------------|-------------------|
| `ubid-main` | `ubidMain` |
| `at-main` | `atMain` |
| `session-id` | `sessionId` |
| `x-main` | `xMain` |

### Required Headers (from `kindle-api/src/http-client.ts:39-45`, `kindle-ai-export`)

| Header | Source | Purpose |
|--------|--------|---------|
| `User-Agent: Mozilla/5.0 ... Chrome/112.0.0.0 Safari/537.36` | kindle-api | Browser impersonation |
| `x-amzn-sessionid: {sessionId}` | kindle-api | Session tracking |
| `x-adp-session-token: {adpToken}` | kindle-api | Device auth |
| `x-amz-rendering-token: {karamelToken}` | kindle-ai-export | Render authorization |

### Amazon TLS Fingerprinting

From `kindle-api/src/http-client.ts:47-53`: Amazon blocks non-browser TLS handshakes. The `kindle-api` library routes all requests through a `tls-client-api` proxy configured as `chrome_112`. **This means direct curl/reqwest calls to the API endpoints may get 403.** The CDP approach avoids this entirely — Chrome handles TLS natively.

### Validated CSS Selectors (from `kindle-ai-export`)

```
Rendered page image:  #kr-renderer .kg-full-page-img img     (line 49)
Next page chevron:    .kr-chevron-container-right             (line 408)
Settings button:      ion-button[aria-label="Reader settings"]  (line 315)
Page footer:          ion-footer ion-title                     (line 355)
Font selector:        #AmazonEmber                            (line 324)
Single column:        [role="radiogroup"][aria-label$=" columns"]  (line 330)
```

### Page Navigation (from `kindle-ai-export` lines 395-430)

- **Method**: Click `.kr-chevron-container-right` (not ArrowRight — more reliable per author's testing at line 404-408)
- **Confirmation**: Poll `img[src]` until the `src` attribute changes (lines 414-425)
- **Timeout**: 10 seconds (50 × 200ms polls), then 1 second retry (line 412)
- **Retries**: 30 retries before giving up (line 430)
- **End detection**: Image src stops changing across attempts

### Blob Interception (from `kindle-ai-export` lines 131-170, `dekindled` interceptor.js)

Override `URL.createObjectURL` in page context:
1. Intercept PNG/WEBP blobs before Kindle's renderer revokes them
2. Read blob bytes via `arrayBuffer()`
3. Convert to base64 via manual byte walk
4. Send back via CDP `Runtime.bindingCalled`

This gives full-resolution rendered images at device scale, without viewport limitations or Chrome UI.

### Blob Payload Format (from `kindle-ai-export` lines 146-155)

```json
{
  "url": "blob:https://read.amazon.com/...",
  "type": "image/png",
  "base64": "..."
}
```

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

## Test Scripts

| Script | Purpose | Status |
|--------|---------|--------|
| `scripts/kindle-api-test.sh` | Test API endpoints with user's cookies | ✅ Updated with verified endpoints/headers |
| `scripts/kindle-zip.sh` | Zero-dependency bash CDP capture | ✅ |

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
| `docproc_kindle_zip` wired to kindle module | ✅ Done (builds, 76 tests pass) |
| Collaborative debugging (trace logging) | Not yet |
| Chrome console co-debugging on failure | Not yet |
