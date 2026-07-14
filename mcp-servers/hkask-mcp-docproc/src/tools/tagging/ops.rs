//! Ontology tagging tools — multi-dimensional chunk annotation.
//!
//! `docproc_tag_chunks`: Tags each chunk with 5W1H interrogatory dimensions,
//! Dublin Core metadata, PKO process concepts, FIBO/GOLEM domain concepts,
//! and expertise level. Uses LLM-based extraction via a Jinja2 template.
//! Every chunk gets at least one 5W1H dimension — no zero-tag chunks.

use crate::tools::semantic::{GUARD, configured_qa_model};
use crate::*;
use hkask_storage::HMem;
use hkask_types::Visibility;
use hkask_types::corpus::TaggedChunk;
use serde::Serialize;

/// Minimal chunk for tagging (from chunks.jsonl).
#[derive(Debug, Clone, Deserialize)]
struct InputChunk {
    entity_ref: String,
    source: String,
    text: String,
    #[serde(default)]
    #[allow(dead_code)]
    word_count: usize,
}

/// Ontology tags extracted by the LLM from a passage.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct OntologyTags {
    /// 5W1H interrogatory dimensions (at least one required).
    #[serde(default)]
    dimensions: Vec<String>,
    /// Dublin Core BIBO type (e.g., "bibo:Book").
    #[serde(default)]
    dc_type: String,
    /// Dublin Core subject keywords.
    #[serde(default)]
    dc_subject: Vec<String>,
    /// Flexible ontology tags keyed by namespace (e.g., "fibo", "golem", "omc", "pko", "other").
    #[serde(default)]
    ontology_tags: std::collections::HashMap<String, Vec<String>>,
    /// Expertise level: "practitioner", "analyst", or "researcher".
    #[serde(default)]
    expertise_level: String,
}

fn read_input_chunks(path: &str) -> Result<Vec<InputChunk>, McpToolError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        McpToolError::invalid_argument(format!("Cannot read chunks_jsonl '{path}': {e}"))
    })?;
    let total_lines = content.lines().filter(|l| !l.trim().is_empty()).count();
    let chunks: Vec<InputChunk> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let dropped = total_lines - chunks.len();
    if dropped > 0 {
        eprintln!("  Warning: dropped {dropped} malformed lines from input");
    }
    Ok(chunks)
}

/// Compute graph-centrality salience across the multi-ontology concept graph.
/// salience = connectivity × (0.5 + 0.5 × diversity)
/// where connectivity = neighbor_count / (n-1) and diversity = concept_count / 10.
/// Dimensions (5W1H) contribute to connectivity but not diversity.
fn compute_salience(tagged: &[TaggedChunk]) -> Vec<f32> {
    let n = tagged.len();
    if n == 0 {
        return Vec::new();
    }

    // Build inverted index: concept → set of chunk indices
    let mut concept_to_chunks: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for (i, chunk) in tagged.iter().enumerate() {
        for concept in &chunk.concepts {
            concept_to_chunks
                .entry(concept.to_string())
                .or_default()
                .push(i);
        }
        // Also index by 5W1H dimensions
        for dim in &chunk.dimensions {
            let dim_key = format!("5w1h:{dim}");
            concept_to_chunks
                .entry(dim_key.clone())
                .or_default()
                .push(i);
        }
    }

    // For each chunk, count neighbors (other chunks sharing at least one concept/dimension)
    tagged
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let mut neighbors: std::collections::HashSet<usize> = std::collections::HashSet::new();
            for concept in &chunk.concepts {
                if let Some(indices) = concept_to_chunks.get(concept.as_str()) {
                    for &j in indices {
                        if j != i {
                            neighbors.insert(j);
                        }
                    }
                }
            }
            for dim in &chunk.dimensions {
                let dim_key = format!("5w1h:{}", dim);
                if let Some(indices) = concept_to_chunks.get(&dim_key) {
                    for &j in indices {
                        if j != i {
                            neighbors.insert(j);
                        }
                    }
                }
            }
            // salience = neighbor_count / max_possible_neighbors, scaled by concept diversity
            let connectivity = neighbors.len() as f32 / (n - 1).max(1) as f32;
            let diversity = (chunk.concepts.len() + chunk.dimensions.len()) as f32 / 15.0; // concepts + dimensions
            (connectivity * (0.5 + 0.5 * diversity.min(1.0))).clamp(0.0, 1.0)
        })
        .collect()
}

#[tool_router(router = tagging_router, vis = "pub")]
impl DocProcServer {
    #[tool(
        description = "Tag chunks with multi-dimensional ontology annotations: 5W1H interrogatory dimensions, Dublin Core metadata, PKO process concepts, FIBO/GOLEM domain concepts, and expertise level. Uses LLM-based extraction via Jinja2 template. Computes graph-centrality salience. Every chunk gets at least one 5W1H dimension — no zero-salience chunks."
    )]
    pub async fn docproc_tag_chunks(
        &self,
        Parameters(req): Parameters<TagChunksRequest>,
    ) -> String {
        execute_tool(self, "docproc_tag_chunks", async {
            let chunks = read_input_chunks(&req.chunks_jsonl)?;
            if chunks.is_empty() {
                return Err(McpToolError::invalid_argument("chunks_jsonl is empty"));
            }
            let total = chunks.len();
            println!("  Tagging {} chunks with ontology dimensions...", total);

            if req.dry_run {
                return Ok(json!({
                    "total_chunks": total,
                    "dry_run": true,
                    "note": "Would tag each chunk with 5W1H + Dublin Core + PKO + FIBO + GOLEM + expertise level"
                }));
            }

            let sem = Arc::new(tokio::sync::Semaphore::new(req.concurrency.max(1)));
            let router = Arc::clone(&self.inference_router);
            let model_override = configured_qa_model(None);

            // Results: index → OntologyTags
            let results: Arc<std::sync::Mutex<Vec<Option<OntologyTags>>>> =
                Arc::new(std::sync::Mutex::new(
                    (0..total).map(|_| None).collect(),
                ));

            let start_time = std::time::Instant::now();
            let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let failed = Arc::new(std::sync::atomic::AtomicUsize::new(0));

            let mut handles = Vec::with_capacity(total);

            for (i, chunk) in chunks.iter().enumerate() {
                let router = Arc::clone(&router);
                let sem = Arc::clone(&sem);
                let results = Arc::clone(&results);
                let completed = Arc::clone(&completed);
                let failed = Arc::clone(&failed);
                let model_override = model_override.clone();
                let text = chunk.text.clone();
                let source = chunk.source.clone();
                let chunk_id = chunk.entity_ref.clone();
                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await;

                    // Render tagging prompt from Jinja2 template
                    let mut vars = std::collections::HashMap::new();
                    vars.insert("text", text.clone());
                    vars.insert("source", source.clone());
                    vars.insert("chunk_id", chunk_id.clone());
                    let prompt = render_docproc_template("tag-chunks", &vars);
                    let prompt = if prompt.is_empty() {
                        // Fallback if template not found
                        format!(
                            "Analyze this passage and extract ontology tags.\n\nPassage (chunk {chunk_id}, source: {source}):\n{text}\n\nRespond with JSON: {{\"dimensions\": [\"what\"], \"dc_type\": \"bibo:Document\", \"dc_subject\": [], \"pko_concepts\": [], \"fibo_concepts\": [], \"golem_concepts\": [], \"other_concepts\": [], \"expertise_level\": \"analyst\"}}"
                        )
                    } else {
                        prompt
                    };

                    // ContentGuard
                    let input_scan = GUARD.scan_input(&prompt);
                    if !input_scan.passed {
                        failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        return;
                    }

                    let params = LLMParameters {
                        temperature: 0.1,
                        top_p: 0.85,
                        max_tokens: 4096,
                        ..Default::default()
                    };

                    // H2: Retry with exponential backoff (3 attempts)
                    let mut _last_error = String::new();
                    let response: Option<_> = {
                        let mut attempts = 0u32;
                        loop {
                            match router
                                .generate_with_model(&prompt, &params, model_override.as_deref(), None)
                                .await
                            {
                                Ok(resp) => break Some(resp),
                                Err(e) => {
                                    attempts += 1;
                                    _last_error = format!("{}", e);
                                    if attempts >= 3 {
                                        break None;
                                    }
                                    let backoff = std::time::Duration::from_secs(
                                        2u64.pow(attempts) * 5
                                    );
                                    tokio::time::sleep(backoff).await;
                                }
                            }
                        }
                    };

                    let parse_result = if let Some(response) = response {
                        // Got a response — parse it
                        let output_scan = GUARD.scan_output(&response.text);
                        let content = output_scan.output.content(&response.text);
                        let cleaned = strip_json_fences(content);
                        serde_json::from_str::<OntologyTags>(&cleaned)
                            .map(|mut tags| {
                                // Validate dimensions against 5W1H allowlist
                                let valid_dims: Vec<String> = tags.dimensions.iter()
                                    .filter(|d| matches!(d.as_str(), "who"|"what"|"when"|"where"|"why"|"how"))
                                    .cloned()
                                    .collect();
                                tags.dimensions = if valid_dims.is_empty() {
                                    vec!["what".to_string()]
                                } else {
                                    valid_dims
                                };
                                // Validate expertise_level
                                if !matches!(tags.expertise_level.as_str(), "practitioner"|"analyst"|"researcher") {
                                    tags.expertise_level = "analyst".to_string();
                                }
                                tags
                            })
                            .ok()
                    } else {
                        None
                    };

                    if let Some(tags) = parse_result {
                        let mut results = results.lock().unwrap();
                        results[i] = Some(tags);
                        completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    } else {
                        // Fallback: minimal tags after retries exhausted
                        let fallback = OntologyTags {
                            dimensions: vec!["what".to_string()],
                            dc_type: "bibo:Document".to_string(),
                            dc_subject: Vec::new(),
                            ontology_tags: std::collections::HashMap::new(),
                            expertise_level: "analyst".to_string(),
                        };
                        let mut results = results.lock().unwrap();
                        results[i] = Some(fallback);
                        failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }


                });
                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.await;
            }

            let c = completed.load(std::sync::atomic::Ordering::Relaxed);
            let f = failed.load(std::sync::atomic::Ordering::Relaxed);
            let elapsed = start_time.elapsed().as_secs_f64();
            println!("  Tagged: {} ok, {} failed, {:.1}s", c, f, elapsed);

            // Build tagged chunk outputs with salience
            let tags_guard = results.lock().unwrap();
            let mut tagged: Vec<TaggedChunk> = chunks
                .iter()
                .enumerate()
                .map(|(i, chunk)| {
                    let tags = tags_guard[i].clone().unwrap_or_else(|| OntologyTags {
                        dimensions: vec!["what".to_string()],
                        dc_type: "bibo:Document".to_string(),
                        dc_subject: Vec::new(),
                        ontology_tags: std::collections::HashMap::new(),
                        expertise_level: "analyst".to_string(),
                    });

                    // Union all ontology_tags values (HashSet for true dedup)
                    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                    let mut concepts: Vec<String> = Vec::new();
                    for concept_list in tags.ontology_tags.values() {
                        for c in concept_list {
                            if seen.insert(c.clone()) {
                                concepts.push(c.clone());
                            }
                        }
                    }

                    TaggedChunk {
                        entity_ref: chunk.entity_ref.clone(),
                        source: chunk.source.clone(),
                        text: chunk.text.clone(),
                        word_count: chunk.word_count,
                        dimensions: tags.dimensions,
                        dc_type: tags.dc_type,
                        dc_subject: tags.dc_subject,
                        ontology_tags: tags.ontology_tags,
                        concepts,
                        expertise_level: tags.expertise_level,
                        salience: 0.0,
                        consolidated_from: Vec::new(),
                        ontology: None,
                    }
                })
                .collect();
            drop(tags_guard);

            // Compute salience
            let salience_scores = compute_salience(&tagged);
            for (i, s) in salience_scores.iter().enumerate() {
                tagged[i].salience = *s;
            }

            // Write output JSONL
            let mut out = String::new();
            for chunk in &tagged {
                out.push_str(&serde_json::to_string(chunk)
                    .map_err(|e| McpToolError::internal(format!("Serialize: {e}")))?);
                out.push('\n');
            }
            std::fs::write(&req.output, &out).map_err(|e| {
                McpToolError::internal(format!("Cannot write output '{}': {}", req.output, e))
            })?;

            // Store tags as h_mems in DB
            let dim = embedding_dim();
            let semantic = SemanticMemory::open(&req.db_path, &req.passphrase, dim)
                .map_err(|e| McpToolError::failed_precondition(format!("Cannot open DB: {e}")))?;
            let webid = hkask_types::WebID::from_persona("corpus".as_bytes());
            let mut stored = 0usize;
            let mut store_failures = 0usize;
            for (i, chunk) in tagged.iter().enumerate() {
                let entity = &chunk.entity_ref;
                let v = serde_json::json!({
                    "dimensions": chunk.dimensions,
                    "dc_type": chunk.dc_type,
                    "dc_subject": chunk.dc_subject,
                    "ontology_tags": chunk.ontology_tags,
                    "concepts": chunk.concepts,
                    "expertise_level": chunk.expertise_level,
                    "salience": chunk.salience,
                    "source": chunk.source,
                });
                let h_mem = HMem::new(entity, "ontology_tags", v, webid)
                    .with_visibility(Visibility::Public)
                    .with_confidence(0.9);
                match semantic.store(h_mem) {
                    Ok(()) => stored += 1,
                    Err(e) => {
                        store_failures += 1;
                        if store_failures <= 5 {
                            eprintln!("  WARN: store tag h_mem for {entity}: {e}");
                        }
                    }
                }
                let _ = i;
            }

            // Stats
            let dim_counts: std::collections::HashMap<&str, usize> = {
                let mut m = std::collections::HashMap::new();
                for chunk in &tagged {
                    for dim in &chunk.dimensions {
                        *m.entry(dim.as_str()).or_default() += 1;
                    }
                }
                m
            };
            let exp_counts: std::collections::HashMap<&str, usize> = {
                let mut m = std::collections::HashMap::new();
                for chunk in &tagged {
                    *m.entry(chunk.expertise_level.as_str()).or_default() += 1;
                }
                m
            };

            let result = json!({
                "total_chunks": total,
                "tagged": c,
                "failed": f,
                "stored_h_mems": stored,
                "store_failures": store_failures,
                "dimensions": dim_counts,
                "expertise_levels": exp_counts,
                "time_seconds": elapsed,
            });

            let outcome = if f > total / 2 { "degraded" } else { "success" };
            self.record_experience(
                "docproc_tag_chunks",
                &format!("{} chunks", total),
                outcome,
                result.clone(),
            );
            Ok(result)
        })
        .await
    }
}

fn embedding_dim() -> usize {
    std::env::var("HKASK_EMBEDDING_DIM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1024)
}
