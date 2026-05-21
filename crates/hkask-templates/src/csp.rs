//! CSP Stage Isolation — Simplified
//!
//! Generic stage executor that reads configuration from csp-stage-isolation.yaml.
//! Rust is the loom. YAML is the thread.
//! ℏKask v0.21.2

use crate::ports::TemplateError;
use hkask_cns::spans::SpanEmitter;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub retry: RetryConfig,
}

fn default_channel_capacity() -> usize {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetryConfig {
    #[serde(default)]
    pub max_retries: u32,
    #[serde(default)]
    pub initial_delay_ms: u64,
    #[serde(default)]
    pub max_delay_ms: u64,
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
}

impl CspExecutor {
    pub fn new(config: CspConfig) -> Self {
        let observer = WebID::new();
        Self {
            config,
            emitter: SpanEmitter::new(observer),
        }
    }

    /// Execute a stage with timeout
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
            }),
        );

        let result = timeout(
            Duration::from_millis(stage.timeout_ms),
            self.run_stage(stage, input),
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        let output = match result {
            Ok(Ok(out)) => {
                self.emitter.emit_tool(
                    "csp.stage.complete",
                    serde_json::json!({
                        "stage": stage.name,
                        "duration_ms": duration_ms,
                    }),
                );
                Ok(out)
            }
            Ok(Err(e)) => {
                self.emitter.emit_tool(
                    "csp.stage.error",
                    serde_json::json!({
                        "stage": stage.name,
                        "error": e.to_string(),
                    }),
                );
                Err(e)
            }
            Err(_) => {
                self.emitter.emit_tool(
                    "csp.stage.timeout",
                    serde_json::json!({
                        "stage": stage.name,
                        "timeout_ms": stage.timeout_ms,
                    }),
                );
                Err(TemplateError::RecursionLimit {
                    stage_name: stage.name.clone(),
                    timeout_ms: stage.timeout_ms,
                })
            }
        };

        StageResult {
            stage_name: stage.name.clone(),
            output,
            duration_ms,
        }
    }

    async fn run_stage(
        &self,
        _stage: &StageConfig,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, TemplateError> {
        // Generic stage execution - actual logic in templates
        Ok(input)
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
    let content = std::fs::read_to_string(yaml_path)
        .map_err(|e| TemplateError::Validation(format!("Failed to read CSP config: {}", e)))?;
    
    serde_yaml::from_str(&content)
        .map_err(|e| TemplateError::Validation(format!("Failed to parse CSP config: {}", e)))
}
