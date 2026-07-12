//! Fusion configuration types — shared across hkask-templates, hkask-inference, and hkask-types.
//!
//! These types define the multi-model deliberation configuration:
//! - `FusionMode` — how the judge processes panel responses
//! - `FusionSkill` — methodology anchors injected into the judge's system context
//! - `FusionConfig` — the full configuration (judge, panel, mode, skills, max_rounds)
//!
//! Lives in hkask-types (not hkask-inference) so manifests and LLMParameters
//! can carry per-manifest and per-call fusion overrides without a dependency
//! on the inference crate.

use serde::{Deserialize, Serialize};

/// Judge deliberation mode for fusion orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FusionMode {
    /// Pick the single best panel response. No synthesis.
    #[serde(rename = "best-of-n")]
    BestOfN,
    /// Compose a unified response incorporating best elements from all panelists.
    #[serde(rename = "synthesis")]
    #[default]
    Synthesis,
    /// 2-round: draft → panel critique → revised final.
    #[serde(rename = "critique")]
    Critique,
    /// Multi-round deliberation with convergence check (up to max_rounds).
    #[serde(rename = "deliberation")]
    Deliberation,
    /// 2-phase Plan-Implement: Phase 1 synthesizes strategy, Phase 2 synthesizes execution plan.
    #[serde(rename = "pi")]
    PlanImplement,
}

impl FusionMode {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            FusionMode::BestOfN => "best-of-n",
            FusionMode::Synthesis => "synthesis",
            FusionMode::Critique => "critique",
            FusionMode::Deliberation => "deliberation",
            FusionMode::PlanImplement => "pi",
        }
    }
}

impl std::str::FromStr for FusionMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "best-of-n" => Ok(FusionMode::BestOfN),
            "synthesis" => Ok(FusionMode::Synthesis),
            "critique" => Ok(FusionMode::Critique),
            "deliberation" => Ok(FusionMode::Deliberation),
            "pi" => Ok(FusionMode::PlanImplement),
            _ => Ok(FusionMode::Synthesis),
        }
    }
}

/// Skill bundle that anchors the judge's reasoning framework.
/// Each skill injects a compact methodology prompt into the judge's system context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FusionSkill {
    #[serde(rename = "pragmatic-semantics")]
    PragmaticSemantics,
    #[serde(rename = "pragmatic-cybernetics")]
    PragmaticCybernetics,
    #[serde(rename = "pragmatic-laziness")]
    PragmaticLaziness,
    #[serde(rename = "coding-guidelines")]
    CodingGuidelines,
    #[serde(rename = "deep-module")]
    DeepModule,
    #[serde(rename = "essentialist")]
    Essentialist,
    #[serde(rename = "superforecasting")]
    Superforecasting,
    #[serde(rename = "mcda")]
    MCDA,
    #[serde(rename = "tdd")]
    TestDrivenDevelopment,
}

impl std::str::FromStr for FusionSkill {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "pragmatic-semantics" => Ok(FusionSkill::PragmaticSemantics),
            "pragmatic-cybernetics" => Ok(FusionSkill::PragmaticCybernetics),
            "pragmatic-laziness" => Ok(FusionSkill::PragmaticLaziness),
            "coding-guidelines" => Ok(FusionSkill::CodingGuidelines),
            "deep-module" => Ok(FusionSkill::DeepModule),
            "essentialist" => Ok(FusionSkill::Essentialist),
            "superforecasting" => Ok(FusionSkill::Superforecasting),
            "mcda" => Ok(FusionSkill::MCDA),
            "tdd" => Ok(FusionSkill::TestDrivenDevelopment),
            _ => Err(()),
        }
    }
}

/// Configuration for fusion multi-model deliberation.
///
/// Provider-agnostic: hKask orchestrates the fusion itself by sending
/// the prompt to all panel models in parallel, collecting responses,
/// then having the judge operate in the configured mode.
///
/// When carried in `LLMParameters.fusion_config`, overrides the global
/// fusion config for that specific inference call.
/// When carried in `BundleManifest.fusion`, provides a per-manifest
/// fusion config for all steps in that skill's pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionConfig {
    /// The judge/fuser model that orchestrates and synthesizes the fusion.
    /// Supports provider prefix routing (e.g., "DI/deepseek-v4-pro").
    pub judge: String,
    /// The panel of analysis models that answer in parallel.
    /// Each model supports provider prefix routing.
    pub panel: Vec<String>,
    /// Judge deliberation mode. Default: Synthesis.
    #[serde(default)]
    pub mode: FusionMode,
    /// Skills that anchor the judge's reasoning framework.
    /// Default: empty (no skill anchoring).
    #[serde(default)]
    pub skills: Vec<FusionSkill>,
    /// Max rounds for deliberation mode. Default: 5.
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u32,
}

fn default_max_rounds() -> u32 {
    5
}

impl FusionConfig {
    /// Return the kask default fusion configuration.
    ///
    /// Reads judge model from `HKASK_FUSION_JUDGE` (default: `"deepseek-v4-pro"`)
    /// and panel models from `HKASK_FUSION_PANEL` (comma-separated,
    /// default: `"Kimi2.7,Qwen3.7 Max,GLM5.2,Minimax3"`).
    pub fn kask_default() -> Self {
        let judge =
            std::env::var("HKASK_FUSION_JUDGE").unwrap_or_else(|_| "deepseek-v4-pro".to_string());
        let panel = std::env::var("HKASK_FUSION_PANEL")
            .unwrap_or_else(|_| "Kimi2.7,Qwen3.7 Max,GLM5.2,Minimax3".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            judge,
            panel,
            mode: FusionMode::Synthesis,
            skills: Vec::new(),
            max_rounds: 5,
        }
    }

    /// The model ID to use when fusion is active (judge model).
    #[must_use]
    pub fn model_id(&self) -> String {
        self.judge.clone()
    }
}

impl FusionSkill {
    /// Human-readable string representation for display.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            FusionSkill::PragmaticSemantics => "pragmatic-semantics",
            FusionSkill::PragmaticCybernetics => "pragmatic-cybernetics",
            FusionSkill::PragmaticLaziness => "pragmatic-laziness",
            FusionSkill::CodingGuidelines => "coding-guidelines",
            FusionSkill::DeepModule => "deep-module",
            FusionSkill::Essentialist => "essentialist",
            FusionSkill::Superforecasting => "superforecasting",
            FusionSkill::MCDA => "mcda",
            FusionSkill::TestDrivenDevelopment => "tdd",
        }
    }
}

impl FusionConfig {
    /// Human-readable description of the fusion setup.
    #[must_use]
    pub fn description(&self) -> String {
        let skills_str = if self.skills.is_empty() {
            String::new()
        } else {
            let names: Vec<&str> = self.skills.iter().map(FusionSkill::as_str).collect();
            format!(" [{}]", names.join(", "))
        };
        format!(
            "{} panel models judged by {} (mode: {}){}",
            self.panel.len(),
            self.judge,
            self.mode.as_str(),
            skills_str
        )
    }
}
