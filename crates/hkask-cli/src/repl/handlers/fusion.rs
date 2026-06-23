//! Fusion mode handler for the REPL.
//!
//! Controls OpenRouter Fusion multi-model deliberation from within a session.

pub(crate) fn handle_fusion(arg1: &str, _state: &mut super::super::ReplState) {
    match arg1 {
        "" | "status" => {
            let config = hkask_services::InferenceConfig::from_env();
            match &config.fusion {
                Some(f) => {
                    println!();
                    println!("  \x1b[1;33m⚡ Fusion mode active\x1b[0m");
                    println!("  Model:   \x1b[36mopenrouter/fusion\x1b[0m");
                    println!("  Judge:   \x1b[36m{}\x1b[0m", f.judge);
                    println!("  Panel:   \x1b[36m{}\x1b[0m", f.panel.join(", "));
                    println!();
                    println!("  \x1b[2mConfigure:  HKASK_FUSION_JUDGE + HKASK_FUSION_PANEL\x1b[0m");
                    println!("  \x1b[2mDisable:    /fusion off\x1b[0m");
                }
                None => {
                    println!();
                    println!("  Fusion mode is \x1b[1;31mOFF\x1b[0m.");
                    println!("  Enable:  \x1b[36m/fusion on\x1b[0m  (requires OpenRouter API key)");
                }
            }
            println!();
        }
        "off" => {
            // SAFETY: called in the REPL event loop, single-threaded.
            unsafe {
                std::env::set_var("HKASK_FUSION_OFF", "1");
            }
            println!();
            println!("  Fusion mode \x1b[1;31mdisabled\x1b[0m for this session.");
            println!("  Restart hKask or use \x1b[36m/fusion on\x1b[0m to re-enable.");
            println!();
        }
        "on" => {
            let config = hkask_services::InferenceConfig::from_env();
            if config.openrouter_api_key.is_empty() {
                println!();
                println!(
                    "  \x1b[31mCannot enable fusion:\x1b[0m OpenRouter API key not configured."
                );
                println!("  Set \x1b[36mOPENROUTER_API_KEY\x1b[0m or \x1b[36mOR_API_KEY\x1b[0m.");
                println!();
                return;
            }
            // SAFETY: called in the REPL event loop, single-threaded.
            unsafe {
                std::env::remove_var("HKASK_FUSION_OFF");
            }
            println!();
            println!("  Fusion mode \x1b[1;32menabled\x1b[0m.");
            println!("  New inference requests will use multi-model deliberation.");
            println!("  \x1b[2mUsing kask defaults (deepseek-v4-pro judge, 4-model panel)\x1b[0m");
            println!();
        }
        _ => {
            println!();
            println!("  \x1b[1m/fusion\x1b[0m — Manage multi-model deliberation");
            println!();
            println!("  \x1b[36m/fusion\x1b[0m          Show current status");
            println!("  \x1b[36m/fusion on\x1b[0m       Enable fusion (uses kask defaults)");
            println!("  \x1b[36m/fusion off\x1b[0m      Disable fusion");
            println!();
            println!("  \x1b[2mConfigure panel:  HKASK_FUSION_JUDGE=deepseek-v4-pro\x1b[0m");
            println!(
                "  \x1b[2m                   HKASK_FUSION_PANEL=Kimi2.7,Qwen3.7 Max,...\x1b[0m"
            );
            println!();
        }
    }
}
