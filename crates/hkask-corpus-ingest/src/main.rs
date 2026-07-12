//! Company Researcher corpus tool — embed + salience for capabilities and research literature.
//!
//! Subcommands:
//!   embed    — Compute Qwen3-Embedding-0.6B vectors and store in memory DB
//!   salience — Tag chunks with investment concepts, compute graph-centrality
//!              salience scores, store as semantic h_mems
//!
//! Usage: cargo run -p hkask-corpus-ingest -- <COMMAND>

mod generate_qa;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use generate_qa::{GenerateQaArgs, run_generate_qa};
use hkask_inference::config::InferenceConfig;
use hkask_inference::embedding_router::EmbeddingRouter;
use hkask_inference::model_constants;
use hkask_memory::SemanticMemory;
use hkask_memory::salience::{self, EntityTags};
use hkask_storage::HMem;
use hkask_types::Visibility;
use hkask_types::visibility::Dimension;
use serde::{Deserialize, Serialize};

/// Qwen3-Embedding-0.6B output dimension.
const EMBEDDING_DIM: usize = 1024;

/// Owner WebID for stored h_mems — use a fixed persona for corpus operations.
const CORPUS_WEBID: &str = "corpus";

// ── QA Generation Parameters ─────────────────────────────────────────

/// Number of Bloom taxonomy QA prompts to generate per qualified chunk.
/// Produces 5 QAs — one per Bloom level (factual/conceptual/analyze/evaluate/create)
/// — for comprehensive cognitive coverage at every tier.
const QA_PROMPTS_PER_CHUNK: usize = 5;

/// Minimum instruction length (chars) for QA quality filtering.
/// Filters out placeholder or malformed instructions.
const MIN_INSTRUCTION_LENGTH: usize = 30;

/// Minimum output length (chars) for QA quality filtering.
/// Set to 50 to preserve factual-level QAs (which naturally produce shorter
/// answers) while still filtering empty/malformed outputs.
const MIN_OUTPUT_LENGTH: usize = 50;

/// Display limit for top-salience chunks in diagnostics.
const TOP_DISPLAY_COUNT: usize = 5;

/// Display limit for top concept frequencies.
const TOP_CONCEPTS_DISPLAY: usize = 20;

#[derive(Debug, Deserialize)]
struct Chunk {
    entity_ref: String,
    source: String,
    text: String,
    /// Word count — used for chunk size distribution reporting at load time.
    #[allow(dead_code)]
    word_count: usize,
}

/// Chunk enriched with investment entity tags and graph salience.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaggedChunk {
    entity_ref: String,
    source: String,
    text: String,
    #[serde(default)]
    concepts: Vec<String>,
    #[serde(default)]
    methods: Vec<String>,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default)]
    salience: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    consolidated_from: Vec<String>,
    /// 5W1H interrogatory dimensions (from ontology tagging).
    #[serde(default)]
    dimensions: Vec<String>,
    /// Dublin Core type (e.g., "bibo:Book").
    #[serde(default)]
    dc_type: String,
    /// Dublin Core subject keywords.
    #[serde(default)]
    dc_subject: Vec<String>,
    /// PKO process concepts.
    #[serde(default)]
    pko_concepts: Vec<String>,
    /// FIBO financial concepts.
    #[serde(default)]
    fibo_concepts: Vec<String>,
    /// GOLEM narrative concepts.
    #[serde(default)]
    golem_concepts: Vec<String>,
    /// Other analytical concepts.
    #[serde(default)]
    other_concepts: Vec<String>,
    /// Expertise level: "practitioner", "analyst", or "researcher".
    #[serde(default)]
    expertise_level: String,
}

#[derive(Parser)]
#[command(name = "corpus-ingest")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compute embeddings and store in memory DB
    Embed(EmbedArgs),
    /// Build QA generation prompts from qualifying chunks
    BuildPrompts(BuildPromptsArgs),
    /// Ingest generated QAs, deduplicate, store as h_mems
    IngestQa(IngestQaArgs),
    /// OCR scanned PDFs using vision model fallback
    Ocr(OcrArgs),
    /// Generate QA pairs from prompts using LLM inference
    GenerateQa(GenerateQaArgs),
    /// Purge QA h_mems and embeddings by entity-ref prefix
    PurgeQa(PurgeQaArgs),
}

#[derive(Parser)]
struct EmbedArgs {
    #[arg(default_value = "corpus/chunks/chunks.jsonl")]
    chunks_jsonl: PathBuf,
    #[arg(short = 'd', long, default_value = "corpus/memory/corpus_memory.db")]
    db_path: String,
    #[arg(short = 'p', long, default_value = "hkask-default-passphrase-2024")]
    passphrase: String,
    #[arg(short = 'b', long, default_value = "50")]
    batch_size: usize,
    #[arg(short = 's', long, default_value = "0")]
    start_at: usize,
    #[arg(short = 'n', long, default_value = "0")]
    max_chunks: usize,
    #[arg(long)]
    dry_run: bool,
}


#[derive(Parser)]
struct BuildPromptsArgs {
    /// Path to tagged chunks with salience scores
    #[arg(default_value = "corpus/chunks/tagged_chunks.jsonl")]
    tagged_jsonl: PathBuf,
    /// Output file for generation prompts (one JSON per line)
    #[arg(short = 'o', long, default_value = "corpus/qa_pairs/prompts.jsonl")]
    output: PathBuf,
    /// Minimum salience to qualify (default: 0.05)
    #[arg(long, default_value = "0.05")]
    min_salience: f32,
    /// Minimum concepts for a chunk to qualify (default: 2)
    #[arg(long, default_value = "2")]
    min_concepts: usize,
    /// Maximum prompts to output (0 = all qualifying)
    #[arg(short = 'n', long, default_value = "0")]
    max_prompts: usize,
    /// Bloom's taxonomy weight distribution: factual,conceptual,analyze,evaluate,create
    #[arg(long, default_value = "1,1,1,1,1")]
    type_distribution: String,
    /// Generate cross-reference prompts: group chunks by shared concepts and produce
    /// synthesis QAs that require consulting multiple passages (RA-DIT method).
    #[arg(long)]
    cross_reference: bool,
    /// Max chunks per cross-reference group (default: 3)
    #[arg(long, default_value = "3")]
    cross_ref_max_chunks: usize,
    /// Max cross-reference prompts to generate (0 = unlimited)
    #[arg(long, default_value = "0")]
    cross_ref_max_prompts: usize,
    /// Path to memory DB for embedding-based context retrieval
    #[arg(short = 'd', long, default_value = "corpus/memory/corpus_memory.db")]
    db_path: String,
    /// Passphrase for the memory DB
    #[arg(short = 'p', long, default_value = "hkask-default-passphrase-2024")]
    passphrase: String,
    /// Number of context passages to retrieve via embedding similarity (default: 3)
    #[arg(long, default_value = "3")]
    context_k: usize,
}

#[derive(Parser)]
struct IngestQaArgs {
    /// Path to generated QAs JSONL (from LLM output)
    #[arg(default_value = "corpus/qa_pairs/generated.jsonl")]
    generated_jsonl: PathBuf,
    /// Output: deduplicated training-ready QAs
    #[arg(short = 'o', long, default_value = "corpus/qa_pairs/train.jsonl")]
    output: PathBuf,
    /// Path to memory DB for h_mem storage
    #[arg(short = 'd', long, default_value = "corpus/memory/corpus_memory.db")]
    db_path: String,
    #[arg(short = 'p', long, default_value = "hkask-default-passphrase-2024")]
    passphrase: String,
    /// Maximum cosine similarity for dedup (0.92 = strict, 0.85 = loose)
    #[arg(long, default_value = "0.92")]
    dedup_threshold: f64,
    /// Dry run — validate and dedup without storing
    #[arg(long)]
    dry_run: bool,
    /// Store QA embedding vectors in EmbeddingStore for KNN similarity search
    #[arg(long)]
    embed_qas: bool,
    /// Dataset name for training_qa_pair h_mems (consumed by training_assemble_dataset)
    #[arg(long, default_value = "corpus-researcher")]
    dataset: String,
}

#[derive(Parser)]
struct PurgeQaArgs {
    /// Entity-ref prefix to purge (e.g. "corpus:qa" for old schema, "training:qa:corpus-researcher" for new)
    #[arg(long, default_value = "corpus:qa")]
    prefix: String,
    /// Path to memory DB
    #[arg(short = 'd', long, default_value = "corpus/memory/corpus_memory.db")]
    db_path: String,
    #[arg(short = 'p', long, default_value = "hkask-default-passphrase-2024")]
    passphrase: String,
}

#[derive(Parser)]
struct OcrArgs {
    /// PDF file or directory of PDFs to OCR
    #[arg(default_value = "corpus/extracted/books")]
    path: PathBuf,
    /// Output directory for extracted text
    #[arg(short = 'o', long, default_value = "corpus/extracted/books")]
    output: PathBuf,
    /// Skip files that already have non-empty txt extraction
    #[arg(long)]
    skip_existing: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();

    // Qwen3-Embedding-0.6B uses 1024 dimensions.
    // Override the default (also 1024) to be explicit about the coupling
    // between this binary and the embedding model. The env var is read by
    // Database::initialize_schema to create the vec0 virtual table.
    // SAFETY: Set before any multi-threaded code runs (called in main before tokio).
    unsafe { std::env::set_var("HKASK_EMBEDDING_DIM", "1024") };

    let cli = Cli::parse();
    match cli.command {
        Command::Embed(args) => run_embed(args).await,
        Command::BuildPrompts(args) => run_build_prompts(args),
        Command::IngestQa(args) => run_ingest_qa(args).await,
        Command::Ocr(args) => run_ocr(args).await,
        Command::GenerateQa(args) => run_generate_qa(args).await,
        Command::PurgeQa(args) => run_purge_qa(args).await,
    }
}

// ── Embed ──────────────────────────────────────────────────────────

async fn run_embed(args: EmbedArgs) -> Result<(), Box<dyn std::error::Error>> {
    let emb_model = model_constants::embedding_model();

    println!("=== Embed ===");
    println!("  Chunks: {}", args.chunks_jsonl.display());
    println!("  DB:     {}", args.db_path);
    println!("  Model:  {} ({} dim)", emb_model, EMBEDDING_DIM);
    println!("  Batch:  {}", args.batch_size);
    println!();

    let all_chunks = read_chunks(&args.chunks_jsonl)?;
    println!("  Loaded: {} chunks", all_chunks.len());

    let end = if args.max_chunks > 0 {
        (args.start_at + args.max_chunks).min(all_chunks.len())
    } else {
        all_chunks.len()
    };
    let chunks_to_embed = &all_chunks[args.start_at..end];
    println!(
        "  To embed: {} (indices {}-{})",
        chunks_to_embed.len(),
        args.start_at,
        end - 1
    );

    if args.dry_run {
        println!("  Dry run. Would embed {} chunks.", chunks_to_embed.len());
        return Ok(());
    }

    let semantic = SemanticMemory::open(&args.db_path, &args.passphrase, EMBEDDING_DIM)?;
    println!(
        "  Existing embeddings: {}",
        semantic.embedding_count().unwrap_or(0)
    );

    let inf_cfg = InferenceConfig::from_env();
    let embedder = EmbeddingRouter::new(inf_cfg);

    let total = chunks_to_embed.len();
    let mut embedded = 0usize;
    let mut failed = 0usize;
    let mut batch_idx = 0usize;

    'batches: for batch in chunks_to_embed.chunks(args.batch_size) {
        let texts: Vec<&str> = batch.iter().map(|c| c.text.as_str()).collect();
        let vectors = loop {
            match embedder.embed_sentences(&emb_model, &texts).await {
                Ok(v) => break v,
                Err(e) => {
                    batch_idx += 1;
                    let backoff_secs = 2u64.pow(batch_idx.min(6).try_into().unwrap()) * 10;
                    let backoff = std::time::Duration::from_secs(backoff_secs);
                    eprintln!(
                        "  ERROR embedding batch (attempt {}): {} — retrying in {:?}",
                        batch_idx, e, backoff
                    );
                    if batch_idx >= 5 {
                        eprintln!(
                            "  GIVING UP after 5 retries on this batch — {} chunks failed",
                            batch.len()
                        );
                        failed += batch.len();
                        continue 'batches;
                    }
                    tokio::time::sleep(backoff).await;
                }
            }
        };
        batch_idx = 0; // reset backoff on success
        for (chunk, vector) in batch.iter().zip(vectors.iter()) {
            match semantic.store_embedding(&chunk.entity_ref, vector, &emb_model) {
                Ok(_) => {
                    let webid = hkask_types::WebID::from_persona(CORPUS_WEBID.as_bytes());
                    let text_h_mem = HMem::new(
                        &chunk.entity_ref,
                        "text",
                        serde_json::json!(chunk.text),
                        webid,
                    )
                    .with_visibility(Visibility::Public)
                    .with_confidence(1.0);
                    let provenance_h_mem = HMem::new(
                        &chunk.entity_ref,
                        "corpus_provenance",
                        serde_json::json!({
                            "source": chunk.source,
                            "word_count": chunk.word_count,
                            "embedding_model": emb_model,
                            "embedding_dimensions": vector.len(),
                            "ingest_kind": "corpus_chunk",
                        }),
                        webid,
                    )
                    .with_visibility(Visibility::Public)
                    .with_confidence(1.0);
                    match (semantic.store(text_h_mem), semantic.store(provenance_h_mem)) {
                        (Ok(()), Ok(())) => embedded += 1,
                        (text_result, provenance_result) => {
                            eprintln!(
                                "  ERROR storing provenance for {}: text={:?}, provenance={:?}",
                                chunk.entity_ref,
                                text_result.err(),
                                provenance_result.err()
                            );
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  ERROR storing {}: {}", chunk.entity_ref, e);
                    failed += 1;
                }
            }
        }
        println!(
            "  [{}/{}] {} embedded, {} failed",
            embedded + failed,
            total,
            embedded,
            failed
        );
    }

    println!("\n=== Done ===");
    println!("  Embedded: {}", embedded);
    println!("  Failed:   {}", failed);
    println!("  Store:    {}", semantic.embedding_count().unwrap_or(0));
    Ok(())
}

// ── Salience ────────────────────────────────────────────────────────


// ── Shared helpers ──────────────────────────────────────────────────

fn read_chunks(path: &PathBuf) -> Result<Vec<Chunk>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}

fn read_tagged_chunks(path: &PathBuf) -> Result<Vec<TaggedChunk>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}

// ── Build Prompts ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum QaType {
    Factual,
    Conceptual,
    Analyze,
    Evaluate,
    Create,
}

impl QaType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Factual => "factual",
            Self::Conceptual => "conceptual",
            Self::Analyze => "analyze",
            Self::Evaluate => "evaluate",
            Self::Create => "create",
        }
    }
}

fn parse_type_distribution(spec: &str) -> Vec<QaType> {
    let nums: Vec<usize> = spec
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let types = [
        QaType::Factual,
        QaType::Conceptual,
        QaType::Analyze,
        QaType::Evaluate,
        QaType::Create,
    ];
    let mut result = Vec::new();
    for (i, &count) in nums.iter().enumerate() {
        for _ in 0..count {
            if i < types.len() {
                result.push(types[i]);
            }
        }
    }
    result
}

fn qa_type_instruction(qt: QaType) -> &'static str {
    match qt {
        QaType::Factual => {
            "Extract ONE fact from passage. Generate FACTUAL question: identify specific capabilities, resources, metrics from passage. Direct answer from text. No explanation. No elaboration. Question asks what system has or achieves. Answer states fact. Keep output concise — caveman mode: drop filler, articles, hedging. Preserve all technical accuracy."
        }
        QaType::Conceptual => {
            "Generate a CONCEPTUAL question: explain the mechanisms linking capabilities to outcomes. How does a described capability theoretically translate into performance? What models or frameworks explain the capability-performance relationship?"
        }
        QaType::Analyze => {
            "Generate an ANALYZE question: compare capability-performance relationships across contexts. Identify patterns in where gaps emerge. Distinguish structural factors from situational ones. Break down the components of a system to understand how they interact."
        }
        QaType::Evaluate => {
            "Generate an EVALUATE question: assess explanations for capability-performance gaps. Critique the evidence. Judge whether claimed causal links are supported. Determine if an identified gap is economically significant or merely measurement noise. Consider what alternative explanations need to be ruled out."
        }
        QaType::Create => {
            "Generate a CREATE question: design interventions to close capability-performance gaps. Synthesize multi-domain strategies. Formulate testable hypotheses about what would happen if specific capabilities were deployed differently. Integrate concepts from the passage into a novel analytical framework."
        }
    }
}

fn run_build_prompts(args: BuildPromptsArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Build Prompts ===");
    let all_chunks = read_tagged_chunks(&args.tagged_jsonl)?;
    println!("  Loaded: {} tagged chunks", all_chunks.len());
    // No qualifying filter — all chunks generate QAs.
    // The old filter (salience >= 0.05, concepts >= 2) excluded 76% of the corpus
    // because it depended on the deleted hardcoded investment_concepts.rs tagger.
    // With ontology-anchored tagging, every chunk has at least 5W1H dimensions.
    let qualifying: Vec<&TaggedChunk> = all_chunks.iter().collect();
    println!("  All chunks: {} (no filter — ontology tagging ensures coverage)", qualifying.len());
    let type_rotation = parse_type_distribution(&args.type_distribution);
    let limit = if args.max_prompts > 0 {
        args.max_prompts.min(qualifying.len())
    } else {
        qualifying.len()
    };
    let mut sorted: Vec<&&TaggedChunk> = qualifying.iter().collect();
    sorted.sort_by(|a, b| {
        b.salience
            .partial_cmp(&a.salience)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // ── Scaffold: bulk-load embeddings from DB for in-memory KNN ──────────
    // We load ALL embeddings once, then do KNN searches in Rust via dot product.
    // This avoids 6,596 individual SQL-based sqlite-vec KNN queries (each doing
    // a linear scan of 37K vectors with blob decode + JOIN overhead).
    let text_map: std::collections::HashMap<&str, &str> = all_chunks
        .iter()
        .map(|c| (c.entity_ref.as_str(), c.text.as_str()))
        .collect();
    let source_map: std::collections::HashMap<&str, &str> = all_chunks
        .iter()
        .map(|c| (c.entity_ref.as_str(), c.source.as_str()))
        .collect();

    let emb_map: std::collections::HashMap<String, Vec<f32>> = match SemanticMemory::open(&args.db_path, &args.passphrase, EMBEDDING_DIM) {
        Ok(sem) => {
            let count = sem.embedding_count().unwrap_or(0);
            println!("  Memory DB: {} ({} embeddings)", args.db_path, count);
            match sem.embeddings_by_prefix("corpus:researcher:") {
                Ok(embs) => {
                    let map: std::collections::HashMap<String, Vec<f32>> = embs
                        .into_iter()
                        .map(|(er, v)| {
                            let mag = (v.iter().map(|x| x * x).sum::<f32>()).sqrt();
                            let norm: Vec<f32> = if mag > 0.0 {
                                v.iter().map(|x| x / mag).collect()
                            } else {
                                v
                            };
                            (er, norm)
                        })
                        .collect();
                    println!("  Bulk-loaded {} normalized embeddings for in-memory KNN", map.len());
                    map
                }
                Err(e) => {
                    println!("  Warning: embedding query failed — scaffold disabled: {e}");
                    std::collections::HashMap::new()
                }
            }
        }
        Err(e) => {
            println!("  Warning: could not open memory DB — scaffold disabled: {e}");
            std::collections::HashMap::new()
        }
    };

    // Group embeddings by source file for scoped KNN search.
    // Context passages from unrelated sources aren't useful — the LLM needs
    // passages from the same book to see how a concept appears in different
    // chapters. Reduces KNN candidates from 37K to ~265 per query.
    let mut emb_by_source: std::collections::HashMap<&str, Vec<(&String, &Vec<f32>)>> =
        std::collections::HashMap::new();
    for chunk in &all_chunks {
        if let Some(v) = emb_map.get(&chunk.entity_ref) {
            emb_by_source
                .entry(chunk.source.as_str())
                .or_default()
                .push((&chunk.entity_ref, v));
        }
    }
    println!("  Source-grouped KNN: {} source groups", emb_by_source.len());

    // ── Scaffold: build concept graph (concept → chunk_count) ──────────────
    let mut concept_connections: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for chunk in &all_chunks {
        for concept in &chunk.concepts {
            *concept_connections.entry(concept.as_str()).or_default() += 1;
        }
    }

    let mut out = String::new();
    let mut ti = 0usize;
    for tc in sorted.iter().take(limit) {
        // ── Scaffold: in-memory KNN search on bulk-loaded embeddings ────────
        // All 37K embeddings are already in emb_map (normalized). Computing KNN
        // via dot product in Rust is ~100x faster than SQL-based sqlite-vec
        // searches (which do linear scans with blob decode + JOIN per query).
        // For 37K vectors, brute-force in-memory is optimal — no ANN index
        // construction overhead, no approximation error.
        // Reference: Johnson et al. (2021) "Billion-scale similarity search with
        // FAISS" — for N < 100K, brute-force is faster than indexed search.
        let context_passages: Vec<serde_json::Value> = {
            let query_vec = match emb_map.get(&tc.entity_ref) {
                Some(v) => v.as_slice(),
                None => &[],
            };
            if query_vec.is_empty() {
                Vec::new()
            } else {
                // Source-scoped KNN: only search within the same source file.
                // This is ~140x faster than scanning all 37K embeddings and
                // produces more useful context (same-book passages).
                let k = args.context_k;
                let candidates = emb_by_source
                    .get(tc.source.as_str())
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                let mut scored: Vec<(&String, f32)> = candidates
                    .iter()
                    .filter(|(er, _)| er.as_str() != tc.entity_ref)
                    .map(|(er, v)| {
                        let dot: f32 = query_vec
                            .iter()
                            .zip(v.iter())
                            .map(|(a, b)| a * b)
                            .sum();
                        (*er, dot)
                    })
                    .collect();
                let top_k: Vec<(&String, f32)> = if scored.len() > k {
                    // Partial selection: partition around the k-th largest element
                    scored.select_nth_unstable_by(k.saturating_sub(1), |a, b| {
                        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    // Sort only the top K (much smaller than full array)
                    scored[..k].sort_by(|a, b| {
                        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    scored.into_iter().take(k).collect()
                } else {
                    scored.sort_by(|a, b| {
                        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    scored.into_iter().collect()
                };
                top_k.into_iter().map(|(er, sim)| {
                        let text = text_map.get(er.as_str()).copied().unwrap_or("");
                        let source = source_map.get(er.as_str()).copied().unwrap_or(er);
                        serde_json::json!({
                            "source": source,
                            "similarity": sim,
                            "text": text,
                        })
                    })
                    .collect()
            }
        };

        // ── Scaffold: build concept graph for this chunk ───────────────────
        // Issue 2 fix: concept salience = connected_chunks (corpus-wide centrality),
        // not the chunk's salience. A concept connected to 50 chunks is more central
        // than one connected to 3, regardless of which chunk we're processing.
        let concept_graph: Vec<serde_json::Value> = tc
            .concepts
            .iter()
            .map(|concept| {
                let connected = concept_connections
                    .get(concept.as_str())
                    .copied()
                    .unwrap_or(1);
                serde_json::json!({
                    "concept": concept,
                    "connected_chunks": connected,
                })
            })
            .collect();

        // Format scaffold as human-readable text for the system prompt.
        let context_text = if context_passages.is_empty() {
            "(none — no embedding context available)".to_string()
        } else {
            context_passages
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let source = p["source"].as_str().unwrap_or("?");
                    let sim = p["similarity"].as_f64().unwrap_or(0.0);
                    let text = p["text"].as_str().unwrap_or("");
                    let truncated = if text.len() > 2000 {
                        let mut end = 2000;
                        while end > 0 && !text.is_char_boundary(end) {
                            end -= 1;
                        }
                        &text[..end]
                    } else {
                        text
                    };
                    format!(
                        "[{}] Source: {}, Similarity: {:.2}\n    {}",
                        i + 1,
                        source,
                        sim,
                        truncated
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        let concept_graph_text = if concept_graph.is_empty() {
            "(none)".to_string()
        } else {
            concept_graph
                .iter()
                .map(|g| {
                    let concept = g["concept"].as_str().unwrap_or("?");
                    let connected = g["connected_chunks"].as_u64().unwrap_or(0);
                    format!("- {} (connected to {} chunks)", concept, connected)
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        // Generate QA_PROMPTS_PER_CHUNK QAs per chunk at consecutive Bloom levels
        for offset in 0..QA_PROMPTS_PER_CHUNK {
            let qt = type_rotation[(ti + offset) % type_rotation.len()];
            // 5W1H / Dublin Core / PKO ontological context with ontology-tagged fields
            let dimensions_str = if tc.dimensions.is_empty() {
                "what".to_string()
            } else {
                tc.dimensions.join(", ")
            };
            let expertise = if tc.expertise_level.is_empty() {
                "analyst"
            } else {
                tc.expertise_level.as_str()
            };
            let dc_type = if tc.dc_type.is_empty() { "bibo:Document" } else { tc.dc_type.as_str() };
            let dc_subject = if tc.dc_subject.is_empty() { &tc.concepts } else { &tc.dc_subject };

            let ontology_text = if tc.consolidated_from.is_empty() {
                format!(
                    "5W1H: passage answers interrogatories [{}]. QA at {} level for {} expertise.\nDublin Core: dcterms:source={}, dcterms:type={}, dcterms:subject={}\nPKO: pko:producedBy=corpus QA generation pipeline\nDomain concepts: FIBO={}, GOLEM={}, PKO={}, other={}",
                    dimensions_str, qt.as_str(), expertise,
                    tc.source, dc_type, dc_subject.join(", "),
                    tc.fibo_concepts.join(", "), tc.golem_concepts.join(", "),
                    tc.pko_concepts.join(", "), tc.other_concepts.join(", ")
                )
            } else {
                format!(
                    "5W1H: passage answers interrogatories [{}]. QA at {} level for {} expertise.\nDublin Core: dcterms:source={}, dcterms:type={}, dcterms:subject={}\nPKO: pko:wasExtractedFrom={} original passages (consolidation preserved all unique information), pko:producedBy=corpus QA generation pipeline\nDomain concepts: FIBO={}, GOLEM={}, PKO={}, other={}",
                    dimensions_str, qt.as_str(), expertise,
                    tc.source, dc_type, dc_subject.join(", "),
                    tc.consolidated_from.len(),
                    tc.fibo_concepts.join(", "), tc.golem_concepts.join(", "),
                    tc.pko_concepts.join(", "), tc.other_concepts.join(", ")
                )
            };

            let system = format!(
                "You are a Capabilities Researcher training data generator. Given a primary passage from capabilities and research literature, generate ONE question-answer pair. Calibrate question depth to the expertise level indicated in the ontological context below.\n\n{}\n\nOutput JSON with: instruction, output, type, difficulty (2-5), concepts (array), source, chunk_ref (the chunk reference provided below), evidence_quotes (array of exact supporting quotations copied from the passage).\n\nGround the answer in the primary passage. Do not invent facts.\n\n## Ontological Context\n{}\n\n## Context Passages (retrieved via embedding similarity)\n{}\n\n## Concept Graph (salience-weighted)\n{}",
                qa_type_instruction(qt),
                ontology_text,
                context_text,
                concept_graph_text
            );
            let prompt = serde_json::json!({
                "chunk_ref": tc.entity_ref,
                "source": tc.source,
                "concepts": tc.concepts,
                "salience": tc.salience,
                "qa_type": qt.as_str(),
                "system": system,
                "user": format!("Generate a {} QA pair from this passage:\n\n---\n{}\n---\n\nConcepts: {}\n\nInclude this chunk_ref in your output: {}", qt.as_str(), tc.text, tc.concepts.join(", "), tc.entity_ref),
            });
            out.push_str(&serde_json::to_string(&prompt)?);
            out.push('\n');
        }
        ti += QA_PROMPTS_PER_CHUNK;
    }
    std::fs::write(&args.output, &out)?;
    println!("  Wrote: {} prompts to {}", ti, args.output.display());

    if args.cross_reference {
        let cr_count = build_cross_reference_prompts(&qualifying, &args);
        println!("  Cross-reference: {} prompts appended", cr_count);
    }

    Ok(())
}

// ── Ingest QAs ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GeneratedQa {
    instruction: String,
    output: String,
    #[serde(rename = "type", default)]
    qa_type: String,
    #[serde(default)]
    difficulty: u8,
    #[serde(default)]
    concepts: Vec<String>,
    #[serde(default)]
    source: String,
    /// Provenance: which chunk this QA was generated from (for traceability).
    #[serde(default)]
    chunk_ref: Option<String>,
    /// Exact quotations from the source chunk that support the answer.
    #[serde(default)]
    evidence_quotes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GeneratedQaEnvelope {
    chunk_ref: String,
    source: String,
    qa_type: String,
    response: GeneratedQaResponse,
}

#[derive(Debug, Deserialize)]
struct GeneratedQaResponse {
    instruction: String,
    output: String,
    #[serde(rename = "type", default)]
    qa_type: String,
    #[serde(default)]
    difficulty: u8,
    #[serde(default)]
    concepts: Vec<String>,
    #[serde(default)]
    evidence_quotes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GeneratedQaRecord {
    Flat(GeneratedQa),
    Envelope(GeneratedQaEnvelope),
}

impl GeneratedQaRecord {
    fn into_generated_qa(self) -> GeneratedQa {
        match self {
            Self::Flat(qa) => qa,
            Self::Envelope(envelope) => GeneratedQa {
                instruction: envelope.response.instruction,
                output: envelope.response.output,
                qa_type: if envelope.response.qa_type.is_empty() {
                    envelope.qa_type
                } else {
                    envelope.response.qa_type
                },
                difficulty: envelope.response.difficulty,
                concepts: envelope.response.concepts,
                source: envelope.source,
                chunk_ref: Some(envelope.chunk_ref),
                evidence_quotes: envelope.response.evidence_quotes,
            },
        }
    }
}

fn has_admissible_qa_provenance(qa: &GeneratedQa) -> bool {
    !qa.source.trim().is_empty()
        && qa
            .chunk_ref
            .as_ref()
            .is_some_and(|chunk_ref| !chunk_ref.trim().is_empty())
}

/// Check that at least one evidence quote from the QA appears verbatim in the
/// referenced chunk text. This enforces provenance: the QA's evidence must be
/// an exact quote from the source chunk, not a paraphrase or fabrication.
fn has_source_supported_evidence(
    qa: &GeneratedQa,
    chunks: &std::collections::HashMap<String, String>,
) -> bool {
    let Some(ref chunk_ref) = qa.chunk_ref else {
        return false;
    };
    let Some(chunk_text) = chunks.get(chunk_ref) else {
        return false;
    };
    !qa.evidence_quotes.is_empty()
        && qa
            .evidence_quotes
            .iter()
            .any(|quote| chunk_text.contains(quote.as_str()))
}

fn build_cross_reference_prompts(qualifying: &[&TaggedChunk], args: &BuildPromptsArgs) -> usize {
    use std::collections::HashMap;

    println!("  Building cross-reference prompts...");

    // Group chunks by shared concepts: concept → list of (chunk, salience)
    let mut concept_groups: HashMap<&str, Vec<&&TaggedChunk>> = HashMap::new();
    for chunk in qualifying {
        for concept in &chunk.concepts {
            concept_groups
                .entry(concept.as_str())
                .or_default()
                .push(chunk);
        }
    }

    // Keep only groups with 2+ chunks, sort each by salience descending
    let mut groups: Vec<(&str, Vec<&&TaggedChunk>)> = concept_groups
        .into_iter()
        .filter(|(_, chunks)| chunks.len() >= 2)
        .collect();
    groups.sort_by(|(_, a), (_, b)| {
        b.len().cmp(&a.len()).then_with(|| {
            let a_max = a.iter().map(|c| c.salience).fold(0.0f32, f32::max);
            let b_max = b.iter().map(|c| c.salience).fold(0.0f32, f32::max);
            b_max
                .partial_cmp(&a_max)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    println!("  Found {} concept groups with 2+ chunks", groups.len());

    let cross_ref_types = ["comparative", "diagnostic", "causal", "applied"];
    let mut out = String::new();
    let mut count = 0;

    for (concept, chunks) in groups.iter() {
        if args.cross_ref_max_prompts > 0 && count >= args.cross_ref_max_prompts {
            break;
        }

        // Sort chunks by salience, then take top-N
        let mut sorted_chunks: Vec<&&TaggedChunk> = chunks.to_vec();
        sorted_chunks.sort_by(|a, b| {
            b.salience
                .partial_cmp(&a.salience)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top: Vec<&&TaggedChunk> = sorted_chunks
            .into_iter()
            .take(args.cross_ref_max_chunks)
            .collect();

        let qt = cross_ref_types[count % cross_ref_types.len()];

        // Build passage block with source attribution
        let mut passages = String::new();
        for (i, chunk) in top.iter().enumerate() {
            passages.push_str(&format!(
                "[Passage {} — source: {}, salience: {:.2}]\n{}\n\n",
                i + 1,
                chunk.entity_ref,
                chunk.salience,
                chunk.text
            ));
        }

        let chunk_refs: Vec<&str> = top.iter().map(|c| c.entity_ref.as_str()).collect();

        let prompt = serde_json::json!({
            "chunk_refs": chunk_refs,
            "concept": concept,
            "qa_type": qt,
            "cross_reference": true,
            "system": format!(
                "You are a Company Research Analyst synthesizing knowledge across multiple sources.\n\nGiven {} passages from different parts of the capabilities and research literature, all related to the concept '{}', generate ONE question-answer pair that requires synthesizing information from MULTIPLE passages.\n\nThe question should test cross-reference understanding:\n- comparative: compare/contrast perspectives across passages\n- diagnostic: identify patterns or tensions that emerge only when reading multiple passages\n- causal: trace how ideas connect or influence each other across sources\n\nThe answer MUST cite which passages it draws from (e.g., 'Per Passage 1, ... while Passage 3 notes ...').\n\nOutput JSON with: instruction, output, type, difficulty (3-5), concepts (list including '{}'), source (all sources), chunk_ref (comma-separated chunk references).\n\nDo not invent facts. Ground every claim in the passages.",
                top.len(), concept, concept
            ),
            "user": format!("Generate a {} cross-reference QA pair from these {} passages about '{}':\n\n{}\nConcepts: {}", qt, top.len(), concept, passages, concept)
        });

        match serde_json::to_string(&prompt) {
            Ok(s) => {
                out.push_str(&s);
                out.push('\n');
                count += 1;
            }
            Err(e) => eprintln!("  WARN: cross-ref prompt serialization failed: {e}"),
        }
    }

    // Append to output file
    if count > 0 {
        let mut existing = std::fs::read_to_string(&args.output).unwrap_or_default();
        existing.push_str(&out);
        std::fs::write(&args.output, &existing).ok();
    }

    count
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a = (a.iter().map(|x| x * x).sum::<f32>()).sqrt();
    let mag_b = (b.iter().map(|x| x * x).sum::<f32>()).sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
    }
}

// ── K-means clustering for SemDeDup ──────────────────────────────────
//
// Simple Lloyd's algorithm on pre-normalized vectors. Uses dot product
// (cosine similarity on normalized vectors) for centroid assignment.
// Returns clusters as Vecs of indices into `embedded_indices`.
//
// Reference: SemDeDup (Abbas et al., 2023) — cluster embeddings with K-means,
// then deduplicate only within clusters. Reduces O(N²) to O(N²/K).

fn kmeans_cluster(
    vectors: &[Vec<f32>],
    embedded_indices: &[usize],
    k: usize,
    max_iter: usize,
) -> Vec<Vec<usize>> {
    let n = embedded_indices.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }
    let dim = vectors[embedded_indices[0]].len();

    // Initialize centroids: evenly spaced selection from the data points
    let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(k);
    let step = n / k;
    for i in 0..k {
        let idx = embedded_indices[(i * step).min(n - 1)];
        centroids.push(vectors[idx].clone());
    }

    let mut assignments = vec![0usize; n];

    for _iter in 0..max_iter {
        // Assignment step: assign each point to nearest centroid (max dot product)
        let mut changed = false;
        for (i, &data_idx) in embedded_indices.iter().enumerate() {
            let v = &vectors[data_idx];
            let mut best_cluster = 0;
            let mut best_sim = f32::MIN;
            for (c, centroid) in centroids.iter().enumerate() {
                let dot: f32 = v.iter().zip(centroid.iter()).map(|(a, b)| a * b).sum();
                if dot > best_sim {
                    best_sim = dot;
                    best_cluster = c;
                }
            }
            if assignments[i] != best_cluster {
                assignments[i] = best_cluster;
                changed = true;
            }
        }

        if !changed {
            break;
        }

        // Update step: compute new centroids as mean of assigned points
        let mut new_centroids = vec![vec![0.0f32; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, &data_idx) in embedded_indices.iter().enumerate() {
            let c = assignments[i];
            counts[c] += 1;
            for (d, &val) in vectors[data_idx].iter().enumerate() {
                new_centroids[c][d] += val;
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                // Normalize centroid (so dot product = cosine similarity)
                let mag = (new_centroids[c].iter().map(|x| x * x).sum::<f32>()).sqrt();
                if mag > 0.0 {
                    for val in &mut new_centroids[c] {
                        *val /= mag;
                    }
                }
            } else {
                // Empty cluster: reinitialize from a data point
                let fallback = embedded_indices[c % n];
                new_centroids[c] = vectors[fallback].clone();
            }
        }
        centroids = new_centroids;
    }

    // Group indices by cluster
    let mut clusters: Vec<Vec<usize>> = vec![Vec::new(); k];
    for (i, &data_idx) in embedded_indices.iter().enumerate() {
        clusters[assignments[i]].push(data_idx);
    }

    // Remove empty clusters
    clusters.retain(|c| !c.is_empty());
    clusters
}

async fn run_ingest_qa(args: IngestQaArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Ingest QAs ===");
    let content = std::fs::read_to_string(&args.generated_jsonl)?;
    let mut malformed = 0usize;
    let qas: Vec<GeneratedQa> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(
            |line| match serde_json::from_str::<GeneratedQaRecord>(line) {
                Ok(record) => Some(record.into_generated_qa()),
                Err(error) => {
                    malformed += 1;
                    eprintln!("  WARN: rejected malformed QA record: {error}");
                    None
                }
            },
        )
        .collect();
    println!("  Parsed: {} ({} malformed rejected)", qas.len(), malformed);

    let filtered: Vec<&GeneratedQa> = qas
        .iter()
        .filter(|q| {
            q.instruction.len() >= MIN_INSTRUCTION_LENGTH
                && q.output.len() >= MIN_OUTPUT_LENGTH
                && !q.qa_type.is_empty()
                && has_admissible_qa_provenance(q)
        })
        .collect();
    println!(
        "  Quality filter: {} (removed {} malformed, short, empty, or untraceable)",
        filtered.len(),
        qas.len() - filtered.len()
    );

    let use_embed = args.dedup_threshold < 1.0;
    let emb_model = model_constants::embedding_model();
    // Unified dedup tracking: each entry is (normalized_embedding_or_none, &GeneratedQa)
    // This fixes the index divergence bug where emb_kept and deduped had different orderings.
    let mut deduped: Vec<(Option<Vec<f32>>, &GeneratedQa)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut no_embedding_count = 0usize;

    if use_embed {
        let inf_cfg = InferenceConfig::from_env();
        let embedder = EmbeddingRouter::new(inf_cfg);
        let instructions: Vec<&str> = filtered.iter().map(|q| q.instruction.as_str()).collect();
        let mut all_v: Vec<Vec<f32>> = Vec::new();
        for batch in instructions.chunks(50) {
            match embedder.embed_sentences(&emb_model, batch).await {
                Ok(v) => {
                    // Guard against partial returns: if the API returns fewer
                    // vectors than inputs, pad with empty to maintain alignment
                    if v.len() != batch.len() {
                        eprintln!(
                            "  WARN: embed batch returned {} vectors for {} inputs — padding",
                            v.len(),
                            batch.len()
                        );
                        let v_len = v.len();
                        all_v.extend(v);
                        for _ in v_len..batch.len() {
                            all_v.push(Vec::new());
                        }
                    } else {
                        let v_len = v.len();
                        all_v.extend(v);
                    }
                }
                Err(e) => {
                    eprintln!("  WARN: embed batch: {e}");
                    for _ in 0..batch.len() {
                        all_v.push(Vec::new());
                    }
                }
            }
        }
        println!(
            "  Embedded: {} / {} instructions",
            all_v.iter().filter(|v| !v.is_empty()).count(),
            all_v.len()
        );

        // Pre-normalize all embeddings so cosine similarity becomes a dot product.
        // This is the critical performance fix: the old code called cosine_similarity
        // (which recomputes magnitudes) in an O(n²) loop. Pre-normalizing reduces
        // each comparison from 3 full-vector iterations to 1.
        let normalized: Vec<Vec<f32>> = all_v
            .iter()
            .map(|v| {
                if v.is_empty() {
                    Vec::new()
                } else {
                    let mag = (v.iter().map(|x| x * x).sum::<f32>()).sqrt();
                    if mag > 0.0 {
                        v.iter().map(|x| x / mag).collect()
                    } else {
                        v.clone()
                    }
                }
            })
            .collect();

        let threshold = args.dedup_threshold as f32;

        // ── SemDeDup Algorithm (Abbas et al., 2023) ──────────────────────
        //
        // Reference: "SemDeDup: Data-efficient learning at web-scale through
        // semantic deduplication" (arXiv:2303.09540, NeurIPS 2023)
        //
        // 1. Embed all data points (done above)
        // 2. K-means cluster the embeddings into K clusters
        // 3. Within each cluster only, compute pairwise cosine similarity
        //    and remove pairs above threshold ε
        //
        // Complexity: O(N²/K) instead of O(N²).
        // With N=17,891 and K≈134 (√N), each cluster has ~134 items.
        // Within-cluster: 134 × 134²/2 ≈ 1.2M comparisons vs 160M brute force.

        // Collect indices of QAs that have valid embeddings
        let embedded_indices: Vec<usize> = (0..filtered.len())
            .filter(|&i| i < normalized.len() && !normalized[i].is_empty())
            .collect();
        let no_embed_indices: Vec<usize> = (0..filtered.len())
            .filter(|&i| i >= normalized.len() || normalized[i].is_empty())
            .collect();
        no_embedding_count = no_embed_indices.len();

        // K-means clustering on normalized vectors (dot product = cosine similarity)
        let n = embedded_indices.len();
        // K = 2.5% of N (per SemDeDup: more clusters = fewer within-cluster
        // comparisons). For N=17,891, K≈447 clusters, avg ~40 items/cluster.
        let k = ((n as f64) * 0.025).round().max(2.0) as usize;
        println!(
            "  SemDeDup: {} embedded QAs, {} clusters (K=2.5% of N)",
            n, k
        );

        let assignments = kmeans_cluster(&normalized, &embedded_indices, k, 10);
        let clusters = assignments.len();
        let max_cluster = assignments.iter().map(|c| c.len()).max().unwrap_or(0);
        let avg_cluster = n / clusters.max(1);
        println!(
            "  K-means: {} clusters (avg {} items, max {})",
            clusters, avg_cluster, max_cluster
        );

        // Within each cluster: greedy dedup by cosine > threshold.
        // Sort by instruction length descending (longer = more specific = keep first).
        // This follows D4 (Tirumala et al. 2023): keep the most informative example.
        for cluster_indices in &assignments {
            let mut sorted = cluster_indices.clone();
            sorted.sort_by(|&a, &b| {
                filtered[b]
                    .instruction
                    .len()
                    .cmp(&filtered[a].instruction.len())
            });

            let mut kept_in_cluster: Vec<usize> = Vec::new();
            for &i in &sorted {
                let is_dup = kept_in_cluster.iter().any(|&k| {
                    let dot: f32 = normalized[i]
                        .iter()
                        .zip(normalized[k].iter())
                        .map(|(a, b)| a * b)
                        .sum();
                    dot > threshold
                });
                if !is_dup {
                    kept_in_cluster.push(i);
                    deduped.push((Some(normalized[i].clone()), &filtered[i]));
                }
            }
        }

        // QAs without embeddings: fall back to exact-match dedup
        for &i in &no_embed_indices {
            if seen.insert(filtered[i].instruction.to_lowercase()) {
                deduped.push((None, &filtered[i]));
            }
        }
    } else {
        for qa in &filtered {
            if seen.insert(qa.instruction.to_lowercase()) {
                deduped.push((None, qa));
            }
        }
    }

    let deduped_count = deduped.len();
    println!(
        "  Deduped: {} (removed {})",
        deduped_count,
        filtered.len() - deduped_count
    );
    if no_embedding_count > 0 {
        println!(
            "  No embedding: {} QAs used exact-match dedup",
            no_embedding_count
        );
    }
    let mut tc = std::collections::HashMap::new();
    for (_, qa) in &deduped {
        *tc.entry(&qa.qa_type).or_insert(0) += 1;
    }
    println!("  Types: {:?}", tc);
    if args.dry_run {
        println!("  Dry run. Would store {} QAs.", deduped_count);
        return Ok(());
    }

    // Write training JSONL
    let train: String = deduped
        .iter()
        .map(|(_, q)| {
            serde_json::to_string(
                &serde_json::json!({"instruction": q.instruction, "input": "", "output": q.output}),
            )
            .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&args.output, train + "\n")?;
    println!(
        "  Wrote: {} QAs to {}",
        deduped_count,
        args.output.display()
    );

    // Store h_mems with 5W1H dimension and Dublin Core / PKO ontology metadata.
    // The dimension anchors each QA pair in the curator's 5W1H core ontology
    // (Dimension::What = a thing/state). The value JSON carries the dual-axis
    // metadata: Dublin Core (state: what is this, where from, who made it)
    // and PKO (process: how was this produced, extracted from what).
    let semantic = SemanticMemory::open(&args.db_path, &args.passphrase, EMBEDDING_DIM)?;
    let webid = hkask_types::WebID::from_persona(CORPUS_WEBID.as_bytes());
    let mut stored = 0;
    let mut store_failures = 0;
    let mut with_chunk_ref = 0;

    for (i, (emb, qa)) in deduped.iter().enumerate() {
        if qa.chunk_ref.is_some() {
            with_chunk_ref += 1;
        }
        let entity = format!("training:qa:{}:{}:{}", args.dataset, qa.source, i);
        let v = serde_json::json!({
            "question": qa.instruction,
            "answer": qa.output,
            "bloom_level": qa.qa_type,
            "source": qa.source,
            "dataset": args.dataset,
            // Provenance fields
            "difficulty": qa.difficulty,
            "concepts": qa.concepts,
            "chunk_ref": qa.chunk_ref,
            "evidence_quotes": qa.evidence_quotes,
            // 5W1H + dual-axis ontology metadata
            "ontology": {
                "dimension": "what",
                "anchor": "dual_axis",
                "dc_type": "bibo:Document",
                "dc_source": qa.source,
                "dc_subject": qa.concepts,
                "pko_produced_by": "docproc_generate_qa",
                "pko_extracted_from": qa.chunk_ref,
            },
        });
        let h_mem = HMem::new(&entity, "training_qa_pair", v, webid)
            .with_visibility(Visibility::Public)
            .with_confidence(0.8)
            .with_dimension(Dimension::What);
        match semantic.store(h_mem) {
            Ok(()) => stored += 1,
            Err(e) => {
                store_failures += 1;
                eprintln!("  WARN: store qa {}: {e}", i);
            }
        }
    }
    println!(
        "  Stored: {} QA h_mems (5W1H: What, dual-axis: DC+BIBO/PKO)",
        stored
    );
    if store_failures > 0 {
        eprintln!("  WARNING: {} QA h_mems failed to store", store_failures);
    }
    println!(
        "  Provenance: {}/{} QAs have chunk_ref (traceable to source passage)",
        with_chunk_ref, deduped_count
    );

    // Store QA embeddings — now using the SAME index as h_mem storage
    // (fixes the index divergence bug where emb_kept had different ordering)
    if args.embed_qas {
        let to_embed: Vec<(usize, &Vec<f32>)> = deduped
            .iter()
            .enumerate()
            .filter_map(|(i, (emb, _))| emb.as_ref().map(|e| (i, e)))
            .collect();
        println!(
            "  Embedding {} QA vectors for KNN search...",
            to_embed.len()
        );
        let mut embedded_count = 0;
        let mut embed_failures = 0;
        for (i, vec) in &to_embed {
            let qa = deduped[*i].1;
            let entity = format!("training:qa:{}:{}:{}", args.dataset, qa.source, i);
            match semantic.store_embedding(&entity, vec, &emb_model) {
                Ok(_) => embedded_count += 1,
                Err(e) => {
                    embed_failures += 1;
                    eprintln!("  WARN: embed qa {}: {e}", i);
                }
            }
        }
        println!("  Embedded: {} QA vectors", embedded_count);
        if embed_failures > 0 {
            eprintln!("  WARNING: {} QA embeddings failed", embed_failures);
        }
    }
    Ok(())
}

// ── Purge QA ──────────────────────────────────────────────────────

async fn run_purge_qa(args: PurgeQaArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Purge QAs ===");
    println!("  DB:     {}", args.db_path);
    println!("  Prefix: {}", args.prefix);
    println!();

    let semantic = SemanticMemory::open(&args.db_path, &args.passphrase, EMBEDDING_DIM)?;
    println!(
        "  Embeddings before purge: {}",
        semantic.embedding_count().unwrap_or(0)
    );

    // Purge embeddings with matching entity_ref prefix
    let purged_embeddings = semantic.purge_by_prefix(&args.prefix)?;
    println!("  Purged embeddings: {}", purged_embeddings);

    // Purge h_mems with matching entity prefix.
    // HMemStore doesn't have a prefix-purge API, so we query by attribute
    // and delete matching h_mems individually.
    // Old schema: entity="corpus:qa", attribute="qa:N"
    // New schema: entity="training:qa:...", attribute="training_qa_pair"
    let mut purged_h_mems = 0usize;
    let mut h_mem_errors = 0usize;

    // For old-schema data (entity="corpus:qa"), the attributes are "qa:0", "qa:1", etc.
    // We need to query all h_mems with entity matching the prefix and delete them.
    // HMemStore::query_by_entity does a LIKE query, but we need prefix matching.
    // The simplest approach: query all h_mems and filter by entity prefix.
    // But that's expensive for large DBs. Instead, we use the fact that old-schema
    // h_mems have entity="corpus:qa" exactly, and new-schema have entity starting
    // with "training:qa:".
    //
    // For old schema: query_by_entity("corpus:qa") deletes all with that exact entity.
    // For new schema: we'd need prefix matching, but purge_by_prefix already handles
    // the embeddings. For h_mems, we query by attribute.
    if args.prefix == "corpus:qa" {
        // Old schema: entity is exactly "corpus:qa", attributes are "qa:N"
        let h_mems = semantic.query_deduped(&args.prefix)?;
        for h_mem in &h_mems {
            match semantic.delete_h_mem(&h_mem.id) {
                Ok(()) => purged_h_mems += 1,
                Err(_) => h_mem_errors += 1,
            }
        }
    } else {
        // New schema or custom prefix: query by attribute "training_qa_pair"
        // and filter by entity prefix.
        let h_mems = semantic.query_by_attribute("training_qa_pair")?;
        for h_mem in &h_mems {
            if h_mem.entity.starts_with(&args.prefix) {
                match semantic.delete_h_mem(&h_mem.id) {
                    Ok(()) => purged_h_mems += 1,
                    Err(_) => h_mem_errors += 1,
                }
            }
        }
    }

    println!("  Purged h_mems:     {}", purged_h_mems);
    if h_mem_errors > 0 {
        eprintln!("  WARNING: {} h_mem deletions failed", h_mem_errors);
    }
    println!(
        "  Embeddings after purge: {}",
        semantic.embedding_count().unwrap_or(0)
    );
    Ok(())
}

// ── OCR ───────────────────────────────────────────────────────────

async fn run_ocr(args: OcrArgs) -> Result<(), Box<dyn std::error::Error>> {
    use hkask_services_corpus::ocr_pdf_bytes;

    println!("=== OCR Scanned PDFs ===");
    println!("  Path:   {}", args.path.display());
    println!("  Output: {}", args.output.display());
    println!();

    let pdfs: Vec<std::path::PathBuf> = if args.path.is_dir() {
        std::fs::read_dir(&args.path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "pdf"))
            .map(|e| e.path())
            .collect()
    } else {
        vec![args.path.clone()]
    };

    let mut processed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for pdf_path in &pdfs {
        let stem = pdf_path.file_stem().unwrap_or_default().to_string_lossy();
        let out_path = args.output.join(format!("{}.txt", stem));

        if args.skip_existing && out_path.exists() {
            let existing = std::fs::read_to_string(&out_path).unwrap_or_default();
            if existing.trim().len() > 100 {
                skipped += 1;
                continue;
            }
        }

        let bytes = std::fs::read(pdf_path)?;
        let url = pdf_path.to_string_lossy().to_string();
        println!(
            "  [{}/{}] OCR: {}",
            processed + failed + skipped + 1,
            pdfs.len(),
            stem
        );

        match ocr_pdf_bytes(&bytes, &url).await {
            Ok(text) => {
                let words = text.split_whitespace().count();
                if words > 0 {
                    std::fs::write(&out_path, &text)?;
                    println!("    -> {} words extracted", words);
                    processed += 1;
                } else {
                    println!("    -> 0 words (empty OCR result)");
                    failed += 1;
                }
            }
            Err(e) => {
                eprintln!("    -> FAILED: {}", e);
                failed += 1;
            }
        }
    }

    println!("\n=== Done ===");
    println!("  Processed: {}", processed);
    println!("  Failed:    {}", failed);
    println!("  Skipped:   {}", skipped);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_generation_record_normalizes_with_source_provenance() {
        let record: GeneratedQaRecord = serde_json::from_str(
            r#"{"chunk_ref":"corpus:researcher:example:0","source":"example.md","qa_type":"analyze","response":{"instruction":"Explain the mechanism behind this result.","output":"The source describes a causal relationship supported by its stated evidence.","type":"analyze"}}"#,
        )
        .expect("envelope record should deserialize");
        let qa = record.into_generated_qa();

        assert_eq!(qa.chunk_ref.as_deref(), Some("corpus:researcher:example:0"));
        assert!(has_admissible_qa_provenance(&qa));
    }

    #[test]
    fn untraceable_qa_is_not_admissible() {
        let qa = GeneratedQa {
            instruction: "Explain this finding in detail.".to_string(),
            output: "The evidence supports the finding.".to_string(),
            qa_type: "analyze".to_string(),
            difficulty: 3,
            concepts: Vec::new(),
            source: String::new(),
            chunk_ref: None,
            evidence_quotes: Vec::new(),
        };

        assert!(!has_admissible_qa_provenance(&qa));
    }

    #[test]
    fn evidence_must_be_an_exact_quote_from_the_referenced_chunk() {
        let qa = GeneratedQa {
            instruction: "Explain this finding in detail.".to_string(),
            output: "The evidence supports the finding.".to_string(),
            qa_type: "analyze".to_string(),
            difficulty: 3,
            concepts: Vec::new(),
            source: "example.md".to_string(),
            chunk_ref: Some("corpus:researcher:example:0".to_string()),
            evidence_quotes: vec!["The evidence supports the finding.".to_string()],
        };
        let chunks = std::collections::HashMap::from([(
            "corpus:researcher:example:0".to_string(),
            "The evidence supports the finding.".to_string(),
        )]);

        assert!(has_source_supported_evidence(&qa, &chunks));
    }
}
