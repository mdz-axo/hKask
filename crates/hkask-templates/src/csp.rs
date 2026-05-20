//! CSP Channel Isolation for Pipeline Stages
//!
//! Implements Gordon Hoare CSP (Communicating Sequential Processes) patterns
//! for pipeline stage isolation. Each stage runs in an isolated tokio task
//! with bounded channels for backpressure.
//!
//! **Design Principles:**
//! - Process isolation via tokio tasks
//! - Bounded channels (capacity=1) for backpressure
//! - Stage timeout with cancellation
//! - Error propagation via channel messages
//!
//! **Stage Lifecycle:**
//! 1. Spawn stage in isolated task
//! 2. Send input via bounded channel
//! 3. Await output or timeout
//! 4. Propagate errors via channel

use crate::error::{CompositionError, RetryConfig};
use crate::skill_translation::{StageOutput, PipelineStage};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Stage execution message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMessage {
    pub stage_number: u32,
    pub stage_name: String,
    pub _input: serde_json::Value,
}

/// Stage execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage_number: u32,
    pub stage_name: String,
    pub output: Result<StageOutput, CompositionError>,
}

/// Stage execution error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageExecutionError {
    Timeout { stage_name: String, timeout_ms: u64 },
    ChannelSendFailed { stage_name: String },
    ChannelRecvFailed { stage_name: String },
    TaskPanic { stage_name: String },
}

impl From<StageExecutionError> for CompositionError {
    fn from(err: StageExecutionError) -> Self {
        match err {
            StageExecutionError::Timeout { stage_name, timeout_ms } => {
                CompositionError::StageTimeout { stage_name, timeout_ms }
            }
            StageExecutionError::ChannelSendFailed { stage_name } => {
                CompositionError::StageCommunicationFailed { stage_name }
            }
            StageExecutionError::ChannelRecvFailed { stage_name } => {
                CompositionError::StageCommunicationFailed { stage_name }
            }
            StageExecutionError::TaskPanic { stage_name } => {
                CompositionError::StageCommunicationFailed { stage_name }
            }
        }
    }
}

/// CSP Stage Executor trait
pub trait StageExecutor: Send + Sync {
    /// Execute a single stage with input
    fn execute(&self, input: serde_json::Value) -> Result<StageOutput, CompositionError>;
}

/// Isolated stage runner for tokio::spawn
pub struct IsolatedStageRunner<E: StageExecutor + 'static> {
    executor: E,
    timeout_ms: u64,
}

impl<E: StageExecutor + 'static> IsolatedStageRunner<E> {
    pub fn new(executor: E, timeout_ms: u64) -> Self {
        Self { executor, timeout_ms }
    }

    /// Run stage in isolated task with timeout
    pub async fn run_isolated(
        self,
        input: serde_json::Value,
        stage_name: String,
        stage_number: u32,
    ) -> StageResult {
        // Create bounded channel for backpressure (capacity=1)
        let (tx, mut rx) = mpsc::channel::<Result<StageOutput, CompositionError>>(1);

        // Spawn isolated task
        let handle = tokio::spawn(async move {
            let result = self.executor.execute(input);
            let _ = tx.send(result).await;
        });

        // Await with timeout
        let timeout_duration = Duration::from_millis(self.timeout_ms);
        let result = match timeout(timeout_duration, rx.recv()).await {
            Ok(Some(Ok(output))) => StageResult {
                stage_number,
                stage_name: stage_name.clone(),
                output: Ok(output),
            },
            Ok(Some(Err(e))) => StageResult {
                stage_number,
                stage_name: stage_name.clone(),
                output: Err(e),
            },
            Ok(None) => StageResult {
                stage_number,
                stage_name: stage_name.clone(),
                output: Err(CompositionError::StageCommunicationFailed { stage_name }),
            },
            Err(_) => {
                // Timeout - abort the task
                handle.abort();
                StageResult {
                    stage_number,
                    stage_name: stage_name.clone(),
                    output: Err(CompositionError::StageTimeout {
                        stage_name,
                        timeout_ms: self.timeout_ms,
                    }),
                }
            }
        };

        result
    }
}

/// Pipeline stage configuration with CSP settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CspStageConfig {
    pub stage: PipelineStage,
    pub timeout_ms: u64,
    pub channel_capacity: usize,
    pub retry_config: RetryConfig,
}

impl CspStageConfig {
    pub fn new(stage: PipelineStage) -> Self {
        Self {
            stage,
            timeout_ms: 30000, // 30 second default timeout
            channel_capacity: 1, // Bounded for backpressure
            retry_config: RetryConfig::default(),
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_channel_capacity(mut self, capacity: usize) -> Self {
        self.channel_capacity = capacity;
        self
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }
}

/// CSP Pipeline Executor
pub struct CspPipelineExecutor {
    stages: Vec<CspStageConfig>,
    default_timeout_ms: u64,
}

impl CspPipelineExecutor {
    pub fn new(stages: Vec<CspStageConfig>) -> Self {
        let default_timeout_ms = stages
            .iter()
            .map(|s| s.timeout_ms)
            .max()
            .unwrap_or(30000);

        Self {
            stages,
            default_timeout_ms,
        }
    }

    /// Get stage configurations
    pub fn stages(&self) -> &[CspStageConfig] {
        &self.stages
    }

    /// Execute pipeline with CSP isolation
    pub async fn execute(
        &self,
        initial_input: serde_json::Value,
    ) -> Result<StageOutput, CompositionError> {
        let mut current_input = initial_input;

        for config in &self.stages {
            let stage_result = self.execute_stage(config, current_input).await?;

            // Extract output for next stage
            current_input = match stage_result {
                StageOutput::Parse(skill) => serde_json::to_value(skill)
                    .map_err(|e| CompositionError::permanent(&format!("JSON serialization failed: {}", e), None))?,
                StageOutput::Map(triples) => serde_json::to_value(triples)
                    .map_err(|e| CompositionError::permanent(&format!("JSON serialization failed: {}", e), None))?,
                StageOutput::Generate { templates, manifests } => {
                    serde_json::json!({ "templates": templates, "manifests": manifests })
                }
                StageOutput::Validate(validated) => serde_json::to_value(validated)
                    .map_err(|e| CompositionError::permanent(&format!("JSON serialization failed: {}", e), None))?,
                StageOutput::Register(registered) => {
                    // Final stage - return result
                    return Ok(StageOutput::Register(registered));
                }
            };
        }

        Err(CompositionError::permanent("Pipeline did not reach register stage", None))
    }

    /// Execute single stage with isolation
    async fn execute_stage(
        &self,
        config: &CspStageConfig,
        input: serde_json::Value,
    ) -> Result<StageOutput, CompositionError> {
        let mut attempt = 0;

        loop {
            let result = self.execute_stage_once(config, input.clone()).await;

            match &result {
                Ok(_) => return result,
                Err(e) => {
                    if e.is_retryable() && config.retry_config.should_retry(attempt) {
                        attempt += 1;
                        let delay = config.retry_config.backoff_delay(attempt);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }
                    return result;
                }
            }
        }
    }

    /// Execute stage once (no retry)
    async fn execute_stage_once(
        &self,
        config: &CspStageConfig,
        _input: serde_json::Value,
    ) -> Result<StageOutput, CompositionError> {
        // Placeholder - actual implementation would create appropriate executor
        // for each stage type (Parse, Map, Generate, Validate, Register)
        // and run it via IsolatedStageRunner

        // For now, return a placeholder error
        Err(CompositionError::transient(&format!(
            "Stage {} not yet implemented with CSP isolation",
            config.stage.name
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExecutor {
        should_fail: bool,
    }

    impl StageExecutor for TestExecutor {
        fn execute(&self, input: serde_json::Value) -> Result<StageOutput, CompositionError> {
            if self.should_fail {
                Err(CompositionError::transient("test failure"))
            } else {
                Ok(StageOutput::Parse(crate::skill_translation::ParsedSkill {
                    id: "test".to_string(),
                    name: "Test".to_string(),
                    description: "Test".to_string(),
                    format: crate::skill_translation::SkillFormat::ClaudeSkill,
                    prompts: vec![],
                    process_logic: Some(input),
                    capabilities: vec![],
                    visibility: "Shared".to_string(),
                }))
            }
        }
    }

    #[tokio::test]
    async fn test_isolated_stage_runner_success() {
        let executor = TestExecutor { should_fail: false };
        let runner = IsolatedStageRunner::new(executor, 5000);

        let result = runner
            .run_isolated(serde_json::json!({}), "test".to_string(), 1)
            .await;

        assert!(result.output.is_ok());
    }

    #[tokio::test]
    async fn test_isolated_stage_runner_failure() {
        let executor = TestExecutor { should_fail: true };
        let runner = IsolatedStageRunner::new(executor, 5000);

        let result = runner
            .run_isolated(serde_json::json!({}), "test".to_string(), 1)
            .await;

        assert!(result.output.is_err());
        assert!(result.output.unwrap_err().is_retryable());
    }

    #[tokio::test]
    async fn test_csp_stage_config_default() {
        let stage = PipelineStage {
            stage_number: 1,
            name: "test".to_string(),
            description: "test".to_string(),
            energy_cap: 100,
            cns_span: "test".to_string(),
        };

        let config = CspStageConfig::new(stage);
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.channel_capacity, 1);
        assert_eq!(config.retry_config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_csp_stage_config_builder() {
        let stage = PipelineStage {
            stage_number: 1,
            name: "test".to_string(),
            description: "test".to_string(),
            energy_cap: 100,
            cns_span: "test".to_string(),
        };

        let config = CspStageConfig::new(stage)
            .with_timeout(60000)
            .with_channel_capacity(10)
            .with_retry_config(RetryConfig::new(5, 2000, 20000));

        assert_eq!(config.timeout_ms, 60000);
        assert_eq!(config.channel_capacity, 10);
        assert_eq!(config.retry_config.max_retries, 5);
    }

    #[tokio::test]
    async fn test_csp_pipeline_executor_new() {
        let stages = vec![
            CspStageConfig::new(PipelineStage {
                stage_number: 1,
                name: "parse".to_string(),
                description: "test".to_string(),
                energy_cap: 100,
                cns_span: "test".to_string(),
            }),
            CspStageConfig::new(PipelineStage {
                stage_number: 2,
                name: "map".to_string(),
                description: "test".to_string(),
                energy_cap: 100,
                cns_span: "test".to_string(),
            }),
        ];

        let executor = CspPipelineExecutor::new(stages);
        assert_eq!(executor.stages().len(), 2);
    }
}