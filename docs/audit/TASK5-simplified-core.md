# Task 5: Simplified Core — Idiomatic Rust Module Design

## Module Structure

```
hkask-types/src/
  loops/
    mod.rs              — Re-exports the five loop modules
    inference.rs        — Inference loop: essential types + functions + subloops
    memory.rs           — Memory loop: essential types + functions + subloops
    governance.rs       — Governance loop: essential types + functions + subloops
    observability.rs    — Observability loop: essential types + functions + subloops
    curation.rs         — Curation loop: metacognitive types + functions + subloops
```

The `loops` module goes in `hkask-types` because it defines capability boundary types (handles) that all other crates depend on. Implementations live in their respective crates; `loops` defines the **interfaces** that enforce capability discipline. The Curation loop's implementation lives in `hkask-agents/src/curator/`; the `loops` module defines its interface types and `CuratorHandle`.

---

## Design Principles

### Type-Driven Design (Hoare)
Every essential function has a concrete input type and output type. No trait objects where concrete types suffice. No generics where a single type resolves. The type system enforces capability boundaries; runtime checks enforce only what types cannot constrain.

Each function is specified as a Hoare triple `{P} f {Q}`: a precondition (what must hold before calling), a postcondition (what the function guarantees), and where relevant, an invariant (what must remain true across all calls). Functions without clear triples are underspecified — decompose further.

### Deliberate Composition (Fowler)
Modules compose through explicit dependency injection — each loop's public surface is a struct with `impl` blocks, and dependencies are passed by reference or handle, not reached through global state.

### Capability Discipline (Miller)
Every function receives exactly the authority it needs — no more. Structs hold handles (not clones of entire stores). The `keystore` crate patterns (HKDF-derived keys, AES-256-GCM, OS keychain integration) are reused as the canonical model for how authority is derived and attenuated.

### Essential Subloops Included
The design includes all 14 subloops identified in Task 2 plus one addition from the alternative simplification analysis (rate limiting) and four from the new Curation loop (Loop 5). Context assembly, prompt caching, circuit breaking, energy budgeting, rate limiting, deduplication, consolidation, Bayesian confidence, revocation, sovereignty checking, goal state machines, algedonic alert generation, bot metrics collection, sovereignty observation, escalation routing, bot evaluation, kata coaching, and threshold calibration are all present — either directly or through their capability handles.

---

## Capability Handle Authority Matrix

| Loop | Handle | Can | Cannot |
|------|--------|-----|--------|
| Inference | `InferenceHandle` | Infer, read memory, emit spans, check cache, circuit-break, rate-limit | Write memory, reset alerts, process sovereignty, revoke capabilities |
| Inference | `EnergyBudgetHandle` | Check remaining budget, request consumption, get usage ratio | Set the cap, reset the budget, change alert threshold |
| Inference | `RateLimiterHandle` | Check token bucket, consume invocation slot | Resize bucket, change refill rate, bypass limiting |
| Memory | `MemoryWriteHandle` | Store episodic triples (own WebID), store semantic triples (with capability) | Delete triples, access other agents' private memories |
| Memory | `MemoryReadHandle` | Query visible triples, assemble context | Store triples, delete triples, access private triples of other agents |
| Governance | `GovernanceHandle` | Verify/attenuate/revoke tokens, check visibility, process alerts, calibrate thresholds | Emit arbitrary spans, store triples, run inference |
| Observability | `CnsWriteHandle` | Emit spans, increment variety counters | Reset alerts, subscribe, process sovereignty events |
| Observability | `CnsGovernReadHandle` | Check variety, process sovereignty events (read-only) | Set expected variety, calibrate thresholds, emit spans |
| Observability | `CnsGovernWriteHandle` | Set expected variety, calibrate thresholds (read + write) | Emit spans, reset alerts, subscribe |
| Observability | `CnsAdminHandle` | Reset alerts, clear old alerts, subscribe listeners | Emit spans, check variety |
| Curation | `CuratorHandle` | Read all loop state, write governance/observability policy, issue directives | Run inference, emit spans directly, access private triples |

---

## Module 1: Inference Loop (`inference.rs`)

```rust
//! Inference Loop — prompt → context → model → response → parse → act
//!
//! Essential functions: render_template, assemble_context, get_or_infer,
//!                      try_consume, check_rate_limit, with_circuit_breaker,
//!                      parse_response, dispatch_action
//! Essential subloops: context assembly, prompt cache, circuit breaker, energy budget,
//!                     rate limiting

use crate::capability::{CapabilityToken, CapabilityResource, CapabilityAction};
use crate::cns::{Span, Phase, SpanCategory};
use crate::template::{TemplateCrate, LLMParameters};

// ─── Capability Handles ───

/// Capability-restricted handle for inference operations.
/// Receives exactly the authority the inference loop needs: inference, read-only memory,
/// rate-limited invocation, and span emission.
/// Compare with the current InferencePort which receives a full AcpRuntime.
pub struct InferenceHandle {
    pub capability: CapabilityToken,
    pub budget: EnergyBudgetHandle,
    pub rate_limiter: RateLimiterHandle,
    pub memory: MemoryReadHandle,
    pub observe: CnsWriteHandle,
    pub cache: PromptCacheHandle,
    pub circuit_breaker: CircuitBreakerHandle,
}

/// Energy budget handle — attenuated from full EnergyBudget.
/// Can: check remaining, request consumption, get usage ratio, check alert threshold.
/// Cannot: set the cap, reset the budget, change the alert threshold.
pub struct EnergyBudgetHandle {
    cap: u64,
    consumed: std::sync::atomic::AtomicU64,
    alert_threshold: f64,
}

impl EnergyBudgetHandle {
    pub fn try_consume(&self, tokens: u64) -> bool {
        let new_total = self.consumed.fetch_add(tokens, std::sync::atomic::Ordering::SeqCst);
        new_total + tokens <= self.cap
    }

    pub fn usage_ratio(&self) -> f64 {
        self.consumed.load(std::sync::atomic::Ordering::SeqCst) as f64 / self.cap as f64
    }

    pub fn should_alert(&self) -> bool {
        self.usage_ratio() >= self.alert_threshold
    }
}

/// Prompt cache handle — attenuated from full PromptCache.
/// Can: check for cached result, store result.
/// Cannot: evict entries, change TTL config, clear the cache.
pub struct PromptCacheHandle { /* ... */ }

/// Circuit breaker handle — attenuated from full CircuitBreaker.
/// Can: check state, attempt request through breaker.
/// Cannot: force open/close, reset failure counts.
pub struct CircuitBreakerHandle { /* ... */ }

/// Rate limiter handle — attenuated from full RateLimiter.
/// Can: check token bucket availability, consume one invocation slot.
/// Cannot: resize the bucket, change the refill rate, bypass limiting.
/// Essential for bounded invocation frequency. Without it, runaway loops
/// exhaust resources even within energy budget limits.
pub struct RateLimiterHandle { /* ... */ }

impl RateLimiterHandle {
    /// {bucket has tokens available} try_consume {returns true AND token consumed}
    /// {bucket empty} try_consume {returns false AND cns.tool.rate_limited span emitted}
    pub fn try_consume(&self) -> bool { /* ... */ }

    /// {always} available_tokens {returns count ≥ 0}
    pub fn available_tokens(&self) -> u64 { /* ... */ }
}

// ─── Essential Functions ───
//
// Each function is annotated with a Hoare triple: {precondition} function {postcondition}.

/// {template exists ∧ context is complete} render_template {output is valid Jinja2 output}
pub fn render_template(
    template: &TemplateCrate,
    context: &std::collections::HashMap<String, String>,
) -> Result<String, crate::error::HkaskError> {
    // Delegates to hkask-templates renderer
}

/// {entity ≠ "" ∧ budget_tokens > 0} assemble_context
///   {context.tokens ≤ budget_tokens ∧ ¬∃ duplicate facts by BLAKE3 hash}
pub fn assemble_context(
    memory: &MemoryReadHandle,
    entity: &str,
    budget_tokens: usize,
) -> String {
    // 1. Query visible triples for entity
    // 2. Deduplicate by BLAKE3 hash
    // 3. Enforce token budget
    // 4. Return assembled context string
}

/// {budget.cap > 0 ∧ circuit is closed} get_or_infer
///   {on cache hit: returns cached result; on cache miss: result is from inference
///    OR error is BudgetExhausted OR error is CircuitOpen OR error is RateLimited}
/// Invariant: token consumption ≤ budget.cap
pub fn get_or_infer(
    cache: &PromptCacheHandle,
    budget: &EnergyBudgetHandle,
    rate_limiter: &RateLimiterHandle,
    circuit: &mut CircuitBreakerHandle,
    prompt: &str,
    params: &LLMParameters,
) -> Result<InferenceResult, InferenceError> {
    // 1. Check rate limiter → denied returns RateLimited error
    // 2. Check cache → hit returns cached result
    // 3. Cache miss → check energy budget
    // 4. Budget denied → return BudgetExhausted
    // 5. Budget approved → attempt through circuit breaker
    // 6. Circuit open → return CircuitOpen error
    // 7. Success → cache result, return
}

/// {response is non-empty model output} parse_response
///   {returns ParsedAction::ToolCall OR ParsedAction::FinalAnswer}
pub fn parse_response(response: &str) -> ParsedAction {
    // Parse tool calls and final answers from model response
}

/// {handle.capability is valid ∧ action is parsed} dispatch_action
///   {on tool call: capability is verified at point of use, MCP invoked OR
///    CapabilityDenied error with cns.sovereignty violation span;
///    on final answer: returns result directly}
/// Invariant: capability check is NOT delegated — enforced at point of dispatch
pub fn dispatch_action(
    handle: &InferenceHandle,
    action: ParsedAction,
) -> Result<ActionResult, InferenceError> {
    // 1. If tool call: verify capability at point of use (micro-governance — see TASK7 Q4)
    // 2. If final answer: return
    // 3. Capability denied: emit Sovereignty violation span, return error
}
```

---

## Module 2: Memory Loop (`memory.rs`)

```rust
//! Memory Loop — experience → encode → store → recall → dedup → consolidate → inform inference
//!
//! Essential functions: encode_triple, store_triple, query_triples, dedup_triples,
//!                      consolidate, combine_confidences, retract_confidence, decay_confidence,
//!                      assemble_context
//! Essential subloops: deduplication, consolidation, Bayesian confidence combination

use crate::id::WebID;
use crate::storage::Triple;
use crate::capability::{CapabilityToken, CapabilityAction, CapabilityResource};
use crate::visibility::Visibility;

// ─── Capability Handles ───

/// Memory write handle — attenuated from full TripleStore.
/// Can: store episodic triples (own WebID), store semantic triples (with capability)
/// Cannot: delete triples, store on behalf of other agents
pub struct MemoryWriteHandle {
    agent_webid: WebID,
    capability: CapabilityToken,
    store: std::sync::Arc<std::sync::Mutex<crate::storage::TripleStore>>,
}

/// Memory read handle — attenuated from full TripleStore + EmbeddingStore.
/// Can: query triples by entity (with visibility filtering), assemble context
/// Cannot: store new triples, delete triples, access other agents' private memories
pub struct MemoryReadHandle {
    observer_webid: WebID,
    capability: CapabilityToken,
    store: std::sync::Arc<std::sync::Mutex<crate::storage::TripleStore>>,
}

// ─── Essential Functions ───

/// {entity, attr, value ≠ "" ∧ perspective set} encode_triple
///   {triple.valid_to = NULL ∧ confidence ∈ [0,1]}
pub fn encode_triple(
    entity: &str, attribute: &str, value: &str,
    perspective: Option<WebID>, confidence: f64,
) -> Triple { /* ... */ }

/// {triples ≠ ∅} dedup_triples
///   {¬∃ t₁, t₂ ∈ result: blake3(t₁) = blake3(t₂)}
pub fn dedup_triples(triples: Vec<Triple>) -> Vec<Triple> {
    // BLAKE3 hash-based first-seen-wins deduplication
}

/// {episodic_triples ≠ ∅ ∧ write_handle.capability is valid} consolidate
///   {∀ t ∈ result: t.perspective = None ∧ t.visibility = Public}
pub fn consolidate(
    write_handle: &MemoryWriteHandle,
    episodic_triples: Vec<Triple>,
) -> Result<Vec<Triple>, MemoryError> {
    // 1. Strip perspective (episodic → semantic)
    // 2. Deduplicate
    // 3. Store each unique semantic triple
}

/// {conf1, conf2 ∈ [0,1]} combine_confidences
///   {result ∈ [max(conf1,conf2), min(1, conf1+conf2)]}
pub fn combine_confidences(conf1: f64, conf2: f64) -> f64 {
    // Bayesian product rule: P(A,B) = P(A)×P(B) / (P(A)+P(B)-P(A)×P(B))
    (conf1 * conf2) / (conf1 + conf2 - conf1 * conf2).max(f64::EPSILON)
}

/// {prior ∈ [0,1] ∧ retraction ∈ [0,1]} retract_confidence
///   {result ≤ prior}
pub fn retract_confidence(prior: f64, retraction: f64) -> f64 {
    // Confidence reduction on evidence retraction
    prior * (1.0 - retraction)
}

/// {confidence ∈ [0,1] ∧ rate > 0 ∧ time_steps ≥ 0} decay_confidence
///   {result ≤ confidence ∧ lim_{t→∞} result = 0}
pub fn decay_confidence(confidence: f64, rate: f64, time_steps: u64) -> f64 {
    // Exponential time decay
    confidence * (-rate * time_steps as f64).exp()
}

/// {triples ≠ ∅ ∧ budget_tokens > 0} assemble_context
///   {output.tokens ≤ budget_tokens}
pub fn assemble_context(
    triples: Vec<Triple>, budget_tokens: usize,
) -> String {
    // Token-budget-constrained context assembly from deduplicated triples
}
```

---

## Module 3: Governance Loop (`governance.rs`)

```rust
//! Governance Loop — request → authorize → dispatch → observe → adapt policy
//!
//! Essential functions: verify_capability, is_revoked, attenuate_token, revoke_capability,
//!                      check_visibility, can_transition_to, try_consume, process_alert,
//!                      calibrate_threshold
//! Essential subloops: revocation, sovereignty checking, goal state machine

use crate::capability::{CapabilityToken, CapabilityResource, CapabilityAction,
    CapabilityChecker, Caveat, VerificationResult};
use crate::cns::{AlgedonicAlert, AlertSeverity, CnsGovernReadHandle};
use crate::id::WebID;
use crate::visibility::Visibility;
use crate::sovereignty::{SovereigntyCheckResult, DataCategory};

// ─── Capability Handles ───

pub struct GovernanceHandle {
    secret: crate::keystore::InternalSecrets,
    cns: CnsGovernReadHandle,
    revocation: std::sync::Arc<std::sync::Mutex<crate::storage::RevocationStore>>,
}

impl GovernanceHandle {
    /// {token.sig is valid ∧ ¬is_revoked(token.id) ∧ ¬expired ∧ token.depth ≤ MAX_ATTENUATION_DEPTH}
    ///   verify_capability {granted ⇔ HMAC valid ∧ within depth limit ∧ not expired}
    /// Invariant: MAX_ATTENUATION_DEPTH = 7 (const, not runtime-configurable)
    pub fn verify_capability(&self, token: &CapabilityToken,
        resource: CapabilityResource, action: CapabilityAction) -> VerificationResult { /* ... */ }

    /// {parent.depth < MAX_ATTENUATION_DEPTH ∧ new_holder ≠ parent.holder}
    ///   attenuate_token {new_token.depth = parent.depth + 1}
    pub fn attenuate_token(&self, parent: &CapabilityToken,
        caveats: Vec<Caveat>) -> Result<CapabilityToken, GovernanceError> { /* ... */ }

    /// {token_id exists} revoke_capability
    ///   {RevocationStore.contains(token_id) ∧ cns.cap.revoked span emitted}
    pub fn revoke_capability(&self, token_id: &str) -> Result<(), GovernanceError> { /* ... */ }

    /// {always} is_revoked {returns true ⇔ token_id ∈ RevocationStore}
    pub fn is_revoked(&self, token_id: &str) -> bool { /* ... */ }

    /// {holder is authenticated ∧ category ∈ {Public, Shared, Private, Semantic}}
    ///   check_visibility {returns Allowed ⇔ holder has DataCategory access per key derivation}
    pub fn check_visibility(&self, holder: &WebID, category: DataCategory,
        visibility: Visibility) -> SovereigntyCheckResult { /* ... */ }

    /// {current and target are valid GoalState variants} can_transition_to
    ///   {returns true ⇔ transition is in allowed state graph}
    pub fn can_transition_to(&self, current: GoalState, target: GoalState) -> bool { /* ... */ }

    /// {alert is validated} process_alert
    ///   {returns EscalationAction matching severity: Info→Log, Warning→Queue, Critical→Curator}
    pub fn process_alert(&self, alert: AlgedonicAlert) -> EscalationAction { /* ... */ }

    /// {domain is registered ∧ new_threshold > 0} calibrate_threshold
    ///   {VarietyTracker.expected_variety[domain] = new_threshold}
    /// Note: Invoked by Curation loop via GovernanceHandle. Governance applies the change;
    /// Curation decides when to apply it.
    pub fn calibrate_threshold(&self, domain: &str, new_threshold: usize) { /* ... */ }
}
```

---

## Module 4: Observability Loop (`observability.rs`)

```rust
//! Observability Loop — emit span → aggregate → detect anomaly → escalate
//!
//! Essential functions: emit_event, increment_variety, check_variety,
//!                      determine_severity, process_alert, record_calibration,
//!                      evaluate_bot, process_sovereignty_event
//! Essential subloops: variety tracking, algedonic escalation, bot metrics, sovereignty observation

use crate::cns::{Span, Phase, SpanCategory, AlgedonicAlert, AlertSeverity};
use crate::id::WebID;

// ─── Capability Handles ───

/// CNS write handle — for inference and memory loops.
pub struct CnsWriteHandle {
    emitter: crate::cns::SpanEmitter,
}

impl CnsWriteHandle {
    pub fn emit(&self, span: Span, phase: Phase, outcome: &str, confidence: f64) { /* ... */ }
    pub fn increment_variety(&self, domain: &str) { /* ... */ }
}

pub struct CnsGovernReadHandle {
    algedonic: std::sync::Arc<parking_lot::RwLock<crate::cns::AlgedonicManager>>,
    sovereignty: std::sync::Arc<parking_lot::Mutex<crate::cns::SovereigntyObserver>>,
}

impl CnsGovernReadHandle {
    pub fn check_variety(&self, domain: &str) -> Option<AlgedonicAlert> { /* ... */ }
    pub fn process_sovereignty_event(&self, event: crate::cns::SovereigntyEvent) { /* ... */ }
    // NO set_expected_variety — read-only handle for Governance
}

/// CNS govern write handle — for the Curation loop.
/// Can: everything CnsGovernReadHandle can do, PLUS set expected variety and calibrate.
pub struct CnsGovernWriteHandle {
    inner: CnsGovernReadHandle,
}

impl CnsGovernWriteHandle {
    pub fn check_variety(&self) -> Option<AlgedonicAlert> { self.inner.check_variety() }
    pub fn process_sovereignty_event(&self, event: crate::cns::SovereigntyEvent) {
        self.inner.process_sovereignty_event(event)
    }
    pub fn set_expected_variety(&self, domain: &str, expected: usize) { /* ... */ }
    pub fn calibrate_threshold(&self, domain: &str, new_threshold: usize) { /* ... */ }
}

/// CNS admin handle — for administrative operations only.
pub struct CnsAdminHandle {
    algedonic: std::sync::Arc<parking_lot::RwLock<crate::cns::AlgedonicManager>>,
    subscribers: std::sync::Arc<parking_lot::RwLock<Vec<crate::cns::AlertSubscriber>>>,
}

impl CnsAdminHandle {
    pub fn reset_alerts(&self) { /* ... */ }
    pub fn clear_old_alerts(&self, max_age: std::time::Duration) { /* ... */ }
    pub fn subscribe(&self, subscriber: crate::cns::AlertSubscriber) { /* ... */ }
}

// ─── Essential Functions ───

/// {handle is valid ∧ observer is authenticated} emit_event
///   {ν-event is persisted in NuEventSink ∧ span.path starts with "cns."}
pub fn emit_event(handle: &CnsWriteHandle, observer: &WebID,
    span: Span, phase: Phase, outcome: &str, confidence: f64) { /* ... */ }

/// {govern is initialized} check_all_variety
///   {returns alerts for all domains where deficit > threshold}
pub fn check_all_variety(govern: &CnsGovernReadHandle) -> Vec<AlgedonicAlert> { /* ... */ }

/// {deficit ≥ 0 ∧ threshold > 0} determine_severity
///   {deficit/threshold < 1.0 → Info; 1.0–1.5 → Warning; ≥ 1.5 → Critical}
pub fn determine_severity(deficit: usize, threshold: usize) -> AlertSeverity {
    let ratio = deficit as f64 / threshold as f64;
    if ratio >= 1.5 { AlertSeverity::Critical }
    else if ratio >= 1.0 { AlertSeverity::Warning }
    else { AlertSeverity::Info }
}

/// {alert.severity ∈ {Info, Warning, Critical}} process_alert
///   {returns EscalationAction matching severity}
pub fn process_alert(alert: &AlgedonicAlert) -> EscalationAction {
    match alert.severity {
        AlertSeverity::Info => EscalationAction::Log,
        AlertSeverity::Warning => EscalationAction::QueueForReview,
        AlertSeverity::Critical => EscalationAction::EscalateToCurator,
    }
}

/// {domain is valid ∧ new_threshold > 0} record_calibration
///   {VarietyTracker.expected_variety[domain] = new_threshold}
pub fn record_calibration(govern: &CnsGovernWriteHandle, domain: &str, new_threshold: usize) {
    govern.set_expected_variety(domain, new_threshold);
}

/// {id is authentic WebID ∧ metrics ∈ BotEvaluationMetrics} evaluate_bot
///   {returns BotHealthStatus reflecting current metrics}
pub fn evaluate_bot(id: &WebID, metrics: &crate::cns::BotEvaluationMetrics)
    -> crate::cns::BotHealthStatus { /* ... */ }

/// {event is valid SovereigntyEvent} process_sovereignty_event
///   {per-WebID violation counter incremented; alert generated if threshold exceeded}
pub fn process_sovereignty_event(govern: &CnsGovernReadHandle,
    event: crate::cns::SovereigntyEvent) { /* ... */ }
```

---

## Module 5: Curation Loop (`curation.rs`)

```rust
//! Curation Loop — observe → evaluate → compose → regulate
//!
//! The Curator is the user's agent counterpart — the meta-agent that observes system
//! state, evaluates health and goal progress, composes adaptations, and regulates
//! system behavior by issuing directives. It is the only loop that reads from ALL
//! other loops and writes policy back into them.
//!
//! Implementation: hkask-agents/src/curator/metacognition.rs — MetacognitionLoop
//!
//! Essential functions: run_cycle, check_escalation_triggers, evaluate_bot,
//!                      identify_capability_gap, direct_bot, save_snapshot
//! Essential subloops: escalation routing, bot evaluation, kata coaching,
//!                    threshold calibration

use crate::cns::{AlgedonicAlert, AlertSeverity, CnsGovernWriteHandle};
use crate::capability::{CapabilityToken, CapabilityResource, CapabilityAction};
use crate::id::WebID;
use crate::curator::{EscalationQueue, SystemHealthSnapshot, BotStatusReport,
    EvaluationResult, KataDirective, BotDirective, DirectiveType};
use crate::storage::StoredHealthSnapshot;

// ─── Capability Handles ───

/// Curator handle — the only handle with write authority across loop boundaries.
/// Reads system state from all loops, writes policy to Governance and Observability.
pub struct CuratorHandle {
    /// Read system state: CNS health, variety, sovereignty, alerts
    observe: CnsGovernWriteHandle,
    /// Write governance: calibrate thresholds, update capabilities, revoke tokens
    govern: crate::governance::GovernanceHandle,
    /// Manage escalation queue: post, resolve, dismiss
    escalation: EscalationQueue,
    /// Persist metacognition snapshots
    memory: crate::memory::MemoryWriteHandle,
}

// ─── Essential Functions ───

/// {CNS is active ∧ escalation_queue is initialized} run_cycle
///   {SystemHealthSnapshot produced ∧ escalations posted if thresholds exceeded}
pub async fn run_cycle(
    handle: &CuratorHandle,
) -> Result<SystemHealthSnapshot, CurationError> {
    // 1. Gather CNS health, variety counters, alerts
    // 2. Gather bot reports
    // 3. Build SystemHealthSnapshot
    // 4. Check escalation triggers
    // 5. Persist snapshot
    // 6. Return snapshot
}

/// {snapshot is valid ∧ thresholds are configured} check_escalation_triggers
///   {escalations posted to queue for: variety deficit, critical alerts, bot failures}
pub async fn check_escalation_triggers(
    handle: &CuratorHandle,
    snapshot: &SystemHealthSnapshot,
) -> Result<(), CurationError> {
    // 1. Check total variety deficit vs threshold → post escalation if exceeded
    // 2. Check critical alert count vs threshold → post escalation if exceeded
    // 3. Check bot failure count vs threshold → post escalation if exceeded
}

/// {bot_id is authentic ∧ metrics are collected} evaluate_bot
///   {EvaluationResult with RecommendedAction and capability gaps}
pub fn evaluate_bot(
    bot_id: &WebID,
    metrics: &crate::cns::BotEvaluationMetrics,
) -> EvaluationResult {
    // 1. Compute health status from metrics
    // 2. Identify capability gaps (variety deficit, low success, sovereignty violations)
    // 3. Determine recommended action (None / Monitor / Coach / Calibrate / Escalate)
}

/// {evaluation has gaps} identify_capability_gap
///   {KataDirective with appropriate KataType}
pub fn identify_capability_gap(
    evaluation: &EvaluationResult,
) -> Option<KataDirective> {
    // Map primary gap to kata type:
    //   LowSuccessRate → Starter or Improvement
    //   VarietyDeficit → Coaching
    //   SovereigntyViolations → Coaching
    //   EnergyBudgetCritical → Starter
}

/// {directive is valid ∧ target bot is active} direct_bot
///   {directive delivered to target bot via ACP message}
pub async fn direct_bot(
    handle: &CuratorHandle,
    directive: BotDirective,
) -> Result<(), CurationError> {
    // Issue directive via ACP message delivery through standing session
    // Directive types: CalibrateThreshold, AdjustEnergyBudget,
    //   TriggerKata, UpdateCapabilities, EscalateToHuman
}

/// {snapshot is produced} save_snapshot
///   {StoredHealthSnapshot persisted to SQLite}
pub fn save_snapshot(
    handle: &CuratorHandle,
    snapshot: &SystemHealthSnapshot,
) -> Result<(), CurationError> {
    // Serialize and persist via MetacognitionStoreAdapter
}
```

---

## Contract Tightening

Contracts adopted from the alternative simplification analysis (system-simplification-core-loops.md Task 6) that strengthen invariants at loop boundaries.

| Area | Current | Tightened | Rationale |
|------|---------|-----------|-----------|
| `CapabilityToken` depth | Runtime-configurable constant, default 7 | `const MAX_ATTENUATION_DEPTH: u32 = 7` — compile-time, non-negotiable | Depth is a security invariant, not a tuning parameter |
| `SecurityGateway.authorize()` | Returns `Result<()>` | Returns `Result<CapabilityToken>` — the attenuated token used for the call | Enables audit chaining; caller can verify which token was used |
| `AgentPod` lifecycle | 5 states, no transition guards | Explicit state machine: `can_transition_to(current, target) → bool`. Invalid transitions return `Err` | Prevents Suspended→Activate without re-grant |
| `TemplateEntry.required_capabilities` | Optional field | Non-optional: `Vec::new()` = public. Every template declares OCAP requirements explicitly | Forces explicit security posture; no implicit public access |
| `ContextAssembler` priorities | 7 fragment priorities | Collapse to 4: System, User, Memory, Tool. `perspective` field discriminates episodic vs semantic within Memory | 7 priorities is historical accretion; 4 cover all essential assembly tiers |
| `verify_admin_passphrase` comparison | `==` on strings (timing-variant) | `subtle::ConstantTimeEq` via `ct_eq` | TASK7 Q6 — fix before implementation |
| Schema naming | Spec says `subject/predicate/object` | Code is authoritative: `entity/attribute/value` | Code naming wins; spec is derivative |
| `transaction_at` | Single timestamp but spec says bitemporal | Adopt uni-temporal with `valid_from` (current behavior is correct; spec is aspirational) | Don't claim bitemporal if you have one timestamp |

---

## hKask Reuse

| Existing Crate | What's Reused |
|---|---|
| `hkask-types` | IDs (`WebID`, `GoalID`), `Visibility`, `Span`, `Phase`, `CapabilityToken`, `Caveat` |
| `hkask-storage` | `TripleStore`, `RevocationStore`, `GoalRepository`, `EscalationQueue` |
| `hkask-cns` | `SpanEmitter`, `AlgedonicManager`, `SovereigntyObserver`, `VarietyTracker` |
| `hkask-keystore` | `InternalSecrets` (HKDF-derived HMAC keys) |
| `hkask-templates` | `TemplateCrate`, `LLMParameters`, `ContextAssembler`, `PromptCache`, `CircuitBreaker` |
| `hkask-memory` | `dedup_triples`, `BayesianOps` (as free functions after F8) |
| `hkask-agents` | `MetacognitionLoop`, `EscalationQueue`, `SystemHealthSnapshot`, `BotDirective`, `KataDirective` |

No parallel infrastructures are created. The capability handles wrap existing types, not new ones.