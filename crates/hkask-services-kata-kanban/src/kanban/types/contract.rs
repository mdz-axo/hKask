use super::*;

// ── Task Contract (rSolidity) ─────────────────────────────────────────────

/// TaskContract — a kanban task assignment expressed as an rSolidity contract.
///
/// Binds delegator and delegate with:
/// - Pre-conditions: acceptance criteria (what must be true before work starts)
/// - Post-conditions: verification conditions (what must be true to accept work)
/// - OCAP gates: capability tokens delegated for the work
/// - Gas limit: maximum energy budget
/// - Timeout: maximum execution time
///
/// Maps to rSolidity's require!/assert!/emit! macros for CNS-observable
/// contract execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TaskContract {
    /// Name of the capability package used.
    pub package_name: String,
    /// The replicant delegating the work.
    pub delegator: WebID,
    /// The agent receiving the delegation.
    pub delegate: WebID,
    /// The task this contract governs.
    pub task_id: TaskId,
    /// Task title for display.
    pub task_title: String,
    /// Pre-conditions (acceptance criteria) — informational expectations.
    pub pre_conditions: Vec<String>,
    /// Post-conditions — informational expectations.
    pub post_conditions: Vec<String>,
    /// Maximum gas/energy budget.
    pub gas_limit: u64,
    /// Maximum execution time in seconds.
    pub timeout: u64,
    /// Maximum attenuation level.
    pub max_attenuation: u8,
    /// Contract state: pending, active, completed, violated.
    pub state: ContractState,
}

/// ContractState — the execution state of a TaskContract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ContractState {
    /// Contract created but not yet active.
    Pending,
    /// All post-conditions satisfied — contract fulfilled.
    Completed,
    /// One or more post-conditions violated.
    Violated,
}

impl TaskContract {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  arguments are valid
    /// post:      /// post: returns new instance with defaults
    pub fn new(package_name: String, delegator: WebID, delegate: WebID, task: &Task) -> Self {
        Self {
            package_name,
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
                "All criteria satisfied".into(),
                "Deliverables verified".into(),
            ],
            gas_limit: 50000,
            timeout: 3600,
            max_attenuation: 3,
            state: ContractState::Pending,
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  state allows activation
    /// Task completion is user-feedback-driven. The agent submits evidence
    /// (a description of what was done) and the user confirms. Criteria are
    /// informational expectations — they guide the work but don't gate completion.
    /// Completion produces: task output (deliverables) + CNS spans + user feedback
    /// → learning signal for the system.
    pub fn check_completion(&mut self, evidence: &str) -> ContractVerification {
        // Evidence IS the completion signal. Non-empty evidence = user confirmed.
        if evidence.trim().is_empty() {
            self.state = ContractState::Violated;
            return ContractVerification {
                passed: false,
                reasoning: "No evidence provided — task not verified.".into(),
            };
        }

        self.state = ContractState::Completed;
        let criteria_list: Vec<String> = self
            .pre_conditions
            .iter()
            .map(|c| format!("  - {c}"))
            .collect();
        let criteria_block = if criteria_list.is_empty() {
            String::new()
        } else {
            format!("\nCriteria:\n{}", criteria_list.join("\n"))
        };

        ContractVerification {
            passed: true,
            reasoning: format!(
                "User feedback received.{} Evidence length: {} chars.",
                criteria_block,
                evidence.len()
            ),
        }
    }
}

/// ContractVerification — result of checking a TaskContract's completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ContractVerification {
    pub passed: bool,
    pub reasoning: String,
}
