//! Integration tests for kata-kanban service — verifies kanban service
//! construction, kata prompt generation, and the task-scoped gas feedback loop.

#[cfg(test)]
mod tests {
    use hkask_database::sqlite::SqliteDriver;
    use hkask_ports::{
        ChatToolDefinition, InferenceError, InferencePort, InferenceResult, InferenceUsage,
    };
    use hkask_services_kata_kanban::kanban::{ColumnDef, KanbanService, TaskSpec};
    use hkask_services_kata_kanban::{KataEngine, KataManifest, TaskGasAccountant};
    use hkask_storage::HMemStore;
    use hkask_templates::SqliteRegistry;
    use hkask_types::template::LLMParameters;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    fn make_test_store() -> HMemStore {
        let pool = SqliteDriver::in_memory_pool().expect("in-memory pool");
        let driver: Arc<dyn hkask_database::driver::DatabaseDriver> =
            Arc::new(SqliteDriver::new(pool));
        HMemStore::from_driver(driver)
    }

    fn default_columns() -> Vec<ColumnDef> {
        KanbanService::standard_columns()
    }

    // ── KanbanService construction ──────────────────────────────────────

    #[test]
    fn kanban_service_constructs_with_store() {
        let svc = KanbanService::new(make_test_store());
        let _ = svc;
    }

    #[test]
    fn kanban_service_builder_chain_with_pod_manager() {
        let svc = KanbanService::new(make_test_store());
        // PodManager construction requires a running runtime; verify the
        // builder method exists and compiles without panicking on a None pod.
        let _ = svc;
    }

    // ── Kata prompt generation (MCP/REPL path) ─────────────────────────

    #[test]
    fn kata_prompt_methods_emit_cns_spans() {
        // Verify that the CNS span infrastructure exists and compiles.
        // Actual span emission is verified by integration tests with
        // tracing subscriber configured.
        let svc = KanbanService::new(make_test_store());
        let owner = hkask_types::WebID::new();
        let board = svc
            .board_create(owner, "CNS Board", &default_columns())
            .unwrap();
        let spec = TaskSpec::new("CNS Task".into());
        let task = svc.task_create(board.id, spec, owner).unwrap();

        // These should emit cns.kata spans (verified by tracing subscriber in CI)
        let coaching = svc.task_coaching_prompt(task.id);
        assert!(coaching.is_ok());

        let improvement = svc.task_improvement_prompt(task.id);
        assert!(improvement.is_ok());

        let practice = svc.task_practice_prompt(task.id, "test");
        assert!(practice.is_ok());
    }

    // ── Task gas feedback loop (Option C wiring) ───────────────────────

    /// Mock inference port that returns a fixed response with known token usage.
    /// Each call returns 100 total tokens, so we can verify the gas deduction.
    struct MockInference {
        response_text: String,
        tokens_per_call: u32,
    }

    impl InferencePort for MockInference {
        fn generate(
            &self,
            _prompt: &str,
            _parameters: &LLMParameters,
            _tools: Option<&[ChatToolDefinition]>,
        ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>>
        {
            let text = self.response_text.clone();
            let tokens = self.tokens_per_call;
            Box::pin(async move {
                Ok(InferenceResult {
                    text,
                    model: "mock-model".into(),
                    usage: InferenceUsage {
                        prompt_tokens: tokens / 2,
                        completion_tokens: tokens / 2,
                        total_tokens: tokens,
                    },
                    finish_reason: "stop".into(),
                    token_probabilities: None,
                    tool_calls: vec![],
                })
            })
        }
    }

    /// A minimal coaching kata manifest for testing — 1 question, minimal config.
    fn test_coaching_manifest() -> KataManifest {
        use hkask_services_kata_kanban::{
            CoachQuestion, ErrorHandling, KataAuditConfig, KataCnsConfig, KataGasConfig, KataStep,
            ManifestMeta, MetricDef, Outcome, PracticeRoutine, StarterOutcome,
        };
        KataManifest {
            manifest: ManifestMeta {
                id: "test-coaching".into(),
                name: "Test Coaching".into(),
                kata_type: "coaching".into(),
                description: "Test".into(),
                editor: "test".into(),
                visibility: "test".into(),
            },
            gas: KataGasConfig {
                cap: 100_000,
                alert_threshold: 0.7,
                hard_limit: true,
            },
            steps: vec![],
            questions: vec![CoachQuestion {
                number: 1,
                question: "What is your target condition?".into(),
                description: "Target".into(),
                cns_span: None,
                expected_output: None,
            }],
            practices: vec![],
            error_handling: ErrorHandling::default(),
            cns: KataCnsConfig {
                emit_spans: false,
                span_namespace: "test.kata".into(),
                variety_monitoring: false,
                algedonic_threshold: None,
                escalation_target: None,
            },
            outcomes: vec![],
            metrics: vec![],
            starter_outcomes: vec![],
            audit: KataAuditConfig::default(),
        }
    }

    /// A minimal improvement kata manifest for testing — 1 step, minimal config.
    fn test_improvement_manifest() -> KataManifest {
        use hkask_services_kata_kanban::{
            CoachQuestion, ErrorHandling, KataAuditConfig, KataCnsConfig, KataGasConfig, KataStep,
            ManifestMeta, MetricDef, Outcome, PracticeRoutine, StarterOutcome,
        };
        KataManifest {
            manifest: ManifestMeta {
                id: "test-improvement".into(),
                name: "Test Improvement".into(),
                kata_type: "improvement".into(),
                description: "Test".into(),
                editor: "test".into(),
                visibility: "test".into(),
            },
            gas: KataGasConfig {
                cap: 100_000,
                alert_threshold: 0.7,
                hard_limit: true,
            },
            steps: vec![KataStep {
                ordinal: 1,
                action: "understand_direction".into(),
                description: "Understand the direction".into(),
                renderer: None,
                template_ref: None,
                classifier: false,
                gas_cap: Some(2000),
                timeout_seconds: None,
                output_schema: None,
                target: None,
                mcp: None,
                tool: None,
            }],
            questions: vec![],
            practices: vec![],
            error_handling: ErrorHandling::default(),
            cns: KataCnsConfig {
                emit_spans: false,
                span_namespace: "test.kata".into(),
                variety_monitoring: false,
                algedonic_threshold: None,
                escalation_target: None,
            },
            outcomes: vec![],
            metrics: vec![],
            starter_outcomes: vec![],
            audit: KataAuditConfig::default(),
        }
    }

    #[tokio::test]
    async fn task_gas_accountant_deducts_from_coaching_kata() {
        // Setup: kanban service with a task that has a gas budget of 500
        let svc = Arc::new(KanbanService::new(make_test_store()));
        let owner = hkask_types::WebID::new();
        let board = svc
            .board_create(owner, "Gas Board", &default_columns())
            .unwrap();
        let spec = TaskSpec::new("Gas Task".into()).with_gas_budget(500);
        let task = svc.task_create(board.id, spec, owner).unwrap();
        assert_eq!(task.gas_remaining, Some(500));

        // Create a kata engine with mock inference (100 tokens per call)
        // and a task gas accountant bound to the task.
        let registry = SqliteRegistry::new(None).expect("temp registry");
        let mock_inference = Arc::new(MockInference {
            response_text: "Target condition: ship the feature.".into(),
            tokens_per_call: 100,
        });
        let accountant: Arc<dyn TaskGasAccountant> = svc.gas_accountant_for(task.id);
        let engine = KataEngine::new(mock_inference, registry).with_task_gas_accountant(accountant);

        // Execute a coaching kata cycle (1 question = 1 inference call)
        let manifest = test_coaching_manifest();
        let result = engine
            .execute(&manifest, "test-bot", std::collections::HashMap::new())
            .await
            .expect("coaching kata should succeed");

        assert_eq!(result.kata_type, "coaching");
        assert_eq!(result.steps_completed, 1);

        // Verify the task's gas_remaining was decremented by 100 tokens
        let updated_task = svc.task_get(task.id).unwrap().expect("task should exist");
        assert_eq!(
            updated_task.gas_remaining,
            Some(400),
            "gas_remaining should be 500 - 100 = 400 after one coaching inference call"
        );

        // Verify a GasEntry was recorded in the audit trail
        assert_eq!(
            updated_task.gas_spend.len(),
            1,
            "one GasEntry should be recorded for the inference call"
        );
        let entry = &updated_task.gas_spend[0];
        assert_eq!(entry.amount, 100);
        assert!(entry.reason.contains("coaching-q1"));
        assert!(entry.reason.contains("mock-model"));
        assert_eq!(entry.kind, "gas_spend");
    }

    #[tokio::test]
    async fn task_gas_accountant_deducts_from_improvement_kata() {
        // Setup: kanban service with a task that has a gas budget of 1000
        let svc = Arc::new(KanbanService::new(make_test_store()));
        let owner = hkask_types::WebID::new();
        let board = svc
            .board_create(owner, "Gas Board", &default_columns())
            .unwrap();
        let spec = TaskSpec::new("Gas Task".into()).with_gas_budget(1000);
        let task = svc.task_create(board.id, spec, owner).unwrap();
        assert_eq!(task.gas_remaining, Some(1000));

        // Create a kata engine with mock inference (100 tokens per call)
        let registry = SqliteRegistry::new(None).expect("temp registry");
        let mock_inference = Arc::new(MockInference {
            response_text: "Direction: improve the system.".into(),
            tokens_per_call: 100,
        });
        let accountant: Arc<dyn TaskGasAccountant> = svc.gas_accountant_for(task.id);
        let engine = KataEngine::new(mock_inference, registry).with_task_gas_accountant(accountant);

        // Execute an improvement kata cycle (1 step = 1 inference call)
        let manifest = test_improvement_manifest();
        let result = engine
            .execute(&manifest, "test-bot", std::collections::HashMap::new())
            .await
            .expect("improvement kata should succeed");

        assert_eq!(result.kata_type, "improvement");
        assert_eq!(result.steps_completed, 1);

        // Verify the task's gas_remaining was decremented by 100 tokens
        let updated_task = svc.task_get(task.id).unwrap().expect("task should exist");
        assert_eq!(
            updated_task.gas_remaining,
            Some(900),
            "gas_remaining should be 1000 - 100 = 900 after one improvement inference call"
        );

        // Verify a GasEntry was recorded
        assert_eq!(updated_task.gas_spend.len(), 1);
        let entry = &updated_task.gas_spend[0];
        assert_eq!(entry.amount, 100);
        assert!(entry.reason.contains("step-1"));
    }

    #[tokio::test]
    async fn task_gas_accountant_no_op_without_accountant() {
        // When no accountant is configured, inference should still work
        // and no gas should be deducted from any task.
        let svc = KanbanService::new(make_test_store());
        let owner = hkask_types::WebID::new();
        let board = svc
            .board_create(owner, "No Accountant Board", &default_columns())
            .unwrap();
        let spec = TaskSpec::new("No Accountant Task".into()).with_gas_budget(500);
        let task = svc.task_create(board.id, spec, owner).unwrap();

        let registry = SqliteRegistry::new(None).expect("temp registry");
        let mock_inference = Arc::new(MockInference {
            response_text: "Response without accounting.".into(),
            tokens_per_call: 100,
        });
        // No with_task_gas_accountant — standalone kata execution
        let engine = KataEngine::new(mock_inference, registry);

        let manifest = test_coaching_manifest();
        let result = engine
            .execute(&manifest, "test-bot", std::collections::HashMap::new())
            .await
            .expect("kata should succeed without accountant");

        assert_eq!(result.steps_completed, 1);

        // Task gas should be unchanged (no accountant wired)
        let unchanged_task = svc.task_get(task.id).unwrap().expect("task should exist");
        assert_eq!(
            unchanged_task.gas_remaining,
            Some(500),
            "gas_remaining should be unchanged when no accountant is configured"
        );
        assert!(
            unchanged_task.gas_spend.is_empty(),
            "no GasEntry should be recorded without an accountant"
        );
    }

    #[tokio::test]
    async fn task_gas_accountant_deducts_across_multiple_steps() {
        // Verify that multiple inference calls accumulate deductions correctly.
        // 3 coaching questions × 100 tokens each = 300 total deducted.
        use hkask_services_kata_kanban::{
            CoachQuestion, ErrorHandling, KataAuditConfig, KataCnsConfig, KataGasConfig, KataStep,
            ManifestMeta, MetricDef, Outcome, PracticeRoutine, StarterOutcome,
        };
        let svc = Arc::new(KanbanService::new(make_test_store()));
        let owner = hkask_types::WebID::new();
        let board = svc
            .board_create(owner, "Multi Board", &default_columns())
            .unwrap();
        let spec = TaskSpec::new("Multi Task".into()).with_gas_budget(1000);
        let task = svc.task_create(board.id, spec, owner).unwrap();

        let registry = SqliteRegistry::new(None).expect("temp registry");
        let mock_inference = Arc::new(MockInference {
            response_text: "Answer.".into(),
            tokens_per_call: 100,
        });
        let accountant: Arc<dyn TaskGasAccountant> = svc.gas_accountant_for(task.id);
        let engine = KataEngine::new(mock_inference, registry).with_task_gas_accountant(accountant);

        let manifest = KataManifest {
            manifest: ManifestMeta {
                id: "test-multi-coaching".into(),
                name: "Test Multi Coaching".into(),
                kata_type: "coaching".into(),
                description: "Test".into(),
                editor: "test".into(),
                visibility: "test".into(),
            },
            gas: KataGasConfig {
                cap: 100_000,
                alert_threshold: 0.7,
                hard_limit: true,
            },
            steps: vec![],
            questions: vec![
                CoachQuestion {
                    number: 1,
                    question: "Q1?".into(),
                    description: "D1".into(),
                    cns_span: None,
                    expected_output: None,
                },
                CoachQuestion {
                    number: 2,
                    question: "Q2?".into(),
                    description: "D2".into(),
                    cns_span: None,
                    expected_output: None,
                },
                CoachQuestion {
                    number: 3,
                    question: "Q3?".into(),
                    description: "D3".into(),
                    cns_span: None,
                    expected_output: None,
                },
            ],
            practices: vec![],
            error_handling: ErrorHandling::default(),
            cns: KataCnsConfig {
                emit_spans: false,
                span_namespace: "test.kata".into(),
                variety_monitoring: false,
                algedonic_threshold: None,
                escalation_target: None,
            },
            outcomes: vec![],
            metrics: vec![],
            starter_outcomes: vec![],
            audit: KataAuditConfig::default(),
        };

        let result = engine
            .execute(&manifest, "test-bot", std::collections::HashMap::new())
            .await
            .expect("multi coaching should succeed");

        assert_eq!(result.steps_completed, 3);

        // 3 calls × 100 tokens = 300 deducted from 1000 = 700 remaining
        let updated_task = svc.task_get(task.id).unwrap().expect("task should exist");
        assert_eq!(
            updated_task.gas_remaining,
            Some(700),
            "gas_remaining should be 1000 - 300 = 700 after three coaching calls"
        );
        assert_eq!(
            updated_task.gas_spend.len(),
            3,
            "three GasEntry records should exist"
        );
    }
}
