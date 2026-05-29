---
title: "Agent Pod Creation Guide"
audience: [developers, agent designers]
last_updated: 2026-05-24
version: "0.21.0"
status: "Active"
domain: "Application"
ddmvss_categories: [domain]
---

# Agent Pod Creation Guide — hKask

---

## Overview

This guide walks you through creating an **Agent Pod** — a runtime container for ACP agents (bots or replicants) in the hKask ecosystem[^hewitt1973]. Agent pods provide:

- **Isolation**: Independent capability tokens, no shared state
- **Identity**: WebID-based ACP registration
- **Access**: Capability-gated MCP tool invocation
- **Observability**: CNS span emission for all lifecycle events
- **Persistence**: Memory artifact generation (episodic/semantic triples)

An agent pod can be:
1. **A standalone crate** in a new workspace
2. **An additional crate** in an existing workspace
3. **A bridge crate** connecting an external workspace to hKask via ACP/A2A protocols

---

## Table of Contents

The creation process follows a requirements-driven methodology[^wiegers2013]:

1. [Prerequisites](#prerequisites)
2. [Agent Pod Architecture](#agent-pod-architecture)
3. [Step 1: Requirements Discovery](#step-1-requirements-discovery)
4. [Step 2: Create Agent Persona](#step-2-create-agent-persona)
5. [Step 3: Create Dispatch Manifest](#step-3-create-dispatch-manifest)
6. [Step 4: Create Templates](#step-4-create-templates)
7. [Step 5: Build Agent Crate](#step-5-build-agent-crate)
8. [Step 6: Register with ACP Runtime](#step-6-register-with-acp-runtime)
9. [Step 7: Activate Pod](#step-7-activate-pod)
10. [Step 8: Configure Visibility](#step-8-configure-visibility)
11. [Common Agent Patterns](#common-agent-patterns)
12. [Troubleshooting](#troubleshooting)

---

## Prerequisites

Before creating an agent pod, ensure you have[^bass2021]:

- **hKask CLI installed**: `cargo install --path crates/hkask-cli`
- **Git CAS configured**: Template path set via `HKASK_TEMPLATES_PATH` or default `./registry/templates/`
- **ACP runtime available**: Either local or remote ACP server
- **MCP servers registered**: Tools your agent will need access to
- **Root authority access**: For issuing capability tokens (administrator only)

---

## Agent Pod Architecture

The architecture follows a hexagonal (ports and adapters) design[^cockburn2005].

### Lifecycle States

```
Populated → Registered → Activated → Deactivated
    ↓           ↓            ↓
  new()    register()   activate()  deactivate()
```

| State | Description | Capabilities |
|-------|-------------|--------------|
| **Populated** | Pod instantiated from template crate | None |
| **Registered** | Registered with ACP runtime | Capability token minted |
| **Activated** | Activated for A2A communication | MCP access granted |
| **Deactivated** | Deactivated, capabilities revoked | None (revoked) |

### Crate Structure

```
my-agent-crate/
├── agent_persona.yaml       # Agent identity and capabilities
├── dispatch_manifest.yaml   # Process workflow definition
├── hlexicon.yaml           # Domain-specific terms (optional)
└── templates/              # Jinja2 templates
    ├── selectors/          # Selection templates
    │   └── selector.j2
    ├── prompts/            # WordAct templates
    │   └── prompt_*.j2
    ├── processes/          # FlowDef templates
    │   └── process_*.yaml
    └── cognitions/         # KnowAct templates
        └── cognition_*.j2
```

---

## Step 1: Requirements Discovery

Answer these questions before creating your agent pod[^wiegers2013]:

### 1.1 Agent Purpose

**Q: What is the agent's primary function?**

| Type | Purpose | Example |
|------|---------|---------|
| **Bot** | Process execution, machine-to-machine (A2A) | `registry-dispatch-bot`, `memory-curator-bot` |
| **Replicant** | Human assistance, human-to-agent (H2A) | `Curator`, `Assistant` |

**Q: What domain does the agent operate in?**

- `WordAct` — Speech acts, LLM calls, template rendering
- `FlowDef` — Multi-step workflows, operations, orchestration
- `KnowAct` — Thinking, learning, calibration, metacognition

### 1.2 Capabilities Required

**Q: What tools does the agent need access to?**

Common capability patterns:

```yaml
# Inference capabilities
capabilities:
  - tool:inference:call
  - tool:inference:stream
  - tool:inference:embed

# Memory capabilities
capabilities:
  - tool:memory:recall
  - tool:memory:remember
  - tool:memory:query
  - tool:memory:embed

# CNS capabilities
capabilities:
  - tool:cns:emit
  - tool:cns:variety
  - tool:cns:calibrate

# Registry capabilities
capabilities:
  - tool:registry:index
  - tool:registry:discover
  - tool:registry:validate

# Template capabilities
capabilities:
  - tool:template:render
  - tool:template:select
  - tool:template:execute

# Ensemble capabilities
capabilities:
  - tool:ensemble:coordinate
  - tool:ensemble:orchestrate
```

### 1.3 Rights and Access

**Q: What data/resources does the agent need to read?**

```yaml
rights:
  - read: registry_index
  - read: template_catalog
  - read: all_public_semantic_memory
  - read: cns_spans_all
  - read: variety_counters_all
  - read: bot_reports_all
```

**Q: What operations does the agent need to execute?**

```yaml
rights:
  - execute: template_dispatch
  - execute: metacognition_ops
  - execute: system_calibration
  - coordinate: bot_ensemble_sessions
```

**Q: What data does the agent need to write?**

```yaml
rights:
  - write: own_episodic_memory
  - write: public_semantic_memory
  - write: bot_status_reports
```

### 1.4 Workspace Integration

**Q: Is this agent part of a larger workspace?**

- **Yes, new workspace**: Create new crate as first member
- **Yes, existing workspace**: Add crate to existing workspace's `Cargo.toml`
- **No, standalone**: Create independent crate

**Q: What external dependencies does the agent have?**

```yaml
depends_on:
  - hkask-mcp-registry
  - hkask-mcp-inference
   - hkask-mcp-registry
  - cns-curator-bot
  - memory-curator-bot
```

### 1.5 Visibility and Sharing

**Q: What is the default visibility for agent artifacts?**

| Visibility | Description | Use Case |
|------------|-------------|----------|
| `public` | Visible to all agents | System bots, shared knowledge |
| `private` | Visible only to agent itself | Personal episodic memory |
| `shared` | Visible to agents with capability | Team collaboration |

```yaml
visibility:
  default: "public"
  episodic_override: "private"
```

### 1.6 Reporting and Escalation

**Q: Who does the agent report to?**

```yaml
reporting:
  receives_from:
    - cns-curator-bot
    - memory-curator-bot
  report_to: standing_ensemble_session
  escalate_to: Curator
  escalation_triggers:
    - variety_deficit_gt_100
    - bot_coordination_failure
```

---

## Step 2: Create Agent Persona

The agent persona defines identity, capabilities, and behavior[^yaml12][^cooper1999].

### 2.1 Bot Persona Template

```yaml
# File: agent_persona.yaml

agent:
  name: my-specialist-bot
  type: Bot                          # Bot | Replicant
  version: "0.1.0"                   # Optional, defaults to "0.1.0"
  binding_contract: true             # Required for ACP compliance
  editor: curator-or-human-admin     # Who can modify this persona

charter:
  description: >                     # Clear purpose statement
    Specialist bot responsible for [domain] operations,
    including [key functions] and [reporting requirements].
  archetype: Specialist              # MaintenanceAdvisory | Specialist | Operator
  visibility: Secondary              # Primary | Secondary

capabilities:
  - tool:inference:call
  - tool:template:render
  - tool:memory:recall
  # Add capabilities from Section 1.2

rights:
  - read: registry_index
  - read: template_catalog
  - execute: template_dispatch
  - write: own_episodic_memory
  # Add rights from Section 1.3

responsibilities:
  - respond_to: [trigger_type]
  - emit: cns.[domain].[action]
  - report_to: [supervisor_bot]
  - record: [memory_type]_to_episodic_memory
  - produce: semantic_triples_from_[operations]

visibility:
  default: "public"
  episodic_override: "private"

# Optional: Reporting configuration
reporting:
  receives_from: []
  report_to: standing_ensemble_session
  escalate_to: Curator
  escalation_triggers: []
  report_interval: on_event_and_hourly_summary

# Optional: Standing session participation
standing_session:
  session_id: system-coordination-standing-session
  role: participant                # orchestrator | participant | observer
  report_interval: hourly
  administrator_visible: true

# Required: Process manifest reference
process_manifest: registry/manifests/my-bot-dispatch.yaml

# Optional: Dependencies
depends_on:
  - hkask-mcp-inference
   - hkask-mcp-registry

# Required: Readiness probe
readiness_probe:
  type: health_check
  endpoint: my_bot::status
  expected:
    registry_index_available: true
    template_selector_ready: true
  timeout_seconds: 10
  retry_count: 3
```

### 2.2 Replicant Persona Template

```yaml
# File: agent_persona.yaml

agent:
  name: MyAssistant
  type: Replicant
  version: "0.1.0"
  binding_contract: true
  editor: hKask-Administrator

charter:
  description: >
    Personal assistant replicant responsible for helping the user
    with [tasks], providing [services], and maintaining [capabilities].
  archetype: Advisory
  visibility: Primary

capabilities:
  - tool:inference:call
  - tool:memory:recall
  - tool:memory:remember
  - tool:template:render
  # Replicants typically have fewer capabilities than bots

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
  default: "private"                 # Replicants default to private
  episodic_override: "private"

# Replicant-specific: Persona for user interaction
persona:
  tone: Friendly and helpful
  verbosity: Concise but thorough
  formatting: GitHub-flavored markdown
  forbidden:
    - preamble
    - postamble
    - emojis
    - conversational filler
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

---

## Step 3: Create Dispatch Manifest

The dispatch manifest defines the agent's workflow as a sequence of steps[^yaml12][^mcp_spec].

### 3.1 Dispatch Manifest Structure

```yaml
# File: dispatch_manifest.yaml

manifest:
  name: my-bot-dispatch
  version: "0.1.0"
  description: Dispatch workflow for [bot name]

# Matroshka (recursion) configuration
matroshka:
  max_depth: 7                       # Hard limit per architecture
  enforce: true
  depth_counter:
    enabled: true
    inherit_from_parent: true
    default: 0
    increment_on_dispatch: true
  cns_monitoring:
    span_namespace: cns.prompt.matroshka_depth
    alert_if_exceeds: 6              # Warning before hard limit
    rationale: "Prevent infinite recursion"

# Workflow steps (executed in order)
steps:
  - ordinal: 1
    action: select                   # Select template
    template_ref: registry/selectors/selector.j2
    renderer: minijinja
    model_tier: fast_local           # fast_local | balanced | high_quality
    matroshka_depth: "${matroshka_depth}"
    output_schema:
      type: object
      properties:
        selected_template_id:
          type: string
        rationale:
          type: string
        confidence:
          type: number
          minimum: 0.0
          maximum: 1.0

  - ordinal: 2
    action: populate                 # Bind input to template
    template_ref: "${selected_template_id}"
    renderer: minijinja
    bindings:
      raw_prompt: "${input.raw_prompt}"
      context: "${input.context}"
      matroshka_depth: "${matroshka_depth + 1}"

  - ordinal: 3
    action: execute                  # Execute rendered template
    target: "${template.contract.target}"
    contract: "${template.contract}"
    mcp: "${template.contract.mcp}"
    model_tier: "${template.contract.model_tier}"
    matroshka_depth: "${matroshka_depth + 1}"

# CNS span emission
cns:
  spans:
    - cns.prompt.select
    - cns.prompt.render
    - cns.prompt.outcome
```

### 3.2 Step Actions

| Action | Purpose | Required Fields |
|--------|---------|-----------------|
| `select` | Template selection | `template_ref`, `renderer`, `model_tier`, `output_schema` |
| `populate` | Field binding | `template_ref`, `renderer`, `bindings` |
| `execute` | Execution | `target`, `contract`, `mcp`, `model_tier` |

### 3.3 Common Dispatch Patterns

#### Pattern 1: Simple Inference Call

```yaml
steps:
  - ordinal: 1
    action: populate
    template_ref: registry/templates/prompt_render.j2
    renderer: minijinja
    bindings:
      raw_prompt: "${input.raw_prompt}"
      context: "${input.context}"

  - ordinal: 2
    action: execute
    target: inference
    contract:
      model_tier: balanced
      mcp: hkask-mcp-inference
```

#### Pattern 2: Memory Recall → Inference

```yaml
steps:
  - ordinal: 1
    action: populate
    template_ref: registry/templates/memory/templates/recall.j2
    renderer: minijinja
    bindings:
      query: "${input.query}"
      memory_type: semantic
      visibility_filter: public

  - ordinal: 2
    action: execute
    target: memory
    contract:
       mcp: hkask-mcp-registry

  - ordinal: 3
    action: populate
    template_ref: registry/templates/prompt_execute.j2
    renderer: minijinja
    bindings:
      raw_prompt: "${input.raw_prompt}"
      retrieved_memory: "${step_1_output.results}"

  - ordinal: 4
    action: execute
    target: inference
    contract:
      model_tier: balanced
      mcp: hkask-mcp-inference
```

#### Pattern 3: CNS Monitoring → Calibration

```yaml
steps:
  - ordinal: 1
    action: populate
    template_ref: registry/templates/cns/selectors/alert-selector.j2
    renderer: minijinja
    bindings:
      variety_deficit: "${cns.variety_deficit}"
      warning_count: "${cns.warning_count}"

  - ordinal: 2
    action: execute
    target: cns
    contract:
      mcp: hkask-mcp-cns

  - ordinal: 3
    action: populate
    template_ref: registry/templates/cognition_calibrate.j2
    renderer: minijinja
    bindings:
      alerts: "${step_2_output.alerts}"
      system_state: "${cns.system_state}"

  - ordinal: 4
    action: execute
    target: inference
    contract:
      model_tier: high_quality
      mcp: hkask-mcp-inference
```

---

## Step 4: Create Templates

Templates are Jinja2 files that define the agent's behavior[^jinja2].

### 4.1 Template Types

| Type | Extension | Purpose | Example |
|------|-----------|---------|---------|
| **Prompt** (WordAct) | `.j2` | LLM calls, speech acts | `prompt_render.j2` |
| **Process** (FlowDef) | `.yaml` | Multi-step workflows | `process_dispatch.yaml` |
| **Cognition** (KnowAct) | `.j2` | Thinking, learning | `cognition_calibrate.j2` |

### 4.2 Selector Template

```jinja2
{# File: templates/selectors/selector.j2 #}

You are a template selector for [domain].

## Available Templates

{% for template in templates %}
### {{ template.id }}
- Type: {{ template.template_type }}
- Lexicon: {{ template.lexicon_terms | join(", ") }}
- Description: {{ template.description }}
{% endfor %}

## User Request

{{ raw_prompt }}

{% if domain_hint %}
Domain hint: {{ domain_hint }}
{% endif %}

## Selection Criteria

1. Match template_type to request nature
2. Match lexicon terms to prompt vocabulary
3. Consider contract compatibility

## Response

Return JSON:
{
  "selected_template_id": "...",
  "rationale": "...",
  "confidence": 0.0
}
```

### 4.3 Prompt Template

```jinja2
{# File: templates/prompts/my-prompt.j2 #}

You are {{ agent_name }}, a [role] in the hKask system.

## Context

{% if context %}
{{ context }}
{% endif %}

## Task

{{ raw_prompt }}

## Instructions

1. Analyze the request
2. Apply domain knowledge
3. Generate appropriate response

## Output Format

[Specify expected output format]

## Response

[Your response here]
```

### 4.4 Cognition Template

```jinja2
{# File: templates/cognitions/my-cognition.j2 #}

You are performing metacognition on [domain].

## Input Data

{% for item in input_data %}
- {{ item.type }}: {{ item.value }}
{% endfor %}

## Analysis Framework

1. **Observe** — Gather relevant data
2. **Orient** — Contextualize within system state
3. **Decide** — Determine appropriate action
4. **Act** — Execute decision

## Output

Return JSON with:
- observation: What you observed
- orientation: How it fits context
- decision: What action to take
- action: Execution details
```

---

## Step 5: Build Agent Crate

Crate structure follows Cargo workspace conventions[^cargo_book].

### 5.1 Crate Directory Structure

```
my-agent-crate/
├── Cargo.toml              # Rust crate metadata
├── agent_persona.yaml      # Agent persona
├── dispatch_manifest.yaml  # Dispatch workflow
├── hlexicon.yaml          # Domain terms (optional)
└── templates/
    ├── selectors/
    │   └── selector.j2
    ├── prompts/
    │   └── my-prompt.j2
    ├── processes/
    │   └── my-process.yaml
    └── cognitions/
        └── my-cognition.j2
```

### 5.2 Cargo.toml Template

```toml
[package]
name = "my-agent-crate"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Agent pod for [purpose]"

[dependencies]
hkask-types = { path = "../hkask-types" }
hkask-agents = { path = "../hkask-agents" }
hkask-templates = { path = "../hkask-templates" }
hkask-mcp = { path = "../hkask-mcp" }
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
hkask-testing = { path = "../hkask-testing" }
```

### 5.3 Workspace Integration

#### Option A: New Workspace

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "my-agent-crate",
]
resolver = "2"

[workspace.dependencies]
serde = "1.0"
serde_json = "1.0"
serde_yaml = "0.9"
tokio = "1.0"
tracing = "0.1"
thiserror = "1.0"
```

#### Option B: Existing Workspace

```toml
# Existing workspace Cargo.toml
[workspace]
members = [
    # ... existing members ...
    "my-agent-crate",  # Add new crate
]
```

### 5.4 hLexicon Terms (Optional)

```yaml
# File: hlexicon.yaml

# Domain-specific terms for template matching
- recognize
- classify
- match
- discriminate
- [your-domain-terms]
```

---

## Step 6: Register with ACP Runtime

Registration issues capability tokens following OCAP principles[^miller2006].

### 6.1 CLI Registration

```bash
# Navigate to crate directory
cd my-agent-crate

# Register agent with ACP runtime
kask pod create \
  --template my-agent-crate \
  --persona agent_persona.yaml \
  --name my-specialist-bot
```

### 6.2 API Registration

```bash
# POST /api/pods
curl -X POST http://localhost:8080/api/pods \
  -H "Content-Type: application/json" \
  -d '{
    "template": "my-agent-crate",
    "persona_yaml": "<contents of agent_persona.yaml>",
    "name": "my-specialist-bot"
  }'
```

### 6.3 Programmatic Registration (Rust)

```rust
use hkask_agents::pod::{PodManager, AgentPersona, PodID};
use hkask_agents::adapters::git_cas::GitCasAdapter;
use hkask_agents::adapters::acp_runtime::AcpRuntimeAdapter;
use hkask_agents::adapters::cns_emitter::CnsEmitterAdapter;
use hkask_agents::adapters::mcp_runtime::McpRuntimeAdapter;
use hkask_types::WebID;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create adapters
    let git_cas = GitCasAdapter::from_path(PathBuf::from("./registry/templates"));
    let acp_runtime = AcpRuntimeAdapter::new();
    let cns_emitter = CnsEmitterAdapter::new(WebID::new());
    let mcp_runtime = McpRuntimeAdapter::new();
    
    // Create pod manager
    let pod_manager = PodManager::new(git_cas, acp_runtime, cns_emitter, mcp_runtime);
    
    // Load persona from YAML
    let persona_yaml = std::fs::read_to_string("agent_persona.yaml")?;
    let persona = AgentPersona::from_yaml(&persona_yaml)?;
    
    // Create pod
    let pod_id = pod_manager
        .create_pod("my-agent-crate", &persona, Some("my-specialist-bot".to_string()))
        .await?;
    
    println!("Pod created: {}", pod_id);
    
    Ok(())
}
```

### 6.4 Capability Token

After registration, the pod receives a capability token:

```rust
pub struct CapabilityToken {
    pub id: String,              // Unique token identifier
    pub tool_name: String,       // Tool this grants access to
    pub delegated_from: WebID,   // Delegator WebID
    pub delegated_to: WebID,     // Holder WebID
    pub signature: String,       // HMAC signature
    pub attenuation_level: u8,   // Delegation depth (0-7)
    pub max_attenuation: u8,     // Maximum allowed depth (default: 7)
    pub root_context_nonce: String,
}
```

**Important:** The token has `attenuation_level=0` and `max_attenuation=7`. Each delegation increments the level. Delegation fails when `attenuation_level >= max_attenuation`.

---

## Step 7: Activate Pod

Activation transitions the pod to a live actor state[^hewitt1973].

### 7.1 CLI Activation

```bash
# Activate pod
kask pod activate <pod-id>
```

### 7.2 API Activation

```bash
# POST /api/pods/:id/activate
curl -X POST http://localhost:8080/api/pods/<pod-id>/activate
```

### 7.3 Programmatic Activation (Rust)

```rust
// Activate pod
pod_manager.activate_pod(&pod_id).await?;
println!("Pod activated: {}", pod_id);
```

### 7.4 What Activation Does

1. **Registers with ACP runtime** (if not already registered)
2. **Grants MCP tool access** per capabilities
3. **Enables A2A communication** with other bots
4. **Emits CNS span**: `cns.agent_pod.activated`

---

## Step 8: Configure Visibility

### 8.1 Visibility Settings

Visibility is configured in the agent persona[^miller2006]:

```yaml
visibility:
  default: "public"              # public | private | shared
  episodic_override: "private"   # Override for episodic memory
```

### 8.2 Visibility Rules

| Visibility | Read Access | Write Access | Use Case |
|------------|-------------|--------------|----------|
| `public` | All agents | Capability-gated | Shared knowledge, system bots |
| `private` | Owner only | Owner only | Personal episodic memory |
| `shared` | Capability holders | Capability holders | Team collaboration |

### 8.3 OCAP Enforcement

Visibility is enforced via OCAP capability tokens:

```rust
// Check capability before accessing resource
let has_access = capability_checker.check_resource(
    &capability_token,
    &requester_webid,
    CapabilityResource::Memory,  // Tool | Template | Cascade
);

if !has_access {
    return Err(VisibilityError::AccessDenied);
}
```

---

## Common Agent Patterns

Agent patterns follow established design pattern conventions[^gamma1994]:

### Pattern 1: Specialist Bot

**Purpose:** Domain-specific expert bot (e.g., memory operations, CNS monitoring)

```yaml
agent:
  name: memory-specialist-bot
  type: Bot
  binding_contract: true
  editor: curator

charter:
  description: Specialist bot for memory operations including recall, storage, and embedding

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

process_manifest: registry/manifests/memory-ops.yaml
```

### Pattern 2: Curator Bot

**Purpose:** System oversight, metacognition, coordination

```yaml
agent:
  name: domain-curator-bot
  type: Bot
  binding_contract: true
  editor: hKask-Administrator

charter:
  description: Curator bot responsible for [domain] oversight and coordination

capabilities:
  - tool:cns:emit
  - tool:cns:variety
  - tool:memory:recall
  - tool:inference:call
  - tool:ensemble:coordinate

rights:
  - read: all_public_semantic_memory
  - read: cns_spans_all
  - read: bot_reports_all
  - write: own_episodic_memory
  - write: public_semantic_memory
  - execute: metacognition_ops
  - coordinate: bot_ensemble_sessions

responsibilities:
  - monitor: domain_health
  - synthesize: bot_reports
  - escalate: critical_alerts_to_curator
  - orchestrate: ensemble_sessions

reporting:
  receives_from:
    - specialist-bot-1
    - specialist-bot-2
  report_to: standing_ensemble_session
  escalate_to: Curator

process_manifest: registry/manifests/domain-curation.yaml
```

### Pattern 3: Replicant Assistant

**Purpose:** Human user assistance

```yaml
agent:
  name: user-assistant
  type: Replicant
  binding_contract: true
  editor: hKask-Administrator

charter:
  description: Personal assistant for user queries and task execution

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
  forbidden:
    - preamble
    - postamble
    - emojis

process_manifest: registry/manifests/assistant-dispatch.yaml
```

### Pattern 4: Bridge Agent (External Workspace)

**Purpose:** Connect external workspace to hKask via ACP

```yaml
agent:
  name: external-workspace-bridge
  type: Bot
  binding_contract: true
  editor: workspace-admin

charter:
  description: Bridge agent connecting [external workspace] to hKask ecosystem

capabilities:
  - tool:inference:call
  - tool:mcp:invoke
  - tool:registry:index
  - tool:ensemble:coordinate

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

depends_on:
  - external-workspace-adapter
  - hkask-mcp-inference
  - hkask-mcp-registry

process_manifest: registry/manifests/bridge-dispatch.yaml

# Bridge-specific: External workspace configuration
external_workspace:
  name: my-external-workspace
  adapter_crate: external-workspace-adapter
  protocol: ACP-A2A
  endpoint: http://workspace.local:8080/acp
  authentication:
    type: macaroon
    key_file: ~/.config/workspace/bridge_key.json
```

---

## Troubleshooting

Diagnostic approaches follow security testing methodology[^owasp_testing].

### Pod Creation Fails

**Error:** `Failed to load template crate`

**Solution:**
1. Verify crate exists in `HKASK_TEMPLATES_PATH` or `./registry/templates/`
2. Check `agent_persona.yaml` is valid YAML
3. Ensure all required fields are present

**Error:** `Invalid persona YAML`

**Solution:**
1. Validate YAML syntax: `yamllint agent_persona.yaml`
2. Check required fields: `agent`, `charter`, `capabilities`, `responsibilities`
3. Verify `binding_contract: true` is present

### Pod Registration Fails

**Error:** `ACP registration failed`

**Solution:**
1. Verify ACP runtime is running
2. Check root authority has issued capability tokens
3. Ensure capabilities match registered tools

**Error:** `Capability verification failed`

**Solution:**
1. Verify capability token signature
2. Check attenuation level hasn't exceeded max
3. Ensure token hasn't expired

### Pod Activation Fails

**Error:** `MCP access grant failed`

**Solution:**
1. Verify MCP servers are registered
2. Check tool names match registered tools
3. Ensure capability token grants access to specified tools

**Error:** `Pod not found`

**Solution:**
1. Verify pod ID is correct (UUID format)
2. Check pod was created successfully
3. Ensure pod hasn't been deactivated

### CNS Span Emission Fails

**Error:** `CNS event emission failed`

**Solution:**
1. Verify CNS emitter is initialized
2. Check span namespace is valid
3. Ensure observation JSON is well-formed

### Visibility/Access Errors

**Error:** `Access denied: insufficient capability`

**Solution:**
1. Verify capability token grants required access
2. Check visibility settings match access pattern
3. Ensure OCAP verification passes

---

## Quick Reference

Reference cards follow architectural documentation conventions[^bass2021].

### Lifecycle Commands

```bash
# Create pod
kask pod create --template <crate> --persona <yaml> --name <name>

# List pods
kask pod list

# Get status
kask pod status <pod-id>

# Activate pod
kask pod activate <pod-id>

# Deactivate pod
kask pod deactivate <pod-id>
```

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/pods` | GET | List all pods |
| `/api/pods` | POST | Create new pod |
| `/api/pods/:id` | GET | Get pod status |
| `/api/pods/:id/activate` | POST | Activate pod |
| `/api/pods/:id/deactivate` | POST | Deactivate pod |

### File Checklist

- [ ] `agent_persona.yaml` — Agent identity and capabilities
- [ ] `dispatch_manifest.yaml` — Workflow definition
- [ ] `hlexicon.yaml` — Domain terms (optional)
- [ ] `templates/selectors/selector.j2` — Template selector
- [ ] `templates/prompts/*.j2` — Prompt templates
- [ ] `templates/processes/*.yaml` — Process templates
- [ ] `templates/cognitions/*.j2` — Cognition templates
- [ ] `Cargo.toml` — Rust crate metadata

### Common Capability Patterns

```yaml
# Inference
- tool:inference:call
- tool:inference:stream

# Memory
- tool:memory:recall
- tool:memory:remember
- tool:memory:embed

# CNS
- tool:cns:emit
- tool:cns:variety

# Registry
- tool:registry:index
- tool:registry:discover

# Ensemble
- tool:ensemble:coordinate
- tool:ensemble:orchestrate
```

---

## Next Steps

After creating your agent pod[^humble2010]:

1. **Test readiness probe**: Verify all dependencies are available
2. **Join ensemble session**: Participate in standing coordination sessions
3. **Monitor CNS spans**: Observe agent behavior via CNS events
4. **Produce memory artifacts**: Generate semantic/episodic triples
5. **Coordinate with other bots**: Establish A2A communication channels

For advanced topics, see:
- [Agent Pod Implementation](../architecture/domain-and-capability.md)
- [Security Architecture](../architecture/trust-security-observability.md)
- [CNS Observers](../architecture/PRINCIPLES.md)
- [Template Header Standard](../architecture/reference/template-header-standard.md)

[^hewitt1973]: Hewitt, C., Bishop, P., & Steiger, R. (1973). A universal modular ACTOR formalism for artificial intelligence. In *Proceedings of the 3rd International Joint Conference on Artificial Intelligence (IJCAI)* (pp. 235-245). https://dl.acm.org/doi/10.5555/1624775.1624804
[^wiegers2013]: Wiegers, K. E., & Beatty, J. (2013). *Software requirements* (3rd ed.). Microsoft Press.
[^bass2021]: Bass, L., Clements, P., & Kazman, R. (2021). *Software architecture in practice* (4th ed.). Addison-Wesley.
[^cockburn2005]: Cockburn, A. (2005). *Hexagonal architecture* (a.k.a. Ports and Adapters). https://alistair.cockburn.us/hexagonal-architecture/
[^yaml12]: Ben-Kiki, O., Evans, C., & döt Net, I. (2009). *YAML ain't markup language (YAML) version 1.2* (3rd ed.). https://yaml.org/spec/1.2/spec.html
[^cooper1999]: Cooper, A. (1999). *The inmates are running the asylum: Why high-tech products drive us Crazy and how to restore the sanity*. SAMS.
[^mcp_spec]: Anthropic. (2024). *Model Context Protocol specification*. https://modelcontextprotocol.io/
[^jinja2]: Ronacher, A. (2024). *Jinja2 documentation*. Pallets Projects. https://jinja.palletsprojects.com/
[^cargo_book]: The Rust Project. (2024). *The Cargo book*. https://doc.rust-lang.org/cargo/
[^miller2006]: Miller, M. S. (2006). *Robust composition: Towards a practical approach to trust in open distributed systems* [Doctoral dissertation, Johns Hopkins University]. https://www.erights.org/
[^gamma1994]: Gamma, E., Helm, R., Johnson, R., & Vlissides, J. (1994). *Design patterns: Elements of reusable object-oriented software*. Addison-Wesley.
[^owasp_testing]: OWASP Foundation. (2024). *OWASP web security testing guide, v4.2*. https://owasp.org/www-project-web-security-testing-guide/
[^humble2010]: Humble, J., & Farley, D. (2010). *Continuous delivery: Reliable software releases through build, test, and deployment automation*. Addison-Wesley.

---

*ℏKask — A Minimal Viable Container for Agents — v0.21.0*
*Rust is the loom. YAML/Jinja2 is the thread.*
