//! REPL /model handler — model listing, switching, and fuzzy search

use crate::handlers::repl_settings::ModelMeta;
use hkask_services_inference::{InferenceContext, InferenceService};
use hkask_tui::ModelSwitchResult;

pub fn populate_model_meta(state: &mut super::super::ReplState, _rt: &tokio::runtime::Handle) {
    state.repl_settings.model_meta = Some(ModelMeta {
        context_length: 16384,
        supports_thinking: false,
        capabilities: Vec::new(),
    });
}

/// Resolve `name` against the model catalog and apply it to `state`.
/// Single match => switch + populate metadata. Zero/err => store verbatim.
/// Multiple => leave model unchanged, return candidate list as detail.
///
/// Shared by the REPL `/model` handler and the TUI `SettingsBridge` so both
/// surfaces use one resolver (no parallel logic).
pub(crate) fn resolve_and_set_model(
    state: &mut super::super::ReplState,
    rt: &tokio::runtime::Handle,
    name: &str,
) -> ModelSwitchResult {
    let ctx = InferenceContext::from(state.service_context.as_ref());
    match rt.block_on(InferenceService::search_models(&ctx, name)) {
        Ok(models) if models.len() == 1 => {
            let m = &models[0];
            state.current_model = m.name.clone();
            let mut detail = String::new();
            if let Some(ref family) = m.family {
                detail.push_str(&format!("Family: {}\n", family));
            }
            if let Some(ref params) = m.parameter_size {
                detail.push_str(&format!("Parameters: {}\n", params));
            }
            if let Some(ref quant) = m.quantization_level {
                detail.push_str(&format!("Quantization: {}\n", quant));
            }
            populate_model_meta(state, rt);
            ModelSwitchResult {
                resolved_name: state.current_model.clone(),
                detail,
            }
        }
        Ok(models) if models.is_empty() => {
            state.current_model = name.to_string();
            ModelSwitchResult {
                resolved_name: name.to_string(),
                detail: "Provider unreachable — model name stored for next inference.".to_string(),
            }
        }
        Ok(models) => {
            let mut d = format!("Multiple matches ({}):\n", models.len());
            for m in &models {
                d.push_str(&format!("  {}\n", m.name));
            }
            d.push_str("Use /model <exact-name> to switch.");
            ModelSwitchResult {
                resolved_name: state.current_model.clone(),
                detail: d,
            }
        }
        Err(_) => {
            state.current_model = name.to_string();
            ModelSwitchResult {
                resolved_name: name.to_string(),
                detail: "Provider unreachable — model name stored for next inference.".to_string(),
            }
        }
    }
}

pub fn handle_model(arg1: &str, rt: &tokio::runtime::Handle, state: &mut super::super::ReplState) {
    if arg1.eq_ignore_ascii_case("refresh") || arg1.eq_ignore_ascii_case("update") {
        // Force a live re-fetch: invalidate the TTL cache, then re-list.
        hkask_services_inference::ModelCache::invalidate();
        let ctx = InferenceContext::from(state.service_context.as_ref());
        let models = rt.block_on(InferenceService::search_models(&ctx, ""));
        match models {
            Ok(models) if models.is_empty() => {
                println!(
                    "  [33mRefreshed — no models reachable. Check providers / Ollama daemon.[0m"
                );
            }
            Ok(models) => {
                println!("  [32mRefreshed — {} models available.[0m", models.len());
                println!("  Use [36m/model list[0m to browse them.");
            }
            Err(e) => {
                println!("  [31mRefresh failed: {}[0m", e);
            }
        }
        println!();
        return;
    }
    if arg1.is_empty() || arg1.eq_ignore_ascii_case("list") {
        if arg1.eq_ignore_ascii_case("list") {
            let ctx = InferenceContext::from(state.service_context.as_ref());
            let models = rt.block_on(InferenceService::search_models(&ctx, ""));
            match models {
                Ok(models) if models.is_empty() => {
                    println!("  No models found — no providers reachable.");
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
                    println!("  No models found — error listing models: {}", e);
                }
            }
        } else {
            println!("  Current model: \x1b[1m{}\x1b[0m", state.current_model);
            println!(
                "  Use \x1b[36m/model <name>\x1b[0m to switch, \x1b[36m/model <query>\x1b[0m to search"
            );
        }
    } else {
        let result = resolve_and_set_model(state, rt, arg1);
        println!("  Model set to: \x1b[1m{}\x1b[0m", result.resolved_name);
        if !result.detail.is_empty() {
            for line in result.detail.lines() {
                println!("  {}", line);
            }
        }
    }
    println!();
}
