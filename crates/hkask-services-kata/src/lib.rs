//! hKask Toyota Kata Engine — Improvement Kata and Coaching Kata.
//!
//! Extracted from `hkask-services` to enable parallel compilation.

mod kata_impl;

pub use kata_impl::{
    ImprovementDirection, ImprovementSignal, KataEngine, KataError, KataHistory, KataManifest,
    KataResult, KataState, KataStep, PracticeEntry, StepExperience,
};
