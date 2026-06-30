use super::*;

// ── Spawn Specification ─────────────────────────────────────────────────

/// SpawnSpec — configuration for spawning a sub-replicant to execute a task.
///
/// Defines what capabilities (skills, memory scope, tool access) the parent
/// replicant delegates to the spawned sub-agent. Spawning is consent-mediated
/// (P1) — the parent chooses what to delegate.
///
/// Delegation levels:
/// - Minimal: read-only access to the task, no memory, restricted tools
/// - Standard: read-write task access, episodic memory, kanban tools
/// - Maximal: full replicant capabilities within the task scope
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnSpec {
    /// The task this spawn is for.
    pub task_id: TaskId,
    /// Delegation level: "minimal", "standard", or "maximal".
    pub delegation_level: String,
    /// Skills to delegate to the spawned replicant.
    pub delegated_skills: Vec<String>,
    /// Memory scope: "none", "episodic", or "full".
    pub memory_scope: String,
    /// Tool servers accessible to the spawned replicant.
    pub tool_servers: Vec<String>,
    /// Maximum gas/energy budget for the spawned replicant.
    pub gas_budget: Option<u64>,
    /// Maximum time the spawned replicant can run (seconds).
    pub timeout_seconds: Option<u64>,
    /// Template/skill registries accessible to the spawned replicant.
    pub registries: Vec<String>,
    /// File paths or artifact roots the agent can access.
    pub artifacts: Vec<String>,
    /// OCAP capability token specs (e.g. "tool:kanban:execute").
    /// These are validated against the parent replicant's tokens at spawn time.
    /// Each entry is a CapabilitySpec string: "resource:domain:action".
    pub capability_tokens: Vec<String>,
}

impl SpawnSpec {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid skills
    /// post: returns Self with skills set
    /// pre:  task_id is valid
    /// post: returns a SpawnSpec with standard delegation defaults
    pub fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            delegation_level: "standard".into(),
            delegated_skills: vec!["kanban".into()],
            memory_scope: "episodic".into(),
            tool_servers: vec!["hkask-mcp-kanban".into()],
            gas_budget: None,
            timeout_seconds: None,
            registries: Vec::new(),
            artifacts: Vec::new(),
            capability_tokens: Vec::new(),
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid timeout
    /// post: returns Self with timeout set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_level(mut self, level: &str) -> Self {
        self.delegation_level = level.into();
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for skills
    /// post:      /// post: returns Self with skills set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.delegated_skills = skills;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for memory
    /// post:      /// post: returns Self with memory set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_memory(mut self, scope: &str) -> Self {
        self.memory_scope = scope.into();
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for gas budget
    /// post:      /// post: returns Self with gas budget set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_gas_budget(mut self, budget: u64) -> Self {
        self.gas_budget = Some(budget);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for timeout
    /// post:      /// post: returns Self with timeout set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for registries
    /// post:      /// post: returns Self with registries set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_registries(mut self, registries: Vec<String>) -> Self {
        self.registries = registries;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for artifacts
    /// post:      /// post: returns Self with artifacts set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_artifacts(mut self, artifacts: Vec<String>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for capability tokens
    /// post:      /// post: returns Self with capability tokens set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_capability_tokens(mut self, tokens: Vec<String>) -> Self {
        self.capability_tokens = tokens;
        self
    }
}

// ── Capability Package ────────────────────────────────────────────────────

/// CapabilityPackage — a named, reusable bundle of delegated capabilities.
///
/// Saved spawn configurations that can be reused across tasks and projects.
/// After a board completes, the user is prompted to save any capability
/// packages they composed, so delegations can reference them by name.
///
/// Stored as YAML in registry/capabilities/ alongside kata manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityPackage {
    /// Unique package name (e.g., "backend-dev", "docs-writer").
    pub name: String,
    /// Human-readable description of what this package provides.
    pub description: String,
    /// Delegation level: minimal, standard, maximal.
    pub delegation_level: String,
    /// Skills delegated to the agent.
    pub skills: Vec<String>,
    /// Memory scope: none, episodic, full.
    pub memory_scope: String,
    /// MCP tool servers accessible.
    pub tool_servers: Vec<String>,
    /// Template/skill registries accessible.
    pub registries: Vec<String>,
    /// File paths or artifact roots.
    pub artifacts: Vec<String>,
    /// Default gas budget (can be overridden per task).
    pub default_gas_budget: Option<u64>,
    /// Default timeout in seconds (can be overridden per task).
    pub default_timeout_seconds: Option<u64>,
    /// OCAP capability token specs delegated to the agent.
    /// These are the actual OCAP strings validated at spawn time
    /// (e.g. "tool:kanban:execute", "registry:templates:read").
    pub capability_tokens: Vec<String>,
    /// Maximum attenuation level for delegated tokens (0-7).
    /// The spawned agent cannot further attenuate beyond this.
    pub max_attenuation: u8,
}

impl CapabilityPackage {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  arguments are valid
    /// post:      /// post: returns new instance with defaults
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            delegation_level: "standard".into(),
            skills: Vec::new(),
            memory_scope: "episodic".into(),
            tool_servers: Vec::new(),
            registries: Vec::new(),
            artifacts: Vec::new(),
            default_gas_budget: None,
            default_timeout_seconds: None,
            capability_tokens: Vec::new(),
            max_attenuation: 3,
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid memory
    /// post: returns Self with memory set
    pub fn to_spawn_spec(&self, task_id: TaskId) -> SpawnSpec {
        SpawnSpec {
            task_id,
            delegation_level: self.delegation_level.clone(),
            delegated_skills: self.skills.clone(),
            memory_scope: self.memory_scope.clone(),
            tool_servers: self.tool_servers.clone(),
            gas_budget: self.default_gas_budget,
            timeout_seconds: self.default_timeout_seconds,
            registries: self.registries.clone(),
            artifacts: self.artifacts.clone(),
            capability_tokens: self.capability_tokens.clone(),
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid artifacts
    /// post: returns Self with artifacts set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_level(mut self, level: &str) -> Self {
        self.delegation_level = level.into();
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid gas
    /// post: returns Self with gas set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid timeout
    /// post: returns Self with timeout set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_memory(mut self, scope: &str) -> Self {
        self.memory_scope = scope.into();
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for tools
    /// post:      /// post: returns Self with tools set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tool_servers = tools;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for registries
    /// post:      /// post: returns Self with registries set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_registries(mut self, registries: Vec<String>) -> Self {
        self.registries = registries;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for artifacts
    /// post:      /// post: returns Self with artifacts set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_artifacts(mut self, artifacts: Vec<String>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post: returns converted value
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_gas(mut self, budget: u64) -> Self {
        self.default_gas_budget = Some(budget);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for timeout
    /// post:      /// post: returns Self with timeout set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.default_timeout_seconds = Some(seconds);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post: returns converted value
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_capability_tokens(mut self, tokens: Vec<String>) -> Self {
        self.capability_tokens = tokens;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for max attenuation
    /// post:      /// post: returns Self with max attenuation set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_max_attenuation(mut self, level: u8) -> Self {
        self.max_attenuation = level.min(7);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  tools list is non-empty
    /// post: populates capability_tokens from tool_servers
    /// Converts "hkask-mcp-kanban" → "tool:kanban:execute".
    pub fn derive_tokens_from_tools(&mut self) {
        self.capability_tokens = self
            .tool_servers
            .iter()
            .filter_map(|server_id| capability_from_server_id(server_id))
            .collect();
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post:      /// post: returns converted representation
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml_neo::to_string(self).map_err(|e| e.to_string())
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  input is valid
    /// post:      /// post: returns parsed object
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml_neo::from_str(yaml).map_err(|e| e.to_string())
    }

    // ── rSolidity Contract Integration ─────────────────────────────────

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post:      /// post: returns converted representation
    /// task contract. The contract binds delegator and delegate with
    /// pre-conditions (acceptance criteria), post-conditions (verification),
    /// and OCAP gates (capability tokens).
    ///
    /// Returns a structured representation suitable for rSolidity
    /// contract execution and CNS span emission.
    pub fn to_task_contract(&self, task: &Task, delegator: WebID, delegate: WebID) -> TaskContract {
        TaskContract {
            package_name: self.name.clone(),
            delegator,
            delegate,
            task_id: task.id,
            task_title: task.title.clone(),
            pre_conditions: task
                .criteria
                .iter()
                .map(|c| c.description.clone())
                .collect(),
            post_conditions: vec![
                "All acceptance criteria satisfied".into(),
                "Deliverables submitted and verified".into(),
            ],
            gas_limit: self.default_gas_budget.unwrap_or(50000),
            timeout: self.default_timeout_seconds.unwrap_or(3600),
            max_attenuation: self.max_attenuation,
            state: ContractState::Pending,
        }
    }
}
