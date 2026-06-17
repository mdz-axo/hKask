//! hKask Text Classification Service — section typing and triple extraction.
//!
//! Extracted from `hkask-services` to enable parallel compilation.

mod classify_impl;

pub use classify_impl::{
    ClassifierConfig, TripleExtraction, classify_batch, extract_triples_batch,
    load_classifier_config,
};
