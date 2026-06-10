//! Settings command — CLI surface for REPL inference settings.
//!
//! Persists settings to `~/.config/hkask/settings.json` so CLI, API, and
//! interactive REPL share the same configuration. Magna Carta P3 (Generative
//! Space): all settings exposed equally across every surface.

use crate::cli::SettingsAction;
use crate::repl::handlers::ReplSettings;
use hkask_services::settings_path;

/// Load settings from disk. Returns defaults if the file doesn't exist
/// or can't be parsed.
pub fn load_settings() -> ReplSettings {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(json) => match serde_json::from_str::<ReplSettings>(&json) {
            Ok(s) => s,
            Err(_) => ReplSettings::default(),
        },
        Err(_) => ReplSettings::default(),
    }
}

/// Save settings to disk.
fn save_settings(settings: &ReplSettings) -> Result<(), String> {
    let path = settings_path();
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// CLI handler for `kask settings {show,set,reset}`.
pub fn run(action: SettingsAction) {
    match action {
        SettingsAction::Show { name } => {
            let settings = load_settings();
            match name {
                None => show_all(&settings),
                Some(key) => show_one(&settings, &key),
            }
        }
        SettingsAction::Set { name, value } => {
            let mut settings = load_settings();
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
        "auto_compact:     {}",
        if s.auto_compact { "on" } else { "off" }
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
        "auto_compact" => println!("{}", if s.auto_compact { "on" } else { "off" }),
        "context_length" => match s.model_meta {
            Some(ref m) => println!("{}", m.context_length),
            None => println!("(not set)"),
        },
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
        "auto_compact" => match value {
            "on" | "true" => settings.auto_compact = true,
            "off" | "false" => settings.auto_compact = false,
            _ => {
                eprintln!("Error: expected 'on' or 'off'");
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
