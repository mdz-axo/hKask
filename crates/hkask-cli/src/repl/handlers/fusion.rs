//! Fusion mode handler for the REPL.
//!
//! Controls multi-model deliberation from within a session.
//! Supports OpenRouter (client-side panel+judge) and KiloCode (server-side auto-routing).

pub(crate) fn handle_fusion(arg1: &str, _state: &mut super::super::ReplState) {
    match arg1 {
        "" | "status" => {
            let config = hkask_services::InferenceConfig::from_env();
            match &config.fusion {
                Some(f) => {
                    println!();
                    println!("  \x1b[1;33m⚡ Fusion mode active\x1b[0m");
                    match f.provider {
                        hkask_services::ProviderId::KiloCode => {
                            let tier = f.kilo_tier.as_deref().unwrap_or("balanced");
                            let mode = f.kilo_mode.as_deref().unwrap_or("auto");
                            println!("  Model:   \x1b[36mKC/kilo-auto/{}\x1b[0m", tier);
                            println!("  Tier:    \x1b[36m{}\x1b[0m", tier);
                            println!("  Mode:    \x1b[36m{}\x1b[0m", mode);
                            println!();
                            println!(
                                "  \x1b[2mConfigure:  HKASK_FUSION_PROVIDER=KC + HKASK_FUSION_KILO_TIER\x1b[0m"
                            );
                            println!(
                                "  \x1b[2m             HKASK_FUSION_KILO_MODE=plan (optional)\x1b[0m"
                            );
                            println!("  \x1b[2mDisable:    /fusion off\x1b[0m");
                        }
                        _ => {
                            println!("  Model:   \x1b[36mopenrouter/fusion\x1b[0m");
                            println!("  Judge:   \x1b[36m{}\x1b[0m", f.judge);
                            println!("  Panel:   \x1b[36m{}\x1b[0m", f.panel.join(", "));
                            println!();
                            println!(
                                "  \x1b[2mConfigure:  HKASK_FUSION_JUDGE + HKASK_FUSION_PANEL\x1b[0m"
                            );
                            println!("  \x1b[2mDisable:    /fusion off\x1b[0m");
                        }
                    }
                }
                None => {
                    println!();
                    println!("  Fusion mode is \x1b[1;31mOFF\x1b[0m.");
                    println!("  Enable OpenRouter:  \x1b[36m/fusion on\x1b[0m");
                    println!(
                        "  Enable KiloCode:    set HKASK_FUSION_PROVIDER=KC + HKASK_FUSION_KILO_TIER"
                    );
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
            let has_openrouter = !config.openrouter_api_key.is_empty();
            let has_kilocode = !config.kilocode_api_key.is_empty();
            if !has_openrouter && !has_kilocode {
                println!();
                println!(
                    "  \x1b[31mCannot enable fusion:\x1b[0m no fusion-capable provider configured."
                );
                println!("  Set \x1b[36mOPENROUTER_API_KEY\x1b[0m for OpenRouter fusion.");
                println!(
                    "  Set \x1b[36mKILOCODE_API_KEY\x1b[0m + \x1b[36mHKASK_FUSION_KILO_TIER\x1b[0m for KiloCode auto-routing."
                );
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
