//! Pipeline state checkpointing — platform-level utility for FlowDef execution.
//!
//! Tracks which pipeline steps have completed and their outputs.
//! Enables resume-after-failure without re-running completed steps.
//!
//! Used by replica server pipeline tools and the FlowDef execution engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// State of a single pipeline step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepState {
    pub status: String, // "complete", "failed", "running"
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub output: serde_json::Value,
}

/// Full pipeline state — persisted as JSON for checkpoint/resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub pipeline_id: String,
    pub version: String,
    pub started_at: Option<String>,
    pub updated_at: String,
    pub steps: HashMap<String, StepState>,
}

impl PipelineState {
    /// Load pipeline state from a checkpoint file, or create a new one.
    pub fn load_or_create(pipeline_id: &str, version: &str, state_path: &Path) -> Self {
        if let Ok(content) = std::fs::read_to_string(state_path)
            && let Ok(state) = serde_json::from_str(&content)
        {
            return state;
        }
        Self {
            pipeline_id: pipeline_id.to_string(),
            version: version.to_string(),
            started_at: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
            steps: HashMap::new(),
        }
    }

    /// Check if a step has already completed successfully.
    pub fn is_complete(&self, step_id: &str) -> bool {
        self.steps
            .get(step_id)
            .map(|s| s.status == "complete")
            .unwrap_or(false)
    }

    /// Mark a step as started.
    pub fn mark_started(&mut self, step_id: &str) {
        self.steps.insert(
            step_id.to_string(),
            StepState {
                status: "running".to_string(),
                started_at: Some(chrono::Utc::now().to_rfc3339()),
                completed_at: None,
                output: serde_json::Value::Null,
            },
        );
    }

    /// Mark a step as complete with output data.
    pub fn mark_complete(&mut self, step_id: &str, output: serde_json::Value) {
        let now = chrono::Utc::now().to_rfc3339();
        self.steps.insert(
            step_id.to_string(),
            StepState {
                status: "complete".to_string(),
                started_at: self.steps.get(step_id).and_then(|s| s.started_at.clone()),
                completed_at: Some(now.clone()),
                output,
            },
        );
        self.updated_at = now;
    }

    /// Mark a step as failed.
    pub fn mark_failed(&mut self, step_id: &str, error: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        self.steps.insert(
            step_id.to_string(),
            StepState {
                status: "failed".to_string(),
                started_at: self.steps.get(step_id).and_then(|s| s.started_at.clone()),
                completed_at: Some(now.clone()),
                output: serde_json::json!({"error": error}),
            },
        );
        self.updated_at = now;
    }

    /// Persist state to the checkpoint file.
    pub fn save(&self, state_path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(state_path, json)
    }

    /// Default state file path for a pipeline.
    pub fn default_path(pipeline_id: &str) -> PathBuf {
        PathBuf::from(format!("corpus/{}-state.json", pipeline_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_resume() {
        let tmp = std::env::temp_dir().join("test-pipeline-state.json");
        let _ = std::fs::remove_file(&tmp);

        let mut state = PipelineState::load_or_create("test", "1.0", &tmp);
        assert!(!state.is_complete("extract"));

        state.mark_started("extract");
        state.mark_complete("extract", serde_json::json!({"docs": 37}));
        assert!(state.is_complete("extract"));

        state.save(&tmp).unwrap();

        let loaded = PipelineState::load_or_create("test", "1.0", &tmp);
        assert!(loaded.is_complete("extract"));
        assert_eq!(
            loaded.steps["extract"].output["docs"],
            serde_json::json!(37)
        );

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn resume_skips_completed() {
        let tmp = std::env::temp_dir().join("test-pipeline-skip.json");
        let _ = std::fs::remove_file(&tmp);

        let mut state = PipelineState::load_or_create("test", "1.0", &tmp);
        state.mark_complete("extract", serde_json::json!({"done": true}));
        state.mark_complete("chunk", serde_json::json!({"chunks": 5000}));
        state.save(&tmp).unwrap();

        let loaded = PipelineState::load_or_create("test", "1.0", &tmp);
        assert!(loaded.is_complete("extract"));
        assert!(loaded.is_complete("chunk"));
        assert!(!loaded.is_complete("embed"));

        let _ = std::fs::remove_file(&tmp);
    }
}
