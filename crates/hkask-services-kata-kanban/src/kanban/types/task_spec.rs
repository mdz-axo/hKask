use super::*;

// ── Task ───────────────────────────────────────────────────────────────────

/// TaskSpec — input specification for creating a new task.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSpec {
    /// Short title for the task.
    pub title: String,
    /// Optional longer description.
    pub description: Option<String>,
    /// Acceptance criteria — what "done" means.
    pub criteria: Vec<VerificationCriterion>,
    /// Optional agent assignment (requires consent).
    pub assignee: Option<WebID>,
    /// Story points for relative sizing (agile convention).
    pub story_points: Option<u32>,
    /// Estimated hours for completion.
    pub estimated_hours: Option<f64>,
    /// Labels/tags for categorization.
    pub labels: Vec<String>,
    /// Priority level.
    pub priority: Option<Priority>,
    /// Optional phase grouping.
    pub phase_id: Option<PhaseId>,
    /// Software-compute gas budget for this task (template exec, tool dispatch).
    pub gas_budget: Option<u64>,
    /// Inference/API rJoule budget (250k rJoules ≈ $1 inference spend).
    pub rjoule_budget: Option<u64>,
}

impl TaskSpec {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  title is non-empty
    /// post: returns a TaskSpec with no description, criteria, or assignee
    pub fn new(title: String) -> Self {
        Self {
            title,
            description: None,
            criteria: Vec::new(),
            assignee: None,
            story_points: None,
            estimated_hours: None,
            labels: Vec::new(),
            priority: None,
            phase_id: None,
            gas_budget: None,
            rjoule_budget: None,
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post: returns self with description set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post: returns self with criteria set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_criteria(mut self, criteria: Vec<VerificationCriterion>) -> Self {
        self.criteria = criteria;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid story points
    /// post: returns Self with story points set
    /// pre:  self is valid; assignee is a valid WebID
    /// post: returns self with assignee set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_assignee(mut self, assignee: WebID) -> Self {
        self.assignee = Some(assignee);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid estimated hours
    /// post: returns Self with estimated hours set
    /// pre:  points is a valid u32
    /// post: returns self with story_points set to Some(points)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_story_points(mut self, points: u32) -> Self {
        self.story_points = Some(points);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid labels
    /// post: returns Self with labels set
    /// pre:  hours is a non-negative f64
    /// post: returns self with estimated_hours set to Some(hours)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_estimated_hours(mut self, hours: f64) -> Self {
        self.estimated_hours = Some(hours);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  priority is a valid Priority variant
    /// post: returns self with priority set to Some(priority)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  labels is a vector of label strings
    /// post: returns self with labels set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  phase_id is a valid PhaseId
    /// post: returns self with phase_id set to Some(phase_id)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_phase(mut self, phase_id: PhaseId) -> Self {
        self.phase_id = Some(phase_id);
        self
    }

    /// Set the gas/rJoule budget for the subagent working on this task.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_gas_budget(mut self, gas: u64) -> Self {
        self.gas_budget = Some(gas);
        self
    }

    /// Set the inference/API rJoule budget for the subagent.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_rjoule_budget(mut self, rjoules: u64) -> Self {
        self.rjoule_budget = Some(rjoules);
        self
    }
}
