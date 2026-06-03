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
