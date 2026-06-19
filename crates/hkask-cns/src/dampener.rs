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
//! Two dampening layers:
//!
//! 1. **Per-fingerprint dedup** — When a directive is issued, the dampener
//!    records a "fingerprint" (type + target) with a timestamp. If the same
//!    fingerprint is seen again within the standard window, the directive is
//!    suppressed. This prevents repeated identical directives.
//!
//! 2. **Override cooldown** — After any metacognitive override
//!    (`override_energy_budget`, `seek_more_evidence`) passes the fingerprint
//!    dedup, ALL subsequent overrides are suppressed for the cooldown period
//!    (default 120s), regardless of type or target. This prevents override
//!    oscillation: a different override targeting a different agent cannot
//!    bypass the cooldown by changing its fingerprint.

use hkask_types::WebID;
use hkask_types::loops::CuratorDirective;
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
/// Metacognitive overrides (`OverrideEnergyBudget`, `SeekMoreEvidence`) represent
/// higher-order reflective interventions and are dampened at a longer window
/// to prevent premature re-issuance while still allowing genuine re-triggering.
pub(crate) const METACOGNITIVE_DAMPEN_WINDOW: Duration = Duration::from_secs(300);

/// Default override cooldown: 120 seconds.
///
/// After any metacognitive override passes the per-fingerprint dedup check,
/// ALL subsequent overrides are suppressed for this duration regardless of
/// type or target. This prevents override oscillation — the scenario where
/// the Curation↔Cybernetics feedback loop rapidly fires different overrides.
pub(crate) const DEFAULT_OVERRIDE_COOLDOWN: Duration = Duration::from_secs(120);

/// A fingerprint that identifies a directive for dampening.
///
/// Two directives with the same fingerprint will be suppressed if the
/// second arrives within the dampening window.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DirectiveFingerprint {
    /// The directive variant name (e.g., "calibrate_threshold", "override_energy_budget").
    variant: String,
    /// The target agent (if applicable).
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
    /// Timestamp of the last metacognitive override that passed dedup.
    /// After any override passes, ALL subsequent overrides are suppressed
    /// for `override_cooldown` seconds. This prevents override oscillation.
    last_override: Mutex<Option<std::time::Instant>>,
    /// Cooldown window after a metacognitive override passes dedup.
    /// Default: 120 seconds. Within this window, ALL overrides are suppressed
    /// regardless of type or target.
    override_cooldown: Duration,
}

impl Dampener {
    /// Create a new dampener with the default 60-second window and 120-second
    /// override cooldown.
    pub(crate) fn new() -> Self {
        Self::with_window(DEFAULT_DAMPEN_WINDOW)
    }

    /// Create a new dampener with a custom standard dampening window.
    ///
    /// The metacognitive window defaults to 300 seconds.
    /// The override cooldown defaults to 120 seconds.
    pub(crate) fn with_window(window: Duration) -> Self {
        Self {
            seen: Mutex::new(HashMap::new()),
            window,
            metacognitive_window: METACOGNITIVE_DAMPEN_WINDOW,
            last_override: Mutex::new(None),
            override_cooldown: DEFAULT_OVERRIDE_COOLDOWN,
        }
    }

    ///
    /// Two dampening layers are applied in order:
    ///
    /// 1. **Per-fingerprint dedup** — if the same (type, target) directive
    ///    was seen within the standard window, suppress.
    ///
    /// 2. **Override cooldown** — for metacognitive overrides only: if any
    ///    override passed dedup within the cooldown period, suppress ALL
    ///    subsequent overrides regardless of type or target.
    ///
    /// If neither layer suppresses the directive, the fingerprint is recorded
    /// and (for overrides) the override timestamp is set.
    pub(crate) async fn should_dampen_directive(&self, directive: &CuratorDirective) -> bool {
        let fingerprint = DirectiveFingerprint {
            variant: directive.variant_name().to_string(),
            target: directive.agent_target(),
        };
        let now = std::time::Instant::now();

        // Step 1: Per-fingerprint dedup check
        {
            let mut seen = self.seen.lock().await;
            // Evict expired entries first (lazy garbage collection).
            // Use the larger window to avoid premature eviction.
            let max_window = self.window.max(self.metacognitive_window);
            seen.retain(|_, last_seen| now.duration_since(*last_seen) < max_window);

            if let Some(last_seen) = seen.get(&fingerprint)
                && now.duration_since(*last_seen) < self.window
            {
                return true; // Dampen: same directive within standard window
            }
        }

        // Step 2: Override cooldown for metacognitive overrides
        if directive.is_metacognitive() {
            let mut last_override = self.last_override.lock().await;
            if let Some(last) = *last_override
                && now.duration_since(last) < self.override_cooldown
            {
                return true; // Dampen: override cooldown active
            }
            // Override passes — record timestamp
            *last_override = Some(now);
        }

        // Step 3: Record fingerprint (directive allowed through)
        {
            let mut seen = self.seen.lock().await;
            seen.insert(fingerprint, now);
        }

        false
    }
}

impl Default for Dampener {
    fn default() -> Self {
        Self::new()
    }
}
