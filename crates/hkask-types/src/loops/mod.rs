//! hKask 4-Loop Architecture
//!
//! Four cybernetic feedback loops following Beer's Viable System Model.
//! Each loop implements sense → compare → compute → act.
//!
//! **Loop Numbering (VSM correspondence):**
//!
//! The numbering follows Stafford Beer's VSM. Loop 3 (Control) is absorbed
//! into Cybernetics — the homeostatic regulator IS the controller.
//! There is no Loop 3; this is intentional, not a gap.
//!
//! | Loop | Name | VSM Role | Category |
//! |------|------|----------|----------|
//! | 1 | Inference | Implementation | Domain |
//! | 2a | Episodic Memory | Coordination (private) | Domain |
//! | 2b | Semantic Memory | Coordination (shared) | Domain |
//! | 5 | Curation | Metasystem (observer) | Meta |
//! | 6 | Cybernetics | Homeostatic regulation | Meta |
//! | 6b | Snapshot | Scheduled CAS snapshots | Meta |
//!
//! **Bridge:**
//! - 2a→2b: Consolidation — episodic → strip perspective → store semantic (one-way)
//!
//! **Authority DAG:** Curation → Cybernetics → {Inference, Episodic, Semantic}
//! No sideways edges. Authority flows downward.

pub mod actions;
pub mod channels;
pub mod core;
pub mod curation;
pub mod episodic;
pub mod signals;

pub use channels::{
    CurationInput, GoalTransitionEvent, RuntimeAlert, SpecEvent, ToolConsumptionEvent,
};
pub use curation::{CuratorDirective, CuratorHandle};
pub use episodic::ExperienceClassification;

pub use actions::{ActionType, LoopAction};
pub use core::{Loop, LoopId, LoopQuality};
pub use signals::{Deviation, DeviationDirection, Signal, SignalMetric};

pub use self::core::Loop as HkaskLoop;

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: types-loop-quality-001 — LoopQuality::default() has zero values
// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    #[test]
    fn loop_quality_default_is_zero() {
        let q = LoopQuality::default();
        assert_eq!(q.delay_ms, 0);
        assert!((q.gain - 0.0).abs() < f64::EPSILON);
        assert!((q.fidelity_score - 0.0).abs() < f64::EPSILON);
    }

    // REQ: types-loop-quality-002 — from_cycle computes gain correctly
// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    #[test]
    fn from_cycle_computes_gain() {
        let sig = Signal::new(LoopId::Cybernetics, SignalMetric::VarietyDeficit, 0.9, 0.5);
        let dev = Deviation::from_signal(&sig).unwrap();
        let action = LoopAction::new(
            LoopId::Curation,
            ActionType::Escalate,
            serde_json::json!({"reason": "variety_deficit_exceeded"}),
        );
        let q = LoopQuality::from_cycle(100, &[dev], &[action]);
        assert_eq!(q.delay_ms, 100);
        assert!((q.gain - 1.0).abs() < f64::EPSILON);
        assert!((q.fidelity_score - 1.0).abs() < f64::EPSILON);
    }

    // REQ: types-loop-quality-003 — from_cycle with no deviations has zero gain
// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    #[test]
    fn from_cycle_no_deviations_zero_gain() {
        let q = LoopQuality::from_cycle(50, &[], &[]);
        assert_eq!(q.delay_ms, 50);
        assert!((q.gain - 0.0).abs() < f64::EPSILON);
        assert!((q.fidelity_score - 0.0).abs() < f64::EPSILON);
    }

    // REQ: types-loop-quality-004 — unmatched deviation reduces fidelity
// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    #[test]
    fn unmatched_deviation_reduces_fidelity() {
        let sig = Signal::new(LoopId::Cybernetics, SignalMetric::ErrorRate, 0.3, 0.1);
        let dev = Deviation::from_signal(&sig).unwrap();
        // Action with unrelated reason
        let action = LoopAction::new(
            LoopId::Inference,
            ActionType::Throttle,
            serde_json::json!({"reason": "energy_budget_low"}),
        );
        let q = LoopQuality::from_cycle(200, &[dev], &[action]);
        assert!((q.fidelity_score - 0.0).abs() < f64::EPSILON);
    }
}
