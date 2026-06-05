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

#[derive(Debug, Deserialize)]
struct CorpusConfig {
    author: String,
    embedding: EmbeddingConfig,
    works: Vec<Work>,
    foundational_rules: Vec<FoundationalRule>,
    chunking: ChunkingConfig,
    centroid_entity_ref: String,
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
    #[allow(dead_code)]
    sentence_boundary: String,
    #[allow(dead_code)]
    overlap_words: usize,
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
    let mut all_passages: Vec<(String, String)> = Vec::new(); // (entity_ref, text)

    for work in &config.works {
        eprintln!("Downloading: {}...", work.title);
        let text = rt.block_on(download_text(&work.url)).unwrap_or_else(|e| {
            eprintln!("Failed to download {}: {}", work.title, e);
            std::process::exit(1);
        });
        let cleaned = strip_gutenberg_headers(&text);
        let chunks = chunk_text(
            &cleaned,
            &config.author,
            &work.slug,
            config.chunking.min_words,
            config.chunking.max_words,
        );
        eprintln!("  Chunked into {} passages", chunks.len());
        all_passages.extend(chunks);
    }

    // 5. Add foundational rules as special passages
    for rule in &config.foundational_rules {
        let entity_ref = format!("style:{}:{}", config.author, rule.slug);
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

    // 7. Compute and store centroid
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
}

async fn download_text(url: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed for {}: {}", url, e))?;
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;
    Ok(text)
}

fn strip_gutenberg_headers(text: &str) -> String {
    // Strip everything before "*** START OF" and after "*** END OF"
    let start_marker = "*** START OF";
    let end_marker = "*** END OF";

    let start_idx = text
        .find(start_marker)
        .map(|i| text[i..].find('\n').map(|j| i + j + 1).unwrap_or(i))
        .unwrap_or(0);

    let end_idx = text.find(end_marker).unwrap_or(text.len());

    text[start_idx..end_idx].trim().to_string()
}

fn chunk_text(
    text: &str,
    author: &str,
    slug: &str,
    min_words: usize,
    max_words: usize,
) -> Vec<(String, String)> {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty() && p.len() > 20)
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

        if !current_chunk.is_empty() {
            current_chunk.push(' ');
        }
        current_chunk.push_str(para);
        current_word_count += word_count;

        // If single paragraph exceeds max_words, split it
        if current_word_count >= max_words {
            let entity_ref = format!("style:{}:{}:{}", author, slug, chunk_index);
            chunks.push((entity_ref, current_chunk.trim().to_string()));
            chunk_index += 1;
            current_chunk.clear();
            current_word_count = 0;
        }
    }

    // Flush remaining
    if current_word_count >= min_words && !current_chunk.is_empty() {
        let entity_ref = format!("style:{}:{}:{}", author, slug, chunk_index);
        chunks.push((entity_ref, current_chunk.trim().to_string()));
    }

    chunks
}

fn compute_centroid(store: &EmbeddingStore, author: &str, dim: usize) -> Result<Vec<f32>, String> {
    // Retrieve all embeddings for this author by searching with a zero vector,
    // then filter by entity_ref prefix.
    let zero_vec = vec![0.0f32; dim];
    let results = store
        .search(&zero_vec, 10000)
        .map_err(|e| format!("Search failed: {}", e))?;

    let author_prefix = format!("style:{}:", author);
    let matching: Vec<&hkask_types::ports::StoredEmbedding> = results
        .iter()
        .filter(|r| r.embedding.entity_ref.starts_with(&author_prefix))
        .map(|r| &r.embedding)
        .collect();

    if matching.is_empty() {
        return Err(format!("No embeddings found for author: {}", author));
    }

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
