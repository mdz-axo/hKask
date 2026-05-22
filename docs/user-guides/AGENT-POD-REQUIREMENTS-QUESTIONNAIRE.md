# Agent Pod Requirements Discovery Questionnaire

**Purpose:** This questionnaire helps you define all requirements for creating an agent pod in hKask. Complete all sections before proceeding to agent pod creation.

---

## Section 1: Agent Identity

### 1.1 Agent Name
**What is the agent's name?**
- Must be unique within your workspace
- Use kebab-case: `my-specialist-bot`
- Example: `memory-curator-bot`, `registry-dispatch-bot`

**Answer:** `________________________`

### 1.2 Agent Type
**Is this agent a Bot or Replicant?**

- [ ] **Bot** — Process execution, machine-to-machine (A2A)
  - Examples: specialist bots, curator bots, dispatch bots
  - Typically public visibility
  - Coordinates with other bots

- [ ] **Replicant** — Human assistance, human-to-agent (H2A)
  - Examples: personal assistants, user-facing agents
  - Typically private visibility
  - Interacts directly with users

**Selection:** `________________________`

### 1.3 Editor/Administrator
**Who can modify this agent's persona?**
- Individual: `hKask-Administrator`
- Role: `curator`, `workspace-admin`
- Team: `team-name-admin`

**Answer:** `________________________`

---

## Section 2: Agent Purpose

### 2.1 Primary Function
**Describe the agent's primary purpose in 1-2 sentences:**

Example: "Specialist bot responsible for memory operations including recall, storage, and semantic triple production."

**Answer:**
```
_________________________________________________________________
_________________________________________________________________
```

### 2.2 Domain
**What domain does the agent operate in?**

- [ ] **WordAct** — Speech acts, LLM calls, template rendering
- [ ] **FlowDef** — Multi-step workflows, operations, orchestration
- [ ] **KnowAct** — Thinking, learning, calibration, metacognition

**Selection:** `________________________`

### 2.3 Archetype (Optional)
**What archetype best describes the agent?**

- [ ] **MaintenanceAdvisory** — System maintenance and advice
- [ ] **Specialist** — Domain-specific expert
- [ ] **Operator** — Operational execution
- [ ] **Advisory** — User guidance and assistance
- [ ] **Orchestrator** — Multi-agent coordination
- [ ] **Other:** `________________________`

---

## Section 3: Capabilities

### 3.1 Required Tools
**What tools does the agent need access to?** (Check all that apply)

#### Inference Tools
- [ ] `tool:inference:call` — Call LLM for generation
- [ ] `tool:inference:stream` — Stream LLM responses
- [ ] `tool:inference:embed` — Generate embeddings

#### Memory Tools
- [ ] `tool:memory:recall` — Retrieve memories
- [ ] `tool:memory:remember` — Store memories
- [ ] `tool:memory:query` — Query memory database
- [ ] `tool:memory:embed` — Create embeddings

#### CNS Tools
- [ ] `tool:cns:emit` — Emit CNS spans
- [ ] `tool:cns:variety` — Access variety counters
- [ ] `tool:cns:calibrate` — Calibrate system parameters

#### Registry Tools
- [ ] `tool:registry:index` — Access registry index
- [ ] `tool:registry:discover` — Discover templates
- [ ] `tool:registry:validate` — Validate templates

#### Template Tools
- [ ] `tool:template:render` — Render templates
- [ ] `tool:template:select` — Select templates
- [ ] `tool:template:execute` — Execute templates

#### Ensemble Tools
- [ ] `tool:ensemble:coordinate` — Coordinate bots
- [ ] `tool:ensemble:orchestrate` — Orchestrate sessions

#### MCP Tools
- [ ] `tool:mcp:invoke` — Invoke MCP tools
- [ ] `tool:mcp:discover` — Discover MCP servers

**List selected capabilities:**
```
_________________________________________________________________
_________________________________________________________________
_________________________________________________________________
```

### 3.2 Custom Tools (Optional)
**Does the agent need access to custom tools not listed above?**

If yes, list them:
```
_________________________________________________________________
_________________________________________________________________
```

---

## Section 4: Rights and Access

### 4.1 Read Access
**What data/resources does the agent need to read?** (Check all that apply)

- [ ] `registry_index` — Template registry index
- [ ] `template_catalog` — Available templates
- [ ] `all_public_semantic_memory` — All public semantic triples
- [ ] `cns_spans_all` — All CNS spans
- [ ] `variety_counters_all` — All variety counters
- [ ] `bot_reports_all` — All bot status reports
- [ ] `system_health_metrics` — System health data
- [ ] `own_episodic_memory` — Agent's own episodic memory
- [ ] `workspace_artifacts` — External workspace artifacts

**Additional read access:**
```
_________________________________________________________________
```

### 4.2 Execute Access
**What operations does the agent need to execute?** (Check all that apply)

- [ ] `template_dispatch` — Dispatch templates
- [ ] `metacognition_ops` — Metacognition operations
- [ ] `system_calibration` — Calibrate system parameters
- [ ] `user_requested_operations` — User-initiated actions
- [ ] `workspace_operations` — External workspace operations

**Additional execute access:**
```
_________________________________________________________________
```

### 4.3 Write Access
**What data does the agent need to write?** (Check all that apply)

- [ ] `own_episodic_memory` — Agent's episodic memory
- [ ] `public_semantic_memory` — Public semantic triples
- [ ] `bot_status_reports` — Status reports to curator
- [ ] `bridge_episodic_memory` — Cross-workspace bridge memory

**Additional write access:**
```
_________________________________________________________________
```

---

## Section 5: Responsibilities

### 5.1 Core Responsibilities
**What is the agent responsible for?** (Check all that apply)

#### Response Responsibilities
- [ ] `respond_to: template_dispatch_requests`
- [ ] `respond_to: user_queries`
- [ ] `respond_to: bot_coordination_requests`
- [ ] `respond_to: system_health_alerts`

#### Emission Responsibilities
- [ ] `emit: cns.prompt.select`
- [ ] `emit: cns.prompt.render`
- [ ] `emit: cns.prompt.outcome`
- [ ] `emit: cns.memory.operation`
- [ ] `emit: cns.agent_pod.*`
- [ ] `emit: cns.ensemble.coordination`

#### Recording Responsibilities
- [ ] `record: operations_to_episodic_memory`
- [ ] `record: user_interactions_to_episodic_memory`
- [ ] `record: bot_reports_to_episodic_memory`

#### Production Responsibilities
- [ ] `produce: semantic_triples_from_operations`
- [ ] `produce: semantic_triples_from_insights`
- [ ] `produce: system_state_updates`

#### Coordination Responsibilities
- [ ] `report_to: Curator`
- [ ] `report_to: standing_ensemble_session`
- [ ] `escalate_to: Curator`
- [ ] `coordinate: bot_ensemble_sessions`

**Additional responsibilities:**
```
_________________________________________________________________
_________________________________________________________________
```

### 5.2 Escalation Triggers
**What conditions should trigger escalation to supervisor?** (Check all that apply)

- [ ] `variety_deficit_gt_100` — CNS variety deficit exceeds 100
- [ ] `bot_coordination_failure` — Bot coordination fails
- [ ] `system_degradation_detected` — System performance degrades
- [ ] `energy_budget_critical` — Energy budget critical
- [ ] `template_selection_failure` — Cannot select template
- [ ] `memory_retrieval_failure` — Cannot retrieve memory
- [ ] `capability_denied` — Access denied repeatedly

**Additional escalation triggers:**
```
_________________________________________________________________
```

---

## Section 6: Workspace Integration

### 6.1 Workspace Type
**How does this agent integrate with workspaces?**

- [ ] **Standalone** — Independent crate, no workspace
- [ ] **New Workspace** — First crate in new workspace
- [ ] **Existing Workspace** — Additional crate in existing workspace
- [ ] **Bridge Agent** — Connects external workspace to hKask

**Selection:** `________________________`

### 6.2 Crate Name
**What is the crate name?**
- Must be valid Rust crate name (kebab-case)
- Example: `my-agent-crate`, `registry-dispatch-bot`

**Answer:** `________________________`

### 6.3 Dependencies
**What hKask crates does the agent depend on?** (Check all that apply)

- [ ] `hkask-mcp-inference` — Inference MCP server
- [ ] `hkask-mcp-memory` — Memory MCP server
- [ ] `hkask-mcp-registry` — Registry MCP server
- [ ] `hkask-mcp-web` — Web search/scrape MCP server
- [ ] `hkask-mcp-scholar` — Academic search MCP server
- [ ] `hkask-mcp-condenser` — Condensation MCP server
- [ ] `cns-curator-bot` — CNS curator bot
- [ ] `memory-curator-bot` — Memory curator bot
- [ ] `inference-curator-bot` — Inference curator bot
- [ ] `ensemble-curator-bot` — Ensemble curator bot

**Additional dependencies:**
```
_________________________________________________________________
```

### 6.4 External Workspace Configuration (Bridge Agents Only)

**External workspace name:** `________________________`

**Adapter crate name:** `________________________`

**Protocol:** 
- [ ] ACP-A2A
- [ ] Custom (specify): `________________________`

**Endpoint URL:** `________________________`

**Authentication method:**
- [ ] Macaroon
- [ ] API Key
- [ ] OAuth2
- [ ] Other: `________________________`

---

## Section 7: Visibility and Sharing

### 7.1 Default Visibility
**What is the default visibility for agent artifacts?**

- [ ] **public** — Visible to all agents
  - Use case: System bots, shared knowledge
- [ ] **private** — Visible only to agent itself
  - Use case: Personal episodic memory
- [ ] **shared** — Visible to agents with capability
  - Use case: Team collaboration

**Selection:** `________________________`

### 7.2 Episodic Memory Override
**Should episodic memory have different visibility?**

- [ ] Yes, episodic memory should be: `public` | `private` | `shared`
- [ ] No, use default visibility

**Selection:** `________________________`

### 7.3 Sharing Configuration
**If visibility is `shared`, which agents should have access?**

List agent names or capability requirements:
```
_________________________________________________________________
_________________________________________________________________
```

---

## Section 8: Reporting and Coordination

### 8.1 Receives Reports From
**Which agents does this agent receive reports from?**

List agent names:
```
_________________________________________________________________
_________________________________________________________________
```

### 8.2 Reports To
**Who does this agent report to?**

- [ ] `Curator` — System curator
- [ ] `standing_ensemble_session` — Ensemble coordination session
- [ ] `workspace-admin` — Workspace administrator
- [ ] Other: `________________________`

### 8.3 Report Interval
**How often should the agent report?**

- [ ] `on_event` — Report on each significant event
- [ ] `hourly` — Hourly summary reports
- [ ] `on_event_and_hourly_summary` — Both
- [ ] `daily` — Daily summary
- [ ] Custom: `________________________`

### 8.4 Standing Session Participation
**Will the agent participate in standing ensemble sessions?**

- [ ] Yes, as **orchestrator**
- [ ] Yes, as **participant**
- [ ] Yes, as **observer**
- [ ] No

**Session ID (if known):** `________________________`

**Report interval in session:** `________________________`

**Administrator visible:** [ ] Yes [ ] No

---

## Section 9: Process Manifest

### 9.1 Dispatch Pattern
**What dispatch pattern does the agent use?**

- [ ] **Simple Inference** — Direct LLM call
- [ ] **Memory Recall → Inference** — Retrieve memory, then infer
- [ ] **Template Selection → Population → Execution** — Full dispatch
- [ ] **CNS Monitoring → Calibration** — Metacognition cycle
- [ ] **Custom** — Describe below

**Custom pattern description:**
```
_________________________________________________________________
_________________________________________________________________
```

### 9.2 Manifest Location
**Where will the dispatch manifest be stored?**

- [ ] `registry/manifests/<agent-name>-dispatch.yaml`
- [ ] Custom path: `________________________`

### 9.3 Matroshka Configuration
**What is the maximum recursion depth?**

- [ ] Default: 7 (architecture limit)
- [ ] Custom: `____` (must be ≤ 7)

**Alert threshold (warning before limit):**
- [ ] Default: 6
- [ ] Custom: `____`

---

## Section 10: Templates

### 10.1 Required Templates
**What templates does the agent need?** (Check all that apply)

#### Selectors
- [ ] Template selector
- [ ] Model selector
- [ ] Tool selector
- [ ] Memory operation selector
- [ ] Custom selector: `________________________`

#### Prompts (WordAct)
- [ ] Prompt render template
- [ ] Prompt execute template
- [ ] Custom prompt: `________________________`

#### Processes (FlowDef)
- [ ] Dispatch workflow
- [ ] Memory operation workflow
- [ ] Custom process: `________________________`

#### Cognitions (KnowAct)
- [ ] Metacognition template
- [ ] Calibration template
- [ ] Detection template
- [ ] Custom cognition: `________________________`

### 10.2 Template Locations
**Where will templates be stored?**

- [ ] `templates/selectors/`
- [ ] `templates/prompts/`
- [ ] `templates/processes/`
- [ ] `templates/cognitions/`
- [ ] Custom: `________________________`

---

## Section 11: Readiness Probe

### 11.1 Health Check Endpoint
**What is the health check endpoint?**

Format: `<agent_name>::<action>_status`

Example: `registry::dispatch_status`, `curator::metacognition_status`

**Answer:** `________________________`

### 11.2 Expected Conditions
**What conditions indicate readiness?** (Check all that apply)

- [ ] `registry_index_available: true`
- [ ] `template_selector_ready: true`
- [ ] `memory_accessible: true`
- [ ] `inference_ready: true`
- [ ] `cns_spans_accessible: true`
- [ ] `bot_reports_available: true`
- [ ] Custom: `________________________`

### 11.3 Timeout and Retry
**Timeout (seconds):** [ ] Default: 10 [ ] Custom: `____`

**Retry count:** [ ] Default: 3 [ ] Custom: `____`

---

## Section 12: MACAROON Configuration (Optional)

### 12.1 Macaroon Issuance
**Does the agent need macaroon-based capability tokens?**

- [ ] Yes (for Russell ACP agent integration)
- [ ] No (standard hKask OCAP tokens)

### 12.2 Root Keys Required
**Which root keys are needed?** (Check all that apply)

- [ ] `rk_hkask_skill_registry` — Skill registration
- [ ] `rk_hkask_mcp` — MCP tool access
- [ ] `rk_hkask_okapi_discharge` — Okapi third-party discharge

### 12.3 Default Caveats
**What caveats should apply to issued macaroons?**

- [ ] `before: 24h` — 24-hour expiration
- [ ] `quota: 1000000-tokens/day` — Token quota
- [ ] `rpm: 100` — Requests per minute limit
- [ ] Custom caveats: `________________________`

---

## Section 13: CNS Integration

### 13.1 Span Emission
**What CNS spans should the agent emit?** (Check all that apply)

#### Lifecycle Spans
- [ ] `cns.agent_pod.registered`
- [ ] `cns.agent_pod.activated`
- [ ] `cns.agent_pod.deactivated`

#### Operation Spans
- [ ] `cns.prompt.select`
- [ ] `cns.prompt.render`
- [ ] `cns.prompt.outcome`
- [ ] `cns.memory.operation`
- [ ] `cns.ensemble.coordination`
- [ ] `cns.tool.invocation`

#### Monitoring Spans
- [ ] `cns.prompt.matroshka_depth`
- [ ] `cns.energy.consumption`
- [ ] `cns.variety.deficit`

**Custom spans:**
```
_________________________________________________________________
```

### 13.2 Algedonic Alerts
**What conditions should trigger algedonic alerts?**

- [ ] Variety deficit > 100
- [ ] Energy budget exceeded
- [ ] Coordination failure
- [ ] Custom: `________________________`

---

## Section 14: Additional Configuration

### 14.1 Energy Budget (Optional)
**Does the agent have specific energy budget requirements?**

- [ ] Yes, budget: `____` tokens/day
- [ ] Yes, budget: `____` tokens/request
- [ ] No, use default budget

### 14.2 Model Tier Preferences
**What model tier does the agent prefer?**

- [ ] `fast_local` — Fast, local models (e.g., qwen3:8b)
- [ ] `balanced` — Balanced speed/quality (e.g., qwen3:14b)
- [ ] `high_quality` — Highest quality (e.g., qwen3:70b)
- [ ] Mixed (specify per operation):
```
_________________________________________________________________
```

### 14.3 Custom Configuration
**Any additional configuration not covered above?**

```
_________________________________________________________________
_________________________________________________________________
_________________________________________________________________
```

---

## Section 15: Review and Validation

### 15.1 Completeness Checklist

- [ ] Section 1: Agent Identity — Complete
- [ ] Section 2: Agent Purpose — Complete
- [ ] Section 3: Capabilities — Complete
- [ ] Section 4: Rights and Access — Complete
- [ ] Section 5: Responsibilities — Complete
- [ ] Section 6: Workspace Integration — Complete
- [ ] Section 7: Visibility and Sharing — Complete
- [ ] Section 8: Reporting and Coordination — Complete
- [ ] Section 9: Process Manifest — Complete
- [ ] Section 10: Templates — Complete
- [ ] Section 11: Readiness Probe — Complete
- [ ] Section 12: MACAROON Configuration — Complete (if applicable)
- [ ] Section 13: CNS Integration — Complete
- [ ] Section 14: Additional Configuration — Complete

### 15.2 Consistency Checks

- [ ] Capabilities match rights (no capability without corresponding right)
- [ ] Responsibilities align with purpose
- [ ] Visibility settings appropriate for agent type
- [ ] Dependencies are available
- [ ] Escalation triggers match supervisor configuration

### 15.3 Sign-off

**Completed by:** `________________________`

**Date:** `________________________`

**Reviewer (if applicable):** `________________________`

**Approval status:** [ ] Approved [ ] Pending review [ ] Needs revision

---

## Next Steps

After completing this questionnaire:

1. **Review answers** with team/stakeholders
2. **Generate agent persona YAML** from responses
3. **Create dispatch manifest** based on process pattern
4. **Develop templates** for selectors, prompts, processes, cognitions
5. **Build agent crate** with all files
6. **Register with ACP runtime** using CLI or API
7. **Activate pod** and verify readiness
8. **Monitor CNS spans** for proper operation

For assistance, refer to:
- [Agent Pod Creation Guide](./AGENT-POD-CREATION-GUIDE.md)
- [hKask Architecture Documentation](../architecture/hKask-architecture-master.md)
- [Agent Pod Implementation](../architecture/AGENT_POD_IMPLEMENTATION.md)

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
