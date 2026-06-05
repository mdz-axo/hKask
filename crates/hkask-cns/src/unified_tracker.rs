//! Unified variety tracker — single SENSE point for all CNS observation domains
//!
//! Consolidates domain variety (4.1), bot metrics (4.3), and goal variety
//! into a single variety accounting structure. All SENSE subloops feed into
//! this one tracker, ensuring consistent variety accounting per Ashby's Law.
//!
//! # Design Rationale
//!
//! The previous design used independent variety-tracking structures:
//! - `VarietyMonitor` (Loop 4.1 — domain-based variety)
//! - `SovereigntyObserver` (Loop 4.4 — sovereignty event variety) — removed in Phase 11a
//! - `GoalVarietyMonitor` (Loop 4.4 — goal variety) — removed in Phase 11a
//! - `BotMetricsCollector` (Loop 4.3 — bot health variety) — removed in Phase 11b
//!
//! Each tracked variety independently, which meant:
//! 1. Inconsistent variety accounting (different windows, different reset policies)
//! 2. Duplicate state management (multiple HashMaps where one suffices)
//! 3. Complex CnsState (multiple fields where one suffices)
//!
//! The unified tracker uses domain-prefixed keys so all variety counting
//! goes through a single `VarietyMonitor`, while preserving the domain-specific
//! methods each subloop needs.

use crate::variety::VarietyMonitor;

/// Domain prefixes for unified variety counting.
///
/// All variety counters use domain-prefixed keys to avoid collisions
/// between different observation domains while sharing a single tracker.
pub mod domains {
    /// Bot variety tracking: `bot:{webid}:{category}`
    /// CNS resilience infrastructure — awaiting runtime wiring
    pub(crate) const BOT: &str = "bot";
}

/// Unified variety tracker for all CNS observation domains.
///
/// A single structure that tracks variety across all SENSE subloops:
/// - Loop 4.1: Domain-based variety (inference, memory, governance, etc.)
/// - Loop 4.3: Bot health metrics (per-WebID evaluation)
/// - Goal variety (per-WebID goal counting)
///
/// All variety counting goes through a single `VarietyMonitor`, ensuring
/// consistent windowing and reset behavior.
pub(crate) struct UnifiedVarietyTracker {
    /// Single variety monitor for all domains
    variety: VarietyMonitor,
}

impl UnifiedVarietyTracker {
    /// Create a new unified tracker.
    pub(crate) fn new() -> Self {
        Self {
            variety: VarietyMonitor::new(),
        }
    }

    // =========================================================================
    // Loop 4.1 — Domain-based variety (Ashby's Law)
    // =========================================================================

    /// Increment variety counter for a domain.
    pub(crate) fn increment_variety(&mut self, domain: &str, state_name: &str) {
        self.variety.counter(domain).increment(state_name);
    }

    /// Get variety count for a specific domain.
    pub(crate) fn variety_for_domain(&self, domain: &str) -> u64 {
        self.variety.variety_for_domain(domain)
    }

    /// Get all domain names with variety counters.
    pub(crate) fn variety_domains(&self) -> Vec<&str> {
        self.variety.domains()
    }

    /// Get total variety deficit across all domains.
    pub(crate) fn total_variety_deficit(&self, expected_per_domain: u64) -> u64 {
        self.variety.total_deficit(expected_per_domain)
    }

    /// Get a reference to the underlying variety monitor.
    pub(crate) fn variety_monitor(&self) -> &VarietyMonitor {
        &self.variety
    }
}

impl Default for UnifiedVarietyTracker {
    fn default() -> Self {
        Self::new()
    }
}
