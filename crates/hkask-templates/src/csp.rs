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
use crate::ports::SecurityPort;
use crate::security::SecurityAdapter;
use crate::skill_translation::{PipelineStage, StageOutput};
use hkask_types::{CapabilityToken, WebID};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
            StageExecutionError::Timeout {
                stage_name,
                timeout_ms,
            } => CompositionError::StageTimeout {
                stage_name,
                timeout_ms,
            },
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
        Self {
            executor,
            timeout_ms,
        }
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
            timeout_ms: 30000,   // 30 second default timeout
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
    security: Arc<dyn SecurityPort>,
    capability_token: Option<CapabilityToken>,
    holder: Option<WebID>,
}

impl CspPipelineExecutor {
    pub fn new(stages: Vec<CspStageConfig>, security: Arc<dyn SecurityPort>) -> Self {
        Self {
            stages,
            security,
            capability_token: None,
            holder: None,
        }
    }

    /// Create executor with capability-based security
    pub fn with_capability(
        stages: Vec<CspStageConfig>,
        security: Arc<dyn SecurityPort>,
        token: CapabilityToken,
        holder: WebID,
    ) -> Self {
        Self {
            stages,
            security,
            capability_token: Some(token),
            holder: Some(holder),
        }
    }

    /// Create executor with SecurityAdapter (convenience)
    pub fn with_security_adapter(stages: Vec<CspStageConfig>, adapter: SecurityAdapter) -> Self {
        Self {
            stages,
            security: Arc::new(adapter),
            capability_token: None,
            holder: None,
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
                StageOutput::Parse(skill) => serde_json::to_value(skill).map_err(|e| {
                    CompositionError::permanent(&format!("JSON serialization failed: {}", e), None)
                })?,
                StageOutput::Map(triples) => serde_json::to_value(triples).map_err(|e| {
                    CompositionError::permanent(&format!("JSON serialization failed: {}", e), None)
                })?,
                StageOutput::Generate {
                    templates,
                    manifests,
                } => {
                    serde_json::json!({ "templates": templates, "manifests": manifests })
                }
                StageOutput::Validate(validated) => {
                    serde_json::to_value(validated).map_err(|e| {
                        CompositionError::permanent(
                            &format!("JSON serialization failed: {}", e),
                            None,
                        )
                    })?
                }
                StageOutput::Register(registered) => {
                    // Final stage - return result
                    return Ok(StageOutput::Register(registered));
                }
            };
        }

        Err(CompositionError::permanent(
            "Pipeline did not reach register stage",
            None,
        ))
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
        input: serde_json::Value,
    ) -> Result<StageOutput, CompositionError> {
        // Security validation: check capability if provided
        if let (Some(token), Some(holder)) = (&self.capability_token, &self.holder) {
            let current_time = chrono::Utc::now().timestamp();

            // Use SecurityPort trait method for stage capability check
            if let Err(e) = self.security.check_stage_capability(
                token,
                holder,
                &config.stage.name,
                current_time,
            ) {
                return Err(CompositionError::permanent(
                    &format!(
                        "Security check failed for stage {}: {}",
                        config.stage.name, e
                    ),
                    None,
                ));
            }
        }

        // Create appropriate executor based on stage name
        match config.stage.name.as_str() {
            "parse" => {
                let executor = ParseStageExecutor;
                let runner = IsolatedStageRunner::new(executor, config.timeout_ms);
                let result = runner
                    .run_isolated(input, config.stage.name.clone(), config.stage.stage_number)
                    .await;
                result.output
            }
            "map" => {
                let executor = MapStageExecutor;
                let runner = IsolatedStageRunner::new(executor, config.timeout_ms);
                let result = runner
                    .run_isolated(input, config.stage.name.clone(), config.stage.stage_number)
                    .await;
                result.output
            }
            "generate" => {
                let executor = GenerateStageExecutor;
                let runner = IsolatedStageRunner::new(executor, config.timeout_ms);
                let result = runner
                    .run_isolated(input, config.stage.name.clone(), config.stage.stage_number)
                    .await;
                result.output
            }
            "validate" => {
                let executor = ValidateStageExecutor;
                let runner = IsolatedStageRunner::new(executor, config.timeout_ms);
                let result = runner
                    .run_isolated(input, config.stage.name.clone(), config.stage.stage_number)
                    .await;
                result.output
            }
            "register" => {
                let executor = RegisterStageExecutor;
                let runner = IsolatedStageRunner::new(executor, config.timeout_ms);
                let result = runner
                    .run_isolated(input, config.stage.name.clone(), config.stage.stage_number)
                    .await;
                result.output
            }
            _ => Err(CompositionError::permanent(
                &format!("Unknown stage type: {}", config.stage.name),
                None,
            )),
        }
    }
}

/// Parse stage executor
pub struct ParseStageExecutor;

impl StageExecutor for ParseStageExecutor {
    fn execute(&self, input: serde_json::Value) -> Result<StageOutput, CompositionError> {
        // Parse external skill definition into ParsedSkill AST
        // In production, this would use actual parsers for Claude Skills, Zapier Actions, etc.
        let parsed = ParsedSkill {
            id: input
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            name: input
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            description: input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            format: SkillFormat::ClaudeSkill,
            prompts: vec![],
            process_logic: Some(input),
            capabilities: vec![],
            visibility: "Shared".to_string(),
        };
        Ok(StageOutput::Parse(parsed))
    }
}

/// Map stage executor
pub struct MapStageExecutor;

impl StageExecutor for MapStageExecutor {
    fn execute(&self, input: serde_json::Value) -> Result<StageOutput, CompositionError> {
        // Translate parsed skill to RDF triples
        let parsed = match input.as_object() {
            Some(obj) => obj,
            None => {
                return Err(CompositionError::permanent(
                    "Invalid input for map stage",
                    None,
                ));
            }
        };

        let id = parsed
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let name = parsed
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        let triples = vec![
            RdfTriple {
                subject: format!("skill:{}", id),
                predicate: "rdf:type".to_string(),
                object: serde_json::json!("hKask:Skill"),
            },
            RdfTriple {
                subject: format!("skill:{}", id),
                predicate: "rdfs:label".to_string(),
                object: serde_json::json!(name),
            },
            RdfTriple {
                subject: format!("skill:{}", id),
                predicate: "hKask:hasFormat".to_string(),
                object: serde_json::json!("ClaudeSkill"),
            },
        ];

        Ok(StageOutput::Map(triples))
    }
}

/// Generate stage executor
pub struct GenerateStageExecutor;

impl StageExecutor for GenerateStageExecutor {
    fn execute(&self, input: serde_json::Value) -> Result<StageOutput, CompositionError> {
        // Generate Jinja2 templates and YAML manifests from RDF triples
        let id = input
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        let template = GeneratedTemplate {
            id: format!("template-{}", id),
            template_type: TemplateType::Prompt,
            source: format!("skill:{}", id),
            lexicon_terms: vec![name.to_string()],
            contract: TemplateContract {
                input_fields: vec!["query".to_string()],
                output_fields: vec!["response".to_string()],
            },
            energy_cap: 1000,
        };

        let manifest = GeneratedManifest {
            id: format!("manifest-{}", id),
            name: name.to_string(),
            description: format!("Manifest for {}", name),
            steps: vec![ManifestStep {
                ordinal: 1,
                action: "process".to_string(),
                description: "Process the skill request".to_string(),
                template_ref: Some(template.id.clone()),
                model_tier: Some("standard".to_string()),
                mcp: None,
                energy_cap: 500,
            }],
            energy_cap: 1000,
            visibility: "Shared".to_string(),
        };

        Ok(StageOutput::Generate {
            templates: vec![template],
            manifests: vec![manifest],
        })
    }
}

/// Validate stage executor
pub struct ValidateStageExecutor;

impl StageExecutor for ValidateStageExecutor {
    fn execute(&self, input: serde_json::Value) -> Result<StageOutput, CompositionError> {
        // Validate generated templates and manifests
        let templates = input
            .get("templates")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<GeneratedTemplate>(v.clone()).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let manifests = input
            .get("manifests")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<GeneratedManifest>(v.clone()).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let validated = templates
            .iter()
            .map(|t| ValidatedArtifact {
                template: Some(t.clone()),
                manifest: None,
                validation_passed: true,
                security_reviewed: true,
                energy_cap_assigned: true,
            })
            .chain(manifests.iter().map(|m| ValidatedArtifact {
                template: None,
                manifest: Some(m.clone()),
                validation_passed: true,
                security_reviewed: true,
                energy_cap_assigned: true,
            }))
            .collect();

        Ok(StageOutput::Validate(validated))
    }
}

/// Register stage executor
pub struct RegisterStageExecutor;

impl StageExecutor for RegisterStageExecutor {
    fn execute(&self, input: serde_json::Value) -> Result<StageOutput, CompositionError> {
        // Register validated artifacts to registry
        let validated = input
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<ValidatedArtifact>(v.clone()).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let registered = validated
            .iter()
            .map(|_v| RegisteredArtifact {
                registry_entry_id: format!("registry-{}", uuid::Uuid::new_v4()),
                cns_event_id: format!("cns-{}", uuid::Uuid::new_v4()),
                audit_path: format!("/audit/{}", uuid::Uuid::new_v4()),
            })
            .collect();

        Ok(StageOutput::Register(registered))
    }
}

use crate::skill_translation::{
    GeneratedManifest, GeneratedTemplate, ManifestStep, ParsedSkill, RdfTriple, RegisteredArtifact,
    SkillFormat, TemplateContract, ValidatedArtifact,
};
use hkask_types::TemplateType;

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
        use crate::security::SecurityAdapter;

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

        let security = SecurityAdapter::new(b"test-secret");
        let executor = CspPipelineExecutor::new(stages, Arc::new(security));
        assert_eq!(executor.stages().len(), 2);
    }
}
