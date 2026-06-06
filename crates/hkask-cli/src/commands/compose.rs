//! Style composition command — generate prose with exemplar retrieval and centroid validation
//!
//! Orchestrates the Hemingway style synthesizer pipeline:
//!   1. Load cognition config (style parameters + embedding config)
//!   2. Embed the user's prompt via Okapi
//!   3. KNN search for exemplar passages in the style corpus
//!   4. Render the system prompt with exemplars injected
//!   5. Send to inference
//!   6. (Optional) Validate centroid distance
//!
//! Manifest: registry/manifests/hemingway-style-synthesizer.yaml
//! Cognition: registry/registries/cognition/hemingway-style-synthesizer.yaml

use crate::cli::ComposeAction;
use hkask_memory::SemanticMemory;
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_templates::{OkapiConfig, OkapiEmbedding, OkapiInference};
use hkask_types::LLMParameters;
use hkask_types::ports::InferencePort;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct CognitionConfig {
    embedding: EmbeddingSection,
    validation: ValidationSection,
}

#[derive(Debug, Deserialize)]
struct EmbeddingSection {
    model: String,
    dim: usize,
    centroid_entity_ref: String,
    retrieval: RetrievalSection,
}

#[derive(Debug, Deserialize)]
struct RetrievalSection {
    #[serde(default = "default_k_min")]
    k_min: usize,
    #[serde(default = "default_k_max")]
    k_max: usize,
    #[serde(default = "default_distance_threshold")]
    distance_threshold: f64,
}

fn default_k_min() -> usize {
    3
}
fn default_k_max() -> usize {
    7
}
fn default_distance_threshold() -> f64 {
    0.30
}

#[derive(Debug, Deserialize)]
struct ValidationSection {
    centroid_distance_max: f64,
}

pub fn run(rt: &tokio::runtime::Runtime, action: ComposeAction) {
    match action {
        ComposeAction::Run {
            prompt,
            cognition,
            db,
            passphrase,
            okapi_url,
            no_validate,
        } => run_compose(
            rt,
            prompt,
            cognition,
            db,
            passphrase,
            okapi_url,
            no_validate,
        ),
    }
}

fn run_compose(
    rt: &tokio::runtime::Runtime,
    prompt: String,
    cognition_path: PathBuf,
    db_path: PathBuf,
    passphrase: String,
    okapi_url: Option<String>,
    no_validate: bool,
) {
    let config_str = std::fs::read_to_string(&cognition_path).unwrap_or_else(|e| {
        eprintln!(
            "Failed to read cognition config {}: {}",
            cognition_path.display(),
            e
        );
        std::process::exit(1);
    });
    let config: CognitionConfig = serde_yaml::from_str(&config_str).unwrap_or_else(|e| {
        eprintln!("Failed to parse cognition config YAML: {}", e);
        std::process::exit(1);
    });
    eprintln!(
        "Compose: model={}, dim={}, centroid={}",
        config.embedding.model, config.embedding.dim, config.embedding.centroid_entity_ref
    );

    let db = Database::open(&db_path.to_string_lossy(), &passphrase).unwrap_or_else(|e| {
        eprintln!("Failed to open database {}: {}", db_path.display(), e);
        std::process::exit(1);
    });
    let conn = db.conn_arc();
    let triple_store = TripleStore::new(Arc::clone(&conn));
    let embedding_store = EmbeddingStore::with_dim(Arc::clone(&conn), config.embedding.dim);
    let semantic = SemanticMemory::new(triple_store, embedding_store);
    // Keep a separate EmbeddingStore for direct access (centroid retrieval)
    let embedding_store_direct = EmbeddingStore::new(Arc::clone(&conn));

    let okapi_config = match okapi_url {
        Some(ref url) => OkapiConfig {
            base_url: url.clone(),
            ..OkapiConfig::default()
        },
        None => OkapiConfig::local_dev(),
    };
    let embedder = OkapiEmbedding::with_model(&config.embedding.model, okapi_config)
        .unwrap_or_else(|e| {
            eprintln!("Failed to create embedding client: {}", e);
            std::process::exit(1);
        });

    eprintln!("Embedding prompt...");
    let prompt_vector = rt
        .block_on(embedder.embed_sentence(&prompt))
        .unwrap_or_else(|e| {
            eprintln!("Failed to embed prompt: {}", e);
            std::process::exit(1);
        });

    eprintln!(
        "Searching for {}-{} exemplar passages...",
        config.embedding.retrieval.k_min, config.embedding.retrieval.k_max
    );
    let results = semantic
        .search_similar(&prompt_vector, config.embedding.retrieval.k_max)
        .unwrap_or_else(|e| {
            eprintln!("Failed to search for exemplar passages: {}", e);
            std::process::exit(1);
        });

    // Filter by prefix and distance threshold
    let prefix = format!(
        "style:{}",
        config
            .embedding
            .centroid_entity_ref
            .split(':')
            .nth(1)
            .unwrap_or("hemingway")
    );
    let exemplar_passages: Vec<String> = results
        .into_iter()
        .filter(|r| {
            r.embedding.entity_ref.starts_with(&prefix)
                && r.embedding.entity_ref != config.embedding.centroid_entity_ref
                && !r.embedding.entity_ref.contains(":rule:")
                && r.distance <= config.embedding.retrieval.distance_threshold
        })
        .take_while(|r| r.distance <= config.embedding.retrieval.distance_threshold)
        .map(|r| {
            // Try to retrieve the passage text from semantic triples
            let entity_ref = &r.embedding.entity_ref;
            match semantic.query_deduped(entity_ref) {
                Ok(triples) if !triples.is_empty() => {
                    // The passage text is stored as the value of the "text" attribute
                    triples
                        .iter()
                        .filter_map(|t| t.value.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                _ => format!("[passage: {}]", entity_ref),
            }
        })
        .collect();

    eprintln!("Found {} exemplar passages", exemplar_passages.len());

    if exemplar_passages.is_empty() {
        eprintln!(
            "Warning: No exemplar passages found within distance threshold {:.3}. \
             The style corpus may not be embedded yet. Run `kask embed-corpus` first.",
            config.embedding.retrieval.distance_threshold
        );
    } else if exemplar_passages.len() < config.embedding.retrieval.k_min {
        eprintln!(
            "Warning: Only {} exemplar passages found (k_min={}). \
             Consider widening the distance threshold or embedding more corpus texts.",
            exemplar_passages.len(),
            config.embedding.retrieval.k_min
        );
    }

    let exemplar_block = if exemplar_passages.is_empty() {
        String::new()
    } else {
        let mut block =
            "\n## Exemplar Passages\nThe following passages exemplify the target style. \
             Use them as reference for rhythm, syntax, and cadence — not as content to imitate.\n\n"
                .to_string();
        for passage in &exemplar_passages {
            block.push_str("---\n");
            block.push_str(passage);
            block.push_str("\n---\n\n");
        }
        block
    };

    let centroid_note = if no_validate {
        String::new()
    } else {
        format!(
            "\n## Centroid Validation\n\
             Your output will be embedded and compared against the style centroid.\n\
             Centroid distance threshold: {:.2}\n\
             If the distance exceeds {:.2}, the output will be rejected.\n",
            config.validation.centroid_distance_max, config.validation.centroid_distance_max
        )
    };

    let system_prompt = format!(
        "You are an expert prose stylist writing in the authentic style of Ernest Hemingway.\n\
         \n\
         ## Kansas City Star Rules (1915)\n\
         - Use short sentences.\n\
         - Use short first paragraphs.\n\
         - Use vigorous English, not forgetful, but positive.\n\
         - Eliminate every superfluous word.\n\
         \n\
         ## Syntactic Mechanics\n\
         - Coordinate 73-76% of clauses (use \"and\" as primary conjunction)\n\
         - Avoid subordinating conjunctions: because, although, since, while, after\n\
         - Use \"when\", \"if\", \"unless\" sparingly\n\
         - Asyndetic coordination (comma-only) is permitted\n\
         - Show causality through juxtaposition, not explanation\n\
         \n\
         ## Iceberg Theory\n\
         - State only the visible 1/8: action, sensation, concrete detail\n\
         - Leave the 7/8 (emotion, judgment, interpretation) unstated\n\
         - Show emotion through action, not through adjectives or explanation\n\
         \n\
         ## Lexical Constraints\n\
         - Prefer concrete nouns, action verbs, simple adjectives\n\
         - Avoid abstract nouns, passive voice, adverbs, qualifiers\n\
         - Average sentence length: 10-20 words, range: 3-35 words\n\
         - First paragraph: 1-3 sentences. Subsequent: 3-8 sentences.\n\
         \n\
         ## Stylistic Devices\n\
         - Polysyndeton (\"He was cold and he was tired and he walked on.\") — for accumulation\n\
         - Asyndeton (\"The sun beat down. The dust rose.\") — for staccato urgency\n\
         - Parataxis (\"The leaves fell. The soldiers marched.\") — default mode\n\
         {exemplar_block}\
         {centroid_note}\
         \n## Task\n\
         {prompt}"
    );

    eprintln!("System prompt composed ({} chars)", system_prompt.len());

    // Use a generation model (not the embedding model) for prose generation.
    // Default to the same model family; override via OKAPI_MODEL env if needed.
    let gen_model = std::env::var("OKAPI_MODEL").unwrap_or_else(|_| config.embedding.model.clone());
    let inference_config = match okapi_url {
        Some(ref url) => OkapiConfig {
            base_url: url.clone(),
            ..OkapiConfig::default()
        },
        None => OkapiConfig::local_dev(),
    };
    let inference = OkapiInference::new(&gen_model, inference_config).unwrap_or_else(|e| {
        eprintln!("Failed to create inference client: {}", e);
        std::process::exit(1);
    });

    eprintln!("Generating prose with model '{}'...", gen_model);
    let params = LLMParameters {
        temperature: 0.7,
        top_p: 0.9,
        top_k: 40,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        max_tokens: 512,
        seed: None,
    };

    let result = rt
        .block_on(inference.generate(&system_prompt, &params))
        .unwrap_or_else(|e| {
            eprintln!("Inference failed: {}", e);
            std::process::exit(1);
        });

    let generated_prose = result.text.trim().to_string();
    eprintln!("\n{}", generated_prose);

    if !no_validate {
        eprintln!("\nValidating style centroid distance...");

        let prose_vector = rt
            .block_on(embedder.embed_sentence(&generated_prose))
            .unwrap_or_else(|e| {
                eprintln!("Failed to embed generated prose: {}", e);
                std::process::exit(1);
            });

        // Get centroid vector from the embedding store
        match embedding_store_direct.get(&config.embedding.centroid_entity_ref) {
            Ok(centroid_embedding) => {
                let centroid_vector = centroid_embedding.vector;
                let distance = cosine_distance(&prose_vector, &centroid_vector);
                let max_dist = config.validation.centroid_distance_max;

                eprintln!(
                    "Centroid distance: {:.4} (threshold: {:.4})",
                    distance, max_dist
                );

                if distance <= max_dist {
                    eprintln!("✓ Style validation PASSED — prose is within style cluster");
                } else {
                    eprintln!(
                        "✗ Style validation FAILED — prose exceeds style cluster boundary ({:.4} > {:.4})",
                        distance, max_dist
                    );
                    eprintln!(
                        "Consider regenerating with stricter adherence to syntactic constraints."
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not retrieve centroid '{}': {}",
                    config.embedding.centroid_entity_ref, e
                );
                eprintln!("Run `kask embed-corpus` to build the style corpus first.");
            }
        }
    }

    eprintln!("\nDone.");
}

/// Compute cosine distance between two vectors.
/// Returns 0.0 for identical vectors, 2.0 for opposite vectors.
fn cosine_distance(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 2.0;
    }
    let dot: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 2.0;
    }
    let similarity = dot / (norm_a * norm_b);
    1.0 - similarity // cosine distance = 1 - cosine similarity
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_distance_identical() {
        let v = vec![1.0, 0.0, 0.0];
        let dist = cosine_distance(&v, &v);
        assert!(dist.abs() < 1e-6);
    }

    #[test]
    fn cosine_distance_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let dist = cosine_distance(&a, &b);
        assert!((dist - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_distance_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let dist = cosine_distance(&a, &b);
        assert!((dist - 2.0).abs() < 1e-6);
    }

    #[test]
    fn cognition_config_deserializes() {
        let yaml = "\
embedding:
  model: \"qwen3-embedding:0.6b\"
  dim: 384
  centroid_entity_ref: \"style:hemingway:centroid\"
  retrieval:
    k_min: 3
    k_max: 7
    distance_threshold: 0.30
validation:
  centroid_distance_max: 0.15";
        let config: CognitionConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.embedding.model, "qwen3-embedding:0.6b");
        assert_eq!(config.embedding.dim, 384);
        assert_eq!(config.embedding.retrieval.k_min, 3);
        assert_eq!(config.embedding.retrieval.k_max, 7);
        assert_eq!(config.validation.centroid_distance_max, 0.15);
    }
}
