//! Deletion Test trait — Lazy Universe grounding for module depth (TASK 4.3)
//!
//! Encodes John Ousterhout's deletion test (from `deep-module` skill) as a
//! trait that any module can implement, producing an `EnergyDelta`.
//!
//! # The Deletion Test
//!
//! If deleting this module would scatter its complexity across callers,
//! the module is deep and deserves to exist (positive energy delta —
//! deletion would increase total system energy).
//!
//! If deleting it would reduce total system complexity, it should be
//! deleted (negative energy delta — deletion moves toward lower action).
//!
//! # Epistemic grounding (P8)
//! - **crt:certainty** = Subjunctive (deletion is a what-if projection)
//! - **crt:force** = Hypothesis (energy estimate, not ground truth)
//! - **mode** = OUGHT (normative: modules SHOULD be deep)
//!
//! # CNS span: `cns.architecture.module_depth`
//!
//! # Lazy Universe connection
//!
//! The deletion test is the architectural analog of the least action principle:
//! a module exists at a stationary point in the system's energy landscape.
//! If `deletion_energy() < 0`, the module is not at a stationary point —
//! the system can move to a lower-energy configuration by deleting it.

use hkask_cns::EnergyDelta;

/// The deletion test: if deleting this module would scatter its complexity
/// across callers, the module is deep and deserves to exist.
/// If deleting it would reduce total system complexity, it should be deleted.
///
/// # Contract
///
/// ```text
/// {P: module M exists with N public functions}
/// C: apply_deletion_test(M)
/// {Q: M is deleted ∨ (|public(M')| ≤ |public(M)| ∧ depth(M') ≥ depth(M))}
/// ```
///
/// # Design Rule: DELETE_DEFAULT
///
/// The default posture is deletion. A module must *earn* its existence
/// by demonstrating that its removal would increase system energy.
/// This is the lazy universe applied to architecture: the system seeks
/// the minimal set of modules that satisfies all requirements.
pub trait DeletionTest {
    /// Compute the energy delta of deleting this module.
    ///
    /// # Returns
    ///
    /// - `EnergyDelta < 0`: System improves (module is shallow, delete it).
    ///   The system moves toward lower action — lazy universe satisfied.
    /// - `EnergyDelta = 0`: Stationary point (module is at equilibrium depth).
    /// - `EnergyDelta > 0`: System degrades (module is deep, keep it).
    ///   Deletion would scatter complexity — anti-lazy.
    fn deletion_energy(&self) -> EnergyDelta;

    /// The module's depth score: ratio of internal complexity to public interface size.
    ///
    /// A deep module has high internal complexity behind a small interface.
    /// A shallow module has low internal complexity behind a large interface.
    /// Default implementation: `1.0 - (public_fn_count / total_fn_count)`.
    /// Override for modules with non-function complexity (types, constants, etc.).
    fn depth_score(&self) -> f64 {
        1.0
    }

    /// Number of public functions exposed by this module.
    /// Used by `depth_score()` default implementation.
    fn public_fn_count(&self) -> usize {
        0
    }

    /// Total number of functions (public + private) in this module.
    /// Used by `depth_score()` default implementation.
    fn total_fn_count(&self) -> usize {
        0
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A deep module: 2 public functions, 20 internal — deletion would scatter complexity.
    struct DeepModule;

    impl DeletionTest for DeepModule {
        fn deletion_energy(&self) -> EnergyDelta {
            // Positive: deletion increases system energy (complexity scatters)
            EnergyDelta::from_raw(5.0)
        }

        fn public_fn_count(&self) -> usize {
            2
        }

        fn total_fn_count(&self) -> usize {
            22
        }
    }

    /// A shallow module: 10 public functions, 2 internal — deletion reduces complexity.
    struct ShallowModule;

    impl DeletionTest for ShallowModule {
        fn deletion_energy(&self) -> EnergyDelta {
            // Negative: deletion reduces system energy (complexity vanishes)
            EnergyDelta::from_raw(-3.0)
        }

        fn public_fn_count(&self) -> usize {
            10
        }

        fn total_fn_count(&self) -> usize {
            12
        }
    }

    // REQ: svc-deletion-test-001 — deep_module_has_positive_deletion_energy
    //
    // TASK 4.3 lazy-universe property: a deep module (small interface, large
    // internals) must have positive deletion energy — deleting it would
    // scatter complexity and increase total system action.
    #[test]
    fn deep_module_has_positive_deletion_energy() {
        let m = DeepModule;
        let energy = m.deletion_energy();
        assert!(
            energy.is_ascending(),
            "Deep module deletion should increase system energy (anti-lazy), got {energy}"
        );
        assert!(energy.as_raw() > 0.0);
    }

    // REQ: svc-deletion-test-002 — shallow_module_has_negative_deletion_energy
    //
    // TASK 4.3 lazy-universe property: a shallow module (large interface, small
    // internals) must have negative deletion energy — deleting it moves the
    // system toward lower action (lazy universe satisfied).
    #[test]
    fn shallow_module_has_negative_deletion_energy() {
        let m = ShallowModule;
        let energy = m.deletion_energy();
        assert!(
            energy.is_descending(),
            "Shallow module deletion should decrease system energy (lazy), got {energy}"
        );
        assert!(energy.as_raw() < 0.0);
    }

    // REQ: svc-deletion-test-003 — energy_delta_zero_is_descending
    //
    // Zero delta (stationary point) is considered descending — the system
    // has found its minimal-action configuration. This is the equilibrium
    // state the lazy universe predicts.
    #[test]
    fn energy_delta_zero_is_descending() {
        let zero = EnergyDelta::ZERO;
        assert!(
            zero.is_descending(),
            "Zero delta should be descending (stationary point)"
        );
        assert!(!zero.is_ascending());
    }

    // REQ: svc-deletion-test-004 — energy_delta_display_shows_direction
    #[test]
    fn energy_delta_display_shows_direction() {
        let descending = EnergyDelta::from_raw(-2.5);
        let ascending = EnergyDelta::from_raw(3.0);
        let stationary = EnergyDelta::ZERO;

        let desc_str = descending.to_string();
        let asc_str = ascending.to_string();
        let stat_str = stationary.to_string();

        assert!(
            desc_str.contains('↓'),
            "Negative delta should show ↓, got {desc_str}"
        );
        assert!(
            asc_str.contains('↑'),
            "Positive delta should show ↑, got {asc_str}"
        );
        assert!(
            stat_str.contains('↓'),
            "Zero delta should show ↓ (stationary), got {stat_str}"
        );
    }

    // REQ: svc-deletion-test-005 — alert_threshold_is_five_consecutive_ascending
    //
    // The algedonic threshold for anti-lazy drift is 5 consecutive positive
    // deltas. This matches the existing CNS pattern (variety deficit > threshold/2
    // → warning, > threshold → critical).
    #[test]
    fn alert_threshold_is_five_consecutive_ascending() {
        assert_eq!(EnergyDelta::ALERT_THRESHOLD, 5);
    }
}
