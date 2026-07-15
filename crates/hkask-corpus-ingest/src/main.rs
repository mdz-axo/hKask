//! Company Researcher corpus tool — CLI utilities (purge-qa, ocr).
//! Pipeline logic lives in the MCP docproc server tools.
//!
//! Usage: `cargo run -p hkask-corpus-ingest -- <COMMAND>`

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use hkask_memory::SemanticMemory;

/// Qwen3-Embedding-0.6B output dimension.
const EMBEDDING_DIM: usize = 1024;

#[derive(Parser)]
#[command(name = "corpus-ingest")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// OCR scanned PDFs using vision model fallback
    Ocr(OcrArgs),
    /// Purge QA h_mems and embeddings by entity-ref prefix
    PurgeQa(PurgeQaArgs),
}

#[derive(Parser)]
struct PurgeQaArgs {
    /// Entity-ref prefix to purge (e.g. "corpus:qa" for old schema, "training:qa:corpus-researcher" for new)
    #[arg(long, default_value = "corpus:qa")]
    prefix: String,
    /// Path to memory DB
    #[arg(short = 'd', long, default_value = "corpus/memory/john-brooks.db")]
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
        Command::Ocr(args) => run_ocr(args).await,
        Command::PurgeQa(args) => run_purge_qa(args).await,
    }
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
