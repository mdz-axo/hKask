# Task 4: hKask Alignment Analysis

## 4.1 Comparison: Hermes/Claude/Codex vs. hKask

| Feature | Hermes Agent | Claude Code | OpenAI Codex | hKask (Current) | hKask (Proposed) |
|---------|--------------|-------------|--------------|-----------------|------------------|
| **Goal Storage** | SessionDB (per-session) | Unknown | Unknown | None | SQLite (cross-session) |
| **Verification** | LLM Judge (auxiliary) | Unknown | Unknown | CNS ν-event (undefined) | Hybrid (CNS + LLM + Bot) |
| **Completion** | Judge says "done" | Unknown | Unknown | Comparator (undefined) | External verification |
| **Budget** | `max_turns: 20` | Unknown | Unknown | None | Turns + energy |
| **Delegation** | N/A | N/A | N/A | Capability tokens | Goal-specific capabilities |
| **Routing** | Manual selection | Unknown | Unknown | Unified registry | `template_type: Goal` |
| **Subgoals** | Mid-loop additions | Unknown | Unknown | None | First-class table |
| **Visibility** | Session-private | Unknown | Unknown | OCAP (general) | Goal-specific gating |
| **Escalation** | User notification | Unknown | Unknown | Algedonic alert | Curator → human |
| **Multi-Agent** | Single-session | Single-session | Single-session | ACP-enabled | Goal delegation via ACP |
| **Security** | Trust session DB | Unknown | Unknown | OCAP + SQLCipher | HMAC + encryption + OCAP |

---

## 4.2 Agent Taxonomy Mapping

### Bot vs. Replicant Goal Handling

| Aspect | Bot | Replicant |
|--------|-----|-----------|
| **Goal Origin** | Assigned by Curator/Replicant | Assigned by human operator |
| **Goal Visibility** | Public/Shared | Episodic=Private, Semantic=Public |
| **Verification** | Self-verification (CNS spans) | External verification (Curator) |
| **Delegation** | Can receive delegated goals | Can delegate to bots |
| **Budget** | Energy budget (system-managed) | Turn budget (user-configurable) |
| **Escalation** | CNS algedonic alert | Curator notification |

**Example Flow:**
```
Human → assigns goal → Replicant (Curator)
Curator → delegates goal → Bot (memory-bot)
Bot → executes → CNS spans emitted
CNS → detects variety deficit → algedonic alert → Curator
Curator → intervenes → goal adjusted
```

---

## 4.3 OCAP Security Model Integration

### Capability Token Design

```rust
/// Goal-specific capability token with attenuation
pub struct GoalCapability {
    pub id: CapabilityId,
    pub goal_id: GoalId,
    pub owner_webid: WebID,
    pub holder_webid: WebID,
    pub allowed_actions: Vec<GoalAction>,
    pub attenuation_level: u8,
    pub max_attenuation: u8,
    pub expiration: UnixTimestamp,
    pub hmac_signature: HmacSha256,
}

/// Actions specific to goal execution
pub enum GoalAction {
    ToolCall { mcp_server: String, tool_name: String },
    ReadFile { path_pattern: GlobPattern },
    WriteFile { path_pattern: GlobPattern },
    ExecuteCommand { allowed_commands: Vec<String> },
    DelegateGoal { max_attenuation: u8 },
}

impl GoalCapability {
    /// Attenuate capability on delegation
    pub fn delegate(&self) -> Result<Self, DelegationError> {
        if self.attenuation_level >= self.max_attenuation {
            return Err(DelegationError::MaxAttenuationReached);
        }
        
        Ok(Self {
            attenuation_level: self.attenuation_level + 1,
            allowed_actions: self.attenuate_actions(),  // Remove write permissions
            expiration: self.expiration / 2,  // Halve remaining time
            ..self.clone()
        })
    }
    
    fn attenuate_actions(&self) -> Vec<GoalAction> {
        // Remove write permissions, keep read-only
        self.allowed_actions
            .iter()
            .filter(|action| matches!(action, GoalAction::ReadFile { .. }))
            .cloned()
            .collect()
    }
}
```

### Capability Table

| Capability | Bot | Replicant | Curator | Human |
|------------|-----|-----------|---------|-------|
| `goal:create` | ❌ | ✅ | ✅ | ✅ |
| `goal:delegate` | ❌ | ✅ | ✅ | ✅ |
| `goal:pause` | ❌ | ✅ | ✅ | ✅ |
| `goal:resume` | ❌ | ✅ | ✅ | ✅ |
| `goal:complete` | ✅ (self-report) | ✅ (verify) | ✅ (verify) | ✅ (verify) |
| `goal:cancel` | ❌ | ✅ | ✅ | ✅ |
| `goal:verify` | ❌ | ✅ | ✅ | ✅ |
| `goal:read` | ✅ (own goals) | ✅ (all) | ✅ (all) | ✅ (all) |

---

## 4.4 Unified Registry Integration

### Template Type Discriminator

**Current Registry Design** (`docs/architecture/registry-templating-prompt-v2.md`):
```yaml
template_type: Prompt | Process | Cognition
```

**Proposed Extension:**
```yaml
template_type: Prompt | Process | Cognition | Goal
```

### Goal Template Example

```yaml
# registry/templates/goal_build_app.j2
[inference]
template_type: Goal
lexicon_terms: [build, compile, test, verify, deliver]
contract:
  input:
    spec_path: string
    verification_commands: array
  output:
    build_status: string
    test_results: object
    artifacts: array
  verification:
    - type: command
      command: "npm test"
      expected_exit_code: 0
    - type: command
      command: "npm run build"
      expected_exit_code: 0
    - type: state
      check: "git status --porcelain"
      expected_pattern: "^\\?\\s+.*$"  # Only untracked files
    - type: semantic
      evaluator: llm_judge
      criteria: "README accurately describes the application"

---
Build the application defined in {{ spec_path }}.

**Done means:**
{% for check in verification %}
- {% if check.type == "command" %}
  `{{ check.command }}` exits with code {{ check.expected_exit_code }}
{% elif check.type == "state" %}
  `{{ check.check }}` matches pattern `{{ check.expected_pattern }}`
{% elif check.type == "semantic" %}
  {{ check.criteria }}
{% endif %}
{% endfor %}

**Constraints:**
- Do not modify existing files unless specified in spec
- Create tests for all new functionality
- Document all public APIs
```

### Dispatch Manifest

```yaml
# registry/manifests/goal_dispatch.yaml
manifest:
  name: goal-dispatch
  description: Route goal to appropriate agent pod

steps:
  - ordinal: 1
    action: select
    description: "Render selector template with goal text + registry index"
    template_ref: registry/templates/goal_selector.j2
    model_tier: fast_local
    output_schema:
      selected_template_id: string
      rationale: string
      confidence: float

  - ordinal: 2
    action: populate
    description: "Bind goal parameters into selected template"
    template_ref: "{{ selected_template_id }}"
    output_schema:
      rendered_goal: string
      completion_criteria: array

  - ordinal: 3
    action: execute
    description: "Spawn agent pod with goal capability token"
    target: agent_pod
    mcp: hkask-mcp-agents
    output_schema:
      pod_id: string
      capability_token: string
      expiration: timestamp
```

---

## 4.5 Constraint Evaluation (P1-P7, C1-C7)

### Principles (P1-P7)

| Principle | Evaluation | Status |
|-----------|------------|--------|
| **P1:** No trait without two consumers | `GoalVerifier` trait: CNS adapter + LLM adapter + Bot adapter | ✅ (3 consumers) |
| **P2:** No generic without two instantiations | `GoalCapability<Owner, Holder>`: Bot→Bot, Replicant→Bot | ✅ (2 instantiations) |
| **P3:** No module directory without encapsulation | `hkask-goals/src/` encapsulates goal logic | ✅ |
| **P4:** No builder without fallibility | `GoalBuilder::build() -> Result<Goal, GoalError>` | ✅ |
| **P5:** No feature flag without activator | `--features goals` requires `kask goal` command | ✅ |
| **P6:** Delete stubs, don't publish them | No stubs in initial commit | ✅ |
| **P7:** Prefer deletion over deprecation | N/A (new feature) | ✅ |

### Constraints (C1-C7)

| Constraint | Evaluation | Status |
|------------|------------|--------|
| **C1:** A type must be worn before tailored | `GoalId` used in `GoalState` before refinement | ✅ |
| **C2:** Distinguish dead from unwired | `GoalExecutor` trait defined, adapters wired in Phase 2 | ✅ |
| **C3:** Unwired code has shelf life | 2-week shelf life per phase | ✅ |
| **C4:** Repetition is missing primitive | Goal subgoals extracted to separate table | ✅ |
| **C5:** Every error variant unique recovery path | `GoalError` variants: `BudgetExhausted`, `VerificationFailed`, etc. | ✅ |
| **C6:** A stub is debt receipt | No stubs (Phase 0 types only) | ✅ |
| **C7:** When implementations diverge, one yields | Single `GoalVerifier` trait, multiple adapters | ✅ |

---

## 4.6 Hexagonal Architecture Mapping

### Ports (Traits)

```rust
/// Inbound port: Goal creation/modification
pub trait GoalRepository {
    fn create(&self, spec: GoalSpec) -> Result<GoalId>;
    fn get(&self, id: GoalId) -> Result<Goal>;
    fn update(&self, id: GoalId, state: GoalState) -> Result<()>;
    fn delete(&self, id: GoalId) -> Result<()>;
    fn list_by_owner(&self, owner: WebID) -> Result<Vec<Goal>>;
}

/// Inbound port: Goal execution
pub trait GoalExecutor {
    fn execute(&self, goal: Goal, capability: GoalCapability) -> Result<GoalOutcome>;
}

/// Inbound port: Goal verification
pub trait GoalVerifier {
    fn verify(&self, goal: Goal, outcome: GoalOutcome) -> Result<Verdict>;
}

/// Outbound port: CNS instrumentation
pub trait CNSSpanEmitter {
    fn emit(&self, span: Span, event: NuEvent) -> Result<()>;
}

/// Outbound port: Capability enforcement
pub trait CapabilityChecker {
    fn check(&self, capability: GoalCapability, action: GoalAction) -> Result<()>;
}

/// Outbound port: Persistence
pub trait GoalStorage {
    fn store(&self, goal: Goal) -> Result<()>;
    fn load(&self, id: GoalId) -> Result<Goal>;
}
```

### Adapters

| Port | Adapter | Crate |
|------|---------|-------|
| `GoalRepository` | `SqliteGoalRepository` | `hkask-storage` |
| `GoalExecutor` | `AgentPodExecutor` | `hkask-agents` |
| `GoalVerifier` | `CNSComparatorVerifier` | `hkask-cns` |
| `GoalVerifier` | `LLMJudgeVerifier` | `hkask-mcp-inference` |
| `GoalVerifier` | `CommandVerifier` | `hkask-mcp` |
| `CNSSpanEmitter` | `CnsSpanEmitter` | `hkask-cns` |
| `CapabilityChecker` | `OcapCapabilityChecker` | `hkask-types` |
| `GoalStorage` | `SqlCipherGoalStorage` | `hkask-storage` |

---

## 4.7 CNS Integration

### New Span Namespace

```rust
pub enum Span {
    // Existing...
    Tool(String),
    Prompt(String),
    AgentPod(String),
    Connector(String),
    
    // New: Goal primitive
    Goal(String),
}

impl Span {
    pub fn goal(event: &str) -> Self {
        Span::Goal(event.to_string())
    }
}

// Specific goal spans:
// - cns.goal.create
// - cns.goal.verify
// - cns.goal.complete
// - cns.goal.delegate
// - cns.goal.block
// - cns.goal.variety_deficit
```

### Variety Counter Integration

```rust
impl GoalManager {
    pub fn check_variety(&self, goal: &Goal) -> Result<(), AlgedonicAlert> {
        let environmental_states = goal.estimate_complexity();
        let internal_states = self.agent.capability_count();
        
        let deficit = environmental_states.saturating_sub(internal_states);
        
        if deficit > 100 {
            Err(AlgedonicAlert::VarietyDeficit {
                goal_id: goal.id,
                deficit,
                environmental_states,
                internal_states,
            })
        } else {
            Ok(())
        }
    }
}
```

---

## 4.8 Line Budget Analysis

| Crate | Current LOC | Proposed Addition | New Total |
|-------|-------------|-------------------|-----------|
| `hkask-types` | ~500 | +200 (GoalId, GoalCapability) | ~700 |
| `hkask-storage` | ~800 | +300 (GoalRepository adapter) | ~1,100 |
| `hkask-cns` | ~600 | +200 (Goal spans, variety) | ~800 |
| `hkask-agents` | ~1,210 | +300 (GoalExecutor) | ~1,510 |
| `hkask-templates` | ~500 | +100 (Goal template routing) | ~600 |
| `hkask-cli` | ~400 | +150 (goal commands) | ~550 |
| **New: `hkask-goals`** | 0 | ~500 (core logic) | ~500 |
| **Total** | ~4,010 | **+1,750** | **~5,760** |

**Budget Remaining:** 30,000 - 5,760 = **24,240 LOC** ✅

---

## 4.9 Recommendation: ADAPT

**Decision:** hKask should **adapt** the `/goal` primitive, not adopt or reject.

**Rationale:**

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **Adopt** (copy Hermes) | Fast implementation, proven design | Lacks security, multi-agent, CNS integration | ❌ |
| **Reject** | No implementation cost | Misses interoperability, user expectations | ❌ |
| **Adapt** (hKask-native) | OCAP security, CNS monitoring, multi-agent, registry routing | Higher implementation cost (~1,750 LOC) | ✅ |

**hKask-Native Distinctions:**
1. **OCAP-Gated Delegation** — Capability tokens with attenuation
2. **CNS Monitoring** — Variety counters, algedonic alerts
3. **Hybrid Verification** — CNS + LLM + Bot verifiers
4. **Registry Routing** — `template_type: Goal` discriminator
5. **Cross-Session Persistence** — SQLite with HMAC integrity
6. **Multi-Agent Support** — ACP message routing, WebID ownership

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*  
*Task 4 Complete: Recommendation is ADAPT — hKask-native goal primitive with OCAP security, CNS monitoring, and registry routing.*