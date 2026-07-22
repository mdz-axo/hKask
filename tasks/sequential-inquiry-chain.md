# Sequential-Inquiry Chain — §2 T1-T6

## T1 — Research Question (IS)

Three evidence briefs produced in `tasks/zed-architecture-briefs.md`. Summary:

| Zed Mechanism | Source File | hKask Equivalent | Transferability |
|---|---|---|---|
| `ContextServerRegistry` (MCP) | `crates/agent/src/agent.rs` | `hkask-mcp` crate + `builtin_servers.rs` | Partial — Zed centralizes; hKask already centralizes via `hkask-mcp` |
| `LanguageModel` trait (inference) | `crates/agent/src/agent.rs` | `InferencePort` in `hkask-ports` | Lateral — both have single trait, hKask's is deeper |
| `ThreadStore` / `ThreadsDatabase` (threads) | `crates/agent/src/thread_store.rs` | `ReplState.thread_registry` + `hkask-storage` | Partial — Zed merges content+metadata; hKask separates crates |
| Single `sqlez` SQLite (storage) | `crates/agent/src/db.rs` | `hkask-storage` + `hkask-database` + `hkask-storage-core` | **Transfers** — Zed uses one storage crate, hKask uses three |
| `LanguageModel` trait in agent crate (foundation) | `crates/agent/src/agent.rs` | `hkask-types` + `hkask-ports` (separate) | **Transfers** — Zed co-locates types+traits, hKask separates them |

Confidence: 0.55 (external source, corroborated against hKask architecture docs).

## T2 — Transferability Branch

| Zed Mechanism | Transfer? | Conflict Reason (hKask principles) |
|---|---|---|
| Flat tool namespace (`mcp:<server>:<tool>`) | NO | P3/P12: OCAP-gated capabilities require per-tool capability declarations |
| Auto-reload on tool list change | Out of scope | §6: new features out of scope; consolidation only |
| From-scratch MCP implementation | NO (opposite) | hKask already uses `rmcp = "1"` (official SDK); Zed is considering adopting it |
| Single `LanguageModel` trait per provider | Lateral | hKask's match-fn dispatch is already minimal; trait dispatch adds a layer |
| Single storage crate (sqlez) | **YES** | No ontology conflict — storage is infrastructure, not surface area |
| Types + traits co-located | **YES** | No ontology conflict — foundation crates, not surface area |
| Auto-compaction in thread lifecycle | Out of scope | §6: behavior change, not consolidation |
| Content/metadata store separation | Lateral | hKask already separates; would not reduce edges |
| `ActionLog` for telemetry | Out of scope | §6: new feature |
| `MAX_SUBAGENT_DEPTH = 1` | Already exists | hKask already enforces this |

**Transferable consolidation patterns:**
1. Merge `hkask-storage` + `hkask-database` + `hkask-storage-core` → single storage crate (Zed uses one `sqlez` store)
2. Merge `hkask-types` + `hkask-ports` → single foundation crate (Zed co-locates types+traits)
3. Merge `hkask-wallet` + `hkask-wallet-types` → single wallet crate (small, low-risk)

## T3 — Falsifiability Delegation

**Target claim:** "Consolidating the codegraph along these seam lines preserves all hKask functionality."

**Admit (Popper gate):** Testable — we can verify MCP tools, skills, provider routes, and CI gates after each merger.

**Hypothesize (Chamberlin — 4 competing shapes):**

- **H-Storage:** Merge storage + database + storage-core into one `hkask-storage` crate.
- **H-Foundation:** Merge types + ports into one `hkask-types` crate.
- **H-Wallet:** Merge wallet + wallet-types into one `hkask-wallet` crate.
- **H-Service:** Merge services-compose into services-context.

**Counterfactual (Pearl do(*not consolidate*)):** If we do NOT consolidate, complexity stays at 397 edges across 58 crates. It does not vanish — it remains distributed. The complexity is real.

But: does consolidation just MOVE complexity? For H-Storage: the 3 crates' internal boundaries become module boundaries — the code doesn't change, just the crate boundary. Net complexity reduction is real (23 crates lose a dependency). For H-Foundation: same — port trait definitions move into types crate, code doesn't change. For H-Wallet: trivially real. For H-Service: the edge between compose and context becomes an intra-crate call — coupling is internalized, not reduced.

**Discriminate (Platt — kill at least one):**

- **H-Service killed:** Merging services-compose into services-context internalizes 2 edges but doesn't reduce actual coupling. The compose service has its own public API consumed by other crates. Moving it inside context would make context a wider, shallower module — violating deep-module discipline. **Eliminated.**

- **H-Storage survives:** 23 crates depend on both storage AND database. Merging saves 23 external edges + 3 internal edges = 26 edges. The merged crate's public API is the union of three crates' APIs — no functionality lost. Storage is infrastructure, not surface area — no MCP tools, no skills, no reg.* spans affected.

- **H-Foundation survives:** 32 crates depend on both types AND ports. Merging saves 32 external edges + 1 internal edge = 33 edges. The merged crate's public API is the union — no functionality lost. Ports traits are type definitions — co-locating them with types is natural.

- **H-Wallet survives:** 2 crates depend on both. Merging saves 2 external + 1 internal = 3 edges. Low risk, small impact.

**Verdict:** H-Storage, H-Foundation, H-Wallet survive. H-Service eliminated.
**Edge total: 26 + 33 + 3 = 62 edges → 15.6% reduction (target: ≥15%). ✓**

## T4 — MCDA Delegation

**Alternatives:** H-Storage, H-Foundation, H-Wallet, H-Storage+H-Foundation+H-Wallet (combined)

**Criteria (weight):** Functionality preservation (0.35), Edge-count reduction (0.20), Interface depth gained (0.20), Migration risk (0.15), Reviewer-load (0.10)

| Alternative | Function | Edge | Depth | Risk↑ | Review↑ | Weighted |
|---|---|---|---|---|---|---|
| H-Storage | 9 | 8 | 7 | 6 | 7 | **7.75** |
| H-Foundation | 8 | 9 | 6 | 3 | 4 | **6.65** |
| H-Wallet | 10 | 4 | 5 | 9 | 9 | **7.55** |
| Combined | 8 | 10 | 7 | 3 | 3 | **6.95** |

**Compensation masking check:**
- H-Foundation: scores 9 on edge reduction but 3 on migration risk. At weight 0.15, the risk score drags it down. However, functionality preservation (the prohibition-tier criterion) scores 8 — acceptable. **No critical masking.**
- Combined: scores 10 on edge reduction but 3 on both risk and review. The combined approach is necessary to meet the 15% target (H-Storage alone = 6.5%, H-Wallet alone = 0.8%). **Must accept the risk to meet the target.**

**Sensitivity analysis:** If migration risk weight increased from 0.15 to 0.30 (from edge reduction), H-Storage would win alone (7.75 → 7.15) but wouldn't meet the 15% target. The combined approach is required regardless of weight distribution.

**Recommendation:** Combined approach (H-Storage + H-Foundation + H-Wallet), executed in risk-ascending order:
1. H-Wallet first (lowest risk, 3 edges)
2. H-Storage second (moderate risk, 26 edges)
3. H-Foundation last (highest risk, 33 edges) — only after 1 and 2 are verified green

## T5 — Diagnose Delegation

**Not invoked** — no slices have been attempted yet. T5 is conditional on slice checkpoint failure.

## T6 — Convergence Check

| Criterion | Status |
|---|---|
| Hypothesis exists and verified | ✅ Combined merger plan (H-Storage + H-Foundation + H-Wallet) |
| Chain complete | ✅ T1-T4 executed, T5 not needed |
| No unresolved branches | ✅ T2 branches all resolved |
| No pending revisions | ✅ No contradictions |
| Confidence calibrated | ✅ solution_confidence = 0.75 |
| Answer synthesized | ✅ Clear, specific, actionable |
| Delegations resolved | ✅ T3 (falsifiability) + T4 (mcda) complete |
| **Convergence metric** | **0.20** (≤0.25 threshold → converged) |

**Blockers:** None. Implementation not yet tested — confidence held at 0.75, not higher.

## Consolidation Plan (for Checkpoint 1 approval)

| Slice | Merger | Edges Removed | Risk | MCP/Skill/Provider Impact |
|---|---|---|---|---|
| T1.1 | H-Wallet: `hkask-wallet` + `hkask-wallet-types` → `hkask-wallet` | 3 | Low | None — wallet is infrastructure |
| T1.2 | H-Storage: `hkask-storage` + `hkask-database` + `hkask-storage-core` → `hkask-storage` | 26 | Moderate | None — storage is infrastructure |
| T1.3 | H-Foundation: `hkask-types` + `hkask-ports` → `hkask-types` | 33 | High | None — types/ports are foundation |
| **Total** | 3 mergers, 5 crates absorbed into 3 | **62** | — | **Zero surface-area impact** |

**Note:** The original directive's T1.1-T1.4 slices (MCP dispatch, skill binding, inference abstraction, REPL lifecycle) are NOT where the edge reduction comes from. The actual consolidation is in the infrastructure layer — storage, foundation types, and wallet. The Zed-inspired insight is that Zed co-locates these concerns in single crates, while hKask over-fragments them. The MCP/skill/inference/REPL surfaces are already well-consolidated in hKask and do not benefit from further merging.