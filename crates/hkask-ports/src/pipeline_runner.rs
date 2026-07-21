//! Pipeline runner — executes PipelineManifest manifests with checkpoint/resume.
//!
//! Reads a PipelineManifest YAML, iterates steps, checkpoints via PipelineState,
//! and delegates step execution to a caller-provided StepExecutor.
//!
//! Platform-level — usable from CLI, replica server, or any runtime.
//! Each step emits reg.pipeline.{step_id} Regulation spans.

use crate::pipeline_manifest::{PipelineManifest, PipelineStep};
use crate::pipeline_state::PipelineState;
use serde::Serialize;

/// Typed errors for pipeline execution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PipelineError {
    /// Step requires affirmative consent (P2) before execution.
    #[error("Step '{step_id}' requires affirmative consent (P2). Approve before running.")]
    ConsentRequired { step_id: String },

    /// Step execution failed with a message from the executor.
    #[error("Step '{step_id}' failed: {message}")]
    StepFailed { step_id: String, message: String },

    /// Pipeline state save/load failed.
    #[error("State I/O error: {0}")]
    StateIo(String),

    /// Output verification failed — field value did not meet the gate.
    #[error("Verification failed for step '{step_id}': {message}")]
    VerificationFailed { step_id: String, message: String },
}

/// Output from running a full pipeline.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineRunResult {
    pub pipeline_id: String,
    pub steps_completed: usize,
    pub steps_skipped: usize,
    pub steps_failed: usize,
    pub total_steps: usize,
    pub state_path: String,
}

/// Callback for executing a single pipeline step.
/// Implementors dispatch to MCP tools, subprocesses, or direct function calls.
pub trait StepExecutor: Send + Sync {
    fn execute(&self, step: &PipelineStep) -> Result<serde_json::Value, PipelineError>;
}

/// The pipeline runner — orchestrates PipelineManifest execution.
pub struct PipelineRunner {
    manifest: PipelineManifest,
    state: PipelineState,
    state_path: std::path::PathBuf,
}

impl PipelineRunner {
    /// Create a runner from an already-parsed manifest.
    /// Loads or creates checkpoint state from the default path.
    pub fn new(manifest: PipelineManifest) -> Result<Self, PipelineError> {
        let state_path = PipelineState::default_path(&manifest.id);
        let mut state = PipelineState::load_or_create(&manifest.id, &manifest.version, &state_path);

        if state.started_at.is_none() {
            state.started_at = Some(chrono::Utc::now().to_rfc3339());
            state
                .save(&state_path)
                .map_err(|e| PipelineError::StateIo(e.to_string()))?;
        }

        Ok(Self {
            manifest,
            state,
            state_path,
        })
    }

    /// Check if a step has already completed (checkpoint resume).
    pub fn is_complete(&self, step_id: &str) -> bool {
        self.state.is_complete(step_id)
    }

    /// Run a single step with checkpointing.
    pub fn run_step(
        &mut self,
        step: &PipelineStep,
        executor: &dyn StepExecutor,
    ) -> Result<serde_json::Value, PipelineError> {
        // Resume: skip completed steps
        if self.state.is_complete(&step.id) {
            return Ok(self.state.steps[&step.id].output.clone());
        }

        // P2: Affirmative consent gate
        if step.requires_consent {
            return Err(PipelineError::ConsentRequired {
                step_id: step.id.clone(),
            });
        }

        self.state.mark_started(&step.id);
        self.state
            .save(&self.state_path)
            .map_err(|e| PipelineError::StateIo(e.to_string()))?;

        match executor.execute(step) {
            Ok(output) => {
                if let Err(e) = PipelineManifest::verify_output(step, &output) {
                    let msg = e.to_string();
                    self.state.mark_failed(&step.id, &msg);
                    self.state.save(&self.state_path).ok();
                    return Err(e);
                }
                self.state.mark_complete(&step.id, output.clone());
                self.state
                    .save(&self.state_path)
                    .map_err(|e| PipelineError::StateIo(e.to_string()))?;
                Ok(output)
            }
            Err(e) => {
                let msg = e.to_string();
                self.state.mark_failed(&step.id, &msg);
                self.state.save(&self.state_path).ok();
                Err(e)
            }
        }
    }

    /// Run all steps in the manifest, checkpointing after each.
    pub fn run_all(&mut self, executor: &dyn StepExecutor) -> PipelineRunResult {
        let mut completed = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;

        let steps = self.manifest.steps.clone();
        for step in &steps {
            let step_result = self.run_step(step, executor);
            match step_result {
                Ok(_) => {
                    let status = self.state.steps[&step.id].status.clone();
                    if status == "skipped" {
                        skipped += 1;
                    } else {
                        completed += 1;
                    }
                }
                Err(_e) => {
                    failed += 1;
                    break;
                }
            }
        }

        PipelineRunResult {
            pipeline_id: self.manifest.id.clone(),
            steps_completed: completed,
            steps_skipped: skipped,
            steps_failed: failed,
            total_steps: self.manifest.steps.len(),
            state_path: self.state_path.to_string_lossy().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExecutor;
    impl StepExecutor for TestExecutor {
        fn execute(&self, step: &PipelineStep) -> Result<serde_json::Value, PipelineError> {
            Ok(serde_json::json!({"step": step.id, "count": 100}))
        }
    }

    #[test]
    fn pipeline_completes_all_steps() {
        let manifest: PipelineManifest = serde_json::from_str(
            r#"{"id":"test-pipeline","version":"1.0","steps":[{"id":"step1","tool":"test_tool","verify":{"field":"count","min":10}},{"id":"step2","tool":"test_tool"}]}"#
        ).unwrap();

        let mut runner = PipelineRunner::new(manifest).unwrap();
        let executor = TestExecutor;
        let result = runner.run_all(&executor);

        assert_eq!(result.steps_completed, 2);
        assert_eq!(result.steps_failed, 0);
    }
}
