//! REPL /model handler — model listing, switching, and fuzzy search

use crate::repl::handlers::repl_settings::ModelMeta;
use hkask_inference::InferenceConfig;
use hkask_services::{InferenceContext, InferenceService};

pub(crate) fn populate_model_meta(
    state: &mut super::super::ReplState,
    rt: &tokio::runtime::Handle,
) {
    let config = state.service_context.config().inference_config.clone();
    let model = state.current_model.clone();
    if let Some(show) = rt.block_on(fetch_model_show(&config, &model)) {
        let context_length = show.context_length().unwrap_or(4096);
        let supports_thinking = show.supports_thinking();
        let capabilities = show.capabilities.unwrap_or_default();
        state.repl_settings.model_meta = Some(ModelMeta {
            context_length,
            supports_thinking,
            capabilities,
        });
        let thinking_str = if supports_thinking {
            "thinking ✓"
        } else {
            ""
        };
        println!("  Window: {} tokens  {}", context_length, thinking_str);
    }
}

/// Fetch per-model detail from Ollama's `/api/show` endpoint.
/// Returns `None` if the endpoint is unreachable or the model is not found.
async fn fetch_model_show(config: &InferenceConfig, model: &str) -> Option<OkapiModelShow> {
    let client = config.build_client().ok()?;
    let request = client
        .get(format!("{}/api/show", config.ollama_base_url))
        .query(&[("name", model)]);
    match request.send().await {
        Ok(resp) => resp.json::<OkapiModelShow>().await.ok(),
        Err(_) => None,
    }
}

/// Per-model detail from Ollama's `/api/show` endpoint.
#[derive(Debug, Clone, serde::Deserialize)]
struct OkapiModelShow {
    #[serde(default)]
    pub model_info: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
}

impl OkapiModelShow {
    fn context_length(&self) -> Option<u32> {
        self.model_info.as_ref()?.iter().find_map(|(k, v)| {
            if k.ends_with(".context_length") {
                v.as_u64().map(|n| n as u32)
            } else {
                None
            }
        })
    }

    fn supports_thinking(&self) -> bool {
        if let Some(ref caps) = self.capabilities
            && caps.iter().any(|c| c == "reasoning" || c == "thinking")
        {
            return true;
        }
        if let Some(ref info) = self.model_info
            && info.iter().any(|(k, v)| {
                (k.contains("reasoning") || k.contains("thinking")) && v.as_bool().unwrap_or(false)
            })
        {
            return true;
        }
        false
    }
}

pub(crate) fn handle_model(
    arg1: &str,
    rt: &tokio::runtime::Handle,
    state: &mut super::super::ReplState,
) {
    if arg1.is_empty() || arg1.eq_ignore_ascii_case("list") {
        if arg1.eq_ignore_ascii_case("list") {
            let ctx = InferenceContext::from(state.service_context.as_ref());
            let models = rt.block_on(InferenceService::search_models(&ctx, ""));
            match models {
                Ok(models) if models.is_empty() => {
                    println!("  No models found — Okapi may be unreachable.");
                }
                Ok(models) => {
                    println!("  \x1b[1mAvailable models ({}):\x1b[0m", models.len());
                    println!("  {:<30} {:<12} {:<15} SIZE", "NAME", "FAMILY", "PARAMS");
                    println!("  {}", "-".repeat(70));
                    for m in &models {
                        let family = m.family.as_deref().unwrap_or("-");
                        let params = m.parameter_size.as_deref().unwrap_or("-");
                        let size_str = m
                            .size_bytes
                            .map(|s| format!("{:.1} GB", s as f64 / 1_073_741_824.0))
                            .unwrap_or_else(|| "-".to_string());
                        println!(
                            "  \x1b[36m{:<30}\x1b[0m {:<12} {:<15} {}",
                            m.name, family, params, size_str
                        );
                    }
                    println!();
                    println!("  Use \x1b[36m/model <name>\x1b[0m to switch to a specific model");
                }
                Err(e) => {
                    println!("  No models found — Okapi may be unreachable: {}", e);
                }
            }
        } else {
            println!("  Current model: \x1b[1m{}\x1b[0m", state.current_model);
            println!(
                "  Use \x1b[36m/model <name>\x1b[0m to switch, \x1b[36m/model <query>\x1b[0m to search"
            );
        }
    } else {
        let ctx = InferenceContext::from(state.service_context.as_ref());
        match rt.block_on(InferenceService::search_models(&ctx, arg1)) {
            Ok(models) if models.is_empty() => {
                state.current_model = arg1.to_string();
                println!("  Model set to: \x1b[1m{}\x1b[0m", state.current_model);
                println!(
                    "  \x1b[2m(Okapi unreachable — model name stored for next inference)\x1b[0m"
                );
            }
            Ok(models) if models.len() == 1 => {
                state.current_model = models[0].name.clone();
                println!("  Model set to: \x1b[1m{}\x1b[0m", state.current_model);
                if let Some(ref family) = models[0].family {
                    println!("  Family: {}", family);
                }
                if let Some(ref params) = models[0].parameter_size {
                    println!("  Parameters: {}", params);
                }
                if let Some(ref quant) = models[0].quantization_level {
                    println!("  Quantization: {}", quant);
                }
                populate_model_meta(state, rt);
            }
            Ok(models) => {
                println!(
                    "  \x1b[1mModels matching '\x1b[36m{}\x1b[0m\x1b[1m' ({}):\x1b[0m",
                    arg1,
                    models.len()
                );
                println!("  {:<30} {:<12} {:<15} SIZE", "NAME", "FAMILY", "PARAMS");
                println!("  {}", "-".repeat(70));
                for m in &models {
                    let family = m.family.as_deref().unwrap_or("-");
                    let params = m.parameter_size.as_deref().unwrap_or("-");
                    let size_str = m
                        .size_bytes
                        .map(|s| format!("{:.1} GB", s as f64 / 1_073_741_824.0))
                        .unwrap_or_else(|| "-".to_string());
                    println!(
                        "  \x1b[36m{:<30}\x1b[0m {:<12} {:<15} {}",
                        m.name, family, params, size_str
                    );
                }
                println!();
                println!("  Use \x1b[36m/model <name>\x1b[0m to switch to a specific model");
            }
            Err(_) => {
                state.current_model = arg1.to_string();
                println!("  Model set to: \x1b[1m{}\x1b[0m", state.current_model);
                println!(
                    "  \x1b[2m(Okapi unreachable — model name stored for next inference)\x1b[0m"
                );
            }
        }
    }
    println!();
}
