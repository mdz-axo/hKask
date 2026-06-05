//! Prompt Decomposition — deterministic sentence analysis for CNS variety sensing
//!
//! **Relocated to `hkask_agents::prompt_analysis`.** This module can no longer
//! re-export from hkask-agents because that would create a circular dependency
//! (hkask-agents depends on hkask-cns). Consumers should import directly from
//! `hkask_agents::prompt_analysis::{PromptAnalysis, SentenceDecomposition, decompose_prompt}`.
//!
//! Prompt decomposition is inference variety sensing (Loop 1), not cybernetic
//! regulation (Loop 6). The CNS consumes the output; it does not produce it.
