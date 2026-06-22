//! Minimal loop types that must stay in hkask-types to avoid circular deps
//! (hkask-ports uses LoopId and cannot depend on hkask-cns).

/// Loop identifiers for the 6-loop model.
///
/// VSM correspondence per `hkask-cns/src/types/loops/mod.rs`:
/// - Loop 1:  Inference    (S1 Implementation)
/// - Loop 2a: Episodic     (S2 Coordination — private memory)
/// - Loop 2b: Semantic     (S2 Coordination — shared memory)
/// - Loop 5:  Curation     (S4 Intelligence — meta-observer)
/// - Loop 6:  Cybernetics  (S3 Control — homeostatic regulation)
/// - Loop 6b: Snapshot     (S3 Control — scheduled CAS snapshots)
///
/// No Loop 3: Control absorbed into Cybernetics (intentional).
/// No Loop 4: VSM S4 = Curation (Loop 5).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LoopId {
    Inference,
    Episodic,
    Semantic,
    Curation,
    Cybernetics,
    Snapshot,
}

impl std::fmt::Display for LoopId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopId::Inference => write!(f, "inference"),
            LoopId::Episodic => write!(f, "episodic"),
            LoopId::Semantic => write!(f, "semantic"),
            LoopId::Curation => write!(f, "curation"),
            LoopId::Cybernetics => write!(f, "cybernetics"),
            LoopId::Snapshot => write!(f, "snapshot"),
        }
    }
}
