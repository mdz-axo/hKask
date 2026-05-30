# Task 3: Composition Graph — Loop Interactions

Every edge between loops must be a **capability reference** — a minimally-authority-bearing handle that grants access to exactly one loop's function, not a god-object.

```mermaid
erDiagram
    INFERENCE ||--o{ MEMORY : "assemble_context + store_episodic"
    INFERENCE ||--o{ GOVERNANCE : "verify_capability + try_consume"
    INFERENCE ||--o{ OBSERVABILITY : "emit_span"
    MEMORY ||--o{ OBSERVABILITY : "emit_span"
    MEMORY ||--o{ INFERENCE : "assemble_context"
    GOVERNANCE ||--o{ OBSERVABILITY : "emit_span + process_sovereignty"
    OBSERVABILITY ||--o{ CURATION : "algedonic_alert + variety + bot_metrics"
    GOVERNANCE ||--o{ CURATION : "sovereignty_violations + revocation_events"
    INFERENCE ||--o{ CURATION : "energy_budget_status + bot_health"
    CURATION ||--o{ GOVERNANCE : "calibrate_threshold + update_capabilities"
    CURATION ||--o{ INFERENCE : "adjust_energy_budget + trigger_kata"
    CURATION ||--o{ OBSERVABILITY : "threshold_calibration"
    CURATION ||--o{ MEMORY : "persist_snapshots"
    GOVERNANCE ||--o{ INFERENCE : "energy_budget_denial"

    INFERENCE {
        render_template fn
        assemble_context fn
        get_or_infer fn
        infer fn
        try_consume fn
        with_circuit_breaker fn
        parse_response fn
        dispatch_action fn
    }
    MEMORY {
        encode_triple fn
        store_triple fn
        query_triples fn
        dedup_triples fn
        consolidate fn
        combine_confidences fn
        retract_confidence fn
        decay_confidence fn
        assemble_context fn
    }
    GOVERNANCE {
        verify_capability fn
        attenuate_token fn
        revoke_capability fn
        check_visibility fn
        is_revoked fn
        can_transition_to fn
        try_consume fn
        process_alert fn
        calibrate_threshold fn
    }
    OBSERVABILITY {
        emit_event fn
        increment_variety fn
        check_variety fn
        determine_severity fn
        process_alert fn
        record_calibration fn
        evaluate_bot fn
        process_sovereignty_event fn
    }
    CURATION {
        run_cycle fn
        check_escalation_triggers fn
        evaluate_bot fn
        identify_capability_gap fn
        direct_bot fn
        save_snapshot fn
    }
}
```

## Exact Types Crossing Boundaries

| Edge | Exact Type Crossing | Authority Granted | Attenuation Candidate? |
|------|---------------------|-------------------|----------------------|
| Inference → Memory | `Triple` (entity, attribute, value, perspective, confidence, visibility) | Write episodic memory | No — carries exactly the data needed |
| Inference → Memory (read) | `Entity` → `Vec<Triple>` | Read memory for context | No — scoped to entity |
| Inference → Governance | `CapabilityToken` + `CapabilityResource` + `CapabilityAction` | Verify one action on one resource | **YES**: `InferencePort` receives full `AcpRuntime` |
| Inference → Observability | `Span` + `Phase` + `Outcome` + `Confidence` | Emit one span | No — `SpanEmitter` is already a capability handle |
| Memory → Observability | `Span` + `Phase` + `Outcome` | Emit one span | No |
| Governance → Observability | `Span` + `Phase` + `Outcome` + `WebID` | Emit one span with attribution | No — `SpanScope` already restricts categories |
| Observability → Curation | `AlgedonicAlert` + `SystemHealthSnapshot` + `BotEvaluationMetrics` | Read system state, receive alerts | No — Curation reads exactly the data it needs |
| Governance → Curation | `SovereigntyCheckResult` + `RevocationEvent` | Read policy violation data | No — read-only access to violation data |
| Inference → Curation | `EnergyBudgetHandle` (read) + `BotStatusReport` | Read energy status and bot health | No — read-only access to system health |
| Curation → Governance | `CalibrateThreshold` + `UpdateCapabilities` directives | Write policy changes | No — Curation is the authorized policy writer |
| Curation → Inference | `AdjustEnergyBudget` + `TriggerKata` directives | Write resource/coaching directives | No — Curation is the authorized directive issuer |
| Curation → Observability | `CnsGovernWriteHandle` — calibration writes | Write threshold changes | **YES**: separate read/write handles for Governance vs. Curation |
| Curation → Memory | `StoredHealthSnapshot` | Persist metacognition data | No — write access scoped to metacognition |
| Governance → Inference | `EnergyBudget` (cap, consumed, threshold) | Reject inference if budget exhausted | No — budget is a narrow numeric gate |
| Memory → Inference | `ContextFragment` (content, source, priority, confidence) | Provide context for prompt assembly | No — fragments are read-only data |

## Capability Attenuation Candidates

### 1. `InferencePort` receives full `AcpRuntime`

**Current authority:** registration, messaging, capability registry access — most not needed for inference.

**Required authority:** `verify_capability(resource, action) → bool`

**Attenuation:** Replace `AcpRuntime` with `CapabilityVerifier` trait exposing only `verify_capability`.

### 2. `McpDispatcher` holds duplicate security fields

**Current authority:** `rate_limiter` + `capability_checker` on the dispatcher AND `security_gateway`.

**Required authority:** All enforcement through `SecurityGateway` only.

**Attenuation:** Remove `rate_limiter` and `capability_checker` from `McpDispatcher`. All security flows through the gateway.

### 3. `CnsRuntime` exposes full read-write access

**Current authority:** `health()`, `alerts()`, `increment_variety()`, `check_variety()`, `subscribe()`, `reset_alerts()`, `process_sovereignty_event()`.

**Attenuation:** Split into four capability handles:
- `CnsWriteHandle` — `increment_variety()`, `emit()` (for inference/memory)
- `CnsGovernReadHandle` — `check_variety()`, `process_sovereignty_event()` (for governance — read only)
- `CnsGovernWriteHandle` — `set_expected_variety()`, `calibrate_threshold()` (for curation — write)
- `CnsAdminHandle` — `reset_alerts()`, `clear_old_alerts()`, `subscribe()` (for administration)

### 4. `PodContext` receives capability secret

**Current authority:** The `secret` field allows minting new tokens — excessive for a pod.

**Attenuation:** `PodContext` should receive a pre-attenuated `CapabilityToken`, not the signing secret.

## Composition Gaps

### 1. Inference → Memory: No explicit capability boundary on write

**Gap:** Inference calls `store()` directly on `TripleStore` with no capability check. Any pod can write any triple.

**Fix:** Inference receives `MemoryWriteHandle` which enforces OCAP on writes. Memory writes go through governance.

### 2. Observability → Curation: Alert delivery has no capability check

**Gap:** Alerts are delivered to Curation without verifying the Curation loop's capability to receive them.

**Fix:** `AlgedonicAlert` delivery goes through `CnsGovernReadHandle` (Curation reads alerts). The handle enforces that only the Curator's `WebID` can subscribe to critical alerts.

### 3. Memory → Inference: No visibility boundary on recall

**Gap:** `assemble_context()` reads triples without checking `Visibility` boundaries. A bot sees other agents' private episodic memories.

**Fix:** `MemoryReadHandle` scopes reads to the caller's `WebID` and capability caveats.

### 4. No capability boundary between Governance and Observability

**Gap:** Governance calls `CnsRuntime` methods directly with full admin access.

**Fix:** Implement the `CnsGovernReadHandle` / `CnsGovernWriteHandle` / `CnsWriteHandle` / `CnsAdminHandle` split. Governance receives `CnsGovernReadHandle` (read-only); Curation receives `CnsGovernWriteHandle` (read + write).