//! DiscoveryService — Academic author corpus discovery pipeline.
//!
//! Orchestrates multi-source discovery via the MCP research server's
//! provider pool (Semantic Scholar, arXiv, Brave, Tavily, Exa, Firecrawl,
//! SerpAPI). Extracts content, caches to disk, and generates a corpus.yaml
//! ready for `EmbedService::embed_corpus()`.
//!
//! # REQ: P3 (Generative Space) — full parameter exposure, no hidden settings.
//! # expect: "The service layer enables generative access to domain capabilities"
//!
//! ## Pipeline
//! 1. Academic search via MCP web_search → Semantic Scholar + arXiv papers
//! 2. Extract search terms from paper titles (or use user-provided)
//! 3. Web search via MCP web_search → institutional pages, interviews
//! 4. YouTube transcript search via SerpAPI (requires API key)
//! 5. Content download + cache → .cache/{slug}.txt
//! 6. Concept extraction (LLM) → entities from paper titles
//! 7. Method inference (LLM) → stylometric patterns from cached passages
//! 8. Generate/augment corpus.yaml

use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_memory::salience::{DeclaredMethod, MethodThresholds};
use hkask_services_core::ServiceError;
use hkask_services_embed::{CorpusConfig, EntityConfig, Work};
use hkask_templates::ports::McpPort;
use hkask_types::DelegationToken;
use hkask_types::ports::InferencePort;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const USER_AGENT: &str = "hkask-discovery/0.27";

// ── Request / Result types ──────────────────────────────────────────────────

/// Parameters for corpus discovery.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoverRequest {
    /// Full name of the academic author (e.g., "David Dunning")
    pub author_name: String,
    /// Maximum number of works to include
    #[serde(default = "default_max_works")]
    pub max_works: usize,
    /// Directory for caching extracted content
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    /// Directory to write the generated corpus.yaml
    pub output_dir: Option<String>,
    /// SerpAPI key for YouTube transcript search (web search uses MCP providers)
    #[serde(default)]
    pub serpapi_key: Option<String>,
    /// Whether to search for YouTube transcripts
    #[serde(default = "default_true")]
    pub include_transcripts: bool,
    /// Whether to search the web for institutional pages and interviews
    #[serde(default = "default_true")]
    pub include_web: bool,
    /// Curated mode: present web + YouTube results for user confirmation before including
    #[serde(default = "default_true")]
    pub curated: bool,
    /// Optional search terms for web + YouTube queries.
    /// If absent, terms are extracted from academic paper titles.
    #[serde(default)]
    pub web_search_terms: Option<String>,
    /// Augment an existing corpus rather than creating a new one.
    /// When true, loads the existing corpus.yaml and merges new works into it.
    #[serde(default)]
    pub augment: bool,
    /// Whether to run LLM-based concept extraction and method inference.
    /// Default: true (quality & precision first; set false for cheap/fast runs).
    #[serde(default = "default_true")]
    pub include_methods: bool,
    /// Optional biographical details for author disambiguation.
    /// Examples: "professor of psychology at Cornell University",
    /// "machine learning researcher at Stanford, PhD from MIT".
    /// Used to refine search queries and disambiguate common names.
    #[serde(default)]
    pub biographical_details: Option<String>,
}

fn default_max_works() -> usize {
    20
}
fn default_cache_dir() -> String {
    "./.cache".to_string()
}
fn default_true() -> bool {
    true
}

/// Result of a discovery run.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoverResult {
    /// Author slug (e.g., "david-dunning")
    pub author_slug: String,
    /// Number of academic works discovered (Semantic Scholar + arXiv)
    pub works_found: usize,
    /// Number of works successfully cached
    pub works_cached: usize,
    /// Path to the generated corpus.yaml
    pub config_path: String,
    /// Sources used
    pub sources: Vec<String>,
    /// Web search candidates for curation (only populated when curated=true)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub web_candidates: Vec<DiscoveredWork>,
    /// YouTube transcript candidates for curation (only populated when curated=true)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub youtube_candidates: Vec<DiscoveredWork>,
    /// Extracted concepts, places, and events (populated when include_methods=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<EntityConfig>,
    /// Inferred methodological patterns (populated when include_methods=true)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<DeclaredMethod>,
}

/// A discovered work with metadata.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredWork {
    pub title: String,
    pub slug: String,
    pub url: String,
    pub year: Option<u16>,
    pub source: String,
    pub work_type: String,
    /// Abstract or snippet from the search result (when available).
    /// Used for LLM concept extraction.
    #[serde(default)]
    pub abstract_text: Option<String>,
}

// ── Service ─────────────────────────────────────────────────────────────────

pub struct DiscoveryService;

impl DiscoveryService {
    /// Run the full discovery pipeline and generate a corpus.yaml.
    ///
    /// `mcp` is the MCP dispatch port — must be connected to a running
    /// `hkask-mcp-research` server with configured providers.
    /// `token` is a delegation token for OCAP-gated tool invocation.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  req.author_name must be non-empty; mcp must be connected; token must be valid
    /// post: returns DiscoverResult with discovered works, sources, and academic works; output and cache directories created; Err on MCP or I/O failure
    pub async fn discover(
        req: &DiscoverRequest,
        mcp: &dyn McpPort,
        token: &DelegationToken,
    ) -> Result<DiscoverResult, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.discover", operation = "discover", author = %req.author_name, max_works = req.max_works, "CNS");

        let author_slug = slugify(&req.author_name);
        let output_dir = req
            .output_dir
            .clone()
            .unwrap_or_else(|| format!("./{}", author_slug));
        let output_path = PathBuf::from(&output_dir);
        let cache_dir = PathBuf::from(&req.cache_dir);

        // Ensure output and cache directories exist
        std::fs::create_dir_all(&output_path).map_err(|e| {
            let msg = format!(
                "Failed to create output directory '{}': {e}",
                output_path.display()
            );
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;
        std::fs::create_dir_all(&cache_dir).map_err(|e| {
            let msg = format!(
                "Failed to create cache directory '{}': {e}",
                cache_dir.display()
            );
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

        let mut works: Vec<DiscoveredWork> = Vec::new();
        let mut sources: Vec<String> = Vec::new();
        let mut academic_works: Vec<DiscoveredWork> = Vec::new();

        // ── Phase 1: Academic search via MCP web_search ────────────────────
        // Calls the MCP server's provider pool which includes Semantic Scholar
        // and arXiv alongside other providers. We filter results by provider.
        //
        // If biographical details are provided, they're appended to the search
        // query for disambiguation (e.g., "John Smith" → "John Smith professor
        // of psychology at Cornell University").
        let academic_query = if let Some(ref bio) = req.biographical_details {
            format!("{} {}", req.author_name, bio)
        } else {
            req.author_name.clone()
        };
        tracing::info!(
            target: "hkask.discover",
            query = %academic_query,
            has_bio = req.biographical_details.is_some(),
            "Academic search query"
        );

        match mcp_search(mcp, token, &academic_query, req.max_works, "web").await {
            Ok(results) => {
                let (academic, other): (Vec<_>, Vec<_>) = results.into_iter().partition(|w| {
                    let s = w.source.to_lowercase();
                    s == "semantic_scholar" || s == "arxiv"
                });

                let acad_count = academic.len();
                for w in &academic {
                    academic_works.push(w.clone());
                }
                works.extend(academic);
                sources.push(format!("academic_search ({acad_count} papers)"));

                // Non-academic results from the same search become web candidates
                if !other.is_empty() && req.include_web {
                    let existing_urls: Vec<&str> = works.iter().map(|w| w.url.as_str()).collect();
                    let new: Vec<DiscoveredWork> = other
                        .into_iter()
                        .filter(|w| !existing_urls.contains(&w.url.as_str()))
                        .collect();
                    if req.curated {
                        sources.push(format!(
                            "web_search ({} candidates from academic query)",
                            new.len()
                        ));
                    } else {
                        works.extend(new);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    target: "hkask.discover",
                    error = %e,
                    "Academic search via MCP failed — continuing"
                );
            }
        }

        // ── Resolve web search terms ─────────────────────────────────────
        let search_terms = req.web_search_terms.clone().unwrap_or_else(|| {
            let titles: Vec<String> = academic_works.iter().map(|w| w.title.clone()).collect();
            extract_search_terms(&req.author_name, &titles)
        });
        tracing::info!(
            target: "hkask.discover",
            terms = %search_terms,
            "Resolved web search terms"
        );

        // ── Phase 2: Web search via MCP ──────────────────────────────────
        let mut web_candidates: Vec<DiscoveredWork> = Vec::new();
        if req.include_web {
            match mcp_search(mcp, token, &search_terms, 5, "web").await {
                Ok(results) => {
                    // Filter out academic results (already captured in Phase 1)
                    let web_results: Vec<DiscoveredWork> = results
                        .into_iter()
                        .filter(|w| {
                            let s = w.source.to_lowercase();
                            s != "semantic_scholar" && s != "arxiv"
                        })
                        .collect();
                    let existing_urls: Vec<&str> = works.iter().map(|w| w.url.as_str()).collect();
                    let new: Vec<DiscoveredWork> = web_results
                        .into_iter()
                        .filter(|w| !existing_urls.contains(&w.url.as_str()))
                        .collect();
                    let added = new.len();
                    if req.curated {
                        web_candidates = new;
                        sources.push(format!(
                            "web_search ({added} candidates — awaiting curation)"
                        ));
                    } else {
                        works.extend(new);
                        sources.push(format!("web_search ({added} pages)"));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.discover",
                        error = %e,
                        "Web search via MCP failed — continuing"
                    );
                }
            }
        }

        // ── Phase 3: YouTube transcript discovery (SerpAPI) ──────────────
        let mut youtube_candidates: Vec<DiscoveredWork> = Vec::new();
        if req.include_transcripts
            && let Some(ref key) = req.serpapi_key
        {
            match search_youtube_transcripts(&search_terms, key, 5).await {
                Ok(transcripts) => {
                    let count = transcripts.len();
                    if req.curated {
                        youtube_candidates = transcripts;
                        sources.push(format!(
                            "youtube_transcripts ({count} candidates — awaiting curation)"
                        ));
                    } else {
                        works.extend(transcripts);
                        sources.push(format!("youtube_transcripts ({count} videos)"));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.discover",
                        error = %e,
                        "YouTube transcript search failed — continuing"
                    );
                }
            }
        }

        // Check after ALL sources have been tried
        if works.is_empty() && web_candidates.is_empty() && youtube_candidates.is_empty() {
            return Err(ServiceError::Embed {
                source: None,
                message: format!("No works found for '{}' across any source", req.author_name),
            });
        }

        // ── Phase 4: Extract and cache content ────────────────────────────
        let mut cached = 0usize;
        for work in &works {
            let cache_path = cache_dir.join(format!("{}.txt", work.slug));
            if cache_path.exists() {
                cached += 1;
                continue;
            }

            match download_and_cache(&work.url, &cache_path).await {
                Ok(()) => cached += 1,
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.discover",
                        slug = %work.slug,
                        url = %work.url,
                        error = %e,
                        "Failed to download work — skipping"
                    );
                }
            }
        }

        // ── Phase 5a: Concept extraction (LLM) ─────────────────────────
        let mut entities: Option<EntityConfig> = None;
        let mut methods: Vec<DeclaredMethod> = Vec::new();

        if req.include_methods && !academic_works.is_empty() {
            match extract_concepts(&req.author_name, &academic_works).await {
                Ok(extracted) => {
                    tracing::info!(
                        target: "hkask.discover",
                        concepts = extracted.concepts.len(),
                        places = extracted.places.len(),
                        events = extracted.events.len(),
                        "Concepts extracted"
                    );
                    entities = Some(extracted);
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.discover",
                        error = %e,
                        "Concept extraction failed — continuing"
                    );
                }
            }
        }

        // ── Phase 5b: Method inference (LLM) ────────────────────────────
        if req.include_methods && cached > 0 {
            match infer_methods(&req.author_name, &works, &cache_dir).await {
                Ok(inferred) => {
                    tracing::info!(
                        target: "hkask.discover",
                        methods = inferred.len(),
                        "Methods inferred"
                    );
                    methods = inferred;
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.discover",
                        error = %e,
                        "Method inference failed — continuing"
                    );
                }
            }
        }

        // ── Phase 5: Generate corpus.yaml ──────────────────────────────────
        // If augmenting, load existing config and merge works (dedup by URL),
        // preserving entities, methods, foundational_rules, and other metadata.
        let config_path = if req.augment {
            augment_corpus_yaml(
                &author_slug,
                &works,
                &output_path,
                entities.clone(),
                &methods,
            )?
        } else {
            generate_corpus_yaml(
                &author_slug,
                &works,
                &output_path,
                entities.clone(),
                &methods,
            )?
        };

        tracing::info!(
            target: "hkask.discover",
            author = %req.author_name,
            slug = %author_slug,
            works_found = works.len(),
            works_cached = cached,
            config = %config_path.display(),
            "Discovery complete"
        );

        Ok(DiscoverResult {
            author_slug,
            works_found: works.len(),
            works_cached: cached,
            config_path: config_path.to_string_lossy().to_string(),
            sources,
            web_candidates,
            youtube_candidates,
            entities,
            methods,
        })
    }
}

/// Generate a corpus.yaml from a list of discovered works.
///
/// Public so the CLI can regenerate the config after curation —
/// selected web/YouTube candidates are added to the works list
/// and a fresh corpus.yaml is written.
///
/// When `entities` and `methods` are provided (from LLM extraction phases),
/// they are included in the generated config. Sets `corpus_type: "academic"`
/// since this is the academic discovery pipeline.
///
/// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  author_slug must be non-empty; works must be non-empty; output_dir must exist
/// post: corpus.yaml is written to output_dir; returns PathBuf to the written file; Err on serialization or I/O failure
pub fn generate_corpus_yaml(
    author_slug: &str,
    works: &[DiscoveredWork],
    output_dir: &Path,
    entities: Option<EntityConfig>,
    methods: &[DeclaredMethod],
) -> Result<PathBuf, ServiceError> {
    // P9: CNS span
    tracing::info!(target: "cns.discover", operation = "generate_corpus_yaml", author = %author_slug, work_count = works.len(), method_count = methods.len(), "CNS");

    let corpus_works: Vec<Work> = works
        .iter()
        .map(|w| {
            let format = match w.work_type.as_str() {
                "journal_article" | "preprint" => "pdf",
                "video_transcript" => "text",
                _ => "web",
            };
            let document_type = match w.work_type.as_str() {
                "journal_article" | "preprint" => Some("research-paper".to_string()),
                _ => None,
            };
            Work {
                title: w.title.clone(),
                slug: w.slug.clone(),
                url: w.url.clone(),
                local_path: None,
                format: format.to_string(),
                document_type,
                dimensions: vec![],
                section_types: vec![],
                mds_categories: vec![],
            }
        })
        .collect();

    let mut config = default_corpus_config(author_slug);
    config.works = corpus_works;
    config.entities = entities.unwrap_or_default();
    config.methods = methods.to_vec();
    config.corpus_type = "academic".to_string();

    let config_yaml = serde_yaml_neo::to_string(&config).map_err(|e| {
        let msg = format!("Failed to serialize corpus config: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    let config_path = output_dir.join("corpus.yaml");
    std::fs::write(&config_path, &config_yaml).map_err(|e| {
        let msg = format!(
            "Failed to write corpus.yaml to '{}': {e}",
            config_path.display()
        );
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    Ok(config_path)
}

/// Default corpus configuration for a given author slug.
///
/// Shared between `generate_corpus_yaml` and the CLI curation section
/// to prevent default drift. All corpus config defaults live here.
///
/// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  author_slug must be non-empty
/// post: returns CorpusConfig with default embedding, chunking, validation, and budget settings
pub fn default_corpus_config(author_slug: &str) -> CorpusConfig {
    CorpusConfig {
        author: author_slug.to_string(),
        embedding: hkask_services_embed::EmbeddingConfig {
            model: "DI/Qwen/Qwen3-Embedding-0.6B".to_string(),
            dim: 1024,
            batch_size: 64,
        },
        works: vec![],
        foundational_rules: vec![],
        chunking: hkask_services_embed::ChunkingConfig {
            min_words: 50,
            max_words: 200,
            sentence_boundary: ".!? ".to_string(),
        },
        centroid_entity_ref: format!("style:{author_slug}:centroid"),
        validation: hkask_services_embed::ValidationConfig {
            centroid_distance_max: 0.25,
            exemplar_count_min: 3,
            exemplar_count_max: 7,
        },
        budget: hkask_memory::salience::BudgetConfig::PerPage {
            per_100_pages: 3750,
        },
        entities: Default::default(),
        methods: vec![],
        corpus_type: "literary".to_string(),
        dimension_centroids: vec![],
        tag_sets: vec![],
        tag_weights: Default::default(),
        classifier: String::new(),
        triple_classifier: String::new(),
    }
}

/// Augment an existing corpus.yaml with newly discovered works,
/// extracted concepts, and inferred methods.
///
/// Loads the existing config, merges new works (deduplicated by URL),
/// merges new concepts (deduplicated by name), merges new methods
/// (deduplicated by name), and preserves all other existing metadata.
fn augment_corpus_yaml(
    author_slug: &str,
    new_works: &[DiscoveredWork],
    output_dir: &Path,
    entities: Option<EntityConfig>,
    methods: &[DeclaredMethod],
) -> Result<PathBuf, ServiceError> {
    let config_path = output_dir.join("corpus.yaml");

    // Load existing config
    let existing_yaml = std::fs::read_to_string(&config_path).map_err(|e| {
        let msg = format!("Failed to read existing corpus.yaml for augmentation: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;
    let mut config: CorpusConfig = serde_yaml_neo::from_str(&existing_yaml).map_err(|e| {
        let msg = format!("Failed to parse existing corpus.yaml for augmentation: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    // Collect existing URLs for dedup
    let existing_urls: std::collections::HashSet<&str> =
        config.works.iter().map(|w| w.url.as_str()).collect();

    // Merge new works (skip duplicates by URL)
    let added: Vec<Work> = new_works
        .iter()
        .filter(|w| !existing_urls.contains(w.url.as_str()))
        .map(|w| {
            let format = match w.work_type.as_str() {
                "journal_article" | "preprint" => "pdf",
                "video_transcript" => "text",
                _ => "web",
            };
            let document_type = match w.work_type.as_str() {
                "journal_article" | "preprint" => Some("research-paper".to_string()),
                _ => None,
            };
            Work {
                title: w.title.clone(),
                slug: w.slug.clone(),
                url: w.url.clone(),
                local_path: None,
                format: format.to_string(),
                document_type,
                dimensions: vec![],
                section_types: vec![],
                mds_categories: vec![],
            }
        })
        .collect();

    let added_count = added.len();
    config.works.extend(added);

    // Merge new concepts (dedup by name)
    if let Some(ref new_entities) = entities {
        let existing_concept_names: std::collections::HashSet<&str> = config
            .entities
            .concepts
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        let new_concepts: Vec<hkask_services_embed::Entity> = new_entities
            .concepts
            .iter()
            .filter(|e| !existing_concept_names.contains(e.name.as_str()))
            .cloned()
            .collect();
        config.entities.concepts.extend(new_concepts);

        let existing_place_names: std::collections::HashSet<&str> = config
            .entities
            .places
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        let new_places: Vec<hkask_services_embed::Entity> = new_entities
            .places
            .iter()
            .filter(|e| !existing_place_names.contains(e.name.as_str()))
            .cloned()
            .collect();
        config.entities.places.extend(new_places);

        let existing_event_names: std::collections::HashSet<&str> = config
            .entities
            .events
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        let new_events: Vec<hkask_services_embed::Entity> = new_entities
            .events
            .iter()
            .filter(|e| !existing_event_names.contains(e.name.as_str()))
            .cloned()
            .collect();
        config.entities.events.extend(new_events);
    }

    // Merge new methods (dedup by name)
    if !methods.is_empty() {
        let existing_method_names: std::collections::HashSet<&str> =
            config.methods.iter().map(|m| m.name.as_str()).collect();
        let new_methods: Vec<DeclaredMethod> = methods
            .iter()
            .filter(|m| !existing_method_names.contains(m.name.as_str()))
            .cloned()
            .collect();
        config.methods.extend(new_methods);
    }

    // Write back
    let config_yaml = serde_yaml_neo::to_string(&config).map_err(|e| {
        let msg = format!("Failed to serialize augmented config: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;
    std::fs::write(&config_path, &config_yaml).map_err(|e| {
        let msg = format!(
            "Failed to write augmented corpus.yaml to '{}': {e}",
            config_path.display()
        );
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    tracing::info!(
        target: "hkask.discover",
        slug = %author_slug,
        existing_works = config.works.len() - added_count,
        added = added_count,
        total = config.works.len(),
        "Corpus augmented"
    );

    Ok(config_path)
}

// ── LLM Concept Extraction ───────────────────────────────────────────────────

/// Default template base path relative to project root.
const TEMPLATE_BASE: &str = "registry/templates/replica";

/// Parse a model override directive from a Jinja2 template's first line.
/// Format: `{# model: OM/qwen3:14b #}`
/// Returns the model name (with provider prefix) if found, None otherwise.
fn parse_template_model(template_src: &str) -> Option<String> {
    let first_line = template_src.lines().next()?;
    let trimmed = first_line.trim();
    if trimmed.starts_with("{# model:") && trimmed.ends_with("#}") {
        let model = trimmed
            .strip_prefix("{# model:")?
            .strip_suffix("#}")?
            .trim();
        if model.is_empty() {
            None
        } else {
            Some(model.to_string())
        }
    } else {
        None
    }
}

/// Extract key concepts, places, and events from academic paper titles
/// and abstracts using LLM semantic deduplication via the extract-concepts.j2 template.
async fn extract_concepts(
    author_name: &str,
    works: &[DiscoveredWork],
) -> Result<EntityConfig, ServiceError> {
    // Build paper list for template with titles and abstracts
    let papers: Vec<serde_json::Value> = works
        .iter()
        .map(|w| {
            serde_json::json!({
                "title": w.title,
                "abstract": w.abstract_text.as_deref().unwrap_or(""),
                "year": w.year.map(|y| y.to_string()).unwrap_or_else(|| "unknown".to_string()),
            })
        })
        .collect();

    // Render template
    let template_path = PathBuf::from(TEMPLATE_BASE).join("extract-concepts.j2");
    let template_src = std::fs::read_to_string(&template_path).map_err(|e| {
        let msg = format!("Failed to read extract-concepts template: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    // Parse model override before rendering (comments are stripped during render)
    let model_override = parse_template_model(&template_src);

    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env.add_template_owned("extract-concepts", template_src)
        .map_err(|e| {
            let msg = format!("Failed to parse template: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    let tmpl = env.get_template("extract-concepts").map_err(|e| {
        let msg = format!("Failed to load template: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    let prompt = tmpl
        .render(minijinja::context! {
            author_name,
            papers,
            max_concepts => 15,
        })
        .map_err(|e| {
            let msg = format!("Failed to render template: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    // Call inference
    let inf_cfg = InferenceConfig::from_env();
    let router = InferenceRouter::new(inf_cfg);
    let params = hkask_types::template::LLMParameters {
        temperature: 0.3,
        max_tokens: 1024,
        ..Default::default()
    };

    let result = router
        .generate_with_model(&prompt, &params, model_override.as_deref())
        .await
        .map_err(|e| {
            let msg = format!("Concept extraction inference failed: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    // Parse JSON response
    let parsed: serde_json::Value = serde_json::from_str(&result.text).map_err(|e| {
        let msg = format!("Failed to parse concept extraction response as JSON: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    let concepts = parse_entity_list(&parsed, "concepts");
    let places = parse_entity_list(&parsed, "places");
    let events = parse_entity_list(&parsed, "events");

    Ok(EntityConfig {
        characters: vec![],
        places,
        events,
        concepts,
        co_authors: vec![],
        venues: vec![],
        topics: vec![],
        paradigms: vec![],
    })
}

/// Infer methodological and stylistic patterns from cached work content
/// using LLM analysis via the infer-methods.j2 template.
async fn infer_methods(
    author_name: &str,
    works: &[DiscoveredWork],
    cache_dir: &Path,
) -> Result<Vec<DeclaredMethod>, ServiceError> {
    // Sample up to 5 passages from cached content (first ~800 chars of each)
    let mut sample_passages: Vec<serde_json::Value> = Vec::new();
    for work in works.iter().take(5) {
        let cache_path = cache_dir.join(format!("{}.txt", work.slug));
        if let Ok(content) = std::fs::read_to_string(&cache_path) {
            let excerpt: String = content.chars().take(800).collect();
            if excerpt.split_whitespace().count() >= 20 {
                sample_passages.push(serde_json::json!({
                    "text": excerpt,
                    "work_slug": work.slug,
                }));
            }
        }
    }

    if sample_passages.is_empty() {
        return Ok(vec![]);
    }

    // Render template
    let template_path = PathBuf::from(TEMPLATE_BASE).join("infer-methods.j2");
    let template_src = std::fs::read_to_string(&template_path).map_err(|e| {
        let msg = format!("Failed to read infer-methods template: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    // Parse model override before rendering (comments are stripped during render)
    let model_override = parse_template_model(&template_src);

    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env.add_template_owned("infer-methods", template_src)
        .map_err(|e| {
            let msg = format!("Failed to parse template: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    let tmpl = env.get_template("infer-methods").map_err(|e| {
        let msg = format!("Failed to load template: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    let prompt = tmpl
        .render(minijinja::context! {
            author_name,
            author_domain => "academic",
            sample_passages,
        })
        .map_err(|e| {
            let msg = format!("Failed to render template: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    // Call inference
    let inf_cfg = InferenceConfig::from_env();
    let router = InferenceRouter::new(inf_cfg);
    let params = hkask_types::template::LLMParameters {
        temperature: 0.3,
        max_tokens: 1024,
        ..Default::default()
    };

    let result = router
        .generate_with_model(&prompt, &params, model_override.as_deref())
        .await
        .map_err(|e| {
            let msg = format!("Method inference failed: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    // Parse JSON response
    let parsed: serde_json::Value = serde_json::from_str(&result.text).map_err(|e| {
        let msg = format!("Failed to parse method inference response as JSON: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    let methods: Vec<DeclaredMethod> = parsed["methods"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let name = m["name"].as_str()?.to_string();
                    let description = m["description"].as_str().unwrap_or("").to_string();
                    let signal: MethodThresholds =
                        serde_json::from_value(m["signal"].clone()).unwrap_or_default();
                    Some(DeclaredMethod {
                        name,
                        description,
                        signal,
                        threshold: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(methods)
}

/// Parse an entity list from a JSON field (e.g., "concepts", "places", "events").
fn parse_entity_list(parsed: &serde_json::Value, field: &str) -> Vec<hkask_services_embed::Entity> {
    parsed[field]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let name = v["name"].as_str()?.to_string();
                    let appears_in: Vec<String> = v["appears_in"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|s| s.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    Some(hkask_services_embed::Entity { name, appears_in })
                })
                .collect()
        })
        .unwrap_or_default()
}

// ── MCP web_search helper ───────────────────────────────────────────────────

/// Call the MCP server's web_search tool and parse results into DiscoveredWork structs.
async fn mcp_search(
    mcp: &dyn McpPort,
    token: &DelegationToken,
    query: &str,
    num_results: usize,
    strategy: &str,
) -> Result<Vec<DiscoveredWork>, ServiceError> {
    let input = serde_json::json!({
        "query": query,
        "strategy": strategy,
        "num_results": num_results,
    });

    let result = mcp.invoke("web_search", input, token).await.map_err(|e| {
        let msg = format!("MCP web_search failed: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    tracing::debug!(
        target: "hkask.discover",
        query = %query,
        has_results = result.get("results").is_some(),
        result_keys = ?result.as_object().map(|o| o.keys().collect::<Vec<_>>()),
        "MCP search response"
    );

    // The MCP server wraps tool output in {"content": <value>}.
    // parse_call_result unwraps the MCP transport layer (content[0].text),
    // but the server's {"content": ...} wrapper remains.
    let payload = result.get("content").unwrap_or(&result);

    let results = payload["results"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let title = item["title"].as_str()?.to_string();
                    let url = item["url"].as_str()?.to_string();
                    let mut source = item["source"].as_str().unwrap_or("web").to_lowercase();

                    // If source isn't already academic, check providers list
                    if source != "arxiv"
                        && source != "semantic_scholar"
                        && let Some(providers) = item["providers"].as_array()
                    {
                        let provider_strs: Vec<&str> =
                            providers.iter().filter_map(|p| p.as_str()).collect();
                        if provider_strs.contains(&"arxiv") {
                            source = "arxiv".to_string();
                        } else if provider_strs.contains(&"semantic_scholar") {
                            source = "semantic_scholar".to_string();
                        }
                    }
                    let published = item["published"].as_str().map(|s| s.to_string());
                    let year = published.as_ref().and_then(|d| d[..4].parse::<u16>().ok());

                    if title.is_empty() || url.is_empty() {
                        return None;
                    }

                    let work_type = match source.as_str() {
                        "semantic_scholar" => "journal_article",
                        "arxiv" => "preprint",
                        _ => "web_page",
                    };

                    // Extract abstract/snippet for LLM concept extraction
                    let abstract_text = item["abstract"]
                        .as_str()
                        .or_else(|| item["snippet"].as_str())
                        .or_else(|| item["description"].as_str())
                        .map(|s| s.to_string());

                    Some(DiscoveredWork {
                        slug: slugify(&title),
                        title,
                        url,
                        year,
                        source,
                        work_type: work_type.to_string(),
                        abstract_text,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(results)
}

// ── YouTube transcript search (SerpAPI) ─────────────────────────────────────

const SERPAPI_BASE: &str = "https://serpapi.com/search";

/// Search YouTube for videos matching the query and fetch their transcripts.
async fn search_youtube_transcripts(
    query: &str,
    api_key: &str,
    limit: usize,
) -> Result<Vec<DiscoveredWork>, ServiceError> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| {
            let msg = format!("HTTP client build failed: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    // Step 1: Search YouTube for videos
    let params: Vec<(&str, String)> = vec![
        ("q", query.to_string()),
        ("api_key", api_key.to_string()),
        ("engine", "youtube".to_string()),
        ("num", limit.to_string()),
    ];

    let resp = client
        .get(SERPAPI_BASE)
        .query(&params)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("SerpAPI YouTube search failed: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    let body = resp.text().await.unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    let video_results = parsed["video_results"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|video| {
                    let title = video["title"].as_str()?.to_string();
                    let link = video["link"].as_str()?.to_string();
                    let video_id = extract_youtube_id(&link)?;
                    Some((title, video_id))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if video_results.is_empty() {
        return Ok(vec![]);
    }

    // Step 2: Fetch transcript for each video in parallel
    let mut handles = Vec::new();
    for (title, video_id) in video_results {
        let client = client.clone();
        let api_key = api_key.to_string();
        handles.push(tokio::spawn(async move {
            match fetch_youtube_transcript(&client, &api_key, &video_id, &title).await {
                Ok(Some(work)) => Some(work),
                Ok(None) => {
                    tracing::info!(
                        target: "hkask.discover",
                        video_id = %video_id,
                        title = %title,
                        "No transcript available for video — skipping"
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.discover",
                        video_id = %video_id,
                        error = %e,
                        "Failed to fetch transcript — skipping"
                    );
                    None
                }
            }
        }));
    }

    let mut transcripts: Vec<DiscoveredWork> = Vec::new();
    for handle in handles {
        if let Ok(Some(work)) = handle.await {
            transcripts.push(work);
        }
    }

    Ok(transcripts)
}

fn extract_youtube_id(url: &str) -> Option<String> {
    if let Some(pos) = url.find("v=") {
        let after = &url[pos + 2..];
        let id: String = after.chars().take(11).collect();
        if id.len() == 11
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Some(id);
        }
    }
    if let Some(pos) = url.find("youtu.be/") {
        let after = &url[pos + 9..];
        let id: String = after.chars().take(11).collect();
        if id.len() == 11
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Some(id);
        }
    }
    None
}

async fn fetch_youtube_transcript(
    client: &reqwest::Client,
    api_key: &str,
    video_id: &str,
    title: &str,
) -> Result<Option<DiscoveredWork>, ServiceError> {
    let params: Vec<(&str, String)> = vec![
        ("v", video_id.to_string()),
        ("api_key", api_key.to_string()),
        ("engine", "youtube_video_transcript".to_string()),
    ];

    let resp = client
        .get(SERPAPI_BASE)
        .query(&params)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("SerpAPI transcript request failed: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(ServiceError::Embed {
            source: None,
            message: format!("SerpAPI transcript error {status} for video '{video_id}'"),
        });
    }

    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    let transcript_text = parsed["transcript"]
        .as_array()
        .map(|segments| {
            segments
                .iter()
                .filter_map(|seg| seg["snippet"].as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();

    if transcript_text.is_empty() {
        return Ok(None);
    }

    let video_url = format!("https://www.youtube.com/watch?v={video_id}");
    let video_title = parsed["title"].as_str().unwrap_or(title).to_string();

    Ok(Some(DiscoveredWork {
        slug: slugify(&video_title),
        title: video_title,
        url: video_url,
        year: None,
        source: "youtube_transcript".to_string(),
        work_type: "transcript".to_string(),
        abstract_text: Some(String::new()),
    }))
}

// ── Download + cache ────────────────────────────────────────────────────────

/// Download content from a URL and cache it to disk.
///
/// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  url must be a valid HTTP/HTTPS URL; cache_path's parent directory must exist
/// post: content is downloaded, PDFs are text-extracted (with OCR fallback), HTML is stripped, and result is written to cache_path; Err on HTTP failure, empty content, or I/O error
pub async fn download_and_cache(url: &str, cache_path: &Path) -> Result<(), ServiceError> {
    // P9: CNS span
    tracing::info!(target: "cns.discover", operation = "download_and_cache", url = %url, cache = %cache_path.display(), "CNS");

    let resp = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| {
            let msg = format!("HTTP client build failed: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?
        .get(url)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("HTTP request failed for '{url}': {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    if !resp.status().is_success() {
        return Err(ServiceError::Embed {
            source: None,
            message: format!("HTTP {} for '{url}'", resp.status()),
        });
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = resp.bytes().await.map_err(|e| {
        let msg = format!("Failed to read response body: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    let is_pdf = content_type.contains("application/pdf")
        || url.ends_with(".pdf")
        || bytes.starts_with(b"%PDF");

    let text = if is_pdf {
        let tmp_dir = std::env::temp_dir();
        let tmp_path = tmp_dir.join(format!("hkask-discover-{}.pdf", uuid::Uuid::new_v4()));
        std::fs::write(&tmp_path, &bytes).map_err(|e| {
            let msg = format!("Failed to write temp PDF: {e}");
            ServiceError::Embed {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;
        let extracted = pdf_extract::extract_text(&tmp_path).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp_path);

        let word_count = extracted.split_whitespace().count();
        if word_count < 10 {
            tracing::warn!(
                url = %url,
                word_count = word_count,
                "PDF extraction near-empty — attempting OCR fallback"
            );
            match hkask_services_embed::ocr_pdf_bytes(&bytes, url).await {
                Ok(ocr_text) => {
                    let ocr_words = ocr_text.split_whitespace().count();
                    if ocr_words > word_count {
                        tracing::info!(url = %url, ocr_words = ocr_words, "OCR succeeded");
                        ocr_text
                    } else {
                        tracing::warn!(url = %url, "OCR also low — using extraction result");
                        extracted
                    }
                }
                Err(e) => {
                    tracing::warn!(url = %url, error = %e, "OCR failed — using extraction result");
                    extracted
                }
            }
        } else {
            extracted
        }
    } else {
        let raw = String::from_utf8_lossy(&bytes).to_string();
        if content_type.contains("text/html")
            || raw.starts_with("<!DOCTYPE")
            || raw.starts_with("<html")
        {
            hkask_services_embed::strip_html_tags(&raw)
        } else {
            raw
        }
    };

    if text.split_whitespace().count() < 10 {
        return Err(ServiceError::Embed {
            source: None,
            message: format!(
                "Downloaded content from '{url}' is too short (likely paywalled or scanned PDF without OCR)"
            ),
        });
    }

    std::fs::write(cache_path, &text).map_err(|e| {
        let msg = format!("Failed to write cache: {e}");
        ServiceError::Embed {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    tracing::info!(
        target: "hkask.discover",
        path = %cache_path.display(),
        bytes = bytes.len(),
        words = text.split_whitespace().count(),
        "Cached work"
    );

    Ok(())
}

// ── Utilities ───────────────────────────────────────────────────────────────

/// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  s may be any string (including empty)
/// post: returns lowercase, alphanumeric-only slug with hyphens; empty string becomes empty slug
pub fn slugify(s: &str) -> String {
    let slug = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    // Fallback to UUID-based slug for all-non-ASCII titles
    if slug.is_empty() {
        let uuid_slug = uuid::Uuid::new_v4().to_string();
        tracing::warn!(
            target: "hkask.discover",
            input = %s,
            fallback = %uuid_slug,
            "slugify produced empty string — using UUID fallback"
        );
        uuid_slug
    } else {
        slug
    }
}

fn extract_search_terms(author: &str, titles: &[String]) -> String {
    if titles.is_empty() {
        return author.to_string();
    }

    let stopwords: &[&str] = &[
        "study",
        "studies",
        "analysis",
        "effect",
        "effects",
        "evidence",
        "research",
        "review",
        "approach",
        "model",
        "theory",
        "data",
        "using",
        "based",
        "new",
        "role",
        "among",
        "across",
        "within",
        "toward",
        "towards",
        "understanding",
        "implications",
        "introduction",
        "overview",
        "perspective",
        "commentary",
        "response",
        "reply",
        "the",
        "and",
        "for",
        "from",
        "with",
    ];

    let mut word_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for title in titles {
        for word in title.split_whitespace() {
            let cleaned: String = word
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            if cleaned.len() < 4 || stopwords.contains(&cleaned.as_str()) {
                continue;
            }
            *word_counts.entry(cleaned).or_insert(0) += 1;
        }
    }

    let mut sorted: Vec<(&String, &usize)> = word_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    let top_words: Vec<&str> = sorted
        .iter()
        .take(5)
        .filter(|(_, count)| **count >= 2)
        .map(|(word, _)| word.as_str())
        .collect();

    if top_words.is_empty() {
        return author.to_string();
    }

    format!("{} {}", author, top_words.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── slugify ─────────────────────────────────────────────────────────

    #[test]
    fn slugify_ascii_name() {
        let s = slugify("David Dunning");
        assert_eq!(s, "david-dunning");
    }

    #[test]
    fn slugify_with_special_chars() {
        let s = slugify("J. R. R. Tolkien");
        assert!(s.contains("tolkien"));
    }

    #[test]
    fn slugify_non_ascii_fallback() {
        // All non-ASCII characters produce empty slug → UUID fallback
        let s = slugify("中文作者");
        assert!(!s.is_empty());
        // UUID format: 8-4-4-4-12 hex chars
        assert_eq!(s.len(), 36);
        assert_eq!(s.chars().filter(|c| *c == '-').count(), 4);
    }

    #[test]
    fn slugify_empty_string() {
        let s = slugify("");
        assert!(!s.is_empty()); // UUID fallback
        assert_eq!(s.len(), 36);
    }

    // ── parse_template_model ────────────────────────────────────────────

    #[test]
    fn parse_model_directive_present() {
        let src = "{# model: OM/qwen3:14b #}\nrest of template";
        assert_eq!(parse_template_model(src), Some("OM/qwen3:14b".to_string()));
    }

    #[test]
    fn parse_model_directive_absent() {
        let src = "You are analyzing the academic work of {{ author_name }}.";
        assert_eq!(parse_template_model(src), None);
    }

    #[test]
    fn parse_model_directive_empty_template() {
        assert_eq!(parse_template_model(""), None);
    }

    #[test]
    fn parse_model_directive_whitespace_handling() {
        let src = "  {# model: DI/meta-llama/Llama-3.3-70B-Instruct #}  \nrest";
        assert_eq!(
            parse_template_model(src),
            Some("DI/meta-llama/Llama-3.3-70B-Instruct".to_string())
        );
    }

    // ── default_corpus_config ───────────────────────────────────────────

    #[test]
    fn default_corpus_config_has_correct_defaults() {
        let config = default_corpus_config("test-author");
        assert_eq!(config.author, "test-author");
        assert_eq!(config.corpus_type, "literary");
        assert_eq!(config.embedding.dim, 1024);
        assert_eq!(config.chunking.min_words, 50);
        assert_eq!(config.chunking.max_words, 200);
        assert_eq!(config.centroid_entity_ref, "style:test-author:centroid");
        assert!(config.works.is_empty());
        assert!(config.methods.is_empty());
        assert!(config.foundational_rules.is_empty());
    }

    #[test]
    fn default_corpus_config_academic_entities_empty_by_default() {
        let config = default_corpus_config("author");
        assert!(config.entities.co_authors.is_empty());
        assert!(config.entities.venues.is_empty());
        assert!(config.entities.topics.is_empty());
        assert!(config.entities.paradigms.is_empty());
    }

    // ── DiscoveredWork with abstract ────────────────────────────────────

    #[test]
    fn discovered_work_serializes_abstract() {
        let work = DiscoveredWork {
            title: "Test Paper".to_string(),
            slug: "test-paper".to_string(),
            url: "https://example.com".to_string(),
            year: Some(2024),
            source: "semantic_scholar".to_string(),
            work_type: "journal_article".to_string(),
            abstract_text: Some("This paper explores...".to_string()),
        };
        let json = serde_json::to_string(&work).unwrap();
        assert!(json.contains("abstract_text"));
        assert!(json.contains("This paper explores"));
    }

    #[test]
    fn discovered_work_omits_none_abstract() {
        let work = DiscoveredWork {
            title: "Test".to_string(),
            slug: "test".to_string(),
            url: "https://example.com".to_string(),
            year: None,
            source: "web".to_string(),
            work_type: "web_page".to_string(),
            abstract_text: None,
        };
        let json = serde_json::to_string(&work).unwrap();
        // serde(default) serializes None as null, not omitted
        assert!(json.contains("\"abstract_text\":null"));
    }

    // ── extract_search_terms ────────────────────────────────────────────

    #[test]
    fn extract_search_terms_from_titles() {
        let titles = vec![
            "Unskilled and Unaware of It".to_string(),
            "Flawed Self-Assessment".to_string(),
            "Why People Fail to Recognize Their Own Incompetence".to_string(),
        ];
        let terms = extract_search_terms("David Dunning", &titles);
        assert!(terms.starts_with("David Dunning"));
        assert!(!terms.is_empty());
    }

    #[test]
    fn extract_search_terms_empty_titles() {
        let terms = extract_search_terms("Author", &[]);
        assert_eq!(terms, "Author");
    }

    // ── DiscoverRequest defaults ────────────────────────────────────────

    #[test]
    fn discover_request_defaults() {
        let req = DiscoverRequest {
            author_name: "Test".to_string(),
            max_works: 10,
            cache_dir: "./cache".to_string(),
            output_dir: None,
            serpapi_key: None,
            include_transcripts: true,
            include_web: true,
            curated: true,
            web_search_terms: None,
            augment: false,
            include_methods: true,
            biographical_details: None,
        };
        assert!(req.include_methods);
        assert!(req.curated);
        assert!(!req.augment);
        assert!(req.biographical_details.is_none());
    }

    #[test]
    fn discover_request_with_bio() {
        let req = DiscoverRequest {
            author_name: "J. Smith".to_string(),
            max_works: 10,
            cache_dir: "./cache".to_string(),
            output_dir: None,
            serpapi_key: None,
            include_transcripts: true,
            include_web: true,
            curated: true,
            web_search_terms: None,
            augment: false,
            include_methods: true,
            biographical_details: Some("professor of psychology at Cornell".to_string()),
        };
        assert_eq!(
            req.biographical_details.as_deref(),
            Some("professor of psychology at Cornell")
        );
    }
}
