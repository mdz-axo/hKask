---
title: "ADR-048: Regulation Type System Decomposition"
audience: [architects, developers]
last_updated: 2026-07-05
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# ADR-048: Regulation Type System Decomposition

**Date:** 2026-07-05  
**Status:** Active  
**Supersedes:** None (new decision)

## Context

**Problem Statement:** The `CnsSpan` enum in `hkask-types` had grown to 72+ variants covering every domain subsystem (wallet, federation, contracts, QA, metrics, deploy, backup, ACP, curator, etc.), creating a monolithic type that violated substrate isolation and forced every domain crate to know about every other domain's observability concerns.

**Stakeholders:** All crate maintainers; Regulation observability consumers; Curator Agent.

**Constraints:**
- `hkask-types` is the substrate crate — must not depend on domain crates
- ν-event namespace strings must remain stable for backward compatibility
- Compile-time namespace validity is required (no runtime string typos)
- The `ObservableSpan` trait already existed but was underutilized

## Decision

**Chosen Approach:** Decompose the monolithic `CnsSpan` into a layered architecture:

1. **Core (hkask-types):** `CnsSpan` reduced to 7 cross-cutting variants (Tool, Inference, AgentPod, Gas, Curation, SelfHeal, MemoryEncode) — spans constructed in 2+ crates from different dependency domains.

2. **Domain enums:** Each domain crate defines its own span enum implementing `ObservableSpan`:
   - `WalletSpan` (hkask-wallet, 14 variants)
   - `FederationSpan` (hkask-federation, 19 variants)
   - Subsystem enums in hkask-regulation: `ContractSpan`, `SeamSpan`, `SloSpan`, `QaSpan`, `AcpSpan`, `ClassifySpan`, `InfraSpan`

3. **Single registry:** `CANONICAL_NAMESPACES` (133 entries in event.rs) is the authoritative list of all valid namespace strings. `SpanNamespace::new()` and `::parse()` validate against it. `SpanNamespace::from_observable()` bridges domain enums to the validated namespace.

4. **Non-span types extracted:** `RetryConfig` → `hkask-types/src/retry.rs`; `SeamCoverage`/`SeamInventory` → `hkask-regulation/src/seam_types.rs`; `SloDefinition`/`SloEvaluation`/`SloSeverity` → `hkask-regulation/src/slo_types.rs`. `reg.rs` reduced from 965 to 498 lines (5 types).

5. **Trait simplification:** Removed `Clone` and `FromStr` supertraits from `ObservableSpan`, making it dyn-compatible. Fixed doc that falsely claimed dyn-compatibility.

**Alternatives Considered:**
1. **Single `Domain(&'static str)` variant** — Keeps CnsSpan as one enum with an extensible string variant. Simpler (zero new types) but loses per-domain compile-time type safety. Rejected: the user explicitly wanted "subtypes in the crates."
2. **Keep monolithic CnsSpan** — No change. Rejected: violates substrate isolation principle; 72+ variants in a substrate crate.
3. **Make CANONICAL_NAMESPACES auto-generated** — Build script collects all `ObservableSpan::as_str()` outputs. Rejected: would couple substrate compilation to domain crates.

**Rationale:** The `ObservableSpan` trait was designed for this decomposition from day one (its doc anticipated "FederationSpan" and "WalletSpan" as future domain enums). The Regulation membrane routes by `SpanCategory` string prefixes, not variant identity — so splitting doesn't break cybernetic homeostasis. The Regulation regulator's variety counter now has richer coverage (133 namespaces vs. ~72 previously filtered through `CnsSpan::from_str()`).

## Consequences

### Positive
- Substrate crate reduced from 29 public types to 5 (essentialist G2 pass)
- Domain crates own their observability — adding a wallet span no longer requires touching core types
- `ObservableSpan` is now dyn-compatible, enabling `Box<dyn ObservableSpan>` for runtime span dispatch
- All 79 production namespace strings registered in CANONICAL_NAMESPACES with zero duplicates
- Regression tests (`*_namespaces_are_canonical`) verify every span enum's strings are registered

### Negative
- Adding a new span requires 3 changes (enum variant + CANONICAL_NAMESPACES + call site) vs. 1 before (just CnsSpan variant)
- `CnsSpan::from_str()` no longer accepts domain namespace strings — any code using it for parsing must switch to `SpanNamespace::parse()`
- 7 new files in hkask-regulation (one per subsystem span enum) — more files to navigate

### Neutral
- CANONICAL_NAMESPACES has 57 entries with no typed enum producer (forward-compatible: registered for future domain enums)
- `SpanNamespace::from_observable()` and `From<CnsSpan>` are structurally identical — deduplicated through shared `from_str_validated()` helper

## Compliance

### Constraint-Driven Design Principles

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P1** (No trait without two consumers) | ✅ | `ObservableSpan` has 10 impls (CnsSpan, WalletSpan, FederationSpan, 7 subsystem spans) |
| **P3** (No module directory without encapsulation) | ✅ | Each span file encapsulates a single subsystem's observability |
| **P6** (Delete stubs, don't publish) | ✅ | 65 removed CnsSpan variants deleted, not deprecated |
| **P7** (Prefer deletion over deprecation) | ✅ | Old CnsSpan variants removed entirely; domain enums replace them |

### Constraints

| Constraint | Compliance | Evidence |
|-----------|-----------|----------|
| **C4** (Repetition is missing primitive) | ✅ | `from_str_validated()` extracted as the single validation path for both `From<CnsSpan>` and `from_observable()` |
| **C7** (Divergence must yield) | ✅ | Split brain between `CnsSpan::from_str()` and `CANONICAL_NAMESPACES` resolved — single registry |

## Verification

```bash
# Workspace compilation
cargo check --workspace

# Regulation tests (174 across types + regulation)
cargo test -p hkask-types -p hkask-regulation --lib

# Namespace audit
grep -c '"regulation\.' crates/hkask-types/src/event.rs  # 133 CANONICAL entries
grep -rn 'CnsSpan::' crates/ --include='*.rs' | grep -v 'reg_span\|reg.rs\|Tool\|Inference\|AgentPod\|Gas\|Curation\|SelfHeal\|MemoryEncode'  # should find nothing
```

**Expected Results:**
- Workspace compiles cleanly (pre-existing hkask-storage-* issues are independent)
- 51 tests pass in hkask-types, 123 in hkask-regulation
- Zero stray references to removed CnsSpan variants
- 133 CANONICAL_NAMESPACES entries, alphabetically sorted, zero duplicates

## Related Documents

- `crates/hkask-types/src/observable_span.rs` — `ObservableSpan` trait definition
- `crates/hkask-types/src/event.rs` — `CANONICAL_NAMESPACES` registry, `SpanNamespace`, `Span`
- `crates/hkask-types/src/reg.rs` — Core `CnsSpan` enum (7 variants)
- `crates/hkask-regulation/src/` — Domain span enums (contract_span, seam_span, slo_span, qa_span, acp_span, classify_span, infra_span)
- `crates/hkask-wallet/src/reg_span.rs` — `WalletSpan`
- `crates/hkask-federation/src/reg_span.rs` — `FederationSpan`

---

*ℏKask - A Minimal Viable Container for UserPods — v0.31.0*
*Decisions are the atoms of architecture.*
