//! EmbedService — Style corpus embedding pipeline with metadata layer.
//! # REQ: P3 (Generative Space) — full parameter exposure, no hidden settings.
//! # expect: "The service layer enables generative access to domain capabilities"
//!
//! ## Pipeline phases
//! 1. **Parse config** — YAML with entities, methods, budget, works
//! 2. **Download & chunk** — Gutenberg texts → tagged passages
//! 3. **Tag** — entity matching + method signal extraction
//! 4. **Salience** — weighted graph degree centrality per passage
//! 5. **Budget gate** — sort by salience, top-N by triple budget
//! 6. **Embed** — all passages get vectors (via inference providers)
//! 7. **Store triples** — budget-selected passages get metadata triples
//! 8. **Centroid** — mean vector over prose passages

use hkask_inference::{EmbeddingRouter, InferenceConfig, InferenceRouter};
use hkask_memory::SemanticMemory;
use hkask_memory::salience::{self, BudgetConfig, DeclaredMethod, EntityTags, MethodSignals};
use hkask_storage::{Database, EmbeddingStore, Triple, TripleStore};
use hkask_types::Visibility;
use hkask_types::id::WebID;
use hkask_types::template::LLMParameters;

use hkask_services_classify::TripleExtraction;

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hkask_services_core::ServiceError;

// ── Re-exports ─────────────────────────────────────────────────────────────

// (salience type re-exports deleted — essentialist review: zero external callers.
//  Import BudgetConfig, DeclaredMethod, MethodSignals directly from hkask_memory::salience.)

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
    Tagging,
    Embedding,
    Triples,
    Centroid,
    Done,
}

impl EmbedProgress {
    /// [P7] Motivating: Evolutionary Architecture — display formatting emerges from usage.
    /// pre:  self is a valid EmbedProgress
    /// post: returns formatted "TODO [N pages · X%] ::: DONE [N pages · Y%]" string
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

    /// [P7] Motivating: Evolutionary Architecture — full status formatting.
    /// pre:  self is a valid EmbedProgress
    /// post: returns formatted "[phase] author — work — page_progress" string
    pub fn format_full(&self) -> String {
        let phase_label = match self.phase {
            EmbedPhase::Parsing => "Parsing",
            EmbedPhase::Tagging => "Tagging",
            EmbedPhase::Embedding => "Embedding",
            EmbedPhase::Triples => "Triples",
            EmbedPhase::Centroid => "Centroid",
            EmbedPhase::Done => "Done",
        };
        format!(
            "[{phase_label}] {} — {}",
            self.author,
            if self.current_work.is_empty() {
                self.format_page_progress()
            } else {
                format!("{} — {}", self.current_work, self.format_page_progress())
            }
        )
    }
}

// ── Configuration ──────────────────────────────────────────────────────────

/// Corpus configuration — defines the author, works, embedding model,
/// chunking parameters, entity declarations, method declarations,
/// budget settings, and validation constraints.
#[derive(Debug, Deserialize, Serialize)]
pub struct CorpusConfig {
    pub author: String,
    pub embedding: EmbeddingConfig,
    pub works: Vec<Work>,
    pub foundational_rules: Vec<FoundationalRule>,
    pub chunking: ChunkingConfig,
    pub centroid_entity_ref: String,
    pub validation: ValidationConfig,

    /// Budget for triple storage per corpus (default: 3,750 triples/100 pages).
    #[serde(default)]
    pub budget: BudgetConfig,

    /// Entity declarations for tagging (who, where, what, why).
    #[serde(default)]
    pub entities: EntityConfig,

    /// Declared methods with signal thresholds (how).
    #[serde(default)]
    pub methods: Vec<DeclaredMethod>,

    /// Corpus type discriminator: "literary" or "academic".
    /// Determines which entity categories are active and which method
    /// signals are computed during embedding. Default: "literary".
    #[serde(default = "default_corpus_type")]
    pub corpus_type: String,

    /// Per-dimension centroid configuration with weights.
    /// Keys are dimension names (gentle, schriver, hopper, lovelace).
    /// Compute one centroid per dimension, then derive composite at query time.
    #[serde(default)]
    pub dimension_centroids: Vec<DimensionCentroid>,

    /// Orthogonal tag sets for multi-axis passage tagging.
    #[serde(default)]
    pub tag_sets: Vec<TagSet>,

    /// Per-document-type tag weight overrides.
    /// Maps document type (specification, guide, reference, etc.) to
    /// per-dimension weights. Applied at query time when comparing
    /// documents of a specific type against the embedding space.
    #[serde(default)]
    pub tag_weights: HashMap<String, HashMap<String, f64>>,

    /// Classifier config name (references registry/classify/{name}.yaml).
    /// If empty, section_type defaults to "Statement" for all passages.
    #[serde(default)]
    pub classifier: String,

    /// Triple extractor classifier config name (references registry/classify/{name}.yaml).
    /// Uses Gemma 4 to extract semantic triples (topic, concepts, entities,
    /// relationships, primary_dimension, quality_flags) from each passage.
    /// Defaults to "triple-extractor". Set to empty string to disable.
    #[serde(default = "default_triple_classifier")]
    pub triple_classifier: String,
}

fn default_corpus_type() -> String {
    "literary".to_string()
}

fn default_triple_classifier() -> String {
    "triple-extractor".to_string()
}

/// Entity declarations for corpus-specific tagging.
///
/// Shared fields (both literary and academic): places, events, concepts.
/// Literary-only: characters.
/// Academic-only: co_authors, venues, topics, paradigms.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct EntityConfig {
    /// Literary: named characters in the author's works.
    #[serde(default)]
    pub characters: Vec<Entity>,
    /// Shared: geographic/institutional places.
    #[serde(default)]
    pub places: Vec<Entity>,
    /// Shared: named events, studies, experiments.
    #[serde(default)]
    pub events: Vec<Entity>,
    /// Shared: key ideas, theories, frameworks.
    #[serde(default)]
    pub concepts: Vec<Entity>,

    // ── Academic-specific categories ──────────────────────────────────────
    /// Academic: co-authors and collaborators.
    #[serde(default)]
    pub co_authors: Vec<Entity>,
    /// Academic: journals, conferences, publishers.
    #[serde(default)]
    pub venues: Vec<Entity>,
    /// Academic: research areas and subfields.
    #[serde(default)]
    pub topics: Vec<Entity>,
    /// Academic: theoretical frameworks, paradigms, schools of thought.
    #[serde(default)]
    pub paradigms: Vec<Entity>,
}

/// A declared entity with name and optional per-work scoping.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Entity {
    pub name: String,
    /// Restrict to specific work slugs (empty = all works).
    #[serde(default)]
    pub appears_in: Vec<String>,
}

impl Entity {
    fn matches_work(&self, work_slug: &str) -> bool {
        self.appears_in.is_empty() || self.appears_in.iter().any(|w| w == work_slug)
    }

    fn name_strings(entities: &[Entity], work_slug: &str) -> Vec<String> {
        entities
            .iter()
            .filter(|e| e.matches_work(work_slug))
            .map(|e| e.name.clone())
            .collect()
    }
}

/// Embedding model and dimension configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct EmbeddingConfig {
    pub model: String,
    pub dim: usize,
    pub batch_size: usize,
}

/// A work (text) to download and embed.
#[derive(Debug, Deserialize, Serialize)]
pub struct Work {
    pub title: String,
    pub slug: String,
    pub url: String,
    /// Local file path for pre-downloaded works (takes precedence over url).
    #[serde(default)]
    pub local_path: Option<String>,
    /// Source format: "text", "pdf", or "web". Determines ingestion path.
    #[serde(default = "default_format")]
    pub format: String,
    /// Document type per MDS_SCAFFOLD.md §2: specification, adr, guide, reference, plan, status, research-paper, book-chapter.
    #[serde(default, alias = "type")]
    pub document_type: Option<String>,
    /// Dimension tags this work contributes to: ["Gentle"], ["Schriver"], ["Hopper"], ["Lovelace"].
    #[serde(default)]
    pub dimensions: Vec<String>,
    /// Section types present in this work: Statement, Evidence, Diagram, Implications.
    #[serde(default)]
    pub section_types: Vec<String>,
    /// MDS categories per MDS.md §1: domain, composition, trust, lifecycle, curation.
    #[serde(default)]
    pub mds_categories: Vec<String>,
}

fn default_format() -> String {
    "text".to_string()
}

/// A foundational rule to include as a passage.
#[derive(Debug, Deserialize, Serialize)]
pub struct FoundationalRule {
    pub slug: String,
    pub text: String,
    /// Dimension tags for this rule.
    #[serde(default)]
    pub dimensions: Vec<String>,
    /// Section type for this rule.
    #[serde(default)]
    pub section_type: Option<String>,
}

/// Chunking parameters for passage splitting.
#[derive(Debug, Deserialize, Serialize)]
pub struct ChunkingConfig {
    pub min_words: usize,
    pub max_words: usize,
    pub sentence_boundary: String,
}

/// Validation constraints for centroid distance and exemplar counts.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidationConfig {
    pub centroid_distance_max: f64,
    pub exemplar_count_min: usize,
    pub exemplar_count_max: usize,
}

/// Per-dimension centroid configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DimensionCentroid {
    /// Dimension name: "gentle", "schriver", "hopper", "lovelace".
    pub name: String,
    /// Entity ref for storing the centroid vector.
    pub ref_name: String,
    /// Weight in the composite centroid (should sum to 1.0 across all dimensions).
    pub weight: f64,
    /// Human-readable description of this dimension.
    #[serde(default)]
    pub description: String,
}

/// Orthogonal tag set definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TagSet {
    /// Tag axis name: "section_type", "mds_category", "document_type", "dimension".
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Allowed values for this tag axis.
    #[serde(default)]
    pub values: Vec<String>,
}

// ── Tagged Passage ─────────────────────────────────────────────────────────

/// A fully tagged passage: text + entity tags + method signals + salience.
///
/// Carries everything needed for both embedding and triple storage.
#[derive(Debug, Clone)]
struct TaggedPassage {
    entity_ref: String,
    text: String,
    work_slug: String,
    work_title: String,
    /// Position within the work (0.0 = start, 1.0 = end).
    position: f32,
    /// Whether this is a foundational rule (excluded from centroid).
    is_rule: bool,
    /// Entity tags from config-declared entity matching.
    tags: EntityTags,
    /// Computed stylometric signals.
    signals: MethodSignals,
    /// Salience score (weighted graph degree).
    salience: f32,
    /// Dimension tag for this passage (from work metadata).
    dimension: String,
    /// Document type tag for this passage (from work metadata).
    document_type: String,
    /// MDS category tags for this passage (from work metadata).
    mds_categories: Vec<String>,
    /// Section type tag for this passage (from classifier or work declaration).
    section_type: String,
    /// Classifier-extracted semantic triples (topic, concepts, entities, relationships, quality).
    semantic_triples: TripleExtraction,
}

impl TaggedPassage {
    /// Count how many metadata triples this passage would consume if stored.
    /// Excludes the `text` triple — text is stored for all passages regardless
    /// of budget, since it's required for exemplar retrieval in compose.
    fn metadata_triple_count(&self) -> usize {
        // 6 structural + entity tags + method tags + 1 salience + 10 signals
        // + 4 orthogonal tags (dimension, doc_type, mds_categories, section_type)
        // + semantic triples: 1 topic + concepts + entities + relationships + 1 dimension + quality_flags
        6 + self.tags.characters.len()
            + self.tags.places.len()
            + self.tags.events.len()
            + self.tags.concepts.len()
            + self.tags.methods.len()
            + 1
            + 11 // salience + 10 method signals
            + 1 // dimension
            + 1 // document_type
            + self.mds_categories.len() // one per mds_category
            + 1 // section_type
            + if !self.semantic_triples.topic.is_empty() { 1 } else { 0 }
            + self.semantic_triples.concepts.len()
            + self.semantic_triples.entities.len()
            + self.semantic_triples.relationships.len()
            + if !self.semantic_triples.primary_dimension.is_empty() { 1 } else { 0 }
            + self.semantic_triples.quality_flags.len()
            + self.semantic_triples.extra.len()
    }

    /// Total triple count including text (for reporting only).
    fn triple_count(&self) -> usize {
        1 + self.metadata_triple_count()
    }
}

// ── Result ─────────────────────────────────────────────────────────────────

/// Result of the embedding pipeline with budget statistics.
/// Summary of a single dimension centroid computation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DimensionCentroidResult {
    pub name: String,
    pub ref_name: String,
    pub passage_count: usize,
}

#[derive(Debug)]
pub struct EmbedResult {
    pub author: String,
    pub purged: usize,
    pub total_passages: usize,
    pub centroid_ref: String,
    pub passage_count: usize,
    pub centroid_stored: bool,
    pub validation: ValidationConfig,
    /// Total triple budget for this corpus.
    pub budget: usize,
    /// Number of passages that earned triple storage.
    pub tagged_passages: usize,
    /// Triples actually stored.
    pub triples_stored: usize,
    /// Passages that got embeddings only (below budget cutoff).
    pub embedding_only: usize,
    /// Per-dimension centroid results (empty if single-centroid path).
    pub dimension_centroids: Vec<DimensionCentroidResult>,
}

const USER_AGENT: &str = "hkask-mcp-research/0.27.0";
const CURATOR_PERSONA: &[u8] = b"Curator";

/// Service for the style corpus embedding pipeline with metadata layer.
pub struct EmbedService;

impl EmbedService {
    /// Run the full style corpus embedding pipeline with metadata tagging,
    /// salience scoring, and budget-gated triple storage.
    ///
    /// See module-level docs for the full phase breakdown.
    pub async fn embed_corpus(
        config_path: &Path,
        db_path: &str,
        db_passphrase: &str,
        cache_dir: Option<&Path>,
        progress: Option<ProgressFn>,
    ) -> Result<EmbedResult, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.embed", operation = "embed_corpus", config = %config_path.display(), "CNS");

        let started = Instant::now();

        // ── Phase 1: Parse config ──────────────────────────────────────
        let config_str = std::fs::read_to_string(config_path).map_err(|e| {
            let msg = format!(
                "Failed to read corpus config {}: {e}",
                config_path.display()
            );
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;
        let config: CorpusConfig = serde_yaml_neo::from_str(&config_str).map_err(|e| {
            let msg = format!("Failed to parse corpus config YAML: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

        let author = config.author.clone();
        let author_prefix = format!("style:{}:", &author);
        let centroid_ref = config.centroid_entity_ref.clone();
        let validation = config.validation.clone();
        let curator_webid = WebID::from_persona(CURATOR_PERSONA);

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
                        let mut p = shared_hb.lock().unwrap_or_else(|e| e.into_inner());
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

        // ── Open DB ────────────────────────────────────────────────────
        let db = Database::open(db_path, db_passphrase).map_err(|e| ServiceError::Storage {
            message: e.to_string(),
        })?;
        let conn = db.conn_arc();
        let triple_store = TripleStore::new(Arc::clone(&conn));
        let embedding_store = EmbeddingStore::with_dim(Arc::clone(&conn), config.embedding.dim);
        let semantic = SemanticMemory::new(triple_store, embedding_store);

        // Purge existing embeddings for idempotent re-ingest
        let purged = semantic.purge_by_prefix(&author_prefix).map_err(|e| {
            let msg = format!("Failed to purge embeddings: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

        // ── Resolve cache directory ────────────────────────────────────
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

        // ── Phase 2: Download, cache, chunk, and tag ───────────────────
        {
            let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
            p.phase = EmbedPhase::Tagging;
        }

        let mut all_passages: Vec<TaggedPassage> = Vec::new();

        for (work_idx, work) in config.works.iter().enumerate() {
            if work_idx > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            {
                let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
                p.current_work = work.title.clone();
                p.completed_passages = work_idx + 1;
                p.total_passages = config.works.len();
            }

            let cache_path = cache.join(format!("{}.txt", work.slug));
            let text = if let Some(ref local) = work.local_path {
                let local_path = std::path::Path::new(local);
                if local_path.exists() {
                    tracing::info!(work = %work.title, path = %local, "Reading local file");
                    std::fs::read_to_string(local_path).map_err(|e| {
                        let msg =
                            format!("Failed to read local file {}: {e}", local_path.display());
                        ServiceError::Embed {
                            source: Some(Box::new(e)),
                            message: msg,
                        }
                    })?
                } else {
                    tracing::warn!(work = %work.title, path = %local, "Local file not found, falling back to cache/download");
                    if cache_path.exists() {
                        tracing::info!(work = %work.title, "Using cached");
                        std::fs::read_to_string(&cache_path).map_err(|e| {
                            let msg = format!("Failed to read cache {}: {e}", cache_path.display());
                            ServiceError::Embed {
                                source: Some(Box::new(e)),
                                message: msg,
                            }
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
                    }
                }
            } else if cache_path.exists() {
                tracing::info!(work = %work.title, "Using cached");
                std::fs::read_to_string(&cache_path).map_err(|e| {
                    let msg = format!("Failed to read cache {}: {e}", cache_path.display());
                    ServiceError::Embed {
                        source: Some(Box::new(e)),
                        message: msg,
                    }
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

            // Tag each chunk
            let total_chunks = chunks.len();
            let work_characters = Entity::name_strings(&config.entities.characters, &work.slug);
            let work_places = Entity::name_strings(&config.entities.places, &work.slug);
            let work_events = Entity::name_strings(&config.entities.events, &work.slug);
            let work_concepts = Entity::name_strings(&config.entities.concepts, &work.slug);

            for (chunk_idx, (entity_ref, text)) in chunks.into_iter().enumerate() {
                let signals = salience::compute_method_signals(&text);
                let mut tags = salience::tag_entities(
                    &text,
                    &work_characters,
                    &work_places,
                    &work_events,
                    &work_concepts,
                );

                // Match declared methods
                for method in &config.methods {
                    if method.matches(&signals) {
                        tags.methods.push(method.name.clone());
                    }
                }

                let position = if total_chunks > 1 {
                    chunk_idx as f32 / (total_chunks - 1) as f32
                } else {
                    0.5
                };

                all_passages.push(TaggedPassage {
                    entity_ref,
                    text,
                    work_slug: work.slug.clone(),
                    work_title: work.title.clone(),
                    position,
                    is_rule: false,
                    tags,
                    signals,
                    salience: 0.0, // computed in batch below
                    dimension: work.dimensions.first().cloned().unwrap_or_default(),
                    document_type: work.document_type.clone().unwrap_or_default(),
                    mds_categories: work.mds_categories.clone(),
                    section_type: String::new(), // filled by classifier below
                    semantic_triples: TripleExtraction::default(), // filled by triple classifier
                });
            }

            tracing::info!(
                work = %work.title,
                passages = total_chunks,
                "Chunked and tagged"
            );
        }

        // Append foundational rules as passages (no tagging, position=0.5, low salience)
        for rule in &config.foundational_rules {
            let entity_ref = format!("style:{}:rule:{}", &config.author, rule.slug);
            let signals = salience::compute_method_signals(&rule.text);
            all_passages.push(TaggedPassage {
                entity_ref,
                text: rule.text.clone(),
                work_slug: String::new(),
                work_title: String::new(),
                position: 0.5,
                is_rule: true,
                tags: EntityTags::default(),
                signals,
                salience: 0.0,
                dimension: rule.dimensions.first().cloned().unwrap_or_default(),
                document_type: String::new(),
                mds_categories: Vec::new(),
                section_type: rule.section_type.clone().unwrap_or_default(),
                semantic_triples: TripleExtraction::default(), // rules get empty extraction
            });
        }

        // ── Classify section types ──────────────────────────────
        {
            let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
            p.phase = EmbedPhase::Tagging;
            p.current_work = "classifying section types".into();
        }

        let passage_count = all_passages.len();

        // Compute registry_dir from config_path for classifier resolution.
        // config_path is registry/styles/gentle-lovelace/corpus.yaml,
        // so registry_dir is 3 levels up.
        let registry_dir = config_path
            .parent() // styles/gentle-lovelace
            .and_then(|p| p.parent()) // styles
            .and_then(|p| p.parent()) // registry
            .unwrap_or_else(|| Path::new("registry"));

        // Load classifier config if specified in corpus.yaml
        let classifier_config = if config.classifier.is_empty() {
            tracing::info!("No classifier configured — all passages default to Statement");
            hkask_services_classify::ClassifierConfig::from_def(&Default::default())
        } else {
            let def =
                hkask_services_classify::load_classifier_config(&config.classifier, registry_dir)?;
            hkask_services_classify::ClassifierConfig::from_def(&def)
        };

        let texts: Vec<String> = all_passages.iter().map(|p| p.text.clone()).collect();

        tracing::info!(
            total_passages = passage_count,
            model = %classifier_config.model,
            concurrency = classifier_config.concurrency,
            "Starting section type classification"
        );

        let classify_results =
            hkask_services_classify::classify_batch(&texts, classifier_config).await?;

        for (passage, result) in all_passages.iter_mut().zip(classify_results.iter()) {
            passage.section_type = result.category.clone();
        }

        let classified_counts: std::collections::HashMap<String, usize> = classify_results
            .iter()
            .fold(std::collections::HashMap::new(), |mut acc, r| {
                *acc.entry(r.category.clone()).or_insert(0) += 1;
                acc
            });
        tracing::info!(?classified_counts, "Section type classification complete");

        // ── Extract semantic triples (Gemma 4 classifier) ───────────
        if !config.triple_classifier.is_empty() {
            let triple_config = {
                let def = hkask_services_classify::load_classifier_config(
                    &config.triple_classifier,
                    registry_dir,
                )?;
                hkask_services_classify::ClassifierConfig::from_def(&def)
            };

            tracing::info!(
                total_passages = passage_count,
                model = %triple_config.model,
                concurrency = triple_config.concurrency,
                "Starting semantic triple extraction"
            );

            let triple_extractions =
                hkask_services_classify::extract_triples_batch(&texts, &triple_config).await?;

            for (passage, extraction) in all_passages.iter_mut().zip(triple_extractions.iter()) {
                passage.semantic_triples = extraction.clone();
            }

            let topics_extracted = triple_extractions
                .iter()
                .filter(|e| !e.topic.is_empty())
                .count();
            let total_concepts: usize = triple_extractions.iter().map(|e| e.concepts.len()).sum();
            tracing::info!(
                topics_extracted,
                total_concepts,
                total_passages = passage_count,
                "Semantic triple extraction complete"
            );
        } else {
            tracing::info!("Triple classifier disabled — skipping semantic extraction");
        }

        // ── Compute batch salience (graph centrality) ────────────────
        {
            let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
            p.phase = EmbedPhase::Tagging; // still in metadata phase
            p.current_work = "computing salience".into();
        }
        let all_tags: Vec<EntityTags> = all_passages.iter().map(|p| p.tags.clone()).collect();
        let salience_scores = salience::compute_salience_batch(&all_tags);
        for (passage, score) in all_passages.iter_mut().zip(salience_scores.iter()) {
            passage.salience = *score;
        }

        tracing::info!(
            total_passages = all_passages.len(),
            max_salience = salience_scores.iter().cloned().fold(0.0f32, f32::max),
            mean_salience =
                salience_scores.iter().sum::<f32>() / salience_scores.len().max(1) as f32,
            "Salience computed"
        );

        // ── Phase 3: Budget gate ───────────────────────────────────────
        {
            let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
            p.current_work = "applying budget gate".into();
        }
        let total_passages = all_passages.len();
        let budget = config.budget.resolve(total_passages);

        // Sort by salience descending, then determine which passages are
        // triple-eligible. Foundational rules always get triples (they
        // carry the style guide / exemplar text).
        let mut indexed: Vec<(usize, f32, usize)> = all_passages
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.salience, p.metadata_triple_count()))
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut triple_eligible: HashSet<usize> = HashSet::new();
        let mut triples_allocated = 0usize;

        for (idx, _salience, triple_cost) in &indexed {
            // Foundational rules always get triples regardless of budget
            if all_passages[*idx].is_rule {
                triple_eligible.insert(*idx);
                triples_allocated += *triple_cost;
                continue;
            }
            if triples_allocated + triple_cost <= budget {
                triple_eligible.insert(*idx);
                triples_allocated += triple_cost;
            }
        }

        let tagged_count = triple_eligible.len();
        let embedding_only = total_passages.saturating_sub(tagged_count);

        tracing::info!(
            total_passages = total_passages,
            budget = budget,
            tagged = tagged_count,
            embedding_only = embedding_only,
            triples_allocated = triples_allocated,
            "Budget gate applied"
        );

        // ── Phase 4: Embed all passages ────────────────────────────────
        tracing::info!(
            total_passages = total_passages,
            batch_size = config.embedding.batch_size,
            model = %config.embedding.model,
            "Starting embedding phase"
        );
        {
            let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
            p.phase = EmbedPhase::Embedding;
            p.current_work.clear();
            p.total_passages = total_passages;
            p.completed_passages = 0;
        }

        let inf_cfg = InferenceConfig::from_env();
        let embedder = EmbeddingRouter::new(inf_cfg);

        let batch_size = config.embedding.batch_size;
        let mut embedded_count = 0;
        let all_refs_and_texts: Vec<(&str, &str)> = all_passages
            .iter()
            .map(|p| (p.entity_ref.as_str(), p.text.as_str()))
            .collect();

        for chunk in all_refs_and_texts.chunks(batch_size) {
            let texts: Vec<&str> = chunk.iter().map(|(_, text)| *text).collect();
            let vectors = embedder
                .embed_sentences(&config.embedding.model, &texts)
                .await
                .map_err(|e| {
                    let msg = format!("Failed to embed batch: {e}");
                    ServiceError::Embed {
                        source: Some(Box::new(e)),
                        message: msg,
                    }
                })?;

            for ((entity_ref, _text), vector) in chunk.iter().zip(vectors.iter()) {
                semantic
                    .store_embedding(entity_ref, vector, &config.embedding.model)
                    .map_err(|e| ServiceError::SemanticMemory {
                        message: e.to_string(),
                    })?;
            }
            embedded_count += chunk.len();
            {
                let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
                p.completed_passages = embedded_count;
            }
            tracing::info!(
                embedded = embedded_count,
                total = total_passages,
                "Embedding progress"
            );
        }

        // ── Phase 5: Store triples for budget-selected passages ────────
        {
            let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
            p.phase = EmbedPhase::Triples;
            p.completed_passages = 0;
            p.total_passages = tagged_count;
        }

        let mut triples_stored = 0usize;
        let mut triple_progress = 0usize;

        for (i, passage) in all_passages.iter().enumerate() {
            if !triple_eligible.contains(&i) {
                continue;
            }

            store_passage_triples(&semantic, passage, &author, curator_webid)?;
            triples_stored += passage.triple_count();
            triple_progress += 1;

            {
                let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
                p.completed_passages = triple_progress;
            }
        }

        tracing::info!(
            triples_stored = triples_stored,
            tagged_passages = tagged_count,
            "Triples stored"
        );

        // ── Phase 6: Compute centroid(s) ────────────────────────────
        {
            let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
            p.phase = EmbedPhase::Centroid;
        }

        if config.dimension_centroids.is_empty() {
            // ── Legacy single-centroid path ──────────────────────────
            tracing::info!("Computing style centroid (single)");
            let rule_prefix = format!("style:{}:rule:", &config.author);
            let centroid_result = semantic
                .compute_centroid(
                    &author_prefix,
                    &rule_prefix,
                    &centroid_ref,
                    config.embedding.dim,
                    Some(&centroid_ref),
                    Some(&config.embedding.model),
                )
                .map_err(|e| ServiceError::SemanticMemory {
                    message: e.to_string(),
                })?;

            {
                let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
                p.phase = EmbedPhase::Done;
                p.completed_passages = total_passages;
            }

            return Ok(EmbedResult {
                author,
                purged,
                total_passages,
                centroid_ref,
                passage_count: centroid_result.passage_count,
                centroid_stored: centroid_result.stored,
                validation,
                budget,
                tagged_passages: tagged_count,
                triples_stored,
                embedding_only,
                dimension_centroids: Vec::new(),
            });
        }

        // ── Multi-dimension centroid path ────────────────────────────
        tracing::info!(
            dimensions = config.dimension_centroids.len(),
            "Computing per-dimension centroids"
        );

        // Create a second embedding store for centroid computation
        // (the primary one was moved into semantic)
        let centroid_store = EmbeddingStore::with_dim(Arc::clone(&conn), config.embedding.dim);

        // Build dimension → entity_refs map from all passages (excluding rules)
        let mut dim_refs: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for passage in &all_passages {
            if passage.is_rule || passage.dimension.is_empty() {
                continue;
            }
            dim_refs
                .entry(passage.dimension.clone())
                .or_default()
                .push(passage.entity_ref.clone());
        }

        let mut dim_centroids: Vec<(String, Vec<f32>, usize)> = Vec::new();

        for dc in &config.dimension_centroids {
            let refs = dim_refs.get(&dc.name);
            let count = refs.map(|r| r.len()).unwrap_or(0);

            if count == 0 {
                tracing::warn!(
                    dimension = %dc.name,
                    "No passages for dimension — skipping centroid"
                );
                continue;
            }

            let Some(refs) = refs else {
                continue;
            };

            let mut centroid = vec![0.0f32; config.embedding.dim];
            let mut fetched = 0usize;

            for entity_ref in refs {
                if let Ok(emb) = centroid_store.get(entity_ref) {
                    for (i, v) in emb.vector.iter().enumerate() {
                        if i < config.embedding.dim {
                            centroid[i] += v;
                        }
                    }
                    fetched += 1;
                }
            }

            if fetched == 0 {
                tracing::warn!(
                    dimension = %dc.name,
                    "No embeddings fetched for dimension — skipping centroid"
                );
                continue;
            }

            let n = fetched as f32;
            for v in centroid.iter_mut() {
                *v /= n;
            }

            // Store dimension centroid
            centroid_store
                .store(&dc.ref_name, &centroid, &config.embedding.model)
                .map_err(|e| {
                    let msg = format!("Failed to store dimension centroid: {e}");
                    ServiceError::Embed {
                        source: Some(Box::new(e)),
                        message: msg,
                    }
                })?;

            tracing::info!(
                dimension = %dc.name,
                ref_name = %dc.ref_name,
                passages = fetched,
                "Dimension centroid stored"
            );

            dim_centroids.push((dc.name.clone(), centroid, fetched));
        }

        // ── Compute composite centroid (weighted mean) ───────────────
        if !dim_centroids.is_empty() {
            let mut composite = vec![0.0f32; config.embedding.dim];
            let mut total_weight = 0.0f64;

            for dc in &config.dimension_centroids {
                if let Some((_name, vec, _count)) =
                    dim_centroids.iter().find(|(name, _, _)| name == &dc.name)
                {
                    for (i, v) in vec.iter().enumerate() {
                        composite[i] += *v * dc.weight as f32;
                    }
                    total_weight += dc.weight;
                }
            }

            if total_weight > 0.0 {
                // Normalize by total weight (handles missing dimensions)
                for v in composite.iter_mut() {
                    *v /= total_weight as f32;
                }

                centroid_store
                    .store(&centroid_ref, &composite, &config.embedding.model)
                    .map_err(|e| {
                        let msg = format!("Failed to store composite centroid: {e}");
                        ServiceError::Embed {
                            source: Some(Box::new(e)),
                            message: msg,
                        }
                    })?;

                tracing::info!(
                    composite_ref = %centroid_ref,
                    composite_weight = total_weight,
                    dimensions = dim_centroids.len(),
                    "Composite centroid stored"
                );
            }
        }

        let multi_passage_count: usize = dim_centroids.iter().map(|(_, _, c)| c).sum();

        // Build dimension centroid results for reporting
        let dim_results: Vec<DimensionCentroidResult> = dim_centroids
            .iter()
            .map(|(name, _vec, count)| {
                let ref_name = config
                    .dimension_centroids
                    .iter()
                    .find(|dc| &dc.name == name)
                    .map(|dc| dc.ref_name.clone())
                    .unwrap_or_default();
                DimensionCentroidResult {
                    name: name.clone(),
                    ref_name,
                    passage_count: *count,
                }
            })
            .collect();

        {
            let mut p = shared.lock().unwrap_or_else(|e| e.into_inner());
            p.phase = EmbedPhase::Done;
            p.completed_passages = total_passages;
        }

        Ok(EmbedResult {
            author,
            purged,
            total_passages,
            centroid_ref,
            passage_count: multi_passage_count,
            centroid_stored: !dim_centroids.is_empty(),
            validation,
            budget,
            tagged_passages: tagged_count,
            triples_stored,
            embedding_only,
            dimension_centroids: dim_results,
        })
    }

    /// Parse a corpus config YAML file.
    pub fn parse_config(path: &Path) -> Result<CorpusConfig, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.embed", operation = "parse_config", config = %path.display(), "CNS");

        let config_str = std::fs::read_to_string(path).map_err(|e| {
            let msg = format!("Failed to read corpus config {}: {e}", path.display());
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;
        serde_yaml_neo::from_str(&config_str).map_err(|e| {
            let msg = format!("Failed to parse corpus config YAML: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }
}

// ── Triple storage helpers ──────────────────────────────────────────────────

fn store_passage_triples(
    semantic: &SemanticMemory,
    passage: &TaggedPassage,
    author: &str,
    owner: WebID,
) -> Result<(), ServiceError> {
    let store = |entity: &str, attr: &str, value: serde_json::Value| -> Result<(), ServiceError> {
        let triple = Triple::new(entity, attr, value, owner).with_visibility(Visibility::Public);
        semantic.store(triple).map_err(|e| {
            let msg = format!("Failed to store triple ({entity}, {attr}): {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    };

    let er = &passage.entity_ref;

    // Passage text — required for exemplar retrieval in compose
    store(er, "text", json!(passage.text))?;

    // Structural metadata
    store(er, "author", json!(*author))?;
    store(er, "work_title", json!(passage.work_title))?;
    store(er, "work_slug", json!(passage.work_slug))?;
    store(er, "position", json!(passage.position))?;
    store(er, "word_count", json!(passage.signals.word_count))?;
    store(
        er,
        "avg_sentence_length",
        json!(passage.signals.avg_sentence_length),
    )?;

    // Entity tags (who, where, what, why)
    for c in &passage.tags.characters {
        store(er, "mentions_character", json!(c))?;
    }
    for p in &passage.tags.places {
        store(er, "mentions_place", json!(p))?;
    }
    for e in &passage.tags.events {
        store(er, "mentions_event", json!(e))?;
    }
    for c in &passage.tags.concepts {
        store(er, "mentions_concept", json!(c))?;
    }

    // Method tags (how)
    for m in &passage.tags.methods {
        store(er, "exhibits_method", json!(m))?;
    }

    // Method signals
    let s = &passage.signals;
    store(er, "parataxis_ratio", json!(s.parataxis_ratio))?;
    store(er, "adjective_density", json!(s.adjective_density))?;
    store(er, "adverb_density", json!(s.adverb_density))?;
    store(er, "passive_voice_ratio", json!(s.passive_voice_ratio))?;
    store(er, "dialogue_ratio", json!(s.dialogue_ratio))?;
    store(
        er,
        "sentence_length_variance",
        json!(s.sentence_length_variance),
    )?;
    store(er, "hedge_density", json!(s.hedge_density))?;
    store(er, "intensifier_density", json!(s.intensifier_density))?;
    store(er, "concrete_noun_ratio", json!(s.concrete_noun_ratio))?;
    store(er, "sensory_word_ratio", json!(s.sensory_word_ratio))?;

    // Salience
    store(er, "salience", json!(passage.salience))?;

    // Orthogonal tags (Gentle Lovelace dimensions)
    if !passage.dimension.is_empty() {
        store(er, "has_dimension", json!(passage.dimension))?;
    }
    if !passage.document_type.is_empty() {
        store(er, "document_type", json!(passage.document_type))?;
    }
    for cat in &passage.mds_categories {
        store(er, "has_mds_category", json!(cat))?;
    }
    if !passage.section_type.is_empty() {
        store(er, "has_section_type", json!(passage.section_type))?;
    }

    // Classifier-extracted semantic triples
    let st = &passage.semantic_triples;
    if !st.topic.is_empty() {
        store(er, "extracted_topic", json!(st.topic))?;
    }
    for concept in &st.concepts {
        store(er, "extracted_concept", json!(concept))?;
    }
    for entity in &st.entities {
        store(er, "extracted_entity", json!(entity))?;
    }
    for rel in &st.relationships {
        store(er, "extracted_relationship", json!(rel))?;
    }
    if !st.primary_dimension.is_empty() {
        store(er, "primary_dimension", json!(st.primary_dimension))?;
    }
    for flag in &st.quality_flags {
        store(er, "has_quality_flag", json!(flag))?;
    }

    // Extra fields from classifier (literary: themes, characters, setting, tone, imagery, etc.)
    for (key, val) in &st.extra {
        store(er, key, val.clone())?;
    }

    Ok(())
}

// ── Internal helpers ───────────────────────────────────────────────────────

async fn download_text(url: &str) -> Result<String, ServiceError> {
    let resp = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| {
            let msg = format!("Failed to build HTTP client: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?
        .get(url)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("HTTP request failed: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    if !resp.status().is_success() {
        return Err(ServiceError::Embed {
            source: None,
            message: format!("HTTP {} for {}", resp.status(), url),
        });
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = resp.bytes().await.map_err(|e| {
        let msg = format!("Failed to read response: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    // ── PDF detection: Content-Type or .pdf extension ──
    let is_pdf = content_type.contains("application/pdf")
        || url.ends_with(".pdf")
        || bytes.starts_with(b"%PDF");

    if is_pdf {
        // Write PDF bytes to a temp file for pdf-extract
        let tmp_dir = std::env::temp_dir();
        let tmp_path = tmp_dir.join(format!("hkask-download-{}.pdf", uuid::Uuid::new_v4()));
        std::fs::write(&tmp_path, &bytes).map_err(|e| {
            let msg = format!("Failed to write temp PDF: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

        let text = pdf_extract::extract_text(&tmp_path).map_err(|e| {
            let msg = format!("Failed to extract text from PDF '{}': {e}", url);
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

        // Clean up temp file
        let _ = std::fs::remove_file(&tmp_path);

        let word_count = text.split_whitespace().count();
        if word_count < 10 {
            tracing::warn!(
                url = %url,
                word_count = word_count,
                "PDF text extraction returned near-empty result — attempting OCR fallback"
            );

            // Attempt OCR via the configured LLM OCR model
            match ocr_pdf_bytes(&bytes, url).await {
                Ok(ocr_text) => {
                    let ocr_words = ocr_text.split_whitespace().count();
                    if ocr_words > word_count {
                        tracing::info!(
                            url = %url,
                            ocr_words = ocr_words,
                            extracted_words = word_count,
                            method = "ocr_fallback",
                            "OCR succeeded where text extraction failed"
                        );
                        return Ok(ocr_text);
                    }
                    tracing::warn!(
                        url = %url,
                        ocr_words = ocr_words,
                        "OCR also returned low word count — returning extraction result"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        url = %url,
                        error = %e,
                        "OCR fallback failed — returning extraction result"
                    );
                }
            }
        }

        tracing::info!(
            url = %url,
            word_count = word_count,
            method = "pdf_extract",
            "Downloaded and extracted PDF"
        );
        return Ok(text);
    }

    // ── HTML detection ──
    let is_html = content_type.contains("text/html")
        || content_type.contains("application/xhtml")
        || bytes.starts_with(b"<!DOCTYPE")
        || bytes.starts_with(b"<html");

    let raw = String::from_utf8_lossy(&bytes).to_string();

    if is_html {
        // Simple HTML tag stripping (same approach as RawFetchProvider)
        let text = strip_html_tags(&raw);
        tracing::info!(
            url = %url,
            word_count = text.split_whitespace().count(),
            method = "html_strip",
            "Downloaded and stripped HTML"
        );
        return Ok(text);
    }

    Ok(raw)
}

/// Strip HTML tags from text, decoding common entities and preserving
/// paragraph breaks from existing newlines in the HTML source.
///
/// [P7] Motivating: Evolutionary Architecture — HTML stripping utility emerged from embedding needs.
/// pre:  html is a valid HTML string
/// post: returns plain text with tags removed, common entities decoded, whitespace collapsed
pub fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut entity_buf = String::new();
    let mut in_entity = false;

    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
            continue;
        }
        if in_tag {
            if ch == '>' {
                in_tag = false;
            }
            continue;
        }
        if ch == '&' {
            in_entity = true;
            entity_buf.clear();
            entity_buf.push(ch);
            continue;
        }
        if in_entity {
            entity_buf.push(ch);
            if ch == ';' {
                in_entity = false;
                // Decode known entities, pass unknown ones through literally
                match entity_buf.as_str() {
                    "&amp;" => result.push('&'),
                    "&lt;" => result.push('<'),
                    "&gt;" => result.push('>'),
                    "&quot;" => result.push('"'),
                    "&apos;" => result.push('\''),
                    "&#160;" | "&nbsp;" => result.push(' '),
                    _ => {
                        // Unknown entity — pass through literally rather than drop
                        result.push_str(&entity_buf);
                    }
                }
            }
            continue;
        }
        if ch.is_whitespace() {
            result.push(' ');
        } else {
            result.push(ch);
        }
    }

    // If we ended mid-entity, push what we accumulated
    if in_entity {
        result.push_str(&entity_buf);
    }

    // Collapse multiple whitespace and blank lines
    let collapsed: String = result
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // Collapse multiple spaces within lines
    collapsed
        .split(' ')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Default OCR model for scanned PDF fallback.
/// Override via settings.json or HKASK_OCR_MODEL env var.
fn ocr_model() -> String {
    hkask_services_core::HkaskSettings::load().ocr_model()
}

/// OCR system prompt — instructs the vision model to extract text faithfully.
const OCR_SYSTEM_PROMPT: &str = "Extract all text from this document image. Output the text exactly as it appears, preserving the document structure and layout as closely as possible. If the document contains tables, preserve them in a readable format. Do not add commentary or description — only the extracted text.";

/// Attempt OCR on PDF bytes using pdftoppm decimation + per-page vision OCR.
///
/// 1. Writes PDF bytes to a temp file.
/// 2. Decimates to per-page PNG images via pdftoppm.
/// 3. OCRs each page via the inference router.
/// 4. Returns concatenated text.
///
/// Falls back to sending raw PDF bytes as base64 if pdftoppm is not installed.
pub async fn ocr_pdf_bytes(bytes: &[u8], url: &str) -> Result<String, ServiceError> {
    // P9: CNS span
    tracing::info!(target: "cns.embed", operation = "ocr_pdf_bytes", url = %url, byte_len = bytes.len(), "CNS");

    let ocr_model = std::env::var("HKASK_OCR_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(ocr_model);

    // Try pdftoppm decimation first
    if let Ok(text) = ocr_via_decimation(bytes, &ocr_model).await {
        return Ok(text);
    }

    // Fallback: send raw PDF bytes as base64 (legacy path)
    let b64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes);

    let inf_cfg = InferenceConfig::from_env();
    let router = InferenceRouter::new(inf_cfg);

    let params = LLMParameters {
        temperature: 0.1,
        top_p: 1.0,
        top_k: 1,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        min_p: 0.0,
        typical_p: 0.0,
        max_tokens: 4096,
        seed: None,
        disable_thinking: false,
        adapter: None,
    };

    match router
        .generate_vision(OCR_SYSTEM_PROMPT, &[b64_data], &params, Some(&ocr_model))
        .await
    {
        Ok(result) => Ok(result.text),
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("not found") {
                Err(ServiceError::Embed {
                    source: None,
                    message: format!(
                        "OCR model '{}' is not available. Ensure it is configured with a cloud provider prefix (e.g., DI/).\n\nOriginal PDF '{}' (source: {}) could not be text-extracted (likely scanned). Set HKASK_OCR_MODEL to override the default model.",
                        ocr_model, url, ocr_model
                    ),
                })
            } else {
                Err(ServiceError::Embed {
                    source: None,
                    message: format!("OCR inference failed for '{}': {}", url, err_msg),
                })
            }
        }
    }
}

/// Decimate PDF to page images and OCR each page individually.
///
/// Returns concatenated text from all pages, or an error if pdftoppm
/// is unavailable or OCR fails on any page.
async fn ocr_via_decimation(bytes: &[u8], model: &str) -> Result<String, String> {
    // Write bytes to temp PDF file
    let temp_dir = tempfile::tempdir().map_err(|e| format!("tempdir: {}", e))?;
    let pdf_path = temp_dir.path().join("input.pdf");
    std::fs::write(&pdf_path, bytes).map_err(|e| format!("write temp PDF: {}", e))?;

    // Decimate via pdftoppm
    let prefix = temp_dir.path().join("page");
    let output = std::process::Command::new("pdftoppm")
        .arg("-png")
        .arg("-r")
        .arg("200")
        .arg(&pdf_path)
        .arg(&prefix)
        .output()
        .map_err(|e| format!("pdftoppm not available: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "pdftoppm failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    // Collect page images
    let mut page = 1;
    let mut page_images: Vec<(usize, Vec<u8>)> = Vec::new();
    loop {
        let page_path = format!("{}-{}.png", prefix.display(), page);
        let path = std::path::Path::new(&page_path);
        if !path.exists() {
            break;
        }
        let png_bytes = std::fs::read(path).map_err(|e| format!("read page {}: {}", page, e))?;
        page_images.push((page, png_bytes));
        page += 1;
    }

    if page_images.is_empty() {
        return Err("pdftoppm produced no output images".into());
    }

    // OCR each page
    let inf_cfg = InferenceConfig::from_env();
    let router = InferenceRouter::new(inf_cfg);
    let params = LLMParameters {
        temperature: 0.1,
        max_tokens: 4096,
        ..Default::default()
    };

    let mut texts: Vec<String> = Vec::with_capacity(page_images.len());
    for (page_num, png_bytes) in &page_images {
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png_bytes);
        let result = router
            .generate_vision(OCR_SYSTEM_PROMPT, &[b64], &params, Some(model))
            .await
            .map_err(|e| format!("OCR failed for page {}: {}", page_num, e))?;
        if !result.text.trim().is_empty() {
            texts.push(result.text);
        }
    }

    Ok(texts.join("\n\n"))
}

// ── Tests ─────────────────────────────────────────────────────────────────
