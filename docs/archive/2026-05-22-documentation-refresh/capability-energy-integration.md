
## Sandbox Review Queue (P3-5)

Located in `crates/hkask-cns/src/review_queue.rs`:

### Violation Severity Levels

```rust
pub enum ViolationSeverity {
    Warning,    // No block
    Moderate,   // 15 minute block
    Severe,     // 2 hour block
    Critical,   // 24 hour block
}
```

### Review Decisions

```rust
pub enum ReviewDecision {
    FalsePositive,  // Release agent, clear block
    Acceptable,     // Release with warning
    UpholdBlock,    // Keep block active
    Escalate,       // Convert to indefinite block
}
```

### Key Components

**Violation:** Sandbox violation record with agent ID, severity, type, description

**TemporaryBlock:** Time-limited block with automatic expiry

**ReviewQueue:** FIFO queue with configurable max size, automatic eviction

**ReviewQueueObserver:** CNS span emitter for observability

### CNS Spans

- `cns.review.violation` — Violation detected and queued
- `cns.review.block` — Temporary block applied
- `cns.review.decision` — Human operator decision recorded
- `cns.review.release` — Agent released from block

### Usage Example

```rust
let mut queue = ReviewQueue::new(100);
let observer = ReviewQueueObserver::new(observer_webid);

// Add violation
let violation = Violation::new(
    agent_id,
    ViolationSeverity::Moderate,
    "memory_access".to_string(),
    "Unauthorized memory access attempt".to_string(),
);
let (violation_id, block_id) = queue.add_violation(violation);

// Check if agent is blocked
if queue.is_agent_blocked(&agent_id) {
    // Agent is blocked - reject operation
}

// Review violation
queue.review_violation(violation_id, ReviewDecision::FalsePositive, operator_id);

// Cleanup expired blocks
queue.cleanup_expired_blocks();
```

### Tests

```bash
cargo test -p hkask-cns -- review_queue
```

**10 tests passing:**
- `test_violation_creation`
- `test_violation_review`
- `test_violation_severity_block_duration`
- `test_temporary_block_expiry`
- `test_review_queue_add_violation`
- `test_review_queue_review_violation`
- `test_review_queue_agent_blocked`
- `test_review_queue_cleanup_expired`
- `test_review_queue_fifo_eviction`
- `test_review_decision_strings`

## Updated Completion Status

| Task | Status | Location | Tests |
|------|--------|----------|-------|
| P3-1: Quota allocation API | ✅ Complete | `hkask-cns/src/energy.rs` | ✅ |
| P3-2: Cryptographic verification | ✅ Complete | `hkask-types/src/capability.rs` | ✅ 7 |
| P3-3: Hybrid expiry | ✅ Complete | `hkask-cns/src/energy.rs` | ✅ 2 |
| P3-4: Clear error messages | ✅ Complete | `hkask-templates/src/error.rs` | ✅ |
| P3-5: Sandbox review queue | ✅ Complete | `hkask-cns/src/review_queue.rs` | ✅ 10 |
| P3-6: Documentation update | ✅ Complete | This document | ✅ |

## Full Test Suite

```bash
cargo test -p hkask-types -p hkask-cns
```

**Expected results:**
- hkask-types: 51 tests passing
- hkask-cns: 36 tests passing (26 energy + 10 review_queue)

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Phase 3 Complete: Capability-Energy Integration + Sandbox Review Queue*
