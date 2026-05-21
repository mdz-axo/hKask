# hKask Bot Reporting Structure — Curator Metacognition

**Version:** v0.21.0  
**Date:** 2026-05-20  
**Status:** Complete — 7 bots + Curator replicant with reporting hierarchy

---

## Reporting Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                    hKask Administrator                       │
│                    (Human User)                              │
│                          ▲                                   │
│                          │                                   │
│              Escalation (critical only)                      │
│                          │                                   │
│                          ▼                                   │
│              ┌─────────────────────┐                         │
│              │   Curator           │                         │
│              │   (Replicant)       │                         │
│              │                     │                         │
│              │ Metacognition       │                         │
│              │ System sense-making │                         │
│              │ Administrator liaison│                        │
│              └──────────┬──────────┘                         │
│                         │                                   │
│         Receives reports from all bots                      │
│                         │                                   │
│         ┌───────────────┼───────────────┐                  │
│         ▼               ▼               ▼                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │ CNS Bot     │ │ Memory Bot  │ │ Inference   │          │
│  │             │ │             │ │ Bot         │          │
│  │ Variety     │ │ Confidence  │ │ Model       │          │
│  │ counters    │ │ anomalies   │ │ failures    │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
│         │               │               │                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │ MCP Bot     │ │ Ensemble    │ │ Git Bot     │          │
│  │             │ │ Bot         │ │             │          │
│  │ OCAP        │ │ Coordination│ │ Provenance  │          │
│  │ violations  │ │ conflicts   │ │ errors      │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
│                         │                                   │
│  ┌───────────────────────┴──────────────────────────────┐  │
│  │              Registry Dispatch Bot                   │  │
│  │              Template dispatch failures              │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Bot Reporting Configuration

Each bot manifest now includes a `reporting` section:

```yaml
reporting:
  escalate_to: Curator
  report_on: [specific_events]
  report_interval: on_event_and_hourly_summary
```

### Bot Reporting Details

| Bot | Escalates to | Reports on | Interval |
|-----|--------------|------------|----------|
| `cns-curator-bot` | Curator | variety_deficit > 100, alerts | On alert + hourly summary |
| `memory-curator-bot` | Curator | Confidence anomalies, retrieval failures | Hourly summary |
| `inference-curator-bot` | Curator | Model failures, high latency, cost overruns | Hourly summary |
| `mcp-dispatch-bot` | Curator | OCAP violations, rate limit exceeded | On event + hourly |
| `ensemble-curator-bot` | Curator | Coordination failures, session outcomes | Per session + hourly |
| `git-curator-bot` | Curator | Merge conflicts, CAS errors | On event + hourly |
| `registry-dispatch-bot` | Curator | Selection failures, low confidence | On event + hourly |

---

## Curator Responsibilities

The Curator replicant has three primary responsibilities:

### 1. Metacognition (System Sense-Making)

**Process:**
1. Gather system state from all bot reports
2. Synthesize CNS variety counters, alerts, bot performance
3. Select metacognitive operation (calibrate, diagnose, escalate, maintain)
4. Execute operation and emit CNS spans

**Templates:**
- `curator/system_state_gather.j2` — Synthesize bot reports
- `curator/metacognition-selector.j2` — Select operation
- `curator/metacognition-calibrate.j2` — Adjust thresholds
- `curator/metacognition-diagnose.j2` — Analyze issues
- `curator/metacognition-escalate.j2` — Notify administrator
- `curator/metacognition-maintain.j2` — Continue monitoring

### 2. Administrator Coordination

**Escalation Triggers:**
- Variety deficit > 100 (Ashby's Law violation)
- Bot coordination failure
- System degradation detected
- Energy budget critical

**Notification Channels:**
- Primary: `kask chat` (real-time)
- Secondary: system log (audit trail)
- Tertiary: email (if configured, critical only)

### 3. System Evolution Dialogue

The Curator maintains ongoing dialogue with the hKask Administrator about:
- System performance trends
- Bot effectiveness and energy budget adjustments
- Template evolution and cascade improvements
- CNS calibration and threshold tuning

---

## CNS Integration

All bot reports and Curator metacognition emit CNS spans:

| Span Namespace | Purpose | Emitted by |
|----------------|---------|------------|
| `cns.tool.invocation` | Tool call tracking | All bots |
| `cns.tool.result` | Tool outcome recording | All bots |
| `cns.prompt.select` | Template selection | registry-dispatch-bot |
| `cns.prompt.render` | Template rendering | registry-dispatch-bot |
| `cns.prompt.outcome` | Execution result | All bots |
| `cns.prompt.metacognition` | Curator metacognition | Curator |
| `cns.agent_pod.activated` | Bot lifecycle | CNS monitoring |
| `cns.alert.*` | Algedonic alerts | CNS monitoring |

**Variety Counter Monitoring:**
- CNS curator bot monitors variety deficit across all spans
- Threshold: deficit > 100 → escalate to Curator
- Curator can escalate to Administrator if systemic

---

## Energy Budget Model

Each bot and the Curator have energy budgets:

| Agent | Energy Cap | Alert Threshold | Hard Limit |
|-------|------------|-----------------|------------|
| CNS Bot | 12,000 | 0.8 | Yes |
| Memory Bot | 12,000 | 0.8 | Yes |
| Inference Bot | 12,000 | 0.8 | Yes |
| MCP Bot | 12,000 | 0.8 | Yes |
| Ensemble Bot | 12,000 | 0.8 | Yes |
| Git Bot | 12,000 | 0.8 | Yes |
| Registry Dispatch Bot | 12,000 | 0.8 | Yes |
| **Curator** | **15,000** | **0.8** | **Yes** |

**Rationale:** Curator has higher cap to handle metacognitive synthesis across all bots.

---

## Reporting Flow Example

### Scenario: CNS Variety Deficit Approaching Threshold

1. **CNS Bot** detects variety deficit = 85 (threshold: 100)
   - Emits `cns.alert.variety_deficit` span
   - Reports to Curator (hourly summary includes trend)

2. **Curator** receives report
   - Synthesizes system state via `system_state_gather.j2`
   - Selects metacognitive operation via `metacognition-selector.j2`
   - Operation: `calibrate` (deficit elevated but not critical)

3. **Curator** executes calibration
   - Renders `metacognition-calibrate.j2`
   - Adjusts bot thresholds to increase variety generation
   - Emits `cns.prompt.metacognition` span

4. **System** responds
   - Variety deficit stabilizes at ~60
   - CNS Bot reports normalized state in next hourly summary

5. **Curator** confirms maintenance mode
   - Renders `metacognition-maintain.j2`
   - No Administrator escalation required

---

## File Inventory

### Bot Manifests (8 total)

```
registry/bots/
├── Curator.yaml                    # Replicant manifest
├── registry-dispatch-bot.yaml
├── cns-curator-bot.yaml
├── memory-curator-bot.yaml
├── inference-curator-bot.yaml
├── mcp-dispatch-bot.yaml
├── ensemble-curator-bot.yaml
└── git-curator-bot.yaml
```

### Dispatch Manifests (8 total)

```
registry/manifests/
├── curator-metacognition.yaml      # Curator metacognition process
├── dispatch.yaml                   # Registry dispatch
├── cns-monitoring.yaml             # CNS monitoring
├── memory-ops.yaml                 # Memory operations
├── inference-dispatch.yaml         # Inference dispatch
├── mcp-dispatch.yaml               # MCP tool dispatch
├── ensemble-orchestration.yaml     # Ensemble coordination
└── git-ops.yaml                    # Git CAS operations
```

### Curator Templates (6 total)

```
registry/templates/curator/
├── system_state_gather.j2          # Synthesize bot reports
├── metacognition-selector.j2       # Select operation
├── metacognition-calibrate.j2      # Adjust thresholds
├── metacognition-diagnose.j2       # Analyze issues
├── metacognition-escalate.j2       # Notify Administrator
├── metacognition-maintain.j2       # Continue monitoring
```

---

## Administrator Commands

The hKask Administrator can interact with Curator via:

```bash
# System health check
kask chat "system health?"

# View bot status
kask bot list --status

# View CNS alerts
kask cns alerts --active

# View Curator metacognition history
kask cns spans --namespace cns.prompt.metacognition

# Adjust Curator energy budget (advanced)
kask bot manifest push Curator.yaml --energy-cap 20000
```

---

## Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Curator as metacognitive agent** | Single replicant responsible for system sense-making |
| **Bot reporting hierarchy** | All bots report to Curator, Curator reports to Administrator |
| **Escalation on threshold** | Variety deficit > 100 → Curator → Administrator |
| **Metacognition via templates** | Jinja2 templates for calibrate/diagnose/escalate/maintain |
| **CNS observability** | All metacognition emits spans for audit trail |
| **Energy budget governance** | Curator can calibrate bot budgets based on performance |

---

## Next Steps

### Immediate (v1.1)
1. **Implement Curator persona** — Port from stack-cli Curator
2. **Wire bot report ingestion** — Curator receives and synthesizes reports
3. **Test metacognition flow** — End-to-end: gather → select → execute → emit
4. **Administrator notification** — kask chat integration for escalation

### Medium-Term
5. **Bot report standardization** — Common schema for all bot reports
6. **Curator learning** — Adjust thresholds based on historical performance
7. **Multi-bot ensemble** — Curator orchestrates bot deliberation on complex issues

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Curator: metacognitive replicant, system sense-maker, Administrator liaison.*
*Bots report to Curator. Curator synthesizes and escalates when needed.*