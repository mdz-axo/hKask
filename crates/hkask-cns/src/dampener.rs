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
pub(crate) const DEFAULT_DAMPEN_WINDOW: Duration = Duration::from_secs(60);

/// Metacognitive override dampening window: 300 seconds.
///
/// Metacognitive overrides (`OverrideGasBudget`, `SeekMoreEvidence`) represent
/// higher-order reflective interventions and are dampened at a longer window
/// to prevent premature re-issuance while still allowing genuine re-triggering.
pub(crate) const METACOGNITIVE_DAMPEN_WINDOW: Duration = Duration::from_secs(300);

/// Default override cooldown: 120 seconds.
///
/// Within this window after ANY metacognitive override, ALL subsequent
/// metacognitive overrides are suppressed — even if they have different
/// fingerprints. This prevents oscillation when Curation overrides
/// Cybernetics and the response triggers a second override.
pub(crate) const DEFAULT_OVERRIDE_COOLDOWN: std::time::Duration =
    std::time::Duration::from_secs(120);

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
            CuratorDirective::OverrideGasBudget { agent, .. } => Self {
                directive_type: "override_gas_budget".to_string(),
                target: Some(*agent),
            },
            CuratorDirective::SeekMoreEvidence {
                context, channel, ..
            } => Self {
                directive_type: "seek_more_evidence".to_string(),
                target: Some(WebID::from_persona(
                    format!("{context}:{channel}").as_bytes(),
                )),
            },
            CuratorDirective::ReplenishBudget {
                agent, priority: _, ..
            } => Self {
                directive_type: "replenish_budget".to_string(),
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
pub(crate) struct Dampener {
    /// Recent directive fingerprints with their last-seen timestamps
    seen: Mutex<HashMap<DirectiveFingerprint, std::time::Instant>>,
    /// Standard dampening window for routine directives
    window: Duration,
    /// Extended dampening window for metacognitive overrides
    metacognitive_window: Duration,
    /// Timestamp of the last metacognitive override that passed fingerprint
    /// deduplication. Used to enforce the override cooldown.
    last_override: Mutex<Option<std::time::Instant>>,
    /// Override cooldown duration. Within this window after ANY metacognitive
    /// override, ALL subsequent metacognitive overrides are suppressed.
    override_cooldown: Duration,
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
            last_override: Mutex::new(None),
            override_cooldown: DEFAULT_OVERRIDE_COOLDOWN,
        }
    }

    /// Set a custom metacognitive override dampening window.
    ///
    /// Builder-style method: `Dampener::new().with_metacognitive_window(dur)`.
    pub(crate) fn with_metacognitive_window(mut self, window: Duration) -> Self {
        self.metacognitive_window = window;
        self
    }

    /// Set a custom override cooldown duration.
    ///
    /// Within this window after ANY metacognitive override, ALL subsequent
    /// metacognitive overrides are suppressed — even if they have different
    /// fingerprints. This prevents oscillation when Curation overrides
    /// Cybernetics and the response triggers a second override.
    pub(crate) fn with_override_cooldown(mut self, cooldown: Duration) -> Self {
        self.override_cooldown = cooldown;
        self
    }

    /// Returns `true` if the directive is a metacognitive override.
    ///
    /// Metacognitive overrides are reflective interventions that go beyond
    /// routine regulation: `OverrideGasBudget` and `SeekMoreEvidence`.
    /// Routine directives (`CalibrateThreshold`, `UpdateCapabilities`,
    /// `ReplenishBudget`) use the standard dampening window.
    fn is_metacognitive(directive: &CuratorDirective) -> bool {
        matches!(
            directive,
            CuratorDirective::OverrideGasBudget { .. } | CuratorDirective::SeekMoreEvidence { .. }
        )
    }

    /// Check if a directive should be dampened (suppressed).
    ///
    /// Returns `true` if the directive was seen recently (within the
    /// dampening window) and should be suppressed.
    ///
    /// Returns `false` if the directive is new or the previous instance
    /// has expired from the window, meaning it should be delivered.
    pub(crate) async fn should_dampen(&self, directive: &CuratorDirective) -> bool {
        let fingerprint = DirectiveFingerprint::from_directive(directive);
        let window = if Self::is_metacognitive(directive) {
            self.metacognitive_window
        } else {
            self.window
        };
        let now = std::time::Instant::now();
        let mut seen = self.seen.lock().await;

        // Evict expired entries first (lazy garbage collection).
        // Use the larger of the two windows to avoid premature eviction of
        // metacognitive entries.
        let max_window = self.window.max(self.metacognitive_window);
        seen.retain(|_, last_seen| now.duration_since(*last_seen) < max_window);

        // Override cooldown: if a metacognitive override was issued recently,
        // suppress ALL subsequent metacognitive overrides regardless of
        // fingerprint.
        if Self::is_metacognitive(directive) {
            let override_guard = self.last_override.lock().await;
            if let Some(last) = *override_guard {
                if now.duration_since(last) < self.override_cooldown {
                    return true; // Cooldown active — dampen
                }
            }
            drop(override_guard);
        }

        // Check if this fingerprint was seen recently
        if let Some(last_seen) = seen.get(&fingerprint)
            && now.duration_since(*last_seen) < window
        {
            return true; // Dampen: same directive within window
        }

        // Record this directive as seen
        seen.insert(fingerprint, now);

        // If this metacognitive override was NOT dampened, record the
        // override timestamp for cooldown tracking.
        if Self::is_metacognitive(directive) {
            let mut override_guard = self.last_override.lock().await;
            *override_guard = Some(now);
        }

        false
    }

    /// Check if a raw directive (by type and target) should be dampened.
    ///
    /// Uses the **standard** dampening window. For metacognitive override
    /// directives, prefer `should_dampen_metacognitive`.
    ///
    /// This accepts the directive type and target directly, for use when the
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

    /// Check if a metacognitive override directive should be dampened.
    ///
    /// Uses the extended metacognitive dampening window (default 300s).
    /// For routine directives, use `should_dampen_directive` instead.
    pub(crate) async fn should_dampen_metacognitive(
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
            && now.duration_since(*last_seen) < self.metacognitive_window
        {
            return true; // Dampen: metacognitive override within extended window
        }

        // Record this directive as seen
        seen.insert(fingerprint, now);
        false
    }

    /// Clear all dampening state.
    ///
    /// Useful for testing or when a major state change invalidates
    /// previous dampening decisions.
    pub(crate) async fn clear(&self) {
        self.seen.lock().await.clear();
        *self.last_override.lock().await = None;
    }

    /// Get the number of currently tracked fingerprints.
    ///
    /// Primarily useful for testing and observability.
    pub(crate) async fn tracked_count(&self) -> usize {
        self.seen.lock().await.len()
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
    use hkask_types::WebID;

    fn test_agent() -> WebID {
        WebID::from_persona(b"test-agent")
    }

    #[tokio::test]
    async fn routine_directive_dampened_at_standard_window() {
        // CalibrateThreshold is a routine directive → 60s window
        let dampener = Dampener::with_window(Duration::from_secs(60));
        let directive = CuratorDirective::CalibrateThreshold {
            domain: "confidence".to_string(),
            new_threshold: 42,
        };

        // First occurrence: not dampened
        assert!(!dampener.should_dampen(&directive).await);
        // Immediate repeat: dampened
        assert!(dampener.should_dampen(&directive).await);
    }

    #[tokio::test]
    async fn metacognitive_override_dampened_at_extended_window() {
        // OverrideGasBudget is a metacognitive override → 300s window
        let dampener = Dampener::new();
        let directive = CuratorDirective::OverrideGasBudget {
            agent: test_agent(),
            new_budget: 500,
        };

        // First occurrence: not dampened
        assert!(!dampener.should_dampen(&directive).await);
        // Immediate repeat: dampened
        assert!(dampener.should_dampen(&directive).await);
    }

    #[tokio::test]
    async fn seek_more_evidence_is_metacognitive() {
        let dampener = Dampener::new();
        let directive = CuratorDirective::SeekMoreEvidence {
            context: "decision-42".to_string(),
            channel: "llm_confidence".to_string(),
            confidence: "0.5".to_string(),
        };

        assert!(!dampener.should_dampen(&directive).await);
        assert!(dampener.should_dampen(&directive).await);
    }

    #[tokio::test]
    async fn replenish_budget_is_routine() {
        let dampener = Dampener::new();
        let directive = CuratorDirective::ReplenishBudget {
            agent: test_agent(),
            amount: 100,
            priority: None,
        };

        assert!(!dampener.should_dampen(&directive).await);
        assert!(dampener.should_dampen(&directive).await);
    }

    #[tokio::test]
    async fn should_dampen_directive_uses_standard_window() {
        let dampener = Dampener::new();
        let agent = test_agent();

        // First occurrence: not dampened
        assert!(
            !dampener
                .should_dampen_directive("calibrate_threshold", agent)
                .await
        );
        // Immediate repeat: dampened (standard window)
        assert!(
            dampener
                .should_dampen_directive("calibrate_threshold", agent)
                .await
        );
    }

    #[tokio::test]
    async fn should_dampen_metacognitive_uses_extended_window() {
        let dampener = Dampener::new();
        let agent = test_agent();

        // First occurrence: not dampened
        assert!(
            !dampener
                .should_dampen_metacognitive("override_gas_budget", agent)
                .await
        );
        // Immediate repeat: dampened (metacognitive window)
        assert!(
            dampener
                .should_dampen_metacognitive("override_gas_budget", agent)
                .await
        );
    }

    #[tokio::test]
    async fn metacognitive_and_standard_use_different_windows() {
        // Use a very short standard window and a longer metacognitive window
        // to verify they behave differently.
        let dampener = Dampener::with_window(Duration::from_millis(50))
            .with_metacognitive_window(Duration::from_secs(300));

        let routine = CuratorDirective::CalibrateThreshold {
            domain: "confidence".to_string(),
            new_threshold: 10,
        };
        let metacog = CuratorDirective::OverrideGasBudget {
            agent: test_agent(),
            new_budget: 999,
        };

        // Both should not be dampened on first call
        assert!(!dampener.should_dampen(&routine).await);
        assert!(!dampener.should_dampen(&metacog).await);

        // Both should be dampened on immediate repeat
        assert!(dampener.should_dampen(&routine).await);
        assert!(dampener.should_dampen(&metacog).await);

        // After standard window expires, routine should pass again
        // while metacognitive is still dampened
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(
            !dampener.should_dampen(&routine).await,
            "routine should pass after standard window"
        );
        assert!(
            dampener.should_dampen(&metacog).await,
            "metacognitive should still be dampened"
        );
    }

    #[tokio::test]
    async fn with_metacognitive_window_builder() {
        let dampener = Dampener::with_window(Duration::from_secs(10))
            .with_metacognitive_window(Duration::from_secs(999));

        assert_eq!(dampener.window, Duration::from_secs(10));
        assert_eq!(dampener.metacognitive_window, Duration::from_secs(999));
    }

    #[tokio::test]
    async fn override_cooldown_dampens_metacognitive_within_window() {
        // Use a very short override cooldown to test without long sleeps.
        let dampener = Dampener::new().with_override_cooldown(Duration::from_millis(100));

        // First override: passes
        let override1 = CuratorDirective::OverrideGasBudget {
            agent: test_agent(),
            new_budget: 500,
        };
        assert!(!dampener.should_dampen(&override1).await);

        // Second override with DIFFERENT fingerprint (different budget) but
        // within cooldown: dampened
        let override2 = CuratorDirective::OverrideGasBudget {
            agent: test_agent(),
            new_budget: 999,
        };
        assert!(
            dampener.should_dampen(&override2).await,
            "different-fingerprint override should be dampened by cooldown"
        );
    }

    #[tokio::test]
    async fn override_cooldown_does_not_affect_routine_directives() {
        let dampener = Dampener::new().with_override_cooldown(Duration::from_secs(300));

        // Issue a metacognitive override to activate the cooldown
        let metacog = CuratorDirective::OverrideGasBudget {
            agent: test_agent(),
            new_budget: 500,
        };
        assert!(!dampener.should_dampen(&metacog).await);

        // Routine directive should NOT be affected by the override cooldown
        let routine = CuratorDirective::CalibrateThreshold {
            domain: "confidence".to_string(),
            new_threshold: 42,
        };
        assert!(
            !dampener.should_dampen(&routine).await,
            "routine directive should not be dampened by override cooldown"
        );
    }

    #[tokio::test]
    async fn override_cooldown_allows_metacognitive_after_expiry() {
        let dampener = Dampener::new().with_override_cooldown(Duration::from_millis(50));

        // Issue a metacognitive override
        let override1 = CuratorDirective::SeekMoreEvidence {
            context: "decision-42".to_string(),
            channel: "llm_confidence".to_string(),
            confidence: "0.5".to_string(),
        };
        assert!(!dampener.should_dampen(&override1).await);

        // Different metacognitive override within cooldown: dampened
        let override2 = CuratorDirective::OverrideGasBudget {
            agent: test_agent(),
            new_budget: 999,
        };
        assert!(
            dampener.should_dampen(&override2).await,
            "override should be dampened during cooldown"
        );

        // After cooldown expires, metacognitive override can proceed
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(
            !dampener.should_dampen(&override2).await,
            "override should pass after cooldown expires"
        );
    }

    #[tokio::test]
    async fn is_metacognitive_classification() {
        let routine_cases = vec![
            CuratorDirective::CalibrateThreshold {
                domain: "d".to_string(),
                new_threshold: 1,
            },
            CuratorDirective::UpdateCapabilities {
                agent: test_agent(),
                additions: vec![],
                removals: vec![],
            },
            CuratorDirective::ReplenishBudget {
                agent: test_agent(),
                amount: 100,
                priority: None,
            },
        ];
        let metacognitive_cases = vec![
            CuratorDirective::OverrideGasBudget {
                agent: test_agent(),
                new_budget: 500,
            },
            CuratorDirective::SeekMoreEvidence {
                context: "c".to_string(),
                channel: "ch".to_string(),
                confidence: "0.5".to_string(),
            },
        ];

        for directive in &routine_cases {
            assert!(
                !Dampener::is_metacognitive(directive),
                "Expected routine: {directive:?}"
            );
        }
        for directive in &metacognitive_cases {
            assert!(
                Dampener::is_metacognitive(directive),
                "Expected metacognitive: {directive:?}"
            );
        }
    }
}
