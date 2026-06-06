//! Template types — Loop 1 (Inference): template rendering and invocation
//
//! Templates are the primary interface for the Inference loop. The registry
//! stores them; Inference renders them; Curation evaluates their output.

use serde::{Deserialize, Serialize};

use crate::capability::DelegationToken;
use crate::id::BotID;

/// LLMParameters — Full parameter set for LLM invocation
/// Loop: Inference
///
/// Temperature is primary. Other parameters support.
/// Temperature breaks the pattern. Other parameters vary the break.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMParameters {
    /// Temperature: primary control for randomness (0.0-1.0)
    /// - Low (0.1-0.3): deterministic, optimal, normative
    /// - High (0.7-0.9): random, suboptimal, creative
    pub temperature: f32,

    /// Top-p (nucleus sampling): cumulative probability threshold (0.0-1.0)
    /// - Lower: more focused
    /// - Higher: more diverse
    pub top_p: f32,

    /// Top-k: sample from top k tokens (1-100)
    /// - Lower: safer
    /// - Higher: more surprising
    pub top_k: u32,

    /// Frequency penalty: penalize repetition (-2.0 to 2.0)
    /// - Higher: more varied vocabulary
    pub frequency_penalty: f32,

    /// Presence penalty: penalize familiar tokens (-2.0 to 2.0)
    /// - Higher: more novel concepts
    pub presence_penalty: f32,

    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// Random seed (None for random, Some for reproducibility)
    pub seed: Option<u64>,
}

impl LLMParameters {
    /// Edge work preset: moderate anti-normative parameters
    /// Temperature: 0.6, top_p: 0.85, top_k: 35, freq: 0.4, presence: 0.4
    pub(crate) fn edge_work() -> Self {
        Self {
            temperature: 0.6,
            top_p: 0.85,
            top_k: 35,
            frequency_penalty: 0.4,
            presence_penalty: 0.4,
            max_tokens: 2048,
            seed: None,
        }
    }
}

impl Default for LLMParameters {
    fn default() -> Self {
        Self::edge_work()
    }
}

/// TemplateOutcome — Result of template invocation
/// Loop: Inference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateOutcome {
    /// Template produced useful output
    Success,
    /// Template produced broken/invalid output
    Failure,
}

impl std::fmt::Display for TemplateOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateOutcome::Success => write!(f, "success"),
            TemplateOutcome::Failure => write!(f, "failure"),
        }
    }
}

/// TemplateInvocation — Record of a single template execution
/// Loop: Inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInvocation {
    pub id: TemplateID,
    pub template_id: TemplateID,
    pub bot_id: BotID,
    pub temperature: f32,
    pub parameters: LLMParameters,
    pub input: serde_json::Value,
    pub outputs: Vec<serde_json::Value>,
    pub selected_index: Option<usize>,
    pub outcome: TemplateOutcome,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Capability token authorizing this invocation (for OCAP verification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_token: Option<DelegationToken>,
}

/// Template file within a crate
/// Loop: Inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
    pub template_type: String, // WordAct, KnowAct, FlowDef
}

/// Template crate structure (loaded from Git CAS)
/// Loop: Inference
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateCrate {
    /// Crate name
    pub name: String,
    /// Git SHA (pinned version)
    pub git_sha: String,
    /// Agent persona YAML content
    pub persona_yaml: String,
    /// Dispatch manifest YAML content
    pub dispatch_manifest_yaml: String,
    /// Template files (path -> content)
    pub templates: Vec<TemplateFile>,
    /// hLexicon terms used
    pub hlexicon_terms: Vec<String>,
}

impl TemplateInvocation {
    pub fn new(
        template_id: TemplateID,
        bot_id: BotID,
        parameters: LLMParameters,
        input: serde_json::Value,
    ) -> Self {
        Self {
            id: TemplateID::new(),
            template_id,
            bot_id,
            temperature: parameters.temperature,
            parameters,
            input,
            outputs: Vec::new(),
            selected_index: None,
            outcome: TemplateOutcome::Failure,
            timestamp: chrono::Utc::now(),
            capability_token: None,
        }
    }
}
