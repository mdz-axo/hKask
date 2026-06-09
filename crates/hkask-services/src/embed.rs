//! EmbedService — Style corpus embedding pipeline.
//!
//! Orchestrates the full embedding pipeline: config parsing → DB open →
//! purge → download + cache + chunk → batch embed → centroid compute.
//! Uses `SemanticMemory` methods (same code that MCP tools
//! semantic_purge, semantic_chunk, semantic_embed, semantic_centroid call).
//!
//! ℏKask - A Minimal Viable Container for Agents

use hkask_memory::SemanticMemory;
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_templates::{OkapiConfig, OkapiEmbedding};

use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;

use crate::error::ServiceError;

/// Corpus configuration — defines the author, works, embedding model,
/// chunking parameters, and validation constraints for a style corpus.
#[derive(Debug, Deserialize)]
pub struct CorpusConfig {
    pub author: String,
    pub embedding: EmbeddingConfig,
    pub works: Vec<Work>,
    pub foundational_rules: Vec<FoundationalRule>,
    pub chunking: ChunkingConfig,
    pub centroid_entity_ref: String,
    pub validation: ValidationConfig,
}

/// Embedding model and dimension configuration.
#[derive(Debug, Deserialize)]
pub struct EmbeddingConfig {
    pub model: String,
    pub dim: usize,
    pub batch_size: usize,
}

/// A work (text) to download and embed.
#[derive(Debug, Deserialize)]
pub struct Work {
    pub title: String,
    pub slug: String,
    pub url: String,
}

/// A foundational rule to include as a passage.
#[derive(Debug, Deserialize)]
pub struct FoundationalRule {
    pub slug: String,
    pub text: String,
}

/// Chunking parameters for passage splitting.
#[derive(Debug, Deserialize)]
pub struct ChunkingConfig {
    pub min_words: usize,
    pub max_words: usize,
    pub sentence_boundary: String,
}

/// Validation constraints for centroid distance and exemplar counts.
#[derive(Debug, Clone, Deserialize)]
pub struct ValidationConfig {
    pub centroid_distance_max: f64,
    pub exemplar_count_min: usize,
    pub exemplar_count_max: usize,
}

/// Result of the embedding pipeline.
#[derive(Debug)]
pub struct EmbedResult {
    /// Author name from the corpus config.
    pub author: String,
    /// Number of existing embeddings purged before re-ingest.
    pub purged: usize,
    /// Total passages embedded and stored.
    pub total_passages: usize,
    /// Entity reference for the computed centroid.
    pub centroid_ref: String,
    /// Number of prose passages used for centroid computation.
    pub passage_count: usize,
    /// Whether the centroid was stored as an embedding.
    pub centroid_stored: bool,
    /// Validation config from corpus config.
    pub validation: ValidationConfig,
}

const USER_AGENT: &str = "hkask-mcp-web/0.22.0";

/// Service for the style corpus embedding pipeline.
///
/// Orchestrates config parsing, DB construction, text download/caching,
/// passage chunking, batch embedding via Okapi, and centroid computation.
pub struct EmbedService;

impl EmbedService {
    /// Run the full style corpus embedding pipeline.
    ///
    /// Reads the corpus config YAML, opens the semantic DB, purges existing
    /// embeddings for the author, downloads/caches/chunks texts, embeds
    /// passages in batches via Okapi, and computes the style centroid.
    ///
    /// If `cache_dir` is None, derives it from the config file's parent
    /// directory joined with `.cache`.
    pub async fn embed_corpus(
        config_path: &Path,
        db_path: &str,
        db_passphrase: &str,
        okapi_url: Option<&str>,
        cache_dir: Option<&Path>,
    ) -> Result<EmbedResult, ServiceError> {
        // Parse config
        let config_str = std::fs::read_to_string(config_path).map_err(|e| {
            ServiceError::Embed(format!(
                "Failed to read corpus config {}: {e}",
                config_path.display()
            ))
        })?;
        let config: CorpusConfig = serde_yaml::from_str(&config_str)
            .map_err(|e| ServiceError::Embed(format!("Failed to parse corpus config YAML: {e}")))?;

        let author = config.author.clone();
        let author_prefix = format!("style:{}:", &author);
        let centroid_ref = config.centroid_entity_ref.clone();
        let validation = config.validation.clone();

        // Open DB and construct semantic memory
        let db = Database::open(db_path, db_passphrase)?;
        let conn = db.conn_arc();
        let triple_store = TripleStore::new(Arc::clone(&conn));
        let embedding_store = EmbeddingStore::with_dim(conn, config.embedding.dim);
        let semantic = SemanticMemory::new(triple_store, embedding_store);

        // Purge existing embeddings for idempotent re-ingest
        let purged = semantic
            .purge_by_prefix(&author_prefix)
            .map_err(|e| ServiceError::Embed(format!("Failed to purge embeddings: {e}")))?;

        // Resolve cache directory
        let default_cache_dir;
        let cache = match cache_dir {
            Some(p) => p,
            None => {
                default_cache_dir = config_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(".cache");
                &default_cache_dir
            }
        };

        // Download, cache, and chunk all works
        let mut all_passages: Vec<(String, String)> = Vec::new();

        for (work_idx, work) in config.works.iter().enumerate() {
            // Rate limit between requests
            if work_idx > 0 {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            let cache_path = cache.join(format!("{}.txt", work.slug));
            let text = if cache_path.exists() {
                tracing::info!(work = %work.title, "Using cached");
                std::fs::read_to_string(&cache_path).map_err(|e| {
                    ServiceError::Embed(format!(
                        "Failed to read cache {}: {e}",
                        cache_path.display()
                    ))
                })?
            } else {
                tracing::info!(work = %work.title, "Downloading");
                let text = download_text(&work.url).await?;
                if let Err(e) = std::fs::write(&cache_path, &text) {
                    tracing::warn!(
                        path = %cache_path.display(),
                        error = %e,
                        "Could not cache download"
                    );
                }
                text
            };

            let cleaned = SemanticMemory::strip_gutenberg_headers(&text);
            let entity_ref_prefix = format!("style:{}:{}", &config.author, work.slug);
            let chunks = SemanticMemory::chunk_text(
                &cleaned,
                &entity_ref_prefix,
                config.chunking.min_words,
                config.chunking.max_words,
                &config.chunking.sentence_boundary,
            );
            tracing::info!(work = %work.title, passages = chunks.len(), "Chunked");
            all_passages.extend(chunks);
        }

        // Append foundational rules as passages
        for rule in &config.foundational_rules {
            let entity_ref = format!("style:{}:rule:{}", &config.author, rule.slug);
            all_passages.push((entity_ref, rule.text.clone()));
        }

        tracing::info!(total = all_passages.len(), "Total passages to embed");

        // Create embedder
        let okapi_config = match okapi_url {
            Some(url) => OkapiConfig {
                base_url: url.to_string(),
                ..OkapiConfig::default()
            },
            None => OkapiConfig::local_dev(),
        };
        let embedder = OkapiEmbedding::with_model(&config.embedding.model, okapi_config)?;

        // Batch embed
        let batch_size = config.embedding.batch_size;
        let mut embedded_count = 0;
        for chunk in all_passages.chunks(batch_size) {
            let texts: Vec<&str> = chunk.iter().map(|(_, text)| text.as_str()).collect();
            let vectors = embedder
                .embed_sentences(&texts)
                .await
                .map_err(|e| ServiceError::Embed(format!("Failed to embed batch: {e}")))?;

            for ((entity_ref, _text), vector) in chunk.iter().zip(vectors.iter()) {
                semantic.store_embedding(entity_ref, vector, &config.embedding.model)?;
            }
            embedded_count += chunk.len();
            tracing::info!(
                embedded = embedded_count,
                total = all_passages.len(),
                "Embedding progress"
            );
        }

        // Compute centroid
        tracing::info!("Computing style centroid");
        let rule_prefix = format!("style:{}:rule:", &config.author);
        let centroid_result = semantic.compute_centroid(
            &author_prefix,
            &rule_prefix,
            &centroid_ref,
            config.embedding.dim,
            Some(&centroid_ref),
            Some(&config.embedding.model),
        )?;

        Ok(EmbedResult {
            author,
            purged,
            total_passages: all_passages.len(),
            centroid_ref,
            passage_count: centroid_result.passage_count,
            centroid_stored: centroid_result.stored,
            validation,
        })
    }

    /// Parse a corpus config YAML file.
    pub fn parse_config(path: &Path) -> Result<CorpusConfig, ServiceError> {
        let config_str = std::fs::read_to_string(path).map_err(|e| {
            ServiceError::Embed(format!(
                "Failed to read corpus config {}: {e}",
                path.display()
            ))
        })?;
        serde_yaml::from_str(&config_str)
            .map_err(|e| ServiceError::Embed(format!("Failed to parse corpus config YAML: {e}")))
    }
}

// ── Internal helpers ────────────────────────────────────────────────────

/// Download text via HTTP GET with proper User-Agent.
async fn download_text(url: &str) -> Result<String, ServiceError> {
    let resp = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| ServiceError::Embed(format!("Failed to build HTTP client: {e}")))?
        .get(url)
        .send()
        .await
        .map_err(|e| ServiceError::Embed(format!("HTTP request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(ServiceError::Embed(format!(
            "HTTP {} for {}",
            resp.status(),
            url
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ServiceError::Embed(format!("Failed to read response: {e}")))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: EmbedService must exist as a service type with public operations
    #[test]
    fn embed_service_has_operations() {
        let _ = EmbedService::embed_corpus;
        let _ = EmbedService::parse_config;
    }

    // REQ: EmbedResult carries pipeline output data
    #[test]
    fn embed_result_carries_pipeline_output() {
        let result = EmbedResult {
            author: "hemingway".to_string(),
            purged: 5,
            total_passages: 120,
            centroid_ref: "style:hemingway:centroid".to_string(),
            passage_count: 115,
            centroid_stored: true,
            validation: ValidationConfig {
                centroid_distance_max: 0.5,
                exemplar_count_min: 3,
                exemplar_count_max: 10,
            },
        };
        assert_eq!(result.author, "hemingway");
        assert_eq!(result.purged, 5);
        assert_eq!(result.total_passages, 120);
        assert_eq!(result.passage_count, 115);
        assert!(result.centroid_stored);
    }

    // REQ: CorpusConfig deserializes from YAML
    #[test]
    fn corpus_config_deserializes_from_yaml() {
        let yaml = r#"
author: hemingway
embedding:
  model: qwen3-embedding:0.6b
  dim: 1024
  batch_size: 32
works:
  - title: The Sun Also Rises
    slug: sun-also-rises
    url: https://example.com/sun.txt
foundational_rules:
  - slug: iceberg
    text: Less is more
chunking:
  min_words: 50
  max_words: 300
  sentence_boundary: "."
centroid_entity_ref: style:hemingway:centroid
validation:
  centroid_distance_max: 0.5
  exemplar_count_min: 3
  exemplar_count_max: 10
"#;
        let config: CorpusConfig = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
        assert_eq!(config.author, "hemingway");
        assert_eq!(config.embedding.model, "qwen3-embedding:0.6b");
        assert_eq!(config.embedding.dim, 1024);
        assert_eq!(config.works.len(), 1);
        assert_eq!(config.works[0].slug, "sun-also-rises");
        assert_eq!(config.foundational_rules.len(), 1);
        assert_eq!(config.chunking.min_words, 50);
        assert_eq!(config.validation.centroid_distance_max, 0.5);
    }

    // REQ: ServiceError::Embed is a string sentinel
    #[test]
    fn embed_error_is_string_sentinel() {
        let err = ServiceError::Embed("config parse failed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Embed failed"));
        assert!(msg.contains("config parse failed"));
    }

    // REQ: ValidationConfig is cloneable for EmbedResult
    #[test]
    fn validation_config_is_cloneable() {
        let vc = ValidationConfig {
            centroid_distance_max: 0.5,
            exemplar_count_min: 3,
            exemplar_count_max: 10,
        };
        let vc2 = vc.clone();
        assert_eq!(vc.centroid_distance_max, vc2.centroid_distance_max);
    }

    // REQ: EmbeddingConfig carries model, dim, batch_size
    #[test]
    fn embedding_config_fields() {
        let ec = EmbeddingConfig {
            model: "test-model".to_string(),
            dim: 768,
            batch_size: 16,
        };
        assert_eq!(ec.model, "test-model");
        assert_eq!(ec.dim, 768);
        assert_eq!(ec.batch_size, 16);
    }

    // REQ: Work carries title, slug, url
    #[test]
    fn work_fields() {
        let w = Work {
            title: "Test Work".to_string(),
            slug: "test-work".to_string(),
            url: "https://example.com/test.txt".to_string(),
        };
        assert_eq!(w.title, "Test Work");
        assert_eq!(w.slug, "test-work");
    }

    // REQ: FoundationalRule carries slug and text
    #[test]
    fn foundational_rule_fields() {
        let r = FoundationalRule {
            slug: "iceberg".to_string(),
            text: "Less is more".to_string(),
        };
        assert_eq!(r.slug, "iceberg");
        assert_eq!(r.text, "Less is more");
    }

    // REQ: ChunkingConfig carries min/max words and sentence boundary
    #[test]
    fn chunking_config_fields() {
        let cc = ChunkingConfig {
            min_words: 50,
            max_words: 300,
            sentence_boundary: ".".to_string(),
        };
        assert_eq!(cc.min_words, 50);
        assert_eq!(cc.max_words, 300);
        assert_eq!(cc.sentence_boundary, ".");
    }
}
