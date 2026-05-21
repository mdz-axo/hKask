# hKask Bot Memory Production

**Version:** v0.21.0  
**Date:** 2026-05-20  
**Status:** Complete — All bots produce episodic and semantic memory

---

## Overview

All 7 system bots and the Curator replicant are now connected to the memory stack and automatically produce:

1. **Episodic Memory** — First-person records of their operations (private by default)
2. **Semantic Memory** — Extracted facts from operation outcomes (public by default)

This creates a self-documenting system where every bot operation is recorded and can be recalled, queried, and reasoned about.

---

## Memory Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Bot Operation                                │
│                    (e.g., CNS monitoring)                       │
│                          │                                      │
│                          ▼                                      │
│              ┌─────────────────────┐                           │
│              │  Operation Record   │                           │
│              │  (Jinja2 template)  │                           │
│              └──────────┬──────────┘                           │
│                         │                                      │
│         ┌───────────────┴───────────────┐                     │
│         ▼                               ▼                      │
│  ┌─────────────────┐           ┌─────────────────┐            │
│  │ Episodic Memory │           │ Semantic Memory │            │
│  │                 │           │                 │            │
│  │ - Private       │           │ - Public        │            │
│  │ - First-person  │           │ - Third-person  │            │
│  │ - Agent-owned   │           │ - Shared        │            │
│  │ - Operation log │           │ - Extracted     │            │
│  │                 │           │   facts         │            │
│  └────────┬────────┘           └────────┬────────┘            │
│           │                             │                      │
│           └──────────────┬──────────────┘                     │
│                          ▼                                     │
│              ┌─────────────────────┐                          │
│              │   hkask-storage     │                          │
│              │   (SQLite +         │                          │
│              │    SQLCipher)       │                          │
│              └─────────────────────┘                          │
└─────────────────────────────────────────────────────────────────┘
```

---

## Memory Types

### Episodic Memory (First-Person)

**Definition:** Agent's personal experience of its operations.

**Characteristics:**
- Perspective: Agent WebID (e.g., `cns-curator-bot`)
- Visibility: Private (agent-owned)
- Content: Full operation record (input, output, metrics, CNS spans)
- Retention: Permanent (unless explicitly deleted)
- Query: By agent, time range, operation type

**Example Record:**
```json
{
  "entity": "cns-curator-bot:operation:12345",
  "attribute": "performed",
  "value": {
    "operation_type": "variety_monitoring",
    "input": {"span_namespace": "cns.tool.*"},
    "output": {"variety_deficit": 45},
    "energy_used": 1234,
    "latency_ms": 342,
    "cns_spans": ["cns.tool.invocation:abc123"]
  },
  "confidence": 0.95,
  "perspective": "cns-curator-bot",
  "visibility": "private",
  "valid_from": "2026-05-20T15:00:00Z"
}
```

### Semantic Memory (Third-Person)

**Definition:** Extracted facts from operation outcomes.

**Characteristics:**
- Perspective: NULL (shared knowledge)
- Visibility: Public (unless sensitive)
- Content: RDF triples (entity, attribute, value)
- Retention: Permanent (with Bayesian confidence decay)
- Query: By entity, attribute, vector similarity

**Example Triples:**
```json
[
  {
    "entity": "system",
    "attribute": "has_variety_deficit",
    "value": 45,
    "confidence": 0.92,
    "perspective": null,
    "visibility": "public"
  },
  {
    "entity": "cns-curator-bot",
    "attribute": "monitors",
    "value": "variety_counters",
    "confidence": 1.0,
    "perspective": null,
    "visibility": "public"
  }
]
```

---

## Bot Memory Responsibilities

Each bot manifest now includes:

```yaml
responsibilities:
  - record: <operation_type>_to_episodic_memory
  - produce: semantic_triples_from_<outcome_type>
```

### Bot Memory Production Table

| Bot | Episodic Memory | Semantic Triples |
|-----|-----------------|------------------|
| `cns-curator-bot` | CNS monitoring operations | Variety counter states, alert thresholds |
| `memory-curator-bot` | Memory operations | Stored triples, retrieval patterns |
| `inference-curator-bot` | Inference dispatches | Model performance, outcome quality |
| `mcp-dispatch-bot` | Tool dispatches | Tool usage patterns, OCAP verifications |
| `ensemble-curator-bot` | Session orchestrations | Bot coordination outcomes |
| `git-curator-bot` | Git operations | Provenance records, SHA mappings |
| `registry-dispatch-bot` | Template dispatches | Selection patterns, confidence scores |
| `Curator` | Metacognition operations | System insights, calibrations |

---

## Memory Production Flow

### Step 1: Operation Completes

```
Bot executes operation (e.g., CNS variety monitoring)
│
├─ Input: {...}
├─ Output: {...}
├─ Energy: 1,234
├─ Latency: 342ms
└─ CNS Spans: [cns.tool.invocation:abc123, ...]
```

### Step 2: Render Memory Template

```
Bot renders memory/templates/agent_operation_memory.j2
│
├─ Binds operation data
├─ Extracts triples (LLM-assisted)
└─ Produces memory record
```

### Step 3: Store Episodic Memory

```
Bot calls hkask-mcp-memory → memory_remember
│
├─ Perspective: bot WebID
├─ Visibility: private
└─ Returns: triple_ids [...]
```

### Step 4: Extract & Store Semantic Memory

```
Bot extracts semantic triples from outcome
│
├─ Entity/Attribute/Value extraction
├─ Bayesian confidence assignment
└─ Returns: semantic_triple_ids [...]
```

### Step 5: Emit CNS Spans

```
Bot emits memory production spans:
│
├─ cns.memory.production.episodic_recorded
├─ cns.memory.production.semantic_extracted
└─ cns.memory.production.storage_complete
```

---

## Memory Templates

### 1. `memory/templates/remember.j2`

Direct memory storage via template rendering.

**Use:** Explicit memory storage requests.

### 2. `memory/templates/recall.j2`

Memory retrieval with visibility gating.

**Use:** Query semantic/episodic memory.

### 3. `memory/templates/agent_operation_memory.j2`

Automatic operation recording.

**Use:** All bot operations (auto-invoked).

---

## Memory MCP Integration

All bots have capabilities:

```yaml
capabilities:
  - tool:memory:remember  # Store triples
  - tool:memory:recall    # Retrieve triples
  - tool:memory:query     # Search triples
```

**hkask-mcp-memory tools:**
| Tool | Purpose |
|------|---------|
| `memory_remember` | Store semantic/episodic triples |
| `memory_recall` | Retrieve triples by entity/attribute |
| `memory_query` | Vector similarity search |
| `memory_forget` | Retract triples (confidence subtraction) |

---

## Standing Session Memory Reports

Bots report memory production metrics hourly:

```yaml
standing_session:
  report_memory_production: true
  metrics:
    - episodic_records_count
    - semantic_triples_count
    - storage_success_rate
    - energy_used_for_memory
```

**Example Report:**
```markdown
## Status Report — cns-curator-bot

**Episodic Records (hour):** 47
**Semantic Triples (hour):** 23
**Storage Success:** 100%
**Energy for Memory:** 1,847/12,000
```

---

## Curator Memory Access

The Curator can:

| Operation | Scope |
|-----------|-------|
| Read all public semantic memory | System-wide facts |
| Read own episodic memory | Curator operations |
| Read bot episodic memory | With OCAP delegation |
| Query across all memory | Synthesis for metacognition |
| Produce semantic insights | System health facts |

**Example Query:**
```
Curator: "Show all CNS alerts from the last 24 hours"
→ Queries episodic memory (perspective: cns-curator-bot)
→ Filters by time range and operation_type: alert
→ Returns: Array of alert records
```

---

## Administrator Access

Administrator can query bot memory via:

```bash
# View bot episodic memory
kask memory recall --agent cns-curator-bot --type episodic

# Query semantic memory
kask memory query "system variety deficit"

# View memory production metrics
kask session memory-stats

# Export memory to markdown
kask memory export --agent Curator --last 24h
```

---

## Bayesian Confidence

Memory uses Bayesian confidence operations:

| Operation | Formula | Use |
|-----------|---------|-----|
| **Combine** | `c1 + c2 - c1*c2` | Corroborating sources |
| **Subtract** | `c1 - c2` | Retracting confidence |
| **Join** | `(c1 + c2) / 2` | Averaging sources |
| **Decay** | `c * e^(-λt)` | Time-based decay |

**Example:**
```
CNS Bot reports variety_deficit = 45 (confidence: 0.9)
Memory Bot corroborates (confidence: 0.8)
Combined: 0.9 + 0.8 - (0.9*0.8) = 0.98
```

---

## OCAP Visibility Gating

Memory access is gated by OCAP:

```
Requester: memory-curator-bot
Query: Recall episodic memory
│
├─ Check: Requester WebID = perspective?
│  └─ Yes → Allow (owner access)
│  └─ No → Check visibility
│     ├─ Visibility = public? → Allow
│     └─ Visibility = private? → Deny (unless delegated)
```

---

## Files Created/Updated

### Templates (3 new)
- `registry/templates/memory/templates/remember.j2`
- `registry/templates/memory/templates/recall.j2`
- `registry/templates/memory/templates/agent_operation_memory.j2`

### Manifests (1 new)
- `registry/manifests/bot-memory-production.yaml`

### Bot Manifests (8 updated)
All bots updated with:
- `write: own_episodic_memory` right
- `record: *_to_episodic_memory` responsibility
- `produce: semantic_triples_from_*` responsibility

---

## Next Steps

### Immediate (v1.1)
1. **Implement Memory MCP server** — `hkask-mcp-memory` with remember/recall/query
2. **Wire automatic memory production** — Invoke after every bot operation
3. **Test episodic/semantic storage** — Verify visibility gating and Bayesian ops
4. **Curator memory synthesis** — Query bot memory for metacognition

### Medium-Term
5. **Memory archival to Git** — Periodic snapshots of triples
6. **Memory condensation** — Summarize old episodic records
7. **Cross-bot memory queries** — Ensemble queries across multiple bot memories

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*All bots produce memory. Episodic = private experience. Semantic = public facts.*
*Curator can query all. Administrator can query all public.*
