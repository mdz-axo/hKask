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
use serde::Serialize;

/// Tagged chunk as produced by the salience phase.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    /// Provenance: original chunk refs consolidated into this chunk (pko:wasExtractedFrom).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    consolidated_from: Vec<String>,
    /// Dublin Core metadata for consolidated chunks.
    /// dcterms:type (bibo:Document), dcterms:subject, dcterms:source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ontology: Option<ChunkOntology>,
}

/// Dublin Core + PKO metadata attached to consolidated chunks.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct ChunkOntology {
    /// Dublin Core type (always "bibo:Document" for consolidated chunks).
    dc_type: String,
    /// Dublin Core subject — the concepts as ontology terms.
    dc_subject: Vec<String>,
    /// Dublin Core source — the original source file.
    dc_source: String,
    /// PKO provenance — wasExtractedFrom the original chunk refs.
    pko_extracted_from: Vec<String>,
}

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

fn embedding_dim() -> usize {
    std::env::var("HKASK_EMBEDDING_DIM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1024)
}

/// Pre-normalize a vector in place so cosine similarity becomes a dot product.
fn normalize_in_place(v: &mut [f32]) {
    let mag = (v.iter().map(|x| x * x).sum::<f32>()).sqrt();
    if mag > 0.0 {
        for x in v.iter_mut() {
            *x /= mag;
        }
    }
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
                    let methods: Vec<String> = cluster
                        .iter()
                        .flat_map(|&idx| chunks[idx].methods.iter().cloned())
                        .collect::<std::collections::HashSet<String>>()
                        .into_iter()
                        .collect();
                    let authors: Vec<String> = cluster
                        .iter()
                        .flat_map(|&idx| chunks[idx].authors.iter().cloned())
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

                    consolidated.push(TaggedChunk {
                        entity_ref,
                        source: source.clone(),
                        text,
                        concepts,
                        methods,
                        authors,
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
}
