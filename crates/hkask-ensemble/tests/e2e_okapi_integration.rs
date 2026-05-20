//! End-to-End Tests for Okapi Integration
//!
//! These tests require a running Okapi instance on localhost:11435.
//! Set OKAPI_E2E_TEST=1 to enable these tests.

#[cfg(test)]
mod e2e_tests {
    use hkask_ensemble::{
        GenerateOptions, GenerateRequest,
        adapters::{OkapiCapabilityFetcher, OkapiHttpClient},
        ports::{CapabilityProvider, InferenceClient, MetricsSource},
    };

    fn is_e2e_enabled() -> bool {
        std::env::var("OKAPI_E2E_TEST")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
    }

    fn okapi_base_url() -> String {
        std::env::var("OKAPI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11435".to_string())
    }

    #[tokio::test]
    #[ignore = "requires running Okapi instance"]
    async fn e2e_test_okapi_capabilities() {
        if !is_e2e_enabled() {
            return;
        }

        let fetcher = OkapiCapabilityFetcher::new(&okapi_base_url());
        let capabilities = fetcher
            .get_capabilities()
            .await
            .expect("Failed to get capabilities");

        println!("Okapi Capabilities: {:?}", capabilities);

        // Verify basic capabilities
        assert!(!capabilities.runner_type.is_empty());
        assert!(
            capabilities.runner_type == "ollamarunner" || capabilities.runner_type == "llamarunner"
        );
    }

    #[tokio::test]
    #[ignore = "requires running Okapi instance"]
    async fn e2e_test_okapi_generate() {
        if !is_e2e_enabled() {
            return;
        }

        let client = OkapiHttpClient::new(&okapi_base_url());
        let request = GenerateRequest {
            model: "qwen3:8b".to_string(),
            prompt: "What is 2+2? Answer with just the number.".to_string(),
            options: Some(GenerateOptions {
                n_probs: Some(3),
                temperature: Some(0.0),
                max_tokens: Some(10),
            }),
        };

        let response = client.generate(&request).await.expect("Failed to generate");

        println!("Generate response: {}", response.response);
        assert!(!response.response.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires running Okapi instance"]
    async fn e2e_test_okapi_chat() {
        if !is_e2e_enabled() {
            return;
        }

        let client = OkapiHttpClient::new(&okapi_base_url());
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": "Say hello in one word."
        })];

        let response = client
            .chat(messages, "qwen3:8b".to_string())
            .await
            .expect("Failed to chat");

        println!("Chat response: {:?}", response);
    }

    #[tokio::test]
    #[ignore = "requires running Okapi instance"]
    async fn e2e_test_capability_aware_validation() {
        if !is_e2e_enabled() {
            return;
        }

        use hkask_templates::{
            capability_validator::CapabilityAwareValidator,
            contract_validator::{OkapiRequirements, RegistrationFrontmatter},
        };
        use hkask_types::TemplateType;

        let fetcher = OkapiCapabilityFetcher::new(&okapi_base_url());
        let validator = CapabilityAwareValidator::from_provider(
            &fetcher,
            vec!["classify".to_string(), "recognize".to_string()],
        )
        .await
        .expect("Failed to create validator");

        let frontmatter = RegistrationFrontmatter {
            template_type: TemplateType::Prompt,
            domain: "WordAct".to_string(),
            requires_okapi: Some(OkapiRequirements {
                n_probs: Some(5),
                grammar: None,
                adapter: None,
            }),
            confidence: None,
            lexicon_terms: vec!["classify".to_string()],
            contract: None,
        };

        let result = validator.validate(&frontmatter);
        println!("Validation result: {:?}", result);

        // Should succeed if Okapi has token_probs capability
        let capabilities = fetcher
            .get_capabilities()
            .await
            .expect("Failed to get capabilities");
        if capabilities.token_probs {
            assert!(
                result.is_ok(),
                "Validation should succeed with token_probs capability"
            );
        }
    }

    #[tokio::test]
    #[ignore = "requires running Okapi instance"]
    async fn e2e_test_metrics_stream() {
        if !is_e2e_enabled() {
            return;
        }

        use hkask_ensemble::adapters::OkapiSseAdapter;

        let adapter = OkapiSseAdapter::new(&okapi_base_url());
        let metrics = adapter.next_metrics().await.expect("Failed to get metrics");

        println!("Okapi Metrics: {:?}", metrics);
        assert!(metrics.context_length > 0);
    }

    #[tokio::test]
    #[ignore = "requires running Okapi instance"]
    async fn e2e_test_confidence_router() {
        if !is_e2e_enabled() {
            return;
        }

        use hkask_ensemble::{
            confidence_router::{ConfidenceConfig, ConfidenceRouter},
            ports::GenerateRequest,
        };

        let config = ConfidenceConfig {
            threshold: 0.75,
            escalate_to_model: "qwen3:70b".to_string(),
            n_probs: 5,
        };

        let client = OkapiHttpClient::new(&okapi_base_url());
        let router = ConfidenceRouter::new(config, client);

        let request = GenerateRequest {
            model: "qwen3:8b".to_string(),
            prompt: "What is the capital of France? Answer with just the city name.".to_string(),
            options: None,
        };

        let response = router
            .generate_with_escalation(&request)
            .await
            .expect("Failed to generate");

        println!(
            "Routed response: {} (model: {})",
            response.response, response.model
        );
        assert!(!response.response.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires running Okapi instance"]
    async fn e2e_test_okapi_integration() {
        if !is_e2e_enabled() {
            return;
        }

        use hkask_cns::CnsRuntime;
        use hkask_ensemble::okapi_integration::OkapiIntegration;
        use std::sync::Arc;

        let cns_runtime = Arc::new(CnsRuntime::new());
        let integration = OkapiIntegration::new(okapi_base_url(), cns_runtime);

        println!("Okapi Integration created: {}", integration.base_url());
        println!("Capability: {:?}", integration.capability().id());

        // Note: start_metrics_translation() will run until stream ends
        // For testing, we just verify creation works
        assert_eq!(integration.base_url(), okapi_base_url());
    }
}
