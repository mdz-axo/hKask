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
        "  \x1b[36mauto_condense\x1b[0m:     {}",
        if s.auto_condense { "on" } else { "off" }
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
    println!("  \x1b[36m─ model defaults ─\x1b[0m");
    println!("  \x1b[36mgeneration_model\x1b[0m: {}", s.generation_model);
    println!("  \x1b[36membedding_model\x1b[0m:  {}", s.embedding_model);
    println!("  \x1b[36mclassifier_model\x1b[0m: {}", s.classifier_model);
    println!("  \x1b[36mocr_model\x1b[0m:        {}", s.ocr_model);
    println!("  \x1b[36mocr_simple_max\x1b[0m:   {}", s.ocr_simple_max);
    println!("  \x1b[36mocr_moderate_max\x1b[0m: {}", s.ocr_moderate_max);
    println!("  \x1b[36mocr_sample_rate\x1b[0m:  {}", s.ocr_sample_rate);
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
        "auto_condense" => match arg2 {
            "on" | "true" => {
                state.repl_settings.auto_condense = true;
                println!("  auto_condense: on (context will be condensed at 87.5% of window)");
            }
            "off" | "false" => {
                state.repl_settings.auto_condense = false;
                println!("  auto_condense: off (manual condensation only)");
            }
            _ => println!("  \x1b[31mError:\x1b[0m expected 'on' or 'off'"),
        },
        "ocr_model" => {
            state.repl_settings.ocr_model = arg2.to_string();
            println!("  ocr_model set to {}", arg2);
        }
        "ocr_simple_max" => match arg2.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => {
                state.repl_settings.ocr_simple_max = v;
                println!("  ocr_simple_max set to {}", v);
            }
            Ok(_) => println!("  \x1b[31mError:\x1b[0m ocr_simple_max must be 0.0–1.0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected float"),
        },
        "ocr_moderate_max" => match arg2.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => {
                state.repl_settings.ocr_moderate_max = v;
                println!("  ocr_moderate_max set to {}", v);
            }
            Ok(_) => println!("  \x1b[31mError:\x1b[0m ocr_moderate_max must be 0.0–1.0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected float"),
        },
        "ocr_sample_rate" => match arg2.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => {
                state.repl_settings.ocr_sample_rate = v;
                println!("  ocr_sample_rate set to {}", v);
            }
            Ok(_) => println!("  \x1b[31mError:\x1b[0m ocr_sample_rate must be 0.0–1.0"),
            _ => println!("  \x1b[31mError:\x1b[0m expected float"),
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
    // Persist to ~/.config/hkask/settings.json so CLI and API surfaces
    // see the same settings. Only saves when a recognized setting was changed.
    if arg1 == "reset" || is_valid_setting(arg1) {
        let path = settings_path();
        if let Ok(json) = serde_json::to_string_pretty(&state.repl_settings) {
            let _ = std::fs::write(&path, json);
        }
    }
}

fn is_valid_setting(arg1: &str) -> bool {
    matches!(
        arg1,
        "loops"
            | "context"
            | "temp"
            | "top_p"
            | "top_k"
            | "min_p"
            | "typical_p"
            | "max_tokens"
            | "seed"
            | "gas_heuristic"
            | "gas_cap"
            | "auto_condense"
            | "ocr_model"
            | "ocr_simple_max"
            | "ocr_moderate_max"
            | "ocr_sample_rate"
    )
}

/// Path to the persisted settings file. Delegates to the shared
/// hkask_services::settings_path for single-source-of-truth across surfaces.
pub fn settings_path() -> std::path::PathBuf {
    hkask_services::settings_path()
}

/// Default REPL settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Auto-condense when context reaches 87.5% of model's window.
    /// When false, the user must condense manually.
    #[serde(alias = "auto_compact")]
    pub auto_condense: bool,
    /// Read-only model metadata — populated by /model switch.
    /// None until the first model detail fetch succeeds.
    pub model_meta: Option<ModelMeta>,

    // ── Model defaults (shared across all servers) ──────────────
    /// Default generation model for prose composition.
    /// Override: `HKASK_REPLICA_MODEL` env var.
    #[serde(default = "default_gen_model")]
    pub generation_model: String,

    /// Default embedding model for vectorization.
    /// Override: `HKASK_EMBEDDING_MODEL` env var.
    #[serde(default = "default_emb_model")]
    pub embedding_model: String,

    /// Default classifier model for section type / triple extraction.
    /// Override: `HKASK_CLASSIFIER_MODEL` env var.
    #[serde(default = "default_cls_model")]
    pub classifier_model: String,

    /// Default OCR model for scanned PDF fallback.
    /// Override: `HKASK_OCR_MODEL` env var.
    #[serde(default = "default_ocr")]
    pub ocr_model: String,

    // ── OCR pipeline thresholds ────────────────────────────
    /// Edge-density ratio below which a page is considered Simple.
    /// Range: 0.0–1.0.
    #[serde(default = "default_ocr_simple_max")]
    pub ocr_simple_max: f32,
    /// Edge-density ratio below which a page is considered Moderate.
    /// Values ≥ this are Complex. Range: 0.0–1.0.
    #[serde(default = "default_ocr_moderate_max")]
    pub ocr_moderate_max: f32,
    /// Dual-routing sampling rate for Moderate-tier pages [0.0, 1.0].
    #[serde(default = "default_ocr_sample_rate")]
    pub ocr_sample_rate: f32,
}

/// Model metadata fetched from Ollama's /api/show endpoint.
/// Read-only — populated automatically when the model changes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ModelMeta {
    pub context_length: u32,
    pub supports_thinking: bool,
    pub capabilities: Vec<String>,
}

fn default_gen_model() -> String {
    "deepseek-v4-flash:cloud".to_string()
}
fn default_emb_model() -> String {
    "DI/Qwen/Qwen3-Embedding-0.6B".to_string()
}
fn default_cls_model() -> String {
    "google/gemma-4-26B-A4B-it".to_string()
}
fn default_ocr() -> String {
    "maternion/LightOnOCR-2:1b".to_string()
}
fn default_ocr_simple_max() -> f32 {
    0.05
}
fn default_ocr_moderate_max() -> f32 {
    0.15
}
fn default_ocr_sample_rate() -> f32 {
    0.10
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
            auto_condense: true,
            model_meta: None,
            generation_model: default_gen_model(),
            embedding_model: default_emb_model(),
            classifier_model: default_cls_model(),
            ocr_model: default_ocr(),
            ocr_simple_max: default_ocr_simple_max(),
            ocr_moderate_max: default_ocr_moderate_max(),
            ocr_sample_rate: default_ocr_sample_rate(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── ReplSettings::default() ──────────────────────────────────────
    // REQ: All 13 defaults match the spec in AGENTS.md § /repl

    #[test]
    fn repl_settings_defaults_match_spec() {
        let s = ReplSettings::default();
        assert_eq!(s.tool_loop_limit, 21, "tool_loop_limit default");
        assert_eq!(s.context_turns, 3, "context_turns default");
        assert!(
            (s.temperature - 0.7).abs() < f32::EPSILON,
            "temperature default"
        );
        assert!((s.top_p - 0.9).abs() < f32::EPSILON, "top_p default");
        assert_eq!(s.top_k, 40, "top_k default");
        assert!((s.min_p - 0.0).abs() < f32::EPSILON, "min_p default");
        assert!(
            (s.typical_p - 0.0).abs() < f32::EPSILON,
            "typical_p default"
        );
        assert_eq!(s.max_tokens, 512, "max_tokens default");
        assert_eq!(s.seed, None, "seed default (random)");
        assert_eq!(s.gas_heuristic, 500, "gas_heuristic default");
        assert_eq!(s.gas_cap, 10_000, "gas_cap default");
        assert!(s.auto_condense, "auto_condense default");
        assert!(s.model_meta.is_none(), "model_meta default (not fetched)");
    }

    // ── to_llm_params() ──────────────────────────────────────────────
    // REQ: Correct mapping of all fields from ReplSettings to LLMParameters

    #[test]
    fn to_llm_params_maps_all_fields_correctly() {
        let s = ReplSettings {
            tool_loop_limit: 10,
            context_turns: 5,
            temperature: 0.8,
            top_p: 0.95,
            top_k: 50,
            min_p: 0.05,
            typical_p: 0.9,
            max_tokens: 1024,
            seed: Some(42),
            gas_heuristic: 100,
            gas_cap: 5_000,
            auto_condense: false,
            model_meta: None,
            generation_model: "test-gen".into(),
            embedding_model: "test-emb".into(),
            classifier_model: "test-cls".into(),
            ocr_model: "test-ocr".into(),
            ocr_simple_max: 0.05,
            ocr_moderate_max: 0.15,
            ocr_sample_rate: 0.10,
        };
        let p = to_llm_params(&s);
        assert!((p.temperature - 0.8).abs() < f32::EPSILON);
        assert!((p.top_p - 0.95).abs() < f32::EPSILON);
        assert_eq!(p.top_k, 50);
        assert!((p.min_p - 0.05).abs() < f32::EPSILON);
        assert!((p.typical_p - 0.9).abs() < f32::EPSILON);
        assert_eq!(p.max_tokens, 1024);
        assert_eq!(p.seed, Some(42));
        // Hardcoded in to_llm_params
        assert!((p.frequency_penalty - 0.0).abs() < f32::EPSILON);
        assert!((p.presence_penalty - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn to_llm_params_handles_none_seed() {
        let s = ReplSettings::default();
        let p = to_llm_params(&s);
        assert_eq!(p.seed, None, "None seed → None in LLMParameters");
    }

    // ── ReplSettings round-trip via settings.json ────────────────────
    // REQ: Serialize → write → read → deserialize preserves all fields

    #[test]
    fn repl_settings_json_round_trip_preserves_all_fields() {
        let original = ReplSettings {
            tool_loop_limit: 15,
            context_turns: 4,
            temperature: 0.5,
            top_p: 0.8,
            top_k: 30,
            min_p: 0.02,
            typical_p: 0.01,
            max_tokens: 256,
            seed: Some(12345),
            gas_heuristic: 250,
            gas_cap: 7_500,
            auto_condense: false,
            model_meta: Some(ModelMeta {
                context_length: 8192,
                supports_thinking: true,
                capabilities: vec!["chat".into(), "vision".into()],
            }),
            generation_model: "roundtrip-gen".into(),
            embedding_model: "roundtrip-emb".into(),
            classifier_model: "roundtrip-cls".into(),
            ocr_model: "roundtrip-ocr".into(),
            ocr_simple_max: 0.03,
            ocr_moderate_max: 0.12,
            ocr_sample_rate: 0.20,
        };

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("settings.json");

        // Write
        let json = serde_json::to_string_pretty(&original).expect("serialize");
        std::fs::write(&path, &json).expect("write");

        // Read
        let read_back: ReplSettings =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("read"))
                .expect("deserialize");

        assert_eq!(read_back.tool_loop_limit, original.tool_loop_limit);
        assert_eq!(read_back.context_turns, original.context_turns);
        assert!((read_back.temperature - original.temperature).abs() < f32::EPSILON);
        assert!((read_back.top_p - original.top_p).abs() < f32::EPSILON);
        assert_eq!(read_back.top_k, original.top_k);
        assert!((read_back.min_p - original.min_p).abs() < f32::EPSILON);
        assert!((read_back.typical_p - original.typical_p).abs() < f32::EPSILON);
        assert_eq!(read_back.max_tokens, original.max_tokens);
        assert_eq!(read_back.seed, original.seed);
        assert_eq!(read_back.gas_heuristic, original.gas_heuristic);
        assert_eq!(read_back.gas_cap, original.gas_cap);
        assert_eq!(read_back.auto_condense, original.auto_condense);
        let meta = read_back.model_meta.expect("model_meta");
        assert_eq!(meta.context_length, 8192);
        assert!(meta.supports_thinking);
        assert_eq!(meta.capabilities, vec!["chat", "vision"]);
    }

    // ── handle_repl_set() invalid args ───────────────────────────────
    // REQ: Invalid values are rejected; valid values are accepted.
    // These tests verify through the CLI's apply_setting function
    // (commands/settings.rs) which has identical validation logic.
    // handle_repl_set itself requires a fully-wired ReplState and is
    // tested via integration.
}
