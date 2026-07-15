//! Semantic extraction tools — QA generation, h_mem extraction, embedding.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::Write;

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

/// Typed errors for QA response parsing.
#[derive(Debug, Clone, thiserror::Error)]
pub enum QaParseError {
    #[error("QA response must be JSON with a qa_pairs array: {0}")]
    InvalidJson(String),
    #[error("QA response must contain at least one QA pair")]
    Empty,
    #[error("QA pair {index} must have non-empty question and answer")]
    EmptyField { index: usize },
    #[error("QA pair {index} has unsupported Bloom level '{level}'")]
    InvalidBloomLevel { index: usize, level: String },
    #[error("cross-reference QA pair {index} must cite at least one passage")]
    MissingCitation { index: usize },
    #[error("cross-reference QA pair {index} cites a passage outside 1..={passage_count}")]
    InvalidCitation { index: usize, passage_count: usize },
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
) -> Result<QaGenerationResponse, QaParseError> {
    let parsed: QaGenerationResponse =
        serde_json::from_str(response).map_err(|e| QaParseError::InvalidJson(e.to_string()))?;

    if parsed.qa_pairs.is_empty() {
        return Err(QaParseError::Empty);
    }

    for (index, pair) in parsed.qa_pairs.iter().enumerate() {
        if pair.question.trim().is_empty() || pair.answer.trim().is_empty() {
            return Err(QaParseError::EmptyField { index });
        }
        if !requested_levels
            .iter()
            .any(|level| level == &pair.bloom_level)
        {
            return Err(QaParseError::InvalidBloomLevel {
                index,
                level: pair.bloom_level.clone(),
            });
        }
        if let Some(passage_count) = cross_reference_passage_count {
            let sources = pair
                .sources
                .as_ref()
                .filter(|sources| !sources.is_empty())
                .ok_or(QaParseError::MissingCitation { index })?;
            if sources
                .iter()
                .any(|source| *source == 0 || *source > passage_count)
            {
                return Err(QaParseError::InvalidCitation {
                    index,
                    passage_count,
                });
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

/// Map an RDF predicate to a 5W1H dimension.
///
/// Migrated from the CLI binary's `predicate_to_dimension` function.
/// Used by `docproc_extract_triples` to assign a Dimension to each stored h_mem.
pub(crate) fn predicate_to_dimension(predicate: &str) -> hkask_types::Dimension {
    let p = predicate.to_lowercase();
    if p.contains("type")
        || p.contains("is_a")
        || p.contains("subclass")
        || p.contains("name")
        || p.contains("label")
        || p.contains("title")
    {
        hkask_types::Dimension::What
    } else if p.contains("location") || p.contains("place") || p.contains("located_in") {
        hkask_types::Dimension::Where
    } else if p.contains("time")
        || p.contains("date")
        || p.contains("when")
        || p.contains("created")
    {
        hkask_types::Dimension::When
    } else if p.contains("person")
        || p.contains("author")
        || p.contains("creator")
        || p.contains("actor")
    {
        hkask_types::Dimension::Who
    } else if p.contains("cause")
        || p.contains("reason")
        || p.contains("why")
        || p.contains("motivation")
    {
        hkask_types::Dimension::Why
    } else if p.contains("method")
        || p.contains("process")
        || p.contains("how")
        || p.contains("uses")
        || p.contains("uses_method")
    {
        hkask_types::Dimension::How
    } else {
        hkask_types::Dimension::What
    }
}

/// Write a QA batch result as one JSONL line to the output file with
/// incremental flush every 10 completions for crash safety.
fn write_qa_result(
    result: &serde_json::Value,
    output_writer: &Arc<Mutex<std::io::BufWriter<std::fs::File>>>,
    write_count: &std::sync::atomic::AtomicUsize,
) {
    let mut w = output_writer.lock().unwrap();
    let _ = serde_json::to_writer(&mut *w, result);
    let _ = writeln!(&mut *w);
    let count = write_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    if count.is_multiple_of(10) {
        let _ = w.flush();
    }
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
                top_p: 0.95,
                max_tokens: 4096,
                frequency_penalty: 0.0,
                presence_penalty: 0.0,
                top_k: 0,
                min_p: 0.0,
                typical_p: 0.0,
                disable_thinking: true,
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
                        &extract_json_from_response(content),
                        &levels,
                        is_cross_ref.then(|| _texts.as_ref().map_or(0, Vec::len)),
                    )
                    .map_err(|e| McpToolError::internal(e.to_string()))?;
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
        description = "Batch-generate QA pairs from multiple text chunks. Same pipeline as docproc_generate_qa (Bloom taxonomy, ContentGuard, templates). Uses configurable concurrency for parallel LLM calls. Reads prompts from prompts_jsonl (one JSON per line: chunk_ref, qa_type, system, user) and writes generated QAs to the output JSONL file. Returns a summary (total + written counts)."
    )]
    pub async fn docproc_generate_qa_batch(
        &self,
        Parameters(GenerateQaBatchRequest {
            prompts_jsonl,
            output,
            concurrency,
            model,
        }): Parameters<GenerateQaBatchRequest>,
    ) -> String {
        execute_tool(self, "docproc_generate_qa_batch", async {
            // Read prompts from JSONL file (file-only mode)
            let content = std::fs::read_to_string(&prompts_jsonl).map_err(|e| {
                McpToolError::invalid_argument(format!(
                    "Cannot read prompts_jsonl '{}': {e}",
                    prompts_jsonl
                ))
            })?;
            let mut prompts_vec: Vec<BatchQaPrompt> = Vec::new();
            for (i, line) in content.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let v: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                    McpToolError::invalid_argument(format!(
                        "prompts_jsonl line {} is not valid JSON: {e}",
                        i + 1
                    ))
                })?;
                // Map build_prompts output fields to BatchQaPrompt:
                // chunk_ref -> chunk_id, system+user -> text, qa_type -> bloom_levels
                let chunk_id = v
                    .get("chunk_ref")
                    .and_then(|v| v.as_str())
                    .or_else(|| v.get("chunk_id").and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .to_string();
                let system = v.get("system").and_then(|v| v.as_str()).unwrap_or("");
                let user = v.get("user").and_then(|v| v.as_str()).unwrap_or("");
                let text = if system.is_empty() && user.is_empty() {
                    v.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string()
                } else {
                    format!("{system}\n\n{user}")
                };
                let bloom_levels = v
                    .get("qa_type")
                    .and_then(|v| v.as_str())
                    .map(|qt| vec![qt.to_string()])
                    .or_else(|| {
                        v.get("bloom_levels").and_then(|v| v.as_array()).map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect()
                        })
                    });
                let source = v
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let concepts = v
                    .get("concepts")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                prompts_vec.push(BatchQaPrompt {
                    text,
                    chunk_id,
                    bloom_levels,
                    source,
                    concepts,
                });
            }

            if prompts_vec.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "prompts_jsonl contains no valid prompts",
                ));
            }

            let selected_model = configured_qa_model(model);
            let total = prompts_vec.len();

            // Concurrent processing with configurable semaphore
            let sem = Arc::new(tokio::sync::Semaphore::new(concurrency.max(1)));
            let router = Arc::clone(&self.inference_router);

            // Output file writer (with incremental flush every 10 completions)
            let file = std::fs::File::create(&output).map_err(|e| {
                McpToolError::internal(format!(
                    "Cannot create output file '{}': {e}",
                    output
                ))
            })?;
            let output_writer = Arc::new(Mutex::new(std::io::BufWriter::new(file)));
            let write_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

            let mut handles = Vec::with_capacity(total);
            for prompt in prompts_vec {
                let router = Arc::clone(&router);
                let sem = Arc::clone(&sem);
                let selected_model = selected_model.clone();
                let output_writer = Arc::clone(&output_writer);
                let write_count = Arc::clone(&write_count);

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
                        let result = json!({"chunk_id": prompt.chunk_id, "error": "Input guard rejected"});
                        write_qa_result(&result, &output_writer, &write_count);
                        return;
                    }
                    let params = LLMParameters { temperature: 0.3, top_p: 0.95, max_tokens: 4096, frequency_penalty: 0.0, presence_penalty: 0.0, top_k: 0, min_p: 0.0, typical_p: 0.0, disable_thinking: true, ..Default::default() };
                    match router
                        .generate_with_model(&prompt_text, &params, selected_model.as_deref(), None)
                        .await
                    {
                        Ok(response) => {
                            let output_scan = GUARD.scan_output(&response.text);
                            let content = output_scan.output.content(&response.text);
                            match parse_qa_response(&extract_json_from_response(content), &levels, None) {
                                Ok(qa_response) => {
                                    // Write one JSONL line per QA pair in envelope format
                                    // (matches what docproc_ingest_qa's parse_qa_record expects)
                                    for pair in qa_response.qa_pairs {
                                        let result = json!({
                                            "chunk_ref": prompt.chunk_id,
                                            "source": prompt.source,
                                            "qa_type": pair.bloom_level,
                                            "response": {
                                                "instruction": pair.question,
                                                "output": pair.answer,
                                                "type": pair.bloom_level,
                                                "concepts": prompt.concepts,
                                            },
                                            "provenance": {
                                                "generator_model": selected_model.as_deref().unwrap_or("router_default"),
                                                "prompt_template": template_source,
                                                "source_chunk_ref": prompt.chunk_id,
                                            },
                                            "tokens_used": response.usage.total_tokens,
                                        });
                                        write_qa_result(&result, &output_writer, &write_count);
                                    }
                                }
                                Err(e) => {
                                    let result = json!({
                                        "chunk_id": prompt.chunk_id,
                                        "error": format!("QA response rejected: {e}"),
                                    });
                                    write_qa_result(&result, &output_writer, &write_count);
                                }
                            }
                        }
                        Err(e) => {
                            let result = json!({"chunk_id": prompt.chunk_id, "error": format!("{}", e)});
                            write_qa_result(&result, &output_writer, &write_count);
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.await;
            }

            {
                let mut w = output_writer.lock().unwrap();
                let _ = w.flush();
            }
            let written = write_count.load(std::sync::atomic::Ordering::Relaxed);
            let result = json!({
                "total": total,
                "written": written,
                "output": output,
            });
            self.record_experience(
                "docproc_generate_qa_batch",
                &format!("batch: {} prompts", total),
                "success",
                result.clone(),
            );
            Ok(result)
        }).await
    }

    #[tool(
        description = "Extract RDF h_mems (subject, predicate, object) from text using the inference engine. Uses the classifier model (Qwen3.6-35B-A3B) with 3-attempt retry. Reads chunks from chunks_jsonl, processes them concurrently, and stores triples as h_mems in the memory DB with entity=entity_ref from each chunk. Returns a summary (total_chunks, succeeded, failed, h_mems_stored)."
    )]
    pub async fn docproc_extract_triples(
        &self,
        Parameters(ExtractTriplesRequest {
            chunks_jsonl,
            db_path,
            passphrase,
            max_triples,
            owner,
            concurrency,
        }): Parameters<ExtractTriplesRequest>,
    ) -> String {
        execute_tool(self, "docproc_extract_triples", async {
            self.extract_triples_batch(
                &chunks_jsonl,
                max_triples,
                &db_path,
                &passphrase,
                &owner,
                concurrency,
            )
            .await
        })
        .await
    }

    /// Batch extract h_mems from chunks JSONL with concurrent LLM calls.
    ///
    /// Opens the DB once and shares it across all concurrent tasks via `Arc<SemanticMemory>`.
    /// Each chunk gets a 3-attempt retry with backoff. Triples are stored as h_mems
    /// with `entity = chunk.entity_ref`.
    #[allow(clippy::too_many_arguments)]
    async fn extract_triples_batch(
        &self,
        chunks_path: &str,
        max_triples: usize,
        db_path: &str,
        passphrase: &str,
        owner: &str,
        concurrency: usize,
    ) -> Result<serde_json::Value, McpToolError> {
        let content = std::fs::read_to_string(chunks_path).map_err(|e| {
            McpToolError::invalid_argument(format!(
                "Cannot read chunks_jsonl '{}': {e}",
                chunks_path
            ))
        })?;

        // Parse chunks: each line has entity_ref and text
        let mut chunks: Vec<(String, String)> = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                McpToolError::invalid_argument(format!(
                    "chunks_jsonl line {} is not valid JSON: {e}",
                    i + 1
                ))
            })?;
            let entity_ref = v
                .get("entity_ref")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let chunk_text = v
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if entity_ref.is_empty() || chunk_text.is_empty() {
                tracing::warn!(
                    target: "hkask.mcp.docproc.triples",
                    line = i + 1,
                    "Skipping chunk with empty entity_ref or text"
                );
                continue;
            }
            chunks.push((entity_ref, chunk_text));
        }

        let total_chunks = chunks.len();
        if total_chunks == 0 {
            return Err(McpToolError::invalid_argument(
                "chunks_jsonl contains no valid chunks",
            ));
        }

        // Open DB once, share across concurrent tasks
        let dim = embedding_dim();
        let semantic = Arc::new(
            hkask_memory::SemanticMemory::open(db_path, passphrase, dim).map_err(|e| {
                McpToolError::failed_precondition(format!("Cannot open memory DB: {e}"))
            })?,
        );
        let webid = owner_webid(owner);
        let classifier = hkask_inference::model_constants::classifier_model();
        // Namespace is fixed to "doc" for corpus chunk extraction (no longer a request field).
        let ns = "doc".to_string();

        let sem = Arc::new(tokio::sync::Semaphore::new(concurrency.max(1)));
        let router = Arc::clone(&self.inference_router);
        let succeeded = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let failed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let h_mems_stored = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut handles = Vec::with_capacity(total_chunks);
        for (entity_ref, chunk_text) in chunks {
            let router = Arc::clone(&router);
            let sem = Arc::clone(&sem);
            let semantic = Arc::clone(&semantic);
            let classifier = classifier.clone();
            let ns = ns.clone();
            let succeeded = Arc::clone(&succeeded);
            let failed = Arc::clone(&failed);
            let h_mems_stored = Arc::clone(&h_mems_stored);

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await;

                // Build prompt from registry template
                let mut vars: std::collections::HashMap<&str, String> =
                    std::collections::HashMap::new();
                vars.insert("limit", max_triples.to_string());
                vars.insert("namespace", ns.clone());
                vars.insert("text", chunk_text.clone());
                let prompt = render_docproc_template("extract-hmems", &vars);
                let prompt = if prompt.is_empty() {
                    format!(
                        "Extract up to {max_triples} factual RDF h_mems from the following text.\n\n\
                         Each h_mem should be in the form (subject, predicate, object) where:\\
                         - subject: an entity mentioned in the text (prefix with '{ns}:')\\
                         - predicate: a relationship or property (use standard RDF predicates like rdf:type, schema:name, etc.)\n\n\
                         - object: another entity, a literal value, or a type\n\n\
                         For each h_mem, also provide a confidence score (0.0-1.0) based on how clearly the text supports it.\n\n\
                         Text:\n{chunk_text}\n\n\
                         Respond in JSON format: {{\"h_mems\": [{{\"subject\": \"...\", \"predicate\": \"...\", \"object\": \"...\", \"confidence\": 0.95}}]}}"
                    )
                } else {
                    prompt
                };

                // Input guard
                let input_scan = GUARD.scan_input(&prompt);
                if !input_scan.passed {
                    tracing::warn!(
                        target: "hkask.mcp.docproc.triples",
                        entity = %entity_ref,
                        "Input guard rejected extraction prompt"
                    );
                    failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return;
                }

                let params = LLMParameters {
                    temperature: 0.1,
                    top_p: 0.95,
                    max_tokens: 4096,
                    frequency_penalty: 0.0,
                    presence_penalty: 0.0,
                    top_k: 0,
                    min_p: 0.0,
                    typical_p: 0.0,
                    disable_thinking: true,
                    ..Default::default()
                };

                // 3-attempt retry with backoff
                let mut attempts = 0u32;
                let response = loop {
                    match router
                        .generate_with_model(&prompt, &params, Some(&classifier), None)
                        .await
                    {
                        Ok(resp) => break resp,
                        Err(e) => {
                            attempts += 1;
                            if attempts >= 3 {
                                tracing::warn!(
                                    target: "hkask.mcp.docproc.triples",
                                    entity = %entity_ref,
                                    error = %e,
                                    "HMem extraction failed after 3 retries"
                                );
                                failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                return;
                            }
                            let backoff = std::time::Duration::from_secs(2u64.pow(attempts) * 5);
                            tracing::warn!(
                                target: "hkask.mcp.docproc.triples",
                                entity = %entity_ref,
                                attempt = attempts,
                                backoff_secs = backoff.as_secs(),
                                error = %e,
                                "HMem extraction retry — backing off"
                            );
                            tokio::time::sleep(backoff).await;
                        }
                    }
                };

                // Output guard + JSON extraction
                let output_scan = GUARD.scan_output(&response.text);
                let content = output_scan.output.content(&response.text);
                if !output_scan.passed {
                    tracing::warn!(
                        target: "cns.guard",
                        entity = %entity_ref,
                        violations = ?output_scan.violations.iter().map(|v| &v.scanner).collect::<Vec<_>>(),
                        "Output guard violations in h_mem extraction — content may be sanitized"
                    );
                }
                let cleaned = extract_json_from_response(content);
                let h_mems: serde_json::Value = match serde_json::from_str(&cleaned) {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!(
                            target: "hkask.mcp.docproc.triples",
                            entity = %entity_ref,
                            "LLM response was not valid JSON"
                        );
                        failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        return;
                    }
                };

                // Store triples as h_mems
                let mut stored = 0usize;
                if let Some(arr) = h_mems.get("h_mems").and_then(|v| v.as_array()) {
                    for triple in arr {
                        let predicate = triple
                            .get("predicate")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let object = triple.get("object").cloned().unwrap_or(json!(null));
                        let confidence = triple
                            .get("confidence")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.8);
                        let dimension = predicate_to_dimension(predicate);
                        let h_mem = hkask_storage::HMem::new(&entity_ref, predicate, object, webid)
                            .with_visibility(hkask_types::Visibility::Public)
                            .with_confidence(confidence)
                            .with_dimension(dimension);
                        match semantic.store(h_mem) {
                            Ok(()) => stored += 1,
                            Err(e) => {
                                tracing::warn!(
                                    target: "hkask.mcp.docproc.triples",
                                    entity = %entity_ref,
                                    error = %e,
                                    "Failed to store triple h_mem"
                                );
                            }
                        }
                    }
                }

                h_mems_stored.fetch_add(stored, std::sync::atomic::Ordering::Relaxed);
                succeeded.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let succeeded = succeeded.load(std::sync::atomic::Ordering::Relaxed);
        let failed = failed.load(std::sync::atomic::Ordering::Relaxed);
        let h_mems_stored = h_mems_stored.load(std::sync::atomic::Ordering::Relaxed);

        let result = json!({
            "total_chunks": total_chunks,
            "succeeded": succeeded,
            "failed": failed,
            "h_mems_stored": h_mems_stored,
        });
        self.record_experience(
            "docproc_extract_triples",
            &format!("batch: {} chunks", total_chunks),
            "success",
            result.clone(),
        );
        Ok(result)
    }

    #[tool(
        description = "Generate embedding vectors for corpus chunks. Uses the configured embedding model via the inference router. Reads chunks from chunks_jsonl (entity_ref, source, text, word_count per line), batch-embeds in groups of batch_size, and stores each vector in the memory DB at db_path. Returns a summary (total, embedded, failed, model) — no inline vectors."
    )]
    pub async fn docproc_embed(
        &self,
        Parameters(EmbedRequest {
            chunks_jsonl,
            db_path,
            passphrase,
            model,
            batch_size,
        }): Parameters<EmbedRequest>,
    ) -> String {
        execute_tool(self, "docproc_embed", async {
            self.embed_batch_from_jsonl(&chunks_jsonl, model, &db_path, &passphrase, batch_size)
                .await
        })
        .await
    }

    /// Batch embed chunks from a JSONL file with configurable batch size.
    ///
    /// Reads chunks (entity_ref, source, text, word_count per line), batch-embeds
    /// in groups of `batch_size`, stores each vector + text/provenance h_mem in the
    /// DB, and returns a summary (no inline vectors — too large for 33K chunks).
    async fn embed_batch_from_jsonl(
        &self,
        chunks_path: &str,
        model: Option<String>,
        db_path: &str,
        passphrase: &str,
        batch_size: usize,
    ) -> Result<serde_json::Value, McpToolError> {
        let Some(ref emb_router) = self.embedding_router else {
            return Err(McpToolError::failed_precondition(
                "Embedding router not configured — inference config may be missing",
            ));
        };

        let content = std::fs::read_to_string(chunks_path).map_err(|e| {
            McpToolError::invalid_argument(format!(
                "Cannot read chunks_jsonl '{}': {e}",
                chunks_path
            ))
        })?;

        // Parse chunks: each line has entity_ref, source, text, word_count
        let mut chunks: Vec<(String, String)> = Vec::new(); // (entity_ref, text)
        for (i, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                McpToolError::invalid_argument(format!(
                    "chunks_jsonl line {} is not valid JSON: {e}",
                    i + 1
                ))
            })?;
            let entity_ref = v
                .get("entity_ref")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let text = v
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if entity_ref.is_empty() || text.is_empty() {
                tracing::warn!(
                    target: "hkask.mcp.docproc.embed",
                    line = i + 1,
                    "Skipping chunk with empty entity_ref or text"
                );
                continue;
            }
            chunks.push((entity_ref, text));
        }

        let total = chunks.len();
        if total == 0 {
            return Err(McpToolError::invalid_argument(
                "chunks_jsonl contains no valid chunks",
            ));
        }

        let model_name = model.unwrap_or_else(|| {
            std::env::var("HKASK_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string())
        });

        let dim = embedding_dim();
        let semantic =
            hkask_memory::SemanticMemory::open(db_path, passphrase, dim).map_err(|e| {
                McpToolError::failed_precondition(format!("Cannot open memory DB: {e}"))
            })?;

        let mut embedded = 0usize;
        let mut failed = 0usize;
        let batch = batch_size.max(1);

        for chunk_batch in chunks.chunks(batch) {
            let batch_texts: Vec<&str> = chunk_batch.iter().map(|c| c.1.as_str()).collect();
            // Retry with backoff (3 attempts) — same pattern as tag_chunks and extract_triples
            let vectors = {
                let mut attempts = 0u32;
                loop {
                    match emb_router.embed_sentences(&model_name, &batch_texts).await {
                        Ok(v) => break v,
                        Err(e) => {
                            attempts += 1;
                            if attempts >= 3 {
                                failed += chunk_batch.len();
                                tracing::warn!(
                                    target: "hkask.mcp.docproc.embed",
                                    batch_size = chunk_batch.len(),
                                    attempts = attempts,
                                    error = %e,
                                    "Batch embedding failed after 3 retries"
                                );
                                break Vec::new();
                            }
                            let backoff = std::time::Duration::from_secs(2u64.pow(attempts) * 5);
                            tracing::warn!(
                                target: "hkask.mcp.docproc.embed",
                                attempt = attempts,
                                backoff_secs = backoff.as_secs(),
                                error = %e,
                                "Embedding retry — backing off"
                            );
                            tokio::time::sleep(backoff).await;
                        }
                    }
                }
            };
            if vectors.is_empty() {
                continue;
            }
            for (c, vector) in chunk_batch.iter().zip(vectors.iter()) {
                // Store embedding vector only — text and provenance h_mems were
                // removed as orphans (no downstream pipeline tool consumed them).
                if let Err(e) = semantic.store_embedding(&c.0, vector, &model_name) {
                    failed += 1;
                    if failed <= 5 {
                        tracing::warn!(
                            target: "hkask.mcp.docproc.embed",
                            entity = %c.0,
                            error = %e,
                            "Failed to store embedding"
                        );
                    }
                    continue;
                }
                embedded += 1;
            }
        }

        let result = json!({
            "total": total,
            "embedded": embedded,
            "failed": failed,
            "model": model_name,
        });
        self.record_experience(
            "docproc_embed",
            &format!("batch: {} chunks", total),
            "success",
            result.clone(),
        );
        Ok(result)
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

// ── Request structs ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateQaRequest {
    /// Single chunk text (mutually exclusive with texts for multi-chunk cross-reference)
    #[serde(default)]
    pub text: Option<String>,
    /// Multiple chunks for cross-reference QA generation (RA-DIT method).
    /// When set, generates QAs that require synthesizing across all passages.
    #[serde(default)]
    pub texts: Option<Vec<String>>,
    pub chunk_id: String,
    #[serde(default)]
    pub bloom_levels: Option<Vec<String>>,
    /// Optional provider-prefixed generation model (for example, `OR/openai/gpt-5.6-terra`).
    /// When absent, uses `HKASK_QA_MODEL`, then `HKASK_DEFAULT_MODEL`.
    #[serde(default)]
    pub model: Option<String>,
}

/// A single prompt spec parsed from prompts_jsonl for batch QA generation.
/// Internal to the batch tool — not part of the public request schema.
#[derive(Debug, Deserialize)]
struct BatchQaPrompt {
    text: String,
    chunk_id: String,
    #[serde(default)]
    bloom_levels: Option<Vec<String>>,
    /// Source file name (from build_prompts output).
    #[serde(default)]
    source: String,
    /// Concepts from the original chunk (from build_prompts output).
    #[serde(default)]
    concepts: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateQaBatchRequest {
    /// Path to prompts JSONL file (one JSON per line with chunk_ref, qa_type, system, user).
    pub prompts_jsonl: String,
    /// Output path for generated QAs JSONL.
    pub output: String,
    /// Max concurrent LLM calls.
    #[serde(default = "default_batch_concurrency")]
    pub concurrency: usize,
    /// Optional provider-prefixed generation model for every prompt in this batch.
    #[serde(default)]
    pub model: Option<String>,
}

fn default_batch_concurrency() -> usize {
    4
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractTriplesRequest {
    /// Path to chunks JSONL for batch processing. Reads (entity_ref, text) per line.
    pub chunks_jsonl: String,
    /// Path to the SQLCipher memory DB for h_mem storage.
    pub db_path: String,
    /// Passphrase for the memory DB.
    #[serde(default = "default_docproc_passphrase")]
    pub passphrase: String,
    /// Maximum h_mems to extract per chunk (default 15).
    #[serde(default = "default_max_triples")]
    pub max_triples: usize,
    /// Owner persona for stored h_mems (e.g. "john-brooks").
    #[serde(default = "default_owner")]
    pub owner: String,
    /// Max concurrent LLM calls for batch processing (default 64).
    #[serde(default = "default_triples_concurrency")]
    pub concurrency: usize,
}

fn default_max_triples() -> usize {
    15
}

fn default_triples_concurrency() -> usize {
    64
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmbedRequest {
    /// Path to chunks JSONL (entity_ref, source, text, word_count per line).
    pub chunks_jsonl: String,
    /// Path to the SQLCipher memory DB for vector storage.
    pub db_path: String,
    /// Passphrase for the memory DB.
    #[serde(default = "default_docproc_passphrase")]
    pub passphrase: String,
    /// Embedding model to use. If not set, uses the configured default.
    #[serde(default)]
    pub model: Option<String>,
    /// Batch size for embedding API calls (default 50).
    #[serde(default = "default_embed_batch_size")]
    pub batch_size: usize,
}

fn default_embed_batch_size() -> usize {
    50
}

/// Default passphrase for the docproc memory DB.
///
/// `tools::storage::default_purge_passphrase` is private to that module, so this
/// module-local default mirrors it for `ExtractTriplesRequest` and `EmbedRequest`.
fn default_docproc_passphrase() -> String {
    "hkask-default-passphrase-2024".to_string()
}
