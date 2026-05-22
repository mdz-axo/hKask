# /goal Primitive Research & hKask Architecture Analysis

**Date:** 2026-05-22  
**Author:** Kilo (AI Engineering Assistant)  
**Status:** Deep Research Complete — Architecture Planning Required  
**Related:** `docs/architecture/hKask-architecture-master.md`, `AGENTS.md`

---

## Executive Summary

This report analyzes the emerging `/goal` primitive across three major AI agent systems (Claude Code, OpenAI Codex CLI, Hermes Agent) and evaluates whether hKask should formalize a shared understanding of "goal" in its architecture.

**Key Finding:** The `/goal` primitive represents a **fundamental shift from prompting to assigning** — agents transition from one-shot commands to persistent objectives with defined completion criteria. This aligns closely with hKask's existing architecture (CNS ν-events, agent pods, unified registry), but requires explicit formalization.

**Recommendation:** **YES** — hKask should formalize the goal primitive, but with distinct architectural integration:
1. Goals as **first-class entities** in the data model (not just prompt metadata)
2. **Verification-based completion** (CNS variety counters, algedonic alerts)
3. **OCAP-gated goal delegation** (capability attenuation on goal handoff)
4. **Registry-driven goal routing** (unified registry with `template_type: Goal`)

---

## Part 1: What /goal Actually Is

### 1.1 The Primitive Shift

| Paradigm | User Role | Agent Role | Completion |
|----------|-----------|------------|------------|
| **Prompting** | Steers every turn | Responds to last message | Implicit (user decides) |
| **/goal** | Defines "done" once | Works autonomously toward target | Explicit (verifier decides) |

**Example:**
```
/goal Build the app described in SPEC.md. Done means tests pass, build passes, 
README is accurate, and git status only shows relevant project files.
```

### 1.2 Hermes Agent Implementation (NousResearch)

**Source:** `hermes_cli/goals.py` (PR #18262, May 2026)

**Core Components:**

```python
@dataclass
class GoalState:
    goal: str                        # Free-form objective
    status: str                      # active | paused | done | cleared
    turns_used: int                  # Budget tracking
    max_turns: int                   # Default: 20
    subgoals: List[str]              # Mid-loop criteria additions
    consecutive_parse_failures: int  # Judge model health
```

**Key Mechanisms:**

1. **Judge Model** — Auxiliary LLM evaluates after each turn:
   - Input: `goal` + `last_response`
   - Output: `{"done": true/false, "reason": "..."}`
   - Fail-open: Judge errors → continue (budget is backstop)

2. **Continuation Loop:**
   ```
   User submits /goal → Agent executes turn → Judge evaluates → 
   If not done: append continuation prompt → repeat
   ```

3. **Session Persistence:** Goal state stored in `SessionDB.state_meta` keyed by `goal:<session_id>`

4. **Commands:**
   - `/goal <text>` — Set standing goal
   - `/goal status` — Show current state
   - `/goal pause` / `/goal resume` — Control loop
   - `/goal clear` — Drop goal
   - `/subgoal <text>` — Add criteria mid-loop

**Line Count:** ~650 LOC (`goals.py` + CLI integration + gateway hooks)

**Bugs Identified:**
- Issue #27585: Judge API errors cause spam loops when agent reaches terminal response
- Fix: Auto-pause on judge error + terminal phrase detection

---

## Part 2: Academic Research on Goal Primitives

### 2.1 BDI (Belief-Desire-Intention) Architecture

**Historical Foundation:**
- Bratman (1987): Philosophical model of practical reasoning
- Rao & Georgeff (1991): Modal logic formalization
- Modern implementations: Jason, AgentSpeak, GOAL, Can

**Goal Lifecycle (BDI):**
```
Pending → Active → [Suspended | Aborted | Successful]
```

**Key Distinctions:**

| Type | Description | Example |
|------|-------------|---------|
| **Achievement Goal** | State to bring about | `on(A, B)` |
| **Maintenance Goal** | State to preserve | `¬exceed_gradient(5%)` |
| **Declarative Goal** | Goal "to be" (belief-independent) | `¬believes(goal_achieved)` |
| **Procedural Goal** | Goal "to do" (plan-triggered) | `respond_to_event(e)` |

**Recent Advances (2020-2026):**

1. **Temporally Extended Goals** (IJCAI 2024):
   - Mix reachability + invariant properties
   - Example: `travel_to(A) ∧ ¬exceed_gradient(5%)`
   - Allows neuro-symbolic architectures (human plans + RL policies)

2. **Goal-Conditioned RL** (AAAI 2026):
   - First-order representation languages
   - Hindsight Experience Replay (HER) relabels failures as successes
   - Goals as full states vs. subsets vs. lifted subgoals

3. **Goal Representation Learning** (2025-2026):
   - **Action-sufficient representations:** Retain information needed for optimal action selection
   - **Dual goal representations:** Characterize state by temporal distances from all other states
   - **Latent-goal architectures:** Task-agnostic state representation + goal channel with information bottleneck

### 2.2 Formal Specification Languages

**Prism** (arXiv 2025):
- Compositional metalanguage for agent behavior
- Grammar-like specifications vs. imperative control flow
- Separation: natural language understanding (LLM) + formal control (Prism policy)

**AgentSPEX** (arXiv 2026):
- YAML syntax for explicit workflow control
- Typed steps, branching, loops, parallel execution
- Goal field in workflow specification:
  ```yaml
  workflow:
    name: deep_research
    goal: "Find and summarize 5 papers on X"
    steps: [...]
  ```

**FALAA** (Springer 2026):
- Framework for Abstraction of Language Agent Architectures
- UML + OCL formal specifications
- Components: Planner, Executor, Evaluator, Reflector, Memory, Environment

### 2.3 Goal Selection & Program Induction

**Program-Based Goal Selection** (OpenReview 2024):
- Goals as `(goal_program, reward_function)` pairs
- Goal programs sampled from generative grammar `G`
- Reduces planning cost via inductive biases

**Key Insight:** RL has no formal goal representation — assumes Markov reward functions, which have theoretical limitations.

---

## Part 3: hKask Current Architecture Analysis

### 3.1 Existing Goal-Related Concepts

**CNS ν-Event Lifecycle:**
```rust
pub struct NuEvent {
    pub span: Span,              // cns.tool.*, cns.prompt.*, cns.agent_pod.*
    pub phase: Phase,            // Observe → Compare → Act
    pub outcome: Value,
    pub confidence: f64,
    pub algedonic_alert: bool,
}
```

**Lifecycle:**
1. **Observation** — Sensor detects environmental state
2. **Regulation** — Comparator checks against **goal**
3. **Outcome** — Effector acts (or algedonic alert escalates)

**Variety Counter:**
```rust
pub struct VarietyCounter {
    environmental_states: usize,
    internal_states: usize,
    deficit_threshold: u64,  // Default: 100
}

pub fn check(&self) -> Option<AlgedonicAlert> {
    let deficit = self.environmental_states
        .saturating_sub(self.internal_states);
    if deficit > self.deficit_threshold as usize {
        Some(AlgedonicAlert::VarietyDeficit(deficit))
    } else { None }
}
```

**Agent Pod Lifecycle:**
```
Populated → Registered → Activated → [Delegated] → Deactivated
```

**Capability Token Attenuation:**
- Max 7 levels of delegation
- HMAC-SHA256 signatures
- Expiration-based revocation

### 3.2 Gaps Identified

| Concept | Hermes/Codex | hKask Current | Gap |
|---------|--------------|---------------|-----|
| **Goal Persistence** | SessionDB storage | None | Goals not first-class entities |
| **Completion Verification** | Judge LLM | CNS ν-event comparator | CNS comparator undefined |
| **Budget Tracking** | `max_turns: 20` | None | No turn/energy budget per goal |
| **Goal Delegation** | N/A | Capability tokens | No goal-specific attenuation |
| **Goal Routing** | Manual selection | Unified registry | No `template_type: Goal` |

---

## Part 4: Proposed hKask Goal Primitive Design

### 4.1 Goal as First-Class Entity

**Database Schema:**
```sql
CREATE TABLE goals (
    id              UUID PRIMARY KEY,
    session_id      UUID NOT NULL,
    owner_webid     TEXT NOT NULL,
    goal_text       TEXT NOT NULL,
    template_ref    TEXT,                    -- Registry template ID
    status          TEXT NOT NULL,           -- active | paused | done | cleared | blocked
    turns_used      INTEGER DEFAULT 0,
    energy_budget   INTEGER,                 -- Optional: energy units
    max_turns       INTEGER DEFAULT 20,
    created_at      INTEGER NOT NULL,
    last_turn_at    INTEGER,
    completed_at    INTEGER,
    blocked_reason  TEXT,
    paused_reason   TEXT,
    visibility      TEXT NOT NULL DEFAULT 'private',
    
    INDEX idx_session (session_id),
    INDEX idx_owner (owner_webid),
    INDEX idx_status (status)
);

CREATE TABLE goal_subgoals (
    goal_id     UUID NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    ordinal     INTEGER NOT NULL,
    text        TEXT NOT NULL,
    satisfied   BOOLEAN DEFAULT FALSE,
    
    PRIMARY KEY (goal_id, ordinal)
);

CREATE TABLE goal_verifications (
    id              UUID PRIMARY KEY,
    goal_id         UUID NOT NULL REFERENCES goals(id),
    nu_event_id     UUID REFERENCES nu_events(id),
    verdict         TEXT NOT NULL,         -- done | continue | blocked
    reason          TEXT,
    confidence      REAL,
    verified_at     INTEGER NOT NULL,
    
    INDEX idx_goal (goal_id)
);
```

### 4.2 Goal Lifecycle State Machine

```
┌─────────────┐
│  Created    │ ← /goal create --text "..." --template registry:templates:goal:build
└──────┬──────┘
       │ register()
       ↓
┌─────────────┐
│  Active     │ ← Default state, agent working
└──────┬──────┘
       │
       ├─────────────┬─────────────┬──────────────┐
       │ pause()     │ delegate()  │ complete()   │ block()
       ↓             ↓             ↓              ↓
┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│   Paused    │ │ Delegated   │ │   Done      │ │   Blocked   │
│ (user-pause)│ │ (OCAP-gated)│ │(verifier)   │ │(need input) │
└──────┬──────┘ └──────┬──────┘ └─────────────┘ └──────┬──────┘
       │               │                                │
       │ resume()      │ return()                       │ resolve()
       └───────────────┴────────────────────────────────┘
```

### 4.3 CNS Integration

**New Spans:**
- `cns.goal.create` — Goal instantiation
- `cns.goal.verify` — Post-turn verification
- `cns.goal.complete` — Completion detection
- `cns.goal.delegate` — Capability-attenuated handoff
- `cns.goal.block` — Escalation to Curator/human

**Variety Counter Integration:**
```rust
impl GoalManager {
    pub fn check_variety(&self, goal: &Goal) -> Result<(), AlgedonicAlert> {
        let environmental_states = goal.estimate_complexity();
        let internal_states = self.agent.capability_count();
        
        if environmental_states > internal_states + 100 {
            Err(AlgedonicAlert::VarietyDeficit(
                environmental_states - internal_states
            ))
        } else {
            Ok(())
        }
    }
}
```

**Verification Contract:**
```rust
pub trait GoalVerifier {
    /// Evaluate whether goal is satisfied based on last response + environment state
    fn verify(
        &self,
        goal: &Goal,
        last_response: &str,
        environment_state: &EnvironmentSnapshot,
    ) -> GoalVerdict {
        // 1. Check explicit completion phrases
        // 2. Run CNS comparator (ν-event outcome vs. goal criteria)
        // 3. Check variety counter (no deficit)
        // 4. If all pass: GoalVerdict::Done, else: GoalVerdict::Continue
    }
}

pub enum GoalVerdict {
    Done { reason: String, confidence: f64 },
    Continue { reason: String },
    Blocked { reason: String, needs_human: bool },
}
```

### 4.4 Registry Integration

**Template Type Discriminator:**
```yaml
# registry/templates/goal_build_app.j2
[inference]
template_type: Goal  # NEW: Goal as first-class template type
lexicon_terms: [build, compile, test, verify]
contract:
  input: {spec_path: string, verification_commands: array}
  output: {build_status: string, test_results: object}
  verification:
    - command: "npm test"
      expected_exit_code: 0
    - command: "npm run build"
      expected_exit_code: 0
    - command: "git status --porcelain"
      expected_pattern: "^\\?\\s+.*$"  # Only untracked files

---
Build the application defined in {{ spec_path }}.
Done means:
{% for check in verification %}
- {{ check.command }} exits with code {{ check.expected_exit_code }}
{% endfor %}
```

**Dispatch Manifest:**
```yaml
# registry/manifests/goal_dispatch.yaml
manifest:
  name: goal-dispatch
  description: Route goal to appropriate agent pod

steps:
  - ordinal: 1
    action: select
    template_ref: registry/templates/goal_selector.j2
    model_tier: fast_local
    output_schema:
      selected_template_id: string
      confidence: float

  - ordinal: 2
    action: populate
    template_ref: "{{ selected_template_id }}"
    output_schema:
      rendered_goal: string

  - ordinal: 3
    action: execute
    target: agent_pod
    mcp: hkask-mcp-agents
    output_schema:
      pod_id: string
      capability_token: string
```

### 4.5 OCAP Goal Delegation

**Capability Token Attenuation:**
```rust
pub struct GoalCapability {
    pub goal_id: GoalId,
    pub attenuation_level: u8,  // Increases per delegation
    pub max_attenuation: u8,    // Default: 7
    pub allowed_actions: Vec<CapabilityAction>,
    pub expiration: UnixTimestamp,
}

impl GoalCapability {
    pub fn delegate(&self) -> Result<Self, DelegationError> {
        if self.attenuation_level >= self.max_attenuation {
            return Err(DelegationError::MaxAttenuationReached);
        }
        
        Ok(Self {
            goal_id: self.goal_id,
            attenuation_level: self.attenuation_level + 1,
            max_attenuation: self.max_attenuation,
            allowed_actions: self.attenuate_actions(),  // Remove write permissions
            expiration: self.expiration / 2,  // Halve remaining time
        })
    }
}
```

**Goal Delegation Flow:**
```
Curator (Replicant) → delegates goal → Bot (memory-bot)
  ├─ Capability: tool:memory:recall (read-only)
  ├─ Attenuation: level 1 → level 2
  └─ Expiration: 1 hour → 30 minutes
```

---

## Part 5: Implementation Plan

### Phase 0: Foundation (Week 1)

**Tasks:**
1. **Database Migration** — Add `goals`, `goal_subgoals`, `goal_verifications` tables
2. **Rust Types** — `GoalId`, `GoalState`, `GoalVerdict`, `GoalCapability`
3. **CNS Spans** — Define `cns.goal.*` namespace in `hkask-cns`

**LOC Budget:** ~400 LOC (types + schema)

### Phase 1: Goal Manager Core (Week 2)

**Tasks:**
1. **GoalManager Struct** — Lifecycle methods (`create`, `pause`, `resume`, `complete`, `block`)
2. **Verifier Trait** — `GoalVerifier` with CNS integration
3. **Session Persistence** — SQLite adapter for goal state

**LOC Budget:** ~600 LOC

### Phase 2: Registry Integration (Week 3)

**Tasks:**
1. **Template Type** — Add `Goal` to `template_type` discriminator
2. **Selector Template** — `goal_selector.j2` for routing
3. **Dispatch Manifest** — `goal_dispatch.yaml` workflow

**LOC Budget:** ~200 LOC (Rust) + unlimited YAML/Jinja2

### Phase 3: OCAP Delegation (Week 4)

**Tasks:**
1. **GoalCapability** — Attenuation logic
2. **Delegation API** — `delegate_goal()` method
3. **CLI Commands** — `kask goal delegate <goal-id> --to <agent-webid>`

**LOC Budget:** ~300 LOC

### Phase 4: CNS Variety Integration (Week 5)

**Tasks:**
1. **Variety Counter** — `check_variety()` integration
2. **Algedonic Alert** — Escalation on variety deficit >100
3. **Curator Handoff** — Automatic escalation path

**LOC Budget:** ~200 LOC

### Phase 5: Testing & Verification (Week 6)

**Tasks:**
1. **Unit Tests** — Goal lifecycle, verifier, delegation
2. **Integration Tests** — Full goal flow (create → execute → verify → complete)
3. **CNS Monitoring** — Verify span emission

**Test LOC:** Excluded from budget (in `hkask-testing`)

**Total LOC:** ~1,700 LOC (well within 30,000 line budget)

---

## Part 6: Comparison Matrix

| Feature | Hermes Agent | hKask (Proposed) |
|---------|--------------|------------------|
| **Goal Storage** | SessionDB (per-session) | SQLite (cross-session, multi-agent) |
| **Verification** | Judge LLM (auxiliary model) | CNS ν-event comparator + variety counter |
| **Completion** | Judge says "done" | Verifier + environment state check |
| **Budget** | Turn count (20 default) | Turns + energy budget |
| **Delegation** | N/A | OCAP capability tokens with attenuation |
| **Routing** | Manual selection | Unified registry with `template_type: Goal` |
| **Subgoals** | Mid-loop additions | First-class `goal_subgoals` table |
| **Visibility** | Session-private | OCAP-enforced (private/public/shared) |
| **Escalation** | User notification | Algedonic alert → Curator/human |

---

## Part 7: Open Questions

### Q1: Goal vs. Manifest — Same Thing?

**Analysis:**
- **Manifest:** Process definition (`FlowDef/do`)
- **Goal:** Objective with completion criteria (`KnowAct/achieve`)

**Decision:** **Distinct but related** — a goal references a manifest for execution, but adds verification layer.

```yaml
# Goal references manifest
goal:
  id: goal_123
  manifest_ref: registry/manifests/dispatch.yaml
  verification:
    - command: "npm test"
      expected: exit_code_0
```

### Q2: Who Verifies Goals?

**Options:**
1. **Dedicated Verifier Bot** — Specialized agent pod
2. **CNS Built-in** — Rust comparator logic
3. **LLM Judge** — Hermes-style auxiliary model

**Decision:** **Hybrid** — CNS built-in for simple checks (exit codes, file existence), LLM judge for semantic verification (README accuracy), verifier bot for complex workflows.

### Q3: Goal Budget — Turns or Energy?

**Analysis:**
- **Turns:** Simple, but model-dependent (fast vs. slow models)
- **Energy:** Abstract, model-agnostic, aligns with CNS energy tracking

**Decision:** **Both** — `max_turns` for immediate control, `energy_budget` for system-wide resource management.

### Q4: Can Goals Have Subgoals?

**Decision:** **Yes** — `goal_subgoals` table with `satisfied` boolean. Subgoals can be:
- User-added mid-loop (`/subgoal` command)
- Agent-discovered during execution
- Manifest-defined prerequisites

### Q5: Goal Visibility — Who Can See/Modify?

**Decision:** **OCAP-enforced** —
- Owner: full access
- Delegated agents: read-only (unless capability grants write)
- Public goals: visible to all agents, writable only by owner

### Q6: Goal Completion — Self-Report or External Verification?

**Decision:** **External verification** — Never trust agent self-report. Verifier must:
1. Check CNS ν-event outcomes
2. Run verification commands
3. Inspect filesystem/git state
4. Confirm variety counter healthy

---

## Part 8: Risks & Mitigations

### Risk 1: Goal Spam Loops

**Scenario:** Verifier errors cause infinite continuation (Hermes Issue #27585)

**Mitigation:**
- Max consecutive parse failures → auto-pause
- Terminal phrase detection → safe stop
- Turn budget as hard backstop

### Risk 2: Capability Attenuation Too Aggressive

**Scenario:** Delegated agent can't complete goal due to permission loss

**Mitigation:**
- Configurable `max_attenuation` (default: 7)
- Delegation audit trail (CNS spans)
- Curator override path

### Risk 3: Variety Counter False Positives

**Scenario:** Algedonic alert triggered on complex but achievable goals

**Mitigation:**
- Adjustable `deficit_threshold` (default: 100)
- Curator review before human escalation
- Goal complexity estimation refinement

### Risk 4: Registry Routing Failures

**Scenario:** Goal selector routes to wrong template

**Mitigation:**
- Confidence threshold (<0.7 → ask user)
- CNS outcome tracking (learn from failures)
- Manual override (`/goal reroute --template <id>`)

---

## Part 9: Completion Criteria

**Goal primitive is complete when:**

1. ✅ Database schema deployed (`goals`, `goal_subgoals`, `goal_verifications`)
2. ✅ `GoalManager` implements full lifecycle (create → pause → resume → complete → block)
3. ✅ CNS spans emitted (`cns.goal.*` namespace)
4. ✅ Registry routes goals via `template_type: Goal` discriminator
5. ✅ OCAP delegation with capability attenuation
6. ✅ Variety counter integration (algedonic alerts on deficit >100)
7. ✅ CLI commands (`kask goal create/pause/resume/complete/block/delegate/status`)
8. ✅ Verifier trait with hybrid verification (CNS + LLM + bot)
9. ✅ Tests passing (unit + integration in `hkask-testing`)
10. ✅ LOC budget ≤2,000 (target: ~1,700)

---

## Part 10: Academic Alignment

### BDI Correspondence

| hKask Concept | BDI Equivalent | Notes |
|---------------|----------------|-------|
| `GoalState` | Desire/Goal | hKask adds verification layer |
| `GoalCapability` | Intention | OCAP attenuation unique to hKask |
| `GoalVerifier` | Plan Selection | hKask uses CNS comparator |
| `VarietyCounter` | Context Condition | Cybernetic addition (Ashby/Von Foerster) |
| `AlgedonicAlert` | Goal Failure | Escalation to Curator/human |

### Novel Contributions

1. **OCAP-Gated Goal Delegation** — Capability attenuation on goal handoff (not in BDI)
2. **CNS Variety Monitoring** — Cybernetic feedback on goal complexity vs. agent capability
3. **Registry-Driven Routing** — Unified registry with `template_type: Goal` (vs. plan library)
4. **Hybrid Verification** — CNS comparator + LLM judge + bot verifier (vs. single judge)

---

## Recommendation

**Formalize the goal primitive in hKask with the following architectural decisions:**

1. **Goals as First-Class Entities** — Database tables, Rust types, CNS spans
2. **Verification-Based Completion** — Never trust self-report; always verify
3. **OCAP Delegation** — Capability attenuation on goal handoff
4. **Registry Routing** — `template_type: Goal` in unified registry
5. **CNS Integration** — Variety counters, algedonic alerts, span emission

**Rationale:**
- Aligns with hKask's cybernetic roots (Beer, Ashby, Von Foerster)
- Extends BDI model with OCAP security and CNS monitoring
- Enables multi-agent goal delegation (unlike Hermes single-session model)
- Maintains ≤30,000 LOC budget (~1,700 LOC addition)

**Next Step:** Begin Phase 0 implementation (database schema + Rust types).

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*  
*Goal primitive: from prompting to assigning.*  
*Rust is the loom. YAML/Jinja2 is the thread. OCAP is the gate. CNS is the monitor.*
