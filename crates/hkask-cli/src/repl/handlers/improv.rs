//! REPL /improv handler — set or display the active improv mode.
//!
//! Improv modes (Plussing, Yes And, Yes But, Freestyling, Riffing) set the
//! replicant's interaction posture. The mode is stored as a string on ReplState;
//! full `ImprovMode` type integration occurs when hkask-improv is wired into
//! the inference pipeline.

use crate::repl::ReplState;

/// Valid improv mode labels.
const VALID_MODES: &[&str] = &["plussing", "yes-and", "yes-but", "freestyling", "riffing"];

/// Mode descriptions for display.
fn mode_description(mode: &str) -> &'static str {
    match mode {
        "plussing" => "Extract agreeable, silently discard, build constructively (never negate)",
        "yes-and" => "Accept whole contribution, extend with novel additive layer",
        "yes-but" => "Accept whole, append constraint that narrows without contradicting",
        "freestyling" => "Rapid collaborative short-response cycling (time-bounded, round-robin)",
        "riffing" => "Solo divergent exploration from a seed (return to group or spawn thread)",
        _ => "Unknown mode",
    }
}

pub(crate) fn handle_improv(arg1: &str, arg2: &str, state: &mut ReplState) {
    let mode_arg = if arg2.is_empty() {
        arg1.to_lowercase()
    } else {
        // Two-part argument (e.g., "freestyle 300" or "riff spawn")
        format!("{} {}", arg1.to_lowercase(), arg2.to_lowercase())
    };

    if mode_arg.is_empty() {
        // Display current mode.
        match &state.improv_mode {
            Some(mode) => {
                println!("  Active improv mode: \x1b[1m{}\x1b[0m", mode);
                println!("  {}", mode_description(mode));
            }
            None => {
                println!("  No improv mode active.");
                println!("  Available modes:");
                for m in VALID_MODES {
                    println!("    \x1b[36m/improv {}\x1b[0m — {}", m, mode_description(m));
                }
                println!("  Freestyling accepts duration: \x1b[36m/improv freestyle 300\x1b[0m");
                println!(
                    "  Riffing accepts return policy: \x1b[36m/improv riff group|spawn|steps:N\x1b[0m"
                );
            }
        }
        println!();
        return;
    }

    // Parse mode — handle freestyling with duration and riffing with policy.
    let (mode_name, _extra) =
        if mode_arg.starts_with("freestyling") || mode_arg.starts_with("freestyle") {
            // /improv freestyle [duration_seconds]
            let parts: Vec<&str> = mode_arg.split_whitespace().collect();
            if parts.len() > 1 {
                ("freestyling", Some(parts[1]))
            } else {
                ("freestyling", None)
            }
        } else if mode_arg.starts_with("riff") {
            // /improv riff [group|spawn|steps:N]
            let parts: Vec<&str> = mode_arg.split_whitespace().collect();
            if parts.len() > 1 {
                ("riffing", Some(parts[1]))
            } else {
                ("riffing", None)
            }
        } else {
            (mode_arg.as_str(), None)
        };

    if !VALID_MODES.contains(&mode_name) {
        println!("  Unknown improv mode: \x1b[31m{}\x1b[0m", mode_name);
        println!("  Valid modes: {}", VALID_MODES.join(", "));
        println!();
        return;
    }

    state.improv_mode = Some(mode_name.to_string());
    println!("  Improv mode set to: \x1b[1m{}\x1b[0m", mode_name);
    println!("  {}", mode_description(mode_name));
    if let Some(extra) = _extra {
        println!("  Parameter: {}", extra);
    }
    println!();
}
