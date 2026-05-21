# hKask Standing Ensemble Session — System Coordination Chat

**Version:** v0.21.0  
**Date:** 2026-05-20  
**Status:** Complete — Persistent group chat for bot coordination

---

## Overview

The **Standing Ensemble Session** is a persistent A2A group chat where:

- All 7 system bots report status and receive instructions
- Curator orchestrates metacognition and system coordination
- hKask Administrator (human) can observe and participate anytime
- All messages are persisted to memory and archived to Git
- Session auto-starts on system bootstrap

**Key Properties:**
- Type: Standing (always open)
- Visibility: Shared (Administrator can observe)
- Participants: 8 (Curator + 7 bots)
- Orchestration: Curator-led (no swarm consensus)
- Retention: 1000 messages + summaries + Git archival

---

## Session Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Standing Ensemble Session                    │
│              "System Coordination Standing Session"             │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Curator (Replicant) — Orchestrator                       │ │
│  │  - Synthesizes bot reports                                │ │
│  │  - Issues instructions                                    │ │
│  │  - Emits metacognition updates                            │ │
│  │  - Escalates to Administrator                             │ │
│  └───────────────────────────────────────────────────────────┘ │
│                              │                                  │
│         All communication flows through Curator                 │
│                              │                                  │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Participant Bots (7 total)                               │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐      │ │
│  │  │ CNS Bot      │ │ Memory Bot   │ │ Inference    │      │ │
│  │  │              │ │              │ │ Bot          │      │ │
│  │  └──────────────┘ └──────────────┘ └──────────────┘      │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐      │ │
│  │  │ MCP Bot      │ │ Ensemble Bot │ │ Git Bot      │      │ │
│  │  │              │ │              │ │              │      │ │
│  │  └──────────────┘ └──────────────┘ └──────────────┘      │ │
│  │  ┌──────────────┐                                         │ │
│  │  │ Registry     │                                         │ │
│  │  │ Dispatch Bot │                                         │ │
│  │  └──────────────┘                                         │ │
│  └───────────────────────────────────────────────────────────┘ │
│                              │                                  │
│         Administrator can observe/participate                   │
│                              ▼                                  │
│              ┌─────────────────────┐                           │
│              │ hKask Administrator │                           │
│              │ (Human User)        │                           │
│              └─────────────────────┘                           │
└─────────────────────────────────────────────────────────────────┘
```

---

## Message Types

| Type | From | To | Priority | Purpose |
|------|------|-----|----------|---------|
| `status_report` | Bots | All | Normal | Hourly status + metrics |
| `metacognition_update` | Curator | All | High | System state synthesis |
| `alert` | Any bot | All | Critical | Immediate notification |
| `instruction` | Curator | Specific bot(s) | High | Directives to bots |
| `administrator_message` | Administrator | All | Critical | Human input |

---

## Session Flow

### Bootstrap (System Start)

```
1. System initializes
2. Standing session auto-starts
3. Curator sends initial message:
   "All participants: report status."
4. Each bot sends status report
5. Session is now active
```

### Normal Operation (Hourly Cycle)

```
1. Each bot sends hourly status report
2. Curator synthesizes all reports
3. Curator emits metacognition update:
   - System health assessment
   - Actions taken
   - Calibrations made
   - Active alerts
4. Cycle repeats hourly
```

### Event-Driven Flow

```
1. Bot detects issue (e.g., variety deficit > 80)
2. Bot sends alert to standing session
3. Curator receives alert
4. Curator diagnoses via metacognition
5. Curator issues instruction to relevant bot(s)
6. Bot(s) acknowledge and execute
7. Bot(s) report completion
8. Curator updates system state
```

### Administrator Participation

```
1. Administrator joins via `kask chat`
2. Curator renders administrator view
3. Administrator can:
   - Observe all messages
   - Request bot status
   - Issue instructions
   - Override Curator decisions
4. Administrator can leave anytime
5. Session continues autonomously
```

---

## Bot Reporting Structure

Each bot manifest now includes:

```yaml
reporting:
  escalate_to: Curator
  report_to: standing_ensemble_session
  report_on: [specific_events]
  report_interval: hourly_summary

standing_session:
  session_id: system-coordination-standing-session
  role: participant
  report_interval: hourly
  administrator_visible: true
```

### Reporting Schedule

| Bot | Scheduled | On Event | On Curator Request |
|-----|-----------|----------|-------------------|
| CNS Bot | Hourly | Variety alerts | Yes |
| Memory Bot | Hourly | Confidence anomalies | Yes |
| Inference Bot | Hourly | Model failures | Yes |
| MCP Bot | Hourly | OCAP violations | Yes |
| Ensemble Bot | Hourly | Coordination issues | Yes |
| Git Bot | Hourly | CAS errors | Yes |
| Registry Dispatch Bot | Hourly | Selection failures | Yes |

---

## Curator Authority

The Curator has authority to:

| Action | Scope | Limit |
|--------|-------|-------|
| Request status | Any bot | None |
| Calibrate thresholds | Any bot | Within safe ranges |
| Adjust energy budgets | Any bot | ±25% of cap |
| Issue instructions | Any bot | Per charter |
| Spawn sub-ensembles | Up to 4 bots | Ad-hoc only |
| Escalate to Administrator | System-wide | On critical triggers |

---

## Administrator Commands

```bash
# View standing session
kask session view system-coordination-standing-session

# Observe messages (read-only)
kask session observe system-coordination-standing-session

# Participate in session
kask session participate system-coordination-standing-session

# Request bot status
kask session status cns-curator-bot

# Issue instruction to bot
kask session instruct memory-curator-bot "Recall recent triples"

# View system health
kask session health

# View active alerts
kask session alerts

# Request calibration
kask session calibrate

# View escalations
kask session escalations
```

---

## CNS Integration

All standing session messages emit CNS spans:

```yaml
cns:
  namespace: cns.ensemble.standing_session
  span_types:
    - cns.ensemble.standing_session.message
    - cns.ensemble.standing_session.status_report
    - cns.ensemble.standing_session.metacognition
    - cns.ensemble.standing_session.alert
    - cns.ensemble.standing_session.instruction
```

**Variety Counter:**
- Messages contribute to variety generation
- Curator monitors variety deficit via CNS spans
- Deficit > 100 → escalate to Administrator

---

## Memory Persistence

All session activity is persisted:

```yaml
memory:
  persist_to: episodic_memory
  visibility: shared
  index_for_recall: true
  schema:
    - session_id
    - message_type
    - from_agent
    - to_agent
    - content
    - timestamp
    - cns_span_ref
```

**Recall Queries:**
- "What did CNS Bot report at 14:00?"
- "Show all alerts from the last hour"
- "What calibrations did Curator make today?"

---

## Git Archival

Session is archived to Git CAS:

```yaml
git:
  archive_interval: daily
  archive_path: registry/ensemble-archives/system-coordination
  format: markdown + JSON
  include_summaries: true
  sha_versioned: true
```

**Archive Structure:**
```
registry/ensemble-archives/system-coordination/
├── 2026-05-20/
│   ├── messages.json
│   ├── summaries.md
│   └── SHA
├── 2026-05-21/
│   ├── messages.json
│   ├── summaries.md
│   └── SHA
```

---

## Health Monitoring

Session health is monitored:

| Status | Conditions |
|--------|------------|
| **Healthy** | ≥5 participants, Curator responsive, latency <5s |
| **Degraded** | 4 participants, Curator unresponsive >60s, latency >10s |
| **Critical** | <3 participants, Curator unresponsive >5min, variety deficit >100 |

**Recovery:**
- Degraded → Curator emits alert, requests Administrator attention
- Critical → Curator escalates to Administrator immediately

---

## Energy Budget

| Component | Cap | Cost | Alert | Hard Limit |
|-----------|-----|------|-------|------------|
| Session total | 50,000 | 100/message | 80% | Yes |
| Curator | 15,000 | — | 80% | Yes |
| Each bot | 12,000 | — | 80% | Yes |

**Rationale:** Standing session has dedicated budget separate from bot operations.

---

## Files Created

### Session Manifest
- `registry/manifests/standing-ensemble-session.yaml` — Standing session configuration

### Templates
- `registry/templates/ensemble/standing_session_status_report.j2` — Bot status template
- `registry/templates/ensemble/standing_session_curator_instruction.j2` — Curator instruction template
- `registry/templates/ensemble/standing_session_metacognition_update.j2` — Metacognition update template
- `registry/templates/ensemble/standing_session_administrator_view.j2` — Administrator view template

### Bot Manifests Updated
All 8 bot manifests updated with `standing_session:` section:
- `Curator.yaml` — Orchestrator role
- `cns-curator-bot.yaml` — Participant
- `memory-curator-bot.yaml` — Participant
- `inference-curator-bot.yaml` — Participant
- `mcp-dispatch-bot.yaml` — Participant
- `ensemble-curator-bot.yaml` — Participant
- `git-curator-bot.yaml` — Participant
- `registry-dispatch-bot.yaml` — Participant

---

## Example Session Transcript

```
=== System Coordination Standing Session ===
Session ID: system-coordination-standing-session
Started: 2026-05-20T08:00:00-07:00
Participants: 8 (Curator + 7 bots)

---

[08:00:00] Curator:
  ## System Coordination Standing Session — Initialized
  
  All participants: report status.
  Administrator: session is open for observation/participation.

[08:00:05] cns-curator-bot:
  ## Status Report — cns-curator-bot
  Status: healthy
  Variety deficit: 45/100
  Active alerts: 0
  Energy: 2,341/12,000

[08:00:06] memory-curator-bot:
  ## Status Report — memory-curator-bot
  Status: healthy
  Triples stored: 15,234
  Retrieval success: 98.7%
  Energy: 1,892/12,000

[08:00:07] inference-curator-bot:
  ## Status Report — inference-curator-bot
  Status: healthy
  Inferences today: 342
  Avg latency: 847ms
  Energy: 3,421/12,000

... [other bots report]

[08:00:15] Curator:
  ## Metacognition Update — System State
  
  System Health: healthy
  Variety Deficit: 45/100 (stable)
  Active Alerts: 0
  
  All systems nominal. Continue monitoring.
  Next synthesis: 09:00:00

---

[14:23:45] cns-curator-bot:
  ## ⚠️ ALERT — Variety Deficit Elevated
  
  Severity: medium
  Current deficit: 78/100
  Trend: increasing (+33 in last hour)
  Recommended action: Calibrate bot thresholds

[14:23:46] Curator:
  ## Instruction — cns-curator-bot
  
  Acknowledged. Initiating calibration.
  
  [Renders metacognition-calibrate.j2]
  
  Calibrating bot variety generation thresholds...
  
  ## Metacognition Update — Calibration Complete
  
  Adjustments made:
  - CNS Bot: variety sampling rate increased 1.2x
  - Memory Bot: confidence decay slowed 0.9x
  - Inference Bot: model tier diversity increased
  
  Expected deficit reduction: -25 within 2 hours.

---

[Administrator joins at 15:00:00]

[15:00:01] hKask-Administrator:
  /session health

[15:00:02] Curator:
  # System Coordination Standing Session
  
  Status: active
  Participants: 8
  System Health: healthy
  Variety Deficit: 62/100 (decreasing)
  
  [Full administrator view rendered]

[15:00:15] hKask-Administrator:
  Good. Keep monitoring.

[15:00:16] Curator:
  Acknowledged. Continuing monitoring.
```

---

## Next Steps

### Immediate (v1.1)
1. **Implement ensemble MCP server** — `hkask-mcp-ensemble` with standing session support
2. **Wire session bootstrap** — Auto-start on system initialization
3. **Implement Curator orchestration** — Render templates, emit to session
4. **Administrator CLI** — `kask session` commands for observe/participate

### Medium-Term
5. **Sub-ensemble spawning** — Curator can spawn ad-hoc bot groups
6. **Session summaries** — Hourly/daily synthesis to markdown
7. **Git archival pipeline** — Daily commits to CAS

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Standing session: always-on group chat for system coordination.*
*Curator orchestrates. Bots report. Administrator can join anytime.*
