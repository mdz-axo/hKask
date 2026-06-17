//! hKask Inference Service — model resolution, routing, and context management.
//!
//! Extracted from `hkask-services`.
mod inference_svc_impl;
pub use inference_svc_impl::{InferenceContext, InferenceService, ModelInfo};
