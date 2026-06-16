//! Live backend integration test — gated on environment variables.
//!
//! Tests that the InferenceRouter correctly routes to DeepInfra and Together
//! backends with real API calls. Skipped when API keys are not set.
//!
//! Run with: DI_API_KEY=... TG_API_KEY=... cargo test -p hkask-inference --test live_backends -- --ignored

use hkask_inference::{InferenceConfig, InferenceRouter, ProviderId};
use hkask_types::LLMParameters;
use hkask_types::ports::InferencePort;

fn load_env() {
    // Load .env from project root
    let dotenv_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../.env");
    if let Ok(contents) = std::fs::read_to_string(dotenv_path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if std::env::var(k).is_err() {
                    // SAFETY: test-only — runs in isolated test process
                    unsafe {
                        std::env::set_var(k, v);
                    }
                }
            }
        }
    }
}

fn condenser_params() -> LLMParameters {
    LLMParameters {
        temperature: 0.3,
        top_p: 0.9,
        top_k: 40,
        min_p: 0.0,
        typical_p: 0.0,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        max_tokens: 200,
        seed: None,
        disable_thinking: true,
        adapter: None,
    }
}

fn make_config(provider: ProviderId, base_url: &str, api_key: &str) -> InferenceConfig {
    InferenceConfig {
        default_provider: provider,
        ollama_base_url: if matches!(provider, ProviderId::Ollama) {
            base_url.to_string()
        } else {
            String::new()
        },
        deepinfra_base_url: if matches!(provider, ProviderId::DeepInfra) {
            base_url.to_string()
        } else {
            String::new()
        },
        deepinfra_api_key: if matches!(provider, ProviderId::DeepInfra) {
            api_key.to_string()
        } else {
            String::new()
        },
        together_base_url: if matches!(provider, ProviderId::Together) {
            base_url.to_string()
        } else {
            String::new()
        },
        together_api_key: if matches!(provider, ProviderId::Together) {
            api_key.to_string()
        } else {
            String::new()
        },
        ..Default::default()
    }
}

// REQ: LIVE-001 — DeepInfra backend produces valid summary with disable_thinking
#[tokio::test]
#[ignore = "requires DI_API_KEY or DEEPINFRA_API_KEY"]
async fn deepinfra_summarization() {
    load_env();
    let api_key = std::env::var("DI_API_KEY")
        .or_else(|_| std::env::var("DEEPINFRA_API_KEY"))
        .expect("DI_API_KEY or DEEPINFRA_API_KEY must be set");

    let config = make_config(ProviderId::DeepInfra, "https://api.deepinfra.com", &api_key);
    let router = InferenceRouter::new(config);

    let prompt = "Summarize this conversation for context compaction. Preserve: key decisions, file paths mentioned, error states encountered, code changes made, and the current task goal.\n\nCurrent task: Continue implementing the condenser\n\nConversation history:\n[user]: Read the file src/main.rs\n\n[assistant]: The file contains a main function that starts the server\n\n[user]: Add error handling to the server startup\n\n[assistant]: I added a match statement for the Result from server::start";

    let result = router
        .generate_with_model(
            &prompt,
            &condenser_params(),
            Some("DI/meta-llama/Llama-3.3-70B-Instruct-Turbo"),
        )
        .await
        .expect("DeepInfra inference should succeed");

    assert!(!result.text.is_empty(), "Summary should not be empty");
    assert!(result.text.len() > 20, "Summary too short: {}", result.text);
    eprintln!(
        "DeepInfra summary: {}",
        &result.text[..200.min(result.text.len())]
    );
    eprintln!(
        "  model: {}, tokens: {}",
        result.model, result.usage.total_tokens
    );
}

// REQ: LIVE-002 — Together backend produces valid summary with disable_thinking
#[tokio::test]
#[ignore = "requires TOGETHER_API_KEY"]
async fn together_summarization() {
    load_env();
    let api_key = std::env::var("TOGETHER_API_KEY").expect("TOGETHER_API_KEY must be set");

    let config = make_config(ProviderId::Together, "https://api.together.xyz", &api_key);
    let router = InferenceRouter::new(config);

    let prompt = "Summarize this conversation for context compaction. Preserve: key decisions, file paths mentioned, error states encountered, code changes made, and the current task goal.\n\nCurrent task: Continue implementing the condenser\n\nConversation history:\n[user]: Read the file src/main.rs\n\n[assistant]: The file contains a main function that starts the server\n\n[user]: Add error handling to the server startup\n\n[assistant]: I added a match statement for the Result from server::start";

    let result = router
        .generate_with_model(
            &prompt,
            &condenser_params(),
            Some("TG/meta-llama/Llama-3.3-70B-Instruct-Turbo"),
        )
        .await
        .expect("Together inference should succeed");

    assert!(!result.text.is_empty(), "Summary should not be empty");
    assert!(result.text.len() > 20, "Summary too short: {}", result.text);
    eprintln!(
        "Together summary: {}",
        &result.text[..200.min(result.text.len())]
    );
    eprintln!(
        "  model: {}, tokens: {}",
        result.model, result.usage.total_tokens
    );
}
