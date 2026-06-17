//! hKask Corpus Discovery Service — find, download, and process corpora.
//!
//! Extracted from `hkask-services` to enable parallel compilation.

mod discover_impl;

pub use discover_impl::{
    DiscoverRequest, DiscoverResult, DiscoveredWork, DiscoveryService, default_corpus_config,
    download_and_cache, generate_corpus_yaml, slugify,
};
