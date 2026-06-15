# Handoff — Code Quality & Smell Reduction, Waves 1–2

**Session date:** 2026-06-15  
**Progress:** Waves 1 complete (PR 1.1 + 1.2), Wave 2 PR 2.1 ~40% done  
**Build status:** `cargo check --workspace` + `cargo clippy --workspace -D warnings` → clean

---

## 2) What Was Done

### Wave 1 — Security Boundary Correctness (✅ Complete)

**PR 1.1:** Created shared P4 Gate 1/2/3 startup verifier

- New file: `crates/hkask-mcp/src/startup.rs`
  - `verify_startup_gates(client, replicant, role, required_tools) -> StartupGateResult`
  - Performs `auth_query` (G1) → `assignment_query` (G2) → `capability_query` per tool (G3)
  - G3 denial is non-fatal — tools collected in `StartupGateResult::denied_tools`
  - No tracing in the shared function (tracing macros require constant `target:` values; callers do their own logging)
  - 6 tests with real Unix socket mock listener (same pattern as existing `daemon.rs` tests)
- Modified: `crates/hkask-mcp/src/lib.rs` — added `pub mod startup` + re-exports

**PR 1.2:** Adopted `verify_startup_gates` across all 10 MCP servers

- Removed ~320 lines of duplicated inline auth+assignment match code across 10 MCP servers
- `hkask-mcp-communication` gained daemon gate verification (previously had none)
- `hkask-mcp-condenser` has 7 G3 tools: `compress`, `classify`, `set_profile`, `stats`, `ping`, `persist`, `thread_summary`
- `hkask-mcp-research` retains its 3-tool list: `web_search`, `web_extract`, `web_browse`
- Other 8 servers use empty tool lists (`&[]`) — G3 infrastructure in place, tool lists can be added later

### Wave 2 — Runtime Reliability (🔄 PR 2.1 partially done)

**PR 2.1 so far:** Fixed `.unwrap()` in `hkask-mcp-media` **helper methods** (7 calls fixed):

All in `mcp-servers/hkask-mcp-media/src/main.rs`:

| Function | Line (approx) | Fix applied |
|---|---|---|
| `resolve_image_url` | L564 | `lock().map_err(\|e\| format!(...))?` |
| `resolve_image_path` | L595 | same |
| `resolve_image_id` | L617 | same |
| `resolve_image_url_by_id` | L644 | same |
| `crop_face_region` | L717 | same |
| `rescan_existing_gallery` (read) | L813 | same |
| `rescan_existing_gallery` (write-back) | L845 | same |

These helpers all return `Result<T, String>`, so `.map_err(|e| format!("Gallery state lock error: {}", e))?` propagates the lock error as a `String` to callers that already handle errors.

---

## 3) What Remains

### HIGH — Complete PR 2.1: Fix remaining `.unwrap()` in hkask-mcp-media tool handlers

**~13 `.unwrap()` calls remain** in `mcp-servers/hkask-mcp-media/src/main.rs`. All are in async tool handler functions that return `String` (not `Result`). Each handler already has a `span: ToolSpanGuard` variable in scope.

**Pattern A — lock unwrap in tool handlers (10 calls):**

```rust
// Before:
let guard = self.gallery_state.lock().unwrap();

// After:
let guard = match self.gallery_state.lock() {
    Ok(g) => g,
    Err(e) => return span.internal_error(
        serde_json::json!({"error": format!("Gallery state lock error: {}", e)})
    ),
};
```

Sites:
1. `gallery_status` — ~L1385
2. `gallery_search` — ~L1409
3. `gallery_find_similar` — ~L1588
4. `gallery_refresh` — ~L1736
5. `gallery_analyze` — ~L2042
6. `gallery_name_face` — ~L2160
7. `gallery_timeline` — ~L2487
8. `image_create_collage` — ~L2675

**Pattern B — lock write-back in gallery_organize (1 call):**

```rust
// Before (approx line 1350):
*self.gallery_state.lock().unwrap() = Some(state);

// After:
*match self.gallery_state.lock() {
    Ok(g) => g,
    Err(e) => return span.internal_error(
        serde_json::json!({"error": format!("Gallery state lock error: {}", e)})
    ),
} = Some(state);
```

**Pattern C — canvas.as_mut_rgba8().unwrap() (~L2862):**

```rust
// Before:
for pixel in canvas.as_mut_rgba8().unwrap().pixels_mut() {

// After (infallible — canvas was created as RGBA8):
for pixel in canvas.as_mut_rgba8().expect("canvas was created as RGBA8").pixels_mut() {
```

**Pattern D — text/image_index.unwrap() in gallery_find_similar (2 occurrences, ~L1645 + ~L1687):**

```rust
// Before:
text.unwrap_or_else(|| format!("image_index={}", image_index.unwrap()))

// After: compute fallback separately to avoid unwrap
let query_label = text.clone().unwrap_or_else(|| {
    format!("image_index={}", image_index.unwrap_or(0))
});
// Then use `query_label` in the json! macro instead of the inline expression
```

**Pattern E — conn.lock().unwrap() in main() (~L4032):**

```rust
// Before:
let conn = conn.lock().unwrap();

// After (startup path — expect is appropriate):
let conn = conn.lock().expect("Failed to lock database connection for gallery table init");
```

**Validation after all fixes:**
```bash
cargo check -p hkask-mcp-media
cargo clippy -p hkask-mcp-media -- -D warnings
# Verify zero remaining .unwrap() in non-test runtime code
grep -n '\.unwrap()' mcp-servers/hkask-mcp-media/src/main.rs | grep -v '#\[cfg(test)\]' | grep -v 'mod tests'
```

### MEDIUM — PR 2.2: Fix .unwrap() in hkask-mcp-docproc

Search for `.unwrap()` in `mcp-servers/hkask-mcp-docproc/src/`. The docproc server delegates most logic to its library crate `hkask-mcp-docproc` (in crates). Check both locations. Apply same patterns as PR 2.1.

### MEDIUM — PR 2.3: Fix .unwrap() in hkask-templates registry

Search for `.unwrap()` in `crates/hkask-templates/src/`. Focus on public seam-reachable paths. Apply typed error propagation using existing error types in the crate.

### Remaining Waves (not started)

- **Wave 3:** P8 REQ coverage (tasks 3 + 4)
- **Wave 4:** Architecture convergence (tasks 5 + 6 + 7)
- **Wave 5:** Module depth + safety governance (tasks 8 + 9)
- **Wave 6:** Sustainment (task 10)

---

## 4) Recommended Skills and Tools

For next session, load these skills after context reset:

| Skill | Reason |
|-------|--------|
| `coding-guidelines` | Before writing any code — enforces simplicity, surgical changes, goal-driven |
| `condenser-continuation` | If continuing from this handoff in a fresh session |

Key commands:
```bash
cargo check -p hkask-mcp-media          # Verify after each batch of unwrap fixes
cargo clippy -p hkask-mcp-media -- -D warnings
cargo test -p hkask-mcp                  # Re-run startup gate tests
cargo check --workspace                  # Full workspace check before moving to PR 2.2
```

---

## 5) Key Decisions to Preserve

1. **No tracing in `verify_startup_gates`** — Tracing macros (`info!`, `warn!`, etc.) require constant `target:` values at compile time. Passing a runtime `trace_target` string fails with `E0435`. The shared function returns a structured result; each caller logs with its own constant target string.

2. **G3 capability denial is non-fatal** — Matches the research server's existing behavior. Servers start in degraded mode; denied tools are reported via `StartupGateResult::denied_tools`. Reason: OCAP principle is that tools are individually gated, not the whole server. Denying one tool shouldn't block the server.

3. **Empty tool lists for G3 are valid** — 8 of 10 servers use `&[]` for `required_tools`. This means Gate 3 is a no-op (no capability queries). Tool lists can be populated as each server's capability requirements are defined. The infrastructure is in place.

4. **For lock poisoning in helper methods: `String` error propagation** — Since these helpers return `Result<T, String>` (not `McpToolError`), the lock poison error is converted to a `format!("Gallery state lock error: {}", e)` string. Callers already handle `String` errors via span guards or error accumulation. This avoids changing function signatures.

5. **For lock poisoning in tool handlers: `span.internal_error()`** — These handlers return `String` directly. The `span` guard is already in scope. Using `span.internal_error()` produces an `McpErrorKind::Internal` error in the JSON output, which is the correct taxonomy for infrastructure faults like lock poisoning.

6. **Communication server now has `try_daemon_flow`** — Previously the only server without daemon verification. Added with role `"communication"` and empty tool list. The `_daemon_client` is created but not yet wired into the `CommunicationServer` struct (that struct has no `daemon` or `replicant` fields — a separate task).
