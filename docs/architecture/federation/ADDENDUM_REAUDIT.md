---
title: "Federation Design — Re-Audit After M1–M3 Consolidation"
audience: [architects, developers]
last_updated: 2026-06-22
version: "0.30.0+fed"
status: "Findings"
domain: "Cross-cutting"
---

# Addendum B: Re-Audit After Type Consolidation (M1–M3)

**Purpose:** Re-examination of the full codebase after fixing M1 (type duplication), M2 (Authority DAG inversion), and M3 (LoopId split). Confirms the federation plan's consistency with the corrected type graph and identifies additional issues.

---

## Verification: M1–M3 Fixes Confirmed

| Fix | Status | Verification |
|-----|--------|-------------|
| M1: CNS duplicated types deleted | ✅ | `hkask-cns/src/types/curation.rs` → 12-line re-export module. `OcapTokenKind` grep finds only 1 definition (in `hkask-types`). |
| M2: CuratorHandle/CuratorDirective in hkask-types | ✅ | Canonical at `hkask-types/src/curator.rs`. CNS re-exports from `hkask-types::curator`. 4 consumers updated. |
| M3: LoopId has 6 variants | ✅ | `Inference, Episodic, Semantic, Curation, Cybernetics, Snapshot`. All match arms updated. Zero `LoopId::Memory` references remain. |
| Workspace build | ✅ | `cargo check --workspace` → 0 errors, 0 warnings |
| Test suite | ✅ | 218+ tests, 0 failures |

---

## Additional Findings (M9–M15)

### M9: RuntimeAlert Name Collision (Medium)

**Finding:** Two different structs named `RuntimeAlert` in the same crate (`hkask-cns`):

| Location | Fields | Purpose |
|----------|--------|---------|
| `hkask-cns/src/algedonic.rs` | 7 fields (domain, deficit, threshold, severity, escalated, timestamp, message) | CNS internal alert — the full state of an algedonic event |
| `hkask-cns/src/types/loops/channels.rs` | 4 fields (current, threshold, deficit, timestamp) | Inter-loop wire format — what gets sent through `CurationInput::Alert(RuntimeAlert)` |

These are semantically distinct types that happen to share a name. The `algedonic::RuntimeAlert` is the CNS internal representation; the `channels::RuntimeAlert` is a stripped-down wire format sent to Curation. Both are re-exported via `types::loops` which shadows `algedonic::RuntimeAlert` within the crate.

**Risk:** Module-qualified imports disambiguate at the call site, but the naming collision is confusing for maintainers. Neither is obviously "wrong" — they serve different layers (internal state vs. channel message).

**Recommendation:** Rename `channels::RuntimeAlert` to `AlertPayload` to distinguish wire format from internal state. Low effort, high clarity gain.

---

### M10: Lingering CNS Imports for Migrated Types (Medium)

**Finding:** Four files still import `CuratorHandle` / `CuratorDirective` from `hkask_cns` rather than from the canonical `hkask_types::curator`:

| File | Types Imported from CNS | Should Import From |
|------|------------------------|-------------------|
| `hkask-cli/src/commands/consolidation.rs` | `CuratorHandle` | `hkask_types::curator::CuratorHandle` |
| `hkask-services/src/consolidation.rs` | `CuratorHandle` | `hkask_types::curator::CuratorHandle` |
| `hkask-services/src/curator.rs` | `CuratorHandle` | `hkask_types::curator::CuratorHandle` |
| `hkask-services-context/src/context_impl.rs` | `CuratorHandle`, `CuratorDirective` | `hkask_types::curator::*` |

These currently work because `hkask-cns::types::loops::curation` still re-exports from `hkask-types`. The backward-compatible re-exports should remain, but new code should use the canonical path.

**Risk:** Low — re-exports ensure compilation. But if the CNS re-export module is ever removed (which it should be, eventually), these would break.

**Recommendation:** Migrate these four files. Surgical change: replace `use hkask_cns::types::loops::CuratorHandle` with `use hkask_types::curator::CuratorHandle` (and similarly for `CuratorDirective`).

---

### M11: ConsolidationToken Constructor is Public Despite Doc (Low)

**Finding:** `hkask-capability/src/tokens.rs` doc comment:
> "Only the Curation Loop (hkask_agents::CurationLoop) can mint this token."

But the constructor is `pub fn new(issuer: WebID)` — anyone can call it. The actual enforcement is at the consumption point via `verify_issuer()`.

**Analysis:** This is the Miller OCAP pattern correctly implemented: tokens are capabilities; verification happens at use. The doc implies structural enforcement ("only X can mint") that doesn't exist at the type level. The enforcement is **behavioral** (check issuer at consumption), not **structural** (private constructor).

**Recommendation:** Update doc to say "Issuer is verified at consumption via `verify_issuer()`" rather than "Only the Curation Loop can mint." Low effort.

---

### M12: hkask-agents/types/ Layers on Top of hkask-types (Informational — NOT a problem)

**Finding:** `hkask-agents/src/types/agent/` defines `AgentDefinition`, `PersonaConstraints`, `RegisteredAgent`, `UserProfile` etc. These have more fields than the base types in `hkask-types`. The `yaml_parser.rs` documents this explicitly:

> "The store holds `hkask_types::RegisteredAgent.source_yaml` (the original YAML). This module parses it into the rich `AgentDefinition` from hkask-agents, recovering fields like `persona`, `process_manifest`, `voice_description` etc. that the base `hkask_types::AgentDefinition` doesn't carry."

**Verdict:** This is correct layering, not shadowing. The agents crate adds domain-specific richness on top of the foundation types. The deletion test confirms: removing the agents' richer types would lose persona/voice/process-manifest data that `hkask-types` intentionally doesn't carry.

---

### M13: Federation Plan Consistency with Corrected Type Graph ✅

**Verification:** The federation design (FEDERATION_DESIGN.md §6) proposed:

| Proposal | Canonical Location After M1-M3 | Status |
|----------|-------------------------------|--------|
| `OcapTokenKind::Federation` | `hkask-types/src/curation.rs` (line 311) | ✅ Correct location |
| Federation `CuratorDirective` variants | `hkask-types/src/curator.rs` (line 77) | ✅ Correct location |
| `CurationThresholdConfig` extension | `hkask-types/src/curator.rs` (line 188) | ✅ Correct location |
| `cns.federation.*` CNS spans | `hkask-types/src/cns.rs` (`CnsSpan` enum) | ✅ Correct location |
| `hkask-federation` crate depends on | `hkask-types`, `hkask-agents`, `hkask-communication`, `hkask-cns` | ✅ All in foundation/agent layers |
| Federation NEVER touches | `hkask-capability` (tokens), `hkask-keystore`, `hkask-wallet` | ✅ Financial/security isolation maintained |

**No changes needed to the federation plan.** The corrected type graph actually makes the federation implementation easier: adding `OcapTokenKind::Federation` and federation `CuratorDirective` variants goes directly into `hkask-types` without worrying about CNS duplication.

---

### M14: CuratorService has an unused import pattern (Low)

**Finding:** `hkask-services/src/curator.rs` line 12: `use hkask_cns::types::loops::CuratorHandle;` — the only use is `CuratorHandle::system()` on line 184. This is a remaining CNS import (see M10).

**Recommendation:** Covered by M10 migration.

---

### M15: context_impl.rs Imports CuratorDirective from CNS (Medium)

**Finding:** `hkask-services-context/src/context_impl.rs` line 39:
```rust
use hkask_cns::types::loops::{CurationInput, CuratorDirective, ToolConsumptionEvent};
```

`CuratorDirective` is now canonical in `hkask-types::curator`. The `CurationInput` and `ToolConsumptionEvent` are legitimately CNS loop infrastructure types and should stay in CNS imports.

**Recommendation:** Split the import:
```rust
use hkask_types::curator::CuratorDirective;
use hkask_cns::types::loops::{CurationInput, ToolConsumptionEvent};
```

---

## Federation Plan Audit: Structural Preconditions

### Precondition 1: Federation CRDT Types Need a Home

The federation plan proposes CRDT types (`VersionVector`, `ORSet<T>`, `LWWMap<K,V>`, `GSet<T>`) in a new `hkask-federation` crate. These are pure data structures with no hKask-specific dependencies.

**Assessment:** ✅ Correct. CRDTs are general-purpose data structures. They belong in `hkask-federation` (or could even be extracted to a general-purpose crate).

### Precondition 2: Federation CNS Spans Need CnsSpan Variants

The federation plan proposes 10 new `CnsSpan` variants (`FederationLinkEstablished` through `FederationConduitRouteLost`).

**Assessment:** ✅ Correct. With M1 fixed, there is exactly one `CnsSpan` enum (in `hkask-types/src/cns.rs`). Adding variants is a single-site change.

### Precondition 3: Federation Needs OcapTokenKind Extension

**Assessment:** ✅ Correct. With M1 fixed, `OcapTokenKind` lives only in `hkask-types/src/curation.rs`. Add `Federation` variant there.

### Precondition 4: Federation Needs CuratorDirective Variants

The federation plan could use directives like `FederateWithServer` or `DisconnectPeer`.

**Assessment:** ✅ Correct. With M2 fixed, `CuratorDirective` lives in `hkask-types/src/curator.rs`. Add federation variants there.

### Precondition 5: Separter Skill Registries

The federation plan says skill registries stay local. Each server has its own `SqliteRegistry`.

**Assessment:** ✅ Correct. P5.1 (Single Source of Truth for Skills) already requires this — skills are registry-crate-grounded, not sharable across servers. The federation design respects this invariant.

### Precondition 6: CuratorSync as Model for FederationSync

The federation plan models `FederationSync` on `CuratorSync` (polling loop, cursor-based incremental sync, CNS span emission).

**Assessment:** ✅ Correct. `CuratorSync` is in `hkask-agents/src/curator/semantic_sync.rs`. The pattern is well-established. Federation extends it horizontally (Curator-to-Curator across servers) rather than vertically (Curator-to-Pod within a server).

---

## Updated Priority Summary

| # | Issue | Severity | Effort | Status |
|---|-------|----------|--------|--------|
| M1 | Duplicated OCAP types | Critical | Done | ✅ Fixed |
| M2 | Curator types in wrong crate | Critical | Done | ✅ Fixed |
| M3 | LoopId split | High | Done | ✅ Fixed |
| M9 | RuntimeAlert name collision | Medium | Low | 🟡 New |
| M10 | Lingering CNS imports | Medium | Low | 🟡 New (4 files) |
| M11 | ConsolidationToken doc | Low | Trivial | 🟡 New |
| M12 | hkask-agents/types/ layering | Info | None | ✅ By design |
| M13 | Federation plan consistency | Info | None | ✅ Verified |
| M14 | CuratorService CNS import | Low | Low | 🟡 Covered by M10 |
| M15 | context_impl.rs split import | Medium | Low | 🟡 New |
| M4 | Curator crate extraction | High | High | ⚪ Deferred (architectural decision) |
| M5 | Wrong comment fixed | Low | Done | ✅ Fixed |
| M6 | 7R7 stringly-typed spans | Low | None | ✅ By design |
| M7 | Service layer audit | Medium | Audit | ⚪ Deferred |
| M8 | LoopId/trait split | Low | None | ✅ Necessary design tension |
