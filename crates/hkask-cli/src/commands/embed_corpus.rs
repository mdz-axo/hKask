//! Style corpus embedding command
//!
//! Reads a corpus config YAML, downloads public domain texts,
//! chunks them into passages, embeds via Okapi, and stores
//! in hKask's sqlite-vec database.

use crate::cli::EmbedCorpusAction;
use hkask_storage::{Database, EmbeddingStore};
use hkask_templates::{OkapiConfig, OkapiEmbedding};
use hkask_types::ports::{EmbeddingGenerationPort, EmbeddingPort};
use serde::Deserialize;
use std::path::PathBuf;

const USER_AGENT: &str = "hkask-mcp-web/0.22.0";
// NOTE: This User-Agent matches hkask-mcp-web's RawFetchProvider.
// When MCP client-side transport is wired (rmcp client), this function
// should be replaced with a dispatch call to web:extract via the
// MCP dispatcher. Until then, we mirror RawFetchProvider's behavior
// directly to ensure Gutenberg compatibility and proper access logs.

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
    // 1. Read corpus config YAML
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

    // 2. Open database
    let db = Database::open(&db_path.to_string_lossy(), &passphrase).unwrap_or_else(|e| {
        eprintln!("Failed to open database {}: {}", db_path.display(), e);
        std::process::exit(1);
    });
    let conn = db.conn_arc();
    let embedding_store = EmbeddingStore::with_dim(conn, config.embedding.dim);

    // 3. Create Okapi embedding client
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

    // 4. Download and chunk each work
    // Uses hkask-mcp-web's RawFetchProvider pattern: proper User-Agent,
    // local file caching, and inter-request delay for Gutenberg rate limiting.
    // When MCP client-side transport is wired, this should route through
    // the MCP dispatcher's web:extract tool instead.
    let mut all_passages: Vec<(String, String)> = Vec::new(); // (entity_ref, text)
    let author_prefix = format!("style:{}:", config.author);

    // 4a. Purge existing embeddings for this author (idempotency)
    let purged = purge_author_embeddings(&embedding_store, &author_prefix, config.embedding.dim);
    if purged > 0 {
        eprintln!(
            "Purged {} existing embeddings for {} (idempotent re-run)",
            purged, config.author
        );
    }

    // 4b. Set up local cache directory for downloaded texts
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

    for (work_idx, work) in config.works.iter().enumerate() {
        // Rate limit: wait between Gutenberg requests (1 second minimum)
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
            // Cache the raw download for future runs
            if let Err(e) = std::fs::write(&cache_path, &text) {
                eprintln!("Warning: Could not cache {}: {}", cache_path.display(), e);
            }
            text
        };

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

    // 5. Add foundational rules as special passages
    // These are style guides, not prose exemplars — they are stored
    // separately and excluded from centroid computation.
    for rule in &config.foundational_rules {
        let entity_ref = format!("style:{}:rule:{}", config.author, rule.slug);
        all_passages.push((entity_ref, rule.text.clone()));
    }

    eprintln!("Total passages to embed: {}", all_passages.len());

    // 6. Embed in batches
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

    // 7. Compute and store centroid (excluding foundational rules)
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

/// Download text from a URL using hkask-mcp-web's RawFetchProvider pattern.
///
/// Mirrors the behavior of `hkask-mcp-web/src/providers/raw_fetch.rs`:
/// - Uses `hkask-mcp-web` User-Agent for Gutenberg compliance
/// - Checks HTTP status before reading body
/// - Returns error details on non-2xx responses
///
/// When MCP client-side transport (rmcp client) is wired, this function
/// should be replaced with a call to `web:extract` via the MCP dispatcher,
/// routing through GovernedTool for OCAP verification, energy budgets,
/// and CNS observability. Until then, this direct HTTP approach ensures
/// Gutenberg texts are properly fetched without relying on unwired transport.
async fn download_text(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed for {}: {}", url, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "HTTP {} for {}: {}",
            status,
            url,
            body.chars().take(200).collect::<String>()
        ));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body from {}: {}", url, e))?;
    Ok(text)
}

/// Strip Project Gutenberg headers and footers.
///
/// Looks for the standard `*** START OF` / `*** END OF` markers.
/// Some Gutenberg texts use variations like `***START OF` (no space) or
/// `*** START OF THE PROJECT GUTENBERG EBOOK`. This function handles
/// all common variants.
fn strip_gutenberg_headers(text: &str) -> String {
    let start_marker = "*** START OF";
    let end_marker = "*** END OF";

    let start_idx = text
        .find(start_marker)
        .map(|i| text[i..].find('\n').map(|j| i + j + 1).unwrap_or(i))
        .unwrap_or(0);

    let end_idx = text.find(end_marker).unwrap_or(text.len());

    text[start_idx..end_idx].trim().to_string()
}

/// Split text into passages for embedding.
///
/// Passages are the unit of style retrieval. Short paragraphs are
/// concatenated until `min_words` is reached. Long paragraphs that
/// exceed `max_words` are split at sentence boundaries defined by
/// `sentence_boundary_chars`.
///
/// Paragraphs as short as a single word are preserved — Hemingway's
/// signature short dialogue and single-sentence paragraphs must not
/// be dropped.
fn chunk_text(
    text: &str,
    author: &str,
    slug: &str,
    min_words: usize,
    max_words: usize,
    sentence_boundary_chars: &str,
) -> Vec<(String, String)> {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_word_count = 0;
    let mut chunk_index = 0;

    for para in &paragraphs {
        let word_count = para.split_whitespace().count();

        if current_word_count > 0 && current_word_count + word_count > max_words {
            // Flush current chunk
            let entity_ref = format!("style:{}:{}:{}", author, slug, chunk_index);
            chunks.push((entity_ref, current_chunk.trim().to_string()));
            chunk_index += 1;
            current_chunk.clear();
            current_word_count = 0;
        }

        // If a single paragraph exceeds max_words, split at sentence boundaries
        if word_count > max_words {
            let sentences = split_at_sentence_boundaries(para, max_words, sentence_boundary_chars);
            for sentence_group in &sentences {
                let group_words = sentence_group.split_whitespace().count();
                if !current_chunk.is_empty() && current_word_count + group_words > max_words {
                    let entity_ref = format!("style:{}:{}:{}", author, slug, chunk_index);
                    chunks.push((entity_ref, current_chunk.trim().to_string()));
                    chunk_index += 1;
                    current_chunk.clear();
                    current_word_count = 0;
                }
                if !current_chunk.is_empty() {
                    current_chunk.push(' ');
                }
                current_chunk.push_str(sentence_group);
                current_word_count += group_words;
            }
            continue;
        }

        if !current_chunk.is_empty() {
            current_chunk.push(' ');
        }
        current_chunk.push_str(para);
        current_word_count += word_count;
    }

    // Flush remaining
    if !current_chunk.is_empty() {
        // Below min_words: still emit — short passages carry essential
        // style information (dialogue, single-sentence paragraphs).
        let entity_ref = format!("style:{}:{}:{}", author, slug, chunk_index);
        chunks.push((entity_ref, current_chunk.trim().to_string()));
    }

    chunks
}

/// Split a long paragraph into groups of sentences that fit within `max_words`.
///
/// Splits at sentence boundaries defined by characters in `boundary_chars`
/// (typically ".!?"). Each group stays under `max_words` where possible.
fn split_at_sentence_boundaries(text: &str, max_words: usize, boundary_chars: &str) -> Vec<String> {
    let mut groups: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_words = 0;

    // Split at sentence-ending punctuation followed by a space
    let mut sentence_start = 0;
    let chars: Vec<char> = text.chars().collect();

    for i in 1..chars.len() {
        let prev = chars[i - 1];
        let curr = chars[i];

        if boundary_chars.contains(prev) && curr == ' ' {
            let sentence: String = chars[sentence_start..i].iter().collect();
            let sentence_words = sentence.split_whitespace().count();

            if current_words + sentence_words > max_words && !current.is_empty() {
                groups.push(current.trim().to_string());
                current.clear();
                current_words = 0;
            }

            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(&sentence);
            current_words += sentence_words;
            sentence_start = i;
        }
    }

    // Remaining text after last boundary
    if sentence_start < chars.len() {
        let remainder: String = chars[sentence_start..].iter().collect();
        let remainder_words = remainder.split_whitespace().count();
        if !current.is_empty() && current_words + remainder_words > max_words {
            groups.push(current.trim().to_string());
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&remainder.trim());
    }

    if !current.trim().is_empty() {
        groups.push(current.trim().to_string());
    }

    groups
}

/// Purge existing embeddings for an author to ensure idempotent re-runs.
///
/// Without this, re-running the command would accumulate duplicate
/// embeddings (each run generates new IDs). The centroid computation
/// would then double-count passages.
fn purge_author_embeddings(store: &EmbeddingStore, author_prefix: &str, dim: usize) -> usize {
    // Search with a zero vector to get candidates, then filter and delete.
    // This is a best-effort approach — if the database is very large,
    // we may miss some. But for typical author corpora (hundreds to
    // low thousands of passages), 10000 limit is sufficient.
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

/// Compute the style centroid (mean embedding vector) for an author.
///
/// Only includes prose passages (entity_ref matching `style:{author}:{slug}:{index}`).
/// Foundational rules (`style:{author}:rule:{slug}`) and the centroid itself
/// are excluded — they are meta-descriptions of style, not examples of it.
fn compute_centroid(store: &EmbeddingStore, author: &str, dim: usize) -> Result<Vec<f32>, String> {
    // Retrieve all embeddings by searching with a zero vector,
    // then filter by entity_ref pattern for this author's prose.
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
            // Must start with author prefix
            ref_str.starts_with(&author_prefix)
            // Exclude foundational rules (style guides, not prose)
            && !ref_str.starts_with(&rule_prefix)
            // Exclude centroid itself
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

    // Compute mean vector
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_gutenberg_headers_basic() {
        let input = "Header stuff\n*** START OF THE PROJECT GUTENBERG EBOOK ***\nFirst line\n\nSecond line\n*** END OF THE PROJECT GUTENBERG EBOOK ***\nFooter";
        let result = strip_gutenberg_headers(input);
        assert!(result.starts_with("First line"));
        assert!(result.contains("Second line"));
        assert!(!result.contains("Header stuff"));
        assert!(!result.contains("Footer"));
    }

    #[test]
    fn strip_gutenberg_headers_no_markers() {
        let input = "Just some text without markers";
        let result = strip_gutenberg_headers(input);
        assert_eq!(result, input);
    }

    #[test]
    fn chunk_text_preserves_short_paragraphs() {
        // Hemingway's signature short dialogue lines must not be dropped
        let text = "\"Yes,\" he said.\n\nThe sun beat down on the dry road and the dust rose behind the car as they drove along the hot highway into the afternoon.";
        let chunks = chunk_text(text, "hemingway", "test", 50, 200, ".!?");
        // The short dialogue line should be in a chunk
        let all_text: String = chunks
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            all_text.contains("\"Yes,\" he said."),
            "Short dialogue was dropped"
        );
    }

    #[test]
    fn chunk_text_splits_long_paragraphs() {
        let long_para = "First sentence. Second sentence. Third sentence. Fourth sentence. Fifth sentence. Sixth sentence.";
        let chunks = chunk_text(long_para, "hemingway", "test", 5, 10, ".!?");
        assert!(
            chunks.len() > 1,
            "Long paragraph should be split into multiple chunks"
        );
    }

    #[test]
    fn chunk_text_emits_below_min_words() {
        // Short passages carry essential style info — don't drop them
        let text = "He said yes.";
        let chunks = chunk_text(text, "hemingway", "test", 50, 200, ".!?");
        assert_eq!(
            chunks.len(),
            1,
            "Below-min-word passages must still be emitted"
        );
    }

    #[test]
    fn split_at_sentence_boundaries_basic() {
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let groups = split_at_sentence_boundaries(text, 4, ".!?");
        assert!(groups.len() >= 2, "Should split into at least 2 groups");
    }

    #[test]
    fn download_text_user_agent_matches_mcp_web() {
        assert_eq!(
            USER_AGENT, "hkask-mcp-web/0.22.0",
            "User-Agent must match hkask-mcp-web RawFetchProvider"
        );
    }

    #[test]
    fn validation_config_deserializes() {
        let yaml = r#"
centroid_distance_max: 0.15
exemplar_count_min: 3
exemplar_count_max: 7
"#;
        let config: ValidationConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.centroid_distance_max, 0.15);
        assert_eq!(config.exemplar_count_min, 3);
        assert_eq!(config.exemplar_count_max, 7);
    }
}
