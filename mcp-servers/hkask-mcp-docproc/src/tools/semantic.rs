//! Semantic extraction tools — QA generation, h_mem extraction, embedding.
use crate::*;
use serde::{Deserialize, Serialize};

// Content safety guard — mandatory at every LLM boundary (OWASP LLM01/02/04/06).
pub(crate) static GUARD: std::sync::LazyLock<hkask_guard::ContentGuard> =
    std::sync::LazyLock::new(|| {
        hkask_guard::ContentGuard::mandatory(&hkask_guard::GuardConfig::default())
    });

#[derive(Debug, Deserialize, Serialize)]
struct QaGenerationResponse {
    qa_pairs: Vec<QaPair>,
}

#[derive(Debug, Deserialize, Serialize)]
struct QaPair {
    question: String,
    answer: String,
    bloom_level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sources: Option<Vec<usize>>,
}

/// Parse model output into source-grounded QA pairs.
///
/// expect: "Generated QA data is safe to admit to the corpus only when it is complete and grounded."
/// [P4] Motivating: Clear Boundaries — the inference boundary rejects malformed or unsupported training data.
/// pre: response is JSON produced for the requested Bloom levels.
/// post: returns only non-empty pairs whose Bloom levels and cross-reference citations are valid.
/// inv: does not repair or silently reinterpret model output.
/// [P1] Constraining: User Sovereignty — provenance remains attached to generated training data.
fn parse_qa_response(
    response: &str,
    requested_levels: &[String],
    cross_reference_passage_count: Option<usize>,
) -> Result<QaGenerationResponse, String> {
    let parsed: QaGenerationResponse = serde_json::from_str(response)
        .map_err(|e| format!("QA response must be JSON with a qa_pairs array: {e}"))?;

    if parsed.qa_pairs.is_empty() {
        return Err("QA response must contain at least one QA pair".to_string());
    }

    for (index, pair) in parsed.qa_pairs.iter().enumerate() {
        if pair.question.trim().is_empty() || pair.answer.trim().is_empty() {
            return Err(format!(
                "QA pair {index} must have a non-empty question and answer"
            ));
        }
        if !requested_levels
            .iter()
            .any(|level| level == &pair.bloom_level)
        {
            return Err(format!(
                "QA pair {index} has unsupported Bloom level '{}'",
                pair.bloom_level
            ));
        }
        if let Some(passage_count) = cross_reference_passage_count {
            let sources = pair
                .sources
                .as_ref()
                .filter(|sources| !sources.is_empty())
                .ok_or_else(|| {
                    format!("cross-reference QA pair {index} must cite at least one passage")
                })?;
            if sources
                .iter()
                .any(|source| *source == 0 || *source > passage_count)
            {
                return Err(format!(
                    "cross-reference QA pair {index} cites a passage outside 1..={passage_count}"
                ));
            }
        }
    }

    Ok(parsed)
}

pub(crate) fn configured_qa_model(requested_model: Option<String>) -> Option<String> {
    requested_model
        .filter(|model| !model.trim().is_empty())
        .or_else(|| {
            std::env::var("HKASK_QA_MODEL")
                .ok()
                .filter(|model| !model.trim().is_empty())
        })
        .or_else(|| {
            std::env::var("HKASK_DEFAULT_MODEL")
                .ok()
                .filter(|model| !model.trim().is_empty())
        })
}

#[tool_router(router = semantic_router, vis = "pub")]
impl DocProcServer {
    #[tool(
        description = "Generate QA pairs from text chunks. Accepts a single chunk (text) or multiple chunks (texts) for cross-reference synthesis. Uses Bloom's taxonomy levels. Multi-chunk mode (texts) generates QAs that require synthesizing across all passages with source citation."
    )]
    pub async fn docproc_generate_qa(
        &self,
        Parameters(GenerateQaRequest {
            text: _text,
            texts: _texts,
            chunk_id,
            bloom_levels,
            model,
        }): Parameters<GenerateQaRequest>,
    ) -> String {
        execute_tool(self, "docproc_generate_qa", async {
            let is_cross_ref = _texts.as_ref().is_some_and(|t| !t.is_empty());
            let single_text = _text.unwrap_or_default();

            if !is_cross_ref && single_text.is_empty() {
                return Err(McpToolError::invalid_argument("text must not be empty (or set texts for cross-reference mode)"));
            }
            if chunk_id.is_empty() {
                return Err(McpToolError::invalid_argument("chunk_id must not be empty"));
            }

            let levels = bloom_levels
                .unwrap_or_else(|| vec!["factual".to_string(), "conceptual".to_string()]);
            let levels_str = levels.join(", ");

            let (prompt, template_source) = if is_cross_ref {
                let passages = _texts.as_ref().unwrap();
                let mut text = String::new();
                for (i, p) in passages.iter().enumerate() {
                    text.push_str(&format!("[Passage {}]\n{}\n\n", i + 1, p));
                }
                (
                    format!(
                        "You are synthesizing knowledge across {} passages.\n\nGenerate question-answer pairs at these Bloom's taxonomy levels: {levels_str}.\n\nThe questions should require synthesizing information from MULTIPLE passages — compare, contrast, diagnose patterns, or trace causal connections across sources.\n\nFor each QA, cite which passages support the answer (e.g., 'Per Passage 1, ... while Passage 2 notes ...').\n\nPassages (chunk group {chunk_id}):\n{text}\n\nRespond in JSON: {{\"qa_pairs\": [{{\"question\": \"...\", \"answer\": \"...\", \"bloom_level\": \"...\", \"sources\": [1, 3]}}]}}",
                        passages.len()
                    ),
                    "inline-cross-reference",
                )
            } else {
                let mut vars: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
                vars.insert("levels", levels_str.clone());
                vars.insert("chunk_id", chunk_id.clone());
                vars.insert("text", single_text.clone());
                let tpl = render_docproc_template("generate-qa", &vars);
                if tpl.is_empty() {
                    (
                        format!(
                            "Based on the following text, generate question-answer pairs at these Bloom's taxonomy levels: {levels_str}.\n\nText (chunk {chunk_id}):\n{single_text}\n\nFor each level, provide question, answer, and bloom_level.\nRespond in JSON: {{\"qa_pairs\": [{{\"question\": \"...\", \"answer\": \"...\", \"bloom_level\": \"...\"}}]}}"
                        ),
                        "inline-fallback",
                    )
                } else {
                    (tpl, "registry/templates/docproc/generate-qa.j2")
                }
            };
            let selected_model = configured_qa_model(model);

            let params = LLMParameters {
                temperature: 0.3,
                max_tokens: 4096,
                ..Default::default()
            };

            // P3.1: mandatory input guard — scan prompt before model invocation
            let input_scan = GUARD.scan_input(&prompt);
            if !input_scan.passed {
                let violations: Vec<String> = input_scan.violations.iter()
                    .map(|v| format!("{}: {}", v.scanner, v.description))
                    .collect();
                return Err(McpToolError::invalid_argument(format!(
                    "Input guard rejected prompt: {}", violations.join("; ")
                )));
            }

            match self
                .inference_router
                .generate_with_model(&prompt, &params, selected_model.as_deref(), None)
                .await
            {
                Ok(response) => {
                    let output_scan = GUARD.scan_output(&response.text);
                    let content = output_scan.output.content(&response.text);
                    if !output_scan.passed {
                        tracing::warn!(
                            target: "cns.guard",
                            violations = ?output_scan.violations.iter().map(|v| &v.scanner).collect::<Vec<_>>(),
                            "Output guard violations in QA generation — content may be sanitized"
                        );
                    }
                    let qa_response = parse_qa_response(
                        &strip_json_fences(content),
                        &levels,
                        is_cross_ref.then(|| _texts.as_ref().map_or(0, Vec::len)),
                    )
                    .map_err(|e| McpToolError::internal(format!("QA response rejected: {e}")))?;
                    let result = json!({
                        "chunk_id": chunk_id,
                        "bloom_levels": levels,
                        "cross_reference": is_cross_ref,
                        "qa_pairs": qa_response.qa_pairs,
                        "provenance": {
                            "generator_model": selected_model.as_deref().unwrap_or("router_default"),
                            "generator_parameters": params,
                            "prompt_template": template_source,
                            "source_chunk_ref": chunk_id,
                        },
                        "tokens_used": response.usage.total_tokens,
                    });
                    self.record_experience("docproc_generate_qa", &chunk_id, "success", result.clone());
                    Ok(result)
                }
                Err(e) => Err(McpToolError::unavailable(format!("QA generation failed: {}", e))),
            }
        })
        .await
    }

    #[tool(
        description = "Batch-generate QA pairs from multiple text chunks. Same pipeline as docproc_generate_qa (Bloom taxonomy, ContentGuard, templates). Uses configurable concurrency for parallel LLM calls."
    )]
    pub async fn docproc_generate_qa_batch(
        &self,
        Parameters(GenerateQaBatchRequest {
            prompts,
            concurrency,
            model,
        }): Parameters<GenerateQaBatchRequest>,
    ) -> String {
        execute_tool(self, "docproc_generate_qa_batch", async {
            if prompts.is_empty() {
                return Err(McpToolError::invalid_argument("prompts must not be empty"));
            }
            let selected_model = configured_qa_model(model);
            let total = prompts.len();

            // Concurrent processing with configurable semaphore
            let sem = Arc::new(tokio::sync::Semaphore::new(concurrency.max(1)));
            let router = Arc::clone(&self.inference_router);
            let results: Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
                Arc::new(std::sync::Mutex::new(Vec::with_capacity(total)));

            let mut handles = Vec::with_capacity(total);
            for prompt in prompts {
                let router = Arc::clone(&router);
                let sem = Arc::clone(&sem);
                let results = Arc::clone(&results);
                let selected_model = selected_model.clone();

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await;

                    let levels = prompt.bloom_levels.clone().unwrap_or_else(|| vec!["factual".to_string(), "conceptual".to_string()]);
                    let levels_str = levels.join(", ");
                    let mut vars: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
                    vars.insert("levels", levels_str.clone());
                    vars.insert("chunk_id", prompt.chunk_id.clone());
                    vars.insert("text", prompt.text.clone());
                    let tpl = render_docproc_template("generate-qa", &vars);
                    let (prompt_text, template_source) = if tpl.is_empty() {
                        (
                            format!("Based on the following text, generate question-answer pairs at these Bloom's taxonomy levels: {levels_str}.\n\nText (chunk {chunk_id}):\n{text}\n\nFor each level, provide question, answer, and bloom_level.\nRespond in JSON: {{\"qa_pairs\": [{{\"question\": \"...\", \"answer\": \"...\", \"bloom_level\": \"...\"}}]}}",
                                levels_str = levels_str,
                                chunk_id = prompt.chunk_id,
                                text = prompt.text
                            ),
                            "inline-fallback",
                        )
                    } else {
                        (tpl, "registry/templates/docproc/generate-qa.j2")
                    };
                    let input_scan = GUARD.scan_input(&prompt_text);
                    if !input_scan.passed {
                        let mut results = results.lock().unwrap();
                        results.push(json!({"chunk_id": prompt.chunk_id, "error": "Input guard rejected"}));
                        return;
                    }
                    let params = LLMParameters { temperature: 0.3, max_tokens: 4096, ..Default::default() };
                    match router
                        .generate_with_model(&prompt_text, &params, selected_model.as_deref(), None)
                        .await
                    {
                        Ok(response) => {
                            let output_scan = GUARD.scan_output(&response.text);
                            let content = output_scan.output.content(&response.text);
                            match parse_qa_response(&strip_json_fences(content), &levels, None) {
                                Ok(qa_response) => {
                                    let mut results = results.lock().unwrap();
                                    results.push(json!({
                                        "chunk_id": prompt.chunk_id,
                                        "bloom_levels": levels,
                                        "qa_pairs": qa_response.qa_pairs,
                                        "provenance": {
                                            "generator_model": selected_model.as_deref().unwrap_or("router_default"),
                                            "generator_parameters": params,
                                            "prompt_template": template_source,
                                            "source_chunk_ref": prompt.chunk_id,
                                        },
                                        "tokens_used": response.usage.total_tokens,
                                    }));
                                }
                                Err(e) => {
                                    let mut results = results.lock().unwrap();
                                    results.push(json!({
                                        "chunk_id": prompt.chunk_id,
                                        "error": format!("QA response rejected: {e}"),
                                    }));
                                }
                            }
                        }
                        Err(e) => {
                            let mut results = results.lock().unwrap();
                            results.push(json!({"chunk_id": prompt.chunk_id, "error": format!("{}", e)}));
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.await;
            }

            let results = results.lock().unwrap().clone();
            Ok(json!({"total": total, "results": results}))
        }).await
    }

    #[tool(
        description = "Extract RDF h_mems (subject, predicate, object) from text using the inference engine. Returns structured knowledge h_mems with confidence scores."
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
            let mut vars: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
            vars.insert("limit", limit.to_string());
            vars.insert("namespace", ns.clone());
            vars.insert("text", text.clone());
            let prompt = render_docproc_template("extract-h_mems", &vars);
            let prompt = if prompt.is_empty() {
                format!(
                    "Extract up to {limit} factual RDF h_mems from the following text.\n\n\
                     Each h_mem should be in the form (subject, predicate, object) where:\n\
                     - subject: an entity mentioned in the text (prefix with '{ns}:')\n\
                     - predicate: a relationship or property (use standard RDF predicates like rdf:type, schema:name, etc.)\n\n\
                     - object: another entity, a literal value, or a type\n\n\
                     For each h_mem, also provide a confidence score (0.0-1.0) based on how clearly the text supports it.\n\n\
                     Text:\n{text}\n\n\
                     Respond in JSON format: {{\"h_mems\": [{{\"subject\": \"...\", \"predicate\": \"...\", \"object\": \"...\", \"confidence\": 0.95}}]}}"
                )
            } else {
                prompt
            };

            let params = LLMParameters {
                temperature: 0.1,
                max_tokens: 4096,
                ..Default::default()
            };

            // P3.1: mandatory input guard — scan prompt before model invocation
            let input_scan = GUARD.scan_input(&prompt);
            if !input_scan.passed {
                let violations: Vec<String> = input_scan.violations.iter()
                    .map(|v| format!("{}: {}", v.scanner, v.description))
                    .collect();
                return Err(McpToolError::invalid_argument(format!(
                    "Input guard rejected h_mem extraction prompt: {}", violations.join("; ")
                )));
            }

            match self.inference_router.generate(&prompt, &params, None).await {
                Ok(response) => {
                    // P3.1: mandatory output guard — scan model output before processing
                    let output_scan = GUARD.scan_output(&response.text);
                    let content = output_scan.output.content(&response.text);
                    if !output_scan.passed {
                        tracing::warn!(
                            target: "cns.guard",
                            violations = ?output_scan.violations.iter().map(|v| &v.scanner).collect::<Vec<_>>(),
                            "Output guard violations in h_mem extraction — content may be sanitized"
                        );
                    }
                    let cleaned = strip_json_fences(content);
                    let h_mems: serde_json::Value = match serde_json::from_str(&cleaned) {
                        Ok(v) => v,
                        Err(_) => {
                            json!({"raw_response": response.text, "parse_error": "LLM response was not valid JSON"})
                        }
                    };

                    let result = json!({
                        "namespace": ns,
                        "max_triples": limit,
                        "h_mems": h_mems,
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
                    "HMem extraction failed: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(
        description = "Generate embedding vectors for a list of texts (passages or h_mems). Uses the configured embedding model via the inference router. Returns raw vectors as JSON — caller must persist them via corpus-ingest or SemanticMemory::store_embedding()."
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qa_response_rejects_missing_qa_pairs_array() {
        let result = parse_qa_response(
            r#"{"question":"What changed?"}"#,
            &["factual".to_string()],
            None,
        );

        assert!(
            result.is_err(),
            "responses without a qa_pairs array must be rejected"
        );
    }

    #[test]
    fn qa_response_rejects_unrequested_bloom_level() {
        let result = parse_qa_response(
            r#"{"qa_pairs":[{"question":"What changed?","answer":"A result changed.","bloom_level":"evaluate"}]}"#,
            &["factual".to_string()],
            None,
        );

        assert!(result.is_err(), "unrequested Bloom levels must be rejected");
    }

    #[test]
    fn cross_reference_qa_requires_valid_citations() {
        let result = parse_qa_response(
            r#"{"qa_pairs":[{"question":"How do they differ?","answer":"They differ.","bloom_level":"analyze","sources":[3]}]}"#,
            &["analyze".to_string()],
            Some(2),
        );

        assert!(
            result.is_err(),
            "citations outside the supplied passages must be rejected"
        );
    }

    #[test]
    fn qa_response_preserves_valid_pairs() {
        let parsed = parse_qa_response(
            r#"{"qa_pairs":[{"question":"What changed?","answer":"A result changed.","bloom_level":"factual","sources":[1]}]}"#,
            &["factual".to_string()],
            Some(1),
        )
        .expect("valid QA output should be accepted");

        assert_eq!(parsed.qa_pairs.len(), 1);
        assert_eq!(parsed.qa_pairs[0].sources.as_deref(), Some(&[1][..]));
    }

    #[test]
    fn requested_model_overrides_environment_default() {
        let model = configured_qa_model(Some("OR/openai/gpt-5.6-terra".to_string()));
        assert_eq!(model.as_deref(), Some("OR/openai/gpt-5.6-terra"));
    }
}
