//! Template types for hKask high-temperature templates

use serde::{Deserialize, Serialize};

use crate::capability::CapabilityToken;
use crate::id::BotID;

pub type TemplateId = crate::id::TemplateID;

/// LLMParameters — Full parameter set for LLM invocation
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
    pub fn new(
        temperature: f32,
        top_p: f32,
        top_k: u32,
        frequency_penalty: f32,
        presence_penalty: f32,
        max_tokens: u32,
        seed: Option<u64>,
    ) -> Self {
        Self {
            temperature: temperature.clamp(0.0, 1.0),
            top_p: top_p.clamp(0.0, 1.0),
            top_k: top_k.clamp(1, 100),
            frequency_penalty: frequency_penalty.clamp(-2.0, 2.0),
            presence_penalty: presence_penalty.clamp(-2.0, 2.0),
            max_tokens,
            seed,
        }
    }

    /// Anti-inferno preset: maximum anti-normative parameters
    /// Temperature: 0.95, top_p: 0.65, top_k: 15, freq: 0.8, presence: 0.8
    pub fn anti_inferno() -> Self {
        Self {
            temperature: 0.95,
            top_p: 0.65,
            top_k: 15,
            frequency_penalty: 0.8,
            presence_penalty: 0.8,
            max_tokens: 2048,
            seed: None,
        }
    }

    /// Edge work preset: moderate anti-normative parameters
    /// Temperature: 0.6, top_p: 0.85, top_k: 35, freq: 0.4, presence: 0.4
    pub fn edge_work() -> Self {
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

    /// Clean place preset: stable parameters for production
    /// Temperature: 0.2, top_p: 0.95, top_k: 80
    pub fn clean_place() -> Self {
        Self {
            temperature: 0.2,
            top_p: 0.95,
            top_k: 80,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateOutcome {
    /// Template produced useful output
    Success,
    /// Template produced broken/invalid output
    Failure,
    /// Template output was merged with other outputs
    Merged,
    /// Template output was discarded by Curator
    Discarded,
}

impl std::fmt::Display for TemplateOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateOutcome::Success => write!(f, "success"),
            TemplateOutcome::Failure => write!(f, "failure"),
            TemplateOutcome::Merged => write!(f, "merged"),
            TemplateOutcome::Discarded => write!(f, "discarded"),
        }
    }
}

/// TemplateInvocation — Record of a single template execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInvocation {
    pub id: TemplateId,
    pub template_id: TemplateId,
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
    pub capability_token: Option<CapabilityToken>,
}

/// Template file within a crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
    pub template_type: String, // Prompt, Process, Cognition
}

/// Template crate structure (loaded from Git CAS)
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
        template_id: TemplateId,
        bot_id: BotID,
        parameters: LLMParameters,
        input: serde_json::Value,
    ) -> Self {
        Self {
            id: TemplateId::new(),
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

    /// Create a new invocation with a capability token for OCAP verification
    pub fn with_capability_token(
        template_id: TemplateId,
        bot_id: BotID,
        parameters: LLMParameters,
        input: serde_json::Value,
        token: CapabilityToken,
    ) -> Self {
        Self {
            id: TemplateId::new(),
            template_id,
            bot_id,
            temperature: parameters.temperature,
            parameters,
            input,
            outputs: Vec::new(),
            selected_index: None,
            outcome: TemplateOutcome::Failure,
            timestamp: chrono::Utc::now(),
            capability_token: Some(token),
        }
    }
}
