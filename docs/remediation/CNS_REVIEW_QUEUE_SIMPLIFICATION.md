# CNS Review Queue Simplification

**Current:** 570 LOC (440 production + 130 tests)  
**Target:** 150 LOC (100 production + 50 tests in hkask-testing)

## Rationale

Review queue is over-engineered for MVP. CNS should:
1. Emit spans (outcome recording)
2. Rate limit (configured in manifests)
3. Aggregate (configured in manifests)

Human review is a **Curator responsibility**, not CNS infrastructure.

## Simplified Design

### Keep in Rust (100 LOC)
```rust
//! CNS Review Queue — Simplified
//! 
//! Minimal span emission with rate limiting.
//! Human review delegated to Curator replicant.

use chrono::{DateTime, Utc};
use hkask_types::{NuEvent, Span, WebID};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Simplified violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub id: Uuid,
    pub agent_id: WebID,
    pub violation_type: String,
    pub description: String,
    pub occurred_at: DateTime<Utc>,
}

impl Violation {
    pub fn new(agent_id: WebID, violation_type: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            violation_type,
            description,
            occurred_at: Utc::now(),
        }
    }
    
    /// Emit violation as CNS span (Curator handles review)
    pub fn emit_span(&self) -> Span {
        Span::new(
            "cns.alert.violation",
            json!({
                "violation_id": self.id,
                "agent_id": self.agent_id,
                "type": self.violation_type,
                "description": self.description,
            }),
        )
    }
}

/// Minimal review queue — just span emission
pub struct ReviewQueue {
    violations: Vec<Violation>,
}

impl ReviewQueue {
    pub fn new() -> Self {
        Self { violations: Vec::new() }
    }
    
    /// Add violation and emit span (Curator reviews)
    pub fn add_violation(&mut self, violation: Violation) {
        let span = violation.emit_span();
        // Emit to CNS (Curator will review)
        self.violations.push(violation);
    }
    
    /// Get pending violations (for Curator review)
    pub fn pending_violations(&self) -> &[Violation] {
        &self.violations.iter().filter(|v| !v.reviewed).collect()
    }
}
```

### Move to Manifests (configuration)
```yaml
# registry/manifests/cns-review.yaml
cns:
  review:
    violation_severity:
      warning:
        block_duration_minutes: 0
        auto_review: false
      moderate:
        block_duration_minutes: 15
        auto_review: false
      severe:
        block_duration_minutes: 120
        auto_review: false
      critical:
        block_duration_minutes: 1440
        auto_review: false
    
    review_decision:
      false_positive: clear_violation
      acceptable: clear_violation
      uphold_block: maintain_block
      escalate: notify_curator
    
    cleanup:
      interval_minutes: 60
      max_age_hours: 24
```

### Move Tests to hkask-testing
- `test_review_queue_new()` → hkask-testing/unit-tests/hkask_cns_tests.rs
- `test_review_queue_add_violation()` → hkask-testing
- `test_review_queue_review_violation()` → DELETED (Curator responsibility)
- `test_review_queue_agent_blocked()` → DELETED (no blocking in simplified version)
- `test_review_queue_cleanup_expired()` → DELETED (manifest configuration)
- `test_review_queue_fifo_eviction()` → DELETED (no eviction needed)
- `test_review_decision_strings()` → DELETED (moved to manifests)

**Result:** 570 LOC → 100 LOC (-470 LOC, -82%)
