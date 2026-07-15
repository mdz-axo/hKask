//! Corpus pipeline tools — semantic chunk dedup and consolidation.
//!
//! These tools operate on tagged chunks JSONL (from the salience phase) and
//! the SQLCipher memory DB (containing chunk embeddings). They are the
//! "Phase 2c" and "Phase 2d" quality gates in the corpus pipeline.
//!
//! - `docproc_dedup_chunks`: Removes near-identical chunks (cosine > 0.85)
//!   using stored embeddings. Keeps highest-salience survivor per cluster.
//! - `docproc_consolidate_chunks`: Clusters semantically related chunks
//!   (cosine > 0.75), uses the inference router to LLM-synthesize each
//!   multi-chunk cluster into a single comprehensive passage, re-embeds
//!   the consolidated text, and stores the new embedding in the DB.

use crate::tools::semantic::{GUARD, configured_qa_model};
use crate::*;
use schemars::JsonSchema;
use serde::Deserialize;

use hkask_types::corpus::{ChunkOntology, TaggedChunk};

fn read_tagged_chunks(path: &str) -> Result<Vec<TaggedChunk>, McpToolError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        McpToolError::invalid_argument(format!("Cannot read tagged_jsonl '{path}': {e}"))
    })?;
    let chunks: Vec<TaggedChunk> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(chunks)
}

/// Greedy clustering within a source file.
/// Returns clusters as Vecs of indices into `chunks` (sorted by salience desc).
fn cluster_within_source(
    chunk_indices: &[usize],
    chunks: &[TaggedChunk],
    norm_map: &std::collections::HashMap<&str, &[f32]>,
    threshold: f32,
    max_per_cluster: usize,
) -> Vec<Vec<usize>> {
    // Gather (index, embedding) pairs, sorted by salience descending
    let mut indexed_embs: Vec<(usize, &[f32])> = chunk_indices
        .iter()
        .filter_map(|&idx| {
            norm_map
                .get(chunks[idx].entity_ref.as_str())
                .copied()
                .map(|emb| (idx, emb))
        })
        .collect();
    indexed_embs.sort_by(|a, b| {
        chunks[b.0]
            .salience
            .partial_cmp(&chunks[a.0].salience)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut cluster_reps: Vec<&[f32]> = Vec::new();
    let mut cluster_members: Vec<Vec<usize>> = Vec::new();

    for (idx, emb) in &indexed_embs {
        let mut found = None;
        for (ci, rep_emb) in cluster_reps.iter().enumerate() {
            let dot: f32 = emb.iter().zip(rep_emb.iter()).map(|(a, b)| a * b).sum();
            if dot > threshold {
                found = Some(ci);
                break;
            }
        }
        match found {
            Some(ci) if cluster_members[ci].len() < max_per_cluster => {
                cluster_members[ci].push(*idx);
            }
            _ => {
                cluster_reps.push(emb);
                cluster_members.push(vec![*idx]);
            }
        }
    }

    // Append chunks without embeddings as singletons
    for &idx in chunk_indices {
        if !norm_map.contains_key(chunks[idx].entity_ref.as_str()) {
            cluster_members.push(vec![idx]);
        }
    }

    cluster_members
}

#[tool_router(router = corpus_router, vis = "pub")]
impl DocProcServer {
    #[tool(
        description = "Deduplicate chunks by semantic embedding similarity. Queries chunk embeddings from the memory DB, clusters within each source file by cosine similarity > threshold (default 0.85), and keeps the highest-salience chunk per cluster. Writes deduplicated tagged chunks JSONL."
    )]
    pub async fn docproc_dedup_chunks(
        &self,
        Parameters(req): Parameters<DedupChunksRequest>,
    ) -> String {
        execute_tool(self, "docproc_dedup_chunks", async {
            let chunks = read_tagged_chunks(&req.tagged_jsonl)?;
            if chunks.is_empty() {
                return Err(McpToolError::invalid_argument("tagged_jsonl is empty"));
            }

            let dim = embedding_dim();
            let semantic = SemanticMemory::open(&req.db_path, &req.passphrase, dim).map_err(|e| {
                McpToolError::failed_precondition(format!("Cannot open memory DB: {e}"))
            })?;
            let embeddings = semantic
                .embeddings_by_prefix(&req.prefix)
                .map_err(|e| McpToolError::internal(format!("Embedding query failed: {e}")))?;

            // Pre-normalize all vectors
            let normalized: Vec<(String, Vec<f32>)> = embeddings
                .into_iter()
                .map(|(er, mut v)| {
                    normalize_in_place(&mut v);
                    (er, v)
                })
                .collect();
            let norm_map: std::collections::HashMap<&str, &[f32]> = normalized
                .iter()
                .map(|(e, v)| (e.as_str(), v.as_slice()))
                .collect();

            // Group by source
            let mut by_source: std::collections::HashMap<&str, Vec<usize>> =
                std::collections::HashMap::new();
            for (i, c) in chunks.iter().enumerate() {
                by_source.entry(c.source.as_str()).or_default().push(i);
            }

            let threshold = req.threshold as f32;
            let mut keep_indices: Vec<usize> = Vec::new();
            let mut clusters_total = 0usize;

            for indices in by_source.values() {
                let clusters = cluster_within_source(
                    indices,
                    &chunks,
                    &norm_map,
                    threshold,
                    usize::MAX, // no cap for dedup
                );
                clusters_total += clusters.len();
                for cluster in &clusters {
                    // Keep the first (highest-salience) member
                    keep_indices.push(cluster[0]);
                }
            }

            keep_indices.sort_unstable();
            keep_indices.dedup();

            let result = json!({
                "original": chunks.len(),
                "deduped": keep_indices.len(),
                "removed": chunks.len() - keep_indices.len(),
                "clusters": clusters_total,
                "sources": by_source.len(),
                "reduction_pct": (1.0 - keep_indices.len() as f64 / chunks.len().max(1) as f64) * 100.0,
            });

            if req.dry_run {
                return Ok(result);
            }

            let mut out = String::new();
            for &idx in &keep_indices {
                out.push_str(&serde_json::to_string(&chunks[idx])
                    .map_err(|e| McpToolError::internal(format!("Serialize: {e}")))?);
                out.push('\n');
            }
            std::fs::write(&req.output, &out).map_err(|e| {
                McpToolError::internal(format!("Cannot write output '{}': {e}", req.output))
            })?;

            self.record_experience(
                "docproc_dedup_chunks",
                &format!("{} → {}", chunks.len(), keep_indices.len()),
                "success",
                result.clone(),
            );
            Ok(result)
        })
        .await
    }

    #[tool(
        description = "Consolidate semantically related chunks via LLM synthesis. Clusters chunks within each source file by cosine similarity > threshold (default 0.75), then uses the inference router to synthesize each multi-chunk cluster into a single comprehensive passage. Re-embeds consolidated text and stores the new embedding. Writes consolidated tagged chunks JSONL with provenance."
    )]
    pub async fn docproc_consolidate_chunks(
        &self,
        Parameters(req): Parameters<ConsolidateChunksRequest>,
    ) -> String {
        execute_tool(self, "docproc_consolidate_chunks", async {
            let chunks = read_tagged_chunks(&req.tagged_jsonl)?;
            if chunks.is_empty() {
                return Err(McpToolError::invalid_argument("tagged_jsonl is empty"));
            }

            let dim = embedding_dim();
            let semantic = SemanticMemory::open(&req.db_path, &req.passphrase, dim).map_err(|e| {
                McpToolError::failed_precondition(format!("Cannot open memory DB: {e}"))
            })?;
            let embeddings = semantic
                .embeddings_by_prefix(&req.prefix)
                .map_err(|e| McpToolError::internal(format!("Embedding query failed: {e}")))?;

            // Pre-normalize all vectors
            let normalized: Vec<(String, Vec<f32>)> = embeddings
                .into_iter()
                .map(|(er, mut v)| {
                    normalize_in_place(&mut v);
                    (er, v)
                })
                .collect();
            let norm_map: std::collections::HashMap<&str, &[f32]> = normalized
                .iter()
                .map(|(e, v)| (e.as_str(), v.as_slice()))
                .collect();

            // Group by source
            let mut by_source: std::collections::HashMap<&str, Vec<usize>> =
                std::collections::HashMap::new();
            for (i, c) in chunks.iter().enumerate() {
                by_source.entry(c.source.as_str()).or_default().push(i);
            }

            let threshold = req.threshold as f32;

            // Phase 1: Cluster
            let mut all_clusters: Vec<Vec<usize>> = Vec::new();
            let mut singletons = 0usize;
            let mut multi = 0usize;

            for indices in by_source.values() {
                let clusters = cluster_within_source(
                    indices,
                    &chunks,
                    &norm_map,
                    threshold,
                    req.max_chunks_per_cluster,
                );
                for c in clusters {
                    if c.len() > 1 {
                        multi += 1;
                    } else {
                        singletons += 1;
                    }
                    all_clusters.push(c);
                }
            }

            let total_members: usize = all_clusters.iter().map(|c| c.len()).sum();
            let absorbed = total_members - all_clusters.len();

            let stats = json!({
                "original": chunks.len(),
                "clusters": all_clusters.len(),
                "singletons": singletons,
                "multi_chunk": multi,
                "absorbed": absorbed,
                "reduction_pct": (absorbed as f64 / chunks.len().max(1) as f64) * 100.0,
            });

            if req.dry_run {
                return Ok(stats);
            }

            // Phase 2: LLM consolidation of multi-chunk clusters
            let multi_indices: Vec<usize> = all_clusters
                .iter()
                .enumerate()
                .filter(|(_, c)| c.len() > 1)
                .map(|(i, _)| i)
                .collect();

            let results: Arc<std::sync::Mutex<Vec<Option<String>>>> =
                Arc::new(std::sync::Mutex::new(
                    (0..all_clusters.len()).map(|_| None).collect(),
                ));

            let sem = Arc::new(tokio::sync::Semaphore::new(req.concurrency));
            let router = Arc::clone(&self.inference_router);
            let model_override = configured_qa_model(None);

            let mut handles = Vec::with_capacity(multi_indices.len());
            for &ci in &multi_indices {
                let router = Arc::clone(&router);
                let sem = Arc::clone(&sem);
                let results = Arc::clone(&results);
                let model_override = model_override.clone();
                let cluster = &all_clusters[ci];

                let texts: Vec<String> = cluster
                    .iter()
                    .map(|&idx| chunks[idx].text.clone())
                    .collect();
                let source = chunks[cluster[0]].source.clone();
                let concepts: Vec<String> = cluster
                    .iter()
                    .flat_map(|&idx| chunks[idx].concepts.iter().cloned())
                    .collect::<std::collections::HashSet<String>>()
                    .into_iter()
                    .collect();

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await;

                    let mut passages = String::new();
                    for (i, text) in texts.iter().enumerate() {
                        passages.push_str(&format!("[Passage {}]\n{}\n\n", i + 1, text));
                    }

                    // Render consolidation prompt from Jinja2 template
                    // (registry/templates/docproc/consolidate-chunks.j2)
                    let mut vars = std::collections::HashMap::new();
                    vars.insert("passage_count", texts.len().to_string());
                    vars.insert("source", source.clone());
                    vars.insert("concepts", concepts.join(", "));
                    vars.insert("passages", passages.clone());
                    let combined = render_docproc_template("consolidate-chunks", &vars);
                    let combined = if combined.is_empty() {
                        format!(
                            "You are a corpus consolidator. Synthesize {n} overlapping passages from \"{source}\" (concepts: {concepts}) into a single comprehensive passage. Preserve ALL unique information, remove redundancy. Output only the consolidated passage text.\n\n{passages}",
                            n = texts.len(), source = source, concepts = concepts.join(", "), passages = passages
                        )
                    } else {
                        combined
                    };

                    // ContentGuard — mandatory at every LLM boundary
                    let input_scan = GUARD.scan_input(&combined);
                    if !input_scan.passed {
                        let mut results = results.lock().unwrap();
                        results[ci] = Some("__FALLBACK__".to_string());
                        return;
                    }

                    let params = LLMParameters {
                        temperature: 0.3,
                        max_tokens: 4096,
                        ..Default::default()
                    };

                    match router
                        .generate_with_model(&combined, &params, model_override.as_deref(), None)
                        .await
                    {
                        Ok(response) => {
                            let output_scan = GUARD.scan_output(&response.text);
                            let content = output_scan.output.content(&response.text);
                            let text = content.trim().to_string();
                            let mut results = results.lock().unwrap();
                            results[ci] = Some(text);
                        }
                        Err(_) => {
                            let mut results = results.lock().unwrap();
                            results[ci] = Some("__FALLBACK__".to_string());
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.await;
            }

            // Phase 3: Build consolidated TaggedChunks (collect data, then drop guard)
            let consolidated_texts: Vec<Option<String>> = results.lock().unwrap().clone();
            let mut consolidated: Vec<TaggedChunk> = Vec::with_capacity(all_clusters.len());
            let mut reembed_texts: Vec<(String, String)> = Vec::new();

            for (ci, cluster) in all_clusters.iter().enumerate() {
                if cluster.len() == 1 {
                    consolidated.push(chunks[cluster[0]].clone());
                } else {
                    let llm_text = consolidated_texts[ci].as_ref().unwrap();
                    let source = &chunks[cluster[0]].source;
                    let entity_ref = format!("corpus:researcher:consolidated:{source}:{ci}");

                    let text = if llm_text == "__FALLBACK__" {
                        chunks[cluster[0]].text.clone()
                    } else {
                        llm_text.clone()
                    };

                    let concepts: Vec<String> = cluster
                        .iter()
                        .flat_map(|&idx| chunks[idx].concepts.iter().cloned())
                        .collect::<std::collections::HashSet<String>>()
                        .into_iter()
                        .collect();

                    let salience = cluster
                        .iter()
                        .map(|&idx| chunks[idx].salience)
                        .fold(0.0f32, f32::max);
                    let consolidated_from: Vec<String> = cluster
                        .iter()
                        .map(|&idx| chunks[idx].entity_ref.clone())
                        .collect();

                    reembed_texts.push((entity_ref.clone(), text.clone()));

                    // Dublin Core + PKO metadata for the consolidated chunk
                    let ontology = ChunkOntology {
                        dc_type: hkask_bridge_dublincore::DOCUMENT.to_string(),
                        dc_subject: concepts.clone(),
                        dc_source: source.clone(),
                        pko_extracted_from: consolidated_from.clone(),
                    };

                    // Merge ontology tags from all cluster members
                    let dimensions: Vec<String> = cluster
                        .iter()
                        .flat_map(|&idx| chunks[idx].dimensions.iter().cloned())
                        .collect::<std::collections::HashSet<String>>()
                        .into_iter()
                        .collect();
                    let dc_type = chunks[cluster[0]].dc_type.clone();
                    let dc_subject: Vec<String> = cluster
                        .iter()
                        .flat_map(|&idx| chunks[idx].dc_subject.iter().cloned())
                        .collect::<std::collections::HashSet<String>>()
                        .into_iter()
                        .collect();
                    // Merge ontology_tags: union all concept lists per namespace
                    let mut merged_tags: std::collections::HashMap<String, std::collections::HashSet<String>> =
                        std::collections::HashMap::new();
                    for &idx in cluster {
                        for (ns, concepts) in &chunks[idx].ontology_tags {
                            merged_tags
                                .entry(ns.clone())
                                .or_default()
                                .extend(concepts.iter().cloned());
                        }
                    }
                    let ontology_tags: std::collections::HashMap<String, Vec<String>> = merged_tags
                        .into_iter()
                        .map(|(ns, set)| (ns, set.into_iter().collect()))
                        .collect();
                    // Rebuild concepts cache from merged ontology_tags
                    let concepts: Vec<String> = {
                        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                        let mut v = Vec::new();
                        for concepts_list in ontology_tags.values() {
                            for c in concepts_list {
                                if seen.insert(c.clone()) {
                                    v.push(c.clone());
                                }
                            }
                        }
                        v
                    };
                    // Take highest expertise level (researcher > analyst > practitioner)
                    let expertise_level = cluster
                        .iter()
                        .map(|&idx| match chunks[idx].expertise_level.as_str() {
                            "researcher" => 3,
                            "analyst" => 2,
                            "practitioner" => 1,
                            _ => 0,
                        })
                        .max()
                        .map(|level| match level {
                            3 => "researcher",
                            2 => "analyst",
                            _ => "practitioner",
                        })
                        .unwrap_or("analyst")
                        .to_string();

                    let word_count = text.split_whitespace().count();
                    consolidated.push(TaggedChunk {
                        entity_ref,
                        source: source.clone(),
                        text,
                        word_count,
                        dimensions,
                        dc_type,
                        dc_subject,
                        ontology_tags,
                        concepts,
                        expertise_level,
                        salience,
                        consolidated_from,
                        ontology: Some(ontology),
                    });
                }
            }

            // Phase 4: Re-embed consolidated chunks
            let mut embedded_count = 0usize;
            if !reembed_texts.is_empty() && self.embedding_router.is_some() {
                let emb_router = self.embedding_router.as_ref().unwrap();
                let emb_model = std::env::var("HKASK_EMBEDDING_MODEL")
                    .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string());

                for batch in reembed_texts.chunks(50) {
                    let texts: Vec<&str> = batch.iter().map(|(_, t)| t.as_str()).collect();
                    if let Ok(vectors) = emb_router.embed_sentences(&emb_model, &texts).await {
                        for ((entity_ref, _), vector) in batch.iter().zip(vectors.iter()) {
                            if semantic
                                .store_embedding(entity_ref, vector, &emb_model)
                                .is_ok()
                            {
                                embedded_count += 1;
                            }
                        }
                    }
                }
            }

            // Phase 5: Write output
            let mut out = String::new();
            for chunk in &consolidated {
                out.push_str(&serde_json::to_string(chunk)
                    .map_err(|e| McpToolError::internal(format!("Serialize: {e}")))?);
                out.push('\n');
            }
            std::fs::write(&req.output, &out).map_err(|e| {
                McpToolError::internal(format!("Cannot write output '{}': {e}", req.output))
            })?;

            let result = json!({
                "original": chunks.len(),
                "consolidated": consolidated.len(),
                "multi_chunk_clusters": multi,
                "absorbed": absorbed,
                "reembedded": embedded_count,
                "reduction_pct": (1.0 - consolidated.len() as f64 / chunks.len().max(1) as f64) * 100.0,
            });

            self.record_experience(
                "docproc_consolidate_chunks",
                &format!("{} → {}", chunks.len(), consolidated.len()),
                "success",
                result.clone(),
            );
            Ok(result)
        })
        .await
    }

    // ── Build Prompts ──────────────────────────────────────────────────────

    #[tool(
        description = "Build QA generation prompts from tagged chunks with KNN context scaffold, ontology context, and h_mem knowledge graph. For each chunk, retrieves embedding-similar passages (KNN), formats ontology tags (5W1H + Dublin Core + PKO), and queries h_mems from the memory DB to build a knowledge graph section. Outputs prompts JSONL consumed by docproc_generate_qa_batch."
    )]
    pub async fn docproc_build_prompts(
        &self,
        Parameters(req): Parameters<BuildPromptsRequest>,
    ) -> String {
        execute_tool(self, "docproc_build_prompts", async {
            let chunks = read_tagged_chunks(&req.tagged_jsonl)?;
            if chunks.is_empty() {
                return Err(McpToolError::invalid_argument("tagged_jsonl is empty"));
            }
            let total = chunks.len();
            tracing::info!("  Build prompts: {} chunks", total);

            // QA type rotation
            let type_rotation = parse_type_distribution(&req.type_distribution);
            let limit = if req.max_prompts > 0 {
                req.max_prompts.min(total)
            } else {
                total
            };

            // Sort by salience descending
            let mut sorted: Vec<&TaggedChunk> = chunks.iter().collect();
            sorted.sort_by(|a, b| {
                b.salience
                    .partial_cmp(&a.salience)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Bulk-load embeddings for in-memory KNN
            let dim = embedding_dim();
            let semantic = SemanticMemory::open(&req.db_path, &req.passphrase, dim)
                .map_err(|e| McpToolError::failed_precondition(format!("Cannot open memory DB: {e}")))?;

            let text_map: std::collections::HashMap<&str, &str> = chunks
                .iter()
                .map(|c| (c.entity_ref.as_str(), c.text.as_str()))
                .collect();
            let source_map: std::collections::HashMap<&str, &str> = chunks
                .iter()
                .map(|c| (c.entity_ref.as_str(), c.source.as_str()))
                .collect();

            let emb_map: std::collections::HashMap<String, Vec<f32>> =
                match semantic.embeddings_by_prefix("corpus:researcher:") {
                    Ok(embs) => {
                        let map: std::collections::HashMap<String, Vec<f32>> = embs
                            .into_iter()
                            .map(|(er, mut v)| {
                                normalize_in_place(&mut v);
                                (er, v)
                            })
                            .collect();
                        tracing::info!("  Bulk-loaded {} normalized embeddings", map.len());
                        map
                    }
                    Err(e) => {
                        tracing::info!("  Warning: embedding query failed — scaffold disabled: {e}");
                        std::collections::HashMap::new()
                    }
                };

            // Group embeddings by source for scoped KNN
            let mut emb_by_source: std::collections::HashMap<&str, Vec<(&String, &Vec<f32>)>> =
                std::collections::HashMap::new();
            for chunk in &chunks {
                if let Some(v) = emb_map.get(&chunk.entity_ref) {
                    emb_by_source
                        .entry(chunk.source.as_str())
                        .or_default()
                        .push((&chunk.entity_ref, v));
                }
            }

            // Build concept graph (concept → chunk_count)
            let mut concept_connections: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for chunk in &chunks {
                for concept in &chunk.concepts {
                    *concept_connections.entry(concept.as_str()).or_default() += 1;
                }
            }

            // Owner is passed to the template via vars; WebID not needed here since
            // build_prompts only queries h_mems (read-only), doesn't store new ones.

            let mut out = String::new();
            let mut ti = 0usize;

            for tc in sorted.iter().take(limit) {
                // KNN scaffold: source-scoped search
                let context_passages: Vec<serde_json::Value> = {
                    let query_vec = match emb_map.get(&tc.entity_ref) {
                        Some(v) => v.as_slice(),
                        None => &[],
                    };
                    if query_vec.is_empty() {
                        Vec::new()
                    } else {
                        let k = req.context_k;
                        let candidates = emb_by_source
                            .get(tc.source.as_str())
                            .map(|v| v.as_slice())
                            .unwrap_or(&[]);
                        let mut scored: Vec<(&String, f32)> = candidates
                            .iter()
                            .filter(|(er, _)| er.as_str() != tc.entity_ref)
                            .map(|(er, v)| {
                                let dot: f32 = query_vec.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
                                (*er, dot)
                            })
                            .collect();
                        let top_k: Vec<(&String, f32)> = if scored.len() > k {
                            scored.select_nth_unstable_by(k.saturating_sub(1), |a, b| {
                                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                            });
                            scored[..k]
                                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                            scored.into_iter().take(k).collect()
                        } else {
                            scored
                                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                            scored.into_iter().collect()
                        };
                        top_k
                            .into_iter()
                            .map(|(er, sim)| {
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

                // Format context text
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
                            format!("[{}] Source: {}, Similarity: {:.2}\n    {}", i + 1, source, sim, truncated)
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n")
                };

                // Concept graph
                let concept_graph_text = tc.concepts.iter().map(|concept| {
                    let connected = concept_connections.get(concept.as_str()).copied().unwrap_or(1);
                    format!("- {} (connected to {} chunks)", concept, connected)
                }).collect::<Vec<_>>().join("\n");
                let concept_graph_text = if concept_graph_text.is_empty() {
                    "(none)".to_string()
                } else {
                    concept_graph_text
                };

                // h_mem knowledge graph — query all h_mems for this chunk
                let kg_text = match semantic.query_deduped(&tc.entity_ref) {
                    Ok(h_mems) if !h_mems.is_empty() => {
                        let mut lines: Vec<String> = Vec::new();
                        for h_mem in &h_mems {
                            if h_mem.attribute == "text" || h_mem.attribute == "corpus_provenance" || h_mem.attribute == "ontology_tags" {
                                continue; // skip non-triple h_mems
                            }
                            let dim = h_mem.dimension.as_ref().map(|d| d.as_str()).unwrap_or("what");
                            let val = match &h_mem.value {
                                serde_json::Value::String(s) => s.clone(),
                                v => v.to_string(),
                            };
                            lines.push(format!("  - [{}] {} --{}--> {}", dim, tc.entity_ref, h_mem.attribute, val));
                        }
                        if lines.is_empty() {
                            "(none)".to_string()
                        } else {
                            lines.join("\n")
                        }
                    }
                    _ => "(none)".to_string(),
                };

                // Generate prompts_per_chunk QAs per chunk at consecutive Bloom levels
                for offset in 0..req.prompts_per_chunk {
                    let qt = type_rotation[(ti + offset) % type_rotation.len()];
                    let qt_str = qa_type_str(qt);

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
                    let dc_type = if tc.dc_type.is_empty() {
                        "bibo:Document"
                    } else {
                        tc.dc_type.as_str()
                    };
                    let dc_subject = if tc.dc_subject.is_empty() {
                        tc.concepts.join(", ")
                    } else {
                        tc.dc_subject.join(", ")
                    };
                    let tags_str: String = tc
                        .ontology_tags
                        .iter()
                        .map(|(ns, concepts)| format!("{}: {}", ns, concepts.join(", ")))
                        .collect::<Vec<_>>()
                        .join(" | ");
                    let consolidated_from = if tc.consolidated_from.is_empty() {
                        String::new()
                    } else {
                        tc.consolidated_from.len().to_string()
                    };

                    // Render system prompt from Jinja2 template
                    let mut vars: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
                    vars.insert("qa_instruction", qa_type_instruction(qt).to_string());
                    vars.insert("dimensions", dimensions_str.clone());
                    vars.insert("qa_type", qt_str.to_string());
                    vars.insert("expertise", expertise.to_string());
                    vars.insert("source", tc.source.clone());
                    vars.insert("dc_type", dc_type.to_string());
                    vars.insert("dc_subject", dc_subject.clone());
                    vars.insert("consolidated_from", consolidated_from);
                    vars.insert("ontology_tags", if tags_str.is_empty() { "(none)".to_string() } else { tags_str.clone() });
                    vars.insert("context_passages", context_text.clone());
                    vars.insert("concept_graph", concept_graph_text.clone());
                    vars.insert("knowledge_graph", kg_text.clone());
                    let system = render_docproc_template("build-prompts", &vars);
                    let system = if system.is_empty() {
                        // Fallback if template not found
                        format!(
                            "You are a Capabilities Researcher training data generator. Given a primary passage from capabilities and research literature, generate ONE question-answer pair. Calibrate question depth to the expertise level indicated below.\n\n{}\n\n## Ontological Context\n5W1H: [{}]. QA at {} for {} expertise.\nSource: {}. Tags: {}\n\n## Context Passages\n{}\n\n## Knowledge Graph\n{}",
                            qa_type_instruction(qt), dimensions_str, qt_str, expertise, tc.source,
                            if tags_str.is_empty() { "(none)" } else { &tags_str },
                            context_text, kg_text
                        )
                    } else {
                        system
                    };

                    let prompt = serde_json::json!({
                        "chunk_ref": tc.entity_ref,
                        "source": tc.source,
                        "concepts": tc.concepts,
                        "salience": tc.salience,
                        "qa_type": qt_str,
                        "system": system,
                        "user": format!("Generate a {} QA pair from this passage:\n\n---\n{}\n---\n\nConcepts: {}\n\nInclude this chunk_ref in your output: {}", qt_str, tc.text, tc.concepts.join(", "), tc.entity_ref),
                    });
                    out.push_str(&serde_json::to_string(&prompt).unwrap_or_default());
                    out.push('\n');
                }
                ti += req.prompts_per_chunk;
            }

            std::fs::write(&req.output, &out).map_err(|e| {
                McpToolError::internal(format!("Cannot write output '{}': {e}", req.output))
            })?;

            let result = json!({
                "total_chunks": total,
                "prompts_written": ti,
                "output": req.output,
            });
            self.record_experience(
                "docproc_build_prompts",
                &format!("{} prompts from {} chunks", ti, total),
                "success",
                result.clone(),
            );
            Ok(result)
        })
        .await
    }

    // ── Ingest QA ─────────────────────────────────────────────────────────

    #[tool(
        description = "Ingest generated QA pairs: parse, quality-filter, SemDeDup (k-means cluster + within-cluster cosine dedup), write training JSONL, store QA h_mems with 5W1H dimension + Dublin Core / PKO metadata, and optionally store QA embeddings. Uses proven SemDeDup algorithm (Abbas et al., 2023)."
    )]
    pub async fn docproc_ingest_qa(&self, Parameters(req): Parameters<IngestQaRequest>) -> String {
        execute_tool(self, "docproc_ingest_qa", async {
            let content = std::fs::read_to_string(&req.generated_jsonl).map_err(|e| {
                McpToolError::invalid_argument(format!("Cannot read generated_jsonl '{}': {e}", req.generated_jsonl))
            })?;

            // Parse QA records — handle both flat and envelope formats
            let mut malformed = 0usize;
            let qas: Vec<ParsedQa> = content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|line| parse_qa_record(line).or_else(|| {
                    malformed += 1;
                    None
                }))
                .collect();
            tracing::info!("  Parsed: {} ({} malformed rejected)", qas.len(), malformed);

            // Quality filter
            let filtered: Vec<&ParsedQa> = qas
                .iter()
                .filter(|q| {
                    q.instruction.len() >= 30
                        && q.output.len() >= 50
                        && !q.qa_type.is_empty()
                        && q.chunk_ref.is_some()
                })
                .collect();
            tracing::info!(
                "  Quality filter: {} (removed {})",
                filtered.len(),
                qas.len() - filtered.len()
            );

            let use_embed = req.dedup_threshold < 1.0 && self.embedding_router.is_some();
            let emb_model = hkask_inference::model_constants::embedding_model();

            // SemDeDup: embed → k-means → within-cluster dedup
            let mut deduped: Vec<(Option<Vec<f32>>, &ParsedQa)> = Vec::new();
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

            if use_embed {
                let emb_router = self.embedding_router.as_ref().unwrap();
                let instructions: Vec<&str> = filtered.iter().map(|q| q.instruction.as_str()).collect();
                let mut all_v: Vec<Vec<f32>> = Vec::new();
                for batch in instructions.chunks(50) {
                    match emb_router.embed_sentences(&emb_model, batch).await {
                        Ok(v) => { all_v.extend(v); }
                        Err(e) => {
                            tracing::warn!("  WARN: embed batch: {e}");
                            for _ in 0..batch.len() { all_v.push(Vec::new()); }
                        }
                    }
                }

                // Pre-normalize
                let normalized: Vec<Vec<f32>> = all_v.iter().map(|v| {
                    if v.is_empty() { return Vec::new(); }
                    let mut nv = v.clone();
                    normalize_in_place(&mut nv);
                    nv
                }).collect();

                let threshold = req.dedup_threshold as f32;

                // K-means clustering
                let embedded_indices: Vec<usize> = (0..filtered.len())
                    .filter(|&i| i < normalized.len() && !normalized[i].is_empty())
                    .collect();
                let n = embedded_indices.len();
                let k = ((n as f64) * 0.025).round().max(2.0) as usize;
                tracing::info!("  SemDeDup: {} embedded QAs, {} clusters", n, k);

                let assignments = kmeans_cluster(&normalized, &embedded_indices, k, 10);

                // Within-cluster greedy dedup
                for cluster_indices in &assignments {
                    let mut sorted = cluster_indices.clone();
                    sorted.sort_by(|&a, &b| filtered[b].instruction.len().cmp(&filtered[a].instruction.len()));
                    let mut kept: Vec<usize> = Vec::new();
                    for &i in &sorted {
                        let is_dup = kept.iter().any(|&k| {
                            let dot: f32 = normalized[i].iter().zip(normalized[k].iter()).map(|(a, b)| a * b).sum();
                            dot > threshold
                        });
                        if !is_dup {
                            kept.push(i);
                            deduped.push((Some(normalized[i].clone()), &filtered[i]));
                        }
                    }
                }

                // QAs without embeddings: exact-match dedup
                for i in 0..filtered.len() {
                    if i >= normalized.len() || normalized[i].is_empty() {
                        if seen.insert(filtered[i].instruction.to_lowercase()) {
                            deduped.push((None, &filtered[i]));
                        }
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
            tracing::info!("  Deduped: {} (removed {})", deduped_count, filtered.len() - deduped_count);

            if req.dry_run {
                return Ok(json!({
                    "parsed": qas.len(),
                    "filtered": filtered.len(),
                    "deduped": deduped_count,
                    "dry_run": true,
                }));
            }

            // Write training JSONL
            let train: String = deduped.iter().map(|(_, q)| {
                serde_json::to_string(&serde_json::json!({"instruction": q.instruction, "input": "", "output": q.output}))
                    .unwrap_or_default()
            }).collect::<Vec<_>>().join("\n");
            std::fs::write(&req.output, train + "\n").map_err(|e| {
                McpToolError::internal(format!("Cannot write output '{}': {e}", req.output))
            })?;
            tracing::info!("  Wrote: {} QAs to {}", deduped_count, req.output);

            // Store h_mems + embeddings
            let dim = embedding_dim();
            let semantic = SemanticMemory::open(&req.db_path, &req.passphrase, dim)
                .map_err(|e| McpToolError::failed_precondition(format!("Cannot open memory DB: {e}")))?;
            let webid = owner_webid(&req.owner);
            let mut stored = 0usize;
            let mut embed_stored = 0usize;

            for (i, (_emb, qa)) in deduped.iter().enumerate() {
                let entity = format!("training:qa:{}:{}:{}", req.dataset, qa.source, i);
                let v = serde_json::json!({
                    "question": qa.instruction,
                    "answer": qa.output,
                    "bloom_level": qa.qa_type,
                    "source": qa.source,
                    "dataset": req.dataset,
                    "difficulty": qa.difficulty,
                    "concepts": qa.concepts,
                    "chunk_ref": qa.chunk_ref,
                    "evidence_quotes": qa.evidence_quotes,
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
                let h_mem = hkask_storage::HMem::new(&entity, "training_qa_pair", v, webid)
                    .with_visibility(hkask_types::Visibility::Public)
                    .with_confidence(0.8)
                    .with_dimension(hkask_types::Dimension::What);
                if semantic.store(h_mem).is_ok() {
                    stored += 1;
                }

                // Store QA embedding
                if req.embed_qas {
                    if let Some(vec) = _emb {
                        if semantic.store_embedding(&entity, vec, &emb_model).is_ok() {
                            embed_stored += 1;
                        }
                    }
                }
            }
            tracing::info!("  Stored: {} QA h_mems, {} embeddings", stored, embed_stored);

            let result = json!({
                "parsed": qas.len(),
                "filtered": filtered.len(),
                "deduped": deduped_count,
                "stored_h_mems": stored,
                "stored_embeddings": embed_stored,
                "output": req.output,
            });
            self.record_experience(
                "docproc_ingest_qa",
                &format!("{} deduped from {}", deduped_count, qas.len()),
                "success",
                result.clone(),
            );
            Ok(result)
        })
        .await
    }
}

// ── Build Prompts helpers ─────────────────────────────────────────────────

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

fn qa_type_str(qt: QaType) -> &'static str {
    qt.as_str()
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
    if result.is_empty() {
        vec![QaType::Factual]
    } else {
        result
    }
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

// ── Ingest QA helpers ──────────────────────────────────────────────────────

struct ParsedQa {
    instruction: String,
    output: String,
    qa_type: String,
    difficulty: usize,
    concepts: Vec<String>,
    source: String,
    chunk_ref: Option<String>,
    evidence_quotes: Vec<String>,
}

/// Parse a QA record from a JSONL line. Handles both flat and envelope formats.
fn parse_qa_record(line: &str) -> Option<ParsedQa> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    // Flat format: {"instruction": ..., "output": ..., "qa_type": ...}
    // Envelope format: {"chunk_ref": ..., "source": ..., "qa_type": ..., "response": {...}}
    let (instruction, output, qa_type, difficulty, concepts, source, chunk_ref, evidence_quotes) =
        if let Some(resp) = v.get("response").and_then(|r| r.as_object()) {
            // Envelope format
            (
                resp.get("instruction")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                resp.get("output")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                v.get("qa_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                resp.get("difficulty").and_then(|v| v.as_u64()).unwrap_or(3) as usize,
                resp.get("concepts")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                v.get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                v.get("chunk_ref")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                resp.get("evidence_quotes")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
            )
        } else {
            // Flat format
            (
                v.get("instruction")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                v.get("output")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                v.get("qa_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                v.get("difficulty").and_then(|v| v.as_u64()).unwrap_or(3) as usize,
                v.get("concepts")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                v.get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                v.get("chunk_ref")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                v.get("evidence_quotes")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
            )
        };
    if instruction.is_empty() || output.is_empty() {
        return None;
    }
    Some(ParsedQa {
        instruction,
        output,
        qa_type,
        difficulty,
        concepts,
        source,
        chunk_ref,
        evidence_quotes,
    })
}

/// Simple k-means clustering on normalized vectors (dot product = cosine sim).
/// Returns cluster assignments as Vecs of indices into `filtered`.
fn kmeans_cluster(
    vectors: &[Vec<f32>],
    indices: &[usize],
    k: usize,
    iterations: usize,
) -> Vec<Vec<usize>> {
    let n = indices.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }
    let k = k.min(n);
    let dim = vectors[indices[0]].len();

    // Initialize centroids: pick k evenly spaced points
    let mut centroids: Vec<Vec<f32>> = (0..k)
        .map(|i| vectors[indices[i * n / k]].clone())
        .collect();

    let mut assignments: Vec<usize> = vec![0; n];

    for _ in 0..iterations {
        // Assign each point to nearest centroid
        for (pi, &idx) in indices.iter().enumerate() {
            let v = &vectors[idx];
            let mut best = 0;
            let mut best_dist = f32::MAX;
            for (ci, c) in centroids.iter().enumerate() {
                // Distance = 1 - cosine similarity (vectors are normalized)
                let dot: f32 = v.iter().zip(c.iter()).map(|(a, b)| a * b).sum();
                let dist = 1.0 - dot;
                if dist < best_dist {
                    best_dist = dist;
                    best = ci;
                }
            }
            assignments[pi] = best;
        }

        // Update centroids
        let mut new_centroids: Vec<Vec<f32>> = vec![vec![0.0; dim]; k];
        let mut counts: Vec<usize> = vec![0; k];
        for (pi, &idx) in indices.iter().enumerate() {
            let c = assignments[pi];
            counts[c] += 1;
            for (j, &val) in vectors[idx].iter().enumerate() {
                new_centroids[c][j] += val;
            }
        }
        for (ci, c) in new_centroids.iter_mut().enumerate() {
            if counts[ci] > 0 {
                for val in c.iter_mut() {
                    *val /= counts[ci] as f32;
                }
                normalize_in_place(c);
            } else {
                // Reinitialize empty cluster to a random point
                *c = vectors[indices[ci % n]].clone();
            }
        }
        centroids = new_centroids;
    }

    // Build cluster groups
    let mut clusters: Vec<Vec<usize>> = vec![Vec::new(); k];
    for (pi, &idx) in indices.iter().enumerate() {
        clusters[assignments[pi]].push(idx);
    }
    clusters.into_iter().filter(|c| !c.is_empty()).collect()
}

// ── Corpus pipeline request structs ───────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DedupChunksRequest {
    /// Path to tagged chunks JSONL (from salience phase).
    pub tagged_jsonl: String,
    /// Output path for deduplicated tagged chunks JSONL.
    pub output: String,
    /// Path to the SQLCipher memory DB containing chunk embeddings.
    pub db_path: String,
    /// Passphrase for the memory DB.
    pub passphrase: String,
    /// Entity-ref prefix for chunk embeddings in the DB (e.g. "corpus:researcher:").
    #[serde(default = "default_corpus_prefix")]
    pub prefix: String,
    /// Cosine similarity threshold — chunks above this are near-duplicates.
    #[serde(default = "default_dedup_threshold")]
    pub threshold: f64,
    /// If true, only report clustering stats without writing output.
    #[serde(default)]
    pub dry_run: bool,
}

fn default_corpus_prefix() -> String {
    "corpus:researcher:".to_string()
}

fn default_dedup_threshold() -> f64 {
    0.85
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConsolidateChunksRequest {
    /// Path to tagged chunks JSONL (from dedup or salience phase).
    pub tagged_jsonl: String,
    /// Output path for consolidated tagged chunks JSONL.
    pub output: String,
    /// Path to the SQLCipher memory DB.
    pub db_path: String,
    /// Passphrase for the memory DB.
    pub passphrase: String,
    /// Entity-ref prefix for chunk embeddings.
    #[serde(default = "default_corpus_prefix")]
    pub prefix: String,
    /// Cosine similarity threshold for clustering (0.75 = semantic overlap).
    #[serde(default = "default_consolidate_threshold")]
    pub threshold: f64,
    /// Max concurrent LLM consolidation calls.
    #[serde(default = "default_consolidate_concurrency")]
    pub concurrency: usize,
    /// Max chunks per consolidation cluster (limits LLM context).
    #[serde(default = "default_max_chunks_per_cluster")]
    pub max_chunks_per_cluster: usize,
    /// If true, only report clustering stats without LLM calls.
    #[serde(default)]
    pub dry_run: bool,
}

fn default_consolidate_threshold() -> f64 {
    0.75
}

fn default_consolidate_concurrency() -> usize {
    12
}

fn default_max_chunks_per_cluster() -> usize {
    5
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BuildPromptsRequest {
    /// Path to tagged chunks JSONL (from consolidate phase).
    pub tagged_jsonl: String,
    /// Output path for prompts JSONL (one JSON per line, consumed by generate_qa_batch).
    pub output: String,
    /// Path to the SQLCipher memory DB for embedding retrieval + h_mem knowledge graph.
    pub db_path: String,
    /// Passphrase for the memory DB.
    pub passphrase: String,
    /// Number of KNN context passages to retrieve per chunk (default 3).
    #[serde(default = "default_context_k")]
    pub context_k: usize,
    /// Number of Bloom-level QA prompts per chunk (default 5 — one per level).
    #[serde(default = "default_prompts_per_chunk")]
    pub prompts_per_chunk: usize,
    /// Bloom's taxonomy weight distribution (e.g. "1,1,1,1,1" = equal).
    #[serde(default = "default_type_distribution")]
    pub type_distribution: String,
    /// Generate cross-reference synthesis prompts.
    #[serde(default)]
    pub cross_reference: bool,
    /// Max prompts to output (0 = all qualifying chunks).
    #[serde(default)]
    pub max_prompts: usize,
    /// Owner persona for h_mem queries (e.g. "john-brooks").
    #[serde(default = "default_owner")]
    pub owner: String,
}

fn default_context_k() -> usize {
    3
}

fn default_prompts_per_chunk() -> usize {
    5
}

fn default_type_distribution() -> String {
    "1,1,1,1,1".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IngestQaRequest {
    /// Path to generated QAs JSONL (from docproc_generate_qa_batch).
    pub generated_jsonl: String,
    /// Output path for training-ready JSONL (instruction/input/output per line).
    pub output: String,
    /// Path to the SQLCipher memory DB for h_mem + embedding storage.
    pub db_path: String,
    /// Passphrase for the memory DB.
    pub passphrase: String,
    /// SemDeDup cosine similarity threshold (0.89 = moderate, 0.92 = strict).
    #[serde(default = "default_dedup_threshold_ingest")]
    pub dedup_threshold: f64,
    /// If true, validate and dedup without storing.
    #[serde(default)]
    pub dry_run: bool,
    /// Store QA embedding vectors in EmbeddingStore for KNN search.
    #[serde(default)]
    pub embed_qas: bool,
    /// Dataset name for training_qa_pair h_mems.
    #[serde(default = "default_dataset")]
    pub dataset: String,
    /// Owner persona for stored h_mems (e.g. "john-brooks").
    #[serde(default = "default_owner")]
    pub owner: String,
}

fn default_dedup_threshold_ingest() -> f64 {
    0.89
}

fn default_dataset() -> String {
    "capabilities-researcher".to_string()
}
