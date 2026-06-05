# hKask Loop Distillation — Handoff Document

**Session purpose:** Execute the 10-task action plan from the adversarial review of hKask's 6-loop architecture.  
**Timestamp:** 2026-06-05  
**Version:** 1.0

---

## Next Session Purpose

Complete the remaining work from the adversarial review action plan. Three areas need attention:

1. **Fix `hkask-api/src/lib.rs` breakage** — a sub-agent incorrectly removed `consolidation_bridge` and `semantic_memory_for_consolidation` fields from `ApiState`. Another agent is rebuilding the consolidation feature — coordinate with them before touching consolidation-related code.
2. **Complete Task 8 (4-stage cycle)** — the `EpisodicLoop` and `SemanticLoop` structs were created but reverted because they reference consolidation types. Wait for the consolidation agent to finish, then re-add these loops without consolidation coupling (or with the new consolidation API).
3. **Verify and clean the workspace** — run `cargo check --workspace --exclude hkask-mcp-semantic` (pre-existing errors in that crate), fix any remaining issues, and run the full test suite.

---

## Progress Summary

| Task | Description | Status | Notes |
|------|-------------|--------|-------|
| **T1** | Dead weight excision (P6/P7) | ✅ Complete | Deleted `BundleEvolver`, `ResponseContract`, 9 dead Registry methods, 10 dead re-exports. Downgraded visibility on `AllostericGate` fields, `GasBudget` test methods, `McpDispatcher::new()`, `CnsRuntime` dead methods. |
| **T2** | Relocate misassigned items | ✅ Complete | A2: Allosteric primitives → `hkask-types`. A1: Curation config loaders → `hkask-cli`. A3: Prompt analysis → `hkask-agents`. |
| **T3** | Wire missing loop connections | ⚠️ Partial | B5 (kill-zone MCP), B7 (replenish budget MCP), B3 (backpressure wired), B1 (consolidation candidate count), B6 (A2A message tool), B8 (improv_turn redesign) — all done. But MCP server files for episodic/semantic were **reverted** because they conflict with consolidation rebuild. B2/B4 skipped. |
| **T4** | Capability boundary violations | ✅ Complete | V2: `PodContext::invoke_tool` now routes through `GovernedTool` when available. V3: `McpGovernor` doc note added. V1, V4, V5, V6 were confirmed non-issues by verification. |
| **T5** | Unify registry architecture | ✅ Complete | Added `AgentRegistrationPort` trait to `hkask-types`, implemented for `AgentRegistryStore`. Documented registry as shared substrate. |
| **T6** | MCP server loop assignments | ✅ Complete | `hkask-mcp-ocap` → L6, `hkask-mcp-keystore` → L6, `hkask-mcp-registry` → L1↔L5 bridge. Added to architecture docs. |
| **T7** | Gas deduplication | ✅ Complete | `InferenceLoop::consume_gas/replenish_gas` → `pub(crate)`. Added `token_usage()` and `sync_gas_state()`. REPL uses single bulk sync from L6 budget. Added `AgentGasStatus` struct. |
| **T8** | 4-stage cycle for all loops | ⚠️ Partial | Added 4-stage methods to Curation, Metacognition, Inference, Communication loops. **Episodic/Semantic loops were reverted** — they reference consolidation types and conflict with the other agent's rebuild. |
| **T9** | MCP server count doc fix | ✅ Complete | Updated PRINCIPLES.md, OPEN_QUESTIONS.md, REQUIREMENTS.md to say 19 servers. |
| **T10** | CNS MCP server surface | ✅ Complete | Added `cns_energy` and `cns_backpressure` tools. `should_alert`/`usage_ratio` no longer dead. |

---

## Key Decisions & Rationale

1. **Dead weight → delete, not just downgrade.** Clippy with `-D warnings` catches `pub(crate)` dead code. Deleted methods entirely rather than keeping them as dead `pub(crate)`. Exception: items needed for planned wiring (B3, B5, B7, T7) got `#[allow(dead_code)]` with doc comments explaining when they'll be wired.

2. **`PromptStrategy::frame()` was NOT dead** despite the review claiming 0 consumers. The API chat route at `crates/hkask-api/src/routes/chat.rs:97` calls it. Restored after initial deletion caused compilation failure. The verification sub-agent missed this caller.

3. **AllostericGate → `hkask-types` (not just re-export).** The authority inversion (L5 depending on L6 for its own regulation primitive) required moving the types to the shared substrate crate. `hkask-cns` now re-exports from `hkask-types` for backward compat.

4. **V2 fix: `governed_tool` on PodContext, not replacing MCPRuntimePort.** Added an optional `Arc<dyn ToolPort>` alongside the existing `mcp_runtime`. When present, tool invocations route through GovernedTool. When absent, falls back to the raw path. This is non-breaking — existing callers don't need to change.

5. **`MCPRuntimePort::resolve_tool_server()` with default impl.** Added to the trait with a default `None` return. Only `McpRuntimeAdapter` implements it. This avoids breaking all existing trait implementors.

6. **Consolidation code is off-limits.** Another agent is rebuilding the consolidation feature. Reverted all changes to `hkask-api`, `hkask-mcp-episodic`, `hkask-mcp-semantic`, and the new `EpisodicLoop`/`SemanticLoop` structs because they reference consolidation types.

---

## Current State

### Working tree changes (uncommitted)

5 files modified, 149 insertions, 29 deletions:

- `crates/hkask-agents/src/communication/communication_loop.rs` — 4-stage cycle methods (Task 8)
- `crates/hkask-agents/src/curator/curation_loop.rs` — 4-stage cycle methods (Task 8)
- `crates/hkask-agents/src/curator_agent/metacognition.rs` — 4-stage cycle methods (Task 8)
- `crates/hkask-agents/src/inference_loop.rs` — 4-stage methods + `token_usage()` + `sync_gas_state()` (Tasks 7+8)
- `crates/hkask-api/src/lib.rs` — **BROKEN**: sub-agent removed consolidation fields. Needs revert to HEAD.

### Already committed (HEAD = 661f1cfc2)

All Task 1, 2, 3, 5, 6, 9, 10 changes are committed. See `git diff HEAD~1..HEAD --stat` for the full list of 31 files.

### Pre-existing issues (NOT caused by this session)

- `hkask-mcp-semantic` has 7 compilation errors (`chunking` not found, `validation_error` not found, `search` not found on SemanticMemory). These existed before any of our changes.
- `hkask-cns` has 10 dead code warnings when run with `cargo clippy -p hkask-cns -- -D warnings` (pre-existing items like `with_allosteric_gate`, `clear_old`, `reset`, `BOT` constant, etc.)

---

## Artifact References

| Type | Path | Relevance |
|------|------|-----------|
| source | `crates/hkask-agents/src/pod/context.rs` | V2 fix: `governed_tool` field + GovernedTool routing in `invoke_tool` |
| source | `crates/hkask-agents/src/pod/manager.rs` | V2 fix: `governed_tool` field + builder methods |
| source | `crates/hkask-agents/src/ports/mcp_runtime.rs` | V2 fix: `resolve_tool_server()` method |
| source | `crates/hkask-agents/src/adapters/mcp_runtime.rs` | V2 fix: `resolve_tool_server()` implementation |
| source | `crates/hkask-types/src/allosteric/` | A2 relocation: AllostericGate, BernoulliDistribution, etc. moved here |
| source | `crates/hkask-cns/src/allosteric/mod.rs` | A2: now re-exports from `hkask-types` |
| source | `crates/hkask-cli/src/curation_config.rs` | A1 relocation: curation threshold loaders moved here |
| source | `crates/hkask-agents/src/prompt_analysis.rs` | A3 relocation: PromptAnalysis moved here |
| source | `crates/hkask-types/src/ports.rs` | T5: `AgentRegistrationPort` trait added |
| source | `mcp-servers/hkask-mcp-cns/src/main.rs` | T3+B5+B7+T10: `cns_kill_zone`, `cns_replenish_budget`, `cns_energy`, `cns_backpressure` tools |
| source | `mcp-servers/hkask-mcp-ensemble/src/main.rs` | T3+B6+B8: `agent_send_message` tool, `improv_turn` redesign |
| source | `crates/hkask-cns/src/runtime.rs` | T3: `register_gas_budget`, `replenish_agent_budget`, `agent_gas_status` added; `emit_backpressure`/`kill_zone_state`/`check_kill_zone` made `pub` |
| source | `crates/hkask-cns/src/energy.rs` | T7+T10: `AgentGasStatus` struct, `From<&GasBudget>` impl |
| doc | `docs/architecture/PRINCIPLES.md` | T9: 19 servers, T6: loop annotations |
| doc | `docs/architecture/loop-architecture.md` | T6: §3.4 MCP Server-to-Loop Mapping table |
| doc | `crates/hkask-mcp/src/governor.rs` | V3: architectural note about misplacement |

---

## Suggested Skills

| Skill | Reason | Priority |
|-------|--------|----------|
| `coding-guidelines` | Before continuing T8 (4-stage cycle), verify changes follow Karpathy's principles (simplicity first, surgical changes) | recommended |
| `handoff` | If session runs long again, compact again for the next handoff | optional |

---

## Open Questions & Risks

| Question | Risk | Context |
|----------|------|---------|
| How to re-add `EpisodicLoop`/`SemanticLoop` without coupling to consolidation? | Medium | Task 8's loop structs were reverted because they use `ConsolidationPort`/`ConsolidationToken`. Wait for consolidation agent to finish, then use their new API. |
| Is `hkask-api/src/lib.rs` still broken in working tree? | High | The file has diff removing consolidation fields. Run `git checkout HEAD -- crates/hkask-api/src/lib.rs` immediately, or coordinate with the consolidation agent. |
| Should `hkask-mcp-episodic` and `hkask-mcp-semantic` changes be re-applied? | Medium | Task 3 added `episodic_consolidate_status` and modified `semantic_consolidate`. Reverted to avoid conflict. Re-apply after consolidation agent finishes. |
| Pre-existing `hkask-mcp-semantic` compilation errors | Low | 7 errors in that MCP server (missing `chunking`, `validation_error`, `search`). Not caused by our changes. Another agent may be fixing these. |
| `hkask-cns` dead code (10 items) — fix or leave? | Low | Pre-existing dead code in `hkask-cns` fails `cargo clippy -p hkask-cns -- -D warnings`. Could be fixed as a separate cleanup pass. |

---

## Redaction Summary

No sensitive data found in this session. 0 redactions applied.