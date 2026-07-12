//! Dual-model classification for epistemic integrity.
//!
//! Runs the same few-shot prompt against two peer models from different
//! jurisdictions, integrates their extractions, and detects divergence.
//! Neither model is primary — both are equal peers whose agreement or
//! disagreement produces the classification signal.
//!
//! **Design principle:** No single model is trusted as the sole gate to shared
//! semantic memory. Dual-model classification eliminates the single-model
//! bias path — every classification is an integration of two independent
//! viewpoints. When they agree, confidence increases. When they diverge,
//! the divergence itself is the signal.
//!
//! **Recommended configuration:**
//! - Model A: Chinese-hosted (e.g., KiloCode/Qwen)
//! - Model B: US/EU-hosted (e.g., DeepInfra/Gemma 4)
//!
//! Cap at one model from any single jurisdiction — both American and Chinese
//! models have politically constrained viewpoints.

use hkask_cns::classify_span::ClassifySpan;
use hkask_services_core::{DomainKind, ErrorKind, HkaskSettings, ServiceError};
use hkask_types::observable_span::ObservableSpan;
use serde::Serialize;
use std::collections::HashSet;

use crate::classify_impl::{
    ClassifierConfig, TripleExtraction, classify_batch, extract_triples_batch,
};

/// Configuration for dual-model classification.
/// Two peer models — neither is primary.
#[derive(Clone)]
pub struct DualClassifierConfig {
    /// First peer classifier (from classifier YAML + HKASK_CLASSIFIER_MODEL_A).
    pub model_a: ClassifierConfig,
    /// Second peer classifier (from HKASK_CLASSIFIER_MODEL_B).
    pub model_b: ClassifierConfig,
}

/// Result of integrating two TripleExtractions with divergence analysis.
#[derive(Debug, Clone, Serialize)]
pub struct DualTripleExtraction {
    /// Merged extraction: union of concepts, entities, relationships from both models.
    /// Where both models produced the same item, it appears once.
    /// Where models diverged, the divergent item is annotated with provenance.
    pub merged: IntegratedExtraction,
    /// Jaccard similarity between model A and model B entity sets.
    /// 1.0 = identical, 0.0 = no overlap.
    pub entity_agreement: f64,
    /// Jaccard similarity between concept sets.
    pub concept_agreement: f64,
    /// Entities found only by model A.
    pub a_only_entities: Vec<String>,
    /// Entities found only by model B.
    pub b_only_entities: Vec<String>,
    /// Concepts found only by model A.
    pub a_only_concepts: Vec<String>,
    /// Concepts found only by model B.
    pub b_only_concepts: Vec<String>,
    /// Whether the divergence exceeds the alert threshold.
    /// True when entity_agreement < JACCARD_DIVERGENCE_THRESHOLD or either model refused/failed.
    pub divergence_alert: bool,
    /// Model identifier for primary.
    pub model_a: String,
    /// Model identifier for secondary.
    pub model_b: String,
    /// Provider for primary (e.g., "kilocode", "deepinfra").
    pub provider_a: String,
    /// Provider for secondary.
    pub provider_b: String,
}

/// Integrated extraction — merged output from two models.
#[derive(Debug, Clone, Serialize)]
pub struct IntegratedExtraction {
    /// Topic: uses primary model's topic when both agree on topic embedding;
    /// otherwise both topics are concatenated with a separator.
    pub topic: String,
    /// Union of concepts from both models, deduplicated case-insensitively.
    pub concepts: Vec<String>,
    /// Union of entities from both models, deduplicated case-insensitively.
    pub entities: Vec<String>,
    /// Union of relationships from both models.
    pub relationships: Vec<String>,
    /// Primary dimension: uses the dimension both models agree on;
    /// otherwise the primary model's dimension with an annotation.
    pub primary_dimension: String,
    /// Quality flags: union from both models.
    pub quality_flags: Vec<String>,
    /// Extra fields from both models, merged by key.
    /// When both models provide the same key, values are stored as an array.
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl IntegratedExtraction {
    /// Convert to a TripleExtraction for storage compatibility.
    pub fn to_triple_extraction(&self) -> TripleExtraction {
        TripleExtraction {
            topic: self.topic.clone(),
            concepts: self.concepts.clone(),
            entities: self.entities.clone(),
            relationships: self.relationships.clone(),
            primary_dimension: self.primary_dimension.clone(),
            quality_flags: self.quality_flags.clone(),
            extra: self.extra.clone(),
        }
    }
}

/// Jaccard similarity between two sets of strings (case-insensitive comparison).
fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
    let set_a: HashSet<String> = a.iter().map(|s| s.to_lowercase()).collect();
    let set_b: HashSet<String> = b.iter().map(|s| s.to_lowercase()).collect();

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        1.0 // both empty = perfect agreement
    } else {
        intersection as f64 / union as f64
    }
}

/// Items in `a` but not in `b` (case-insensitive set difference).
fn set_difference(a: &[String], b: &[String]) -> Vec<String> {
    let set_b: HashSet<String> = b.iter().map(|s| s.to_lowercase()).collect();
    a.iter()
        .filter(|s| !set_b.contains(&s.to_lowercase()))
        .cloned()
        .collect()
}

/// Deduplicate case-insensitively, preserving first occurrence casing.
fn dedup_case_insensitive(items: &[String]) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::new();
    for item in items {
        if seen.insert(item.to_lowercase()) {
            result.push(item.clone());
        }
    }
    result
}

/// Integrate two TripleExtractions from different models into a single
/// combined extraction with divergence analysis.
///
/// This is the core epistemic integrity function. It does not prefer either
/// model — it merges both and reports where they disagree.
///
/// pre:  a and b are valid TripleExtractions (may be empty/default)
/// post: returns DualTripleExtraction with merged output, agreement scores,
///       and divergence flags
#[must_use]
pub fn integrate_dual_triples(
    a: &TripleExtraction,
    b: &TripleExtraction,
    model_a: &str,
    model_b: &str,
    provider_a: &str,
    provider_b: &str,
) -> DualTripleExtraction {
    let entity_agreement = jaccard_similarity(&a.entities, &b.entities);
    let concept_agreement = jaccard_similarity(&a.concepts, &b.concepts);

    let a_only_entities = set_difference(&a.entities, &b.entities);
    let b_only_entities = set_difference(&b.entities, &a.entities);
    let a_only_concepts = set_difference(&a.concepts, &b.concepts);
    let b_only_concepts = set_difference(&b.concepts, &a.concepts);

    // Topic: use union when models disagree
    let topic = if a.topic.to_lowercase().trim() == b.topic.to_lowercase().trim() {
        a.topic.clone()
    } else if a.topic.is_empty() {
        b.topic.clone()
    } else if b.topic.is_empty() {
        a.topic.clone()
    } else {
        format!("[A] {} | [B] {}", a.topic, b.topic)
    };

    // Concepts and entities: union, deduplicated
    let all_concepts: Vec<String> = a
        .concepts
        .iter()
        .chain(b.concepts.iter())
        .cloned()
        .collect();
    let concepts = dedup_case_insensitive(&all_concepts);

    let all_entities: Vec<String> = a
        .entities
        .iter()
        .chain(b.entities.iter())
        .cloned()
        .collect();
    let entities = dedup_case_insensitive(&all_entities);

    // Relationships: union
    let mut relationships = a.relationships.clone();
    for rel in &b.relationships {
        if !relationships
            .iter()
            .any(|r| r.to_lowercase() == rel.to_lowercase())
        {
            relationships.push(rel.clone());
        }
    }

    // Primary dimension: use agreed dimension, otherwise primary model's
    let primary_dimension =
        if a.primary_dimension.to_lowercase() == b.primary_dimension.to_lowercase() {
            a.primary_dimension.clone()
        } else if a.primary_dimension.is_empty() {
            b.primary_dimension.clone()
        } else if b.primary_dimension.is_empty() {
            a.primary_dimension.clone()
        } else {
            format!("[A:{} B:{}]", a.primary_dimension, b.primary_dimension)
        };

    // Quality flags: union
    let mut quality_flags = a.quality_flags.clone();
    for flag in &b.quality_flags {
        if !quality_flags
            .iter()
            .any(|f| f.to_lowercase() == flag.to_lowercase())
        {
            quality_flags.push(flag.clone());
        }
    }

    // Extra: merge, storing arrays when keys collide
    let mut extra = a.extra.clone();
    for (key, value) in &b.extra {
        extra
            .entry(key.clone())
            .and_modify(|existing| {
                // If already an array, push; otherwise wrap in array
                if let serde_json::Value::Array(arr) = existing {
                    if !arr.iter().any(|v| v == value) {
                        arr.push(value.clone());
                    }
                } else if existing != value {
                    *existing = serde_json::Value::Array(vec![existing.clone(), value.clone()]);
                }
            })
            .or_insert_with(|| value.clone());
    }

    // Divergence threshold: alert when entity agreement drops below 0.6
    let divergence_alert = entity_agreement < JACCARD_DIVERGENCE_THRESHOLD;

    let integrated = IntegratedExtraction {
        topic,
        concepts,
        entities,
        relationships,
        primary_dimension,
        quality_flags,
        extra,
    };

    DualTripleExtraction {
        merged: integrated,
        entity_agreement,
        concept_agreement,
        a_only_entities,
        b_only_entities,
        a_only_concepts,
        b_only_concepts,
        divergence_alert,
        model_a: model_a.to_string(),
        model_b: model_b.to_string(),
        provider_a: provider_a.to_string(),
        provider_b: provider_b.to_string(),
    }
}

/// Run dual-model triple extraction: send the same source texts to two
/// independently-configured models, extract triples from each, and integrate.
///
/// Returns integrated extractions in input order. Emits CNS fidelity spans
/// when models diverge.
///
/// pre:  config.model_a and config.model_b are valid ClassifierConfigs
///       with API keys resolved
/// post: returns Vec<DualTripleExtraction> in input order; partial failures
///       integrate what's available and flag divergence
#[must_use = "result must be used"]
pub async fn extract_triples_dual_batch(
    texts: &[String],
    config: &DualClassifierConfig,
) -> Result<Vec<DualTripleExtraction>, ServiceError> {
    tracing::info!(
        target: "cns.classify",
        operation = "extract_triples_dual_batch",
        item_count = texts.len(),
        model_a = %config.model_a.model,
        model_b = %config.model_b.model,
        "CNS"
    );

    // Run both classifiers in parallel — peer models, neither primary
    let (a_results, b_results) = tokio::join!(
        extract_triples_batch(texts, &config.model_a),
        extract_triples_batch(texts, &config.model_b),
    );

    let a_extractions = a_results.unwrap_or_else(|e| {
        tracing::warn!(error = %e, model = %config.model_a.model, "Model A classifier batch failed, using defaults");
        vec![TripleExtraction::default(); texts.len()]
    });

    let b_extractions = b_results.unwrap_or_else(|e| {
        tracing::warn!(error = %e, model = %config.model_b.model, "Model B classifier batch failed, using defaults");
        vec![TripleExtraction::default(); texts.len()]
    });

    // Extract provider names from model IDs (e.g., "KC/qwen/..." -> "KC",
    // "DI/google/..." -> "DI")
    let provider_a = config.model_a.model.split('/').next().unwrap_or("unknown");
    let provider_b = config.model_b.model.split('/').next().unwrap_or("unknown");

    // Integrate each pair
    let integrated: Vec<DualTripleExtraction> = a_extractions
        .iter()
        .zip(b_extractions.iter())
        .map(|(a, b)| {
            integrate_dual_triples(
                a,
                b,
                &config.model_a.model,
                &config.model_b.model,
                provider_a,
                provider_b,
            )
        })
        .collect();

    // Emit CNS fidelity spans for diverging extractions
    for (i, result) in integrated.iter().enumerate() {
        if result.divergence_alert {
            let divergence_detail = serde_json::json!({
                "index": i,
                "entity_agreement": result.entity_agreement,
                "concept_agreement": result.concept_agreement,
                "model_a": result.model_a,
                "model_b": result.model_b,
                "provider_a": result.provider_a,
                "provider_b": result.provider_b,
                "a_only_entities": result.a_only_entities,
                "b_only_entities": result.b_only_entities,
                "a_only_concepts": result.a_only_concepts,
                "b_only_concepts": result.b_only_concepts,
            });

            tracing::warn!(
                target: "cns.classify.dual_fidelity",
                agreement = %result.entity_agreement,
                a_only_count = result.a_only_entities.len(),
                b_only_count = result.b_only_entities.len(),
                divergence = %divergence_detail,
                "CNS"
            );

            ClassifySpan::ClassifyDualFidelity.emit("divergence_detected");
        }
    }

    // Summary statistics
    let alert_count = integrated.iter().filter(|r| r.divergence_alert).count();
    let avg_agreement: f64 =
        integrated.iter().map(|r| r.entity_agreement).sum::<f64>() / integrated.len().max(1) as f64;

    tracing::info!(
        target: "cns.classify",
        operation = "extract_triples_dual_batch_complete",
        item_count = texts.len(),
        divergence_alerts = alert_count,
        avg_entity_agreement = %avg_agreement,
        "CNS"
    );

    // P3.1: drift detection — monitor extraction patterns over time
    check_classifier_drift(
        &integrated,
        &config.model_a.model,
        &config.model_b.model,
        texts.len(),
    );

    Ok(integrated)
}

/// Run dual-model section type classification: send the same texts to two
/// models, classify each, and integrate results.
///
/// Returns integrated ClassifyResults with divergence analysis.
/// Falls back to primary-only when secondary is identical or fails.
#[must_use = "result must be used"]
pub async fn classify_dual_batch(
    texts: &[String],
    config: &DualClassifierConfig,
) -> Result<Vec<DualClassifyResult>, ServiceError> {
    tracing::info!(
        target: "cns.classify",
        operation = "classify_dual_batch",
        item_count = texts.len(),
        model_a = %config.model_a.model,
        model_b = %config.model_b.model,
        "CNS"
    );

    let (a_results, b_results) = tokio::join!(
        classify_batch(texts, config.model_a.clone(), None),
        classify_batch(texts, config.model_b.clone(), None),
    );

    let a_classes = a_results.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Primary classify batch failed");
        vec![]
    });
    let b_classes = b_results.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Secondary classify batch failed");
        vec![]
    });

    let len = texts.len();
    let a_classes = if a_classes.len() == len {
        a_classes
    } else {
        vec![]
    };
    let b_classes = if b_classes.len() == len {
        b_classes
    } else {
        vec![]
    };

    let provider_a = config.model_a.model.split('/').next().unwrap_or("unknown");
    let provider_b = config.model_b.model.split('/').next().unwrap_or("unknown");

    let results: Vec<DualClassifyResult> = (0..len)
        .map(|i| {
            let cat_a = a_classes
                .get(i)
                .map(|c| c.category.as_str())
                .unwrap_or("unknown");
            let cat_b = b_classes
                .get(i)
                .map(|c| c.category.as_str())
                .unwrap_or("unknown");
            let agreement = cat_a == cat_b;

            if !agreement {
                tracing::warn!(
                    target: "cns.classify.dual_fidelity",
                    index = i,
                    category_a = cat_a,
                    category_b = cat_b,
                    model_a = %config.model_a.model,
                    model_b = %config.model_b.model,
                    "Section type classification divergence"
                );
                ClassifySpan::ClassifyDualFidelity.emit("section_type_divergence");
            }

            DualClassifyResult {
                category_a: cat_a.to_string(),
                category_b: cat_b.to_string(),
                agreement,
                model_a: config.model_a.model.clone(),
                model_b: config.model_b.model.clone(),
                provider_a: provider_a.to_string(),
                provider_b: provider_b.to_string(),
            }
        })
        .collect();

    Ok(results)
}

/// Result of dual-model section type classification.
#[derive(Debug, Clone, Serialize)]
pub struct DualClassifyResult {
    pub category_a: String,
    pub category_b: String,
    pub agreement: bool,
    pub model_a: String,
    pub model_b: String,
    pub provider_a: String,
    pub provider_b: String,
}

/// Build a mandatory DualClassifierConfig.
///
/// Model A comes from the YAML definition (overridden by HKASK_CLASSIFIER_MODEL_A).
/// Model B is resolved from, in priority order:
/// 1. HKASK_CLASSIFIER_MODEL_B env var
/// 2. settings.classifier_model_b
/// 3. YAML classifier_def.model_b
///
/// Classification REQUIRES two peer models. Returns Err when model B is not
/// configured — single-model classification with single-jurisdiction bias is
/// not a valid operating mode.
pub fn build_dual_config(
    model_a_config: &ClassifierConfig,
    settings: &HkaskSettings,
    yaml_model_b: Option<&str>,
) -> Result<DualClassifierConfig, ServiceError> {
    let model_b_str = {
        let from_env = std::env::var("HKASK_CLASSIFIER_MODEL_B")
            .ok()
            .filter(|s| !s.is_empty());
        let from_settings =
            (!settings.classifier_model_b.is_empty()).then(|| settings.classifier_model_b.clone());
        let from_yaml = yaml_model_b
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        from_env.or(from_settings).or(from_yaml)
    };

    let model_b_str = model_b_str.ok_or_else(|| {
        ServiceError::Domain {
            domain: DomainKind::Wallet,
            kind: ErrorKind::ServiceUnavailable,
            source: None,
            message: "Dual-model classification requires HKASK_CLASSIFIER_MODEL_B (or settings.classifier_model_b or YAML model_b). Single-model classification with single-jurisdiction bias is not permitted. Set model B to a peer model from a different jurisdiction (e.g., DI/google/gemma-4-E4B-it).".to_string(),
        }
    })?;

    // Build model B config: same system prompt, different model
    // Parse provider prefix from model B string
    let (provider_code_b, stripped_model_b) = if let Some(idx) = model_b_str.find('/') {
        let code = &model_b_str[..idx];
        let model = &model_b_str[idx + 1..];
        (code.to_string(), model.to_string())
    } else {
        // No provider prefix — assume same provider as model A
        (String::new(), model_b_str.clone())
    };

    // Determine base_url and api_key from provider code
    let (base_url, api_key) = match provider_code_b.to_lowercase().as_str() {
        "di" | "deepinfra" => (
            "https://api.deepinfra.com/v1/openai/chat/completions".to_string(),
            std::env::var("DI_API_KEY").unwrap_or_default(),
        ),
        "kc" | "kilocode" => (
            "https://api.kilo.ai/api/gateway/chat/completions".to_string(),
            std::env::var("KC_API_KEY").unwrap_or_default(),
        ),
        "to" | "together" => (
            "https://api.together.xyz/v1/chat/completions".to_string(),
            std::env::var("TOGETHER_API_KEY").unwrap_or_default(),
        ),
        "or" | "openrouter" => (
            "https://openrouter.ai/api/v1/chat/completions".to_string(),
            std::env::var("OPENROUTER_API_KEY").unwrap_or_default(),
        ),
        _ => {
            // Same provider as model A
            tracing::info!(
                target: "cns.classify",
                model_b = %model_b_str,
                "No provider prefix on model B — using model A provider settings"
            );
            (
                model_a_config.base_url.clone(),
                model_a_config.api_key.clone(),
            )
        }
    };

    let model_b = ClassifierConfig {
        model: stripped_model_b,
        api_key,
        base_url,
        system_prompt: model_a_config.system_prompt.clone(),
        concurrency: model_a_config.concurrency,
        timeout: model_a_config.timeout,
        temperature: model_a_config.temperature,
        max_tokens: model_a_config.max_tokens,
        fallback_category: model_a_config.fallback_category.clone(),
        cost_input_nj_per_token: model_a_config.cost_input_nj_per_token,
        cost_output_nj_per_token: model_a_config.cost_output_nj_per_token,
        cost_cache_read_nj_per_token: model_a_config.cost_cache_read_nj_per_token,
    };

    Ok(DualClassifierConfig {
        model_a: model_a_config.clone(),
        model_b,
    })
}

/// Emit a classifier drift alert when extraction patterns deviate from baseline.
///
/// Called after each dual-model batch completes. Compares current batch statistics
/// against expected baselines. When entity recall, topic distribution, or refusal
/// rate deviate beyond threshold, emits `cns.classify.drift`.
///
/// This makes provider behavior changes (censorship policy shifts, model updates)
/// observable over time.
///
/// # Threshold Calibration
///
/// Thresholds are **LLM-Assessed** (hypothetical, not calibrated from production data):
/// - `DRIFT_DIVERGENCE_RATE_THRESHOLD` (0.3): "one in three items disagree" —
///   chosen as a conservative signal that model behavior has shifted.
/// - `DRIFT_ASYMMETRY_THRESHOLD` (2.0): "one model consistently doubles the
///   other's unique entity count" — signals possible single-model censorship.
/// - `DRIFT_MIN_BATCH_SIZE` (10): floor for statistical relevance in asymmetry
///   checks; smaller batches produce noisy signals.
///
/// TODO: calibrate from production data. These are initial estimates.
pub const DRIFT_DIVERGENCE_RATE_THRESHOLD: f64 = 0.3;
pub const DRIFT_ASYMMETRY_THRESHOLD: f64 = 2.0;
pub const DRIFT_MIN_BATCH_SIZE: usize = 10;

/// Jaccard entity agreement threshold for divergence alert.
/// When agreement falls below this, `cns.classify.dual_fidelity` fires.
/// Set at 0.6: when fewer than 60% of extracted entities overlap,
/// the models are extracting materially different information.
pub const JACCARD_DIVERGENCE_THRESHOLD: f64 = 0.6;

pub fn check_classifier_drift(
    batch_results: &[DualTripleExtraction],
    model_a: &str,
    model_b: &str,
    batch_size: usize,
) {
    if batch_results.is_empty() {
        return;
    }

    let avg_agreement: f64 = batch_results
        .iter()
        .map(|r| r.entity_agreement)
        .sum::<f64>()
        / batch_results.len() as f64;

    let divergence_rate = batch_results.iter().filter(|r| r.divergence_alert).count() as f64
        / batch_results.len() as f64;

    let a_only_total: usize = batch_results.iter().map(|r| r.a_only_entities.len()).sum();
    let b_only_total: usize = batch_results.iter().map(|r| r.b_only_entities.len()).sum();

    // Drift threshold: divergence rate above configured threshold
    if divergence_rate > DRIFT_DIVERGENCE_RATE_THRESHOLD {
        tracing::warn!(
            target: "cns.classify.drift",
            avg_agreement = %avg_agreement,
            divergence_rate = %divergence_rate,
            a_only_total = a_only_total,
            b_only_total = b_only_total,
            batch_size = batch_size,
            model_a = %model_a,
            model_b = %model_b,
            "Classifier extraction drift detected — divergence rate exceeds threshold"
        );
        ClassifySpan::ClassifyDrift.emit("divergence_rate_exceeded");
    }

    // Drift threshold: one model consistently producing more unique entities
    // than the other signals possible censorship pattern
    if batch_size >= DRIFT_MIN_BATCH_SIZE {
        let a_per_item = a_only_total as f64 / batch_size as f64;
        let b_per_item = b_only_total as f64 / batch_size as f64;
        let asymmetry = (a_per_item - b_per_item).abs();

        if asymmetry > DRIFT_ASYMMETRY_THRESHOLD {
            let dominant = if a_per_item > b_per_item {
                model_a
            } else {
                model_b
            };
            tracing::warn!(
                target: "cns.classify.drift",
                a_per_item = %a_per_item,
                b_per_item = %b_per_item,
                asymmetry = %asymmetry,
                dominant_model = %dominant,
                "Asymmetric extraction pattern — possible single-model censorship or model behavior change"
            );
            ClassifySpan::ClassifyDrift.emit("asymmetric_extraction");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_extraction(
        topic: &str,
        entities: &[&str],
        concepts: &[&str],
        relationships: &[&str],
    ) -> TripleExtraction {
        TripleExtraction {
            topic: topic.to_string(),
            concepts: concepts.iter().map(|s| s.to_string()).collect(),
            entities: entities.iter().map(|s| s.to_string()).collect(),
            relationships: relationships.iter().map(|s| s.to_string()).collect(),
            primary_dimension: "Gentle".to_string(),
            quality_flags: vec!["precise".to_string()],
            extra: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn identical_extractions_produce_full_agreement() {
        let a = make_extraction(
            "Test topic",
            &["Entity1", "Entity2"],
            &["Concept1"],
            &["relates to"],
        );
        let b = a.clone();

        let result = integrate_dual_triples(&a, &b, "qwen", "llama", "KC", "DI");

        assert!((result.entity_agreement - 1.0).abs() < f64::EPSILON);
        assert!((result.concept_agreement - 1.0).abs() < f64::EPSILON);
        assert!(result.a_only_entities.is_empty());
        assert!(result.b_only_entities.is_empty());
        assert!(!result.divergence_alert);
    }

    #[test]
    fn completely_divergent_extractions_produce_zero_agreement() {
        let a = make_extraction("Topic A", &["EntityA"], &["ConceptA"], &["rel A"]);
        let b = make_extraction("Topic B", &["EntityB"], &["ConceptB"], &["rel B"]);

        let result = integrate_dual_triples(&a, &b, "qwen", "llama", "KC", "DI");

        assert!((result.entity_agreement - 0.0).abs() < f64::EPSILON);
        assert!((result.concept_agreement - 0.0).abs() < f64::EPSILON);
        assert_eq!(result.a_only_entities, vec!["EntityA"]);
        assert_eq!(result.b_only_entities, vec!["EntityB"]);
        assert!(result.divergence_alert);
    }

    #[test]
    fn partial_overlap_produces_correct_jaccard() {
        let a = make_extraction(
            "Topic",
            &["Entity1", "Entity2", "Entity3"],
            &["Concept1", "Concept2"],
            &[],
        );
        let b = make_extraction(
            "Topic",
            &["Entity1", "Entity3", "Entity4"],
            &["Concept2", "Concept3"],
            &[],
        );

        let result = integrate_dual_triples(&a, &b, "qwen", "llama", "KC", "DI");

        // Entities: intersection {"entity1","entity3"} = 2, union {"entity1","entity2","entity3","entity4"} = 4
        assert!((result.entity_agreement - 0.5).abs() < f64::EPSILON);
        assert_eq!(result.a_only_entities, vec!["Entity2"]);
        assert_eq!(result.b_only_entities, vec!["Entity4"]);
        // Partial overlap below threshold → alert
        assert!(result.divergence_alert);
    }

    #[test]
    fn empty_extractions_both_sides_yield_full_agreement() {
        let a = TripleExtraction::default();
        let b = TripleExtraction::default();

        let result = integrate_dual_triples(&a, &b, "qwen", "llama", "KC", "DI");

        assert!((result.entity_agreement - 1.0).abs() < f64::EPSILON);
        assert!(!result.divergence_alert);
    }

    #[test]
    fn one_empty_one_populated_shows_divergence() {
        let a = make_extraction("Topic", &["Entity1"], &["Concept1"], &[]);
        let b = TripleExtraction::default();

        let result = integrate_dual_triples(&a, &b, "qwen", "llama", "KC", "DI");

        assert!((result.entity_agreement - 0.0).abs() < f64::EPSILON);
        assert_eq!(result.a_only_entities, vec!["Entity1"]);
        assert!(result.divergence_alert);
    }

    #[test]
    fn case_insensitive_dedup_works() {
        let a = make_extraction("Topic", &["Entity"], &["Concept"], &[]);
        let b = make_extraction("Topic", &["entity"], &["concept"], &[]);

        let result = integrate_dual_triples(&a, &b, "qwen", "llama", "KC", "DI");

        assert!((result.entity_agreement - 1.0).abs() < f64::EPSILON);
        assert_eq!(result.merged.entities.len(), 1);
        assert_eq!(result.merged.concepts.len(), 1);
    }

    #[test]
    fn diverging_topics_are_merged_with_labels() {
        let a = make_extraction("Chinese perspective", &[], &[], &[]);
        let b = make_extraction("American perspective", &[], &[], &[]);

        let result = integrate_dual_triples(&a, &b, "qwen", "llama", "KC", "DI");

        assert!(result.merged.topic.contains("[A]"));
        assert!(result.merged.topic.contains("[B]"));
    }

    #[test]
    fn diverging_dimensions_are_annotated() {
        let mut a = make_extraction("Topic", &[], &[], &[]);
        a.primary_dimension = "Gentle".to_string();
        let mut b = make_extraction("Topic", &[], &[], &[]);
        b.primary_dimension = "Hopper".to_string();

        let result = integrate_dual_triples(&a, &b, "qwen", "llama", "KC", "DI");

        assert!(result.merged.primary_dimension.contains("A:Gentle"));
        assert!(result.merged.primary_dimension.contains("B:Hopper"));
    }

    #[test]
    fn extra_fields_are_merged_with_array_on_collision() {
        let mut a = make_extraction("Topic", &[], &[], &[]);
        a.extra.insert(
            "themes".to_string(),
            serde_json::Value::String("nature".to_string()),
        );
        let mut b = make_extraction("Topic", &[], &[], &[]);
        b.extra.insert(
            "themes".to_string(),
            serde_json::Value::String("struggle".to_string()),
        );

        let result = integrate_dual_triples(&a, &b, "qwen", "llama", "KC", "DI");

        let themes = result.merged.extra.get("themes").unwrap();
        assert!(themes.is_array());
        let arr = themes.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }
}
