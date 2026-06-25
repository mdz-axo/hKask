//! Document processing tools — convert, OCR, chunk.
use crate::*;

#[tool_router(router = document_router, vis = "pub")]
impl DocProcServer {
    #[tool(
        description = "Extract text from a document. Detects format, extracts text with automatic OCR fallback for scanned/image-based PDFs. For PDF: tries text extraction first, falls back to vision OCR if result is near-empty. For other supported formats (TXT, MD, HTML): extracts plain text. Requires HKASK_OCR_MODEL for OCR fallback."
    )]
    pub async fn docproc_convert(
        &self,
        Parameters(ConvertRequest { path, force_ocr }): Parameters<ConvertRequest>,
    ) -> String {
        execute_tool(self, "docproc_convert", async {
            let path_clone = path.clone();
            hkask_mcp::validate_identifier("path", &path, 4096)
                .map_err(|e| McpToolError::new(e.kind, e.to_json_string()))?;

            let (format, _, _) = convert::detect_format(&path);

            // Read the file
            let file_bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    return Err(McpToolError::internal(format!(
                        "Failed to read file '{}': {}",
                        path, e
                    )));
                }
            };

            if file_bytes.is_empty() {
                return Err(McpToolError::invalid_argument(format!(
                    "File '{}' is empty",
                    path
                )));
            }

            // When force_ocr is set, skip text extraction entirely.
            if force_ocr {
                if let Ok(image) = image::load_from_memory(&file_bytes) {
                    let model = match self.resolve_ocr_model(None).await {
                        Ok(m) => m,
                        Err(guidance) => {
                            return Err(McpToolError::failed_precondition(guidance));
                        }
                    };

                    let (text, word_count, outcome) = self.run_ocr_pipeline(vec![image], &model).await;
                    let result = serde_json::json!({
                        "format": format, "path": path, "method": "ocr_pipeline",
                        "model": model, "text": text, "word_count": word_count,
                        "verification_passed": outcome.report.passed,
                        "page_count_match": outcome.report.page_count_match,
                        "empty_pages": outcome.report.empty_pages,
                        "error_count": outcome.errors.len(),
                    });
                    self.record_experience("docproc_convert", &path_clone, "success", result.clone());
                    return Ok(result);
                }

                // Not an image — try decimation + pipeline for PDFs
                if format == "pdf" {
                    match decimation::pdf_to_images(std::path::Path::new(&path), 200).await {
                        Ok(page_images) => {
                            let model = match self.resolve_ocr_model(None).await {
                                Ok(m) => m,
                                Err(guidance) => {
                                    return Err(McpToolError::failed_precondition(guidance));
                                }
                            };
                            let expected = page_images.len();
                            let emb = self.embedding_router.as_ref().map(|r| {
                                (r, default_embedding_model())
                            });
                            let outcome = pipeline::run_pipeline(
                                page_images,
                                expected,
                                self,
                                &self.ocr_thresholds,
                                Some(&model),
                                emb,
                            )
                            .await;
                            self.persist_pipeline_outcome(&outcome).await;
                            let text = outcome
                                .results
                                .iter()
                                .map(|r| r.text.as_str())
                                .collect::<Vec<_>>()
                                .join("\n\n");
                            let result = serde_json::json!({
                                "format": format, "path": path, "method": "ocr_pipeline",
                                "model": model, "text": text,
                                "word_count": text.split_whitespace().count(),
                                "pages": expected,
                                "verification_passed": outcome.report.passed,
                                "page_count_match": outcome.report.page_count_match,
                                "empty_pages": outcome.report.empty_pages,
                                "error_count": outcome.errors.len(),
                            });
                            self.record_experience(
                                "docproc_convert",
                                &path_clone,
                                "success",
                                result.clone(),
                            );
                            return Ok(result);
                        }
                        Err(_) => {
                            // Decimation failed — fall through to do_ocr
                        }
                    }
                }

                // Final fallback: raw bytes OCR
                match self.resolve_ocr_model(None).await {
                    Ok(model) => match self
                        .do_ocr(&file_bytes, &model, default_ocr_max_tokens())
                        .await
                    {
                        Ok(text) => {
                            let result = serde_json::json!({
                                "format": format,
                                "path": path,
                                "method": "ocr",
                                "model": model,
                                "text": text,
                                "word_count": text.split_whitespace().count(),
                            });
                            self.record_experience(
                                "docproc_convert",
                                &path_clone,
                                "success",
                                result.clone(),
                            );
                            return Ok(result);
                        }
                        Err(e) => {
                            return Err(McpToolError::unavailable(e));
                        }
                    },
                    Err(guidance) => {
                        return Err(McpToolError::failed_precondition(guidance));
                    }
                }
            }

            // ── Text extraction path ──
            // GAP-10/C6: Try fast text extraction first for PDFs before the expensive
            // typed OCR pipeline. For text-native PDFs (searchable, well-formed),
            // this returns in ~50ms instead of ~45s for a 300-page document.
            // Only fall back to the pipeline when text extraction is insufficient.
            //
            // `pdf_extract_result` caches the first extraction to avoid calling
            // extract_text() twice on the slow path (B1 audit fix).
            let mut pdf_extract_result: Option<ExtractOutcome> = None;
            if format == "pdf" {
                let quick_result = extract_text(&path).await?;
                if let ExtractOutcome::Success { ref text, word_count } = quick_result
                    && word_count >= OCR_FALLBACK_WORD_THRESHOLD
                {
                    let result = serde_json::json!({
                        "format": format, "path": path,
                        "method": "text_extraction", "text": text, "word_count": word_count,
                    });
                    self.record_experience("docproc_convert", &path_clone, "success", result.clone());
                    return Ok(result);
                }

                // Insufficient text — try the typed OCR pipeline
                if self.has_ocr()
                    && let Ok(model) = self.resolve_ocr_model(None).await
                    && let Ok(page_images) =
                        decimation::pdf_to_images(std::path::Path::new(&path), 200).await
                {
                        let expected = page_images.len();
                        let emb = self.embedding_router.as_ref().map(|r| {
                            (r, default_embedding_model())
                        });
                        let outcome = pipeline::run_pipeline(
                            page_images, expected, self, &self.ocr_thresholds, Some(&model), emb,
                        ).await;
                        self.persist_pipeline_outcome(&outcome).await;
                        let text = outcome.results.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join("\n\n");
                        let word_count = text.split_whitespace().count();
                        let result = serde_json::json!({
                            "format": format, "path": path, "method": "ocr_pipeline",
                            "model": model, "text": text, "word_count": word_count,
                            "pages": expected,
                            "verification_passed": outcome.report.passed,
                            "page_count_match": outcome.report.page_count_match,
                            "empty_pages": outcome.report.empty_pages,
                            "error_count": outcome.errors.len(),
                            "cross_validations": outcome.cross_validations.len(),
                        });
                    self.record_experience("docproc_convert", &path_clone, "success", result.clone());
                    return Ok(result);
                }

                // Pipeline unavailable or failed — reuse the cached extraction result
                pdf_extract_result = Some(quick_result);
            }

            let extract_result = if let Some(cached) = pdf_extract_result {
                cached
            } else {
                extract_text(&path).await?
            };

            match extract_result {
                ExtractOutcome::Success { text, word_count } => {
                    let result = serde_json::json!({
                        "format": format,
                        "path": path,
                        "method": "text_extraction",
                        "text": text,
                        "word_count": word_count,
                    });
                    self.record_experience("docproc_convert", &path_clone, "success", result.clone());
                    Ok(result)
                }
                ExtractOutcome::NeedsOcr {
                    partial_text,
                    word_count,
                } => {
                    // Fall back to OCR — re-read file bytes for do_ocr
                    let file_bytes = std::fs::read(&path).map_err(|e| {
                        McpToolError::internal(format!("Failed to read file '{}' for OCR: {}", path, e))
                    })?;
                    match self.resolve_ocr_model(None).await {
                        Ok(model) => {
                            match self
                                .do_ocr(&file_bytes, &model, default_ocr_max_tokens())
                                .await
                            {
                                Ok(ocr_text) => {
                                    let ocr_word_count = ocr_text.split_whitespace().count();
                                    let (final_text, final_word_count, method) =
                                        if ocr_word_count > word_count {
                                            (ocr_text, ocr_word_count, "ocr")
                                        } else {
                                            (
                                                partial_text,
                                                word_count,
                                                "text_extraction_ocr_fallback_insufficient",
                                            )
                                        };
                                    let result = serde_json::json!({
                                        "format": format,
                                        "path": path,
                                        "method": method,
                                        "model": model,
                                        "text": final_text,
                                        "word_count": final_word_count,
                                        "extraction_word_count": word_count,
                                    });
                                    self.record_experience(
                                        "docproc_convert",
                                        &path_clone,
                                        "success",
                                        result.clone(),
                                    );
                                    Ok(result)
                                }
                                Err(e) => {
                                    if word_count > 0 {
                                        Ok(serde_json::json!({
                                            "format": format,
                                            "path": path,
                                            "method": "text_extraction_ocr_failed",
                                            "text": partial_text,
                                            "word_count": word_count,
                                            "ocr_error": e,
                                        }))
                                    } else {
                                        Err(McpToolError::unavailable(format!(
                                            "Text extraction returned near-empty result and OCR failed: {}",
                                            e
                                        )))
                                    }
                                }
                            }
                        }
                        Err(guidance) => {
                            if word_count > 0 {
                                Ok(serde_json::json!({
                                    "format": format,
                                    "path": path,
                                    "method": "text_extraction_no_ocr_available",
                                    "text": partial_text,
                                    "word_count": word_count,
                                    "ocr_available": false,
                                    "ocr_guidance": guidance,
                                }))
                            } else {
                                Err(McpToolError::failed_precondition(format!(
                                    "PDF text extraction returned no text and no OCR model is configured. {}",
                                    guidance
                                )))
                            }
                        }
                    }
                }
            }
        })
        .await
    }

    #[tool(
        description = "OCR a document using a local vision model. Requires HKASK_OCR_MODEL env var or explicit model parameter. The model must be a vision-capable model available in the inference catalog."
    )]
    pub async fn docproc_ocr(
        &self,
        Parameters(OcrRequest {
            path,
            model,
            max_tokens,
        }): Parameters<OcrRequest>,
    ) -> String {
        execute_tool(self, "docproc_ocr", async {
            let path_clone = path.clone();
            hkask_mcp::validate_identifier("path", &path, 4096)
                .map_err(|e| McpToolError::new(e.kind, e.to_json_string()))?;

            let model = match self.resolve_ocr_model(model.as_deref()).await {
                Ok(m) => m,
                Err(guidance) => {
                    return Err(McpToolError::failed_precondition(guidance));
                }
            };

            let file_bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    return Err(McpToolError::internal(format!(
                        "Failed to read file '{}': {}",
                        path, e
                    )));
                }
            };

            match self.do_ocr(&file_bytes, &model, max_tokens).await {
                Ok(text) => {
                    let result = serde_json::json!({
                        "path": path,
                        "model": model,
                        "text": text,
                        "word_count": text.split_whitespace().count(),
                    });
                    self.record_experience("docproc_ocr", &path_clone, "success", result.clone());
                    Ok(result)
                }
                Err(e) => Err(McpToolError::unavailable(e)),
            }
        })
        .await
    }

    #[tool(
        description = "Chunk text into passages at configurable token granularity. Accepts raw text or a file path (extracts text from PDF/MD/HTML/TXT with OCR fallback for scanned PDFs). Supports single-tier or multi-tier (coarse/medium/fine) output."
    )]
    pub async fn docproc_chunk(
        &self,
        Parameters(ChunkRequest {
            text,
            path,
            entity_ref_prefix,
            max_tokens,
            overlap_tokens,
            strip_gutenberg,
            multi_tier,
            coarse_max_tokens,
            medium_max_tokens,
            fine_max_tokens,
            index,
        }): Parameters<ChunkRequest>,
    ) -> String {
        execute_tool(self, "docproc_chunk", async {
            // Exactly one of text or path must be provided
            let has_text = text.as_ref().is_some_and(|t| !t.is_empty());
            let has_path = path.as_ref().is_some_and(|p| !p.is_empty());
            if has_text == has_path {
                return Err(McpToolError::invalid_argument(
                    "Exactly one of 'text' or 'path' must be provided",
                ));
            }

            if entity_ref_prefix.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "entity_ref_prefix must not be empty",
                ));
            }
            hkask_mcp::validate_identifier("entity_ref_prefix", &entity_ref_prefix, 256)
                .map_err(|e| McpToolError::new(e.kind, e.to_json_string()))?;

            // Resolve the source text
            let source_text: String;
            let source_label: String;

            if let Some(ref raw_text) = text
                && !raw_text.is_empty()
            {
                source_text = raw_text.clone();
                source_label = entity_ref_prefix.clone();
            } else if let Some(ref file_path) = path
                && !file_path.is_empty()
            {
                // Use shared extract_text for format detection + text extraction
                match extract_text(file_path).await? {
                    ExtractOutcome::Success {
                        text: extracted, ..
                    } => {
                        source_text = extracted;
                    }
                    ExtractOutcome::NeedsOcr {
                        partial_text,
                        word_count: _,
                    } => {
                        // Try OCR fallback; use partial_text if OCR unavailable/fails
                        if let Ok(model) = self.resolve_ocr_model(None).await {
                            let file_bytes = std::fs::read(file_path).map_err(|e| {
                                McpToolError::internal(format!(
                                    "Failed to read '{}': {}",
                                    file_path, e
                                ))
                            })?;
                            match self
                                .do_ocr(&file_bytes, &model, default_ocr_max_tokens())
                                .await
                            {
                                Ok(ocr_text) if !ocr_text.is_empty() => {
                                    source_text = ocr_text;
                                }
                                _ => {
                                    source_text = partial_text;
                                }
                            }
                        } else {
                            source_text = partial_text;
                        }
                    }
                }
                source_label = file_path.replace(['/', '\\', '.', ' '], "_");
            } else {
                return Err(McpToolError::invalid_argument("No text or path provided"));
            }

            // Apply Gutenberg stripping if requested
            let processed = if strip_gutenberg.unwrap_or(false) {
                SemanticMemory::strip_gutenberg_headers(&source_text)
            } else {
                source_text
            };

            let boundary = ".!? ";

            if multi_tier.unwrap_or(false) {
                // Multi-tier: coarse / medium / fine
                let chunk_tier = |tier: &str, max_tok: Option<usize>, default: usize| -> Vec<_> {
                    let w = tokens_to_words(max_tok.unwrap_or(default));
                    SemanticMemory::chunk_text(
                        &processed,
                        &format!("{source_label}:{tier}"),
                        w / 4,
                        w,
                        boundary,
                    )
                };

                let coarse = chunk_tier("coarse", coarse_max_tokens, 2048);
                let medium = chunk_tier("medium", medium_max_tokens, 512);
                let fine = chunk_tier("fine", fine_max_tokens, 128);

                let result = json!({
                    "source": source_label,
                    "multi_tier": true,
                    "coarse_max_tokens": coarse_max_tokens.unwrap_or(2048),
                    "medium_max_tokens": medium_max_tokens.unwrap_or(512),
                    "fine_max_tokens": fine_max_tokens.unwrap_or(128),
                    "coarse": serialize_passages(&coarse),
                    "medium": serialize_passages(&medium),
                    "fine": serialize_passages(&fine),
                });

                // Auto-index if requested
                let indexed = if index {
                    let all: Vec<_> = coarse.into_iter().chain(medium).chain(fine).collect();
                    self.index_passages(&all, &source_label).await
                } else {
                    0
                };

                let mut result = result;
                result["indexed"] = json!(indexed);
                self.record_experience("docproc_chunk", &source_label, "success", result.clone());
                Ok(result)
            } else {
                // Single-tier
                let (max_words, min_words) = chunk_word_bounds(max_tokens, overlap_tokens);

                let passages = SemanticMemory::chunk_text(
                    &processed,
                    &entity_ref_prefix,
                    min_words,
                    max_words,
                    boundary,
                );

                let total_passages = passages.len();
                let serialized = serialize_passages(&passages);

                // Auto-index if requested
                let indexed = if index {
                    self.index_passages(&passages, &source_label).await
                } else {
                    0
                };

                let result = json!({
                    "source": source_label,
                    "multi_tier": false,
                    "total_passages": total_passages,
                    "passages": serialized,
                    "max_tokens": max_tokens.unwrap_or(512),
                    "overlap_tokens": overlap_tokens.unwrap_or(64),
                    "max_words": max_words,
                    "min_words": min_words,
                    "sentence_boundary": boundary,
                    "stripped_gutenberg": strip_gutenberg.unwrap_or(false),
                    "indexed": indexed,
                });
                self.record_experience("docproc_chunk", &source_label, "success", result.clone());
                Ok(result)
            }
        })
        .await
    }
}
