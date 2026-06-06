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
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::Mutex;

/// Default dampening window: 60 seconds.
///
/// Within this window, the same directive (same type + target) will be
/// suppressed to prevent feedback oscillation.
pub(crate) const DEFAULT_DAMPEN_WINDOW: Duration = Duration::from_secs(60);

/// Metacognitive override dampening window: 300 seconds.
///
/// Metacognitive overrides (`OverrideGasBudget`, `SeekMoreEvidence`) represent
/// higher-order reflective interventions and are dampened at a longer window
/// to prevent premature re-issuance while still allowing genuine re-triggering.
pub(crate) const METACOGNITIVE_DAMPEN_WINDOW: Duration = Duration::from_secs(300);

/// A fingerprint that identifies a directive for dampening.
///
/// Two directives with the same fingerprint will be suppressed if the
/// second arrives within the dampening window.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DirectiveFingerprint {
    /// The directive type: "calibrate_threshold", "update_capabilities", "adjust_gas_budget"
    directive_type: String,
    /// The target agent (if applicable)
    target: Option<WebID>,
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
pub(crate) struct Dampener {
    /// Recent directive fingerprints with their last-seen timestamps
    seen: Mutex<HashMap<DirectiveFingerprint, std::time::Instant>>,
    /// Standard dampening window for routine directives
    window: Duration,
    /// Extended dampening window for metacognitive overrides (used for eviction)
    metacognitive_window: Duration,
}

impl Dampener {
    /// Create a new dampener with the default 60-second window.
    pub(crate) fn new() -> Self {
        Self::with_window(DEFAULT_DAMPEN_WINDOW)
    }

    /// Create a new dampener with a custom standard dampening window.
    ///
    /// The metacognitive window defaults to 300 seconds.
    pub(crate) fn with_window(window: Duration) -> Self {
        Self {
            seen: Mutex::new(HashMap::new()),
            window,
            metacognitive_window: METACOGNITIVE_DAMPEN_WINDOW,
        }
    }

    /// Check if a directive should be dampened (suppressed).
    ///
    /// Accepts the directive type and target directly, for use when the
    /// full `CuratorDirective` is not available (e.g., from
    /// `LoopPayload::CurationDirective`).
    pub(crate) async fn should_dampen_directive(
        &self,
        directive_type: &str,
        target: WebID,
    ) -> bool {
        let fingerprint = DirectiveFingerprint {
            directive_type: directive_type.to_string(),
            target: Some(target),
        };
        let now = std::time::Instant::now();
        let mut seen = self.seen.lock().await;

        // Evict expired entries first (lazy garbage collection).
        // Use the larger window to avoid premature eviction.
        let max_window = self.window.max(self.metacognitive_window);
        seen.retain(|_, last_seen| now.duration_since(*last_seen) < max_window);

        // Check if this fingerprint was seen recently
        if let Some(last_seen) = seen.get(&fingerprint)
            && now.duration_since(*last_seen) < self.window
        {
            return true; // Dampen: same directive within standard window
        }

        // Record this directive as seen
        seen.insert(fingerprint, now);
        false
    }
}

impl Default for Dampener {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_agent() -> WebID {
        WebID::from_persona(b"test-agent")
    }

    #[tokio::test]
    async fn routine_directive_dampened_at_standard_window() {
        let dampener = Dampener::new();
        let agent = test_agent();
        assert!(
            !dampener
                .should_dampen_directive("calibrate_threshold", agent)
                .await
        );
        assert!(
            dampener
                .should_dampen_directive("calibrate_threshold", agent)
                .await
        );
    }

    #[tokio::test]
    async fn replenish_budget_is_routine() {
        let dampener = Dampener::new();
        let agent = test_agent();
        // replenish_budget uses the standard window
        assert!(
            !dampener
                .should_dampen_directive("replenish_budget", agent)
                .await
        );
        assert!(
            dampener
                .should_dampen_directive("replenish_budget", agent)
                .await
        );
    }

    #[tokio::test]
    async fn should_dampen_directive_uses_standard_window() {
        let dampener = Dampener::with_window(Duration::from_millis(100));
        let agent = test_agent();
        assert!(
            !dampener
                .should_dampen_directive("calibrate_threshold", agent)
                .await
        );
        assert!(
            dampener
                .should_dampen_directive("calibrate_threshold", agent)
                .await
        );
        // After window expires, directive is no longer dampened
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(
            !dampener
                .should_dampen_directive("calibrate_threshold", agent)
                .await
        );
    }
}
