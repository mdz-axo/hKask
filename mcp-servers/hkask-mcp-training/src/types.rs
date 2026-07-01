//! Request types for the Training MCP server — all tool input structs and their supporting types.
//!
//! Twenty tools: ingest_qa, submit, status, cancel, delete_adapter, assemble_dataset,
//! generate_traces, evaluate, register_adapter, recommend_model, record_invocation,
//! curate_feedback, retrain, ingest_dataset, sweep, generate_chain_of_thought,
//! merge_adapters, deploy, deployment_status, teardown.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use chrono;
use hkask_adapter::{EndpointLifecycle, EndpointPhase};

use crate::providers::TrainingParams;
use hkask_inference::ProviderId;
use hkask_types::template::LLMParameters;

// ── Data generation config ───────────────────────────────────────────────

/// Sampling configuration for training data generation (traces, CoT, contrastive).
///
/// Controls how the LLM produces training examples — temperature, diversity,
/// and output length. This is a data-generation concern, not a training concern.
/// The `TrainingParams → HarnessAdapter → TrainingHost` path is unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TraceGenerationConfig {
    /// Temperature for sampling (default: 0.7).
    #[serde(default = "default_gen_temperature")]
    pub temperature: f32,
    /// Nucleus sampling threshold (default: 0.95).
    #[serde(default = "default_gen_top_p")]
    pub top_p: f32,
    /// Top-k sampling (default: 50).
    #[serde(default = "default_gen_top_k")]
    pub top_k: u32,
    /// Frequency penalty — higher reduces repetition (default: 0.3).
    #[serde(default = "default_gen_frequency_penalty")]
    pub frequency_penalty: f32,
    /// Maximum new tokens to generate (default: 4096).
    #[serde(default = "default_gen_max_tokens")]
    pub max_new_tokens: u32,
    /// Per-Bloom-level overrides. Keys: "remembering", "understanding",
    /// "applying", "analyzing", "evaluating", "creating".
    /// When set, traces targeting that level use these overrides.
    #[serde(default)]
    pub bloom_level_configs: Option<HashMap<String, TraceGenerationConfig>>,
}

fn default_gen_temperature() -> f32 {
    0.7
}
fn default_gen_top_p() -> f32 {
    0.95
}
fn default_gen_top_k() -> u32 {
    50
}
fn default_gen_frequency_penalty() -> f32 {
    0.3
}
fn default_gen_max_tokens() -> u32 {
    4096
}

impl Default for TraceGenerationConfig {
    fn default() -> Self {
        Self {
            temperature: default_gen_temperature(),
            top_p: default_gen_top_p(),
            top_k: default_gen_top_k(),
            frequency_penalty: default_gen_frequency_penalty(),
            max_new_tokens: default_gen_max_tokens(),
            bloom_level_configs: None,
        }
    }
}

impl TraceGenerationConfig {
    /// Convert to LLMParameters for the inference engine.
    pub fn to_llm_params(&self) -> LLMParameters {
        LLMParameters {
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            frequency_penalty: self.frequency_penalty,
            max_tokens: self.max_new_tokens,
            ..Default::default()
        }
    }
}

// ── Trace type — word-act, flow-def, know-act ────────────────────────────

/// Skill decomposition trace type.
///
/// Each skill document produces traces of one (or more) of these types.
/// The auto-detector counts vocabulary terms per category — highest density wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TraceType {
    /// Persona calibration — "how to sound".
    /// Structure: {context, persona_constraints, target_utterance, calibration_notes}
    WordAct,
    /// Procedural decomposition — "how to think".
    /// Structure: {situation, decomposition_sequence, synthesis, verification}
    FlowDef,
    /// Pattern recognition — "how to classify".
    /// Structure: {pattern_exemplar, positive_cases[], negative_cases[], decision_boundary}
    KnowAct,
    /// Alternating WordAct/FlowDef segments for skills that require both.
    Composite,
}

impl TraceType {
    /// Auto-detect trace type from skill document text by counting vocabulary terms.
    pub fn detect(skill_text: &str) -> Self {
        let text_lower = skill_text.to_lowercase();

        let wordact_hits = [
            "persona",
            "tone",
            "voice",
            "utter",
            "speak",
            "sound like",
            "posture",
            "calibrat",
            "dialogue",
            "conversation",
            "replicant",
            "socratic",
        ]
        .iter()
        .filter(|t| text_lower.contains(*t))
        .count();

        let flowdef_hits = [
            "procedure",
            "decomposition",
            "step",
            "sequence",
            "transform",
            "pipeline",
            "verify",
            "validate",
            "check",
            "process",
            "situation",
            "synthesis",
            "decompose",
        ]
        .iter()
        .filter(|t| text_lower.contains(*t))
        .count();

        let knowact_hits = [
            "pattern",
            "classify",
            "recognize",
            "identify",
            "exemplar",
            "misclassification",
            "decision boundary",
            "category",
            "taxonomy",
            "positive case",
            "negative case",
            "rule",
        ]
        .iter()
        .filter(|t| text_lower.contains(*t))
        .count();

        if wordact_hits > flowdef_hits && wordact_hits > knowact_hits {
            TraceType::WordAct
        } else if knowact_hits > flowdef_hits {
            TraceType::KnowAct
        } else if flowdef_hits > 0 {
            TraceType::FlowDef
        } else if wordact_hits > 0 && flowdef_hits > 0 {
            TraceType::Composite
        } else {
            TraceType::FlowDef // default for most skills
        }
    }
}

// ── Deployment provider ─────────────────────────────────────────────────

/// Cloud provider for adapter deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentProvider {
    /// Together AI — fine-tuned models auto-deployed. ~30s setup.
    Together,
    /// Baseten — multi-LoRA serving on a single base model. ~5min setup.
    Baseten,
    /// Runpod — GPU pod with adapter weights mounted. ~3min setup.
    Runpod,
}

impl DeploymentProvider {
    /// Map to `hkask_inference::ProviderId` for use with `hkask-adapter::AdapterRouter`.
    pub fn as_provider_id(&self) -> ProviderId {
        match self {
            DeploymentProvider::Together => ProviderId::Together,
            DeploymentProvider::Baseten => ProviderId::Baseten,
            DeploymentProvider::Runpod => ProviderId::Runpod,
        }
    }

    /// Estimated setup time in seconds.
    pub fn setup_seconds(&self) -> u64 {
        match self {
            DeploymentProvider::Together => 30,
            DeploymentProvider::Baseten => 300,
            DeploymentProvider::Runpod => 180,
        }
    }

    /// Estimated cost per hour in USD.
    pub fn cost_per_hour(&self, gpu: Option<&str>) -> f32 {
        match self {
            DeploymentProvider::Together => 0.0, // included in fine-tune pricing
            DeploymentProvider::Baseten => match gpu.unwrap_or("H100") {
                "H100" => 3.50,
                "A100" => 2.50,
                _ => 1.50,
            },
            DeploymentProvider::Runpod => match gpu.unwrap_or("RTX 4090") {
                "A100" => 1.99,
                "H100" => 2.99,
                _ => 0.79,
            },
        }
    }
}

// ── Request helpers ─────────────────────────────────────────────────────

/// Parameter sweep space — each field is a list of values to try.
/// All combinations (cartesian product) are submitted as separate jobs.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParamSweep {
    /// Learning rates to try.
    pub learning_rates: Vec<f32>,
    /// LoRA ranks to try.
    pub lora_ranks: Vec<u32>,
    /// Batch sizes to try.
    pub batch_sizes: Vec<u32>,
    /// Number of epochs to try.
    pub num_epochs: Vec<u32>,
}

// ── Request helpers ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QaItem {
    pub question: String,
    pub answer: String,
    #[serde(default)]
    pub bloom_level: Option<String>,
}

// ── Request structs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IngestQaRequest {
    /// QA pairs to ingest for training.
    pub qa_items: Vec<QaItem>,
    /// Source document or dataset identifier.
    pub source: String,
    /// Optional training dataset name (default: "default").
    #[serde(default)]
    pub dataset: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainSubmitRequest {
    /// Path to the training dataset file.
    pub dataset_path: String,
    /// Base model to fine-tune (provider-prefixed, e.g., "OM/qwen3:8b").
    pub base_model: String,
    /// Optional training hyperparameters. Uses defaults if not provided.
    #[serde(default)]
    pub params: Option<TrainingParams>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainStatusRequest {
    /// Job ID from a previous `training_submit` call.
    pub job_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainCancelRequest {
    /// Job ID to cancel.
    pub job_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainDeleteAdapterRequest {
    /// Adapter ID to delete.
    pub adapter_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AssembleDatasetRequest {
    /// Training dataset name to filter by (matches QA pairs ingested with this dataset).
    #[serde(default)]
    pub dataset: Option<String>,
    /// Source identifier to filter by.
    #[serde(default)]
    pub source: Option<String>,
    /// Bloom level to filter by (e.g., "remembering", "applying").
    #[serde(default)]
    pub bloom_level: Option<String>,
    /// Path to write the assembled ChatML JSONL file.
    pub output_path: String,
    /// Fraction of examples to reserve for training (default 1.0 = all train, no test split).
    /// Set to 0.8 for an 80/20 train/test split. Test file is written to {output_path}.test.jsonl.
    #[serde(default)]
    pub train_split: Option<f64>,
    /// Maximum number of examples to include (default: all matching).
    #[serde(default)]
    pub max_examples: Option<usize>,
    /// Optional system prompt to prepend to each assembled conversation.
    /// Sets agent persona/context for fine-tuning (e.g., "You are an hKask agent trained in constraint classification.").
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateTracesRequest {
    /// Path to the skill document (SKILL.md) or inline text describing the process.
    pub skill_document: String,
    /// Name of the skill for output tracking.
    pub skill_name: String,
    /// Number of decomposition traces to generate (default 50).
    #[serde(default)]
    pub num_traces: Option<usize>,
    /// Trace type — WordAct (persona), FlowDef (procedure), KnowAct (classification),
    /// or Composite (mixed). Default: auto-detected from skill document content.
    #[serde(default)]
    pub trace_type: Option<TraceType>,
    /// Bloom taxonomy levels to target (e.g., ["applying", "analyzing"]).
    /// Default: all levels.
    #[serde(default)]
    pub bloom_levels: Option<Vec<String>>,
    /// Path to write the generated ChatML JSONL file.
    pub output_path: String,
    /// Optional system prompt to prepend to each trace (sets agent persona/context).
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Model to use for trace generation (provider-prefixed, e.g., "DI/meta-llama/Llama-3.3-70B-Instruct").
    /// Defaults to the server's configured default model.
    #[serde(default)]
    pub model: Option<String>,
    /// Sampling configuration for trace generation (temperature, top_p, top_k, etc.).
    /// Default: temperature=0.7, top_p=0.95, top_k=50, frequency_penalty=0.3, max_new_tokens=4096.
    #[serde(default)]
    pub generation_config: Option<TraceGenerationConfig>,
    /// Generate contrastive trace pairs (chosen + rejected) instead of single traces.
    /// Each pair has the same situation with a correct decomposition and an intentionally
    /// incorrect one. Used for ContrastiveTrace training mode.
    #[serde(default)]
    pub contrastive: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainEvaluateRequest {
    /// Adapter ID or fine-tuned model name to evaluate.
    pub adapter_id: String,
    /// Path to test dataset (ChatML JSONL). Each line must have a "messages" array
    /// with user/assistant turns. The last assistant message is the expected answer.
    pub test_dataset_path: String,
    /// Model identifier to run evaluation against (provider-prefixed).
    /// For Together AI adapters, use the fine-tuned model name
    /// (e.g., "mdz-axolotl/Qwen3.5-9B-ft-abc123").
    pub model: String,
    /// Evaluation method: "exact_match" (default), "contains", or "semantic".
    /// - exact_match: generated == expected after trimming
    /// - contains: expected substring is found in generated
    /// - semantic: uses a second inference call to judge correctness
    #[serde(default)]
    pub method: Option<String>,
    /// Maximum number of examples to evaluate (default: all).
    #[serde(default)]
    pub max_examples: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainRegisterAdapterRequest {
    /// Adapter ID (from training job completion).
    pub adapter_id: String,
    /// Human-readable name for the adapter (e.g., "pragmatic-semantics-v1").
    pub name: String,
    /// Skill name this adapter serves (e.g., "pragmatic-semantics").
    /// Enables adapter-to-skill mapping for the registry.
    pub skill_name: String,
    /// Base model the adapter was trained on (provider-prefixed).
    pub base_model: String,
    /// Content hash of the training dataset.
    #[serde(default)]
    pub dataset_hash: Option<String>,
    /// ID of the originating training job.
    #[serde(default)]
    pub training_job_id: Option<String>,
    /// Size of adapter weights in bytes.
    #[serde(default)]
    pub size_bytes: Option<u64>,
    /// Final training loss.
    #[serde(default)]
    pub loss: Option<f32>,
    /// Perplexity at end of training.
    #[serde(default)]
    pub perplexity: Option<f32>,
    /// Training duration in seconds.
    #[serde(default)]
    pub training_duration_secs: Option<u64>,
    /// Number of tokens processed.
    #[serde(default)]
    pub tokens_processed: Option<u64>,
    /// Adapter version number (default: 1). Increment on retraining.
    #[serde(default)]
    pub version: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainRecommendModelRequest {
    /// Task type: "classification", "generation", "procedural", "reasoning", or "chat".
    pub task_type: String,
    /// Budget constraint: "low" (<$1/run), "medium" (<$10/run), or "high" (unlimited).
    #[serde(default)]
    pub budget: Option<String>,
    /// Latency requirement: "realtime" (<2s), "batch" (minutes ok), or "flexible".
    #[serde(default)]
    pub latency: Option<String>,
    /// License requirement: "apache2", "mit", "open", or "any".
    #[serde(default)]
    pub license: Option<String>,
    /// Preferred provider: "together", "deepinfra", "openrouter", or "any".
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainRecordInvocationRequest {
    /// Adapter ID that was used.
    pub adapter_id: String,
    /// Skill name that was invoked.
    pub skill_name: String,
    /// Summary of the user's input/query.
    pub input_summary: String,
    /// Summary of the adapter's output/response.
    pub output_summary: String,
    /// CNS span identifier for correlation (e.g., "cns.training.invoke.pragmatic-semantics").
    #[serde(default)]
    pub cns_span: Option<String>,
    /// Confidence score for the invocation (0.0–1.0).
    #[serde(default)]
    pub confidence: Option<f64>,
    /// Whether the invocation was successful (default: true).
    #[serde(default)]
    pub success: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainCurateFeedbackRequest {
    /// Dataset name to filter QA pairs by.
    #[serde(default)]
    pub dataset: Option<String>,
    /// Source identifier to filter by.
    #[serde(default)]
    pub source: Option<String>,
    /// Path to write the corrected ChatML JSONL feedback file.
    pub output_path: String,
    /// Model to use for validation/correction (provider-prefixed).
    /// Defaults to the server's configured default model.
    #[serde(default)]
    pub model: Option<String>,
    /// Maximum number of QA pairs to review (default: 50).
    #[serde(default)]
    pub max_pairs: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainRetrainRequest {
    /// Path to the original training dataset.
    pub original_dataset_path: String,
    /// Path to the feedback JSONL file (from training_curate_feedback).
    pub feedback_path: String,
    /// Base model to fine-tune (provider-prefixed).
    pub base_model: String,
    /// Adapter name for the new version (e.g., "pragmatic-semantics-v2").
    pub adapter_name: String,
    /// Skill name for the adapter registry.
    pub skill_name: String,
    /// Optional training hyperparameters. Uses defaults if not provided.
    #[serde(default)]
    pub params: Option<TrainingParams>,
    /// Path to write the merged dataset (default: auto-generated in cache dir).
    #[serde(default)]
    pub merged_output_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainIngestDatasetRequest {
    /// Path to the raw dataset file (JSONL, JSON, or TXT).
    pub dataset_path: String,
    /// Optional cache directory override (default: server's configured cache dir).
    #[serde(default)]
    pub cache_dir: Option<String>,
}

// ── Chain-of-Thought trace generation ─────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateChainOfThoughtRequest {
    /// Path to the skill document (SKILL.md) or inline text.
    pub skill_document: String,
    /// Name of the skill for output tracking.
    pub skill_name: String,
    /// Number of CoT traces to generate (default 20).
    #[serde(default)]
    pub num_traces: Option<usize>,
    /// Number of reasoning steps per trace (default 3).
    #[serde(default)]
    pub num_steps: Option<usize>,
    /// Path to write the generated ChatML JSONL file.
    pub output_path: String,
    /// Optional system prompt.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Model to use for generation (provider-prefixed).
    #[serde(default)]
    pub model: Option<String>,
    /// Sampling configuration.
    #[serde(default)]
    pub generation_config: Option<TraceGenerationConfig>,
}

// ── LoRA adapter merge ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MergeAdaptersRequest {
    /// List of adapter IDs to merge (first is base, rest are addends).
    pub adapter_ids: Vec<String>,
    /// Name for the merged adapter (e.g., "pragmatic-semantics+essentialist").
    pub merged_name: String,
    /// Skill name for the adapter registry.
    pub skill_name: String,
    /// Optional weights for each adapter (default: equal weighting).
    /// Must match length of adapter_ids.
    #[serde(default)]
    pub weights: Option<Vec<f32>>,
    /// Density factor for TIES merging (0.0–1.0, default: 0.5).
    #[serde(default)]
    pub density: Option<f32>,
}

// ── Sweep / Deploy / Teardown ─────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainSweepRequest {
    /// Path to the training dataset file.
    pub dataset_path: String,
    /// Base model to fine-tune (provider-prefixed, e.g., "OM/qwen3:8b").
    pub base_model: String,
    /// Parameter values to sweep over. Each field is a Vec of values to try.
    /// All combinations are submitted as separate jobs (cartesian product).
    pub sweep: ParamSweep,
    /// Maximum concurrent training jobs (default: 2).
    #[serde(default)]
    pub max_concurrent: Option<usize>,
    /// Skill name for adapter naming (e.g., "pragmatic-semantics").
    #[serde(default)]
    pub skill_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainDeployRequest {
    /// Adapter ID or skill/expertise name to deploy (e.g., "pragmatic-semantics-v1").
    pub adapter_name: String,
    /// Cloud inference provider for deployment.
    pub provider: DeploymentProvider,
    /// Base model the adapter was trained on. Auto-resolved from AdapterStore if omitted.
    #[serde(default)]
    pub base_model: Option<String>,
    /// GPU type preference (e.g., "A100", "H100", "RTX 4090"). Provider-specific.
    #[serde(default)]
    pub gpu_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainTeardownRequest {
    /// Deployment ID from a previous training_deploy call.
    pub deployment_id: String,
}

// ── Training mode — expertise vs skill vs contrastive ──────────────────

/// What kind of training data is being produced.
///
/// **Expertise** — "What to know" — factual domain knowledge.
/// Training data is QA pairs (ingest_qa → assemble_dataset).
/// Evaluation uses exact/contains/semantic match.
/// Produces an *expertise adapter* that answers factual questions about a domain.
///
/// **Decomposition Trace** — "How to think" — procedural decomposition of problems.
/// Training data is generated traces from SKILL.md (generate_traces).
/// Evaluation uses decomposition accuracy.
/// Produces a *skill adapter* that applies a methodology to novel situations.
///
/// **Contrastive Trace** — "What to prefer" — trains judgment by contrasting correct vs. incorrect decompositions.
/// Training data is trace pairs (chosen/rejected) with the same situation.
/// Evaluation uses preference accuracy (does model produce chosen over rejected?).
/// Uses the existing A/B evaluation loop for comparing adapter outputs.
///
/// **Hybrid** — Both expertise and skill traces, with configurable weighting (default 30% expertise / 70% traces).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrainingMode {
    /// Expertise (factual knowledge) fine-tuning — domain QA pairs.
    Expertise,
    /// Skill (procedural) decomposition trace fine-tuning — from SKILL.md.
    DecompositionTrace,
    /// Contrastive preference training — correct vs. incorrect trace pairs.
    ContrastiveTrace,
    /// Weighted combination of expertise QA and skill decomposition traces.
    Hybrid,
}

// ── A/B evaluation baseline ──────────────────────────────────────────────

/// Metrics from the previous adapter version, used as baseline for A/B comparison
/// when retraining. The new adapter must improve on at least 2 of 3 metrics
/// (loss, perplexity, or eval accuracy) to be promoted.
#[derive(Debug, Clone, Serialize)]
pub struct AbBaseline {
    pub previous_version: u32,
    pub previous_loss: f32,
    pub previous_perplexity: f32,
}

/// A deployed adapter endpoint — tracks the lifecycle of a trained adapter
/// that has been deployed to a cloud inference provider.
///
/// Uses `EndpointLifecycle` for state machine governance:
///   Provisioning → Ready → Active → Draining → Terminated
#[derive(Debug, Clone, Serialize)]
pub struct AdapterDeployment {
    pub deployment_id: String,
    pub adapter_name: String,
    pub base_model: String,
    pub provider: DeploymentProvider,
    pub endpoint_url: Option<String>,
    /// Lifecycle state machine — governs phase transitions.
    #[serde(skip)]
    pub lifecycle: EndpointLifecycle,
    pub estimated_cost_per_hour: f32,
    pub deployed_at: chrono::DateTime<chrono::Utc>,
}

impl AdapterDeployment {
    /// Current phase from the lifecycle state machine.
    pub fn phase(&self) -> EndpointPhase {
        self.lifecycle.phase
    }

    /// Accrued cost from the lifecycle.
    pub fn cost_accrued(&self) -> f64 {
        self.lifecycle.cost_accrued
    }
}
