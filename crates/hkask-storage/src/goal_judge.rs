//! Goal judge adapter — LLM-based goal completion verification
//!
//! Uses LLM to judge goal completion (avoids Goodhart's law).
//! Deterministic checks would be gamed; semantic verification required.

use hkask_types::goal::{Goal, GoalArtifact, GoalCriterion, GoalID, GoalVerification, GoalVerdict};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    /// 
    /// # Arguments
    /// * `goal_text` — The original goal statement
    /// * `outcome_summary` — Summary of what the agent accomplished
    /// * `artifacts` — List of artifacts produced
    /// 
    /// # Returns
    /// * `GoalVerification` — Verdict, reason, and confidence
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
        // Placeholder for actual inference call
        // In production, this would:
        // 1. Load goal_judge.j2 template from registry
        // 2. Populate with goal_text, outcome_summary, artifacts
        // 3. Call LLM via hkask-mcp-inference or InferencePort
        // 4. Parse JSON response
        
        // For now, return a simple heuristic-based response
        let verdict = if outcome_summary.contains("completed") 
            || outcome_summary.contains("done") 
            || outcome_summary.contains("finished")
            || outcome_summary.contains("accomplished") {
            "done"
        } else if outcome_summary.contains("blocked") 
            || outcome_summary.contains("failed") 
            || outcome_summary.contains("error")
            || outcome_summary.contains("unable") {
            "blocked"
        } else {
            "continue"
        }.to_string();

        Ok(GoalJudgeResponse {
            verdict,
            reason: "Heuristic-based judgment (LLM inference not configured)".to_string(),
            confidence: 0.5,
        })
    }

    /// Call actual inference port with goal_judge template
    pub async fn judge_with_inference<T: InferencePort>(
        &self,
        inference_port: &T,
        goal_text: &str,
        outcome_summary: &str,
        artifacts: &[GoalArtifact],
    ) -> Result<GoalVerification, GoalJudgeError> {
        use hkask_types::TemplateInvocation;
        
        // Build invocation for goal_judge.j2 template
        let artifacts_list: Vec<Value> = artifacts
            .iter()
            .map(|a| serde_json::json!({
                "type": a.artifact_type,
                "ref": a.artifact_ref,
            }))
            .collect();

        let invocation = TemplateInvocation::new(
            self.template_ref.clone(),
            serde_json::json!({
                "goal_text": goal_text,
                "outcome_summary": outcome_summary,
                "artifacts": artifacts_list,
            }),
        );

        // Call inference port
        let response = inference_port.invoke(invocation).await
            .map_err(|e| GoalJudgeError::InferenceFailed(e.to_string()))?;

        // Parse response as GoalJudgeResponse
        let judge_response: GoalJudgeResponse = serde_json::from_value(response.output)
            .map_err(|e| GoalJudgeError::InvalidResponse(e.to_string()))?;

        Ok(judge_response.to_verification(GoalID::new()))
    }
}

impl Default for GoalJudgeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Inference port trait for goal judge
#[async_trait::async_trait]
pub trait InferencePort {
    type Error: std::error::Error;
    
    async fn invoke(&self, invocation: TemplateInvocation) -> Result<TemplateOutcome, Self::Error>;
}

use hkask_types::TemplateOutcome;

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
                // Fallback to simple criteria check
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

    /// Verify goal using actual LLM inference
    pub async fn verify_with_inference<T: InferencePort>(
        goal: &Goal,
        criteria: &[GoalCriterion],
        outcome_summary: &str,
        artifacts: &[GoalArtifact],
        inference_port: &T,
    ) -> GoalVerification {
        let judge = GoalJudgeAdapter::new();
        
        match judge.judge_with_inference(inference_port, &goal.text, outcome_summary, artifacts).await {
            Ok(verification) => verification,
            Err(_) => {
                // Fallback to simple criteria check
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
