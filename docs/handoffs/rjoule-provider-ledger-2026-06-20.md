# rJoule Cost System — Architecture Handoff

**Date:** 2026-06-20
**Session scope:** rJoule dual-track cost system implementation, provider intelligence design, ledger architecture
**Handoff to:** Next agent session continuing rJoule + provider intelligence + ledger work

---

## 1. Session Context

Implemented the rJoule dual-track cost system (gas + API → µrJ → rJ) across 4 crates. Completed all 6 phases of the implementation plan. Then designed two new sub-systems: Provider Intelligence (real-time provider cost tracking with adaptive usage monitoring) and hKask-ledger (unified double-entry accounting as a core crate). These specs are ready for review and implementation planning. The QA script builder skill was started earlier in the session and is documented in a separate handoff at `docs/handoffs/qa-script-builder-2026-06-20.md`.

## 2. What Was Built

### rJoule Cost System (Implemented — 4 crates, 57 tests passing)

| File | Changes |
|------|---------|
| `crates/hkask-services-classify/src/classify_impl.rs` | `ClassifyResult` gains `prompt_tokens`, `completion_tokens`, `cost_urj`, `failed`. Error path recovers tokens. Provider auto-pricing table added. |
| `crates/hkask-test-harness/src/qa_script.rs` | `CostTracker`, `CostSummary`, `StepCost`, `CostSnapshot`. `GasConfig` reworked (250,000 gas = 1 rJ). 6 CNS spans. Per-step cost breakdown. `gas_multiplier` on steps. `training_cost_urj` on steps. Verification invariants. |
| `crates/hkask-cli/src/commands/qa.rs` | Cost summary display. Per-step cost display. Token propagation. |
| `mcp-servers/hkask-mcp-training/src/providers.rs` | `TrainingJob.estimated_cost_urj` (non-optional, computed from host + epochs + model size). Cost estimation function. |
| `mcp-servers/hkask-mcp-training/src/lib.rs` | Submit response includes `estimated_cost_urj`. CNS span `cns.qa.cost.training_job`. |
| `registry/classify/qa-triage.yaml` | Added `cost_input_nj_per_token: 30`, `cost_output_nj_per_token: 60` |
| `registry/classify/qa-feedback.yaml` | Same |

### Specifications Written (Ready for Review)

| File | Status |
|------|--------|
| `docs/architecture/specs/rjoule-cost-system.md` | Complete — unit system, dual-track model, SCI derivation, CostTracker design, verification |
| `docs/architecture/specs/provider-intelligence.md` | Draft — `ProviderIntelligence` trait, 7 per-provider profiles, adaptive monitoring, integration with cost tracker |
| `docs/architecture/specs/hkask-ledger.md` | Draft — double-entry ledger as core crate, schema, API, invariants, 3 domain ledgers sharing same backend, account naming convention |
| `docs/plans/rjoule-cost-tracking-implementation.md` | Complete — 6-phase plan, all phases marked done |

## 3. Architecture Decisions (Do Not Reverse Without Understanding)

1. **250,000 gas = 1 rJ (not 500,000).** Based on 0.02 kWh per function call — doubled from Glass et al. 0.01 kWh to account for infrastructure overhead (CNS, tracing, registry, YAML) and provisioned-vs-utilized energy per SCI specification. 1 gas = 4 µrJ.

2. **API costs flow from the classify service, not the manifest.** Provider pricing lives in `registry/classify/*.yaml` and/or the provider auto-pricing table in `classify_impl.rs`. The manifest's `GasConfig` only has `gas_per_function` — no API pricing fields.

3. **CNS is active signalling, not passive logging.** QA classify steps raise direct algedonic alerts (`alert:` config per step) that flow to the Curator. `cns_span` is tracing, `alert` is escalation — orthogonal.

4. **Integer micro-rJ (µrJ) for all internal accounting.** No floating-point. 1 µrJ = $0.000001. Future-proofed for rJ tokenization.

5. **Ledger as a single core crate serving three domains.** Cost ledger, crypto ledger, and securities ledger all use the same `hkask-ledger` backend with different account namespaces. Based on Formance/Blnk patterns. Not three separate implementations.

6. **Provider intelligence is an adaptive daemon.** Not periodic cron. Checks accelerate from daily → 10-minute intervals as usage approaches provider limits. Detects pre-paid → marginal shift in real-time.

## 4. Specs Awaiting Review

### Provider Intelligence (`docs/architecture/specs/provider-intelligence.md`)

4 open questions needing answers before implementation:
- Self-tracked providers: persistent call counter? (Answer: yes, must persist between runs)
- Provider config for "dumb" APIs: admin-defined tier/limits in YAML config
- Multi-key: aggregate tracking
- Ledger integration: provider rate changes written as ledger transactions

### hKask Ledger (`docs/architecture/specs/hkask-ledger.md`)

4 open questions needing answers:
- Single ledger.db or per-namespace?
- Materialized balances or computed from postings?
- Portfolio migration path?
- Internal-only or MCP-exposed?

## 5. What Remains (Ordered by Dependency)

### P0 — Finalize Specs
- Resolve open questions in provider-intelligence.md and hkask-ledger.md
- Get user sign-off on account naming convention

### P1 — Build hKask-ledger Core Crate
- `crates/hkask-ledger/` — SQLite-backed double-entry ledger
- Implement `Ledger`, `LedgerTransaction`, `Posting`, invariants
- Tests: idempotency, double-entry validation, balance computation
- **Depends on:** Finalized spec

### P2 — Wire CostTracker → Ledger
- `CostTracker` accepts `Option<Arc<Ledger>>`
- On run completion, commit cost transactions
- CLI displays ledger-confirmed balances
- **Depends on:** hKask-ledger built

### P3 — Provider Intelligence Trait + DeepInfra Implementation
- `ProviderIntelligence` trait in `hkask-services-classify` or new crate
- `DeepInfraProvider` implementation (lowest complexity, always marginal)
- Wire into `classify_batch` for actual cost lookups
- **Depends on:** Finalized spec

### P4 — Remaining Provider Implementations
- OpenRouter, Together, Brave, Firecrawl, Tavily, Exa, FMP, EODHD, Runpod, Baseten
- Provider config YAML files in `registry/providers/`
- Self-tracked call counters for providers without usage APIs
- **Depends on:** Provider intelligence trait built

### P5 — Adaptive Usage Daemon
- Per-provider monitoring schedule
- Acceleration logic (daily → 10min based on usage %)
- CNS spans for marginal activation
- **Depends on:** Provider implementations

### P6 — Portfolio Migration
- Port `PortfolioManager` in `hkask-mcp-companies` to use `hkask-ledger`
- Or keep as-is with ledger as parallel option
- **Depends on:** hKask-ledger built

## 6. Build Status

- `cargo check`: clean, zero warnings from our changes
- 76 tests passing (57 + 19)
- All changes on stable Rust

## 7. Key Files

| File | Purpose |
|------|--------|
| `docs/architecture/specs/rjoule-cost-system.md` | Unit system, cost model, derivation |
| `docs/architecture/specs/provider-intelligence.md` | Provider trait, adaptive monitoring, per-provider profiles |
| `docs/architecture/specs/hkask-ledger.md` | Core ledger crate design |
| `docs/plans/rjoule-cost-tracking-implementation.md` | Completed 6-phase plan |
| `docs/handoffs/qa-script-builder-2026-06-20.md` | QA script builder skill handoff (separate thread) |
