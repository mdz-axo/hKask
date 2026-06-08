//! REPL /hhh handler — HHH alignment mode (Helpful, Harmless, Honest)

use std::sync::Arc;

use hkask_agents::HhhMode;
use hkask_types::ports::InferencePort;

pub(crate) fn handle_hhh(arg: &str, state: &mut super::super::ReplState) {
    match arg.trim() {
        "on" => {
            if state.gate_inference_port.is_none() {
                println!(
                    "  \x1b[31m\u{2717} HHH mode unavailable\x1b[0m — gate model initialization failed."
                );
                println!(
                    "  Run \x1b[36m/hhh model <name>\x1b[0m to configure a different gate model."
                );
            } else {
                state.hhh_mode = HhhMode::Active;
                println!(
                    "  \x1b[32m\u{2713} HHH mode activated\x1b[0m (Helpful, Harmless, Honest)"
                );
                println!(
                    "  Gate model: \x1b[1m{}\x1b[0m, max iterations: {}",
                    state.hhh_config.gate_model, state.hhh_config.max_iterations
                );
            }
        }
        "off" => {
            state.hhh_mode = HhhMode::Inactive;
            println!("  \x1b[33m\u{2717} HHH mode deactivated\x1b[0m");
        }
        "status" | "" => {
            let mode_str = match state.hhh_mode {
                HhhMode::Active => "\x1b[32mACTIVE\x1b[0m",
                HhhMode::Inactive => "\x1b[33mINACTIVE\x1b[0m",
            };
            println!("  HHH Mode:    {}", mode_str);
            println!(
                "  Gate Model:  \x1b[1m{}\x1b[0m",
                state.hhh_config.gate_model
            );
            println!("  Iterations:  {}", state.hhh_config.max_iterations);
            println!("  Threshold:   {}", state.hhh_config.pass_threshold);
            if state.gate_inference_port.is_none() {
                println!(
                    "  \x1b[31m\u{26a0} Gate model unavailable\x1b[0m — use /hhh model <name> to configure"
                );
            }
        }
        arg_str if arg_str.starts_with("model ") => {
            let model_name = arg_str[6..].trim();
            if model_name.is_empty() {
                println!("  Usage: \x1b[36m/hhh model <name>\x1b[0m");
            } else {
                // Recreate the gate inference port with the new model
                match hkask_templates::OkapiInference::new(model_name, state.okapi_config.clone()) {
                    Ok(port) => {
                        state.gate_inference_port = Some(Arc::new(port) as Arc<dyn InferencePort>);
                        state.hhh_config.gate_model = model_name.to_string();
                        println!(
                            "  Gate model set to: \x1b[1m{}\x1b[0m",
                            state.hhh_config.gate_model
                        );
                    }
                    Err(e) => {
                        println!("  \x1b[31mFailed to initialize gate model: {}\x1b[0m", e);
                    }
                }
            }
        }
        _ => {
            println!("  \x1b[1mHHH Alignment Mode\x1b[0m (Helpful, Harmless, Honest)");
            println!();
            println!("  \x1b[36m/hhh on\x1b[0m      Activate HHH mode");
            println!("  \x1b[36m/hhh off\x1b[0m     Deactivate HHH mode");
            println!("  \x1b[36m/hhh status\x1b[0m  Show current HHH settings");
            println!("  \x1b[36m/hhh model\x1b[0m   Change gate model");
        }
    }
    println!();
}
