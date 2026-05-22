# hKask Goal Primitive Implementation Plan (Minimalist)

**Date:** 2026-05-22  
**Principle:** Minimalist design — modules, not crates  
**Budget:** ~1,200 LOC (reduced from 2,050)

---

## Architecture: Modules, Not Crates

**Decision:** Goal primitive lives as modules in existing crates:

| Module | Location | Purpose |
|--------|----------|---------|
| `goal.rs` | `hkask-types/src/goal.rs` | Core types (`GoalId`, `Goal`, `GoalSpec`, `GoalState`) |
| `goal_capability.rs` | `hkask-types/src/goal_capability.rs` | OCAP types (`GoalCapability`, `GoalAction`) |
| `goal_repository.rs` | `hkask-storage/src/goal_repository.rs` | SQLite persistence |
| `goal_verifier.rs` | `hkask-cns/src/goal_verifier.rs` | CNS verification |
| `goal_judge.rs` | `hkask-mcp-inference/src/goal_judge.rs` | LLM judge |
| `goal_executor.rs` | `hkask-agents/src/goal_executor.rs` | AgentPod execution |
| `goal_variety.rs` | `hkask-cns/src/goal_variety.rs` | Variety counter |

**Rationale:**
- No new crate overhead (no `Cargo.toml`, no dependency management)
- Ports defined where consumed (hexagonal architecture)
- Types shared via `hkask-types` (single source of truth)
- LOC counted toward existing crates (no separate budget tracking)

---

## Phase 1: Core Types (Week 1)

### Step 1.1: Goal Types (`hkask-types/src/goal.rs`)

```rust
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Unique identifier for a goal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GoalId(pub Uuid);

impl GoalId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// WordAct commissive acts — goal commitment semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalCommitment {
    Pledge,      // Weak: "I intend to..."
    Commit,      // Medium: "I will..."
    Undertake,   // Strong: "I accept responsibility for..."
    Promise,     // Strongest: "I guarantee..."
}

/// Goal lifecycle state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalState {
    Active,
    Paused { reason: String },
    Done { reason: String },
    Cleared,
    Blocked { reason: String },
}

/// FlowDef decomposition patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "flow_type", rename_all = "snake_case")]
pub enum GoalFlow {
    Sequence { steps: Vec<SubgoalSpec> },
    Parallel { branches: Vec<SubgoalSpec> },
    Choice { branches: Vec<(String, SubgoalSpec)> },
}

/// Subgoal specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgoalSpec {
    pub description: String,
    pub effort_estimate: Option<String>,
}

/// Completion criterion types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompletionCriterion {
    Command {
        command: String,
        expected_exit_code: i32,
    },
    State {
        check: String,
        expected_pattern: String,
    },
    Semantic {
        evaluator: String,
        criteria: String,
    },
}

/// Goal specification (creation input)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSpec {
    pub owner_webid: String,
    pub session_id: String,
    pub goal_text: String,
    pub template_ref: Option<String>,
    pub commitment_level: GoalCommitment,
    pub flow: Option<GoalFlow>,
    pub completion_criteria: Vec<CompletionCriterion>,
    pub max_turns: Option<u32>,
    pub energy_budget: Option<u64>,
    pub visibility: Visibility,
}

/// Visibility (OCAP-enforced)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Private,
    Public,
    Shared,
}

/// Goal entity (full representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: GoalId,
    pub session_id: String,
    pub owner_webid: String,
    pub goal_text: String,
    pub template_ref: Option<String>,
    pub state: GoalState,
    pub commitment_level: GoalCommitment,
    pub flow: Option<GoalFlow>,
    pub completion_criteria: Vec<CompletionCriterion>,
    pub subgoals: Vec<Subgoal>,
    pub turns_used: u32,
    pub energy_budget: Option<u64>,
    pub energy_used: u64,
    pub max_turns: u32,
    pub created_at: i64,
    pub last_turn_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub visibility: Visibility,
}

/// Subgoal (user-added criteria)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subgoal {
    pub ordinal: u32,
    pub text: String,
    pub satisfied: bool,
}

/// Goal outcome (execution result)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome_type", rename_all = "snake_case")]
pub enum GoalOutcome {
    Success {
        summary: String,
        artifacts: Vec<String>,
    },
    Failure {
        reason: String,
        recoverable: bool,
    },
    Partial {
        summary: String,
        completed_criteria: Vec<usize>,
        failed_criteria: Vec<usize>,
    },
}

/// Verification verdict
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    Done {
        reason: String,
        confidence: f64,
    },
    Continue {
        reason: String,
    },
    Blocked {
        reason: String,
        needs_human: bool,
    },
}
```

**LOC:** ~250

---

### Step 1.2: Goal Capability (`hkask-types/src/goal_capability.rs`)

```rust
use crate::goal::GoalId;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};

type HmacSha256 = Hmac<Sha256>;

/// Capability identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub Uuid);

impl CapabilityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Goal-specific actions (OCAP)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action_type", rename_all = "snake_case")]
pub enum GoalAction {
    ToolCall {
        mcp_server: String,
        tool_name: String,
    },
    ReadFile {
        path_pattern: String,
    },
    WriteFile {
        path_pattern: String,
    },
    ExecuteCommand {
        allowed_commands: Vec<String>,
    },
    DelegateGoal {
        max_attenuation: u8,
    },
}

/// Goal capability token with attenuation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalCapability {
    pub id: CapabilityId,
    pub goal_id: GoalId,
    pub owner_webid: String,
    pub holder_webid: String,
    pub allowed_actions: Vec<GoalAction>,
    pub attenuation_level: u8,
    pub max_attenuation: u8,
    pub expiration: i64,
    pub hmac_signature: Vec<u8>,
}

impl GoalCapability {
    pub fn new(
        goal_id: GoalId,
        owner_webid: String,
        holder_webid: String,
        allowed_actions: Vec<GoalAction>,
        max_attenuation: u8,
        expiration: i64,
        secret_key: &[u8],
    ) -> Self {
        let id = CapabilityId::new();
        let mut mac = HmacSha256::new_from_slice(secret_key).expect("HMAC can take key of any size");
        
        mac.update(id.0.as_bytes());
        mac.update(goal_id.0.as_bytes());
        mac.update(holder_webid.as_bytes());
        mac.update(&expiration.to_be_bytes());
        
        let signature = mac.finalize().into_bytes().to_vec();
        
        Self {
            id,
            goal_id,
            owner_webid,
            holder_webid,
            allowed_actions,
            attenuation_level: 0,
            max_attenuation,
            expiration,
            hmac_signature: signature,
        }
    }
    
    pub fn verify(&self, secret_key: &[u8]) -> Result<(), CapabilityError> {
        let mut mac = HmacSha256::new_from_slice(secret_key)
            .map_err(|_| CapabilityError::InvalidKey)?;
        
        mac.update(self.id.0.as_bytes());
        mac.update(self.goal_id.0.as_bytes());
        mac.update(self.holder_webid.as_bytes());
        mac.update(&self.expiration.to_be_bytes());
        
        mac.verify_slice(&self.hmac_signature)
            .map_err(|_| CapabilityError::InvalidSignature)
    }
    
    pub fn delegate(&self, secret_key: &[u8]) -> Result<Self, DelegationError> {
        if self.attenuation_level >= self.max_attenuation {
            return Err(DelegationError::MaxAttenuationReached);
        }
        
        if self.expiration <= get_current_timestamp() {
            return Err(DelegationError::Expired);
        }
        
        // Attenuate: remove write permissions
        let attenuated_actions = self.allowed_actions
            .iter()
            .filter(|action| {
                matches!(action, GoalAction::ReadFile { .. } | GoalAction::ToolCall { .. })
            })
            .cloned()
            .collect();
        
        let mut child = Self::new(
            self.goal_id,
            self.owner_webid.clone(),
            self.holder_webid.clone(),
            attenuated_actions,
            self.max_attenuation,
            self.expiration / 2,
            secret_key,
        );
        
        child.attenuation_level = self.attenuation_level + 1;
        Ok(child)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityError {
    InvalidKey,
    InvalidSignature,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelegationError {
    MaxAttenuationReached,
    Expired,
}

fn get_current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}
```

**LOC:** ~150

---

## Phase 2: Adapters (Week 2-3)

### Step 2.1: SQLite Repository (`hkask-storage/src/goal_repository.rs`)

**Port trait (defined locally):**
```rust
use hkask_types::goal::{Goal, GoalId, GoalSpec};

pub trait GoalRepository {
    fn create(&self, spec: GoalSpec) -> Result<GoalId>;
    fn get(&self, id: GoalId) -> Result<Goal>;
    fn update(&self, id: GoalId, goal: Goal) -> Result<()>;
    fn delete(&self, id: GoalId) -> Result<()>;
    fn list_by_owner(&self, owner: &str) -> Result<Vec<Goal>>;
}
```

**Implementation:** `SqliteGoalRepository` (~200 LOC)

---

### Step 2.2: CNS Verifier (`hkask-cns/src/goal_verifier.rs`)

**Port trait:**
```rust
use hkask_types::goal::{Goal, GoalOutcome, Verdict};

pub trait GoalVerifier {
    fn verify(&self, goal: &Goal, outcome: &GoalOutcome) -> Result<Verdict>;
}
```

**Implementation:** `CNSComparatorVerifier` (~100 LOC)

---

### Step 2.3: LLM Judge (`hkask-mcp-inference/src/goal_judge.rs`)

**Implementation:** `LLMJudgeVerifier` (~100 LOC)

---

### Step 2.4: AgentPod Executor (`hkask-agents/src/goal_executor.rs`)

**Implementation:** `AgentPodExecutor` (~150 LOC)

---

## Phase 3: CNS Integration (Week 4)

### Step 3.1: Add `Span::Goal` (`hkask-cns/src/span.rs`)

```rust
pub enum Span {
    // Existing...
    Tool(String),
    Prompt(String),
    AgentPod(String),
    
    // New: Goal primitive
    Goal(String),
}

impl Span {
    pub fn goal(event: &str) -> Self {
        Span::Goal(event.to_string())
    }
}
```

**LOC:** ~10

---

### Step 3.2: Variety Counter (`hkask-cns/src/goal_variety.rs`)

```rust
pub struct GoalVarietyCounter {
    threshold: u64,  // Default: 100
}

impl GoalVarietyCounter {
    pub fn check(&self, goal: &Goal, agent: &AgentPod) -> Result<(), AlgedonicAlert> {
        let environmental_states = goal.completion_criteria.len();
        let internal_states = agent.capability_count();
        
        let deficit = environmental_states.saturating_sub(internal_states);
        
        if deficit > self.threshold as usize {
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

**LOC:** ~50

---

## SQL Migration

Same as previous plan (`docs/storage/migrations/001_goals.sql`).

---

## LOC Budget

| Module | LOC |
|--------|-----|
| `hkask-types/src/goal.rs` | ~250 |
| `hkask-types/src/goal_capability.rs` | ~150 |
| `hkask-storage/src/goal_repository.rs` | ~200 |
| `hkask-cns/src/goal_verifier.rs` | ~100 |
| `hkask-cns/src/goal_variety.rs` | ~50 |
| `hkask-mcp-inference/src/goal_judge.rs` | ~100 |
| `hkask-agents/src/goal_executor.rs` | ~150 |
| **Total** | **~1,000** |

**Budget Remaining:** 30,000 - 1,000 = **29,000 LOC** ✅

---

## Verification

```bash
# Phase 1
cargo check -p hkask-types

# Phase 2
cargo check -p hkask-storage
cargo check -p hkask-cns
cargo check -p hkask-mcp-inference
cargo check -p hkask-agents

# Full workspace
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*  
*Minimalist design: modules, not crates. Rust is the loom. YAML/Jinja2 is the thread.*