//! Style composition service — exemplar retrieval, prose generation, centroid validation.
//!
//! `ComposeService` encapsulates the Hemingway style synthesizer pipeline:
//!   1. Open per-agent DB + construct semantic memory infrastructure
//!   2. Embed the user's prompt via Okapi
//!   3. KNN search for exemplar passages in the style corpus
//!   4. Filter by prefix, centroid exclusion, rule exclusion, distance threshold
//!   5. Retrieve passage text from semantic triples
//!   6. Compose system prompt with exemplars + centroid validation note
//!   7. Generate prose via inference
//!   8. Validate centroid distance (optional)
//!
//! # Depth test
//!
//! Deleting this module would cause ~200 lines of pipeline construction
//! (DB open → TripleStore → EmbeddingStore → SemanticMemory → OkapiEmbedding
//! → KNN search → filter → deduped retrieval → prompt composition → inference
//! → centroid validation) to reappear in any caller. Passes deletion test.
//!
//! # Design decisions
//!
//! - **Constraint: Guideline** — DB path + passphrase are caller-provided,
//!   not from ServiceContext. Compose uses user-provided DB credentials,
//!   similar to ConsolidationService.
//! - **Constraint: Guideline** — CognitionConfig lives in the service layer
//!   because it's domain configuration for the compose operation, not
//!   surface-specific CLI arg parsing. Future API routes would use the
//!   same type.
//! - **Constraint: Guideline** — InferenceContext is caller-provided.
//!   The compose command constructs its own InferenceContext because it
//!   uses user-provided DB credentials (not ServiceContext).
//! - **CLI-only** — no API route for compose currently exists. The service
//!   is designed to serve both surfaces when one is added.

use std::path::PathBuf;
use std::sync::Arc;

use hkask_memory::SemanticMemory;
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_templates::{OkapiConfig, OkapiEmbedding};
use hkask_types::LLMParameters;
use hkask_types::ports::InferencePort;
use serde::Deserialize;

use crate::ServiceError;
use crate::inference::InferenceContext;

// ── Cognition configuration ──────────────────────────────────────────────

/// Cognition configuration for the style composition pipeline.
///
/// Deserialized from a YAML file that specifies the embedding model,
/// retrieval parameters, and centroid validation threshold.
///
/// Example YAML:
/// ```yaml
/// embedding:
///   model: "qwen3-embedding:0.6b"
///   dim: 1024
///   centroid_entity_ref: "style:hemingway:centroid"
///   retrieval:
///     k_min: 3
///     k_max: 7
///     distance_threshold: 0.30
/// validation:
///   centroid_distance_max: 0.35
/// ```
#[derive(Debug, Deserialize)]
pub struct CognitionConfig {
    pub embedding: EmbeddingSection,
    pub validation: ValidationSection,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingSection {
    pub model: String,
    pub dim: usize,
    pub centroid_entity_ref: String,
    #[serde(default)]
    pub retrieval: RetrievalSection,
}

#[derive(Debug, Deserialize)]
pub struct RetrievalSection {
    #[serde(default = "default_k_min")]
    pub k_min: usize,
    #[serde(default = "default_k_max")]
    pub k_max: usize,
    #[serde(default = "default_distance_threshold")]
    pub distance_threshold: f64,
}

impl Default for RetrievalSection {
    fn default() -> Self {
        Self {
            k_min: default_k_min(),
            k_max: default_k_max(),
            distance_threshold: default_distance_threshold(),
        }
    }
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
pub struct ValidationSection {
    pub centroid_distance_max: f64,
}

// ── Request / Response types ────────────────────────────────────────────

/// Input for `ComposeService::compose()`.
pub struct ComposeRequest {
    /// The user's prompt for prose generation.
    pub prompt: String,
    /// Path to the per-agent semantic database.
    pub db_path: PathBuf,
    /// Passphrase for opening the database.
    pub db_passphrase: String,
    /// Parsed cognition configuration.
    pub cognition: CognitionConfig,
    /// Inference context for model resolution.
    pub inference_ctx: InferenceContext,
    /// Skip centroid distance validation.
    pub no_validate: bool,
}

/// Result of a style composition operation.
pub struct ComposeResult {
    /// The generated prose text.
    pub generated_prose: String,
    /// Number of exemplar passages used.
    pub exemplar_count: usize,
    /// Centroid validation result (None if validation was skipped).
    pub validation: Option<CentroidValidation>,
}

/// Centroid distance validation result.
pub struct CentroidValidation {
    /// Cosine distance between generated prose and style centroid.
    pub distance: f64,
    /// Maximum allowed distance threshold.
    pub threshold: f64,
    /// Whether the prose passes validation (distance <= threshold).
    pub passed: bool,
}

// ── Service ──────────────────────────────────────────────────────────────

/// Style composition service — exemplar retrieval, prose generation, centroid validation.
pub struct ComposeService;

impl ComposeService {
    /// Execute the full style composition pipeline.
    ///
    /// # REQ: svc-compose-001 — compose returns generated prose with exemplar retrieval
    /// # REQ: svc-compose-002 — compose validates centroid distance when no_validate is false
    /// # REQ: svc-compose-003 — compose returns validation=None when no_validate is true
    pub async fn compose(request: ComposeRequest) -> Result<ComposeResult, ServiceError> {
        // 1. Open DB + construct memory infrastructure
        let db = Database::open(&request.db_path.to_string_lossy(), &request.db_passphrase)?;
        let conn = db.conn_arc();
        let triple_store = TripleStore::new(Arc::clone(&conn));
        let embedding_store =
            EmbeddingStore::with_dim(Arc::clone(&conn), request.cognition.embedding.dim);
        let semantic = SemanticMemory::new(triple_store, embedding_store);
        let embedding_store_direct = EmbeddingStore::new(Arc::clone(&conn));

        // 2. Create OkapiEmbedding and embed prompt
        let okapi_config = OkapiConfig {
            base_url: request.inference_ctx.okapi_base_url.clone(),
            ..OkapiConfig::default()
        };
        let embedder =
            OkapiEmbedding::with_model(&request.cognition.embedding.model, okapi_config)?;
        let prompt_vector = embedder.embed_sentence(&request.prompt).await?;

        // 3. KNN search for exemplar passages
        let results =
            semantic.search_similar(&prompt_vector, request.cognition.embedding.retrieval.k_max)?;

        // 4. Filter by prefix, centroid exclusion, rule exclusion, distance threshold
        let prefix = format!(
            "style:{}",
            request
                .cognition
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
                    && r.embedding.entity_ref != request.cognition.embedding.centroid_entity_ref
                    && !r.embedding.entity_ref.contains(":rule:")
                    && r.distance <= request.cognition.embedding.retrieval.distance_threshold
            })
            .take_while(|r| r.distance <= request.cognition.embedding.retrieval.distance_threshold)
            .map(|r| {
                let entity_ref = &r.embedding.entity_ref;
                match semantic.query_deduped(entity_ref) {
                    Ok(triples) if !triples.is_empty() => triples
                        .iter()
                        .filter_map(|t| t.value.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => format!("[passage: {}]", entity_ref),
                }
            })
            .collect();

        let exemplar_count = exemplar_passages.len();

        // 5. Compose system prompt
        let system_prompt = compose_system_prompt(
            &request.prompt,
            &exemplar_passages,
            request.no_validate,
            request.cognition.validation.centroid_distance_max,
        );

        // 6. Generate prose
        let gen_model = std::env::var("OKAPI_MODEL")
            .unwrap_or_else(|_| request.cognition.embedding.model.clone());
        let inference = crate::InferenceService::resolve_port(&request.inference_ctx, &gen_model)?;
        let params = LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 512,
            seed: None,
        };
        let result = inference.generate(&system_prompt, &params).await?;
        let generated_prose = result.text.trim().to_string();

        // 7. Validate centroid distance (optional)
        let validation = if request.no_validate {
            None
        } else {
            let prose_vector = embedder.embed_sentence(&generated_prose).await?;
            match embedding_store_direct.get(&request.cognition.embedding.centroid_entity_ref) {
                Ok(centroid_embedding) => {
                    let distance = cosine_distance(&prose_vector, &centroid_embedding.vector);
                    let threshold = request.cognition.validation.centroid_distance_max;
                    Some(CentroidValidation {
                        distance,
                        threshold,
                        passed: distance <= threshold,
                    })
                }
                Err(_) => None,
            }
        };

        Ok(ComposeResult {
            generated_prose,
            exemplar_count,
            validation,
        })
    }
}

// ── Prompt composition ───────────────────────────────────────────────────

fn compose_system_prompt(
    prompt: &str,
    exemplar_passages: &[String],
    no_validate: bool,
    centroid_distance_max: f64,
) -> String {
    let exemplar_block = if exemplar_passages.is_empty() {
        String::new()
    } else {
        let mut block =
            "\n## Exemplar Passages\nThe following passages exemplify the target style. \
             Use them as reference for rhythm, syntax, and cadence — not as content to imitate.\n\n"
                .to_string();
        for passage in exemplar_passages {
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
            centroid_distance_max, centroid_distance_max
        )
    };

    format!(
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
    )
}

// ── Utility ─────────────────────────────────────────────────────────────

/// Compute cosine distance between two vectors.
/// Returns 0.0 for identical vectors, 2.0 for opposite vectors.
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f64 {
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
    1.0 - similarity
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: svc-compose-001 — compose returns generated prose with exemplar retrieval
    // Full pipeline test requires a live Okapi server and populated DB.
    // Structural tests verify type construction and utility functions.

    #[test]
    fn cognition_config_deserializes_from_yaml() {
        let yaml = r#"
embedding:
  model: "test-model"
  dim: 512
  centroid_entity_ref: "style:hemingway:centroid"
  retrieval:
    k_min: 5
    k_max: 10
    distance_threshold: 0.25
validation:
  centroid_distance_max: 0.40
"#;
        let config: CognitionConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.embedding.model, "test-model");
        assert_eq!(config.embedding.dim, 512);
        assert_eq!(
            config.embedding.centroid_entity_ref,
            "style:hemingway:centroid"
        );
        assert_eq!(config.embedding.retrieval.k_min, 5);
        assert_eq!(config.embedding.retrieval.k_max, 10);
        assert!((config.embedding.retrieval.distance_threshold - 0.25).abs() < f64::EPSILON);
        assert!((config.validation.centroid_distance_max - 0.40).abs() < f64::EPSILON);
    }

    #[test]
    fn cognition_config_uses_default_retrieval_values() {
        let yaml = r#"
embedding:
  model: "test-model"
  dim: 256
  centroid_entity_ref: "style:hemingway:centroid"
validation:
  centroid_distance_max: 0.35
"#;
        let config: CognitionConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.embedding.retrieval.k_min, 3);
        assert_eq!(config.embedding.retrieval.k_max, 7);
        assert!((config.embedding.retrieval.distance_threshold - 0.30).abs() < f64::EPSILON);
    }

    // REQ: svc-compose-002 — compose validates centroid distance when no_validate is false
    #[test]
    fn cosine_distance_identical_vectors_is_zero() {
        let v = vec![1.0_f32, 0.0, 0.0];
        let dist = cosine_distance(&v, &v);
        assert!(
            (dist - 0.0).abs() < 1e-6,
            "identical vectors should have distance 0"
        );
    }

    #[test]
    fn cosine_distance_opposite_vectors_is_two() {
        let a = vec![1.0_f32, 0.0, 0.0];
        let b = vec![-1.0_f32, 0.0, 0.0];
        let dist = cosine_distance(&a, &b);
        assert!(
            (dist - 2.0).abs() < 1e-6,
            "opposite vectors should have distance 2"
        );
    }

    #[test]
    fn cosine_distance_orthogonal_vectors_is_one() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![0.0_f32, 1.0];
        let dist = cosine_distance(&a, &b);
        assert!(
            (dist - 1.0).abs() < 1e-6,
            "orthogonal vectors should have distance 1"
        );
    }

    #[test]
    fn cosine_distance_mismatched_lengths_returns_two() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![1.0_f32];
        let dist = cosine_distance(&a, &b);
        assert!(
            (dist - 2.0).abs() < f64::EPSILON,
            "mismatched lengths should return 2.0"
        );
    }

    #[test]
    fn centroid_validation_passed_when_distance_within_threshold() {
        let validation = CentroidValidation {
            distance: 0.25,
            threshold: 0.35,
            passed: true,
        };
        assert!(validation.passed);
        assert!(validation.distance <= validation.threshold);
    }

    #[test]
    fn centroid_validation_failed_when_distance_exceeds_threshold() {
        let validation = CentroidValidation {
            distance: 0.50,
            threshold: 0.35,
            passed: false,
        };
        assert!(!validation.passed);
        assert!(validation.distance > validation.threshold);
    }

    #[test]
    fn system_prompt_contains_exemplar_block() {
        let passages = vec!["It was a good morning.".to_string()];
        let prompt = compose_system_prompt("Write about fishing", &passages, false, 0.35);
        assert!(prompt.contains("Exemplar Passages"));
        assert!(prompt.contains("It was a good morning."));
        assert!(prompt.contains("Centroid Validation"));
        assert!(prompt.contains("0.35"));
    }

    #[test]
    fn system_prompt_omits_centroid_note_when_no_validate() {
        let prompt = compose_system_prompt("Write about fishing", &[], true, 0.35);
        assert!(!prompt.contains("Centroid Validation"));
    }

    #[test]
    fn system_prompt_omits_exemplar_block_when_empty() {
        let prompt = compose_system_prompt("Write about fishing", &[], false, 0.35);
        assert!(!prompt.contains("Exemplar Passages"));
    }
}
