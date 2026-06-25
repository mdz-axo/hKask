//! Storage and query tools — cache, passage query, similarity.
use crate::*;

#[tool_router(router = storage_router, vis = "pub")]
impl DocProcServer {
    #[tool(
        description = "Cache processed document text for reference. Stores content keyed by label in the docproc cache directory (~/.config/hkask/docproc-cache/)."
    )]
    pub async fn docproc_cache(
        &self,
        Parameters(CacheRequest { content, label }): Parameters<CacheRequest>,
    ) -> String {
        execute_tool(self, "docproc_cache", async {
            if content.is_empty() {
                return Err(McpToolError::invalid_argument("content must not be empty"));
            }

            if label.is_empty() {
                return Err(McpToolError::invalid_argument("label must not be empty"));
            }

            // Resolve cache directory
            let cache_dir = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("hkask")
                .join("docproc-cache");

            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                return Err(McpToolError::internal(format!(
                    "Failed to create cache directory '{}': {}",
                    cache_dir.display(),
                    e
                )));
            }

            // Sanitize label for filesystem
            let safe_label: String = label
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let cache_path = cache_dir.join(format!("{}.md", safe_label));

            match std::fs::write(&cache_path, &content) {
                Ok(()) => {
                    let result = json!({
                        "label": label,
                        "path": cache_path.display().to_string(),
                        "size_bytes": content.len(),
                    });
                    self.record_experience("docproc_cache", &label, "success", result.clone());
                    Ok(result)
                }
                Err(e) => Err(McpToolError::internal(format!(
                    "Failed to write cache file '{}': {}",
                    cache_path.display(),
                    e
                ))),
            }
        })
        .await
    }

    #[tool(
        description = "Query the in-memory vector index for passages relevant to a natural language question. Embeds the query, computes cosine similarity against indexed passages, and returns top-k results. Optionally generates an LLM-augmented answer from retrieved context."
    )]
    pub async fn docproc_query(
        &self,
        Parameters(QueryRequest {
            query,
            top_k,
            generate_answer,
        }): Parameters<QueryRequest>,
    ) -> String {
        execute_tool(self, "docproc_query", async {
            if query.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "query must not be empty",
                ));
            }

            let k = top_k.unwrap_or(5).clamp(1, 50);

            // Embed the query
            let Some(ref emb_router) = self.embedding_router else {
                return Err(McpToolError::failed_precondition(
                    "Embedding router not configured — cannot embed query",
                ));
            };

            let model_name = std::env::var("HKASK_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string());

            let query_embedding = match emb_router
                .embed_sentences(&model_name, &[query.as_str()])
                .await
            {
                Ok(v) => v.into_iter().next().unwrap_or_default(),
                Err(e) => {
                    return Err(McpToolError::unavailable(format!(
                        "Query embedding failed: {}",
                        e
                    )));
                }
            };

            if query_embedding.is_empty() {
                return Err(McpToolError::unavailable(
                    "Query embedding returned empty vector",
                ));
            }

            // Search the index (scoped to drop guard before any await)
            let (results, total_indexed) = {
                let index = match self.index.lock() {
                    Ok(i) => i,
                    Err(e) => {
                        return Err(McpToolError::internal(format!(
                            "Index lock error: {}",
                            e
                        )));
                    }
                };
                if index.is_empty() {
                    return Ok(json!({
                        "query": query,
                        "results": [],
                        "total_indexed": 0,
                        "note": "No passages indexed. Run docproc_chunk with index=true first.",
                    }));
                }

                let mut scored: Vec<(f32, &IndexedPassage)> = index
                    .iter()
                    .map(|p| (cosine_similarity(&query_embedding, &p.embedding), p))
                    .collect();

                scored.sort_by(|a, b| {
                    b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
                });
                scored.truncate(k);

                let results: Vec<serde_json::Value> = scored
                    .iter()
                    .map(|(score, p)| {
                        json!({
                            "text": p.text.clone(),
                            "metadata": p.metadata.clone(),
                            "score": score,
                        })
                    })
                    .collect();

                (results, index.len())
            }; // guard dropped here

            let mut result = json!({
                "query": query,
                "results": results,
                "total_indexed": total_indexed,
            });

            // Optionally generate an LLM-augmented answer
            if generate_answer.unwrap_or(false) && !results.is_empty() {
                let context: String = results
                    .iter()
                    .map(|r| r["text"].as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join("\n\n");

                // C10: Load prompt from registry template, fall back to inline if unavailable
                let mut vars = std::collections::HashMap::new();
                vars.insert("context", context.clone());
                vars.insert("question", query.clone());
                let prompt = render_docproc_template("rag-answer", &vars);
                let prompt = if prompt.is_empty() {
                    format!(
                        "Answer the following question based on the provided context. If the context doesn't contain enough information, say so.\n\n\
                         Context:\n{context}\n\n\
                         Question: {query}\n\n\
                         Answer:"
                    )
                } else {
                    prompt
                };

                let router = InferenceRouter::new(self.inference_config.clone());
                let params = LLMParameters {
                    temperature: 0.3,
                    max_tokens: 1024,
                    ..Default::default()
                };

                match router.generate(&prompt, &params, None).await {
                    Ok(response) => {
                        result["answer"] = json!(response.text);
                        result["answer_tokens"] = json!(response.usage.total_tokens);
                    }
                    Err(e) => {
                        result["answer_error"] = json!(format!("{}", e));
                    }
                }
            }

            self.record_experience("docproc_query", &query, "success", result.clone());
            Ok(result)
        })
        .await
    }

    #[tool(
        description = "Clear the in-memory vector index. Call this when starting a new document set to avoid cross-document contamination in query results."
    )]
    pub async fn docproc_clear_index(
        &self,
        Parameters(ClearIndexRequest { index_id: _ }): Parameters<ClearIndexRequest>,
    ) -> String {
        execute_tool(self, "docproc_clear_index", async {
            let mut index = match self.index.lock() {
                Ok(i) => i,
                Err(e) => {
                    return Err(McpToolError::internal(format!("Index lock error: {}", e)));
                }
            };
            let cleared = index.len();
            index.clear();
            Ok(json!({"cleared": cleared}))
        })
        .await
    }
}
