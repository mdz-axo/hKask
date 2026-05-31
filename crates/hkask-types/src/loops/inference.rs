//! Loop 1: Inference — prompt → context → model → response → parse → act
//!
//! The Inference loop delegates LLM calls to Okapi.
//!
//! Essential subloop:
//! - 1.1 Context Assembly (FILTER) — filter and assemble context for inference
//!
//! Cybernetics regulation actions applied TO Inference:
//! - Energy throttling — Cybernetics owns the energy budget
//! - Circuit breaking — Cybernetics governs circuit state
//! - Energy cap adjustment — Curation can adjust budgets through Cybernetics

use crate::id::WebID;

/// Inference loop capability handle.
///
/// Identity handle: carries the agent's WebID. Energy and circuit state
/// are owned by Cybernetics, not Inference.
pub struct InferenceHandle {
    agent_webid: WebID,
}

impl InferenceHandle {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            agent_webid: WebID::new(),
        }
    }

    pub fn new(agent_webid: WebID) -> Self {
        Self { agent_webid }
    }

    pub fn agent(&self) -> &WebID {
        &self.agent_webid
    }
}
