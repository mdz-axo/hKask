//! REPL settings handler — /repl command for user-configurable inference parameters.
//!
//! Magna Carta P3 (Generative Space): all parameters are user-exposed,
//! no privileged engineer access. The /repl command surfaces every
//! inference parameter that was previously hardcoded.

use super::super::ReplState;
use hkask_types::LLMParameters;

/// Show all REPL settings.
pub(crate) fn handle_repl_show(state: &ReplState) {
    let s = &state.repl_settings;
    println!("  \x1b[1mREPL Settings\x1b[0m");
    println!();
    println!("  \x1b[36mtool_loop_limit\x1b[0m:  {}", s.tool_loop_limit);
    println!(
        "  \x1b[36mcontext_turns\x1b[0m:   {} (0 = no history)",
        s.context_turns
    );
    println!("  \x1b[36mtemperature\x1b[0m:     {}", s.temperature);
    println!("  \x1b[36mtop_p\x1b[0m:           {}", s.top_p);
    println!("  \x1b[36mtop_k\x1b[0m:           {}", s.top_k);
    println!("  \x1b[36mmin_p\x1b[0m:          {}", s.min_p);
    println!("  \x1b[36mtypical_p\x1b[0m:       {}", s.typical_p);
    println!("  \x1b[36mmax_tokens\x1b[0m:      {}", s.max_tokens);
    println!(
        "  \x1b[36mseed\x1b[0m:            {}",
        s.seed.map_or("random".to_string(), |v| v.to_string())
    );
    println!("  \x1b[36mgas_heuristic\x1b[0m:    {}", s.gas_heuristic);
    println!("  \x1b[36mgas_cap\x1b[0m:         {}", s.gas_cap);
    println!(
        "  \x1b[36mauto_compact\x1b[0m:     {}",
        if s.auto_compact { "on" } else { "off" }
    );
    if let Some(ref meta) = s.model_meta {
        println!("  \x1b[36m─ model info ─\x1b[0m");
        println!("  \x1b[36m  context_length\x1b[0m: {}", meta.context_length);
        println!(
            "  \x1b[36m  thinking\x1b[0m:       {}",
            if meta.supports_thinking { "yes" } else { "no" }
        );
        if !meta.capabilities.is_empty() {
            println!(
                "  \x1b[36m  capabilities\x1b[0m:   {}",
                meta.capabilities.join(", ")
            );
        }
    } else {
        println!("  \x1b[36m─ model info ─\x1b[0m  (not fetched yet — switch models to populate)");
    }
    println!();
}

/// Parse a /repl subcommand and apply the setting.
pub(crate) fn handle_repl_set(arg1: &str, arg2: &str, state: &mut ReplState) {
    match arg1 {
        "loops" => match arg2.parse::<usize>() {
            Ok(n) if n > 0 => {
                state.repl_settings.tool_loop_limit = n;
                println!("  tool_loop_limit set to {}", n);
            }
            Ok(0) => println!("  \x1b[31mError:\x1b[0m tool_loop_limit must be > 0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected positive integer"),
        },
        "context" => match arg2.parse::<usize>() {
            Ok(n) => {
                state.repl_settings.context_turns = n;
                if n == 0 {
                    println!("  context_turns set to 0 (history disabled)");
                } else {
                    println!("  context_turns set to {}", n);
                }
            }
            _ => println!("  \x1b[31mError:\x1b[0m expected non-negative integer"),
        },
        "temp" => match arg2.parse::<f32>() {
            Ok(v) if (0.0..=2.0).contains(&v) => {
                state.repl_settings.temperature = v;
                println!("  temperature set to {}", v);
            }
            Ok(_) => println!("  \x1b[31mError:\x1b[0m temperature must be 0.0–2.0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected float"),
        },
        "top_p" => match arg2.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => {
                state.repl_settings.top_p = v;
                println!("  top_p set to {}", v);
            }
            Ok(_) => println!("  \x1b[31mError:\x1b[0m top_p must be 0.0–1.0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected float"),
        },
        "top_k" => match arg2.parse::<u32>() {
            Ok(v) if v >= 1 => {
                state.repl_settings.top_k = v;
                println!("  top_k set to {}", v);
            }
            Ok(0) => println!("  \x1b[31mError:\x1b[0m top_k must be >= 1"),
            _ => println!("  \x1b[31mError:\x1b[0m expected positive integer"),
        },
        "min_p" => match arg2.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => {
                state.repl_settings.min_p = v;
                if v == 0.0 {
                    println!("  min_p set to 0.0 (disabled)");
                } else {
                    println!("  min_p set to {}", v);
                }
            }
            Ok(_) => println!("  \x1b[31mError:\x1b[0m min_p must be 0.0–1.0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected float"),
        },
        "typical_p" => match arg2.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => {
                state.repl_settings.typical_p = v;
                if v == 0.0 {
                    println!("  typical_p set to 0.0 (disabled)");
                } else {
                    println!("  typical_p set to {}", v);
                }
            }
            Ok(_) => println!("  \x1b[31mError:\x1b[0m typical_p must be 0.0–1.0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected float"),
        },
        "max_tokens" => match arg2.parse::<u32>() {
            Ok(v) if v > 0 => {
                state.repl_settings.max_tokens = v;
                println!("  max_tokens set to {}", v);
            }
            Ok(0) => println!("  \x1b[31mError:\x1b[0m max_tokens must be > 0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected positive integer"),
        },
        "seed" => match arg2 {
            "off" | "random" => {
                state.repl_settings.seed = None;
                println!("  seed set to random");
            }
            _ => match arg2.parse::<u32>() {
                Ok(v) => {
                    state.repl_settings.seed = Some(v);
                    println!("  seed set to {}", v);
                }
                _ => println!("  \x1b[31mError:\x1b[0m expected u32 or 'off'"),
            },
        },
        "gas_heuristic" => match arg2.parse::<u64>() {
            Ok(v) if v > 0 => {
                state.repl_settings.gas_heuristic = v;
                println!("  gas_heuristic set to {}", v);
            }
            Ok(0) => println!("  \x1b[31mError:\x1b[0m gas_heuristic must be > 0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected positive integer"),
        },
        "gas_cap" => match arg2.parse::<u64>() {
            Ok(v) if v > 0 => {
                state.repl_settings.gas_cap = v;
                println!("  gas_cap set to {}", v);
            }
            Ok(0) => println!("  \x1b[31mError:\x1b[0m gas_cap must be > 0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected positive integer"),
        },
        "auto_compact" => match arg2 {
            "on" | "true" => {
                state.repl_settings.auto_compact = true;
                println!("  auto_compact: on (context will be compacted at 87.5% of window)");
            }
            "off" | "false" => {
                state.repl_settings.auto_compact = false;
                println!("  auto_compact: off (manual compaction only)");
            }
            _ => println!("  \x1b[31mError:\x1b[0m expected 'on' or 'off'"),
        },
        "reset" => {
            state.repl_settings = ReplSettings::default();
            println!("  \x1b[32mAll REPL settings reset to defaults\x1b[0m");
            handle_repl_show(state);
        }
        "" | "status" => {
            handle_repl_show(state);
        }
        _ => {
            println!("  Unknown setting: \x1b[31mrepl_{}\x1b[0m", arg1);
            println!("  Type \x1b[36m/repl\x1b[0m to see all settings.");
        }
    }
}

/// Default REPL settings.
#[derive(Debug, Clone)]
pub(crate) struct ReplSettings {
    /// Maximum tool-call loop iterations per turn.
    pub tool_loop_limit: usize,
    /// Past conversation turns to append as context (0 = no history).
    pub context_turns: usize,
    /// LLM sampling temperature.
    pub temperature: f32,
    /// Nucleus sampling threshold.
    pub top_p: f32,
    /// Top-k filter.
    pub top_k: u32,
    /// Min-p threshold.
    pub min_p: f32,
    /// Typical-p threshold (locally typical sampling).
    pub typical_p: f32,
    /// Maximum completion tokens per call.
    pub max_tokens: u32,
    /// Deterministic seed (None = random).
    pub seed: Option<u32>,
    /// Per-turn gas reservation heuristic.
    pub gas_heuristic: u64,
    /// Total session energy budget cap.
    pub gas_cap: u64,
    /// Auto-compact when context reaches 87.5% of model's window.
    /// When false, the user must compact manually.
    pub auto_compact: bool,
    /// Read-only model metadata — populated by /model switch.
    /// None until the first model detail fetch succeeds.
    pub model_meta: Option<ModelMeta>,
}

/// Model metadata fetched from Ollama's /api/show endpoint.
/// Read-only — populated automatically when the model changes.
#[derive(Debug, Clone)]
pub(crate) struct ModelMeta {
    pub context_length: u32,
    pub supports_thinking: bool,
    pub capabilities: Vec<String>,
}

impl Default for ReplSettings {
    fn default() -> Self {
        Self {
            tool_loop_limit: 21,
            context_turns: 3,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            max_tokens: 512,
            seed: None,
            gas_heuristic: 500,
            gas_cap: 10_000,
            auto_compact: true,
            model_meta: None,
        }
    }
}

/// Build LLMParameters from ReplSettings for inference calls.
pub(crate) fn to_llm_params(settings: &ReplSettings) -> LLMParameters {
    LLMParameters {
        temperature: settings.temperature,
        top_p: settings.top_p,
        top_k: settings.top_k,
        min_p: settings.min_p,
        typical_p: settings.typical_p,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        max_tokens: settings.max_tokens,
        seed: settings.seed.map(|s| s as u64),
    }
}
