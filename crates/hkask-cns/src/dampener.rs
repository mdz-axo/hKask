//! DAMPEN — Suppress repeated directives within a configurable time window
//!
//! Implements the DAMPEN messenger function (4.3: FILTER+RECONCILE) from the
//! 6-loop architecture. The Curation→Cybernetics→Curation
//! feedback cycle can produce repeated identical directives. DAMPEN prevents
//! the same directive from being issued within a configurable time window.
//!
//! # Why this lives in the CNS crate
//!
//! Dampening is a Cybernetics regulation function — it prevents oscillation
//! in the Curation↔Cybernetics feedback cycle. As such, it is owned by the
//! Cybernetics loop and lives in `hkask-cns`, the crate responsible for
//! homeostatic self-regulation. The dampener operates on `CuratorDirective`
//! data, but its purpose is regulatory, not curatorial: it is a FILTER
//! function that enforces the cybernetic stability of the system.
//!
//! # How it works
//!
//! When a directive is issued, the dampener records a "fingerprint" of the
//! directive (type + target) along with a timestamp. If the same fingerprint
//! is seen again within the dampening window, the directive is suppressed.
//!
//! This prevents oscillation in cybernetic feedback loops without preventing
//! genuine new directives from being delivered.

use hkask_types::WebID;
use hkask_types::loops::curation::CuratorDirective;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::Mutex;

/// Default dampening window: 60 seconds.
///
/// Within this window, the same directive (same type + target) will be
/// suppressed to prevent feedback oscillation.
pub const DEFAULT_DAMPEN_WINDOW: Duration = Duration::from_secs(60);

/// A fingerprint that identifies a directive for dampening.
///
/// Two directives with the same fingerprint will be suppressed if the
/// second arrives within the dampening window.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DirectiveFingerprint {
    /// The directive type: "calibrate_threshold", "update_capabilities", "adjust_energy_budget"
    directive_type: String,
    /// The target agent (if applicable)
    target: Option<WebID>,
}

impl DirectiveFingerprint {
    /// Create a fingerprint from a `CuratorDirective`.
    fn from_directive(directive: &CuratorDirective) -> Self {
        match directive {
            CuratorDirective::CalibrateThreshold { domain, .. } => Self {
                directive_type: "calibrate_threshold".to_string(),
                target: Some(WebID::from_persona(domain.as_bytes())),
            },
            CuratorDirective::UpdateCapabilities { agent, .. } => Self {
                directive_type: "update_capabilities".to_string(),
                target: Some(*agent),
            },
            CuratorDirective::AdjustEnergyBudget { agent, .. } => Self {
                directive_type: "adjust_energy_budget".to_string(),
                target: Some(*agent),
            },
        }
    }
}

/// DAMPEN — Suppress repeated directives within a configurable time window.
///
/// This implements the DAMPEN Cybernetics regulation function that prevents
/// feedback oscillation in the Curation→Cybernetics→Curation cycle.
///
/// # OCAP Discipline
///
/// The dampener does not change directives — it only decides whether to
/// pass them through or suppress them. It is a pure FILTER function.
pub struct Dampener {
    /// Recent directive fingerprints with their last-seen timestamps
    seen: Mutex<HashMap<DirectiveFingerprint, std::time::Instant>>,
    /// The dampening window duration
    window: Duration,
}

impl Dampener {
    /// Create a new dampener with the default 60-second window.
    pub fn new() -> Self {
        Self::with_window(DEFAULT_DAMPEN_WINDOW)
    }

    /// Create a new dampener with a custom dampening window.
    pub fn with_window(window: Duration) -> Self {
        Self {
            seen: Mutex::new(HashMap::new()),
            window,
        }
    }

    /// Check if a directive should be dampened (suppressed).
    ///
    /// Returns `true` if the directive was seen recently (within the
    /// dampening window) and should be suppressed.
    ///
    /// Returns `false` if the directive is new or the previous instance
    /// has expired from the window, meaning it should be delivered.
    pub async fn should_dampen(&self, directive: &CuratorDirective) -> bool {
        let fingerprint = DirectiveFingerprint::from_directive(directive);
        let now = std::time::Instant::now();
        let mut seen = self.seen.lock().await;

        // Evict expired entries first (lazy garbage collection)
        seen.retain(|_, last_seen| now.duration_since(*last_seen) < self.window);

        // Check if this fingerprint was seen recently
        if let Some(last_seen) = seen.get(&fingerprint)
            && now.duration_since(*last_seen) < self.window
        {
            return true; // Dampen: same directive within window
        }

        // Record this directive as seen
        seen.insert(fingerprint, now);
        false
    }

    /// Clear all dampening state.
    ///
    /// Useful for testing or when a major state change invalidates
    /// previous dampening decisions.
    pub async fn clear(&self) {
        self.seen.lock().await.clear();
    }

    /// Get the number of currently tracked fingerprints.
    ///
    /// Primarily useful for testing and observability.
    pub async fn tracked_count(&self) -> usize {
        self.seen.lock().await.len()
    }
}

impl Default for Dampener {
    fn default() -> Self {
        Self::new()
    }
}
