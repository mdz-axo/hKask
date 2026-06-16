//! Settings command — CLI surface for REPL inference settings.
//!
//! Persists settings to `~/.config/hkask/settings.json` so CLI, API, and
//! interactive REPL share the same configuration. Magna Carta P3 (Generative
//! Space): all settings exposed equally across every surface.
//!
//! Delegates load/save to `hkask_services::settings` for shared persistence.

use crate::cli::SettingsAction;
use crate::repl::handlers::ReplSettings;
use hkask_services::settings::{load_settings, save_settings};

/// CLI handler for `kask settings {show,set,reset}`.
/// REQ: CLI-072
/// pre:  action is a valid SettingsAction variant (Show, Set, Reset)
/// post: loads/saves REPL settings from ~/.config/hkask/settings.json; prints current values or confirmation
pub fn run(action: SettingsAction) {
    match action {
        SettingsAction::Show { name } => {
            let settings: ReplSettings = load_settings();
            match name {
                None => show_all(&settings),
                Some(key) => show_one(&settings, &key),
            }
        }
        SettingsAction::Set { name, value } => {
            let mut settings: ReplSettings = load_settings();
            if apply_setting(&mut settings, &name, &value) {
                match save_settings(&settings) {
                    Ok(()) => println!("Saved."),
                    Err(e) => eprintln!("Error saving settings: {}", e),
                }
            }
        }
        SettingsAction::Reset => {
            let settings = ReplSettings::default();
            match save_settings(&settings) {
                Ok(()) => {
                    println!("Settings reset to defaults:");
                    show_all(&settings);
                }
                Err(e) => eprintln!("Error saving settings: {}", e),
            }
        }
    }
}

fn show_all(settings: &ReplSettings) {
    let s = settings;
    println!("tool_loop_limit:  {}", s.tool_loop_limit);
    println!("context_turns:    {} (0 = no history)", s.context_turns);
    println!("temperature:      {}", s.temperature);
    println!("top_p:            {}", s.top_p);
    println!("top_k:            {}", s.top_k);
    println!("min_p:            {}", s.min_p);
    println!("typical_p:        {}", s.typical_p);
    println!("max_tokens:       {}", s.max_tokens);
    match s.seed {
        Some(v) => println!("seed:             {}", v),
        None => println!("seed:             random"),
    }
    println!("gas_heuristic:    {}", s.gas_heuristic);
    println!("gas_cap:          {}", s.gas_cap);
    println!(
        "auto_condense:     {}",
        if s.auto_condense { "on" } else { "off" }
    );
    if let Some(ref meta) = s.model_meta {
        println!(
            "context_length:   {} (model read-only)",
            meta.context_length
        );
        println!(
            "supports_thinking: {}",
            if meta.supports_thinking { "yes" } else { "no" }
        );
    }
    println!("── model defaults ──");
    println!("generation_model: {}", s.generation_model);
    println!("embedding_model:  {}", s.embedding_model);
    println!("classifier_model: {}", s.classifier_model);
    println!("ocr_model:        {}", s.ocr_model);
    println!("── ocr thresholds ──");
    println!("ocr_simple_max:   {}", s.ocr_simple_max);
    println!("ocr_moderate_max: {}", s.ocr_moderate_max);
    println!("ocr_sample_rate:  {}", s.ocr_sample_rate);
}

fn show_one(settings: &ReplSettings, key: &str) {
    let s = settings;
    match key {
        "tool_loop_limit" | "loops" => println!("{}", s.tool_loop_limit),
        "context_turns" | "context" => println!("{}", s.context_turns),
        "temperature" | "temp" => println!("{}", s.temperature),
        "top_p" => println!("{}", s.top_p),
        "top_k" => println!("{}", s.top_k),
        "min_p" => println!("{}", s.min_p),
        "typical_p" => println!("{}", s.typical_p),
        "max_tokens" => println!("{}", s.max_tokens),
        "seed" => match s.seed {
            Some(v) => println!("{}", v),
            None => println!("random"),
        },
        "gas_heuristic" => println!("{}", s.gas_heuristic),
        "gas_cap" => println!("{}", s.gas_cap),
        "auto_condense" => println!("{}", if s.auto_condense { "on" } else { "off" }),
        "context_length" => match s.model_meta {
            Some(ref m) => println!("{}", m.context_length),
            None => println!("(not set)"),
        },
        "generation_model" | "gen_model" => println!("{}", s.generation_model),
        "embedding_model" | "emb_model" => println!("{}", s.embedding_model),
        "classifier_model" | "cls_model" => println!("{}", s.classifier_model),
        "ocr_model" => println!("{}", s.ocr_model),
        "ocr_simple_max" => println!("{}", s.ocr_simple_max),
        "ocr_moderate_max" => println!("{}", s.ocr_moderate_max),
        "ocr_sample_rate" => println!("{}", s.ocr_sample_rate),
        _ => eprintln!("Unknown setting: {}", key),
    }
}

/// Apply a key=value pair to settings. Returns true if the setting was recognized.
fn apply_setting(settings: &mut ReplSettings, name: &str, value: &str) -> bool {
    match name {
        "tool_loop_limit" | "loops" => {
            if let Ok(n) = value.parse::<usize>() {
                if n > 0 {
                    settings.tool_loop_limit = n;
                } else {
                    eprintln!("Error: tool_loop_limit must be > 0");
                    return false;
                }
            } else {
                eprintln!("Error: expected positive integer");
                return false;
            }
        }
        "context_turns" | "context" => {
            if let Ok(n) = value.parse::<usize>() {
                settings.context_turns = n;
            } else {
                eprintln!("Error: expected non-negative integer");
                return false;
            }
        }
        "temperature" | "temp" => {
            if let Ok(v) = value.parse::<f32>() {
                if (0.0..=2.0).contains(&v) {
                    settings.temperature = v;
                } else {
                    eprintln!("Error: temperature must be 0.0–2.0");
                    return false;
                }
            } else {
                eprintln!("Error: expected float");
                return false;
            }
        }
        "top_p" => {
            if let Ok(v) = value.parse::<f32>() {
                if (0.0..=1.0).contains(&v) {
                    settings.top_p = v;
                } else {
                    eprintln!("Error: top_p must be 0.0–1.0");
                    return false;
                }
            } else {
                eprintln!("Error: expected float");
                return false;
            }
        }
        "top_k" => {
            if let Ok(v) = value.parse::<u32>() {
                if v >= 1 {
                    settings.top_k = v;
                } else {
                    eprintln!("Error: top_k must be >= 1");
                    return false;
                }
            } else {
                eprintln!("Error: expected positive integer");
                return false;
            }
        }
        "min_p" => {
            if let Ok(v) = value.parse::<f32>() {
                if (0.0..=1.0).contains(&v) {
                    settings.min_p = v;
                } else {
                    eprintln!("Error: min_p must be 0.0–1.0");
                    return false;
                }
            } else {
                eprintln!("Error: expected float");
                return false;
            }
        }
        "typical_p" => {
            if let Ok(v) = value.parse::<f32>() {
                if (0.0..=1.0).contains(&v) {
                    settings.typical_p = v;
                } else {
                    eprintln!("Error: typical_p must be 0.0–1.0");
                    return false;
                }
            } else {
                eprintln!("Error: expected float");
                return false;
            }
        }
        "max_tokens" => {
            if let Ok(v) = value.parse::<u32>() {
                if v > 0 {
                    settings.max_tokens = v;
                } else {
                    eprintln!("Error: max_tokens must be > 0");
                    return false;
                }
            } else {
                eprintln!("Error: expected positive integer");
                return false;
            }
        }
        "seed" => {
            if value == "off" || value == "random" {
                settings.seed = None;
            } else if let Ok(v) = value.parse::<u32>() {
                settings.seed = Some(v);
            } else {
                eprintln!("Error: expected u32 or 'off'");
                return false;
            }
        }
        "gas_heuristic" => {
            if let Ok(v) = value.parse::<u64>() {
                if v > 0 {
                    settings.gas_heuristic = v;
                } else {
                    eprintln!("Error: gas_heuristic must be > 0");
                    return false;
                }
            } else {
                eprintln!("Error: expected positive integer");
                return false;
            }
        }
        "gas_cap" => {
            if let Ok(v) = value.parse::<u64>() {
                if v > 0 {
                    settings.gas_cap = v;
                } else {
                    eprintln!("Error: gas_cap must be > 0");
                    return false;
                }
            } else {
                eprintln!("Error: expected positive integer");
                return false;
            }
        }
        "auto_condense" => match value {
            "on" | "true" => settings.auto_condense = true,
            "off" | "false" => settings.auto_condense = false,
            _ => {
                eprintln!("Error: expected 'on' or 'off'");
                return false;
            }
        },
        "generation_model" | "gen_model" => settings.generation_model = value.to_string(),
        "embedding_model" | "emb_model" => settings.embedding_model = value.to_string(),
        "classifier_model" | "cls_model" => settings.classifier_model = value.to_string(),
        "ocr_model" => settings.ocr_model = value.to_string(),
        "ocr_simple_max" => match value.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => settings.ocr_simple_max = v,
            Ok(_) => {
                eprintln!("Error: ocr_simple_max must be 0.0–1.0");
                return false;
            }
            _ => {
                eprintln!("Error: expected float");
                return false;
            }
        },
        "ocr_moderate_max" => match value.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => settings.ocr_moderate_max = v,
            Ok(_) => {
                eprintln!("Error: ocr_moderate_max must be 0.0–1.0");
                return false;
            }
            _ => {
                eprintln!("Error: expected float");
                return false;
            }
        },
        "ocr_sample_rate" => match value.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => settings.ocr_sample_rate = v,
            Ok(_) => {
                eprintln!("Error: ocr_sample_rate must be 0.0–1.0");
                return false;
            }
            _ => {
                eprintln!("Error: expected float");
                return false;
            }
        },
        _ => {
            eprintln!("Unknown setting: {}", name);
            return false;
        }
    }
    println!("{} = {}", name, value);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_settings() -> ReplSettings {
        ReplSettings::default()
    }

    // REQ: Invalid values are rejected (function returns false, value unchanged)

    #[test]
    fn apply_setting_rejects_zero_loop_limit() {
        let mut s = default_settings();
        assert!(!apply_setting(&mut s, "loops", "0"));
        assert_eq!(s.tool_loop_limit, 21);
    }

    #[test]
    fn apply_setting_rejects_negative_loop_limit() {
        let mut s = default_settings();
        assert!(!apply_setting(&mut s, "loops", "-1"));
        assert_eq!(s.tool_loop_limit, 21);
    }

    #[test]
    fn apply_setting_rejects_temperature_oor() {
        let mut s = default_settings();
        assert!(!apply_setting(&mut s, "temp", "3.0"));
        assert!((s.temperature - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_setting_rejects_top_p_oor() {
        let mut s = default_settings();
        assert!(!apply_setting(&mut s, "top_p", "1.5"));
        assert!((s.top_p - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_setting_rejects_top_k_zero() {
        let mut s = default_settings();
        assert!(!apply_setting(&mut s, "top_k", "0"));
        assert_eq!(s.top_k, 40);
    }

    #[test]
    fn apply_setting_rejects_garbage_value() {
        let mut s = default_settings();
        assert!(!apply_setting(&mut s, "temp", "not-a-number"));
        assert!((s.temperature - 0.7).abs() < f32::EPSILON);
    }

    // REQ: Valid values are accepted (function returns true, value updated)

    #[test]
    fn apply_setting_accepts_valid_temperature() {
        let mut s = default_settings();
        assert!(apply_setting(&mut s, "temp", "0.3"));
        assert!((s.temperature - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_setting_accepts_valid_loop_limit() {
        let mut s = default_settings();
        assert!(apply_setting(&mut s, "loops", "100"));
        assert_eq!(s.tool_loop_limit, 100);
    }

    #[test]
    fn apply_setting_accepts_auto_condense_off() {
        let mut s = default_settings();
        assert!(apply_setting(&mut s, "auto_condense", "off"));
        assert!(!s.auto_condense);
    }

    #[test]
    fn apply_setting_accepts_auto_condense_on() {
        let mut s = default_settings();
        s.auto_condense = false;
        assert!(apply_setting(&mut s, "auto_condense", "true"));
        assert!(s.auto_condense);
    }

    #[test]
    fn apply_setting_accepts_seed_value() {
        let mut s = default_settings();
        assert!(apply_setting(&mut s, "seed", "42"));
        assert_eq!(s.seed, Some(42));
    }

    #[test]
    fn apply_setting_accepts_seed_off() {
        let mut s = default_settings();
        s.seed = Some(99);
        assert!(apply_setting(&mut s, "seed", "off"));
        assert_eq!(s.seed, None);
    }
}
