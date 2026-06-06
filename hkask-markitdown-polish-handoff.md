# Handoff Document — hKask Local OCR / Markitdown Integration

**Session Purpose:** Documentation, verification & polish for the markitdown + vision inference integration
**Timestamp:** 2026-06-06
**Document Version:** 1.0.0

---

## Next Session Purpose

Continue polish and end-to-end verification of the markitdown OCR pipeline. The documentation updates are complete; remaining work is runtime testing and optional code improvements.

---

## Progress Summary

| # | Task | Status | Notes |
|---|------|--------|-------|
| T1 | Update architecture docs (19→21 MCP servers) | ✅ Complete | All 10+ docs updated |
| T2 | Add `hkask-mcp-markitdown` to MCP server inventory tables | ✅ Complete | domain-and-capability, audit, tools-inventory |
| T3 | Add `hkask-mcp-doc-knowledge` to MCP server inventory tables | ✅ Complete | Was missing from inventory — added alongside markitdown |
| T4 | Add `inference_generate_vision` tool to inventory | ✅ Complete | mcp-tools-inventory: 4 inference tools (was 3) |
| T5 | Document vision inference in okapi-integration.md | ✅ Complete | New "Vision Inference" section |
| T6 | Add `HKASK_OCR_MODEL` env var documentation | ✅ Complete | Covered in okapi-integration + credential table |
| T7 | Document OCR fallback pipeline | ✅ Complete | markitdown_convert flow documented |
| T8 | Verify no constraint violations | ✅ Complete | Clean grep results |
| T9 | Verify workspace compiles | ✅ Complete | `cargo check --workspace` passes |
| T10 | Verify clippy passes | ✅ Complete | `-D warnings` clean |
| T11 | Verify unit tests pass | ✅ Complete | 5/5 in markitdown convert module |
| T12 | Correct MCP server count (was 20 in handoff, actually 21) | ✅ Complete | doc-knowledge was also new |

---

## Key Decisions & Rationale

1. **21 MCP servers, not 20.** The handoff stated 19→20, but `hkask-mcp-doc-knowledge` was also added after the original 19. Actual count is 21.
2. **No shared `strip_html` crate (yet).** Two identical copies exist. Per C7, two copies is acceptable until a third consumer appears.
3. **`generate_vision` is not on the `InferencePort` trait.** It's a direct `impl OkapiInference` method because the trait is text-only. Intentional design.
4. **`default_ocr_max_tokens` is 4096.** May be too low for long documents. Flagged for future.
5. **DOCX/PPTX/XLSX extraction explicitly deferred.** `detect_format` recognizes but `markitdown_convert` returns `InvalidArgument` with guidance.

---

## Current State

All documentation is updated and consistent. Workspace compiles, clippy passes, unit tests pass. Remaining: runtime E2E testing and optional code polish.

---

## Artifact References

| Type | Path | Relevance |
|------|------|-----------|
| source | `crates/hkask-templates/src/inference_port.rs` | `Message.images` + `generate_vision()` |
| source | `mcp-servers/hkask-mcp-markitdown/src/tools.rs` | 3 MCP tools + `MarkitdownServer` + `do_ocr` |
| source | `mcp-servers/hkask-mcp-markitdown/src/convert.rs` | `detect_format`, `strip_html`, `is_format_supported` |
| source | `mcp-servers/hkask-mcp-markitdown/src/main.rs` | Entry point, `HKASK_OCR_MODEL` env var |
| source | `mcp-servers/hkask-mcp-doc-knowledge/src/main.rs` | Updated doc_knowledge tools (PDF→markitdown redirect) |
| doc | `AGENTS.md` | Updated: 21 servers, crate map, MCP server list |
| doc | `docs/architecture/domain-and-capability.md` | Updated: 21 servers, inventory table |
| doc | `docs/architecture/reference/okapi-integration.md` | New: Vision Inference section |

---

## Open Questions & Risks

| Question | Risk Level | Context |
|----------|-----------|---------|
| `default_ocr_max_tokens` (4096) too low? | Low | Consider configurable or 8192 default |
| `do_ocr` creates `OkapiInference` per call | Low | Cache client if hot path |
| `markitdown_convert` reads PDF bytes twice | Low | Could cache; current impl is correct |
| E2E OCR pipeline not tested | Medium | Requires running Okapi + vision model |
| `hkask-cns` build stability | Medium | Verify with `cargo clean && cargo check` |
| Zed settings.json binary name | Low | Binary is `hkask-mcp-markitdown`, not `hkask-markitdown` |