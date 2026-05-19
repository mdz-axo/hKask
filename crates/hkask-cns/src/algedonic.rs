//! Algedonic alerts — Variety deficit escalation

/// Algedonic alert threshold
pub const DEFAULT_THRESHOLD: u64 = 100;

/// Algedonic alert
#[derive(Debug, Clone)]
pub struct AlgedonicAlert {
    pub deficit: u64,
    pub threshold: u64,
    pub escalated: bool,
}

impl AlgedonicAlert {
    pub fn new(deficit: u64, threshold: u64) -> Self {
        Self {
            deficit,
            threshold,
            escalated: deficit > threshold,
        }
    }

    pub fn should_escalate(&self) -> bool {
        self.escalated
    }
}
