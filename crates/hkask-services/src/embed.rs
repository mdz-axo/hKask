//! EmbedService — Style corpus embedding pipeline.
//! # REQ: P3 (Generative Space) — full parameter exposure, no hidden settings.

use hkask_memory::SemanticMemory;
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_templates::{OkapiConfig, OkapiEmbedding};

use serde::Deserialize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::ServiceError;

/// Progress callback — called every 3 seconds during embedding.
pub type ProgressFn = Arc<dyn Fn(&EmbedProgress) + Send + Sync>;

/// Live progress state shared between the embed loop and the heartbeat task.
#[derive(Debug, Clone)]
pub struct EmbedProgress {
    pub phase: EmbedPhase,
    pub author: String,
    pub current_work: String,
    pub total_passages: usize,
    pub completed_passages: usize,
    pub elapsed: std::time::Duration,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmbedPhase {
    Parsing,
    Embedding,
    Centroid,
    Done,
}

impl EmbedProgress {
    /// Format as page-turning progress: `TODO [X pages · Y%] ::: DONE [A pages · B%]`
    pub fn format_page_progress(&self) -> String {
        let todo = self.total_passages.saturating_sub(self.completed_passages);
        let todo_pct = if self.total_passages > 0 {
            (todo as f64 / self.total_passages as f64) * 100.0
        } else {
            0.0
        };
        let done_pct = if self.total_passages > 0 {
            (self.completed_passages as f64 / self.total_passages as f64) * 100.0
        } else {
            0.0
        };
        format!(
            "TODO [{todo} pages · {todo_pct:.0}%] ::: DONE [{done} pages · {done_pct:.0}%]",
            todo = todo,
            todo_pct = todo_pct,
            done = self.completed_passages,
            done_pct = done_pct,
        )
    }

    /// Format with phase and work name
    pub fn format_full(&self) -> String {
        match self.phase {
            EmbedPhase::Parsing => format!(
                "[Parsing] {} — {} — {}",
                self.author,
                self.current_work,
                self.format_page_progress()
            ),
            EmbedPhase::Embedding => format!(
                "[Embedding] {} — {}",
                self.author,
                self.format_page_progress()
            ),
            _ => format!(
                "[{}] {} — {}",
                match self.phase {
                    EmbedPhase::Parsing => "Parsing",
                    EmbedPhase::Embedding => "Embedding",
                    EmbedPhase::Centroid => "Centroid",
                    EmbedPhase::Done => "Done",
                },
                self.author,
                if self.current_work.is_empty() {
                    self.format_page_progress()
                } else {
                    format!("{} — {}", self.current_work, self.format_page_progress())
                }
            ),
        }
    }
}

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
    ///
    /// If `progress` is provided, it's called every 3 seconds with live
    /// page-turning progress (phase, work, completed/total).
    pub async fn embed_corpus(
        config_path: &Path,
        db_path: &str,
        db_passphrase: &str,
        okapi_url: Option<&str>,
        cache_dir: Option<&Path>,
        progress: Option<ProgressFn>,
    ) -> Result<EmbedResult, ServiceError> {
        let started = Instant::now();

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

        // ── Shared progress state + heartbeat ──
        let shared = Arc::new(Mutex::new(EmbedProgress {
            phase: EmbedPhase::Parsing,
            author: author.clone(),
            current_work: String::new(),
            total_passages: 0,
            completed_passages: 0,
            elapsed: Duration::ZERO,
        }));
        let _heartbeat = if let Some(ref cb) = progress {
            let shared_hb = Arc::clone(&shared);
            let cb_hb = Arc::clone(cb);
            Some(tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    let p = {
                        let mut p = shared_hb.lock().unwrap();
                        p.elapsed = started.elapsed();
                        p.clone()
                    };
                    if p.phase == EmbedPhase::Done {
                        cb_hb(&p);
                        break;
                    }
                    cb_hb(&p);
                }
            }))
        } else {
            None
        };

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

            {
                let mut p = shared.lock().unwrap();
                p.current_work = work.title.clone();
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

            // Update progress: treat each work as one parsing step
            {
                let mut p = shared.lock().unwrap();
                p.completed_passages = work_idx + 1;
                p.total_passages = config.works.len();
            }
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

        {
            let mut p = shared.lock().unwrap();
            p.phase = EmbedPhase::Embedding;
            p.current_work.clear();
            p.total_passages = all_passages.len();
            p.completed_passages = 0;
        }

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
            {
                let mut p = shared.lock().unwrap();
                p.completed_passages = embedded_count;
            }
            tracing::info!(
                embedded = embedded_count,
                total = all_passages.len(),
                "Embedding progress"
            );
        }

        // Compute centroid
        {
            let mut p = shared.lock().unwrap();
            p.phase = EmbedPhase::Centroid;
        }
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

        {
            let mut p = shared.lock().unwrap();
            p.phase = EmbedPhase::Done;
            p.completed_passages = all_passages.len();
        }

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
