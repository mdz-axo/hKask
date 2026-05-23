//! Goal variety counter — CNS monitoring for active goal diversity
//!
//! Per Ashby's Law of Requisite Variety, the system must track
//! goal state diversity to maintain control.
//!
//! Algedonic Alert: >10 concurrent active goals per user → escalate

use hkask_types::cns::{AlgedonicAlert, CnsSpan, VarietyCounter};
use hkask_types::id::WebID;

/// Goal variety counter — tracks active goals per user
pub struct GoalVarietyCounter {
    webid: WebID,
    active_goal_count: u64,
    threshold: u64,
}

impl GoalVarietyCounter {
    pub fn new(webid: WebID) -> Self {
        Self {
            webid,
            active_goal_count: 0,
            threshold: 10,
        }
    }

    pub fn with_threshold(mut self, threshold: u64) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn update_count(&mut self, count: u64) {
        self.active_goal_count = count;
    }

    pub fn increment(&mut self) {
        self.active_goal_count += 1;
    }

    pub fn decrement(&mut self) {
        if self.active_goal_count > 0 {
            self.active_goal_count -= 1;
        }
    }

    pub fn needs_alert(&self) -> bool {
        self.active_goal_count > self.threshold
    }

    pub fn emit_algedonic_alert(&self) -> AlgedonicAlert {
        AlgedonicAlert::new(
            self.active_goal_count,
            self.threshold,
            CnsSpan::Goal,
        )
    }

    pub fn variety_counter(&self) -> VarietyCounter {
        VarietyCounter(self.active_goal_count)
    }
}

/// Goal variety monitor — manages variety counters for all users
pub struct GoalVarietyMonitor {
    counters: std::collections::HashMap<WebID, GoalVarietyCounter>,
    default_threshold: u64,
}

impl GoalVarietyMonitor {
    pub fn new() -> Self {
        Self {
            counters: std::collections::HashMap::new(),
            default_threshold: 10,
        }
    }

    pub fn get_or_create(&mut self, webid: WebID) -> &mut GoalVarietyCounter {
        self.counters.entry(webid).or_insert_with(|| {
            GoalVarietyCounter::new(webid).with_threshold(self.default_threshold)
        })
    }

    pub fn check_all(&mut self) -> Vec<AlgedonicAlert> {
        let mut alerts = Vec::new();
        
        for counter in self.counters.values_mut() {
            if counter.needs_alert() {
                alerts.push(counter.emit_algedonic_alert());
            }
        }
        
        alerts
    }

    pub fn set_threshold(&mut self, threshold: u64) {
        self.default_threshold = threshold;
        for counter in self.counters.values_mut() {
            counter.threshold = threshold;
        }
    }
}

impl Default for GoalVarietyMonitor {
    fn default() -> Self {
        Self::new()
    }
}
