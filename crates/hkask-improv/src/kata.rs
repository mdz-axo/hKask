//! Kata-improv integration — maps kata phases to recommended improv modes.
//!
//! **Starter Kata:**
//! - Observation Drill → Plussing (silently filter incorrect observations)
//! - Five Questions Drill → Yes And (reinforce correct answers)
//! - PDCA Cycle → Yes But (constrain experiment scope)
//!
//! **Coaching Kata:**
//! - Question 4 ("Next step? What do you expect?") → Yes But (introduce constraints)
//! - Question 5 ("How quickly can we go and see?") → Plussing (amplify design)
//!
//! CNS span: `cns.kata.improv.effectiveness` — tracks automaticity score delta
//! when improv modes are active vs. baseline kata performance.

use crate::modes::ImprovMode;

/// Alert threshold: alert if automaticity delta is negative (improv degraded kata).
const KATA_IMPROV_EFFECTIVENESS_ALERT: f64 = 0.0;

/// Kata phase — which phase of which kata is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KataPhase {
    /// Starter Kata — Observation Drill (distinguishing facts from interpretations).
    StarterObservation,
    /// Starter Kata — Five Questions Drill (practicing the 5-question sequence).
    StarterFiveQuestions,
    /// Starter Kata — PDCA Cycle (Plan-Do-Check-Act experimentation).
    StarterPdca,
    /// Coaching Kata — Question 4 (next step + prediction).
    CoachingQ4,
    /// Coaching Kata — Question 5 (feedback loop closure).
    CoachingQ5,
}

impl KataPhase {
    /// The recommended improv mode for this kata phase.
    ///
    /// Returns `None` for neutral phases (Questions 1-3) where the coach
    /// is just gathering information.
    pub fn recommended_mode(&self) -> Option<ImprovMode> {
        match self {
            KataPhase::StarterObservation => Some(ImprovMode::Plussing),
            KataPhase::StarterFiveQuestions => Some(ImprovMode::YesAnd),
            KataPhase::StarterPdca => Some(ImprovMode::YesBut),
            KataPhase::CoachingQ4 => Some(ImprovMode::YesBut),
            KataPhase::CoachingQ5 => Some(ImprovMode::Plussing),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            KataPhase::StarterObservation => "starter-observation",
            KataPhase::StarterFiveQuestions => "starter-five-questions",
            KataPhase::StarterPdca => "starter-pdca",
            KataPhase::CoachingQ4 => "coaching-q4",
            KataPhase::CoachingQ5 => "coaching-q5",
        }
    }
}

/// Result of applying an improv mode to a kata phase.
///
/// Tracks automaticity score delta. Alerts if delta is negative
/// (improv made kata performance worse).
#[derive(Debug, Clone)]
pub struct KataImprovResult {
    pub phase: KataPhase,
    pub mode: ImprovMode,
    /// Positive = improvement, negative = degradation, 0.0 = no change.
    pub automaticity_delta: f64,
    /// True if delta is below the alert threshold.
    pub should_alert: bool,
}

impl KataImprovResult {
    pub fn new(phase: KataPhase, mode: ImprovMode, automaticity_delta: f64) -> Self {
        let should_alert = automaticity_delta < KATA_IMPROV_EFFECTIVENESS_ALERT;
        Self {
            phase,
            mode,
            automaticity_delta,
            should_alert,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kata_phases_map_to_correct_modes() {
        assert!(matches!(
            KataPhase::StarterObservation.recommended_mode(),
            Some(ImprovMode::Plussing)
        ));
        assert!(matches!(
            KataPhase::StarterFiveQuestions.recommended_mode(),
            Some(ImprovMode::YesAnd)
        ));
        assert!(matches!(
            KataPhase::StarterPdca.recommended_mode(),
            Some(ImprovMode::YesBut)
        ));
        assert!(matches!(
            KataPhase::CoachingQ4.recommended_mode(),
            Some(ImprovMode::YesBut)
        ));
        assert!(matches!(
            KataPhase::CoachingQ5.recommended_mode(),
            Some(ImprovMode::Plussing)
        ));
    }

    #[test]
    fn kata_improv_result_delta_detection() {
        // Positive delta — no alert.
        let improved = KataImprovResult::new(KataPhase::CoachingQ4, ImprovMode::YesBut, 0.15);
        assert!(!improved.should_alert);

        // Negative delta — alert.
        let degraded = KataImprovResult::new(KataPhase::CoachingQ4, ImprovMode::YesBut, -0.05);
        assert!(degraded.should_alert);

        // Zero delta — no alert (0.0 < 0.0 is false).
        let neutral =
            KataImprovResult::new(KataPhase::StarterObservation, ImprovMode::Plussing, 0.0);
        assert!(!neutral.should_alert);
    }
}
