---
title: "ADR-053: Corpus Pipeline Input Guard Toggle"
audience: [architects, developers]
last_updated: 2026-07-19
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [curation, trust]
---

# ADR-053: Corpus Pipeline Input Guard Toggle

**Date:** 2026-07-16
**Status:** Active (amended 2026-07-19)
**Supersedes:** —

## Context

The `hkask-guard` content safety guard enforces mandatory input/output scanning at every LLM boundary (OWASP LLM01/02/04/06). Its input pipeline (`TokenLimit` → `RoleOverride` → `BanSubstrings` → `Deobfuscate`, first-hit) is designed to refuse prompt-injection and role-override attacks before model invocation. The `ContentGuard::mandatory()` constructor is intentionally non-configurable — core scanners cannot be disabled (P3.1 floor).

The docproc corpus curation pipeline (`docproc_tag_chunks`, `docproc_embed`, `docproc_extract_triples`, `docproc_consolidate_chunks`, `docproc_generate_qa`, `docproc_generate_qa_batch`) feeds *operator-curated, public-domain research literature* into LLM prompts. That source text routinely contains phrases the `RoleOverride` scanner flags as attacks:

- "Imagine you are randomly drawing balls from an urn" (Fermi problems)
- "You are a superforecaster" (Tetlock)
- "Act as if the principle were universal" (Kantian ethics)

These are content, not attacks. During the Capabilities Researcher (John Brooks) corpus build, the input guard rejected ~20% of QA-generation prompts on this basis — a false-positive rate that materially reduced QA yield without providing meaningful security, because the pipeline's inputs are not untrusted user input.

An operator-set environment variable, `HKASK_ENABLE_CONTENT_GUARD=false`, already existed in `corpus/env/pipeline.env` but was **dead code**: nothing in the guard or docproc crates referenced it. The operator's explicit configuration was silently ignored.

**Problem Statement:** The content guard's input pipeline produces ~20% false positives on curated research literature, and the operator's existing opt-out env var is non-functional.

**Stakeholders:** Corpus pipeline operators; interactive chat/MCP users (who must remain protected); security reviewers.

**Constraints:**

- P3.1 (Social Generativity): core content-safety controls are the floor — the guard *object* must remain non-configurable-off.
- P4 (Clear Boundaries): the inference boundary must remain explicit.
- P1 (User Sovereignty): the operator's explicit `HKASK_ENABLE_CONTENT_GUARD=false` must be honored.
- The output guard (secret stripping) is load-bearing: model output enters shared memory and must never carry credentials, regardless of input trust.

## Decision

Distinguish **guard object always-on** from **corpus pipeline input invocation operator-controlled**.

- The `ContentGuard` continues to be constructed via `mandatory()` and remains non-configurable at the crate level. Its public contract (FR-GD1) is unchanged.
- A process-local `INPUT_GUARD_ENABLED: LazyLock<bool>` in `hkask-mcp-docproc::tools::semantic` reads `HKASK_ENABLE_CONTENT_GUARD` once (unset or any value other than `false`/`0`/`off`/`no` → enabled). Four docproc corpus `scan_input` call sites are gated on `*INPUT_GUARD_ENABLED`.
- The **output** guard (`scan_output`) is invoked unconditionally at every docproc LLM boundary — secrets are always stripped before shared memory.
- The **interactive classifier** path in `hkask-services-runtime` (`classify_one`, `extract_triples_one`) does **not** honor the flag and remains unconditionally guarded, preserving FR-GD4/FR-GD5.

### Amendment (2026-07-19): `docproc_tag_chunks` is always-on

The `docproc_tag_chunks` boundary is now **non-disableable** — it calls `GUARD.scan_input()` unconditionally, ignoring `INPUT_GUARD_ENABLED`. This reverses the original decision for this one call site.

**Rationale:** The tagging boundary is the first LLM call on raw chunk text (PDFs, HTML, plain text). The original ADR's threat model assumed "operator-curated literature," but the pipeline ingests arbitrary document files, and a poisoned PDF can contain prompt-injection text that reaches the LLM unfiltered. Combined with the brace-balanced JSON extraction fix (RR-0017) and the `validate_ontology_tags` schema enforcement (RR-0016), the always-on input guard closes the C1/C2/M2 attack chain identified in the adversarial review. The other four call sites (`embed`, `extract_triples`, `consolidate`, `generate_qa`, `generate_qa_batch`) process already-tagged data or LLM-produced JSON, so the original operator-controlled toggle remains appropriate for them.

**Impact on false positives:** The tagging prompt is a fixed template with the chunk text in the `{{ text }}` slot. The false-positive phrases ("Imagine you are...", "You are a superforecaster") appear in the chunk text and will trigger the `RoleOverride` scanner. Operators who hit this on the tagging boundary should pre-filter the corpus or whitelist specific patterns in the guard config — not disable the guard.

**Chosen Approach:** Operator-controlled input-scan gating for 4 of 5 docproc corpus call sites; `docproc_tag_chunks` is always-on; output guard and interactive boundaries untouched.

**Alternatives Considered:**

1. *Whitelist known-safe patterns ("Imagine you are", etc.) in `RoleOverride`* — rejected: brittle, does not generalize across corpora, and weakens the guard for interactive use where the same phrases can be genuine attacks.
2. *Scan only the user portion, not the system prompt* — rejected: the false positives originate in the chunk *text* (the user portion), so this would not fix the issue and requires invasive system/user splitting.
3. *Accept the ~20% rejection rate* — rejected as the default: it discards a material fraction of curated training data and ignores the operator's existing (dead) opt-out.
4. *Make the guard crate itself read the env var* — rejected: blurs the guard's "always active" contract and pushes corpus-pipeline policy into a security primitive. The pipeline decides whether to *invoke* input scanning; the guard stays agnostic.

**Rationale:** The guard's input pipeline exists to stop untrusted user input from hijacking an interactive agent's system prompt. A batch corpus curation pipeline over curated literature has no untrusted user input — the threat model does not apply. Letting the operator skip input scanning there honors P1 without weakening the P3.1 floor (the guard object, the output guard, and interactive boundaries all remain mandatory). This is the smallest change that makes the operator's existing configuration functional. The `docproc_tag_chunks` exception (amendment above) restores the guard on the one boundary where raw, untrusted document text reaches the LLM.

## Consequences

### Positive

- Operator's `HKASK_ENABLE_CONTENT_GUARD=false` now takes effect on 4 of 5 corpus call sites; QA yield on research literature recovers the ~20% previously lost to false positives.
- `docproc_tag_chunks` is always guarded, closing the C1/C2/M2 attack chain on the tagging boundary.
- Output secret-stripping remains a hard floor — credentials never enter shared memory regardless of the input setting.
- Interactive chat/MCP and classifier boundaries remain fully guarded; no regression in the threat model that motivated the guard.
- The decision is auditable via a single env var read once per process.

### Negative

- A misconfigured operator could disable input scanning for `embed`/`extract_triples`/`consolidate`/`generate_qa`/`generate_qa_batch` on a corpus that later includes untrusted text. Mitigated by: (a) default-on, (b) output guard still active, (c) scope limited to the corpus pipeline, not interactive boundaries, (d) the tagging boundary (first LLM call on raw text) is always-on.
- Two guard invocation regimes (corpus vs interactive) now exist; contributors must understand the boundary. Mitigated by this ADR and the `install-and-configure.md` note.
- The `docproc_tag_chunks` boundary may reject ~20% of chunks that contain role-override phrases in their text. Operators must pre-filter or whitelist.

### Neutral

- `INPUT_GUARD_ENABLED` is a `LazyLock<bool>` read once per process; changing the env var requires a process restart (consistent with per-`kask mcp invoke` process semantics).

## Compliance

### Constraint-Driven Design Principles

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P1** (No trait without two consumers) | ✅ | `INPUT_GUARD_ENABLED` consumed by 4 docproc corpus call sites |
| **P5** (No feature flag without activator) | ✅ | Env var is the activator; default-on; documented in `install-and-configure.md` |
| **P7** (Prefer deletion over deprecation) | ✅ | No deprecated API; dead env var is wired rather than removed |

### Magna Carta

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P1** (User Sovereignty) | ✅ | Operator's explicit `HKASK_ENABLE_CONTENT_GUARD=false` is honored on 4 of 5 call sites |
| **P3.1** (Social Generativity — floor) | ✅ | Guard object + output guard + interactive boundaries + tagging boundary remain always-on |
| **P4** (Clear Boundaries) | ✅ | Corpus-vs-interactive boundary is explicit and documented |

## Verification

```bash
# Guard crate unchanged and green
cargo test -p hkask-guard

# Corpus pipeline compiles clean under strict clippy
cargo clippy -p hkask-mcp-docproc -p hkask-guard -- -D warnings

# Four corpus scan_input sites are gated on the toggle (tagging is always-on)
grep -rn "INPUT_GUARD_ENABLED" mcp-servers/hkask-mcp-docproc/src/

# The tagging boundary is always-on (no INPUT_GUARD_ENABLED check)
grep -A2 "scan_input" mcp-servers/hkask-mcp-docproc/src/tools/tagging/ops.rs

# The env var is now referenced (was previously dead)
grep -rn "HKASK_ENABLE_CONTENT_GUARD" mcp-servers/ corpus/env/
```

**Expected Results:**

- `cargo test -p hkask-guard`: 6 passed, 0 failed.
- `cargo clippy`: no warnings.
- `grep INPUT_GUARD_ENABLED`: 7 matches — 1 definition + 4 call sites + 2 imports (down from 8 in the original ADR — the tagging call site no longer checks the flag).
- `grep HKASK_ENABLE_CONTENT_GUARD`: matches in `semantic/mod.rs` (read) and `pipeline.env` (operator setting).
- `grep scan_input tagging/ops.rs`: shows unconditional `GUARD.scan_input(&prompt)` without an `INPUT_GUARD_ENABLED` guard.

## Related Documents

- [ADR-050 — Ontology-Anchored Embedding](ADR-050-ontology-anchored-embedding.md) (same corpus pipeline)
- [Install and Configure — Content Guard Configuration](../../how-to/install-and-configure.md)
- [Functional Specification §3.20 — Content Safety Guard](../core/FUNCTIONAL_SPECIFICATION.md)
- [RR-0016 — Ontology tagging validation](../../../security/regressions/RR-0016.yaml)
- [RR-0017 — Brace-balanced JSON extraction](../../../security/regressions/RR-0017.yaml)

## References

[^owasp-llm]: OWASP Foundation. (2025). *OWASP Top 10 for LLM Applications.* <https://owasp.org/www-project-top-10-for-large-language-model-applications> — LLM01 (Prompt Injection) and LLM06 (Sensitive Information Disclosure) threat models that define the guard's input/output split.

[^tetlock-superforecasting]: Tetlock, P., & Gardner, D. (2015). *Superforecasting: The Art and Science of Prediction.* Crown — source of the "you are a superforecaster" framing that exemplifies the false-positive class addressed here.

---

*ℏKask - A Minimal Viable Container for UserPods — v0.31.0*