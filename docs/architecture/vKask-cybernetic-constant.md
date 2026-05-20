# νKask — The Cybernetic Constant

## Executive Summary

νKask (nu-Kask, or "neuro-Kask") is the **Planck constant of cybernetic systems** — the minimum viable unit of intelligence derived from first principles of cybernetics, autopoiesis, and recursive self-reference. This document replaces the OKH/feedback loop architecture with a principled cybernetic substrate grounded in 70 years of systems theory.

**Name Rationale:** 
- **ν (nu)** = Greek letter representing frequency in physics, neuro in cognitive science
- Also evokes "new" (νέος) — a fresh start from first principles
- Complements ℏKask (Planck constant) — νKask is the *cybernetic* constant, ℏKask is the *structural* constant

---

## Part I: Theoretical Foundations

### 1.1 The Cybernetic Planck Constant

**Definition:** The minimum distinguishable state change in a cognitive-regulatory system.

**Physical Analogy:** Just as Planck's constant ℏ defines the minimum action in quantum mechanics (E = ℏω), the **Cybernetic Constant κ** defines the minimum cognitive action required for a system to regulate itself.

```
κ = τ_min × ΔS_min

Where:
- τ_min = minimum observation-regulation cycle time (~50-150ms in humans)
- ΔS_min = minimum distinguishable state difference (1 bit of regulatory information)
```

**Empirical Calibration:**
- Human reaction time: ~150ms (simple stimulus-response)
- OODA loop (military): 30-90 seconds for complex decisions
- LLM inference + validation: 500ms-5s depending on model size
- **νKask target:** κ = 100ms base cycle (configurable per agent)

### 1.2 Ashby's Law — Formalized

**Law of Requisite Variety** (Ashby, 1956):

```
V(R) ≥ V(D)

Where:
- V(R) = variety of the Regulator
- V(D) = variety of Disturbances
```

**Implication for νKask:** The system must have at least as many distinguishable internal states as the environmental states it must regulate. This drives:
- Memory capacity requirements
- Template diversity
- Agent pod cardinality

**Variety Engineering:** Three levers:
1. **Reduce V(D)** — constrain the environment (capability gating)
2. **Increase V(R)** — add more regulatory states (more templates, agents)
3. **Amplify V(R) via recursion** — meta-cognition multiplies effective variety

### 1.3 Beer's VSM — Minimal Viable Structure

**Five Systems** (necessary and sufficient):

| System | Function | νKask Equivalent |
|--------|----------|------------------|
| **S1** | Operations | MCP Tools, Agent Pods |
| **S2** | Coordination | Template Registry (prevents conflicts) |
| **S3** | Control | Cybernetic Monitor (here-and-now regulation) |
| **S4** | Intelligence | Meta-Cognition (there-and-then adaptation) |
| **S5** | Policy | User Sovereignty (identity, ultimate authority) |

**Recursive Theorem:** Every viable system contains and is contained in a viable system.
- νKask is viable at the agent-pod level
- νKask is viable at the multi-agent hive level
- νKask is viable at the user+agents ecosystem level

### 1.4 Second-Order Cybernetics — The Observer Included

**Von Foerster's Distinction** (1974):
- **First-order:** Cybernetics of *observed* systems (the system is separate from the observer)
- **Second-order:** Cybernetics of *observing* systems (the observer is part of the system)

**Implication:** νKask must observe its own observing. Every telemetry event includes:
- What was observed
- The act of observation itself (meta-observation)
- The observer's state (agent identity, template used, confidence)

**Maturana's Theorem:** "Everything said is said by an observer."
- Every OKH span includes observer identity
- Every feedback loop is attributed to a specific agent-pod-template tuple

### 1.5 Autopoiesis — Self-Production

**Definition** (Maturana & Varela, 1980): A system is autopoietic if it produces its own organization through a network of processes that recursively generate the same network.

**νKask Autopoietic Properties:**
- Templates generate template invocations (which update template quality metrics)
- Agent pods create agent pods (via UCAN delegation with attenuation)
- Feedback loops modify feedback loop parameters (recursive calibration)
- Memory recalls trigger memory writes (crystallization)

**Test:** If you remove the external developer, can νKask continue to operate and improve? If yes → autopoietic. If no → allopoietic (made by others).

---

## Part II: νKask Architecture

### 2.1 The Cybernetic Triad

Every cybernetic act in νKask consists of three inseparable phases:

```
┌─────────────────────────────────────────────────────────────┐
│                    CYBERNETIC TRIAD                         │
│                                                             │
│   OBSERVE ──────► REGULATE ──────► ACT                      │
│      ▲                                │                     │
│      └────────────────────────────────┘                     │
│                    (recursive loop)                         │
└─────────────────────────────────────────────────────────────┘
```

**OBSERVE (ν-observe):**
- Telemetry capture (OKH spans)
- Pattern recognition (cognition templates)
- State estimation (bitemporal query)

**REGULATE (ν-regulate):**
- Compare to contract (LmContract validation)
- Compute error signal (pass/fail, Brier score)
- Select corrective action (template choice, agent routing)

**ACT (ν-act):**
- Invoke tool (MCP call)
- Render template (prompt/process/cognition)
- Write memory (bitemporal assert)

**Key Insight:** These are NOT sequential stages — they are **simultaneous aspects** of a single cybernetic event. The separation is analytical, not temporal.

### 2.2 The νKask State Machine

```rust
pub enum CyberneticState {
    /// System is observing environment
    Observing {
        channel: ObservationChannel,
        bandwidth: usize,  // bits/cycle
        latency_ms: u64,
    },
    
    /// System is computing regulation
    Regulating {
        error_signal: f64,  // -1.0 to 1.0
        variety_available: usize,
        variety_required: usize,
    },
    
    /// System is acting on environment
    Acting {
        action: CyberneticAction,
        expected_outcome: OutcomeContract,
    },
    
    /// System is observing its own observation (second-order)
    MetaObserving {
        base_observation: ObservationId,
        observer_state: ObserverState,
        blind_spot_detected: bool,
    },
}
```

### 2.3 Core Data Structures

#### 2.3.1 Cybernetic Event (ν-Event)

The atomic unit of νKask — replaces OKH spans + feedback loops.

```rust
pub struct CyberneticEvent {
    /// Unique identifier
    pub id: EventId,
    
    /// Timestamp (HLC — hybrid logical clock)
    pub timestamp: HybridLogicalClock,
    
    /// Observer identity (which agent-pod-template produced this)
    pub observer: ObserverRef,
    
    /// The triad phase (observe/regulate/act/meta-observe)
    pub phase: CyberneticPhase,
    
    /// What was observed (telemetry payload)
    pub observation: Observation,
    
    /// Regulation computed (error signal, decision)
    pub regulation: Option<Regulation>,
    
    /// Action taken (tool call, template render, memory write)
    pub action: Option<Action>,
    
    /// Outcome (pass/fail, Brier score, latency)
    pub outcome: Option<Outcome>,
    
    /// Recursive depth (0 = first-order, 1 = second-order, etc.)
    pub recursion_depth: u8,
    
    /// Parent event (for meta-observation chains)
    pub parent_event: Option<EventId>,
}

pub enum CyberneticPhase {
    Observe,
    Regulate,
    Act,
    MetaObserve,
}
```

#### 2.3.2 Observer Reference

Every event is attributed to a specific observer.

```rust
pub struct ObserverRef {
    /// Agent pod identity
    pub pod_id: PodId,
    
    /// WebID (sovereign identity)
    pub webid: AgentWebId,
    
    /// Template used (if any)
    pub template: Option<TemplateRef>,
    
    /// Observation channel (which "sense")
    pub channel: ObservationChannel,
}

pub enum ObservationChannel {
    Telemetry,      // OKH spans
    MemoryRecall,   // Bitemporal queries
    ToolOutput,     // MCP tool results
    UserInput,      // Direct user commands
    MetaCognition,  // Self-observation
}
```

#### 2.3.3 Variety Counter

Tracks regulatory capacity vs. environmental complexity.

```rust
pub struct VarietyCounter {
    /// Environmental variety (disturbances detected)
    pub v_disturbance: usize,
    
    /// Regulatory variety (available responses)
    pub v_regulator: usize,
    
    /// Required variety (minimum to maintain control)
    pub v_required: usize,
    
    /// Variety deficit (v_required - v_regulator, negative = ok)
    pub variety_deficit: i64,
    
    /// Alert threshold (trigger when deficit > this)
    pub alert_threshold: i64,
}

impl VarietyCounter {
    pub fn is_viable(&self) -> bool {
        self.variety_deficit <= 0
    }
    
    pub fn algedonic_alert(&self) -> bool {
        // Algedonic alert: pain signal when variety insufficient
        self.variety_deficit > self.alert_threshold
    }
}
```

**Algedonic Alert:** Beer's term for a pain signal that triggers System 5 intervention when lower systems cannot handle variety.

### 2.4 The Cybernetic Monitor

Replaces OKH telemetry + feedback aggregation.

```rust
pub struct CyberneticMonitor {
    /// Event buffer (circular, fixed size)
    pub event_buffer: CircularBuffer<CyberneticEvent>,
    
    /// Variety counters per agent-pod
    pub variety_counters: HashMap<PodId, VarietyCounter>,
    
    /// Algedonic alert handler
    pub alert_handler: Arc<dyn AlgedonicHandler>,
    
    /// Bitemporal store (audit trail)
    pub audit_store: Arc<dyn BitemporalStore>,
    
    /// Cybernetic constant (minimum cycle time)
    pub kappa: Duration,
}

impl CyberneticMonitor {
    /// Record a cybernetic event
    pub fn record(&mut self, event: CyberneticEvent) {
        // Check variety
        if let Some(counter) = self.variety_counters.get_mut(&event.observer.pod_id) {
            counter.v_disturbance += 1;
            
            // Algedonic alert?
            if counter.algedonic_alert() {
                self.alert_handler.trigger(AlertLevel::Critical, &event);
            }
        }
        
        // Persist to audit trail
        self.audit_store.assert_datom(event.to_datom());
        
        // Add to buffer
        self.event_buffer.push(event);
    }
    
    /// Compute pass rate for a template (replaces OutcomeAggregator)
    pub fn pass_rate(&self, template: &TemplateRef, window: Duration) -> f64 {
        let events = self.event_buffer.query(window);
        let relevant = events.iter()
            .filter(|e| e.observer.template.as_ref() == Some(template))
            .collect::<Vec<_>>();
        
        let passes = relevant.iter()
            .filter(|e| e.outcome.as_ref().map(|o| o.pass).unwrap_or(false))
            .count();
        
        passes as f64 / relevant.len() as f64
    }
    
    /// Detect regressions (replaces OutcomeAggregator::check_regressions)
    pub fn check_regressions(&self, threshold: f64, min_events: usize) -> Vec<RegressionAlert> {
        // ... implementation
    }
}
```

### 2.5 Algedonic Alert System

**Definition:** A pain signal that escalates to System 5 (policy) when lower-level regulation fails.

```rust
pub enum AlertLevel {
    Info,       // Variety deficit detected
    Warning,    // Deficit growing
    Critical,   // Deficit exceeding threshold
    Emergency,  // System viability at risk
}

pub trait AlgedonicHandler: Send + Sync {
    /// Trigger an alert
    fn trigger(&self, level: AlertLevel, event: &CyberneticEvent);
    
    /// Escalate to System 5 (user intervention required)
    fn escalate_to_system5(&self, alert: AlertLevel, context: AlertContext);
}

pub struct AlertContext {
    pub pod_id: PodId,
    pub variety_deficit: i64,
    pub recent_failures: usize,
    pub suggested_action: String,
}
```

**Escalation Path:**
1. **Info** → Log to audit trail
2. **Warning** → Notify agent pod (self-regulation opportunity)
3. **Critical** → Notify user (System 5 intervention)
4. **Emergency** → Suspend agent pod, request human takeover

---

## Part III: Implementation Plan

### 3.1 Crate Structure

Replace `stack-meta` + `stack-fpl` feedback + OKH instrumentation with:

```
stack-cybernetics/
├── src/
│   ├── lib.rs              # Public API
│   ├── event.rs            # CyberneticEvent, ObserverRef
│   ├── monitor.rs          # CyberneticMonitor
│   ├── variety.rs          # VarietyCounter, algedonic alerts
│   ├── triad.rs            # Observe/Regulate/Act logic
│   ├── recursion.rs        # Meta-observation, recursion depth
│   └── audit.rs            # Bitemporal audit trail
├── Cargo.toml
└── README.md
```

### 3.2 Migration Strategy

**Phase 1: Parallel Implementation**
- Build `stack-cybernetics` alongside existing OKH/feedback
- Emit both OKH spans and ν-events during transition
- Validate that ν-events capture all OKH information

**Phase 2: API Unification**
- Replace `OutcomeAggregator` with `CyberneticMonitor::pass_rate()`
- Replace `OkhSpan` with `CyberneticEvent::observation`
- Replace feedback loops with `CyberneticEvent::regulation`

**Phase 3: Deletion**
- Remove `stack-meta`, `stack-fpl` feedback code
- Remove OKH span instrumentation
- Update all MCP servers to emit ν-events

### 3.3 Instrumentation Example

**Before (OKH spans):**
```rust
#[tracing::instrument(
    skip(params),
    fields(
        okh.tool.invoked = "memory_recall",
        okh.prompt.template = "recall_query",
    )
)]
async fn recall(params: &RecallParams) -> Result<MemoryResult> {
    let result = /* ... */;
    OutcomeAggregator::shared().record("recall_query", result.pass);
    Ok(result)
}
```

**After (ν-events):**
```rust
async fn recall(params: &RecallParams, observer: &ObserverRef) -> Result<MemoryResult> {
    let event = CyberneticEvent {
        phase: CyberneticPhase::Act,
        observer: observer.clone(),
        observation: Observation::ToolCall {
            tool: "memory_recall",
            params: params.clone(),
        },
        ..Default::default()
    };
    
    monitor.record(event);
    
    let result = /* ... */;
    
    let outcome_event = CyberneticEvent {
        phase: CyberneticPhase::MetaObserve,
        observer: observer.clone(),
        observation: Observation::Outcome {
            template: "recall_query",
            pass: result.is_ok(),
        },
        parent_event: Some(event.id),
        recursion_depth: 1,
        ..Default::default()
    };
    
    monitor.record(outcome_event);
    
    Ok(result)
}
```

---

## Part IV: Acceptance Criteria

νKask is complete when:

- [ ] **Cybernetic Event** structure captures all OKH span information + observer identity
- [ ] **Variety Counter** correctly tracks V(D) vs V(R) for each agent pod
- [ ] **Algedonic Alerts** trigger when variety deficit exceeds threshold
- [ ] **Pass Rate** computation matches previous `OutcomeAggregator` within 1%
- [ ] **Regression Detection** identifies template degradation at 10% drop
- [ ] **Second-Order Observation** (recursion_depth > 0) works for meta-cognition templates
- [ ] **Audit Trail** persists all ν-events to bitemporal store
- [ ] **Performance** overhead < 10% vs. OKH spans (measured in latency)
- [ ] **Documentation** updated: AGENTS.md, architecture docs, OKH standard deprecated

---

## Part V: Open Questions — Future Task

### Underspecified Areas

1. **Cybernetic Constant Calibration**
   - What is the optimal κ for LLM-based agents?
   - Should κ be adaptive (faster for simple tasks, slower for complex)?
   - How do we measure τ_min empirically in the system?

2. **Variety Measurement**
   - How do we count V(D) (environmental variety) in practice?
   - Is it the number of distinct tool calls? Template invocations?
   - Should we use Shannon entropy instead of raw counts?

3. **Recursion Depth Limits**
   - What is the maximum useful recursion_depth?
   - Should we enforce a hard limit (like matroshka number 7)?
   - How do we detect infinite meta-observation loops?

4. **Algedonic Threshold Tuning**
   - What is the default alert_threshold for variety_deficit?
   - Should thresholds be per-agent or global?
   - How do we prevent alert fatigue (too many false positives)?

5. **Observer Identity in Cascades**
   - When a cascade skill executes, who is the observer?
   - The root template? Each sub-skill? Both?
   - How do we attribute outcomes in nested executions?

6. **Bitemporal Audit Retention**
   - How long do we keep ν-events in the audit trail?
   - Should we crystallize frequently-accessed events?
   - What is the retention policy (time-based, space-based)?

7. **User Sovereignty and Alerts**
   - When should algedonic alerts interrupt the user vs. log silently?
   - What is the System 5 escalation protocol?
   - Can users configure alert sensitivity?

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| **κ (kappa)** | Cybernetic constant — minimum observation-regulation cycle time |
| **ν-event** | Atomic cybernetic event (observe + regulate + act + meta-observe) |
| **Variety** | Number of distinguishable states in a system (Ashby) |
| **Requisite Variety** | Regulatory variety ≥ environmental variety |
| **Algedonic Alert** | Pain signal triggering System 5 intervention |
| **Observer** | Agent-pod-template tuple producing a cybernetic event |
| **Second-Order** | Observation of observation (meta-cognition) |
| **Autopoiesis** | Self-producing organization (Maturana & Varela) |
| **VSM** | Viable System Model (Beer) — 5 necessary/sufficient subsystems |
| **Triad** | Observe → Regulate → Act (simultaneous aspects) |

---

## Appendix B: References

1. Ashby, W.R. (1956). *An Introduction to Cybernetics*. Chapman & Hall.
2. Beer, S. (1972). *Brain of the Firm*. Wiley.
3. Beer, S. (1979). *Heart of Enterprise*. Wiley.
4. Maturana, H.R. & Varela, F.J. (1980). *Autopoiesis and Cognition*. Reidel.
5. von Foerster, H. (1974). "Cybernetics of Cybernetics". Biological Computer Laboratory.
6. von Foerster, H. (2003). *Understanding Understanding*. Springer.
7. Glanville, R. (2004). "Second-Order Cybernetics: An Historical Introduction". Kybernetes.
8. Fields, C. et al. (2021). "Minimal Physicalism as a Scale-Free Substrate for Cognition and Consciousness". NeuroQuantology.
9. Ayvazov, A. (2025). "Principia Teleodynamica Vol. II: Recursive Intelligence". PhilArchive.
10. Trukovich, J. (2025). "Recursive Emergence Across Scales". SSRN.

---

*This document specifies νKask — the cybernetic constant for hKask. All cybernetic instrumentation must trace back to these requirements.*
