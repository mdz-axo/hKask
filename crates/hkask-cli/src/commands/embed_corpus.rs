//! Style corpus embedding command — thin CLI orchestrator
//!
//! Reads a corpus config YAML and orchestrates the embedding pipeline
//! through existing MCP tools:
//!   - hkask-mcp-web/web_extract      — download texts
//!   - hkask-mcp-semantic/semantic_purge — idempotent re-ingest
//!   - hkask-mcp-semantic/semantic_embed — store passage vectors
//!   - hkask-mcp-semantic/semantic_centroid — compute style centroid
//!
//! Manifest: registry/manifests/style-corpus-embed.yaml
//! Skill:    registry/registries/skills/style-corpus-embed.yaml
//!
//! The chunking logic (Gutenberg header stripping, paragraph splitting,
//! min/max word bounds) is kept here as a local text-processing step —
//! it has no MCP tool equivalent yet and is pure data transformation.

use crate::cli::EmbedCorpusAction;
use hkask_storage::{Database, EmbeddingStore};
use hkask_templates::{OkapiConfig, OkapiEmbedding};
use hkask_types::ports::{EmbeddingGenerationPort, EmbeddingPort};
use serde::Deserialize;
use std::path::PathBuf;

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
    #[allow(dead_code)]
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

    // ── Step 2: Open database + purge (semantic_purge) ─────────────────────
    let db = Database::open(&db_path.to_string_lossy(), &passphrase).unwrap_or_else(|e| {
        eprintln!("Failed to open database {}: {}", db_path.display(), e);
        std::process::exit(1);
    });
    let conn = db.conn_arc();
    let embedding_store = EmbeddingStore::with_dim(conn, config.embedding.dim);

    let author_prefix = format!("style:{}:", config.author);
    let purged = purge_author_embeddings(&embedding_store, &author_prefix, config.embedding.dim);
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

        // ── Step 4: Chunk text (local text processing) ────────────────────
        let cleaned = strip_gutenberg_headers(&text);
        let chunks = chunk_text(
            &cleaned,
            &config.author,
            &work.slug,
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
            // ── Step 7: Store embedding (semantic_embed) ──────────────────
            embedding_store
                .store(entity_ref, vector, &config.embedding.model)
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

    // ── Step 8: Compute and store centroid (semantic_centroid) ────────────
    eprintln!("Computing style centroid...");
    let centroid = compute_centroid(&embedding_store, &config.author, config.embedding.dim)
        .unwrap_or_else(|e| {
            eprintln!("Failed to compute centroid: {}", e);
            std::process::exit(1);
        });
    embedding_store
        .store(
            &config.centroid_entity_ref,
            &centroid,
            &config.embedding.model,
        )
        .unwrap_or_else(|e| {
            eprintln!(
                "Failed to store centroid {}: {}",
                config.centroid_entity_ref, e
            );
            std::process::exit(1);
        });

    eprintln!("Done. Centroid stored as: {}", config.centroid_entity_ref);
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
// Text processing — local chunking (no MCP equivalent yet)
// ========================================================================

fn strip_gutenberg_headers(text: &str) -> String {
    let start_marker = "*** START OF";
    let end_marker = "*** END OF";

    let start = text
        .find(start_marker)
        .and_then(|i| text[i..].find('\n').map(|j| i + j + 1))
        .unwrap_or(0);

    let end = text.find(end_marker).unwrap_or(text.len());

    text[start..end].trim().to_string()
}

fn chunk_text(
    text: &str,
    author: &str,
    work_slug: &str,
    min_words: usize,
    max_words: usize,
    sentence_boundary: &str,
) -> Vec<(String, String)> {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    let mut passages = Vec::new();
    let mut buffer = String::new();
    let mut buffer_words = 0;
    let mut chunk_index = 0;

    for paragraph in &paragraphs {
        let word_count = paragraph.split_whitespace().count();

        if buffer_words + word_count > max_words && buffer_words >= min_words {
            // Flush buffer as a passage
            let entity_ref = format!("style:{}:{}:{}", author, work_slug, chunk_index);
            passages.push((entity_ref, buffer.trim().to_string()));
            chunk_index += 1;
            buffer.clear();
            buffer_words = 0;
        }

        if word_count > max_words {
            // Split long paragraph at sentence boundaries
            if !buffer.is_empty() && buffer_words >= min_words {
                let entity_ref = format!("style:{}:{}:{}", author, work_slug, chunk_index);
                passages.push((entity_ref, buffer.trim().to_string()));
                chunk_index += 1;
                buffer.clear();
                buffer_words = 0;
            }

            let sentences = split_at_sentence_boundaries(paragraph, max_words, sentence_boundary);
            for sentence_group in sentences {
                let sg_words = sentence_group.split_whitespace().count();
                if sg_words >= min_words {
                    let entity_ref = format!("style:{}:{}:{}", author, work_slug, chunk_index);
                    passages.push((entity_ref, sentence_group));
                    chunk_index += 1;
                } else if !buffer.is_empty() {
                    buffer.push(' ');
                    buffer.push_str(&sentence_group);
                    buffer_words += sg_words;
                } else {
                    // Below min_words with empty buffer — emit anyway
                    let entity_ref = format!("style:{}:{}:{}", author, work_slug, chunk_index);
                    passages.push((entity_ref, sentence_group));
                    chunk_index += 1;
                }
            }
        } else {
            if !buffer.is_empty() {
                buffer.push(' ');
            }
            buffer.push_str(paragraph);
            buffer_words += word_count;
        }
    }

    // Flush remaining buffer
    if !buffer.is_empty() {
        let entity_ref = format!("style:{}:{}:{}", author, work_slug, chunk_index);
        passages.push((entity_ref, buffer.trim().to_string()));
    }

    passages
}

fn split_at_sentence_boundaries(text: &str, max_words: usize, boundary_chars: &str) -> Vec<String> {
    let boundary_bytes: Vec<u8> = boundary_chars.bytes().collect();
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.len() <= max_words {
        return vec![text.to_string()];
    }

    let mut groups = Vec::new();
    let mut current = Vec::new();

    for word in &words {
        current.push(*word);

        if current.len() >= max_words {
            let last = current.last().unwrap();
            let ends_with_boundary = last
                .chars()
                .last()
                .map(|c| boundary_bytes.contains(&(c as u8)))
                .unwrap_or(false);

            if ends_with_boundary || current.len() >= max_words * 2 {
                groups.push(current.join(" "));
                current = Vec::new();
            }
        }
    }

    if !current.is_empty() {
        groups.push(current.join(" "));
    }

    groups
}

// ========================================================================
// Embedding store operations — now also available via MCP tools
// ========================================================================
// These functions are kept as direct store calls for the CLI path.
// The MCP equivalents are:
//   purge_author_embeddings → semantic_purge
//   compute_centroid         → semantic_centroid

fn purge_author_embeddings(store: &EmbeddingStore, author_prefix: &str, dim: usize) -> usize {
    let zero_vec = vec![0.0f32; dim];
    let results = match store.search(&zero_vec, 10000) {
        Ok(r) => r,
        Err(_) => return 0,
    };

    let to_delete: Vec<String> = results
        .iter()
        .filter(|r| r.embedding.entity_ref.starts_with(author_prefix))
        .map(|r| r.embedding.entity_ref.clone())
        .collect();

    let mut count = 0;
    for entity_ref in &to_delete {
        if store.delete(entity_ref).is_ok() {
            count += 1;
        }
    }
    count
}

fn compute_centroid(store: &EmbeddingStore, author: &str, dim: usize) -> Result<Vec<f32>, String> {
    let zero_vec = vec![0.0f32; dim];
    let results = store
        .search(&zero_vec, 10000)
        .map_err(|e| format!("Search failed: {}", e))?;

    let author_prefix = format!("style:{}:", author);
    let rule_prefix = format!("style:{}:rule:", author);
    let centroid_ref = format!("style:{}:centroid", author);

    let matching: Vec<&hkask_types::ports::StoredEmbedding> = results
        .iter()
        .filter(|r| {
            let ref_str = &r.embedding.entity_ref;
            ref_str.starts_with(&author_prefix)
                && !ref_str.starts_with(&rule_prefix)
                && ref_str != &centroid_ref
        })
        .map(|r| &r.embedding)
        .collect();

    if matching.is_empty() {
        return Err(format!("No prose embeddings found for author: {}", author));
    }

    eprintln!(
        "  Centroid computed from {} prose passages (excluded rules)",
        matching.len()
    );

    let mut centroid = vec![0.0f32; dim];
    for emb in &matching {
        for (i, v) in emb.vector.iter().enumerate() {
            centroid[i] += v;
        }
    }
    let count = matching.len() as f32;
    for v in centroid.iter_mut() {
        *v /= count;
    }

    Ok(centroid)
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_gutenberg_headers_basic() {
        let text = "Header\n*** START OF THIS PROJECT GUTENBERG EBOOK ***\nContent here.\n*** END OF THIS PROJECT GUTENBERG EBOOK ***\nFooter";
        let cleaned = strip_gutenberg_headers(text);
        assert!(cleaned.contains("Content here."));
        assert!(!cleaned.contains("Header"));
        assert!(!cleaned.contains("Footer"));
    }

    #[test]
    fn strip_gutenberg_headers_no_markers() {
        let text = "Just some text without markers.";
        let cleaned = strip_gutenberg_headers(text);
        assert_eq!(cleaned, text);
    }

    #[test]
    fn chunk_text_preserves_short_paragraphs() {
        let text = "Short one.\n\nShort two.\n\nShort three.";
        let chunks = chunk_text(text, "test", "work", 2, 200, ".!? ");
        assert!(!chunks.is_empty());
    }

    #[test]
    fn chunk_text_splits_long_paragraphs() {
        let long = (0..300)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join(" ");
        let text = format!("Intro.\n\n{}", long);
        let chunks = chunk_text(&text, "test", "work", 5, 200, ".!? ");
        assert!(chunks.len() > 1);
    }

    #[test]
    fn chunk_text_emits_below_min_words() {
        let text = "One.";
        let chunks = chunk_text(text, "test", "work", 50, 200, ".!? ");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "style:test:work:0");
    }

    #[test]
    fn split_at_sentence_boundaries_basic() {
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let groups = split_at_sentence_boundaries(text, 3, ".!? ");
        assert!(!groups.is_empty());
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
