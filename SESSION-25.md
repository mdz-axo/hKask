# Session 25 Brief — hKask Post-Extraction Follow-Up

**Read these files first (in this order):**

1. **`HANDOFF.md`** — Full session history (Sessions 12–24), key decisions (#1–#74),
   service module inventory (§3), file reference map (§6), open questions (§7).
2. **`CONTINUATION.md`** — Task status matrix (F9/F5 ✅ done, 6 remaining), Session 24
   summary, build commands.
3. **`CONTINUATION-PROMPT.md`** — Detailed per-task strategies, constraint
   classifications, depth-test evaluations, and discipline requirements.

**Load these skills before any code changes:**

1. `refactor-service-layer` — Required. Governing methodology for service-layer
   and domain-crate changes.
2. `coding-guidelines` — Required. Surgical changes only.
3. `constraint-forces` — Required. Classify every design decision.
4. `zoom-out` — Required before each task.

---

## What's Done

- **F9** (Session 24): `RecalledEpisode` typed DTO replaces `Vec<serde_json::Value>`
  from `EpisodicStoragePort::recall_episodic`. Domain types (`Confidence`,
  `Visibility`, `Option<WebID>`) replace fragile `.get().and_then()` destructuring
  in `routes/episodic.rs`.
- **F5** (Session 24): `PodManager::new_mock()` uses deterministic test ACP secret
  (`b"hkask-mock-acp-secret-32-bytes!!"`). 4 pod tests now pass without
  `HKASK_ACP_SECRET_KEY`.

## Next Steps (Priority-Ordered)

| # | Task | Priority | Est. |
|---|------|----------|------|
| 1 | F10 — Typed DTOs for `SemanticStoragePort` | 🟡 Medium | ~1–2h |
| 2 | OPEN_QUESTIONS.md — Document F1–F10 | 🟡 Medium | ~30m |
| 3 | Test inventory update | 🟡 Medium | ~1h |
| 4 | `hkask-mcp-condenser` build fix | 🟡 Medium | ~15–30m |
| 5 | F3 — Unified auth context | ⚪ Speculative | — |
| 6 | F4 — MCP server service access | ⚪ Speculative | — |
| 7 | F1 — Streaming responses | ⚪ Speculative | — |
| 8 | F8 — GovernedTool membrane | ⚪ Speculative | — |

**F10 is the natural next step** — it's the symmetric follow-up to F9. The same
fragile `.get("value").and_then(|v| v.as_str())` pattern exists in
`ChatService::recall_semantic` and `PodContext::recall_semantic`. A `RecalledSemantic`
struct in `hkask-agents/src/ports/memory_storage.rs` would fix it the same way
`RecalledEpisode` fixed episodic recall. If `triple_to_json` has no remaining
callers after F10, delete it.

The condenser fix (#4) is quick and unblocks `cargo clippy --workspace`.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*