//! Semantic extraction tools — QA generation, triple extraction, embedding.
use crate::*;

#[tool_router(router = semantic_router, vis = "pub")]
impl DocProcServer {
    #[tool(
        description = "Generate QA pairs from a text chunk by calling the inference engine. Returns structured question-answer pairs at specified Bloom's taxonomy levels."
    )]
    pub async fn docproc_generate_qa(
        &self,
        Parameters(GenerateQaRequest {
            text,
            chunk_id,
            bloom_levels,
        }): Parameters<GenerateQaRequest>,
    ) -> String {
        execute_tool(self, "docproc_generate_qa", async {
            if text.is_empty() {
                return Err(McpToolError::invalid_argument("text must not be empty"));
            }

            if chunk_id.is_empty() {
                return Err(McpToolError::invalid_argument("chunk_id must not be empty"));
            }

            let levels = bloom_levels
                .unwrap_or_else(|| vec!["factual".to_string(), "conceptual".to_string()]);

            let levels_str = levels.join(", ");

            // C10: Load prompt from registry template, fall back to inline if unavailable
            let mut vars = std::collections::HashMap::new();
            vars.insert("levels", levels_str.clone());
            vars.insert("chunk_id", chunk_id.clone());
            vars.insert("text", text.clone());
            let prompt = render_docproc_template("generate-qa", &vars);
            let prompt = if prompt.is_empty() {
                format!(
                    "Based on the following text, generate question-answer pairs at these Bloom's taxonomy levels: {levels_str}.\n\n\
                     Text (chunk {chunk_id}):\n{text}\n\n\
                     For each level, provide:\n\
                     - A question that tests understanding at that level\n\
                     - A concise, accurate answer derived from the text\n\
                     - The bloom_level classification\n\n\
                     Respond in JSON format: {{\"qa_pairs\": [{{\"question\": \"...\", \"answer\": \"...\", \"bloom_level\": \"...\"}}]}}"
                )
            } else {
                prompt
            };

            let router = InferenceRouter::new(self.inference_config.clone());
            let params = LLMParameters {
                temperature: 0.3,
                max_tokens: 4096,
                ..Default::default()
            };

            match router.generate(&prompt, &params, None).await {
                Ok(response) => {
                    let cleaned = strip_json_fences(&response.text);
                    let qa_pairs: serde_json::Value = match serde_json::from_str(&cleaned) {
                        Ok(v) => v,
                        Err(_) => {
                            json!({"raw_response": response.text, "parse_error": "LLM response was not valid JSON"})
                        }
                    };

                    let result = json!({
                        "chunk_id": chunk_id,
                        "bloom_levels": levels,
                        "qa_pairs": qa_pairs,
                        "tokens_used": response.usage.total_tokens,
                    });
                    self.record_experience(
                        "docproc_generate_qa",
                        &chunk_id,
                        "success",
                        result.clone(),
                    );
                    Ok(result)
                }
                Err(e) => Err(McpToolError::unavailable(format!(
                    "QA generation failed: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(
        description = "Extract RDF triples (subject, predicate, object) from text using the inference engine. Returns structured knowledge triples with confidence scores."
    )]
    pub async fn docproc_extract_triples(
        &self,
        Parameters(ExtractTriplesRequest {
            text,
            namespace,
            max_triples,
        }): Parameters<ExtractTriplesRequest>,
    ) -> String {
        execute_tool(self, "docproc_extract_triples", async {
            if text.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "text must not be empty",
                ));
            }

            let ns = namespace.unwrap_or_else(|| "doc".to_string());
            let limit = max_triples.unwrap_or(50);

            // C10: Load prompt from registry template, fall back to inline if unavailable
            let mut vars = std::collections::HashMap::new();
            vars.insert("limit", limit.to_string());
            vars.insert("namespace", ns.clone());
            vars.insert("text", text.clone());
            let prompt = render_docproc_template("extract-triples", &vars);
            let prompt = if prompt.is_empty() {
                format!(
                    "Extract up to {limit} factual RDF triples from the following text.\n\n\
                     Each triple should be in the form (subject, predicate, object) where:\n\
                     - subject: an entity mentioned in the text (prefix with '{ns}:')\n\
                     - predicate: a relationship or property (use standard RDF predicates like rdf:type, schema:name, etc.)\n\n\
                     - object: another entity, a literal value, or a type\n\n\
                     For each triple, also provide a confidence score (0.0-1.0) based on how clearly the text supports it.\n\n\
                     Text:\n{text}\n\n\
                     Respond in JSON format: {{\"triples\": [{{\"subject\": \"...\", \"predicate\": \"...\", \"object\": \"...\", \"confidence\": 0.95}}]}}"
                )
            } else {
                prompt
            };

            let router = InferenceRouter::new(self.inference_config.clone());
            let params = LLMParameters {
                temperature: 0.1,
                max_tokens: 4096,
                ..Default::default()
            };

            match router.generate(&prompt, &params, None).await {
                Ok(response) => {
                    let cleaned = strip_json_fences(&response.text);
                    let triples: serde_json::Value = match serde_json::from_str(&cleaned) {
                        Ok(v) => v,
                        Err(_) => {
                            json!({"raw_response": response.text, "parse_error": "LLM response was not valid JSON"})
                        }
                    };

                    let result = json!({
                        "namespace": ns,
                        "max_triples": limit,
                        "triples": triples,
                        "tokens_used": response.usage.total_tokens,
                    });
                    self.record_experience(
                        "docproc_extract_triples",
                        &ns,
                        "success",
                        result.clone(),
                    );
                    Ok(result)
                }
                Err(e) => Err(McpToolError::unavailable(format!(
                    "Triple extraction failed: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(
        description = "Generate embedding vectors for a list of texts (passages or triples). Uses the configured embedding model via the inference router."
    )]
    pub async fn docproc_embed(
        &self,
        Parameters(EmbedRequest { texts, model }): Parameters<EmbedRequest>,
    ) -> String {
        execute_tool(self, "docproc_embed", async {
            if texts.is_empty() {
                return Err(McpToolError::invalid_argument("texts must not be empty"));
            }

            let Some(ref emb_router) = self.embedding_router else {
                return Err(McpToolError::failed_precondition(
                    "Embedding router not configured — inference config may be missing",
                ));
            };

            let model_name = model.unwrap_or_else(|| {
                std::env::var("HKASK_EMBEDDING_MODEL")
                    .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string())
            });

            let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

            match emb_router.embed_sentences(&model_name, &text_refs).await {
                Ok(vectors) => {
                    let result = json!({
                        "count": texts.len(),
                        "dimensions": vectors.first().map(|v| v.len()).unwrap_or(0),
                        "vectors": vectors,
                        "model": model_name,
                    });
                    self.record_experience(
                        "docproc_embed",
                        &format!("{} texts", texts.len()),
                        "success",
                        result.clone(),
                    );
                    Ok(result)
                }
                Err(e) => Err(McpToolError::unavailable(format!(
                    "Embedding failed: {}",
                    e
                ))),
            }
        })
        .await
    }
}
