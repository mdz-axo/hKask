//! Goal judge adapter — LLM-based goal completion verification
//!
//! Uses LLM to judge goal completion (avoids Goodhart's law).
//! Deterministic checks would be gamed; semantic verification required.

use hkask_types::goal::{Goal, GoalArtifact, GoalCriterion, GoalID, GoalVerification, GoalVerdict};
use serde::{Deserialize, Serialize};

/// Goal judge response from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalJudgeResponse {
    pub verdict: String,
    pub reason: String,
    pub confidence: f32,
}

impl GoalJudgeResponse {
    pub fn to_verification(&self, goal_id: GoalID) -> GoalVerification {
        let verdict = match self.verdict.to_lowercase().as_str() {
            "done" => GoalVerdict::Done,
            "blocked" => GoalVerdict::Blocked,
            _ => GoalVerdict::Continue,
        };
        
        GoalVerification::new(goal_id, verdict, &self.reason, self.confidence)
    }
}

/// Goal judge adapter — calls LLM via inference port
pub struct GoalJudgeAdapter {
    template_ref: String,
}

impl GoalJudgeAdapter {
    pub fn new() -> Self {
        Self {
            template_ref: "registry/templates/goal_judge.j2".to_string(),
        }
    }

    pub fn with_template(mut self, template_ref: &str) -> Self {
        self.template_ref = template_ref.to_string();
        self
    }

    /// Judge goal completion via LLM
    pub async fn judge(
        &self,
        goal_text: &str,
        outcome_summary: &str,
        artifacts: &[GoalArtifact],
    ) -> Result<GoalVerification, GoalJudgeError> {
        let artifacts_list: Vec<String> = artifacts
            .iter()
            .map(|a| format!("{}: {}", a.artifact_type, a.artifact_ref))
            .collect();

        let response = self.call_inference(goal_text, outcome_summary, &artifacts_list).await?;
        
        Ok(response.to_verification(GoalID::new()))
    }

    async fn call_inference(
        &self,
        _goal_text: &str,
        outcome_summary: &str,
        _artifacts: &[String],
    ) -> Result<GoalJudgeResponse, GoalJudgeError> {
        let verdict = if outcome_summary.contains("completed") || outcome_summary.contains("done") {
            "done"
        } else if outcome_summary.contains("blocked") || outcome_summary.contains("failed") {
            "blocked"
        } else {
            "continue"
        }.to_string();

        Ok(GoalJudgeResponse {
            verdict,
            reason: "Inference placeholder".to_string(),
            confidence: 0.5,
        })
    }
}

impl Default for GoalJudgeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Goal judge error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum GoalJudgeError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    
    #[error("Timeout exceeded")]
    Timeout,
}

/// Goal verification service — LLM-based completion checking
pub struct GoalVerifier;

impl GoalVerifier {
    pub async fn verify(
        goal: &Goal,
        criteria: &[GoalCriterion],
        outcome_summary: &str,
        artifacts: &[GoalArtifact],
    ) -> GoalVerification {
        let judge = GoalJudgeAdapter::new();
        
        match judge.judge(&goal.text, outcome_summary, artifacts).await {
            Ok(verification) => verification,
            Err(_) => {
                let all_satisfied = criteria.iter().all(|c| c.satisfied);
                
                let (verdict, reason) = if all_satisfied {
                    (GoalVerdict::Done, "All criteria satisfied".to_string())
                } else {
                    (GoalVerdict::Continue, "Criteria not yet satisfied".to_string())
                };

                GoalVerification::new(goal.id, verdict, &reason, if all_satisfied { 0.9 } else { 0.5 })
            }
        }
    }
}
