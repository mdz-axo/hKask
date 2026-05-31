//! DAMPEN — Suppress repeated directives within a configurable time window
//!
//! Implements the DAMPEN messenger function (6.3: FILTER+RECONCILE) from the
//! 8-loop architecture. The Cybernetic loop (7) manages the
//! Observability→Governance feedback cycle, which can produce
//! repeated identical directives. DAMPEN prevents the same directive from
//! being issued within a configurable time window.
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
use hkask_types::loops::dispatch::LoopMessage;
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
/// This implements the DAMPEN messenger function (6.3) that prevents
/// feedback oscillation in the Cybernetic loop (7): the
/// Observability→Governance feedback cycle.
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

    /// Check if a `LoopMessage` carrying a governance directive should be dampened.
    ///
    /// This is a convenience method that extracts the `CuratorDirective`
    /// from a `LoopPayload::GovernanceDirective` if present, then checks
    /// dampening. Non-governance messages are never dampened.
    pub async fn should_dampen_message(&self, message: &LoopMessage) -> bool {
        if let hkask_types::loops::dispatch::LoopPayload::GovernanceDirective {
            directive_type,
            target,
            parameters,
        } = &message.payload
        {
            // Reconstruct a CuratorDirective from the LoopMessage payload
            let directive = match directive_type.as_str() {
                "calibrate_threshold" => CuratorDirective::CalibrateThreshold {
                    domain: parameters
                        .get("domain")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    new_threshold: parameters
                        .get("new_threshold")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                },
                "update_capabilities" => CuratorDirective::UpdateCapabilities {
                    agent: *target,
                    additions: parameters
                        .get("additions")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    removals: parameters
                        .get("removals")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                },
                "adjust_energy_budget" => CuratorDirective::AdjustEnergyBudget {
                    agent: *target,
                    new_budget: parameters
                        .get("new_budget")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                },
                _ => return false, // Unknown directive type — don't dampen
            };
            self.should_dampen(&directive).await
        } else {
            false // Non-governance messages are never dampened
        }
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
