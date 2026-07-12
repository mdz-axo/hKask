---
title: "CNS Span Emission — 4-Namespace Sequence"
audience: [architects, developers, agents]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Cybernetics"
mds_categories: [domain, observability, trust]
diataxis: "reference"
---

# CNS Span Emission — 4-Namespace Sequence

## Description

The CNS (Cybernetic Nervous System) emits structured spans across four canonical namespaces — `cns.tool`, `cns.inference`, `cns.agent_pod`, and `cns.curation` — using `tracing::info!(target: "cns.X")` as the emission surface. The `CnsRuntime` subscriber layer collects these spans, constructs `NuEvent` records with `SpanParent` relationships for child spans, persists them through the `NuEventSink`, and routes algedonic signals to the `AlgedonicManager`. The `ToolSpanGuard` RAII guard ensures every tool invocation emits a span: explicit `ok()`/`error()` calls emit the appropriate status, and the `Drop` implementation catches forgotten spans with a `"dropped"` outcome.

**Key source:** `crates/hkask-mcp/src/server/tool_span.rs:18-189` (`ToolSpanGuard`), `crates/hkask-cns/src/runtime.rs:540-615` (`increment_variety`, `check_variety`), `crates/hkask-types/src/event.rs:16-93` (`NuEvent`, `parent_event`), `crates/hkask-types/src/event.rs:370-429` (`SpanKind`, namespace mappings).

### Span Namespace Model

| Namespace | SpanKinds | Subscriber Interest |
|-----------|-----------|---------------------|
| `cns.tool` | `Invoked`, `Completed`, `Error`, `dropped` | GovernedTool, CyberneticsLoop |
| `cns.inference` | inference spans (via `tracing::span!`) | InferenceAdapter |
| `cns.agent_pod` | `Registered`, `Activated`, `Deactivated` | PodManager |
| `cns.curation` | `DirectiveAcknowledged`, `Escalation` | CuratorAgent |

```mermaid
sequenceDiagram
    participant Code as Code Site<br/>(tool handler / inference / pod / curator)
    participant Guard as ToolSpanGuard
    participant Trace as tracing<br/>(target: "cns.X")
    participant Rtm as CnsRuntime<br/>(subscriber)
    participant Sink as NuEventSink
    participant Algd as AlgedonicManager

    rect rgb(245, 248, 252)
        Note over Code,Algd: cns.tool — Tool Invocation Span Emission

        Code->>+Guard: ToolSpanGuard::new(tool_name, caller)
        Note over Guard: emitted = false<br/>start = Instant::now()
        Code->>+Code: business logic → Result

        alt explicit ok()
            Code->>+Guard: guard.ok(output)
            Guard->>+Trace: info!(target: "cns.tool", outcome="ok", duration_ms, caller)
            Trace-->>-Guard: ()
            Guard->>+Guard: emitted = true
            Guard-->>-Code: output
        else explicit error()
            Code->>+Guard: guard.error(kind, output)
            Guard->>+Trace: info!(target: "cns.tool", outcome="error", error_kind, caller)
            Trace-->>-Guard: ()
            opt heal callback set
                Guard->>+Guard: heal_error_cb(output, tool_name)
            end
            Guard->>+Guard: emitted = true
            Guard-->>-Code: output
        end

        Note over Guard,Trace: RAII — Drop impl for abandoned guards
        rect rgb(255, 245, 240)
            Note over Guard: Drop: if !emitted
            Guard->>+Trace: info!(target: "cns.tool", outcome="dropped", duration_ms, caller)
            Trace-->>-Guard: ()
            Note over Guard: "dropped" status — forgotten span caught
        end
    end

    rect rgb(245, 252, 245)
        Note over Code,Algd: Subscriber Dispatch — CnsRuntime receives the span

        Trace->>+Rtm: tracing layer captures span
        Rtm->>+Rtm: SpanNamespace::parse(domain)
        Rtm->>+Rtm: construct NuEvent { span, phase, observation }
        Rtm->>+Sink: persist(NuEvent)

        alt SpanParent relationship
            Note over Rtm,Sink: NuEvent.with_parent(parent_id)
            Rtm->>+Sink: persist(NuEvent { parent_event: Some(parent_id) })
            Note over Sink: child span linked to parent EventID
        end

        Sink-->>-Rtm: ()

        loop for each CnsObserver in subscribers
            Rtm->>+Rtm: observer.interest_mask() matches span_ns?
            alt interest matches
                Rtm->>+Rtm: observer.on_event(&event)
            end
        end
    end

    rect rgb(252, 252, 245)
        Note over Code,Algd: Variety Counter → Algedonic Check

        Rtm->>+Rtm: increment_variety(domain, state_name)
        Rtm->>+Rtm: state.tracker.counter(domain).increment(state_name)
        Rtm->>+Rtm: check_variety(domain)
        Rtm->>+Algd: mgr.check(counter, domain)

        alt deficit > threshold
            Algd-->>Rtm: Some(RuntimeAlert::Critical)
            Algd->>+Algd: error!(target: "cns.algedonic", "ALGEDONIC ALERT")
            Rtm->>+Rtm: emit_critical_depletion(alert)
        else deficit > threshold/2
            Algd-->>Rtm: Some(RuntimeAlert::Warning)
            Algd->>+Algd: warn!(target: "cns.algedonic", "Variety deficit approaching threshold")
        else healthy
            Algd-->>Rtm: None
        end
        Algd->>+Algd: alerts.push(alert)
    end

    rect rgb(248, 245, 255)
        Note over Code,Algd: cns.inference — Inference Span

        Code->>+Trace: tracing::span!(target: "cns.inference")
        Trace->>+Rtm: layer captures inference span
        Rtm->>+Sink: persist(NuEvent { span: cns.inference.* })
        Sink-->>-Rtm: ()
    end

    rect rgb(248, 245, 255)
        Note over Code,Algd: cns.agent_pod — Agent Pod Lifecycle Span

        Code->>+Trace: info!(target: "cns.agent_pod", ...)
        Note over Trace: SpanKind::AgentPodRegistered<br/>SpanKind::AgentPodActivated<br/>SpanKind::AgentPodDeactivated
        Trace->>+Rtm: layer captures pod span
        Rtm->>+Sink: persist(NuEvent)
        Sink-->>-Rtm: ()
    end

    rect rgb(248, 245, 255)
        Note over Code,Algd: cns.curation — Curation Span

        Code->>+Trace: info!(target: "cns.curation", ...)
        Note over Trace: SpanKind::CurationDirectiveAcknowledged<br/>SpanKind::CurationEscalation
        Trace->>+Rtm: layer captures curation span
        Rtm->>+Sink: persist(NuEvent)
        Sink-->>-Rtm: ()
    end
```

## SpanParent Relationship Model

```mermaid
sequenceDiagram
    participant Parent as NuEvent<br/>(parent)
    participant Child as NuEvent<br/>(child)

    Note over Parent,Child: Child span construction
    Parent->>+Child: NuEvent::new(...).with_parent(parent.id)
    Note over Child: child.parent_event = Some(parent.id)
    Child->>+Child: persisted with parent reference
    Note over Parent,Child: Enables trace reconstruction:<br/>parent_id → child_id chain
```

## ToolSpanGuard Drop Behavior

| Method | `emitted` | Outcome in trace | Callbacks Fired |
|--------|-----------|------------------|-----------------|
| `guard.ok(output)` | `true` | `"ok"` | `experience_cb("success")` |
| `guard.error(kind, output)` | `true` | `"error"` | `heal_error_cb` + `experience_cb("error")` |
| `guard.finish(Result)` | `true` | `"ok"` / `"error"` | Context-dependent |
| **No explicit call → `Drop`** | `false` | `"dropped"` | None |

The `Drop` impl enforces that a span is always emitted — even if the code path panics, returns early, or the developer forgets to call `ok()`/`error()`. The `"dropped"` outcome is an observability signal: it tells the CNS that a tool execution began but never reached a terminal state.

---

<!-- DIAGRAM_ALIGNMENT
id: DIAG-TO-004
verified_date: 2026-07-01
verified_against: >
  crates/hkask-mcp/src/server/tool_span.rs:18-189 (ToolSpanGuard, Drop impl, emit_tool_span),
  crates/hkask-cns/src/runtime.rs:295-299 (CnsRuntime struct),
  crates/hkask-cns/src/runtime.rs:540-615 (increment_variety, check_variety, subscriber dispatch),
  crates/hkask-types/src/event.rs:16-93 (NuEvent, parent_event, with_parent builder),
  crates/hkask-types/src/event.rs:105-157 (CANONICAL_NAMESPACES),
  crates/hkask-types/src/event.rs:320-429 (Span, SpanKind, namespace_and_path),
  crates/hkask-cns/src/algedonic.rs:139-296 (AlgedonicManager, check)
status: VERIFIED
-->

## Cross-Reference

| Reference | Description |
|-----------|-------------|
| [`ToolSpanGuard`](crates/hkask-mcp/src/server/tool_span.rs:18-189) | RAII span guard with `ok()`, `error()`, `Drop` for forgotten spans |
| [`CnsRuntime`](crates/hkask-cns/src/runtime.rs:294-299) | CNS runtime with subscribers and algedonic manager |
| [`NuEvent`](crates/hkask-types/src/event.rs:16-93) | CNS event with `parent_event` for span parent relationships |
| [`SpanKind`](crates/hkask-types/src/event.rs:370-429) | Typed span kind enum with canonical namespace/path mapping |
| [`CANONICAL_NAMESPACES`](crates/hkask-types/src/event.rs:105-157) | All valid CNS span namespaces |
| [`AlgedonicManager`](crates/hkask-cns/src/algedonic.rs:139-296) | Alert manager with variety deficit checking |
| [PRINCIPLES.md §P9](docs/architecture/core/PRINCIPLES.md) | Homeostatic Self-Regulation |
| [`sequence-algedonic-escalation.md`](docs/diagrams/sequence-algedonic-escalation.md) | Algedonic escalation flow (DIAG-TO-005) |
