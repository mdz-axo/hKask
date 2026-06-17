//! hKask Embedding Pipeline — corpus processing, chunking, OCR, entity extraction.
//!
//! Extracted from `hkask-services` to enable parallel compilation.

mod embed_impl;

pub use embed_impl::{
    ChunkingConfig, CorpusConfig, EmbedPhase, EmbedProgress, EmbedResult, EmbedService,
    EmbeddingConfig, Entity, EntityConfig, FoundationalRule, ProgressFn, ValidationConfig, Work,
    ocr_pdf_bytes, strip_html_tags,
};
