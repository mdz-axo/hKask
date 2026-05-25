# Agent Interaction Implementation Plan

**Date:** 2026-05-24  
**Status:** Draft  
**Scope:** Tasks 1–6 — bringing hKask's agents to life as inhabitants of a governed digital space  
**Version:** v0.21.0  

---

## 1. Context and Motivation

hKask provides a **digital space for agents**. Its fundamental service is granting any agent — human, bot, or replicant — simultaneous access to **memory**, **inference**, **tools** (16 MCP servers), and a **community of other agents**. The pod is the unit of access: an agent enters a pod and the space opens to it.

Five user-facing questions motivated this plan:

1. How do I chat with the Curator?
2. How do I create a multi-agent chat with the Curator and Russell to manage the local system?
3. For the bots (the 7R7) to be fully functioning — what do we need to do?
4. Can I chat directly with Russell in `kask`?
5. How do I register Russell and other ACP agents?

These questions share two structural root causes:

### Gap A — The space is unoccupied

Bot YAMLs define 7 agents with rich identities, capabilities, and responsibilities in `registry/bots/`. None are loaded into runtime. The standing ensemble session is 300 lines of YAML (`registry/manifests/standing-ensemble-session.yaml`) with zero bootstrap code. The Curator has a 333-line persona spec (`docs/architecture/hKask-Curator-persona.md`) but no enforcement path. The agents exist as documents, not as inhabitants.

### Gap B — The pod doesn't open for individuals

`kask chat` works but routes everything through a single inference path with heuristic template selection (`main.rs:654-701`). There is no way to address a specific agent because the pod — the unit of access to memory + inference + tools — is not connected to agent identity. The CLI creates `PodManager::new_mock()` instances that are ephemeral and disconnected.

---

## 2. Design Principles

| Principle | Implication |
|-----------|-------------|
| **Minimal viable** | Do the least that makes the system whole. No speculative features. |
| **Pre-release** | Rewrite freely. Delete stubs, dead code, and patterns that no longer serve. No migration burden. |
| **Agents are citizens** | The Curator and 7R7 have personas, backstories, and relationships. Bring them to life. |
| **Pod is the space** | Memory + inference + tools are accessed through a pod. No pod, no access. |
| **Hexagonal integrity** | Domain logic in the core. Infrastructure at the edges. Ports define boundaries. |

---

## 3. Task Dependency Graph

```
Task 1 (Registry Loader)
  ├──► Task 2 (Pod-as-Space)
  │      └──► Task 5 (Personas)
  ├──► Task 3 (Standing Ensemble)
  │      └──► Task 5 (Personas)
  └──► Task 4 (ACP Registration)
         └──► Task 2 (Pod-as-Space, for Russell)

Task 6 (Future) — open questions, no implementation
```

**Critical path:** Task 1 → Task 2 → Task 5. Task 1 is the keystone; everything depends on agents existing at runtime.

---

## 4. Task 1 — Agent Registry Loader

### 4.1 Goal

On boot, parse `registry/bots/*.yaml`, register each agent with `AcpRuntime`, mint capability tokens, and make the fleet queryable.

### 4.2 Why This Is the Keystone

Every other task depends on agents existing at runtime. Without this, the Curator is a YAML file, the 7R7 are a concept, and Russell has no home.

### 4.3 Current State

| Component | File | Status |
|-----------|------|--------|
| Bot YAML definitions (7 bots + Curator) | `registry/bots/*.yaml` | Complete |
| `AcpRuntime::register_agent()` | `crates/hkask-agents/src/acp.rs:388` | Implemented |
| `RootAuthority` token minting | `crates/hkask-agents/src/acp.rs` | Implemented |
| `kask bot list` / `kask bot grant` | `crates/hkask-cli/src/main.rs:970-982` | Stubbed |
| YAML → `Bot` struct loader | — | Does not exist |
| Persistent agent registry | — | Does not exist |

### 4.4 Implementation

#### 4.4.1 New module: `hkask-agents/src/registry_loader.rs`

```rust
// Pseudocode — domain logic, not adapter code

pub struct BotRegistryLoader {
    registry_path: PathBuf,        // "registry/bots/"
    acp_runtime: Arc<dyn AcpPort>,
    storage: Arc<dyn StoragePort>,
}

impl BotRegistryLoader {
    /// Load all bot YAMLs, register each with ACP, persist registry.
    pub async fn load_all(&self) -> Result<Vec<RegisteredBot>, RegistryError> {
        let yamls = self.read_bot_yamls()?;
        let mut registered = Vec::new();
        
        for yaml in yamls {
            let bot_def = BotDefinition::from_yaml(&yaml)?;
            self.validate_capabilities(&bot_def.capabilities)?;  // no wildcards
            
            let token = self.acp_runtime.register_agent(
                bot_def.webid.clone(),
                bot_def.agent_type.as_str(),
                bot_def.capabilities.clone(),
            ).await?;
            
            self.storage.persist_agent(&bot_def, &token)?;
            registered.push(RegisteredBot { definition: bot_def, token });
        }
        
        Ok(registered)
    }
    
    /// Restore from storage if available, else load from YAML.
    pub async fn boot(&self) -> Result<Vec<RegisteredBot>, RegistryError> {
        match self.storage.load_agent_registry().await? {
            Some(registry) => Ok(registry),
            None => self.load_all().await,
        }
    }
}
```

#### 4.4.2 Bot YAML schema (parse target)

Each `registry/bots/*.yaml` has this structure (from `Curator.yaml`):

```yaml
agent:
  name: Curator
  type: Replicant
  binding_contract: true

charter:
  description: >
    Canonical system persona responsible for metacognition...
  archetype: MaintenanceAdvisory

capabilities:
  - tool:cns:emit
  - tool:memory:recall
  - tool:inference:call
  # ...

rights:
  - read: all_public_semantic_memory
  - write: own_episodic_memory
  # ...

responsibilities:
  - monitor: system_health_via_cns
  - escalate: critical_alerts_to_administrator
  # ...

persona:
  tone: Direct and to the point
  verbosity: Minimal
  forbidden: [preamble, postamble, emojis]
```

The loader must parse all of these fields into a `BotDefinition` struct that captures the full agent identity — not just name and capabilities, but charter, rights, responsibilities, and persona.

#### 4.4.3 New types in `hkask-types`

```rust
pub struct BotDefinition {
    pub name: String,
    pub webid: WebID,
    pub agent_type: AgentType,           // Bot | Replicant
    pub charter: Charter,
    pub capabilities: Vec<String>,
    pub rights: Vec<Right>,
    pub responsibilities: Vec<Responsibility>,
    pub persona: PersonaConstraints,
    pub depends_on: Vec<String>,
    pub readiness_probe: Option<ReadinessProbe>,
}

pub struct Charter {
    pub description: String,
    pub archetype: String,
    pub visibility: Visibility,
}

pub struct PersonaConstraints {
    pub tone: String,
    pub verbosity: String,
    pub forbidden: Vec<String>,
    pub required: Vec<String>,
}
```

#### 4.4.4 Persistence via `hkask-storage`

Add a table to the existing SQLCipher database:

```sql
CREATE TABLE IF NOT EXISTS agent_registry (
    webid        TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    agent_type   TEXT NOT NULL,
    capabilities TEXT NOT NULL,  -- JSON array
    charter      TEXT NOT NULL,  -- JSON object
    persona      TEXT NOT NULL,  -- JSON object
    token_hash   TEXT NOT NULL,  -- HMAC of capability token
    registered_at TEXT NOT NULL,
    source_yaml  TEXT NOT NULL   -- path to source YAML for reload
);
```

#### 4.4.5 CLI wiring

**Delete** the stubbed handlers at `main.rs:970-982`.

**Replace** with:

```
kask bot list     → query AcpRuntime::is_registered() + list_capabilities() for each loaded bot
kask bot grant    → AcpRuntime capability attenuation via RootAuthority::attenuate()
kask bot status   → show each bot's readiness probe result
```

#### 4.4.6 The 7R7

The 7 system bots become the **7R7** by virtue of being loaded from the registry. No separate manifest needed. The fleet *is* the registry contents:

| Bot | YAML | Domain |
|-----|------|--------|
| `cns-curator-bot` | `registry/bots/cns-curator-bot.yaml` | CNS monitoring, variety counters, algedonic alerts |
| `memory-curator-bot` | `registry/bots/memory-curator-bot.yaml` | Memory operations, confidence tracking, visibility gating |
| `inference-curator-bot` | `registry/bots/inference-curator-bot.yaml` | Inference orchestration, model tier selection |
| `mcp-dispatch-bot` | `registry/bots/mcp-dispatch-bot.yaml` | MCP tool dispatch, OCAP verification |
| `ensemble-curator-bot` | `registry/bots/ensemble-curator-bot.yaml` | Multi-agent coordination, H2A session orchestration |
| `git-curator-bot` | `registry/bots/git-curator-bot.yaml` | Git CAS operations, versioning |
| `registry-dispatch-bot` | `registry/bots/registry-dispatch-bot.yaml` | Template dispatch orchestration |

---

## 5. Task 2 — Pod-as-Space: Agent-Addressed Chat

### 5.1 Goal

Any `kask chat` invocation opens a pod that grants the addressed agent access to memory, inference, and tools. The agent's persona shapes the interaction.

### 5.2 Why This Matters

The pod is hKask's core service. Right now `kask chat` runs inference without a pod context — no memory access, no tool access, no agent identity. This is the gap between "a chatbot" and "a digital space for agents."

### 5.3 Current State

| Component | File | Status |
|-----------|------|--------|
| `kask chat --interactive` | `main.rs:616-652` | Working (Okapi inference) |
| `process_chat_input_async()` | `main.rs:654-701` | Working (no pod context) |
| Template auto-selection | `main.rs:670-690` | Heuristic string matching |
| `PodManager` | `hkask-agents/src/pod.rs` | Implemented (ports defined) |
| `PodManager::new_mock()` in CLI | `commands.rs:114,133,162,177,192` | Ephemeral, disconnected |
| `POST /api/chat` | `routes.rs:688-756` | Working (no pod context) |

### 5.4 Implementation

#### 5.4.1 Refactor `process_chat_input_async()`

The current flow:

```
stdin → heuristic template select → OkapiInference::generate() → stdout
```

The target flow:

```
stdin → resolve agent → open pod(agent, capabilities) → 
  pod provides {memory, inference, tools} →
  agent persona as system prompt → OkapiInference::generate() → stdout
```

```rust
// Pseudocode

async fn process_chat_input_async(input: &str, agent_name: Option<&str>) -> Result<String> {
    // 1. Resolve agent
    let agent = match agent_name {
        Some("russell") => AgentRef::Russell,
        Some(name) => AgentRef::Registered(name.to_string()),
        None => AgentRef::Curator,  // default
    };
    
    // 2. Open pod with agent's capability set
    let pod = pod_manager.create(
        agent.webid(),
        agent.capabilities(),
    ).await?;
    
    // 3. Build system prompt from agent persona
    let system_prompt = agent.compose_system_prompt();
    
    // 4. Select template via registry (not heuristic)
    let template = registry.search(input, &agent.template_types())?
        .first()
        .cloned();
    
    // 5. Generate within pod context
    let response = inference.generate(
        &system_prompt,
        input,
        template.as_ref(),
        &pod,  // pod provides memory recall, tool access
    ).await?;
    
    Ok(response)
}
```

#### 5.4.2 CLI interface

```
kask chat                          # Chat with Curator (default)
kask chat --agent russell          # Chat with Russell
kask chat --agent cns-curator-bot  # Chat with a specific bot
kask chat --interactive            # Interactive mode (Curator default)
kask chat --interactive --agent russell  # Interactive with Russell
```

#### 5.4.3 Russell-specific path

When `--agent russell`:

1. `RussellAcpAdapter` creates/reuses a session via `acp/session.create`
2. The pod grants Russell its bridged capabilities
3. Russell's persona ("Jack") loads from its config
4. Interactive loop: stdin → `acp/session.message` → stdout
5. Session ID persisted in `hkask-storage` for reconnection

#### 5.4.4 Delete

- **Delete** heuristic template selector (string matching on `?`, `"create"`, etc.)
- **Delete** `PodManager::new_mock()` pattern from all CLI commands
- **Delete** the inline inference logic in `POST /api/chat` — route through the same pod-based path the CLI uses

#### 5.4.5 Reuse

| Existing Component | Role |
|--------------------|------|
| `PodManager` | Pod lifecycle |
| `MemoryStoragePort` | Memory access within pod |
| `InferencePort` | Okapi inference within pod |
| `MCPRuntimePort` | Tool access within pod |
| `RussellAcpAdapter` | Russell session management |
| `SqliteRegistry` | Template resolution |

---

## 6. Task 3 — Standing Ensemble: Agents in Community

### 6.1 Goal

The standing ensemble session boots automatically. The 7R7 report to the Curator. The Administrator observes and participates. This is the "community of agents" half of hKask's core service.

### 6.2 Why This Matters

hKask is not just tools-for-individual-agents. It is a space where agents coordinate, deliberate, and govern together. The standing session is where this happens. Without it, the bots are isolated workers, not a community.

### 6.3 Current State

| Component | File | Status |
|-----------|------|--------|
| `EnsembleChat` | `hkask-ensemble/src/chat.rs` | Implemented (participants, messages, sovereignty) |
| `DeliberationCoordinator` | `hkask-ensemble/src/deliberation.rs` | Implemented |
| `dispatch_to_bot()` | `chat.rs:168-180` | Simulated (returns formatted string) |
| Standing session YAML | `registry/manifests/standing-ensemble-session.yaml` | Complete (300 lines) |
| API ensemble routes | `routes.rs:996-1103` | Stubbed (return success, no logic) |
| CLI ensemble commands | `main.rs:452-533` | Working (ephemeral `OnceLock` singletons) |

### 6.4 Implementation

#### 6.4.1 Boot sequence

After Task 1 loads the agent registry:

```
1. Parse standing-ensemble-session.yaml
2. Create EnsembleChat session with ID "system-coordination-standing-session"
3. Register Curator as orchestrator
4. Register each 7R7 bot as participant
5. Activate session
6. Emit initial Curator message: "All participants: report status."
7. Each bot responds with its readiness status
8. Persist session to hkask-storage
```

#### 6.4.2 Standing session bootstrap module

New module: `hkask-ensemble/src/standing_session.rs`

```rust
pub struct StandingSession {
    chat: EnsembleChat,
    config: StandingSessionConfig,  // parsed from YAML
    storage: Arc<dyn StoragePort>,
}

impl StandingSession {
    /// Boot the standing session from YAML config + registered agents.
    pub async fn boot(
        config_path: &Path,
        registry: &[RegisteredBot],
        storage: Arc<dyn StoragePort>,
    ) -> Result<Self, EnsembleError> {
        let config = StandingSessionConfig::from_yaml(config_path)?;
        let mut chat = EnsembleChat::new(config.session_id.clone());
        
        // Register Curator
        chat.register_participant(
            config.curator_webid(),
            ChatParticipantRole::Curator,
        )?;
        
        // Register each 7R7 bot
        for participant in &config.participants {
            if participant.agent != "Curator" {
                chat.register_participant(
                    participant.webid(),
                    ChatParticipantRole::Custom(participant.agent.clone()),
                )?;
            }
        }
        
        // Activate and persist
        chat.activate()?;
        storage.persist_ensemble_session(&chat).await?;
        
        Ok(Self { chat, config, storage })
    }
    
    /// Administrator sends a message to the standing session.
    pub async fn administrator_message(&mut self, message: &str) -> Result<String, EnsembleError> {
        // Curator receives first, decides which bots to involve
        let routing = self.curator_route(message).await?;
        
        // Dispatch to relevant bots
        for target in &routing.targets {
            self.chat.send_message(
                &self.config.curator_webid(),
                target,
                &routing.instruction_for(target),
            ).await?;
        }
        
        // Collect responses and synthesize
        let responses = self.chat.collect_responses(&routing.correlation_id).await?;
        let synthesis = self.curator_synthesize(message, &responses).await?;
        
        self.storage.persist_messages(&self.chat.session_id(), &responses).await?;
        
        Ok(synthesis)
    }
}
```

#### 6.4.3 Replace simulated dispatch

**Delete** `dispatch_to_bot()` at `chat.rs:168-180`.

**Replace** with real template execution:

```rust
async fn dispatch_to_bot(
    bot_webid: &WebID,
    message: &str,
    pod_manager: &PodManager,
    registry: &SqliteRegistry,
) -> Result<String, EnsembleError> {
    // 1. Open a pod for the bot
    let pod = pod_manager.create(bot_webid, bot_capabilities(bot_webid)).await?;
    
    // 2. Resolve bot's template set
    let templates = registry.search_by_agent(bot_webid)?;
    
    // 3. Match message to template
    let template = select_template(&templates, message)?;
    
    // 4. Execute via CuratorPipeline within the pod
    let result = CuratorPipeline::evaluate(template, message, &pod).await?;
    
    Ok(result)
}
```

#### 6.4.4 Connect API routes

**Delete** stub returns at `routes.rs:996-1103`.

**Replace** with calls to the live `StandingSession` instance. The API server holds an `Arc<StandingSession>` initialized at boot.

#### 6.4.5 CLI

- `kask ensemble status` — show standing session health, participant list, last report times
- `kask ensemble send --message <text>` — send to standing session as Administrator
- `kask ensemble history --limit <n>` — show recent messages

**Delete** the `OnceLock` singleton pattern. The standing session lives in `hkask-storage`, not in process memory.

---

## 7. Task 4 — ACP Registration: Agents Joining the Space

### 7.1 Goal

Any external agent (Russell, future agents) can register with hKask, receive a capability token, and access the space through a pod.

### 7.2 Current State

| Component | File | Status |
|-----------|------|--------|
| `POST /api/v1/acp/register` | `routes.rs:194-230` | Working (inline logic) |
| `AcpRuntime::register_agent()` | `acp.rs:388` | Implemented |
| `RootAuthority` token minting | `acp.rs` | Implemented (HMAC-SHA256) |
| `RussellAcpAdapter` | `adapters/russell_acp.rs` | Implemented (JSON-RPC over stdio) |
| `RevocationStore` | `acp.rs` | Implemented |
| `AuditLog` | `acp.rs:902` | Implemented (in-memory + SQLite) |
| CLI `kask agent register` | — | Does not exist |
| Persistent agent registry | — | Does not exist (in-memory `HashMap` only) |

### 7.3 Implementation

#### 7.3.1 CLI commands

```
kask agent register --webid <URI> --type <Bot|Replicant> --capabilities <cap1,cap2,...>
kask agent unregister --webid <URI>
kask agent list
kask agent capabilities --webid <URI>
```

Each delegates to `AcpRuntime` — the same path the API uses. One registration flow, two interfaces.

#### 7.3.2 Unify API and CLI registration paths

**Delete** the inline logic in `POST /api/v1/acp/register` (`routes.rs:194-230`).

**Replace** with a call to a shared service:

```rust
pub struct AgentRegistrationService {
    acp_runtime: Arc<AcpRuntime>,
    storage: Arc<dyn StoragePort>,
    audit_log: Arc<AuditLog>,
}

impl AgentRegistrationService {
    pub async fn register(
        &self,
        webid: WebID,
        agent_type: &str,
        capabilities: Vec<String>,
        operator: &str,
    ) -> Result<RegistrationReceipt, AcpError> {
        // Validate
        self.validate_no_wildcards(&capabilities)?;
        self.validate_not_duplicate(&webid).await?;
        
        // Register
        let token = self.acp_runtime.register_agent(
            webid.clone(), agent_type, capabilities.clone(),
        ).await?;
        
        // Persist
        self.storage.persist_agent_registration(&webid, agent_type, &capabilities, &token).await?;
        
        // Audit
        self.audit_log.record(AuditEvent::AgentRegistered {
            webid: webid.clone(),
            agent_type: agent_type.to_string(),
            capabilities: capabilities.clone(),
            operator: operator.to_string(),
            timestamp: Utc::now(),
        }).await?;
        
        Ok(RegistrationReceipt { webid, token, registered_at: Utc::now() })
    }
}
```

Both the CLI command and the API route call `AgentRegistrationService::register()`.

#### 7.3.3 Persistence

Agent registrations persist to SQLCipher via `hkask-storage`. On boot, `AcpRuntime` restores from storage before `BotRegistryLoader` runs (Task 1). This means:

- Manually registered agents (Russell, external agents) survive restarts
- Registry-loaded agents (7R7) are re-registered from YAML if not already in storage
- No conflict: YAML load is idempotent (skip if already registered)

#### 7.3.4 Russell-specific registration

Russell registers via `RussellAcpAdapter` which already implements `AcpPort`. The flow:

1. `kask agent register --webid urn:russell --type Bot --capabilities chat:russell,inference:call`
2. `AgentRegistrationService` mints a token
3. `RussellAcpAdapter` uses the `bridge_secret` to establish trust between hKask's `RootAuthority` and Russell's session tokens
4. On `kask chat --agent russell`, the pod verifies the bridge token before granting access

#### 7.3.5 Security

| Concern | Mitigation |
|---------|-----------|
| Token storage | SQLCipher encryption at rest |
| Bridge secret | Rotated on restart, never persisted in plaintext |
| Token expiry | `CapabilityToken` carries `expires_at` field |
| Revocation | `RevocationStore` checked on every `send_message()` |
| Rate limiting | `RateLimiter` at 100 msg/min default per agent |
| Cross-machine ACP | Excluded by design (loopback-only transports) |
| Wildcard capabilities | Rejected at registration (`validate_no_wildcards`) |

#### 7.3.6 Capability composition (Miller)

Each agent receives only the minimum capability set. Tokens are attenuable:

```rust
// RootAuthority::attenuate() derives a narrower token from a broader one
let narrow_token = root_authority.attenuate(
    &parent_token,
    vec!["tool:memory:recall"],  // subset of parent's capabilities
)?;
```

Capability intersection enforced at dispatch: `agent_token ∩ template_required_capabilities = authorized_set`. If the intersection is empty, dispatch is denied.

---

## 8. Task 5 — Bring the Personas to Life

### 8.1 Goal

The Curator and 7R7 bots behave according to their defined personas, backstories, and governance roles — not just as struct fields, but as agents with voice, judgment, and responsibility.

### 8.2 Why This Matters

The registry YAMLs define rich agents. The Curator has metacognition duties and escalation triggers. `cns-curator-bot` watches variety counters. `memory-curator-bot` gates visibility between episodic and semantic. These are not microservices. They are inhabitants of a digital space with roles and relationships.

### 8.3 Implementation

#### 8.3.1 YAML → System Prompt

Each bot's YAML becomes its system prompt when loaded into a pod. The composition:

```rust
impl BotDefinition {
    pub fn compose_system_prompt(&self) -> String {
        let mut prompt = String::new();
        
        // Identity
        prompt.push_str(&format!("You are {}, a {} in the hKask system.\n\n",
            self.name, self.agent_type));
        
        // Charter
        prompt.push_str(&format!("## Charter\n{}\n\n", self.charter.description));
        
        // Responsibilities
        prompt.push_str("## Responsibilities\n");
        for r in &self.responsibilities {
            prompt.push_str(&format!("- {}\n", r));
        }
        
        // Rights
        prompt.push_str("\n## Rights\n");
        for r in &self.rights {
            prompt.push_str(&format!("- {}\n", r));
        }
        
        // Persona constraints
        prompt.push_str(&format!("\n## Voice\nTone: {}\nVerbosity: {}\n",
            self.persona.tone, self.persona.verbosity));
        if !self.persona.forbidden.is_empty() {
            prompt.push_str(&format!("Never use: {}\n",
                self.persona.forbidden.join(", ")));
        }
        
        prompt
    }
}
```

This is the **only** persona enforcement mechanism. No post-hoc classifiers. No output filters. The system prompt *is* the persona. If Okapi doesn't follow it, refine the prompt.

#### 8.3.2 Curator metacognition loop

The Curator's primary responsibility is governing the space, not answering chat. Implement a periodic metacognition cycle:

```rust
pub struct MetacognitionLoop {
    curator: BotDefinition,
    standing_session: Arc<StandingSession>,
    cns: Arc<dyn CnsPort>,
    escalation_queue: Arc<EscalationQueue>,
    interval: Duration,  // from YAML: hourly
}

impl MetacognitionLoop {
    pub async fn run(&self) {
        loop {
            tokio::time::sleep(self.interval).await;
            
            // 1. Query CNS spans
            let health = self.cns.system_health().await;
            let variety = self.cns.variety_counters().await;
            
            // 2. Collect bot status reports from standing session
            let bot_reports = self.standing_session.latest_bot_reports().await;
            
            // 3. Synthesize system state
            let synthesis = self.curator_synthesize(&health, &variety, &bot_reports).await;
            
            // 4. Check escalation triggers
            if self.should_escalate(&variety, &health, &bot_reports) {
                self.escalation_queue.add(Escalation {
                    reason: self.escalation_reason(&variety, &health),
                    system_state: synthesis.clone(),
                    timestamp: Utc::now(),
                }).await;
            }
            
            // 5. Post synthesis to standing session
            self.standing_session.post_curator_update(&synthesis).await;
        }
    }
    
    fn should_escalate(&self, variety: &VarietyCounters, health: &SystemHealth, reports: &[BotReport]) -> bool {
        variety.deficit() > 100
            || health.is_degraded()
            || reports.iter().any(|r| r.status == BotStatus::Critical)
    }
}
```

#### 8.3.3 Escalation visibility

The `EscalationQueue` (already implemented at `curator/escalation.rs`) persists to SQLite. When the Administrator runs `kask chat`, the Curator checks the queue and surfaces unresolved escalations:

```
Curator: You have 2 unresolved escalations:
1. [CRITICAL] variety_deficit at 142 — exceeds threshold of 100
2. [HIGH] memory-curator-bot unresponsive for 120s
```

#### 8.3.4 Readiness probes

Each bot YAML defines a `readiness_probe`. The Curator invokes these via the standing session:

```yaml
readiness_probe:
  type: health_check
  endpoint: curator::metacognition_status
  expected:
    bot_reports_available: true
    cns_spans_accessible: true
  timeout_seconds: 15
```

The Curator calls each bot's probe endpoint. If a bot fails its probe, the Curator reports it in the standing session and may escalate.

#### 8.3.5 Keep it simple

- No persona classifiers
- No output post-processors
- No regex filters for forbidden words
- The system prompt is the persona
- If the model doesn't follow it, the prompt is wrong — fix the prompt

---

## 9. Task 6 — Future (Open Questions)

These aspects remain underspecified and require design decisions before implementation.

### 9.1 Russell's Memory Boundary

When Russell participates in ensemble chat and produces an insight, does it become public semantic memory automatically or require Curator promotion?

**Options:**
- **Automatic:** Russell's ensemble messages are indexed as semantic memory with `visibility: public`. Simple but loses sovereignty control.
- **Curator-promoted:** Russell's insights stay in Russell's episodic memory until the Curator explicitly promotes them to semantic memory. Respects memory boundaries but adds latency.
- **Consent-gated:** Russell's `SovereigntyPort` is consulted — Russell consents to promotion per-message or per-session. Most aligned with hKask sovereignty principles.

### 9.2 Bot Execution Model

Are the 7R7 long-running daemons, periodic jobs, or event-driven reactors?

**Constraint:** The standing session YAML allocates 15,000 tokens/bot/session with a session cap of 150,000. This implies bounded, periodic activity — not continuous presence.

**Recommendation:** Event-driven reactors. Each bot activates when:
- The Curator dispatches to it via the standing session
- Its domain triggers an event (CNS alert, memory threshold, git commit)
- Its scheduled report interval fires (hourly)

Between activations, the bot is dormant — no process, no resource consumption. The pod opens on activation and closes on completion.

### 9.3 Pod Lifecycle Across Sessions

When `kask chat` ends, does the pod persist or dissolve?

**Recommendation:**
- **Standing session pods:** Persist. The 7R7 and Curator pods live as long as the standing session.
- **Ad-hoc chat pods:** Dissolve on session end. Memory artifacts produced during the session are persisted to `hkask-storage` before dissolution.
- **Russell pods:** Persist for the duration of the Russell session (which may span multiple `kask chat` invocations via session ID reconnection).

### 9.4 Agent-to-Agent Trust Delegation

When the Curator delegates to a bot, the bot acts within its own capability set. But what if the Curator needs a bot to act *on its behalf* with Curator-level access?

**Current capability:** `RootAuthority::attenuate()` supports deriving narrower tokens. The inverse — *amplification* — is not supported and should not be. Instead:

- The Curator can **request** a bot perform an action within the bot's own capabilities
- If the action requires Curator-level access, the Curator performs it directly
- No delegation of authority across agent boundaries

This is the simplest model and aligns with OCAP principles: you can only give away what you have, and you can't create authority you don't possess.

### 9.5 The 7R7 Backstories

Each bot YAML has a charter and responsibilities but limited narrative identity. The Curator has a 333-line persona doc. Do the 7R7 bots need comparable persona documents?

**Recommendation:** No. The YAML charter is sufficient for system prompt generation (Task 5). The Curator's extended persona doc exists because the Curator is the human-facing agent — its voice matters more. The 7R7 bots communicate primarily with each other and the Curator in the standing session, where precision matters more than personality.

If a specific bot's voice needs refinement, extend its YAML `persona` section — don't create separate persona documents.

### 9.6 Multi-Tier Inference Cascade

`ConfidenceRouter` supports 2-tier escalation (`qwen3:8b` → `qwen3:70b`). Should this generalize?

**Recommendation:** Not yet. 2-tier is sufficient for the current fleet. Generalize when a concrete use case demands it. The `ConfidenceRouter` interface is already extensible — adding tiers is a configuration change, not an architectural one.

---

## 10. Implementation Order

| Phase | Tasks | Estimated Complexity | Dependencies |
|-------|-------|---------------------|--------------|
| **Phase 1** | Task 1 (Registry Loader) | Medium | None |
| **Phase 2** | Task 2 (Pod-as-Space) + Task 4 (ACP Registration) | High | Task 1 |
| **Phase 3** | Task 3 (Standing Ensemble) | High | Tasks 1, 2 |
| **Phase 4** | Task 5 (Personas) | Medium | Tasks 2, 3 |
| **Phase 5** | Task 6 (Future) | TBD | Design decisions |

### Phase 1 Deliverables

- `BotRegistryLoader` module in `hkask-agents`
- `BotDefinition`, `Charter`, `PersonaConstraints` types in `hkask-types`
- `agent_registry` table in `hkask-storage`
- Working `kask bot list` / `kask bot status` commands
- `cargo test -p hkask-agents` passes with registry loader tests

### Phase 2 Deliverables

- Refactored `process_chat_input_async()` with pod context
- `kask chat --agent <name>` working for Curator, Russell, and any registered bot
- `kask agent register/unregister/list/capabilities` CLI commands
- `AgentRegistrationService` unifying CLI and API paths
- Persistent agent registrations in SQLCipher
- `cargo test -p hkask-cli` and `cargo test -p hkask-agents` pass

### Phase 3 Deliverables

- Standing session boots from YAML on system start
- Real template-mediated dispatch replaces simulated `dispatch_to_bot()`
- API ensemble routes connected to live `StandingSession`
- `kask ensemble status/send/history` commands
- `cargo test -p hkask-ensemble` passes

### Phase 4 Deliverables

- System prompt composition from YAML for all agents
- Curator metacognition loop running on interval
- Escalation visibility in `kask chat`
- Readiness probe invocation by Curator
- Integration test: full boot → standing session → metacognition cycle → escalation

---

## 11. Verification Strategy

### Unit Tests

- `BotRegistryLoader`: parse each YAML, validate capabilities, idempotent re-registration
- `AgentRegistrationService`: register, unregister, duplicate rejection, wildcard rejection
- `BotDefinition::compose_system_prompt()`: output contains charter, responsibilities, persona
- `StandingSession::boot()`: correct participant count, Curator as orchestrator

### Integration Tests

- Boot sequence: YAML load → ACP register → standing session activate → all 8 participants present
- Chat flow: `kask chat` → pod opens → Curator persona in system prompt → Okapi response
- Russell flow: `kask chat --agent russell` → Russell session created → messages round-trip
- Escalation: variety deficit > 100 → `EscalationQueue` populated → surfaced in next chat

### Manual Verification

- `kask chat` produces Curator-voiced responses (no preamble, no emoji, minimal)
- `kask chat --agent russell` opens Russell session
- `kask bot list` shows all 7R7 + Curator with capabilities
- `kask ensemble status` shows standing session with 8 participants
- `kask agent register` mints and persists a token

---

## 12. Files to Create

| File | Purpose |
|------|---------|
| `crates/hkask-agents/src/registry_loader.rs` | `BotRegistryLoader` |
| `crates/hkask-ensemble/src/standing_session.rs` | `StandingSession` bootstrap |
| `crates/hkask-agents/src/registration_service.rs` | `AgentRegistrationService` |
| `crates/hkask-agents/src/metacognition.rs` | `MetacognitionLoop` |

## 13. Files to Modify

| File | Change |
|------|--------|
| `crates/hkask-types/src/lib.rs` | Add `BotDefinition`, `Charter`, `PersonaConstraints`, `ReadinessProbe` |
| `crates/hkask-cli/src/main.rs` | Refactor chat, add agent commands, wire bot commands |
| `crates/hkask-api/src/routes.rs` | Unify registration, connect ensemble routes |
| `crates/hkask-ensemble/src/chat.rs` | Replace `dispatch_to_bot()` with real template execution |
| `crates/hkask-storage/src/lib.rs` | Add `agent_registry` table, session persistence |

## 14. Files to Delete

| File/Code | Reason |
|-----------|--------|
| `main.rs:970-982` (stubbed bot handlers) | Replaced by live queries |
| `main.rs:670-690` (heuristic template selector) | Replaced by registry-based routing |
| `commands.rs:114,133,162,177,192` (`PodManager::new_mock()`) | Pods are real or they don't exist |
| `routes.rs:996-1103` (stubbed ensemble routes) | Connected to live `StandingSession` |
| `chat.rs:168-180` (simulated dispatch) | Replaced by real template execution |

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
