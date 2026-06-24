//! hKask Corpus Services — discovery and embedding pipeline.
//!
//! Merged from `hkask-services-discover` and `hkask-services-embed`.

mod discover_impl;
mod embed_impl;

pub use discover_impl::{
    DiscoverRequest, DiscoverResult, DiscoveredWork, DiscoveryService, default_corpus_config,
    download_and_cache, generate_corpus_yaml, slugify,
};
pub use embed_impl::{
    ChunkingConfig, CorpusConfig, EmbedPhase, EmbedProgress, EmbedResult, EmbedService,
    EmbeddingConfig, Entity, EntityConfig, FoundationalRule, ProgressFn, ValidationConfig, Work,
    ocr_pdf_bytes, strip_html_tags,
};
