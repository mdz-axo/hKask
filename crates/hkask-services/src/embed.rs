//! EmbedService — Style corpus embedding pipeline with metadata layer.
//! # REQ: P3 (Generative Space) — full parameter exposure, no hidden settings.
//!
//! ## Pipeline phases
//! 1. **Parse config** — YAML with entities, methods, budget, works
//! 2. **Download & chunk** — Gutenberg texts → tagged passages
//! 3. **Tag** — entity matching + method signal extraction
//! 4. **Salience** — weighted graph degree centrality per passage
//! 5. **Budget gate** — sort by salience, top-N by triple budget
//! 6. **Embed** — all passages get vectors (via Okapi/DeepInfra)
//! 7. **Store triples** — budget-selected passages get metadata triples
//! 8. **Centroid** — mean vector over prose passages

use hkask_memory::SemanticMemory;
use hkask_memory::salience::{self, BudgetConfig, DeclaredMethod, EntityTags, MethodSignals};
use hkask_storage::{Database, EmbeddingStore, Triple, TripleStore};
use hkask_templates::{OkapiConfig, OkapiEmbedding};
use hkask_types::Visibility;
use hkask_types::id::WebID;

use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::ServiceError;

// ── Re-exports ─────────────────────────────────────────────────────────────

pub use hkask_memory::salience::{
    BudgetConfig as BudgetConfigReexport, DeclaredMethod as DeclaredMethodReexport,
    MethodSignals as MethodSignalsReexport,
};

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
#[derive(Debug, Deserialize)]
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
}

/// Entity declarations for corpus-specific tagging.
#[derive(Debug, Default, Deserialize)]
pub struct EntityConfig {
    #[serde(default)]
    pub characters: Vec<Entity>,
    #[serde(default)]
    pub places: Vec<Entity>,
    #[serde(default)]
    pub events: Vec<Entity>,
    #[serde(default)]
    pub concepts: Vec<Entity>,
}

/// A declared entity with name and optional per-work scoping.
#[derive(Debug, Clone, Deserialize)]
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
}

impl TaggedPassage {
    /// Count how many metadata triples this passage would consume if stored.
    /// Excludes the `text` triple — text is stored for all passages regardless
    /// of budget, since it's required for exemplar retrieval in compose.
    fn metadata_triple_count(&self) -> usize {
        // 6 structural + entity tags + method tags + 1 salience + 10 signals
        6 + self.tags.characters.len()
            + self.tags.places.len()
            + self.tags.events.len()
            + self.tags.concepts.len()
            + self.tags.methods.len()
            + 1
            + 11 // salience + 10 method signals
    }

    /// Total triple count including text (for reporting only).
    fn triple_count(&self) -> usize {
        1 + self.metadata_triple_count()
    }
}

// ── Result ─────────────────────────────────────────────────────────────────

/// Result of the embedding pipeline with budget statistics.
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
        okapi_url: Option<&str>,
        cache_dir: Option<&Path>,
        progress: Option<ProgressFn>,
    ) -> Result<EmbedResult, ServiceError> {
        let started = Instant::now();

        // ── Phase 1: Parse config ──────────────────────────────────────
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

        // ── Open DB ────────────────────────────────────────────────────
        let db = Database::open(db_path, db_passphrase)?;
        let conn = db.conn_arc();
        let triple_store = TripleStore::new(Arc::clone(&conn));
        let embedding_store = EmbeddingStore::with_dim(conn, config.embedding.dim);
        let semantic = SemanticMemory::new(triple_store, embedding_store);

        // Purge existing embeddings for idempotent re-ingest
        let purged = semantic
            .purge_by_prefix(&author_prefix)
            .map_err(|e| ServiceError::Embed(format!("Failed to purge embeddings: {e}")))?;

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
            let mut p = shared.lock().unwrap();
            p.phase = EmbedPhase::Tagging;
        }

        let mut all_passages: Vec<TaggedPassage> = Vec::new();

        for (work_idx, work) in config.works.iter().enumerate() {
            if work_idx > 0 {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            {
                let mut p = shared.lock().unwrap();
                p.current_work = work.title.clone();
                p.completed_passages = work_idx + 1;
                p.total_passages = config.works.len();
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
            });
        }

        // ── Compute batch salience (graph centrality) ────────────────
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
        {
            let mut p = shared.lock().unwrap();
            p.phase = EmbedPhase::Embedding;
            p.current_work.clear();
            p.total_passages = total_passages;
            p.completed_passages = 0;
        }

        let okapi_config = match okapi_url {
            Some(url) => OkapiConfig {
                base_url: url.to_string(),
                ..OkapiConfig::default()
            },
            None => OkapiConfig::local_dev(),
        };
        let embedder = OkapiEmbedding::with_model(&config.embedding.model, okapi_config)?;

        let batch_size = config.embedding.batch_size;
        let mut embedded_count = 0;
        let all_refs_and_texts: Vec<(&str, &str)> = all_passages
            .iter()
            .map(|p| (p.entity_ref.as_str(), p.text.as_str()))
            .collect();

        for chunk in all_refs_and_texts.chunks(batch_size) {
            let texts: Vec<&str> = chunk.iter().map(|(_, text)| *text).collect();
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
                total = total_passages,
                "Embedding progress"
            );
        }

        // ── Phase 5: Store triples for budget-selected passages ────────
        {
            let mut p = shared.lock().unwrap();
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
                let mut p = shared.lock().unwrap();
                p.completed_passages = triple_progress;
            }
        }

        tracing::info!(
            triples_stored = triples_stored,
            tagged_passages = tagged_count,
            "Triples stored"
        );

        // ── Phase 6: Compute centroid ──────────────────────────────────
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
            p.completed_passages = total_passages;
        }

        Ok(EmbedResult {
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
            ServiceError::Embed(format!("Failed to store triple ({entity}, {attr}): {e}"))
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

    Ok(())
}

// ── Internal helpers ───────────────────────────────────────────────────────

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

// ── Tests ─────────────────────────────────────────────────────────────────
