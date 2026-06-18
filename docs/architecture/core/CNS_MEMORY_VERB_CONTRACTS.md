---
title: "CNS Memory Verb Contract Table"
audience: [engineers, agents, replicants]
last_updated: 2026-06-18
version: "0.28.0"
status: "Active"
domain: "Memory"
mds_categories: [domain, trust, lifecycle]
anchored_on: ["docs/architecture/core/PRINCIPLES.md", "docs/architecture/core/TESTING_DISCIPLINE.md"]
---

# CNS Memory Verb Contract Table

Calibration and testing guide for the 13 CNS `MemoryEncode` NuEvents emitted by
autonomous memory operations. Each entry defines the trigger condition, expected
payload, and verification criteria.

## Verb Inventory

| # | Verb | Source File | Trigger | Key Payload Fields | Contract ID |
|---|------|------------|---------|-------------------|-------------|
| 1 | `episodic_stored` | `episodic.rs` | `EpisodicMemory::store()` succeeds | `entity`, `attribute` | `P4-mem-cns-episodic-stored` |
| 2 | `semantic_stored` | `semantic.rs` | `SemanticMemory::store()` succeeds | `entity`, `attribute` | `P4-mem-cns-semantic-stored` |
| 3 | `consolidated` | `semantic.rs` | `SemanticMemory::store_consolidated()` succeeds | `entity`, `attribute` | `P4-mem-cns-consolidated` |
| 4 | `episodic_consolidated` | `episodic_loop.rs` | Budget enforcement triggers consolidation bridge, outcome > 0 | `consolidated`, `failed`, `reason` | `P4-mem-cns-episodic-consolidated` |
| 5 | `episodic_consolidation_failed` | `episodic_loop.rs` | Consolidation bridge returns Err | `error`, `reason` | `P4-mem-cns-episodic-consolidation-failed` |
| 6 | `episodic_budget_exceeded_no_bridge` | `episodic_loop.rs` | Budget exceeded but no `ConsolidationBridge` configured | `overage` | `P4-mem-cns-episodic-budget-exceeded-no-bridge` |
| 7 | `episodic_calibrate` | `episodic_loop.rs` | Non-budget calibration action | `action_type`, `target_loop` | `P4-mem-cns-episodic-calibrate` |
| 8 | `episodic_regulate` | `episodic_loop.rs` | Unhandled regulatory action (fallback) | `action_type`, `target_loop` | `P4-mem-cns-episodic-regulate` |
| 9 | `confidence_decayed` | `episodic.rs` | `EpisodicMemory::query_for_deduped()` applies decay at recall | `entity`, `triple_count`, `deduped_count`, `decay_rate` | `P4-mem-cns-confidence-decayed` |
| 10 | `semantic_condensed` | `semantic_loop.rs` | Condensation groups old triples by entity, soft-deletes duplicates | `total_candidates`, `condensed`, `summaries_stored`, `entity_groups`, `window_days` | `P3-mem-cns-semantic-condensed` |
| 11 | `semantic_budget_enforced` | `semantic_loop.rs` | Low-confidence review deletes triples (implicit via `emit_cns` in act) | `reason: "semantic_low_confidence_review"` or `"semantic_triple_count_exceeded"` | `P3-mem-cns-semantic-budget-enforced` |
| 12 | `consolidation_completed` | `consolidation.rs` | `ConsolidationBridge::consolidate_inner()` finishes | `consolidated_count`, `expired_count`, `failed_count`, `candidate_count` | `P4-mem-cns-consolidation-completed` |
| 13 | `consolidation_service_completed` | `consolidation_service.rs` | `ConsolidationService::consolidate()` finishes all 3 phases | `consolidated_count`, `deleted_count`, `failed_count`, `confidence_floor`, `max_semantic_triples` | `P4-mem-cns-consolidation-service-completed` |

## Verb Contract Details

### 1. episodic_stored
```
REQ: P4-mem-cns-episodic-stored
expect: "CNS observes episodic_stored with entity and attribute when a first-person triple is stored" [P4]
pre:  triple passed visibility/perspective guards
post: NuEvent emitted with verb "episodic_stored", payload contains entity and attribute
test: store episodic triple → assert NuEvent in sink with verb "episodic_stored" and matching entity
```

### 2. semantic_stored
```
REQ: P4-mem-cns-semantic-stored
expect: "CNS observes semantic_stored with entity and attribute when a shared triple is stored" [P4]
pre:  triple passed Public visibility and no-perspective guards
post: NuEvent emitted with verb "semantic_stored", payload contains entity and attribute
test: store semantic triple → assert NuEvent in sink with verb "semantic_stored"
```

### 3. consolidated
```
REQ: P4-mem-cns-consolidated
expect: "CNS observes consolidated with entity and attribute when episodic triple is promoted to semantic" [P4]
pre:  episodic triple stripped of perspective, stored in semantic memory
post: NuEvent emitted with verb "consolidated", payload contains entity and attribute
test: consolidate triple → assert NuEvent with verb "consolidated"
```

### 4. episodic_consolidated
```
REQ: P4-mem-cns-episodic-consolidated
expect: "CNS observes episodic_consolidated with consolidated count when budget enforcement triggers consolidation bridge" [P4]
pre:  episodic loop detected budget overage, consolidation bridge is configured
post: NuEvent emitted with verb "episodic_consolidated", payload contains consolidated > 0, failed count, reason
calibration: consolidated should be ≤ overage, failed should be 0 in normal operation
```

### 5. episodic_consolidation_failed
```
REQ: P4-mem-cns-episodic-consolidation-failed
expect: "CNS observes episodic_consolidation_failed with error when consolidation bridge fails" [P4]
pre:  consolidation bridge returned Err
post: NuEvent emitted with verb "episodic_consolidation_failed", payload contains error message and reason
calibration: any non-zero count signals a CNS alert — consolidation should not fail in normal operation
```

### 6. episodic_budget_exceeded_no_bridge
```
REQ: P4-mem-cns-episodic-budget-exceeded-no-bridge
expect: "CNS observes episodic_budget_exceeded_no_bridge with overage when budget exceeded but no consolidation bridge is configured" [P4]
pre:  episodic loop detected budget overage, no ConsolidationBridge configured
post: NuEvent emitted with verb "episodic_budget_exceeded_no_bridge", payload contains overage
calibration: this verb firing means episodic memory grows without bound — bridge must be wired
```

### 7. episodic_calibrate
```
REQ: P4-mem-cns-episodic-calibrate
expect: "CNS observes episodic_calibrate with action_type and target_loop when episodic loop calibrates" [P4]
pre:  non-budget Calibrate action found (e.g., confidence review)
post: NuEvent emitted with verb "episodic_calibrate", payload contains action_type and target_loop
```

### 8. episodic_regulate
```
REQ: P4-mem-cns-episodic-regulate
expect: "CNS observes episodic_regulate with action_type and target_loop for unhandled regulatory actions" [P4]
pre:  non-Calibrate action found (fallback arm)
post: NuEvent emitted with verb "episodic_regulate", payload contains action_type and target_loop
calibration: high frequency of this verb indicates unexpected action types reaching the loop
```

### 9. confidence_decayed
```
REQ: P4-mem-cns-confidence-decayed
expect: "CNS observes confidence_decayed with triple_count and deduped_count when episodic recall applies decay" [P4]
pre:  EpisodicMemory::query_for_deduped() called, triples found
post: NuEvent emitted with verb "confidence_decayed", payload contains entity, triple_count, deduped_count, decay_rate
calibration: triple_count ≥ deduped_count (dedup reduces count); decay_rate should match DEFAULT_DECAY_RATE
```

### 10. semantic_condensed
```
REQ: P3-mem-cns-semantic-condensed
expect: "CNS observes semantic_condensed with total_candidates, condensed, summaries_stored, entity_groups, window_days when condensation runs" [P3]
pre:  auto_condense enabled, triple count > budget, old triples exist (older than window_days)
post: NuEvent emitted with verb "semantic_condensed", payload contains condensation stats
calibration: condensed > 0, summaries_stored > 0, entity_groups > 0; window_days should match DEFAULT_CONDENSATION_WINDOW_DAYS (30)
test: insert 5 old triples (2 entities), trigger condensation → assert NuEvent with condensed ≥ 2 (one kept per entity)
```

### 11. semantic_budget_enforced
```
REQ: P3-mem-cns-semantic-budget-enforced
expect: "CNS observes semantic budget enforcement when low-confidence review or budget overage triggers deletion" [P3]
pre:  low_confidence_count > 0 OR triple_count > budget
post: NuEvent emitted via emit_cns in SemanticLoop::act(), reason identifies which trigger fired
calibration: reason is "semantic_low_confidence_review" or "semantic_triple_count_exceeded"
```

### 12. consolidation_completed
```
REQ: P4-mem-cns-consolidation-completed
expect: "CNS observes consolidation_completed with consolidated_count, expired_count, failed_count, candidate_count when bridge consolidation finishes" [P4]
pre:  ConsolidationBridge::consolidate_inner() called with candidates > 0
post: NuEvent emitted with verb "consolidation_completed", payload contains all 4 counts
calibration: consolidated_count + failed_count ≤ candidate_count; expired_count should ≈ consolidated_count
```

### 13. consolidation_service_completed
```
REQ: P4-mem-cns-consolidation-service-completed
expect: "CNS observes consolidation_service_completed with consolidated_count, deleted_count, failed_count, confidence_floor, max_semantic_triples when service consolidation finishes" [P4]
pre:  ConsolidationService::consolidate() called, all 3 phases complete
post: NuEvent emitted with verb "consolidation_service_completed", payload contains all counts plus optional floor/max params
calibration: confidence_floor and max_semantic_triples are None if not set in request
```

## CNS Span

All 13 verbs use the same CNS span namespace:

```
SpanNamespace: CnsSpan::MemoryEncode
Phase: Phase::Act
```

The `owner_webid` varies:
- Episode operations: the `perspective` WebID (agent who owns the experience)
- Semantic operations: the triple's `owner_webid`
- Consolidation operations: the `perspective` WebID

## Calibration Summary

| Condition | Expected CNS Signal |
|-----------|-------------------|
| Memory is working normally | Low-frequency `episodic_stored`, `semantic_stored`, `consolidated`, `confidence_decayed` |
| Budget exceeded, bridge wired | `episodic_consolidated` fires with consolidated > 0 |
| Budget exceeded, no bridge | `episodic_budget_exceeded_no_bridge` — **alert: wire the bridge** |
| Old semantic triples, budget exceeded | `semantic_condensed` fires with condensed > 0 |
| Consolidation fails | `episodic_consolidation_failed` — **alert: investigate error** |
| User triggers service consolidation | `consolidation_service_completed` fires once |
| Episodic recall with decay | `confidence_decayed` fires per recall |
| Abnormal action types reach loops | `episodic_regulate` — **alert: unexpected action type** |

## Test Coverage

Current state: 16 memory tests pass. CNS NuEvent assertions are present in the
code via `// REQ:` comments at emission sites in `episodic.rs`, `semantic.rs`,
`consolidation.rs`, and `consolidation_service.rs`.

Loop-level verbs (episodic_loop.rs, semantic_loop.rs) have function-level REQ
contracts on their public API methods but `// REQ:` comments at individual
`emit_cns()` call sites were deferred due to code formatting constraints.
The behavioral contracts in this document serve as the calibration reference.
