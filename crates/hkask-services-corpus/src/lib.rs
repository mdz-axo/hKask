//! hKask Corpus Services — discovery and embedding pipeline.
//!
//! Merged from `hkask-services-discover` and `hkask-services-embed`.

mod discover;
mod embed;

pub use discover::{
    default_corpus_config, download_and_cache, generate_corpus_yaml, slugify, DiscoverRequest,
    DiscoverResult, DiscoveredWork, DiscoveryService,
};
pub use embed::{
    ocr_pdf_bytes, strip_html_tags, ChunkingConfig, CorpusConfig, EmbedPhase, EmbedProgress,
    EmbedResult, EmbedService, EmbeddingConfig, Entity, EntityConfig, FoundationalRule,
    ProgressFn, ValidationConfig, Work,
};
