---
title: "ADR-060: Memory Recall Deduplication — Single-Layer, Per-Surface Rendering"
audience: [architects, developers]
last_updated: 2026-07-21
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [curation]
---

# ADR-060: Memory Recall Deduplication — Single-Layer, Per-Surface Rendering

**Date:** 2026-07-21  
**Status:** Active  
**Supersedes:** None (corrects documentation that anticipated an unbuilt module)

## Context

`hkask-memory` historically documented a "Two-Layer DRY System" (in
`crates/hkask-memory/src/lib.rs`) and a "three-layer DRY system" (in
`crates/hkask-memory/src/recall_dedup.rs`) in which:

- **Layer 1** was recall-time deduplication in `recall_dedup.rs`
  (BLAKE3 hash of canonical entity-attribute-value content).
- **Layer 2** was "prompt assembly dedup" in
  `hkask-templates/src/context_assembly.rs`.

**Layer 2 was never built.** `hkask-templates/src/context_assembly.rs` does
not exist. The two documents also disagreed on whether the system had two
or three layers. Meanwhile the actual rendering of recalled memories — the
join of semantic + episodic results into a prompt string or a JSON payload —
was implemented independently at each consuming surface:

| Surface | Rendering strategy |
|---|---|
| `hkask-services-chat` (`MemoryService::recall_memory`) | `format!("{}\n\n{}", s, e)` string concat for prompt injection |
| `hkask-mcp-memory` (`MemoryServer::memory_recall`) | JSON object with `semantic` and `episodic` keys for tool callers |
| `hkask-api` (`routes/memory.rs::memory_recall`) | `MemoryRecallResponse` struct with `episodic: Vec<EpisodeResponse>` and `semantic: Option<Vec<SemanticTripleResponse>>` |

**Problem Statement:** Two documents in `hkask-memory` claimed a canonical
Layer 2 module that did not exist, and disagreed on the layer count. The
rendering each surface actually performed was unowned and triplicated.

**Stakeholders:** `hkask-memory` maintainers, chat service authors, MCP
server authors, HTTP API authors, anyone reading the memory docs to
understand the dedup story.

**Constraints:**

- **P5 (Essentialism):** modules must earn their existence via the
  deletion test — a module with a single consumer is a pass-through.
- **P7 (Evolutionary Architecture):** types and modules emerge from
  usage, not speculation.
- **P8 (Semantic Grounding):** documentation must describe the system
  as it actually is, not as it was anticipated to be.
- **Deep-module discipline:** a module with one consumer that adds no
  behavior beyond a direct call is a pass-through and should be deleted.

## Decision

**Chosen Approach:** Memory deduplication is a **single-layer** system.
Rendering is **each surface's responsibility** — there is no shared
context-assembly module.

1. **Recall dedup (Layer 1) is the only dedup layer.** It lives in
   `hkask-memory/src/recall_dedup.rs` and runs at recall time inside
   `EpisodicMemory::query_for_deduped` and `SemanticMemory::query_deduped`.
   It is a BLAKE3 hash over canonical entity-attribute-value content,
   metadata-independent, first-seen-wins.

2. **Rendering is unowned by design.** Each consuming surface (chat
   service, MCP server, HTTP API, TUI) joins and serializes recalled
   memories in the shape its own consumer needs. The chat service joins
   into a prompt string; the MCP server joins into a JSON object; the
   HTTP API joins into a typed response struct. These are not
   duplicates of a missing module — they are three different
   serializations of one logical recall, optimized for three different
   consumers.

3. **The recall itself is shared** via the `EpisodicStoragePort` and
   `SemanticStoragePort` traits (see ADR-042). The seam between
   "recall" (shared, in `hkask-memory`) and "rendering" (per-surface,
   in each consumer) is the port boundary. That is the right seam to
   share; rendering is not.

**Alternatives Considered:**

1. **Build the missing Layer 2** as `hkask-memory::context_assembly` —
   Rejected by the deletion test: the chat service is the only prompt
   assembler. A module with a single consumer that joins two strings is
   a pass-through. The MCP server and HTTP API do not produce prompt
   strings at all — they serialize to JSON — so they would not consume
   a prompt-assembly module. Building it would create a single-consumer
   module dressed up as a shared layer, violating P5 and P7. We would
   also have to invent a second consumer to justify the module, which is
   speculative architecture.

2. **Build a generic `PairedRecall` helper that returns both recall
   vectors and let each surface render** — Rejected as the *subject* of
   this ADR (it is a rendering concern, not a dedup concern), but
   retained as a *future* option if a second prompt assembler appears.
   See "Future Evolution" below.

3. **Leave the docs as-is** — Rejected: P8 (Semantic Grounding) prohibits
   documenting modules that do not exist as canonical. The
   inconsistency between "two-layer" and "three-layer" was an
   additional P8 violation.

**Rationale:** The deletion test (Ousterhound's *A Philosophy of Software
Design*[^ousterhout]) is decisive. Layer the question: *if we delete the
proposed `context_assembly` module, what behavior vanishes?* Answer: none
— it does not exist, and each surface already renders for itself. *If we
build it and then delete it, what behavior vanishes?* Answer: the chat
service's `format!("{}\n\n{}", s, e)` — a one-line join that is trivial to
inline. A module whose deletion loses only a one-line join is a
pass-through and fails the deletion test.

The three rendering strategies are not duplicates of a missing primitive
(constraint C4: "repetition is missing primitive"). They are three
serializations of one recall into three different consumer shapes
(string, JSON object, typed struct). Sharing the recall via ports is the
correct response to the repetition; sharing the rendering would be a
pass-through abstraction.

## Consequences

### Positive

- **Documentation is accurate.** `hkask-memory` no longer references a
  module that does not exist. The layer-count disagreement is resolved.
- **No premature abstraction.** The system does not carry a
  single-consumer module dressed as a shared layer (P5, P7).
- **Each surface optimizes its own rendering.** The chat service can
  tune prompt formatting; the MCP server can tune JSON shape; the HTTP
  API can tune its response struct — without coordinating through a
  shared layer that would constrain all three.
- **The seam is explicit.** "Recall is shared via ports; rendering is
  per-surface" is now a written rule, not an implicit default.

### Negative

- **If a second prompt assembler appears**, the join logic in
  `MemoryService::recall_memory` would need to be extracted into a
  shared helper at that point. This is an accepted YAGNI cost: we pay
  it when the second consumer materializes, not before.
- **Reviewers must distinguish "rendering duplication" from "logic
  duplication."** Three surfaces joining recalled memories into three
  shapes is *not* a code-smell to fix; three surfaces re-implementing
  the same salience scorer *is*. This ADR does not license the latter.

### Neutral

- The "DRY system" framing is retired. `recall_dedup` is now described
  as "recall deduplication," not "Layer 1 of an N-layer system."
- `hkask-memory`'s public surface is unchanged. This ADR documents
  existing behavior; it does not add or remove code.

## Future Evolution

This decision is reversible and conditional on consumer count. The
trigger for re-evaluation is explicit:

> **When a second prompt assembler materializes** (a second surface that
> needs to inject recalled memories into an LLM prompt as a string),
> extract the join logic from `MemoryService::recall_memory` into a
> `hkask-memory::context_assembly` module at that point. The module
> earns its existence only when its deletion would cause complexity to
> reappear in two callers.

Until that trigger fires, the per-surface rendering pattern stands.
A `PairedRecall` helper that returns `(Vec<RecalledSemantic>,
Vec<RecalledEpisode>)` is a separate, optional cleanup (see the memory
cleanup plan) and is not governed by this ADR.

## Compliance

### Constraint-Driven Design Principles

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P5** (Essentialism) | ✅ | No single-consumer module is created. The proposed Layer 2 would have failed the deletion test. |
| **P7** (Evolutionary Architecture) | ✅ | Modules emerge from usage. The second-consumer trigger is the explicit re-evaluation criterion. |
| **P8** (Semantic Grounding) | ✅ | Documentation now describes the system as it is. The phantom `context_assembly.rs` reference is removed. |

### Constraints

| Constraint | Compliance | Evidence |
|-----------|-----------|----------|
| **C4** (Repetition is missing primitive) | ✅ | The three rendering strategies are *not* repetition of a missing primitive — they are three serializations of one recall into three consumer shapes. The recall itself is shared via ports (ADR-042). |
| **C7** (Divergence must yield) | ✅ | The doc divergence ("two-layer" vs "three-layer") is resolved to one. |

## Verification

```bash
# 1. The phantom module reference is gone from the memory crate.
grep -rn "context_assembly" crates/hkask-memory/
# Expected: no matches

# 2. The layer-count claims are gone from the memory crate.
grep -rniE "two-layer|three-layer|layer DRY" crates/hkask-memory/
# Expected: no matches

# 3. recall_dedup.rs no longer claims to be "Layer 1 of an N-layer system."
grep -n "Layer 1 of" crates/hkask-memory/src/recall_dedup.rs
# Expected: no matches

# 4. This ADR exists.
test -f docs/architecture/ADRs/ADR-060-memory-recall-dedup-single-layer.md
# Expected: exit 0

# 5. No code changed — docs-only ADR.
cargo check --workspace
# Expected: same result as before this ADR
```

**Expected Results:**

- `crates/hkask-memory/` no longer references `context_assembly` or an
  N-layer DRY system.
- `recall_dedup.rs` describes itself as "recall deduplication," not
  "Layer 1 of a multi-layer system."
- No code changes; build and tests unaffected.

## Related Documents

- [ADR-042: Port Trait Location — Promotion Rule](ADR-042-port-promotion-rule.md)
  — defines where the shared `EpisodicStoragePort` / `SemanticStoragePort`
  traits live. This ADR depends on that seam: recall is shared via ports;
  rendering is per-surface.
- [`crates/hkask-memory/src/recall_dedup.rs`](../../../crates/hkask-memory/src/recall_dedup.rs)
  — the single dedup layer this ADR governs.
- [`crates/hkask-memory/src/lib.rs`](../../../crates/hkask-memory/src/lib.rs)
  — crate-level docs updated to reflect this decision.
- [`docs/reference/api-reference.md`](../../reference/api-reference.md) —
  memory module table updated to reflect this decision.

## References

[^ousterhout]: Ousterhout, J. (2018). *A Philosophy of Software Design.*
  Yaknyam Press — the "deep modules" and "deletion test" framework that
  governs when a module earns its existence. This ADR applies the
  deletion test to the proposed Layer 2 and rejects it as a
  pass-through.

---

*ℏKask v0.31.0 — A Sovereign Chat Client for Human Users*
*Decisions are the atoms of architecture.*