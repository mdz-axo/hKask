---
title: "MCP Fleet Test-Seam Audit and Follow-Up Resolution"
audience: [architects, security-reviewers, developers, agents]
last_updated: 2026-07-17
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# MCP Fleet Test-Seam Audit and Follow-Up Resolution

**Diataxis type:** Explanation / status · **Trigger:** follow-up questions raised by the `hkask-mcp-filesystem` adversarial review (see [`reference/mcp-servers/filesystem.md`](../reference/mcp-servers/filesystem.md)).

This document records the investigation of five follow-up questions raised after the filesystem review and resolves the design questions they pose. It is evidence-anchored: every claim is verified against the codebase in the same session.

## 1. Q1 — Is the test-seam misalignment a fleet-wide pattern?

**Verdict: yes, fleet-wide.** The filesystem review's root cause (contract tests exercised `sandbox_path` in isolation, asserting zero tool-behavior contracts, so three logic bugs shipped) is the norm, not the exception.

### Audit method

For each of the 15 built-in MCP servers, checked (a) presence of a `tests/` directory and (b) whether any test calls a tool method through its public `Parameters<T>` seam (the only way to exercise a tool's actual contract).

| Server | Has `tests/` | Tool-behavior tests (`Parameters(`) |
|--------|:-----------:|:------------------------------------:|
| filesystem | ✅ | ✅ (added during the review — 24 tests) |
| kata-kanban | ✅ | ✅ |
| codegraph | ✅ | ❌ helper/service seams only |
| communication | ✅ | ❌ helper seams only |
| condenser | ✅ | ❌ helper seams only |
| curator | ✅ | ❌ helper seams only |
| docproc | ✅ | ❌ helper seams only |
| media | ✅ | ❌ helper seams only |
| memory | ✅ | ❌ helper seams only |
| replica | ✅ | ❌ helper seams only |
| research | ✅ | ❌ helper seams only |
| skill | ✅ | ❌ helper seams only |
| training | ✅ | ❌ helper seams only |
| scenarios | ❌ | ❌ no tests at all |
| companies | ❌ | ❌ no tests at all |

**Result:** only 2 of 13 tested servers verify tool behavior through the public tool methods; 2 servers have no tests at all. 11 servers test only helper/service seams — the exact gap that hid the filesystem bugs.

### Reframe: the risk is unverified contracts, not panic-density

A panic-pattern scan (`bare unwrap()` / `panic!` / `todo!` / `unimplemented!` in production tool code) was run to prioritize. The result overturned the naive hypothesis: production tool code across the fleet is largely panic-free — the apparent hotspots (media 34, memory 12, companies 3) are almost entirely `unwrap_or*` (safe) or `.expect()`/`.unwrap()` inside `#[cfg(test)]` modules (acceptable in tests).

This is the important nuance: **the filesystem bugs were not `unwrap()` panics.** They were slice-indexing on unvalidated input (`lines[start..end]`, `&str[..max_bytes]`), canonicalize-on-non-existent, silent no-ops, and error-swallowing — logic bugs that a panic-grep cannot see and that only a tool-behavior contract test can catch. So the fleet risk is **unverified tool contracts**, not panic-prone code. The remediation is contract coverage, not "remove unwraps."

### Recommendation

1. Establish a testing standard (recorded in [`reference/mcp-servers/README.md`](../reference/mcp-servers/README.md) §Common Patterns): every MCP server must include tool-behavior contract tests that invoke tools through their public `Parameters<T>` seam, covering at least: happy path, invalid input, boundary/edge cases, and error-specificity. Helper-seam-only tests are necessary but not sufficient.
2. Prioritize remediation by **contract complexity and blast radius**, not panic count: servers whose tools take rich structured input or mutate durable state (scenarios, companies, docproc, training, memory) before servers with thin pass-through tools.
3. Use the filesystem contract suite as the exemplar pattern (`test_server` helper + `Parameters(...)` + `parse_content`/`error_message` helpers + one `#[tokio::test]` per contract).

## 2. Q2 — Is `shell_exec` "OCAP-governed" accurate or aspirational?

**Verdict: accurate, with one architectural nuance to document.** OCAP is enforced — at the dispatcher membrane, not at the server.

### Evidence

[`crates/hkask-mcp/src/dispatch.rs`](../../crates/hkask-mcp/src/dispatch.rs) is explicit:

- `McpDispatcher::invoke` routes every call through a `GovernedTool<RawMcpToolPort>` membrane; if the membrane is absent it returns *"GovernedTool membrane not configured — all tool invocations require governance."*
- The membrane performs OCAP verification via a `DelegationToken` per call (`CapabilityDenied`, `EnergyBudgetExceeded` are real error kinds).
- `RawMcpToolPort` (the actual `runtime.call_tool` path) is documented *"Never expose this port directly to agents."*
- The module doc: *"The dispatcher is the transport pipe; the governed tool membrane is the security property."*

The MCP server binary (e.g. `hkask-mcp-filesystem`) does **not** re-check capabilities per call — by design. Governance is a membrane concern; the server is transport. This is the canonical hKask split.

### Threat-surface narrowing

[`crates/hkask-cli/src/commands/serve.rs`](../../crates/hkask-cli/src/commands/serve.rs) defines `API_EXCLUDED = ["filesystem", "curator", "kanban", "skill"]` — the filesystem server is **deliberately excluded from the headless HTTP API surface**. So `shell_exec` is reachable only via the local stdio MCP path, i.e. by an hKask agent in the runtime that already holds a `shell_exec` capability token granted through the consent layer.

### Resolution of the "unrestricted command string" concern

The command string is not confined to `project_root`; only `cwd` is. This is **consistent with OCAP, not a violation of it**: a capability token grants the *authority* to run a shell; the holder of that capability is the trusted actor who exercises it. The safeguard is not command-string filtering at the tool — it is **consent-gated capability granting** (Magna Carta P2, Affirmative Consent). The right follow-up is therefore a *policy* question (when is `shell_exec` capability granted, and to whom), not a code change to `shell_exec`. The filesystem reference page is updated to state this precisely so the "OCAP-governed" label is not mistaken for "command-string-confined."

## 3. Q3 — Are operation-level Regulation spans consumed, or dead signal?

**Verdict: tracing-only; not dead, but not a regulation input. Keep, and document accurately.**

A search for consumers of the operation verbs (`file.read`, `file.written`, `file.deleted`, `command.completed`, `command.failed`, `path.rejected`) found matches only in:

- the filesystem server itself (`emit_cns` call sites and the doc comment),
- `crates/hkask-types/src/regulation.rs` and `event.rs` (the `ToolSubsystem::Filesystem` enum + canonical namespace registration).

No code path reads the operation verb for regulation. The two emission paths are:

| Path | Mechanism | Consumed by |
|------|-----------|-------------|
| Framework `execute_tool` | `ToolSpanGuard` → `tracing::info!(target:"reg.tool", …)` **+** `record_via_daemon` (semantic memory) | tracing logs **and** the daemon (regulation/memory) |
| Server `emit_cns` | `RegulationSpan::Tool{Filesystem}.emit(verb)` → `tracing::info!` | tracing logs only |

So `emit_cns` adds operation-verb granularity to the **tracing substrate** (humans, log aggregators) but does not feed programmatic Regulation regulation. The framework span already records outcome (`ok`/`error`) and feeds the daemon. Conclusion: the dual emission is intentional tracing granularity, not cruft — but it is *not* a regulation signal. This validates the earlier essentialist call ("do not delete `emit_cns` without evidence of disuse") and sharpens the documentation: the filesystem reference page now states operation spans are success-path tracing events, while the framework span is the regulation/memory path.

## 4. Q4 — Does the TOCTOU in `sandbox_path` matter under multi-agent use?

**Verdict: acceptable within a single trusting workspace; the caveat is sharpened.**

The filesystem server reads `HKASK_PROJECT_ROOT` once at startup ([`mcp-servers/hkask-mcp-filesystem/src/main.rs`](../../mcp-servers/hkask-mcp-filesystem/src/main.rs)) and serves stdio for the life of the process. One process serves one workspace; multiple agents in that workspace share it. The canonicalize-then-act TOCTOU in `sandbox_path` is therefore a *single-workspace* concern: it is exploitable only if agents within the same workspace are mutually adversarial (one swaps a symlink while another's check races). Under hKask's model, agents in a workspace share one user's sovereignty and are cooperating, so the TOCTOU is low-risk — but the documentation now says exactly that, rather than the vaguer "single-user" phrasing, so a future multi-tenant workspace design isn't misled.

## 5. Q5 — Do any consumers depend on the old `fs_read`/`shell_exec` wire shape?

**Verdict: no in-repo consumers; low risk.**

A search for in-repo parsers of the filesystem tool response fields (`"content"`, `"range"`, `"truncated"`, `"files_skipped"`, `"stdout"`) found matches only in the filesystem server and its own tests. The filesystem MCP server is consumed by LLM agents over MCP, not by in-repo Rust code. The review's wire changes are:

- `fs_search` adds `files_skipped` (additive; old clients ignore unknown fields),
- `fs_read` now returns `range: "3-"` / `"-2"` for partial reads (previously `null`; agents read `range` flexibly),
- `shell_exec` `truncated` now reflects stdout **or** stderr (previously stdout only; strictly more accurate).

No in-repo consumer breaks. External agent clients read these fields flexibly. Risk is low; no action beyond recording it here.

## Summary of actions taken

| Follow-up | Resolution | Artifact |
|-----------|------------|----------|
| Q1 fleet audit | Finding recorded + testing standard added | this doc; [`reference/mcp-servers/README.md`](../reference/mcp-servers/README.md) §Common Patterns |
| Q2 OCAP threat model | Sharpened: membrane-gated, local-stdio-only, command trusted to capability holder | [`reference/mcp-servers/filesystem.md`](../reference/mcp-servers/filesystem.md) §Security model |
| Q3 Regulation consumption | Sharpened: operation spans are tracing-only; framework span is the regulation path | [`reference/mcp-servers/filesystem.md`](../reference/mcp-servers/filesystem.md) §Security model notes |
| Q4 TOCTOU | Sharpened to "single trusting workspace" framing | [`reference/mcp-servers/filesystem.md`](../reference/mcp-servers/filesystem.md) §Security model notes |
| Q5 wire shape | No in-repo consumers; recorded here | this doc |

## Open items deferred (require a human decision, not code)

- **`fs_edit` chaining semantics** (from the prior review): keep sequential chaining (current) or switch to non-interacting edits. Documented either way; awaiting intent.
- **`shell_exec` capability-granting policy**: when is the `shell_exec` capability granted, and is consent-gating enforced for it? This is the actual security lever for the unrestricted command string — a policy/consent question, not a `shell_exec` code change.
- **Fleet remediation**: add tool-behavior contract tests to the 11 helper-seam-only servers + the 2 untested servers, prioritized by contract complexity.

## Cross-links

- [Filesystem Server Reference](../reference/mcp-servers/filesystem.md) — sandbox model, security model notes, DIAG-RF-003
- [MCP Server Registry](../reference/mcp-servers/README.md) — 15 servers, common patterns, testing standard
- [Regulation Span Registry](../reference/regulation-spans.md) — `RegulationSpan::Tool` and `ToolSubsystem::Filesystem`