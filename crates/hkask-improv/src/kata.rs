//! Kata-improv integration — wired improv modes into kata coaching loops.
//!
//! Maps each kata phase to its recommended improv mode:
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

use crate::cns::KATA_IMPROV_EFFECTIVENESS_ALERT;
use crate::modes::ImprovMode;

/// Kata phase identifiers — which phase of which kata is active.
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
    /// Improvement Kata — general PDCA experimentation.
    ImprovementPdca,
}

impl KataPhase {
    /// The recommended improv mode for this kata phase.
    ///
    /// Returns `None` if no improv mode is recommended (neutral phases
    /// like Questions 1-3 where the coach is just gathering information).
    pub fn recommended_mode(&self) -> Option<ImprovMode> {
        match self {
            // Starter Kata: Observation Drill uses Plussing to silently filter
            // incorrect observations without discouraging the learner.
            KataPhase::StarterObservation => Some(ImprovMode::Plussing),

            // Starter Kata: Five Questions Drill uses Yes And to reinforce
            // correct answers and build momentum.
            KataPhase::StarterFiveQuestions => Some(ImprovMode::YesAnd),

            // Starter Kata: PDCA Cycle uses Yes But to constrain the experiment
            // scope — "yes, try that, but limit to one variable at a time."
            KataPhase::StarterPdca => Some(ImprovMode::YesBut),

            // Coaching Kata Q4: "What is your next step? What do you expect?"
            // Uses Yes But to introduce constraints that guide the learner's
            // next experiment without dictating the answer.
            KataPhase::CoachingQ4 => Some(ImprovMode::YesBut),

            // Coaching Kata Q5: "How quickly can we go and see?"
            // Uses Plussing to amplify what the learner got right in their
            // experimental design before suggesting refinements.
            KataPhase::CoachingQ5 => Some(ImprovMode::Plussing),

            // Improvement Kata: General PDCA — can use a cascade of
            // Yes But (constrain) → Plussing (amplify) for balanced coaching.
            KataPhase::ImprovementPdca => Some(ImprovMode::YesBut),
        }
    }

    /// Human-readable label for this phase.
    pub fn label(&self) -> &'static str {
        match self {
            KataPhase::StarterObservation => "starter-observation",
            KataPhase::StarterFiveQuestions => "starter-five-questions",
            KataPhase::StarterPdca => "starter-pdca",
            KataPhase::CoachingQ4 => "coaching-q4",
            KataPhase::CoachingQ5 => "coaching-q5",
            KataPhase::ImprovementPdca => "improvement-pdca",
        }
    }
}

/// Result of applying an improv mode to a kata phase.
///
/// Tracks whether the mode improved or degraded kata performance,
/// measured as an automaticity score delta.
#[derive(Debug, Clone)]
pub struct KataImprovResult {
    /// The kata phase that was coached.
    pub phase: KataPhase,
    /// The improv mode that was applied.
    pub mode: ImprovMode,
    /// Automaticity score delta (positive = improvement, negative = degradation).
    /// A delta of 0.0 means no measurable change.
    pub automaticity_delta: f64,
    /// Whether this result should trigger a CNS alert.
    pub should_alert: bool,
}

impl KataImprovResult {
    /// Create a new result and check against alert thresholds.
    pub fn new(phase: KataPhase, mode: ImprovMode, automaticity_delta: f64) -> Self {
        let should_alert = automaticity_delta < KATA_IMPROV_EFFECTIVENESS_ALERT;
        Self {
            phase,
            mode,
            automaticity_delta,
            should_alert,
        }
    }

    /// Whether the improv mode improved kata performance.
    pub fn improved(&self) -> bool {
        self.automaticity_delta > 0.0
    }

    /// Whether the improv mode degraded kata performance (needs investigation).
    pub fn degraded(&self) -> bool {
        self.automaticity_delta < 0.0
    }
}

/// Build a recommended improv cascade for a full coaching kata session.
///
/// Returns a cascade of: Yes But (Q4) → Plussing (Q5), representing
/// the full coaching dialogue with improv postures.
pub fn coaching_kata_cascade() -> ImprovMode {
    use crate::cascade::ImprovCascade;
    // This is a 2-step cascade — well within the matryoshka limit.
    let cascade = ImprovCascade::new(vec![
        KataPhase::CoachingQ4.recommended_mode().unwrap(),
        KataPhase::CoachingQ5.recommended_mode().unwrap(),
    ])
    .expect("2-step coaching cascade is within matryoshka limit");
    ImprovMode::Cascade(cascade)
}

/// Build a recommended improv cascade for a full starter kata session.
///
/// Returns a cascade of: Plussing (Observation) → Yes And (5 Questions) → Yes But (PDCA).
pub fn starter_kata_cascade() -> ImprovMode {
    use crate::cascade::ImprovCascade;
    let cascade = ImprovCascade::new(vec![
        KataPhase::StarterObservation.recommended_mode().unwrap(),
        KataPhase::StarterFiveQuestions.recommended_mode().unwrap(),
        KataPhase::StarterPdca.recommended_mode().unwrap(),
    ])
    .expect("3-step starter cascade is within matryoshka limit");
    ImprovMode::Cascade(cascade)
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: Each kata phase maps to the correct improv mode
    #[test]
    fn kata_phases_map_to_correct_modes() {
        assert_eq!(
            KataPhase::StarterObservation.recommended_mode(),
            Some(ImprovMode::Plussing)
        );
        assert_eq!(
            KataPhase::StarterFiveQuestions.recommended_mode(),
            Some(ImprovMode::YesAnd)
        );
        assert_eq!(
            KataPhase::StarterPdca.recommended_mode(),
            Some(ImprovMode::YesBut)
        );
        assert_eq!(
            KataPhase::CoachingQ4.recommended_mode(),
            Some(ImprovMode::YesBut)
        );
        assert_eq!(
            KataPhase::CoachingQ5.recommended_mode(),
            Some(ImprovMode::Plussing)
        );
        assert_eq!(
            KataPhase::ImprovementPdca.recommended_mode(),
            Some(ImprovMode::YesBut)
        );
    }

    // REQ: KataImprovResult correctly identifies improvement vs degradation
    #[test]
    fn kata_improv_result_delta_detection() {
        let improved = KataImprovResult::new(
            KataPhase::CoachingQ4,
            ImprovMode::YesBut,
            0.15, // Positive delta
        );
        assert!(improved.improved());
        assert!(!improved.degraded());
        assert!(!improved.should_alert);

        let degraded = KataImprovResult::new(
            KataPhase::CoachingQ4,
            ImprovMode::YesBut,
            -0.05, // Negative delta
        );
        assert!(!degraded.improved());
        assert!(degraded.degraded());
        assert!(degraded.should_alert); // Below 0.0 threshold

        let neutral = KataImprovResult::new(
            KataPhase::StarterObservation,
            ImprovMode::Plussing,
            0.0, // No change
        );
        assert!(!neutral.improved());
        assert!(!neutral.degraded());
        assert!(neutral.should_alert); // At threshold (0.0 ≤ 0.0)
    }

    // REQ: Coaching kata cascade is 2 steps (within limit)
    #[test]
    fn coaching_kata_cascade_is_valid() {
        let mode = coaching_kata_cascade();
        match mode {
            ImprovMode::Cascade(c) => {
                assert_eq!(c.step_count(), 2);
                assert!(c.total_applications() <= 7);
            }
            other => panic!("Expected Cascade, got {:?}", other),
        }
    }

    // REQ: Starter kata cascade is 3 steps (within limit)
    #[test]
    fn starter_kata_cascade_is_valid() {
        let mode = starter_kata_cascade();
        match mode {
            ImprovMode::Cascade(c) => {
                assert_eq!(c.step_count(), 3);
                assert!(c.total_applications() <= 7);
            }
            other => panic!("Expected Cascade, got {:?}", other),
        }
    }
}
