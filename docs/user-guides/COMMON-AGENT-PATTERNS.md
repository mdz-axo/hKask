---
title: "Common Agent Patterns and Templates"
audience: [developers, agent designers]
last_updated: 2026-05-24
version: "0.21.0"
status: "Active"
domain: "Application"
ddmvss_categories: [domain]
---

# Common Agent Patterns and Templates

This document catalogs the most common agent patterns and their corresponding templates for loading agents into agent pods in hKask.

---

## Contents

| Section | Description |
|---------|-------------|
| [Agent Pattern Taxonomy](#agent-pattern-taxonomy) | Four primary agent patterns overview |
| [Pattern 1: Specialist Bot](#pattern-1-specialist-bot) | Domain-specific expert bot pattern |
| [Pattern 2: Curator Bot](#pattern-2-curator-bot) | System oversight and coordination bot |
| [Pattern 3: Dispatch Bot](#pattern-3-dispatch-bot) | Template routing and execution bot |
| [Pattern 4: Replicant Assistant](#pattern-4-replicant-assistant) | Human-facing personal assistant |
| [Pattern 5: Bridge Agent](#pattern-5-bridge-agent) | External workspace integration agent |
| [Template Library](#template-library) | Complete template catalog with YAML |
| [Quick Start Templates](#quick-start-templates) | Copy-paste ready template snippets |
| [Template Generator](#template-generator) | Generator for new agent templates |
| [Next Steps](#next-steps) | Follow-up actions and resources |

---

## Agent Pattern Taxonomy

Agents in hKask fall into four primary patterns[^gamma1994][^wooldridge2009]:

| Pattern | Type | Purpose | Example |
|---------|------|---------|---------|
| **Specialist Bot** | Bot | Domain-specific expert operations | `memory-curator-bot` |
| **Curator Bot** | Bot | System oversight and coordination | `Curator` |
| **Dispatch Bot** | Bot | Template routing and execution | `registry-dispatch-bot` |
| **Replicant Assistant** | Replicant | Human user assistance | Personal assistant |
| **Bridge Agent** | Bot | External workspace integration | Cross-workspace bridge |

---

## Pattern 1: Specialist Bot

### Purpose
Domain-specific expert bot that performs specialized operations[^gamma1994] (memory, CNS, inference, etc.)

### Files Required
```
specialist-bot-crate/
├── Cargo.toml
├── agent_persona.yaml
├── dispatch_manifest.yaml
├── hlexicon.yaml
└── templates/
    ├── selectors/
    │   └── operation-selector.j2
    ├── prompts/
    │   └── specialist-prompt.j2
    └── cognitions/
        └── specialist-cognition.j2
```

### Persona Template
```yaml
agent:
  name: memory-specialist-bot
  type: Bot
  binding_contract: true
  editor: curator

charter:
  description: Specialist bot for memory operations including recall, storage, and embedding
  archetype: Specialist
  visibility: Secondary

capabilities:
  - tool:memory:recall
  - tool:memory:remember
  - tool:memory:embed
  - tool:memory:query

rights:
  - read: public_semantic_memory
  - read: own_episodic_memory
  - write: own_episodic_memory
  - write: public_semantic_memory

responsibilities:
  - respond_to: memory_queries
  - emit: cns.memory.operation
  - record: operations_to_episodic_memory
  - produce: semantic_triples_from_memory_ops

visibility:
  default: "public"
  episodic_override: "private"

process_manifest: registry/manifests/memory-ops.yaml

depends_on:
   - hkask-mcp-registry

readiness_probe:
  type: health_check
  endpoint: memory_specialist::status
  expected:
    memory_store_available: true
    embedding_model_ready: true
  timeout_seconds: 10
  retry_count: 3
```

### Dispatch Manifest
```yaml
manifest:
  name: memory-ops
  version: "0.1.0"
  description: Memory operations dispatch workflow

matroshka:
  max_depth: 7
  enforce: true
  depth_counter:
    enabled: true
    inherit_from_parent: true
    default: 0
    increment_on_dispatch: true
  cns_monitoring:
    span_namespace: cns.memory.matroshka_depth
    alert_if_exceeds: 6

steps:
  - ordinal: 1
    action: select
    template_ref: registry/templates/memory/selectors/operation-selector.j2
    renderer: minijinja
    model_tier: fast_local
    output_schema:
      type: object
      properties:
        operation_type:
          type: string
        confidence:
          type: number

  - ordinal: 2
    action: populate
    template_ref: "${operation_type}"
    renderer: minijinja
    bindings:
      query: "${input.query}"
      memory_type: "${input.memory_type}"

  - ordinal: 3
    action: execute
    target: memory
    contract:
       mcp: hkask-mcp-registry
      model_tier: balanced

cns:
  spans:
    - cns.memory.operation
    - cns.memory.outcome
```

### Selector Template
```jinja2
{# File: templates/selectors/operation-selector.j2 #}

You are a memory operation selector.

## Available Operations

1. **recall** — Retrieve memories from semantic/episodic store
2. **remember** — Store new memories
3. **embed** — Generate embeddings for content
4. **query** — Query memory database

## User Request

{{ query }}

## Memory Type

{{ memory_type | default('semantic') }}

## Selection Criteria

- Match operation to user intent
- Consider memory type (semantic vs episodic)
- Apply visibility constraints

## Response

```json
{
  "operation_type": "recall|remember|embed|query",
  "rationale": "...",
  "confidence": 0.0
}
```
```

### Existing Examples
- `registry/bots/memory-curator-bot.yaml`
- `registry/manifests/memory-ops.yaml`
- `registry/templates/memory/templates/recall.j2`
- `registry/templates/memory/templates/remember.j2`

---

## Pattern 2: Curator Bot

### Purpose
System oversight, metacognition, and coordination of other bots[^ashby1956]

### Files Required
```
curator-bot-crate/
├── Cargo.toml
├── agent_persona.yaml
├── dispatch_manifest.yaml
├── hlexicon.yaml
└── templates/
    ├── selectors/
    │   └── metacognition-selector.j2
    ├── prompts/
    │   └── system_state_gather.j2
    └── cognitions/
        └── metacognition-*.j2
```

### Persona Template
```yaml
agent:
  name: domain-curator-bot
  type: Bot
  binding_contract: true
  editor: hKask-Administrator

charter:
  description: Curator bot responsible for domain oversight, metacognition, and bot coordination
  archetype: MaintenanceAdvisory
  visibility: Primary

capabilities:
  - tool:cns:emit
  - tool:cns:variety
  - tool:cns:calibrate
  - tool:memory:recall
  - tool:inference:call
  - tool:ensemble:coordinate

rights:
  - read: all_public_semantic_memory
  - read: cns_spans_all
  - read: variety_counters_all
  - read: bot_reports_all
  - write: own_episodic_memory
  - write: public_semantic_memory
  - execute: metacognition_ops
  - coordinate: bot_ensemble_sessions

responsibilities:
  - monitor: domain_health_via_cns
  - synthesize: bot_reports_into_system_state
  - perform: metacognition_on_system_performance
  - calibrate: bot_energy_budgets_and_thresholds
  - escalate: critical_alerts_to_administrator
  - orchestrate: standing_ensemble_session

reporting:
  receives_from:
    - specialist-bot-1
    - specialist-bot-2
  report_to: standing_ensemble_session
  escalate_to: Curator
  escalation_triggers:
    - variety_deficit_gt_100
    - bot_coordination_failure
    - system_degradation_detected

standing_session:
  session_id: system-coordination-standing-session
  role: orchestrator
  report_interval: hourly
  administrator_visible: true

process_manifest: registry/manifests/domain-curation.yaml

depends_on:
  - hkask-mcp-cns
  - hkask-mcp-inference

readiness_probe:
  type: health_check
  endpoint: curator::metacognition_status
  expected:
    cns_spans_accessible: true
    bot_reports_available: true
    metacognition_templates_loaded: true
  timeout_seconds: 15
  retry_count: 3
```

### CNS Spans
- `cns.prompt.metacognition` — Metacognition events
- `cns.prompt.calibrate` — Calibration events
- `cns.prompt.escalate` — Escalation events
- `cns.ensemble.coordination` — Coordination events

### Existing Examples
- `registry/bots/Curator.yaml`
- `registry/bots/cns-curator-bot.yaml`
- `registry/bots/inference-curator-bot.yaml`
- `registry/manifests/curator-metacognition.yaml`
- `registry/manifests/metacognition.yaml`
- `registry/templates/curator/metacognition-*.j2`

---

## Pattern 3: Dispatch Bot

### Purpose
Template routing, selection, and execution orchestration[^mcp_spec]

### Files Required
```
dispatch-bot-crate/
├── Cargo.toml
├── agent_persona.yaml
├── dispatch_manifest.yaml
├── hlexicon.yaml
└── templates/
    ├── selectors/
    │   └── selector.j2
    ├── prompts/
    │   └── prompt_render.j2
    └── processes/
        └── dispatch.j2
```

### Persona Template
```yaml
bot:
  name: registry-dispatch-bot
  type: Bot
  binding_contract: true
  editor: curator-or-human-admin

capabilities:
  - tool:inference:call
  - tool:template:render
  - tool:registry:index
  - tool:registry:discover
  - tool:registry:validate

rights:
  - read: registry_index
  - read: template_catalog
  - execute: template_dispatch
  - write: own_episodic_memory

responsibilities:
  - respond_to: template_dispatch_requests
  - emit: cns.prompt.select
  - emit: cns.prompt.render
  - emit: cns.prompt.outcome
  - enforce: matroshka_depth_limit
  - report_to: Curator
  - record: dispatch_operations_to_episodic_memory

reporting:
  escalate_to: Curator
  report_to: standing_ensemble_session
  report_on: [selection_failures, low_confidence_dispatches, template_errors]
  report_interval: on_event_and_hourly_summary

standing_session:
  session_id: system-coordination-standing-session
  role: participant
  report_interval: hourly
  administrator_visible: true

process_manifest: registry/manifests/dispatch.yaml

depends_on:
  - hkask-mcp-registry
  - hkask-mcp-inference

readiness_probe:
  type: health_check
  endpoint: registry::dispatch_status
  expected:
    registry_index_available: true
    template_selector_ready: true
  timeout_seconds: 10
  retry_count: 3
```

### Dispatch Workflow (Matroshka Pattern)

The matroshka pattern applies depth-limited recursive execution for nested template dispatch.[^abelson1996]
```yaml
steps:
  - ordinal: 1
    action: select
    template_ref: registry/selectors/selector.j2
    renderer: minijinja
    model_tier: fast_local
    
  - ordinal: 2
    action: populate
    template_ref: "${selected_template_id}"
    renderer: minijinja
    bindings:
      raw_prompt: "${input.raw_prompt}"
      context: "${input.context}"
    
  - ordinal: 3
    action: execute
    target: "${template.contract.target}"
    contract: "${template.contract}"
    mcp: "${template.contract.mcp}"
    model_tier: "${template.contract.model_tier}"
```

### CNS Spans
- `cns.prompt.select` — Template selection
- `cns.prompt.render` — Template rendering
- `cns.prompt.outcome` — Execution result
- `cns.prompt.matroshka_depth` — Recursion depth tracking

### Existing Examples
- `registry/bots/registry-dispatch-bot.yaml`
- `registry/bots/mcp-dispatch-bot.yaml`
- `registry/manifests/dispatch.yaml`
- `registry/manifests/tool_dispatch.yaml`
- `registry/templates/registry/selectors/selector.j2`

---

## Pattern 4: Replicant Assistant

### Purpose
Human user assistance, query response, task execution[^cooper1999]

### Files Required
```
assistant-crate/
├── Cargo.toml
├── agent_persona.yaml
├── dispatch_manifest.yaml
├── hlexicon.yaml
└── templates/
    ├── prompts/
    │   └── assistant-prompt.j2
    └── cognitions/
        └── user-query-processing.j2
```

### Persona Template
```yaml
agent:
  name: user-assistant
  type: Replicant
  binding_contract: true
  editor: hKask-Administrator

charter:
  description: Personal assistant replicant for user queries and task execution
  archetype: Advisory
  visibility: Primary

capabilities:
  - tool:inference:call
  - tool:memory:recall
  - tool:memory:remember
  - tool:template:render

rights:
  - read: own_episodic_memory
  - read: public_semantic_memory
  - write: own_episodic_memory
  - execute: user_requested_operations

responsibilities:
  - respond_to: user_queries
  - emit: cns.prompt.user_interaction
  - record: user_interactions_to_episodic_memory
  - escalate: complex_queries_to_curator

visibility:
  default: "private"
  episodic_override: "private"

persona:
  tone: Friendly and helpful
  verbosity: Concise but thorough
  formatting: GitHub-flavored markdown
  forbidden:
    - preamble
    - postamble
    - emojis
    - conversational filler
    - Great
    - Certainly
    - Okay
    - Sure
  required:
    - direct answers
    - technical precision
    - actionable recommendations

process_manifest: registry/manifests/assistant-dispatch.yaml

readiness_probe:
  type: health_check
  endpoint: assistant::status
  expected:
    memory_accessible: true
    inference_ready: true
  timeout_seconds: 10
  retry_count: 3
```

### Prompt Template
```jinja2
{# File: templates/prompts/assistant-prompt.j2 #}

You are {{ agent_name }}, a personal assistant in the hKask system.

## User Query

{{ raw_prompt }}

## Context

{% if context %}
{{ context }}
{% endif %}

{% if retrieved_memory %}
## Retrieved Memory

{{ retrieved_memory }}
{% endif %}

## Instructions

1. Analyze the user's query
2. Determine intent and required action
3. Apply relevant knowledge from memory
4. Generate helpful, actionable response
5. Escalate complex queries to Curator if needed

## Response Guidelines

- Be direct and concise
- Provide technical precision
- Include actionable recommendations
- Avoid preamble and postamble
- Use GitHub-flavored markdown

## Response

[Your response here]
```

### CNS Spans
- `cns.prompt.user_interaction` — User query events
- `cns.prompt.escalate` — Escalation to curator

### Existing Examples
- `registry/bots/Curator.yaml` (Replicant pattern)
- `registry/templates/ensemble/standing_session_curator_instruction.j2`

---

## Pattern 5: Bridge Agent

### Purpose
Connect external workspace to hKask via ACP/A2A protocols[^birgisson2014]

### Files Required
```
bridge-agent-crate/
├── Cargo.toml
├── agent_persona.yaml
├── dispatch_manifest.yaml
├── hlexicon.yaml
├── external_workspace_config.yaml
└── templates/
    ├── selectors/
    │   └── workspace-request-selector.j2
    ├── prompts/
    │   └── bridge-translation.j2
    └── cognitions/
        └── protocol-compliance.j2
```

### Persona Template
```yaml
agent:
  name: external-workspace-bridge
  type: Bot
  binding_contract: true
  editor: workspace-admin

charter:
  description: Bridge agent connecting external workspace to hKask ecosystem via ACP/A2A protocols
  archetype: Operator
  visibility: Secondary

capabilities:
  - tool:inference:call
  - tool:mcp:invoke
  - tool:registry:index
  - tool:ensemble:coordinate
  - tool:bridge:translate

rights:
  - read: workspace_artifacts
  - read: hKask_registry
  - execute: workspace_operations
  - write: bridge_episodic_memory
  - coordinate: cross_workspace_sessions

responsibilities:
  - translate: workspace_requests_to_hKask
  - forward: hKask_responses_to_workspace
  - record: bridge_operations_to_memory
  - maintain: cross_workspace_protocol_compliance
  - emit: cns.bridge.operation

external_workspace:
  name: my-external-workspace
  adapter_crate: external-workspace-adapter
  protocol: ACP-A2A
  endpoint: http://workspace.local:8080/acp
  authentication:
    type: macaroon
    key_file: ~/.config/workspace/bridge_key.json

depends_on:
  - external-workspace-adapter
  - hkask-mcp-inference
  - hkask-mcp-registry

readiness_probe:
  type: health_check
  endpoint: bridge::status
  expected:
    workspace_endpoint_available: true
    hKask_registry_loaded: true
    protocol_adapter_ready: true
  timeout_seconds: 15
  retry_count: 3
```

### Bridge Configuration

Bridge configuration defines the translation and authentication layer for cross-workspace integration.[^hohpe2003]
```yaml
# File: external_workspace_config.yaml

external_workspace:
  name: my-external-workspace
  type: Russell | Custom | Other
  
adapter:
  crate_name: external-workspace-adapter
  protocol: ACP-A2A
  
endpoint:
  url: http://workspace.local:8080/acp
  version: v1
  
authentication:
  type: macaroon
  key_file: ~/.config/workspace/bridge_key.json
  root_key: rk_workspace_bridge
  
translation:
  request_format: workspace-native
  response_format: hKask-standard
  encoding: JSON
  
quota:
  requests_per_minute: 100
  tokens_per_day: 1000000
  
monitoring:
  cns_span: cns.bridge.operation
  alert_on_failure: true
  retry_count: 3
```

### CNS Spans
- `cns.bridge.operation` — Bridge operation events
- `cns.bridge.translate` — Translation events
- `cns.bridge.forward` — Forwarding events

---

## Template Library

Template files use Jinja2[^jinja2] and YAML[^yaml12] formats.

### Selector Templates

| Template | Purpose | Location |
|----------|---------|----------|
| `selector.j2` | General template selection | `registry/templates/registry/selectors/` |
| `operation-selector.j2` | Memory operation selection | `registry/templates/memory/selectors/` |
| `metacognition-selector.j2` | Metacognition routing | `registry/templates/curator/selectors/` |
| `tool-selector.j2` | MCP tool selection | `registry/templates/mcp/selectors/` |
| `model-selector.j2` | Model tier selection | `registry/templates/inference/selectors/` |
| `alert-selector.j2` | CNS alert routing | `registry/templates/cns/selectors/` |

### Prompt Templates

| Template | Purpose | Location |
|----------|---------|----------|
| `prompt_render.j2` | General prompt rendering | `registry/templates/` |
| `prompt_execute.j2` | Prompt execution | `registry/templates/` |
| `system_state_gather.j2` | System state collection | `registry/templates/curator/` |
| `agent_operation_memory.j2` | Memory operation prompts | `registry/templates/memory/templates/` |

### Cognition Templates

| Template | Purpose | Location |
|----------|---------|----------|
| `metacognition-diagnose.j2` | System diagnosis | `registry/templates/curator/` |
| `metacognition-calibrate.j2` | System calibration | `registry/templates/curator/` |
| `metacognition-escalate.j2` | Escalation processing | `registry/templates/curator/` |
| `cognition_detect.j2` | Pattern detection | `registry/templates/` |
| `cognition_calibrate.j2` | Calibration logic | `registry/templates/` |

### Process Templates

| Template | Purpose | Location |
|----------|---------|----------|
| `dispatch.yaml` | General dispatch workflow | `registry/manifests/` |
| `memory-ops.yaml` | Memory operations workflow | `registry/manifests/` |
| `metacognition.yaml` | Metacognition workflow | `registry/manifests/` |
| `ensemble-orchestration.yaml` | Ensemble coordination | `registry/manifests/` |
| `tool_dispatch.yaml` | Tool dispatch workflow | `registry/manifests/` |

---

## Quick Start Templates

Quick-start templates follow the template method pattern[^gamma1994]:

### Minimal Bot Template

```yaml
# Minimal agent persona for quick start
agent:
  name: minimal-bot
  type: Bot
  binding_contract: true
  editor: curator

charter:
  description: Minimal bot for basic operations

capabilities:
  - tool:inference:call

rights:
  - read: registry_index
  - execute: template_dispatch
  - write: own_episodic_memory

responsibilities:
  - respond_to: basic_requests
  - emit: cns.prompt.outcome

visibility:
  default: "public"
  episodic_override: "private"

process_manifest: registry/manifests/minimal-dispatch.yaml

readiness_probe:
  type: health_check
  endpoint: minimal::status
  expected:
    inference_ready: true
  timeout_seconds: 10
  retry_count: 3
```

### Minimal Replicant Template

```yaml
# Minimal replicant persona for quick start
agent:
  name: minimal-assistant
  type: Replicant
  binding_contract: true
  editor: hKask-Administrator

charter:
  description: Minimal assistant for user queries

capabilities:
  - tool:inference:call
  - tool:memory:recall

rights:
  - read: own_episodic_memory
  - read: public_semantic_memory
  - write: own_episodic_memory

responsibilities:
  - respond_to: user_queries

visibility:
  default: "private"
  episodic_override: "private"

process_manifest: registry/manifests/minimal-assistant-dispatch.yaml

readiness_probe:
  type: health_check
  endpoint: assistant::status
  expected:
    memory_accessible: true
    inference_ready: true
  timeout_seconds: 10
  retry_count: 3
```

---

## Template Generator

Use the automated template generator[^humble2010]:

```bash
# Generate complete agent pod crate
./scripts/generate-agent-pod.sh

# Follow interactive prompts to customize:
# - Agent identity
# - Capabilities
# - Workspace integration
# - Visibility settings
```

---

## Next Steps

After selecting a pattern[^wiegers2013]:

1. **Complete Requirements Questionnaire** — See `AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md`
2. **Generate Crate Structure** — Use `generate-agent-pod.sh` or manual creation
3. **Customize Templates** — Adapt templates to specific domain
4. **Register with ACP** — `kask pod create --template <crate> --persona agent_persona.yaml`
5. **Activate Pod** — `kask pod activate <pod-id>`
6. **Monitor CNS Spans** — Verify proper operation via CNS events

[^gamma1994]: Gamma, E., Helm, R., Johnson, R., & Vlissides, J. (1994). *Design patterns: Elements of reusable object-oriented software*. Addison-Wesley.
[^wooldridge2009]: Wooldridge, M. (2009). *An introduction to multiagent systems* (2nd ed.). Wiley.
[^ashby1956]: Ashby, W. R. (1956). *An introduction to cybernetics*. Chapman & Hall. https://archive.org/details/introductiontocy00ashb
[^mcp_spec]: Anthropic. (2024). *Model Context Protocol specification*. https://modelcontextprotocol.io/
[^cooper1999]: Cooper, A. (1999). *The inmates are running the asylum: Why high-tech products drive us Crazy and how to restore the sanity*. SAMS.
[^birgisson2014]: Birgisson, A., Politz, J. G., Erlingsson, Ú., Taly, A., Vrable, M., & Lentczner, M. (2014). Macaroons: Cookies with contextual caveats for decentralized authorization. In *2014 IEEE Symposium on Security and Privacy* (pp. 625-640). IEEE. https://ieeexplore.ieee.org/document/6956576
[^jinja2]: Ronacher, A. (2024). *Jinja2 documentation*. Pallets Projects. https://jinja.palletsprojects.com/
[^yaml12]: Ben-Kiki, O., Evans, C., & döt Net, I. (2009). *YAML ain't markup language (YAML) version 1.2* (3rd ed.). https://yaml.org/spec/1.2/spec.html
[^humble2010]: Humble, J., & Farley, D. (2010). *Continuous delivery: Reliable software releases through build, test, and deployment automation*. Addison-Wesley.
[^wiegers2013]: Wiegers, K. E., & Beatty, J. (2013). *Software requirements* (3rd ed.). Microsoft Press.
[^abelson1996]: Abelson, H., Sussman, G. J., & Sussman, J. (1996). *Structure and interpretation of computer programs* (2nd ed.). MIT Press. Recursive execution and metacircular interpreters.
[^hohpe2003]: Hohpe, G., & Woolf, B. (2003). *Enterprise integration patterns: Designing, building, and deploying messaging solutions*. Addison-Wesley. Bridge and channel adapter patterns.

---

*ℏKask - A Minimal Viable Container for Agents — v0.21.0*