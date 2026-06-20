# Handoff: kindle-zip — 2026-06-19

## Original Request

> "create something to flip through a book in my kindle library (just use the open browser session and assume that I left it open to the main page: https://read.amazon.com/kindle-library) and (1) open a book given the title; (2) take a picture of each page and (3) save it as a pdf. compose this as a yaml manifest"

The deliverable is a YAML manifest — a composable flow definition in the hKask registry pattern. The user's Chrome session is already open and authenticated at `read.amazon.com/kindle-library`.

## What Exists

### Manifest: `registry/manifests/kindle-zip.yaml`

Dispatches via `kask dispatch kindle-zip --param book_title="..."`. Two steps:

1. **Capture** — dispatches to `hkask-mcp-docproc` → `docproc_kindle_zip` tool. Connects to local Chrome via DevTools Protocol (Chrome must be running with `--remote-debugging-port=9222`). Pages through the Kindle Cloud Reader book by pressing ArrowRight, captures each page via `Page.captureScreenshot`, assembles into PDF.

2. **CNS feedback** — emits capture metrics.

### Tool: `docproc_kindle_zip` in `hkask-mcp-docproc`

Registered in the docproc `#[tool_router]` at `mcp-servers/hkask-mcp-docproc/src/lib.rs:1824`. Three components:

- **`ChromeCdpClient`** — raw WebSocket CDP client. Connects to `localhost:9222/json` to find the Kindle tab, then WebSocket for `Page.captureScreenshot`, `Input.dispatchKeyEvent`, `Runtime.evaluate`.

- **Capture loop** — screenshots each page, presses ArrowRight, sleeps `page_wait_ms`, repeats up to `max_pages`. Stops on screenshot error or too-small image.

- **`assemble_kindle_pdf`** — takes `Vec<Vec<u8>>` PNG screenshots, re-encodes as JPEG via `image` crate, writes a minimal PDF with one DCT-encoded image per page. Zero external dependencies — uses only workspace crates `image`, `base64`.

**Dependencies added:** `tokio-tungstenite`, `futures-util` (for WebSocket CDP).

**Tests:** 76 pass (73 original + 3 kindle-zip: PDF assembly validation, empty rejection, default values).

## What the Pipeline Does at Runtime

```
User's laptop                          Docproc MCP server
─────────────                          ──────────────────
chromium --remote-debugging-port=9222
  │ (already logged into Amazon,
  │  book open in Kindle Cloud Reader)
  │
  │                    kask dispatch kindle-zip --param book_title="Find Your Frame"
  │                         │
  │                         ▼
  │              ChromeCdpClient::connect()
  │                GET localhost:9222/json → find read.amazon.com tab
  │                WebSocket to tab's debugger URL
  │                         │
  │              Runtime.evaluate("window.location.href") → verify
  │                         │
  │              FOR each page:
  │                Page.captureScreenshot → base64 PNG ←─────────┘
  │                Input.dispatchKeyEvent("ArrowRight")
  │                sleep(page_wait_ms)
  │                         │
  │              assemble_kindle_pdf(pngs, output_path)
  │                → writes PDF to disk
```

## What's Needed to Run

1. Chrome launched with `--remote-debugging-port=9222` on the user's laptop
2. Kindle Cloud Reader open with the book visible
3. `kask dispatch kindle-zip --param book_title="Find Your Frame"`

The docproc server doesn't need `FIRECRAWL_API_KEY` — it connects to local Chrome directly.

## Key Decisions

1. **Chrome CDP, not Firecrawl.** The user's Amazon session exists in their local Chrome. A remote headless browser can't access it. The tool connects to the user's existing browser via DevTools Protocol.

2. **`docproc_kindle_zip` in docproc, not research.** The user identified this as document processing — turning a book into a PDF. Docproc owns document workflows. Research finds things on the web.

3. **Hand-rolled PDF assembler over external tool.** Uses workspace `image` crate to re-encode PNGs as JPEG and writes a minimal PDF. No ImageMagick, no external process calls. Portable, testable, zero runtime deps beyond workspace crates.

4. **ArrowRight key for page turning.** Kindle Cloud Reader responds to keyboard navigation. More reliable than coordinate-based clicks which break on viewport changes.

## Open Questions

- Does the Chromium snap on Ubuntu allow `--remote-debugging-port=9222`? Snap sandboxing may block the port.
- Will ArrowRight reliably turn pages, or does Kindle Cloud Reader require click/touch events on some browsers?
- The tool assumes the correct book is already open. The original request said "open a book given the title" — the library search and book selection aren't yet implemented in the CDP client. The current implementation expects the book to be already visible in the reader.
