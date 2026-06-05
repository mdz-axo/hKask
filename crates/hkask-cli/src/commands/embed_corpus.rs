//! Style corpus embedding command — thin CLI orchestrator
//
//! Reads a corpus config YAML and orchestrates the embedding pipeline
//! through SemanticMemory methods (which are the same code that MCP tools
//! semantic_purge, semantic_chunk, semantic_embed, semantic_centroid call):
//!   - SemanticMemory::purge_by_prefix   — idempotent re-ingest (→ semantic_purge)
//!   - SemanticMemory::chunk_text         — passage chunking (→ semantic_chunk)
//!   - SemanticMemory::store_embedding    — store vectors (→ semantic_embed)
//!   - SemanticMemory::compute_centroid    — style centroid (→ semantic_centroid)
//
//! Manifest: registry/manifests/style-corpus-embed.yaml
//! Skill:    registry/registries/skills/style-corpus-embed.yaml

use crate::cli::EmbedCorpusAction;
use hkask_memory::SemanticMemory;
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_templates::{OkapiConfig, OkapiEmbedding};
use hkask_types::ports::EmbeddingGenerationPort;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct CorpusConfig {
    author: String,
    #[allow(dead_code)]
    author_full: String,
    #[allow(dead_code)]
    style_synthesizer: String,
    embedding: EmbeddingConfig,
    works: Vec<Work>,
    foundational_rules: Vec<FoundationalRule>,
    chunking: ChunkingConfig,
    centroid_entity_ref: String,
    validation: ValidationConfig,
}

#[derive(Debug, Deserialize)]
struct EmbeddingConfig {
    model: String,
    dim: usize,
    #[allow(dead_code)]
    okapi_endpoint: String,
    batch_size: usize,
}

#[derive(Debug, Deserialize)]
struct Work {
    title: String,
    slug: String,
    url: String,
    #[allow(dead_code)]
    year: u32,
    #[allow(dead_code)]
    r#type: String,
    #[allow(dead_code)]
    public_domain: bool,
}

#[derive(Debug, Deserialize)]
struct FoundationalRule {
    #[allow(dead_code)]
    title: String,
    slug: String,
    text: String,
    #[allow(dead_code)]
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct ChunkingConfig {
    min_words: usize,
    max_words: usize,
    #[allow(dead_code)]
    split_on: String,
    sentence_boundary: String,
    #[allow(dead_code)]
    overlap_words: usize,
}

#[derive(Debug, Deserialize)]
struct ValidationConfig {
    centroid_distance_max: f64,
    exemplar_count_min: usize,
    exemplar_count_max: usize,
}

pub fn run(rt: &tokio::runtime::Runtime, action: EmbedCorpusAction) {
    match action {
        EmbedCorpusAction::Run {
            config,
            db,
            passphrase,
            okapi_url,
        } => run_embed(rt, config, db, passphrase, okapi_url),
    }
}

fn run_embed(
    rt: &tokio::runtime::Runtime,
    config_path: PathBuf,
    db_path: PathBuf,
    passphrase: String,
    okapi_url: Option<String>,
) {
    // ── Step 1: Read corpus config (declarative manifest input) ───────────
    let config_str = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        eprintln!(
            "Failed to read corpus config {}: {}",
            config_path.display(),
            e
        );
        std::process::exit(1);
    });
    let config: CorpusConfig = serde_yaml::from_str(&config_str).unwrap_or_else(|e| {
        eprintln!("Failed to parse corpus config YAML: {}", e);
        std::process::exit(1);
    });
    eprintln!(
        "Corpus: {} ({} works, {}d embeddings via {})",
        config.author,
        config.works.len(),
        config.embedding.dim,
        config.embedding.model
    );

    // ── Step 2: Open database + purge (→ semantic_purge) ──────────────────
    let db = Database::open(&db_path.to_string_lossy(), &passphrase).unwrap_or_else(|e| {
        eprintln!("Failed to open database {}: {}", db_path.display(), e);
        std::process::exit(1);
    });
    let conn = db.conn_arc();
    let triple_store = TripleStore::new(Arc::clone(&conn));
    let embedding_store = EmbeddingStore::with_dim(conn, config.embedding.dim);
    let semantic = SemanticMemory::new(triple_store, embedding_store);

    let author_prefix = format!("style:{}:", config.author);
    let purged = semantic
        .purge_by_prefix(&author_prefix)
        .unwrap_or_else(|e| {
            eprintln!("Failed to purge embeddings: {}", e);
            std::process::exit(1);
        });
    if purged > 0 {
        eprintln!(
            "Purged {} existing embeddings for {} (idempotent re-run)",
            purged, config.author
        );
    }

    // ── Step 3: Download texts (web_extract) ──────────────────────────────
    // Download via HTTP. When hkask-mcp-web MCP server is available,
    // this should route through web_extract instead.
    // See: registry/manifests/style-corpus-embed.yaml Step 2
    let cache_dir = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(".cache");
    std::fs::create_dir_all(&cache_dir).unwrap_or_else(|e| {
        eprintln!(
            "Warning: Could not create cache directory {}: {}",
            cache_dir.display(),
            e
        );
    });

    let mut all_passages: Vec<(String, String)> = Vec::new(); // (entity_ref, text)

    for (work_idx, work) in config.works.iter().enumerate() {
        // Rate limit between requests
        if work_idx > 0 {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        let cache_path = cache_dir.join(format!("{}.txt", work.slug));
        let text = if cache_path.exists() {
            eprintln!("Using cached: {}", work.title);
            std::fs::read_to_string(&cache_path).unwrap_or_else(|e| {
                eprintln!("Failed to read cache {}: {}", cache_path.display(), e);
                std::process::exit(1);
            })
        } else {
            eprintln!("Downloading: {}...", work.title);
            let text = rt.block_on(download_text(&work.url)).unwrap_or_else(|e| {
                eprintln!("Failed to download {}: {}", work.title, e);
                std::process::exit(1);
            });
            if let Err(e) = std::fs::write(&cache_path, &text) {
                eprintln!("Warning: Could not cache {}: {}", cache_path.display(), e);
            }
            text
        };

        // ── Step 4: Chunk text (→ semantic_chunk) ───────────────────────
        let cleaned = SemanticMemory::strip_gutenberg_headers(&text);
        let entity_ref_prefix = format!("style:{}:{}", config.author, work.slug);
        let chunks = SemanticMemory::chunk_text(
            &cleaned,
            &entity_ref_prefix,
            config.chunking.min_words,
            config.chunking.max_words,
            &config.chunking.sentence_boundary,
        );
        eprintln!("  Chunked into {} passages", chunks.len());
        all_passages.extend(chunks);
    }

    // ── Step 5: Add foundational rules (semantic_store) ────────────────────
    for rule in &config.foundational_rules {
        let entity_ref = format!("style:{}:rule:{}", config.author, rule.slug);
        all_passages.push((entity_ref, rule.text.clone()));
    }

    eprintln!("Total passages to embed: {}", all_passages.len());

    // ── Step 6: Embed in batches (Okapi embed_sentences) ───────────────────
    let okapi_config = match okapi_url {
        Some(url) => OkapiConfig {
            base_url: url,
            ..OkapiConfig::default()
        },
        None => OkapiConfig::local_dev(),
    };
    let embedder = OkapiEmbedding::with_model(&config.embedding.model, okapi_config)
        .unwrap_or_else(|e| {
            eprintln!("Failed to create Okapi embedding client: {}", e);
            std::process::exit(1);
        });

    let batch_size = config.embedding.batch_size;
    let mut embedded_count = 0;
    for chunk in all_passages.chunks(batch_size) {
        let texts: Vec<&str> = chunk.iter().map(|(_, text)| text.as_str()).collect();
        let vectors = rt
            .block_on(embedder.embed_sentences(&texts))
            .unwrap_or_else(|e| {
                eprintln!("Failed to embed batch: {}", e);
                std::process::exit(1);
            });

        for ((entity_ref, _text), vector) in chunk.iter().zip(vectors.iter()) {
            // ── Step 7: Store embedding (→ semantic_embed) ──────────────
            semantic
                .store_embedding(entity_ref, vector, &config.embedding.model)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to store embedding {}: {}", entity_ref, e);
                    std::process::exit(1);
                });
        }
        embedded_count += chunk.len();
        eprintln!(
            "  Embedded {}/{} passages",
            embedded_count,
            all_passages.len()
        );
    }

    // ── Step 8: Compute and store centroid (→ semantic_centroid) ─────────
    eprintln!("Computing style centroid...");
    let rule_prefix = format!("style:{}:rule:", config.author);
    let centroid_ref = config.centroid_entity_ref.clone();
    let result = semantic
        .compute_centroid(
            &author_prefix,
            &rule_prefix,
            &centroid_ref,
            config.embedding.dim,
            Some(&centroid_ref),
            Some(&config.embedding.model),
        )
        .unwrap_or_else(|e| {
            eprintln!("Failed to compute centroid: {}", e);
            std::process::exit(1);
        });

    eprintln!("Done. Centroid stored as: {}", centroid_ref);
    eprintln!(
        "Centroid computed from {} prose passages (stored: {})",
        result.passage_count, result.stored
    );
    eprintln!("Total passages embedded: {}", all_passages.len());
    eprintln!(
        "Validation config: centroid_distance_max={}, exemplar_count={}..{}",
        config.validation.centroid_distance_max,
        config.validation.exemplar_count_min,
        config.validation.exemplar_count_max,
    );
}

// ========================================================================
// Text download — simple HTTP GET with proper User-Agent
// ========================================================================
// When hkask-mcp-web MCP server is available, replace this with
// a dispatch call to web:extract via the MCP dispatcher.
// See: registry/manifests/style-corpus-embed.yaml Step 2

const USER_AGENT: &str = "hkask-mcp-web/0.22.0";

async fn download_text(url: &str) -> Result<String, String> {
    let resp = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} for {}", resp.status(), url));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok(text)
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::{USER_AGENT, ValidationConfig};
    use hkask_memory::SemanticMemory;

    #[test]
    fn strip_gutenberg_headers_basic() {
        let text = "Header\n*** START OF THIS PROJECT GUTENBERG EBOOK ***\nContent here.\n*** END OF THIS PROJECT GUTENBERG EBOOK ***\nFooter";
        let cleaned = SemanticMemory::strip_gutenberg_headers(text);
        assert!(cleaned.contains("Content here."));
        assert!(!cleaned.contains("Header"));
        assert!(!cleaned.contains("Footer"));
    }

    #[test]
    fn strip_gutenberg_headers_no_markers() {
        let text = "Just some text without markers.";
        let cleaned = SemanticMemory::strip_gutenberg_headers(text);
        assert_eq!(cleaned, text);
    }

    #[test]
    fn chunk_text_preserves_short_paragraphs() {
        let text = "Short one.\n\nShort two.\n\nShort three.";
        let chunks = SemanticMemory::chunk_text(text, "style:test:work", 2, 200, ".!? ");
        assert!(!chunks.is_empty());
    }

    #[test]
    fn chunk_text_splits_long_paragraphs() {
        let long = (0..500)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join(" ");
        let text = format!("Intro.\n\n{}", long);
        let chunks = SemanticMemory::chunk_text(&text, "style:test:work", 5, 200, ".!? ");
        assert!(chunks.len() > 1);
    }

    #[test]
    fn chunk_text_emits_below_min_words() {
        let text = "One.";
        let chunks = SemanticMemory::chunk_text(text, "style:test:work", 50, 200, ".!? ");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "style:test:work:0");
    }

    #[test]
    fn download_text_user_agent_matches_mcp_web() {
        // The User-Agent must match hkask-mcp-web's RawFetchProvider
        // for Gutenberg compatibility and proper access logs.
        assert!(USER_AGENT.starts_with("hkask-mcp-web/"));
    }

    #[test]
    fn validation_config_deserializes() {
        let yaml = "centroid_distance_max: 0.15\nexemplar_count_min: 3\nexemplar_count_max: 7";
        let config: ValidationConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.centroid_distance_max, 0.15);
        assert_eq!(config.exemplar_count_min, 3);
        assert_eq!(config.exemplar_count_max, 7);
    }
}
