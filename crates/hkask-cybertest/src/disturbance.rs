use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DisturbanceKind {
    Latency,
    Timeout,
    MalformedPayload,
    CapabilityDenied,
    TransientConnectorFailure,
    StateCorruption,
    PromptValidationFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Disturbance {
    pub kind: DisturbanceKind,
    pub mode: DisturbanceMode,
}

impl Disturbance {
    pub fn new(kind: DisturbanceKind, mode: DisturbanceMode) -> Self {
        Self { kind, mode }
    }

    pub fn transient_failures(n_times: u32) -> Self {
        Self::new(
            DisturbanceKind::TransientConnectorFailure,
            DisturbanceMode::Occurrences(n_times),
        )
    }

    pub fn capability_denied() -> Self {
        Self::new(DisturbanceKind::CapabilityDenied, DisturbanceMode::Always)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DisturbanceMode {
    Always,
    Millis(u64),
    Occurrences(u32),
}
