//! Generate QA pairs from built prompts using the inference router.
//!
//! Processes prompts.jsonl in parallel with configurable concurrency,
//! producing generated.jsonl with provenance-enriched QA entries.

use std::path::PathBuf;
use std::sync::Arc;

use hkask_inference::config::InferenceConfig;
use hkask_inference::inference_router::InferenceRouter;
use hkask_ports::inference_port::InferencePort;
use hkask_types::template::LLMParameters;
use serde::Deserialize;

/// Prompt record as produced by build-prompts.
#[derive(Debug, Deserialize)]
pub struct QaPrompt {
    pub chunk_ref: String,
    pub source: String,
    pub concepts: Vec<String>,
    /// Salience score carried for round-trip fidelity; consumed by downstream
    /// QA metrics, not read directly in the generate-qa path.
    #[allow(dead_code)]
    pub salience: f32,
    pub qa_type: String,
    pub system: String,
    pub user: String,
}

/// Per-Bloom-level LLM sampling parameters.
///
/// Empirically verified against GLM-5.2 via KiloCode (2026-07-13):
///
/// - min_p: SILENTLY IGNORED by ZhipuAI endpoint — not used
/// - disable_thinking (enable_thinking=false): IGNORED — GLM-5.2 always reasons
///   (~640-830 reasoning tokens regardless of parameter or prompt style)
/// - frequency_penalty / presence_penalty: minimal observable effect
/// - temperature + top_p: VERIFIED EFFECTIVE — primary controls
///
/// GLM-5.2 reasoning overhead by Bloom level (measured):
///   factual: ~640 reasoning tokens, ~55 content tokens
///   analyze: ~640 reasoning tokens, ~170 content tokens
///   create: ~830 reasoning tokens, ~300 content tokens
/// All well within max_tokens=4096.
///
/// Cognitive differentiation comes from:
///   1. Prompt instructions (qa_type_instruction) — primary
///   2. Temperature (0.1 → 0.8) — randomness gradient
///   3. top_p (0.85 → 0.98) — token pool width
///
/// Research basis:
/// - Renze (2024, EMNLP): low temp for factual accuracy, high for creativity
/// - Nguyen et al. (2025, ICLR): min-p maintains coherence at high temp
///   (but min-p unsupported on this provider — top_p serves as coherence guard)
/// - Wang & Zhou (2024): moderate temp encourages beneficial reasoning diversity
fn bloom_params(qa_type: &str) -> LLMParameters {
    let (temperature, top_p) = match qa_type {
        "factual"    => (0.1, 0.85),  // extraction: deterministic, focused nucleus
        "conceptual" => (0.3, 0.90),  // explanation: slight flexibility
        "analyze"    => (0.5, 0.95),  // reasoning: moderate exploration
        "evaluate"   => (0.5, 0.95),  // reasoning: same as analyze
        "create"     => (0.8, 0.98),  // divergent: high creativity, wide nucleus
        _            => (0.3, 0.90),
    };
    LLMParameters {
        temperature,
        top_p,
        max_tokens: 4096,
        ..Default::default()
    }
}

/// CLI arguments for the generate-qa subcommand.
#[derive(clap::Parser)]
pub struct GenerateQaArgs {
    /// Path to prompts JSONL (from build-prompts)
    #[arg(default_value = "corpus/qa_pairs/prompts.jsonl")]
    pub prompts_jsonl: PathBuf,
    /// Output: generated QAs (one JSON per line)
    #[arg(short = 'o', long, default_value = "corpus/qa_pairs/generated.jsonl")]
    pub output: PathBuf,
    /// Maximum concurrent LLM calls (default: 5)
    #[arg(short = 'c', long, default_value = "5")]
    pub concurrency: usize,
    /// Maximum prompts to process (0 = all)
    #[arg(short = 'n', long, default_value = "0")]
    pub max_prompts: usize,
    /// Resume from a specific line number (1-indexed)
    #[arg(short = 'r', long, default_value = "1")]
    pub resume_at: usize,
    /// Dry run
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run_generate_qa(args: GenerateQaArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Generate QA ===");
    println!("  Prompts: {}", args.prompts_jsonl.display());
    println!("  Output:  {}", args.output.display());
    println!("  Concurrency: {}", args.concurrency);
    println!();

    let prompts: Vec<QaPrompt> = {
        let content = std::fs::read_to_string(&args.prompts_jsonl)?;
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<QaPrompt>(l).ok())
            .collect()
    };
    println!("  Loaded: {} prompts", prompts.len());

    let range = if args.resume_at > 1 {
        let start = args.resume_at - 1;
        let end = if args.max_prompts > 0 {
            (start + args.max_prompts).min(prompts.len())
        } else {
            prompts.len()
        };
        println!("  Resume: lines {}-{}", start + 1, end);
        &prompts[start..end]
    } else if args.max_prompts > 0 {
        let end = args.max_prompts.min(prompts.len());
        &prompts[..end]
    } else {
        &prompts[..]
    };
    println!("  Processing: {} prompts", range.len());

    if args.dry_run {
        println!("  Dry run. Would process {} prompts.", range.len());
        return Ok(());
    }

    let inf_cfg = InferenceConfig::from_env();
    let router = Arc::new(InferenceRouter::new(inf_cfg));

    let sem = Arc::new(tokio::sync::Semaphore::new(args.concurrency));
    let output_path = args.output.clone();
    let out_mutex = Arc::new(tokio::sync::Mutex::new(Vec::<String>::with_capacity(
        range.len(),
    )));

    let start_time = std::time::Instant::now();
    let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let failed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let failures_for_backoff = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Build a Vec of owned prompt tuples for 'static lifetime in tokio::spawn
    let prompts_vec: Vec<_> = range
        .iter()
        .map(|p| {
            (
                p.chunk_ref.clone(),
                p.source.clone(),
                p.concepts.clone(),
                p.qa_type.clone(),
                p.system.clone(),
                p.user.clone(),
            )
        })
        .collect();
    let prompts_arc = Arc::new(prompts_vec);

    let mut handles = Vec::with_capacity(prompts_arc.len());

    for idx in 0..prompts_arc.len() {
        let router = Arc::clone(&router);
        let sem = Arc::clone(&sem);
        let out_mutex = Arc::clone(&out_mutex);
        let completed = Arc::clone(&completed);
        let failed = Arc::clone(&failed);
        let failures_for_backoff = Arc::clone(&failures_for_backoff);
        let prompts_arc = Arc::clone(&prompts_arc);
        let output_path = output_path.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let (chunk_ref, source, concepts, qa_type, system, user) = &prompts_arc[idx];

            // Truncate passage text to keep prompt size manageable.
            // User prompts embed full chunk text between '---' markers;
            // truncate the passage portion to ~3000 chars.
            let truncated_user = if let Some((before, rest)) = user.split_once("---\n") {
                if let Some((passage, after)) = rest.split_once("\n---") {
                    let max_passage = 3000usize;
                    let short_passage = if passage.len() > max_passage {
                        // Truncate at a valid UTF-8 char boundary
                        let mut end = max_passage;
                        while end > 0 && !passage.is_char_boundary(end) {
                            end -= 1;
                        }
                        &passage[..end]
                    } else {
                        passage
                    };
                    format!("{before}---\n{short_passage}\n---{after}")
                } else {
                    user.clone()
                }
            } else {
                user.clone()
            };
            let combined_prompt = format!("{system}\n\n{truncated_user}");

            let params = bloom_params(qa_type);

            let model_override: Option<String> = std::env::var("HKASK_QA_MODEL").ok();

            let response = loop {
                match router
                    .generate_with_model(&combined_prompt, &params, model_override.as_deref(), None)
                    .await
                {
                    Ok(response) => break Some(response),
                    Err(e) => {
                        let attempt =
                            failures_for_backoff.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let backoff_secs = 2u64.pow((attempt + 1).min(6).try_into().unwrap()) * 10;
                        tracing::warn!(
                            target: "corpus.qa",
                            attempt = attempt + 1,
                            backoff_secs = backoff_secs,
                            error = %e,
                            "LLM call failed — retrying after backoff"
                        );
                        if attempt >= 4 {
                            tracing::error!(
                                target: "corpus.qa",
                                error = %e,
                                "GIVING UP after 5 retries on this prompt"
                            );
                            failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let err_entry = serde_json::json!({
                                "chunk_ref": chunk_ref,
                                "error": format!("{}", e),
                            });
                            {
                                let mut out = out_mutex.lock().await;
                                out.push(serde_json::to_string(&err_entry).unwrap_or_default());
                            }
                            // Skip to next prompt — can't use continue in a loop-break pattern,
                            // so we emit a sentinel and break out.
                            // The response handling below checks for this.
                            break None;
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                    }
                }
            };

            match response {
                Some(response) => {
                    failures_for_backoff.store(0, std::sync::atomic::Ordering::Relaxed);
                    let trimmed = response.text.trim();
                    let cleaned = trimmed
                        .strip_prefix("```json")
                        .or_else(|| trimmed.strip_prefix("```"))
                        .unwrap_or(trimmed);
                    let cleaned = cleaned.strip_suffix("```").unwrap_or(cleaned).trim();

                    let qa_entry = match serde_json::from_str::<serde_json::Value>(cleaned) {
                        Ok(v) => {
                            let enriched = serde_json::json!({
                                "chunk_ref": chunk_ref,
                                "source": source,
                                "qa_type": qa_type,
                                "concepts": concepts,
                                "tokens_used": response.usage.total_tokens,
                                "response": v,
                            });
                            serde_json::to_string(&enriched).unwrap_or_default()
                        }
                        Err(_) => {
                            let fallback = serde_json::json!({
                                "chunk_ref": chunk_ref,
                                "source": source,
                                "qa_type": qa_type,
                                "concepts": concepts,
                                "tokens_used": response.usage.total_tokens,
                                "raw_response": response.text,
                                "parse_error": "LLM response was not valid JSON",
                            });
                            serde_json::to_string(&fallback).unwrap_or_default()
                        }
                    };

                    {
                        let mut out = out_mutex.lock().await;
                        out.push(qa_entry);
                        // Incremental write every 10 completions for crash safety
                        if out.len() % 10 == 0
                            && let Err(e) = std::fs::write(&output_path, out.join("\n") + "\n")
                        {
                            tracing::warn!(target: "corpus.qa", error = %e, "Incremental write failed — data held in memory");
                        }
                    }
                    completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                None => {
                    // Error already logged and counted in the retry loop above.
                }
            }

            let c = completed.load(std::sync::atomic::Ordering::Relaxed);
            let f = failed.load(std::sync::atomic::Ordering::Relaxed);
            if (c + f).is_multiple_of(10) || c + f == 1 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let rate = (c + f) as f64 / elapsed.max(1.0);
                eprintln!(
                    "  [{done}/{total}] {c} ok, {f} fail ({rate:.1}/s)",
                    done = c + f,
                    total = prompts_arc.len(),
                    c = c,
                    f = f,
                    rate = rate
                );
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    // Write output
    {
        let out = out_mutex.lock().await;
        std::fs::write(&output_path, out.join("\n") + "\n")?;
    }

    let elapsed = start_time.elapsed();
    let c = completed.load(std::sync::atomic::Ordering::Relaxed);
    let f = failed.load(std::sync::atomic::Ordering::Relaxed);
    println!("\n=== Done ===");
    println!("  Generated: {c}");
    println!("  Failed:    {f}");
    println!("  Time:      {:.1}s", elapsed.as_secs_f64());
    println!("  Output:    {}", output_path.display());
    Ok(())
}
