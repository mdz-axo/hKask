---
title: "How to Read CNS Alerts — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Read CNS Alerts

**Goal:** Read and interpret CNS (Cybernetic Nervous System) spans, variety counters, and algedonic alerts to understand system health and respond to violations.

The CNS is Loop 6 of hKask's cybernetic architecture — an observability substrate that emits typed spans for every operation affecting system state. When variety drops or errors spike, the CNS emits **algedonic alerts** (pleasure/pain signals) to the operator.

---

## 1. What CNS Alerts Are

CNS spans are typed identifiers in a dot-separated namespace (e.g., `cns.tool.web_search`, `cns.inference`, `cns.gas.reserved`). Every tool invocation, inference call, gas consumption, contract lifecycle event, and sovereignty check emits a span.

Spans flow through two paths:

| Path | Mechanism | Purpose |
|------|-----------|---------|
| **Tracing** | `tracing::info!(target: "cns", ...)` | Structured logging |
| **ν-event** | `NuEvent` → `NuEventSink` → SQLite | Persistent cybernetic audit trail |

Spans describe *what* happened; ν-events describe *who observed it, when, in what context, and what they saw*.

**Algedonic alerts** are generated when the CNS detects that variety (number of distinct operational states) has fallen below a threshold, signaling a potential loss of system responsiveness.

---

## 2. Reading Health Status

The fastest way to see overall system health:

```bash
kask cns health
```

Output breakdown:

```
CNS Health Status
=================

Runtime Status:
  • Healthy: true | false            ← Overall health
  • Overall variety deficit: <N>     ← How far below expected variety
  • Critical alerts: <N>             ← Critical threshold breaches
  • Warning alerts: <N>              ← Warning threshold breaches

Variety Counter Summary:
  • cns.tool.web_search: 12 states    ← Per-namespace variety counts
  • cns.inference: 8 states
  • cns.tool.condenser: 3 states
  ...

Active Algedonic Alerts:
  • [Critical] cns.tool: Tool variety critically low
  • [Warning] cns.inference: Inference error rate elevated
  ...

Energy Budget Status:
  • Model: Energy tracking (subsumes rate limiting)
  • Status: OPERATIONAL
```

Key indicators to watch:

- **`Healthy: false`** — immediate investigation needed
- **`Critical alerts: > 0`** — at least one domain has fallen below its critical threshold
- **`Overall variety deficit`** — growing deficit means the system is seeing fewer distinct operational patterns

---

## 3. Viewing Active Alerts

To see only alerts without the full health report:

```bash
kask cns alerts
```

Output:

```
Algedonic alerts:
  • [Critical] cns.tool: Tool call failures exceeded threshold
  • [Warning] cns.gas: Energy budget running low
```

If no alerts are active:

```
Algedonic alerts:
  (no active alerts)
```

---

## 4. Viewing Variety Counters

Variety measures how many distinct operational states each namespace is experiencing:

```bash
kask cns variety
```

Output:

```
Variety counters:
  • cns.tool.web_search: 12 states
  • cns.inference: 8 states
  • cns.gas: 5 states
  • cns.curation: 3 states
```

Low variety in a namespace signals the system is stuck in a narrow operational band — it's not exploring, adapting, or handling diverse inputs.

---

## 5. Interpreting Set Point Violations

Set points define the CNS's expected operating parameters. View current set points:

```bash
kask cns set-points
```

Output:

```
CNS Set-Points
==============
  gas_min_remaining:       100
  variety_max_deficit:        50
  error_rate_max:             0.25
  connector_latency_max_secs: 5
  communication_backpressure_threshold: 1000
```

| Set Point | Meaning | What Happens When Breached |
|-----------|---------|---------------------------|
| `gas_min_remaining` | Minimum energy budget before depletion signal | `DepletionSignal` broadcast to subscribers |
| `variety_max_deficit` | Maximum tolerated variety drop | Algedonic alert fires; severity depends on deficit size |
| `error_rate_max` | Maximum tolerated error rate (0.0–1.0) | Error rate alert fires |
| `connector_latency_max_secs` | Maximum connector response latency | Latency alert fires |
| `communication_backpressure_threshold` | Queue depth before backpressure engages | Backpressure alert fires |

To configure set points, provide the flags:

```bash
kask cns set-points \
  --gas-min-remaining 50 \
  --variety-max-deficit 100 \
  --error-rate-max 0.30
```

---

## 6. Interpreting Algedonic Alerts

Algedonic alerts escalate through severity levels based on Ashby's Law of Requisite Variety:

| Deficit vs Threshold | Severity | Action |
|----------------------|----------|--------|
| deficit ≤ threshold/2 | **Info** | Logged, no escalation |
| deficit > threshold/2, ≤ threshold | **Warning** | `warn!` log emitted |
| deficit > threshold | **Critical** | `error!` log + `DepletionSignal` broadcast |

The default variety threshold is `DEFAULT_VARIETY_MAX_DEFICIT` (from `hkask_cns`). Per-domain expected variety can be configured via `AlgedonicManager::set_expected_variety()`.

### Alert Severity Rules of Thumb

| Severity | Meaning | Response |
|----------|---------|----------|
| **Info** | Normal operation, noted for audit | No action required |
| **Warning** | Degraded but functional — variety dipping or errors rising | Monitor; check recent spans for patterns |
| **Critical** | Threshold breached — system may be blind to important states | Investigate immediately; review the affected domain |
| **Fatal** | System cannot continue | `DepletionSignal` broadcast; agent pods will halt |

---

## 7. Filtering Spans by Namespace

Query CNS spans by namespace to focus on specific subsystems:

```bash
# Sovereignty-related spans (P1–P2 enforcement)
kask cns subscribe --agent curator --spans cns.sovereignty

# Tool invocation spans (P4 OCAP enforcement)
kask cns subscribe --agent curator --spans cns.tool

# MCP startup gate spans
kask cns subscribe --agent curator --spans cns.mcp

# Federation spans
kask cns subscribe --agent curator --spans cns.federation
```

### Live Event Subscription

Subscribe to live CNS events for specific span namespaces:

```bash
# Subscribe to tool and inference events for the Curator agent
kask cns subscribe --agent curator --spans cns.tool.web_search,cns.inference
```

Output:

```
CNS Event Subscription
=====================
  Agent: curator
  Span namespaces:
    • cns.tool.web_search
    • cns.inference

  Note: Subscription is active for the lifetime of this process.
  Events matching the specified namespaces will be delivered.
```

---

## 8. Common CNS Span Namespaces

| Namespace | What It Tracks | Algedonic? |
|-----------|---------------|------------|
| `cns.tool.*` | MCP tool invocations (web_search, condenser, etc.) | Yes — variety |
| `cns.inference` | LLM inference calls | Yes — variety |
| `cns.gas` | Gas (energy budget) consumption | Yes — depletion |
| `cns.sovereignty` | Consent grants, revocations, checks | No (audit only) |
| `cns.curation` | Curator consolidation and directive operations | No (audit only) |
| `cns.contract.*` | Spec contract lifecycle (proposed, accepted, violated) | Aggregated into quality scores |
| `cns.guard.violation` | Guard rule triggered | Event-based |
| `cns.qa.repair_exhausted` | QA repair attempts exhausted | Strong signal — escalate to Curator |
| `cns.architecture.seam.drift` | Architecture seam divergence | Triggers warnings |
| `cns.slo.evaluated` | SLO metric evaluation | SLO breach → Critical if SLO severity is Critical |
| `cns.federation.*` | Federation link lifecycle | Yes — link degradation |

For the full span catalog, see `docs/reference/cns-spans.md` (100+ entries across 11 domain enum types).

---

## 9. What to Do When You See a Critical Alert

1. **Identify the domain** — The alert message names the affected namespace (e.g., `cns.tool`)

2. **Check variety counters** — `kask cns variety` to see which namespace is deficient

3. **Check recent alerts** — `kask cns alerts` to see active alerts, or `kask cns subscribe --agent curator --spans <namespace>` to monitor a specific namespace

4. **Check the energy budget** — `kask cns health` shows gas status; depletion can cascade into tool failures

5. **Inspect pod state** — `kask pod list` then `kask pod status <pod_id>` to verify agents are healthy

6. **Review escalation log** — If the alert triggered a `DepletionSignal`, check agent pod escalation records

7. **Address the root cause**:
   - **Variety deficit**: The system is seeing too few distinct inputs — check connectivity, tool availability, or inference model health
   - **Gas depletion**: Increase the energy cap or reduce consumption
   - **Error rate spike**: Check `cns.tool.*` for tool failures, `cns.inference` for model errors
   - **SLO breach**: Review the breached service-level objective and its time window

8. **Escalate if unresolved** — Persistent critical alerts should be escalated to the Curator daemon for metacognitive review:

   ```bash
   kask curator escalations
   kask curator metacognition
   ```

---

## 10. Programmatic Access

Within Rust code, access CNS data through `CnsRuntime`:

```rust
use hkask_cns::CnsRuntime;

let rt = CnsRuntime::with_threshold(100);
let variety = rt.variety().await; // HashMap<SpanNamespace, u64>
let health = rt.health().await;   // CnsHealth { healthy, overall_deficit, ... }
let alerts = rt.alerts().await;   // Vec<RuntimeAlert>
```

---

## Related

- [CNS Span Registry](../reference/cns-spans.md) — Full span taxonomy (100+ entries)
- [Audit Sovereignty](audit-sovereignty.md) — Sovereign CNS spans for P1–P4 enforcement
- [Configure Content Guard](configure-guard.md) — `cns.guard.violation` spans
