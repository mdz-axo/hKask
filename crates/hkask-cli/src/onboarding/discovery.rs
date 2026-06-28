//! Dynamic model discovery pipeline for onboarding.
//!
//! 4-layer architecture:
//!   1. Query HuggingFace API for text-generation models sorted by recency
//!   2. Filter to providers with >5000 followers, classify each model as
//!      Thinking (reasoning) or Instruct (standard/flash)
//!   3. Per provider-family: keep the latest Thinking AND latest Instruct model
//!   4. (Planned) Web cross-reference verification pass
//!
//! Falls back to provider API model listing (InferenceRouter) when HF is unreachable,
//! and to static curated lists as last resort.

use chrono::Utc;
use hkask_inference::{InferenceConfig, InferenceRouter, ProviderId, RouterModelEntry};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

// ── HuggingFace API types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
struct HfModel {
    #[serde(rename = "_id")]
    id: Option<String>,
    #[serde(rename = "modelId")]
    model_id: Option<String>,
    author: Option<String>,
    #[serde(rename = "lastModified")]
    last_modified: Option<String>,
    #[serde(rename = "pipeline_tag")]
    pipeline_tag: Option<String>,
    tags: Option<Vec<String>>,
    downloads: Option<u64>,
    likes: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
struct HfUser {
    followers: Option<u64>,
}

// ── Internal types ─────────────────────────────────────────────────────────

/// Classification of a model's reasoning approach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModelKind {
    /// Reasoning/thinking models (R1, thinking variants, deep reasoning)
    Thinking,
    /// Standard instruct-tuned / flash models
    Instruct,
}

/// A discovered model from the pipeline.
#[derive(Debug, Clone)]
pub(crate) struct DiscoveredModel {
    pub model_id: String,
    pub family: String,
    pub kind: ModelKind,
    pub last_updated: String,
    pub followers: u64,
    pub description: String,
}

// ── Onboarding model (UI-facing) ───────────────────────────────────────────

/// Whether a model entry came from dynamic discovery or the static fallback.
#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum ModelSource {
    Dynamic,
    Fallback,
}

/// A model entry for the onboarding UI.
#[derive(Debug, Clone)]
pub(crate) struct OnboardingModel {
    pub label: String,
    pub full_id: String,
    pub description: String,
    pub provider: ProviderId,
    pub score: u32,
    pub source: ModelSource,
    pub kind: ModelKind,
    pub family: String,
}

// ── HF API Client ──────────────────────────────────────────────────────────

const HF_BASE: &str = "https://huggingface.co/api";
const FOLLOWER_THRESHOLD: u64 = 5000;
const RECENCY_DAYS: i64 = 180;

/// Fetch models from HuggingFace API filtered to text-generation, sorted by recency.
async fn fetch_hf_models(client: &reqwest::Client) -> Result<Vec<HfModel>, String> {
    let url = format!(
        "{}/models?sort=lastModified&direction=-1&limit=100&filter=text-generation-inference",
        HF_BASE
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "hKask-onboarding/0.31")
        .send()
        .await
        .map_err(|e| format!("HF API request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HF API returned {}", resp.status()));
    }

    let models: Vec<HfModel> = resp
        .json()
        .await
        .map_err(|e| format!("HF parse error: {e}"))?;

    Ok(models)
}

/// Fetch a HuggingFace user's follower count.
async fn fetch_hf_user_followers(client: &reqwest::Client, username: &str) -> Result<u64, String> {
    let url = format!("{}/users/{}", HF_BASE, username);
    let resp = client
        .get(&url)
        .header("User-Agent", "hKask-onboarding/0.31")
        .send()
        .await
        .map_err(|e| format!("HF user request failed: {e}"))?;

    if !resp.status().is_success() {
        return Ok(0); // Graceful: author may not exist
    }

    let user: HfUser = resp
        .json()
        .await
        .map_err(|e| format!("HF user parse error: {e}"))?;

    Ok(user.followers.unwrap_or(0))
}

// ── Model classification ───────────────────────────────────────────────────

/// Keywords that signal a model is a thinking/reasoning variant.
const THINKING_KEYWORDS: &[&str] = &[
    "thinking",
    "reasoning",
    "r1-",
    "-r1",
    "deep-think",
    "deepseek-r1",
    "qwen3-max-thinking",
    "kimi-thinking",
    "o1-",
    "o3-",
    "o4-",
    "reasoner",
    "deep-thought",
];

/// Keywords excluding models that are NOT LLMs.
const NON_LLM_KEYWORDS: &[&str] = &[
    "embedding",
    "bge-",
    "gte-",
    "e5-",
    "stella",
    "jina-embeddings",
    "tts",
    "speech",
    "whisper",
    "parakeet",
    "zonos",
    "chatterbox",
    "ocr",
    "paddleocr",
    "got-ocr",
    "flux",
    "stable-diffusion",
    "sd-",
    "sdxl",
    "dall-e",
    "text-to-image",
    "image-generat",
    "imagen-",
    "text-to-video",
    "video-generat",
    "ltx-video",
    "clip-",
    "blip-",
    "reranker",
    "bge-reranker",
    "jina-reranker",
    "voice",
    "audio",
    "melotts",
    "fish-speech",
    "bgm",
    "musicgen",
    "upscale",
    "segmentation",
    "detection",
    "depth",
    "computer-vision",
];

/// Classify a model AND score it. Returns None if model should be excluded
/// (non-LLM or too small). Otherwise returns (kind, quality_score).
fn classify_and_score(model_id: &str) -> Option<(ModelKind, u32)> {
    let lower = model_id.to_lowercase();

    // Exclude non-LLM models
    if NON_LLM_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
        return None;
    }

    // Exclude small models
    for kw in &["-1b", "-3b", "-7b", "-8b", "-0.5b", "-1.5b", "-0.6b",
                "-mini", "-tiny", "-small", "-nano"] {
        if lower.contains(kw) { return None; }
    }

    let kind = if THINKING_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
        ModelKind::Thinking
    } else {
        ModelKind::Instruct
    };

    let score = compute_quality_score(&lower);
    Some((kind, score))
}

/// Compute a quality score from a model name (already lowercased).
fn compute_quality_score(lower: &str) -> u32 {
    let mut score: u32 = 10;

    // Parameter count signals
    for cap in lower.split(|c: char| !c.is_alphanumeric() && c != '.') {
        if cap.ends_with('t') {
            if let Ok(val) = cap.trim_end_matches('t').parse::<f64>() {
                score = score.max((val * 100.0) as u32);
            }
        }
        if cap.ends_with('b') {
            if let Ok(val) = cap.trim_end_matches('b').parse::<f64>() {
                score = score.max((val / 10.0) as u32);
            }
        }
    }

    // Quality tier signals
    for (signal, points) in &[("pro", 20u32), ("max", 18), ("ultra", 18),
                                ("super", 15), ("flash", 8), ("large", 10)] {
        if lower.contains(signal) { score += points; }
    }

    // Version signals
    for word in lower.split(|c: char| !c.is_alphanumeric()) {
        if word.len() >= 2 && word.starts_with('v') {
            if let Ok(val) = word[1..].parse::<u32>() {
                score += val.min(10);
            }
        }
    }

    score
}

// ── Family extraction ──────────────────────────────────────────────────────

/// Map HuggingFace author names to canonical model family names.
/// This is a stable structural mapping, not a model list.
fn author_to_family(author: &str, model_id: &str) -> String {
    match author.to_lowercase().as_str() {
        "deepseek-ai" => "deepseek".to_string(),
        "zai-org" => "glm".to_string(),
        "moonshotai" => "kimi".to_string(),
        "minimaxai" => "minimax".to_string(),
        "qwen" => "qwen".to_string(),
        "google" => "gemma".to_string(),
        "nvidia" => "nemotron".to_string(),
        "meta-llama" => "llama".to_string(),
        "mistralai" | "mistral" => "mistral".to_string(),
        "microsoft" => "phi".to_string(),
        "stepfun-ai" => "step".to_string(),
        "openai" => "openai".to_string(),
        "xiaomimimo" => "mimo".to_string(),
        "ai21labs" | "ai21" => "jamba".to_string(),
        "cohere" | "cohereforai" => "command".to_string(),
        "allenai" => "olmo".to_string(),
        "tiiuae" => "falcon".to_string(),
        "ibm" | "ibm-granite" => "granite".to_string(),
        _ => {
            // Fallback: parse family from model ID
            extract_family_from_id(model_id)
        }
    }
    .to_string()
}

/// Extract model family from the model ID by parsing the first segment.
fn extract_family_from_id(model_id: &str) -> String {
    let lower = model_id.to_lowercase();
    // Try splitting on `/` — take the last part, then first segment before `-`
    let base = lower.rsplit('/').next().unwrap_or(&lower);
    let first = base.split('-').next().unwrap_or(base);
    // Strip trailing digits/version numbers from family name
    first
        .trim_end_matches(|c: char| c.is_ascii_digit() || c == '.')
        .trim()
        .to_string()
}

// ── Provider-to-hKask provider mapping ─────────────────────────────────────

// ── Main pipeline ──────────────────────────────────────────────────────────

/// Run the full discovery pipeline.
///
/// Returns `(models, source_label)` where models is the deduplicated list
/// and source_label describes where they came from.
pub(crate) async fn discover_models(config: &InferenceConfig) -> (Vec<OnboardingModel>, String) {
    // ── Layer 1-2: HuggingFace pipeline ───────────────────────────────
    let hf_results: Vec<DiscoveredModel> = if let Ok(client) = config.build_client() {
        run_hf_pipeline(&client, config).await.unwrap_or_default()
    } else {
        Vec::new()
    };

    // ── Layer 3: Cross-reference with configured provider ──────────────
    if !hf_results.is_empty() {
        let router = InferenceRouter::new(config.clone());
        let provider_models = router.list_models().await;
        if !provider_models.is_empty() {
            let cross_ref = cross_reference_with_provider(&hf_results, &provider_models, config);
            if !cross_ref.is_empty() {
                return (
                    cross_ref,
                    format!("HF + {} (≤6mo, >5k followers)", provider_label(config)),
                );
            }
        }
    }

    // ── Fallback 1: Provider API directly ─────────────────────────────
    let router = InferenceRouter::new(config.clone());
    let router_models = router.list_models().await;
    if !router_models.is_empty() {
        let models = build_from_router(router_models, config);
        if !models.is_empty() {
            return (models, format!("provider API ({})", provider_label(config)));
        }
    }

    // ── Fallback 2: Static curated lists ──────────────────────────────
    let models = build_fallback(config);
    (
        models,
        format!("curated fallback ({} unreachable)", provider_label(config)),
    )
}

/// Run the HuggingFace pipeline: fetch, filter, classify, deduplicate.
async fn run_hf_pipeline(
    client: &reqwest::Client,
    config: &InferenceConfig,
) -> Result<Vec<DiscoveredModel>, String> {
    // Layer 1: Fetch recent text-gen models from HF
    let hf_models = fetch_hf_models(client).await?;
    if hf_models.is_empty() {
        return Err("No models returned from HF".into());
    }

    // Collect unique authors
    let mut authors: HashSet<String> = HashSet::new();
    for m in &hf_models {
        if let Some(ref author) = m.author {
            authors.insert(author.clone());
        }
    }

    // Fetch follower counts for all authors (with basic dedup caching)
    let mut followers: HashMap<String, u64> = HashMap::new();
    for author in &authors {
        match fetch_hf_user_followers(client, author).await {
            Ok(count) => {
                followers.insert(author.clone(), count);
            }
            Err(_) => {
                followers.insert(author.clone(), 0);
            }
        }
    }

    // Layer 2: Filter to leading providers, classify models
    let cutoff = Utc::now() - chrono::Duration::days(RECENCY_DAYS);
    let mut by_family: HashMap<String, Vec<HfModel>> = HashMap::new();

    for m in &hf_models {
        let author = match &m.author {
            Some(a) => a,
            None => continue,
        };
        let follower_count = followers.get(author).copied().unwrap_or(0);
        if follower_count < FOLLOWER_THRESHOLD {
            continue;
        }

        let model_id = m.model_id.as_deref().unwrap_or("");
        if model_id.is_empty() {
            continue;
        }

        // Check recency
        let recent = m
            .last_modified
            .as_ref()
            .and_then(|ts| {
                chrono::DateTime::parse_from_rfc3339(ts)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            })
            .map(|dt| dt >= cutoff)
            .unwrap_or(false);

        if !recent {
            continue;
        }

        // Classify
        let kind = match classify_and_score(model_id) {
            Some(k) => k,
            None => continue,
        };

        let family = author_to_family(author, model_id);
        by_family.entry(family).or_default().push((*m).clone());
    }

    // Layer 3: Per family, keep best Thinking + best Instruct (by recency)
    let mut results: Vec<DiscoveredModel> = Vec::new();

    for (family, models) in &by_family {
        let mut best_thinking: Option<&HfModel> = None;
        let mut best_instruct: Option<&HfModel> = None;

        for m in models {
            let model_id = m.model_id.as_deref().unwrap_or("");
            let kind = classify_and_score(model_id).map(|(k, _)| k).unwrap_or(ModelKind::Instruct);

            match kind {
                ModelKind::Thinking => {
                    if best_thinking.is_none() || is_newer(m, best_thinking.unwrap()) {
                        best_thinking = Some(m);
                    }
                }
                ModelKind::Instruct => {
                    if best_instruct.is_none() || is_newer(m, best_instruct.unwrap()) {
                        best_instruct = Some(m);
                    }
                }
            }
        }

        let _author = models
            .first()
            .and_then(|m| m.author.as_deref())
            .unwrap_or("");
        let followers = followers.get(_author).copied().unwrap_or(0);

        if let Some(m) = best_thinking {
            let hf_id = m.model_id.as_deref().unwrap_or("");
            results.push(DiscoveredModel {
                model_id: hf_id.to_string(),
                family: family.clone(),
                kind: ModelKind::Thinking,
                last_updated: m.last_modified.clone().unwrap_or_default(),
                followers,
                description: format!("⚡ Thinking — {}", m.model_id.as_deref().unwrap_or("")),
            });
        }

        if let Some(m) = best_instruct {
            let hf_id = m.model_id.as_deref().unwrap_or("");
            results.push(DiscoveredModel {
                model_id: hf_id.to_string(),
                family: family.clone(),
                kind: ModelKind::Instruct,
                last_updated: m.last_modified.clone().unwrap_or_default(),
                followers,
                description: format!("Instruct — {}", m.model_id.as_deref().unwrap_or("")),
            });
        }
    }

    if results.is_empty() {
        return Err("No qualifying models found".into());
    }

    Ok(results)
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn is_newer(a: &HfModel, b: &HfModel) -> bool {
    let ta = a.last_modified.as_deref().unwrap_or("");
    let tb = b.last_modified.as_deref().unwrap_or("");
    ta > tb
}

fn provider_label(config: &InferenceConfig) -> &'static str {
    match config.default_provider {
        ProviderId::KiloCode => "KiloCode",
        ProviderId::DeepInfra => "DeepInfra",
        ProviderId::Together => "Together AI",
        ProviderId::Fal => "fal.ai",
        ProviderId::OpenRouter => "OpenRouter",
        _ => "provider",
    }
}

pub(crate) fn shorten_for_display(id: &str) -> String {
    // Strip provider prefix (DI/, KC/, etc.) and org prefix
    let base = id.splitn(2, '/').nth(1).unwrap_or(id);
    let base = base.splitn(2, '/').nth(1).unwrap_or(base);
    let base = if base.is_empty() { id } else { base };

    base.replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => {
                    f.to_uppercase().collect::<String>() + c.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── Fallback builders ──────────────────────────────────────────────────────

fn build_from_router(
    models: Vec<RouterModelEntry>,
    config: &InferenceConfig,
) -> Vec<OnboardingModel> {
    // Classify and deduplicate by family
    let mut by_family: HashMap<String, Vec<RouterModelEntry>> = HashMap::new();

    for m in models {
        let Some((kind, _score)) = classify_and_score(&m.model) else { continue; };
        let family = extract_family_from_id(&m.model);
        by_family.entry(family).or_default().push(m);
    }

    let mut results: Vec<OnboardingModel> = Vec::new();
    for (family, family_models) in &by_family {
        // Separate into thinking and instruct, scored
        let mut thinking: Vec<(&RouterModelEntry, u32)> = family_models
            .iter()
            .filter_map(|m| classify_and_score(&m.model).and_then(|(k, s)| if k == ModelKind::Thinking { Some((m, s)) } else { None }))
            .collect();
        let mut instruct: Vec<(&RouterModelEntry, u32)> = family_models
            .iter()
            .filter_map(|m| classify_and_score(&m.model).and_then(|(k, s)| if k == ModelKind::Instruct { Some((m, s)) } else { None }))
            .collect();

        thinking.sort_by(|a, b| b.1.cmp(&a.1));
        instruct.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((m, _)) = thinking.first() {
            results.push(build_onboarding_entry(
                m,
                &family,
                ModelKind::Thinking,
                config,
            ));
        }
        if let Some((m, _)) = instruct.first() {
            results.push(build_onboarding_entry(
                m,
                &family,
                ModelKind::Instruct,
                config,
            ));
        }
    }

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(24); // 12 families × 2 models each
    results
}

fn build_onboarding_entry(
    m: &RouterModelEntry,
    family: &str,
    kind: ModelKind,
    _config: &InferenceConfig,
) -> OnboardingModel {
    let base_score = compute_quality_score(&m.model.to_lowercase());
    let score = base_score + if kind == ModelKind::Thinking { 50 } else { 0 };
    let kind_label = if kind == ModelKind::Thinking { "⚡ Thinking" } else { "Instruct" };
    OnboardingModel {
        label: shorten_for_display(&m.prefixed_name),
        full_id: m.prefixed_name.clone(),
        description: format!("{} — {}", kind_label, m.model),
        provider: m.provider,
        score,
        source: ModelSource::Dynamic,
        kind,
        family: family.to_string(),
    }
}


// ── Fallback lists ─────────────────────────────────────────────────────────

const DEEPINFRA_FALLBACK: &[(&str, &str, ModelKind)] = &[
    (
        "deepseek-ai/DeepSeek-V4-Pro",
        "1.6T MoE, 1M ctx, top coding",
        ModelKind::Instruct,
    ),
    (
        "deepseek-ai/DeepSeek-R1-0528",
        "R1 reasoning, deep thought",
        ModelKind::Thinking,
    ),
    (
        "zai-org/GLM-5.2",
        "744B MoE, MIT, GPQA leader",
        ModelKind::Instruct,
    ),
    (
        "moonshotai/Kimi-K2.6",
        "1T MoE, agent swarms",
        ModelKind::Instruct,
    ),
    (
        "MiniMaxAI/MiniMax-M3",
        "1M ctx, multimodal, SWE-Bench",
        ModelKind::Instruct,
    ),
    (
        "Qwen/Qwen3.5-397B-A17B",
        "397B MoE, Apache 2.0",
        ModelKind::Instruct,
    ),
    (
        "nvidia/NVIDIA-Nemotron-3-Super-120B-A12B",
        "120B MoE, 1M ctx",
        ModelKind::Instruct,
    ),
    (
        "google/gemma-4-31B-it",
        "31B dense, Apache 2.0",
        ModelKind::Instruct,
    ),
    (
        "deepseek-ai/DeepSeek-V4-Flash",
        "284B MoE, efficient 1M ctx",
        ModelKind::Instruct,
    ),
];

const KILOCODE_FALLBACK: &[(&str, &str, ModelKind)] = &[
    ("deepseek-v4-pro", "1.6T MoE, 1M ctx", ModelKind::Instruct),
    ("deepseek-r1", "R1 reasoning", ModelKind::Thinking),
    ("glm-5.2", "744B MoE, MIT", ModelKind::Instruct),
    ("kimi-k2.6", "1T MoE, agent swarms", ModelKind::Instruct),
    ("minimax-m3", "1M ctx, multimodal", ModelKind::Instruct),
    (
        "qwen3.5-397b-a17b",
        "397B MoE, Apache 2.0",
        ModelKind::Instruct,
    ),
    ("nemotron-3-super", "120B MoE, 1M ctx", ModelKind::Instruct),
    (
        "gemma-4-31b-it",
        "31B dense, Apache 2.0",
        ModelKind::Instruct,
    ),
    (
        "deepseek-v4-flash",
        "284B MoE, efficient",
        ModelKind::Instruct,
    ),
];

fn build_fallback(config: &InferenceConfig) -> Vec<OnboardingModel> {
    let list = match config.default_provider {
        ProviderId::KiloCode => KILOCODE_FALLBACK,
        _ => DEEPINFRA_FALLBACK,
    };

    list.iter()
        .map(|(id, desc, kind)| OnboardingModel {
            label: shorten_for_display(id),
            full_id: config.default_provider.prefix_model(id),
            description: format!(
                "{} — {}",
                if *kind == ModelKind::Thinking {
                    "⚡ Thinking"
                } else {
                    "Instruct"
                },
                desc
            ),
            provider: config.default_provider,
            score: 0,
            source: ModelSource::Fallback,
            kind: kind.clone(),
            family: extract_family_from_id(id),
        })
        .collect()
}

// ── Shared filters (used by router fallback too) ───────────────────────────

fn is_likely_llm(model_id: &str) -> bool {
    let lower = model_id.to_lowercase();
    !NON_LLM_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

fn is_likely_small_model(model_id: &str) -> bool {
    let lower = model_id.to_lowercase();
    for kw in &[
        "-1b", "-3b", "-7b", "-8b", "-0.5b", "-1.5b", "-0.6b", "-mini", "-tiny", "-small", "-nano",
    ] {
        if lower.contains(kw) {
            return true;
        }
    }
    false
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_thinking_models() {
        assert_eq!(
            classify_and_score("deepseek-ai/DeepSeek-R1-0528").map(|(k, _)| k),
            Some(ModelKind::Thinking)
        );
        assert_eq!(
            classify_and_score("Qwen/Qwen3-Max-Thinking").map(|(k, _)| k),
            Some(ModelKind::Thinking)
        );
    }

    #[test]
    fn classify_instruct_models() {
        assert_eq!(
            classify_and_score("deepseek-ai/DeepSeek-V4-Pro").map(|(k, _)| k),
            Some(ModelKind::Instruct)
        );
        assert_eq!(
            classify_and_score("google/gemma-4-31B-it").map(|(k, _)| k),
            Some(ModelKind::Instruct)
        );
    }

    #[test]
    fn classify_rejects_non_llm() {
        assert_eq!(classify_and_score("BAAI/bge-m3"), None);
        assert_eq!(classify_and_score("black-forest-labs/FLUX.1-dev"), None);
    }

    #[test]
    fn family_extraction() {
        assert_eq!(
            author_to_family("deepseek-ai", "deepseek-ai/DeepSeek-V4-Pro"),
            "deepseek"
        );
        assert_eq!(author_to_family("zai-org", "zai-org/GLM-5.2"), "glm");
        assert_eq!(author_to_family("qwen", "Qwen/Qwen3.5-397B-A17B"), "qwen");
    }

    #[test]
    fn family_from_id_fallback() {
        assert_eq!(
            extract_family_from_id("some-org/UnknownModel-v2"),
            "unknownmodel"
        );
        assert_eq!(extract_family_from_id("deepseek-v4-pro"), "deepseek");
        assert_eq!(extract_family_from_id("glm-5.2"), "glm");
    }

    #[test]
    fn shorten_display_strips_prefixes() {
        assert_eq!(
            shorten_for_display("DI/deepseek-ai/DeepSeek-V4-Pro"),
            "Deepseek V4 Pro"
        );
        assert_eq!(shorten_for_display("KC/deepseek-v4-pro"), "Deepseek V4 Pro");
    }

    #[test]
    fn classify_and_score_ranks_pro_over_flash() {
        let (_, pro) = classify_and_score("deepseek-v4-pro").unwrap();
        let (_, flash) = classify_and_score("deepseek-v4-flash").unwrap();
        assert!(pro > flash);
    }

    #[test]
    fn classify_and_score_rejects_small_models() {
        assert_eq!(classify_and_score("qwen3-8b"), None);
        assert_eq!(classify_and_score("phi-3-mini"), None);
    }


}

/// Cross-reference HF-discovered models against what's actually available
/// from the configured provider. Uses normalized fuzzy matching because
/// provider model IDs differ from HF IDs (e.g., KiloCode "deepseek-v4-pro"
/// vs HF "deepseek-ai/DeepSeek-V4-Pro").
fn cross_reference_with_provider(
    hf_models: &[DiscoveredModel],
    provider_models: &[RouterModelEntry],
    config: &InferenceConfig,
) -> Vec<OnboardingModel> {
    if provider_models.is_empty() {
        return Vec::new();
    }

    let provider_index: Vec<(&RouterModelEntry, String)> = provider_models
        .iter()
        .map(|m| (m, normalize_for_match(&m.model)))
        .collect();

    let mut results: Vec<OnboardingModel> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for d in hf_models {
        let hf_norm = normalize_for_match(&d.model_id);

        let best = provider_index.iter().find(|(_, norm)| {
            norm.contains(&hf_norm)
                || hf_norm.contains(norm.as_str())
                || fuzzy_family_match(norm, &hf_norm, &d.family)
        });

        if let Some((entry, _)) = best {
            // Deduplicate per family+kind
            let key = format!("{}:{:?}", d.family, d.kind);
            if !seen.insert(key) {
                continue;
            }

            let display = shorten_for_display(&entry.prefixed_name);
            let score = if d.kind == ModelKind::Thinking {
                100
            } else {
                50
            } + d.followers.min(50000) / 1000;

            results.push(OnboardingModel {
                label: display,
                full_id: entry.prefixed_name.clone(),
                description: d.description.clone(),
                provider: config.default_provider,
                score: score as u32,
                source: ModelSource::Dynamic,
                kind: d.kind.clone(),
                family: d.family.clone(),
            });
        }
    }

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(24);
    results
}

/// Normalize a model ID for fuzzy comparison: lowercase, strip separators.
fn normalize_for_match(id: &str) -> String {
    id.split('/')
        .last()
        .unwrap_or(id)
        .to_lowercase()
        .replace(['-', '_', ' ', '.'], "")
}

/// Check if two normalized IDs share the same model family.
fn fuzzy_family_match(provider_norm: &str, hf_norm: &str, family: &str) -> bool {
    let f = family.to_lowercase().replace(['-', '_'], "");
    provider_norm.contains(&f) && hf_norm.contains(&f)
}
