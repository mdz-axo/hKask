# Capability-Energy Integration

## Overview

This document describes the integration between capability-based security (OCAP) and energy budgeting in hKask.

## Architectural Decisions

### Q1: Single Attenuation
**Decision:** Support single attenuation only (no infinite delegation chains)
**Rationale:** Simplest model, predictable nested execution, avoids complexity of multi-level delegation tracking

### Q2: Quota System
**Decision:** Implement energy quota allocation from parent to child capabilities
**Rationale:** Flexible energy distribution enables fine-grained resource control

### Q3: No Delegation Between Agents
**Decision:** Each agent operates with independent capabilities
**Rationale:** Simplifies security model, avoids cross-agent capability leakage

### Q4: Cryptographic Verification (Paxos/CRDT Lazy)
**Decision:** Self-verifying capabilities with eventual consistency
**Rationale:** Enables distributed verification without central authority

### Q5: Hybrid Expiry
**Decision:** Lazy verification + periodic cleanup
**Rationale:** Balances performance (lazy check on use) with resource management (periodic cleanup)

### Q6: Clear Error Messages
**Decision:** Hard abort (security) + Escalate (user-facing) with transparent messages
**Rationale:** Security violations must be clear; user-facing errors must include specific "ask"

### Q7: ~~Capabilities Revocation~~ REMOVED
**Correction:** Capabilities are **persistent** by OCAP definition — cannot be "revoked" mid-execution
**Rationale:** OCAP principle — capabilities represent authority, not ACL entries

### Q8: Temporary Block + Human Review
**Decision:** Sandbox violations trigger temporary block + review queue
**Rationale:** Balanced security — automatic protection with human override

## Implementation

### Cryptographic Verification (P3-2)

Located in `crates/hkask-types/src/capability.rs`:

```rust
pub fn verify_lazy(&self, secret: &[u8], local_time: i64) -> VerificationResult {
    let signature_valid = self.verify(secret);
    let expired = self.is_expired(local_time);

    if !signature_valid {
        VerificationResult::Invalid
    } else if expired {
        VerificationResult::Zombie // Valid signature, but expired
    } else {
        VerificationResult::Valid
    }
}

pub fn fingerprint(&self) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}",
        self.id,
        self.resource.as_str(),
        self.resource_id,
        self.action.as_str(),
        self.delegated_to,
        self.attenuation_level
    )
}

pub fn is_compatible_with(&self, other: &CapabilityToken) -> bool {
    self.resource == other.resource
        && self.resource_id == other.resource_id
        && self.action == other.action
        && self.delegated_to == other.delegated_to
}
```

**VerificationResult states:**
- `Valid` — Signature valid, not expired — capability can be used
- `Zombie` — Signature valid, but expired — capability is "zombie" (valid but unusable)
- `Invalid` — Signature invalid — capability is tampered or forged

**Zombie state enables CRDT eventual consistency** — machines may disagree on expiry but agree on signature validity.

### Hybrid Expiry (P3-3)

Located in `crates/hkask-cns/src/energy.rs`:

```rust
pub fn cleanup_expired_capabilities(
    capabilities: &[CapabilityToken],
    current_time: i64,
) -> (usize, usize) {
    let mut kept = 0;
    let mut removed = 0;

    for cap in capabilities {
        if cap.is_expired(current_time) {
            removed += 1;
        } else {
            kept += 1;
        }
    }

    (kept, removed)
}

pub fn recommended_cleanup_interval() -> u64 {
    300 // 5 minutes
}
```

**Two-phase approach:**
1. **Lazy verification** — Every capability use checks expiry via `verify_lazy()`
2. **Periodic cleanup** — Background job removes expired capabilities from registry

### Quota Allocation (P3-1)

Located in `crates/hkask-cns/src/energy.rs`:

```rust
pub enum EnergySpanType {
    Allocate,
    Consume,
    Opportunity,
    Deficit,
    Actual,
    Quota,      // NEW: Quota allocation from parent to child
    Overflow,   // NEW: Energy budget exceeded
}
```

### Clear Error Messages (P3-4, Q6)

Located in `crates/hkask-templates/src/error.rs`:

```rust
pub enum TemplateError {
    EnergyBudgetHardAbort {
        manifest_id: String,
        capability_id: String,
        allocated: u64,
        consumed: u64,
        overage_percent: f64,
    },
    EnergyBudgetEscalate {
        manifest_id: String,
        capability_id: String,
        allocated: u64,
        consumed: u64,
        overage_percent: f64,
        ask: String, // Specific user action required
    },
}
```

**Error message format:**
```
Energy budget exceeded: manifest={id}, capability={cap_id}
Allocated: {allocated} energy units
Consumed: {consumed} energy units ({overage}% over budget)
Action required: {ask}
```

## Testing

### Capability Tests
```bash
cargo test -p hkask-types -- capability
```

**Tests:**
- `test_capability_token_creation` — Token structure validation
- `test_capability_token_expiry` — Expiry tracking
- `test_capability_token_verify_lazy` — Three-state verification
- `test_capability_token_fingerprint` — CRDT fingerprint generation
- `test_capability_token_compatibility` — CRDT merge compatibility
- `test_capability_token_attenuation` — Single attenuation
- `test_verification_result` — Result state machine

### Energy Tests
```bash
cargo test -p hkask-cns -- energy
```

**Tests:**
- `test_cleanup_expired_capabilities` — Hybrid expiry cleanup
- `test_recommended_cleanup_interval` — Cleanup scheduling

## Registry Integration

28 energy_budget entries across 9 manifests in `registry/manifests/`:
- `bootstrap.yaml`
- `composition.yaml`
- `inference.yaml`
- `memory.yaml`
- `storage.yaml`
- `templates.yaml`
- `ensemble.yaml`
- `mcp.yaml`
- `ocap.yaml`

Each manifest includes:
```yaml
energy_budget:
  cap: <energy_units>
  capability_id: <capability_reference>
```

## Completion Status

| Task | Status | Location |
|------|--------|----------|
| P3-1: Quota allocation API | ✅ Complete | `hkask-cns/src/energy.rs` |
| P3-2: Cryptographic verification | ✅ Complete | `hkask-types/src/capability.rs` |
| P3-3: Hybrid expiry | ✅ Complete | `hkask-cns/src/energy.rs` |
| P3-4: Clear error messages | ✅ Complete | `hkask-templates/src/error.rs` |
| P3-5: Sandbox review queue | ⏳ Pending | Requires MCP integration |
| P3-6: Documentation update | ✅ Complete | This document |

## Next Steps

1. Implement sandbox temporary block mechanism (P3-5)
2. Integrate review queue with MCP dispatch
3. Add CNS spans for review queue events
4. Test end-to-end capability-energy flow

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Capabilities persist. Energy is finite. Composition is the goal.*
