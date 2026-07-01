//! Utility functions — text chunking, trace prompts, failure classification.

use crate::types::TraceType;
use std::collections::HashMap;

/// Build trace-type-specific prompt guidance.
pub fn trace_type_prompt(tt: TraceType) -> String {
    match tt {
        TraceType::WordAct => "TRACE TYPE: WordAct — Persona Calibration.\n\n\
             These traces train HOW TO SOUND. Each trace calibrates agent persona:\n\
             - ContEXT: A conversational situation where the persona matters.\n\
             - PERSONA CONSTRAINTS: What tone, posture, and phrasing the agent should use.\n\
             - TARGET UTTERANCE: The calibrated response in persona.\n\
             - CALIBRATION NOTES: Why this utterance fits and alternatives that would not.\n\
             Focus on tone, voice, dialogue patterns, and conversational posture."
            .to_string(),
        TraceType::FlowDef => "TRACE TYPE: FlowDef — Procedural Decomposition.\n\n\
             These traces train HOW TO THINK. Each trace decomposes a problem:\n\
             - SITUATION: An ill-formed scenario requiring the skill's process.\n\
             - DECOMPOSITION SEQUENCE: Step-by-step application of the skill's procedure.\n\
             - SYNTHESIS: Resolution derived from the decomposed sub-questions.\n\
             - VERIFICATION: Check that the resolution satisfies the original situation.\n\
             Focus on procedural correctness, step ordering, and verification."
            .to_string(),
        TraceType::KnowAct => "TRACE TYPE: KnowAct — Pattern Recognition & Classification.\n\n\
             These traces train HOW TO CLASSIFY. Each trace distinguishes patterns:\n\
             - PATTERN EXEMPLAR: A clear example of the pattern being taught.\n\
             - POSITIVE CASES: Examples that match the pattern (vary difficulty).\n\
             - NEGATIVE CASES: Near-miss examples that look like the pattern but aren't.\n\
             - DECISION BOUNDARY: The rule or heuristic that separates matches from non-matches.\n\
             Focus on classification precision, boundary cases, and misclassification avoidance."
            .to_string(),
        TraceType::Composite => "TRACE TYPE: Composite — Mixed WordAct + FlowDef.\n\n\
             This skill requires both persona calibration AND procedural decomposition.\n\
             Generate traces that alternate between:\n\
             - WordAct segments: persona-appropriate utterances within the procedure.\n\
             - FlowDef segments: procedural decomposition of the task at hand.\n\
             Ensure persona consistency across procedural steps."
            .to_string(),
    }
}

/// Classify failure category from judge text.
pub fn classify_failure(judge_text: &str) -> &'static str {
    let lower = judge_text.to_lowercase();
    if lower.contains("hallucinat") || lower.contains("fabricat") || lower.contains("made up") {
        "hallucination"
    } else if lower.contains("omit") || lower.contains("missing") || lower.contains("incomplete") {
        "omission"
    } else if lower.contains("step")
        || lower.contains("order")
        || lower.contains("procedure")
        || lower.contains("sequence")
    {
        "procedural_error"
    } else if lower.contains("irrelevant")
        || lower.contains("off topic")
        || lower.contains("misunderst")
    {
        "off_target"
    } else {
        "other"
    }
}

/// Count failures by category.
pub fn failure_counts(traces: &[serde_json::Value]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for trace in traces {
        if let Some(cat) = trace.get("failure_category").and_then(|v| v.as_str()) {
            *counts.entry(cat.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

/// Split text into chunks at paragraph boundaries, each under `max_chars`.
/// Splits at double-newline boundaries first, then falls back to single-newline
/// if a paragraph exceeds the limit.
pub fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut current = String::new();

    for para in paragraphs {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if current.len() + para.len() + 2 > max_chars && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current = String::new();
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(para);

        // If a single paragraph exceeds the limit, split by sentences (newlines within)
        while current.len() > max_chars {
            if let Some(split_point) = current[..max_chars].rfind('\n') {
                let take = current[..split_point].trim().to_string();
                if !take.is_empty() {
                    chunks.push(take);
                }
                current = current[split_point + 1..].trim().to_string();
            } else {
                // No newline found — hard split at max_chars
                let take = current[..max_chars].trim().to_string();
                if !take.is_empty() {
                    chunks.push(take);
                }
                current = current[max_chars..].trim().to_string();
            }
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    if chunks.is_empty() {
        vec![text.to_string()]
    } else {
        chunks
    }
}
