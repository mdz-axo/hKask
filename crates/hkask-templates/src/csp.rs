//! CSP Stage Isolation — Simplified
//!
//! Generic stage executor that reads configuration from csp-stage-isolation.yaml.
//! Rust is the loom. YAML is the thread.
//! ℏKask v0.21.2

use crate::config::load_yaml_config;
use crate::ports::TemplateError;
use hkask_cns::spans::SpanEmitter;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

/// Named operation function type
pub type OperationFn =
    Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value, TemplateError> + Send + Sync>;

/// Stage configuration (loaded from YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageConfig {
    pub name: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub retry_on_failure: bool,
}

fn default_timeout() -> u64 {
    5000
}

/// CSP stage configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CspConfig {
    pub stage_execution: StageExecutionConfig,
    #[serde(default)]
    pub stages: StageDefinitions,
    #[serde(default)]
    pub error_handling: ErrorHandlingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageExecutionConfig {
    #[serde(default = "default_timeout")]
    pub default_timeout_ms: u64,
    #[serde(default = "default_channel_capacity")]
    pub channel_capacity: usize,
    #[serde(default)]
    pub retry: hkask_types::cns::RetryConfig,
}

fn default_channel_capacity() -> usize {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageDefinitions {
    #[serde(default)]
    pub pre_process: Vec<StageConfig>,
    #[serde(default)]
    pub core_process: Vec<StageConfig>,
    #[serde(default)]
    pub post_process: Vec<StageConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorHandlingConfig {
    #[serde(default)]
    pub classification: ErrorClassification,
    #[serde(default)]
    pub escalation: EscalationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorClassification {
    #[serde(default)]
    pub retryable: Vec<String>,
    #[serde(default)]
    pub non_retryable: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EscalationConfig {
    #[serde(default)]
    pub on_max_retries_exceeded: String,
    #[serde(default)]
    pub on_stage_panic: String,
    #[serde(default)]
    pub on_channel_failure: String,
}

/// Stage execution result
#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage_name: String,
    pub output: Result<serde_json::Value, TemplateError>,
    pub duration_ms: u64,
}

/// CSP Stage Executor
pub struct CspExecutor {
    config: CspConfig,
    emitter: SpanEmitter,
    operations: HashMap<String, OperationFn>,
}

impl CspExecutor {
    pub fn new(config: CspConfig) -> Self {
        let observer = WebID::new();
        Self {
            config,
            emitter: SpanEmitter::new(observer),
            operations: HashMap::new(),
        }
    }

    /// Register a named operation
    pub fn register_operation<F>(&mut self, name: &str, operation: F)
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, TemplateError>
            + Send
            + Sync
            + 'static,
    {
        self.operations
            .insert(name.to_string(), Box::new(operation));
    }

    /// Classify an error as retryable or non-retryable
    fn classify_error(&self, error: &TemplateError) -> bool {
        let error_str = error.to_string();

        // Check if error matches any retryable pattern
        for pattern in &self.config.error_handling.classification.retryable {
            if error_str.contains(pattern) {
                return true;
            }
        }

        // Check if error matches any non-retryable pattern
        for pattern in &self.config.error_handling.classification.non_retryable {
            if error_str.contains(pattern) {
                return false;
            }
        }

        // Default: non-retryable
        false
    }

    /// Execute a stage with timeout and retry
    pub async fn execute_stage(
        &self,
        stage: &StageConfig,
        input: serde_json::Value,
    ) -> StageResult {
        let start = std::time::Instant::now();

        self.emitter.emit_tool(
            "csp.stage.start",
            serde_json::json!({
                "stage": stage.name,
                "timeout_ms": stage.timeout_ms,
                "retry_on_failure": stage.retry_on_failure,
            }),
        );

        let max_retries = if stage.retry_on_failure {
            self.config.stage_execution.retry.max_retries
        } else {
            0
        };

        let mut attempt = 0;
        let mut last_error = None;

        while attempt <= max_retries {
            let result = timeout(
                Duration::from_millis(stage.timeout_ms),
                self.run_stage(stage, input.clone()),
            )
            .await;

            let duration_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(Ok(out)) => {
                    self.emitter.emit_tool(
                        "csp.stage.complete",
                        serde_json::json!({
                            "stage": stage.name,
                            "duration_ms": duration_ms,
                            "attempts": attempt + 1,
                        }),
                    );
                    return StageResult {
                        stage_name: stage.name.clone(),
                        output: Ok(out),
                        duration_ms,
                    };
                }
                Ok(Err(e)) => {
                    // Classify error
                    let is_retryable = self.classify_error(&e);

                    self.emitter.emit_tool(
                        "csp.stage.error",
                        serde_json::json!({
                            "stage": stage.name,
                            "error": e.to_string(),
                            "retryable": is_retryable,
                            "attempt": attempt + 1,
                        }),
                    );

                    if !is_retryable || attempt >= max_retries {
                        last_error = Some(e);
                        break;
                    }

                    // Exponential backoff
                    let delay_ms =
                        self.config.stage_execution.retry.initial_delay_ms * 2u64.pow(attempt);
                    let delay_ms = delay_ms.min(self.config.stage_execution.retry.max_delay_ms);

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    attempt += 1;
                }
                Err(_) => {
                    self.emitter.emit_tool(
                        "csp.stage.timeout",
                        serde_json::json!({
                            "stage": stage.name,
                            "timeout_ms": stage.timeout_ms,
                            "attempt": attempt + 1,
                        }),
                    );

                    if attempt >= max_retries {
                        last_error = Some(TemplateError::Manifest(format!(
                            "Stage '{}' timed out after {} attempts (timeout: {}ms)",
                            stage.name,
                            attempt + 1,
                            stage.timeout_ms
                        )));
                        break;
                    }

                    // Exponential backoff for timeout
                    let delay_ms =
                        self.config.stage_execution.retry.initial_delay_ms * 2u64.pow(attempt);
                    let delay_ms = delay_ms.min(self.config.stage_execution.retry.max_delay_ms);

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    attempt += 1;
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        StageResult {
            stage_name: stage.name.clone(),
            output: Err(
                last_error.unwrap_or_else(|| TemplateError::Manifest("Stage failed".to_string()))
            ),
            duration_ms,
        }
    }

    async fn run_stage(
        &self,
        stage: &StageConfig,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, TemplateError> {
        if let Some(operation) = self.operations.get(&stage.name) {
            operation(input)
        } else {
            Ok(input)
        }
    }

    /// Execute pipeline stages
    pub async fn execute_pipeline(
        &self,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, TemplateError> {
        let mut current = input;

        // Execute pre-process stages
        for stage in &self.config.stages.pre_process {
            let result = self.execute_stage(stage, current).await;
            current = result.output?;
        }

        // Execute core stages
        for stage in &self.config.stages.core_process {
            let result = self.execute_stage(stage, current).await;
            current = result.output?;
        }

        // Execute post-process stages
        for stage in &self.config.stages.post_process {
            let result = self.execute_stage(stage, current).await;
            current = result.output?;
        }

        Ok(current)
    }
}

/// Load CSP config from YAML
pub fn load_csp_config(yaml_path: &str) -> Result<CspConfig, TemplateError> {
    load_yaml_config(yaml_path)
}
