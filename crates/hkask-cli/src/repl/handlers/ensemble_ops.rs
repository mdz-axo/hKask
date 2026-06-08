//! REPL /filter and /mode handlers — ensemble participation threshold and orchestration mode

use hkask_services::ServiceContext;

pub(crate) fn handle_filter(
    arg: &str,
    active_session: &Option<String>,
    svc_ctx: &ServiceContext,
    rt: &tokio::runtime::Handle,
) {
    let session_id = match active_session {
        Some(s) => s.clone(),
        None => {
            println!(
                "  \x1b[31mNo active session.\x1b[0m Use \x1b[36m/into <session>\x1b[0m first."
            );
            println!();
            return;
        }
    };
    if arg.is_empty() {
        let config = rt.block_on(async {
            crate::commands::ensemble_improv_config(svc_ctx, &session_id).await
        });
        match config {
            Ok(cfg) => {
                println!(
                    "  Participation threshold: \x1b[1m{:.2}\x1b[0m",
                    cfg.participation_threshold
                );
                println!("  (0.0 = all speak, 1.0 = nobody speaks, 0.75 = default)");
            }
            Err(e) => println!("  Error: {}", e),
        }
    } else {
        match arg.parse::<f64>() {
            Ok(threshold) => {
                rt.block_on(async {
                    match crate::commands::ensemble_improv_set_threshold(
                        svc_ctx,
                        &session_id,
                        threshold,
                    )
                    .await
                    {
                        Ok(()) => {
                            let clamped = threshold.clamp(0.0, 1.0);
                            println!(
                                "  Participation threshold set to \x1b[1m{:.2}\x1b[0m",
                                clamped
                            );
                            if clamped < 0.5 {
                                println!("  \x1b[2m(low — most agents will speak)\x1b[0m");
                            } else if clamped > 0.9 {
                                println!("  \x1b[2m(high — very selective)\x1b[0m");
                            }
                        }
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
            Err(_) => {
                println!(
                    "  Invalid threshold: \x1b[31m{}\x1b[0m. Must be 0.0-1.0",
                    arg
                );
            }
        }
    }
    println!();
}

pub(crate) fn handle_mode(
    arg: &str,
    active_session: &Option<String>,
    svc_ctx: &ServiceContext,
    rt: &tokio::runtime::Handle,
) {
    let session_id = match active_session {
        Some(s) => s.clone(),
        None => {
            println!(
                "  \x1b[31mNo active session.\x1b[0m Use \x1b[36m/into <session>\x1b[0m first."
            );
            println!();
            return;
        }
    };
    if arg.is_empty() {
        let config = rt.block_on(async {
            crate::commands::ensemble_improv_config(svc_ctx, &session_id).await
        });
        match config {
            Ok(cfg) => {
                println!("  Ensemble mode: \x1b[1m{}\x1b[0m", cfg.mode.as_str());
                println!("  Options: freeform, curator_led, round_robin");
            }
            Err(e) => println!("  Error: {}", e),
        }
    } else {
        match hkask_ensemble::ImprovMode::parse_mode(arg.trim()) {
            Some(mode) => {
                rt.block_on(async {
                    match crate::commands::ensemble_improv_set_mode(
                        svc_ctx,
                        &session_id,
                        mode.clone(),
                    )
                    .await
                    {
                        Ok(()) => {
                            println!("  Ensemble mode set to \x1b[1m{}\x1b[0m", mode.as_str());
                            match mode {
                                hkask_ensemble::ImprovMode::Freeform => {
                                    println!("  \x1b[2m(agents self-select by relevance)\x1b[0m");
                                }
                                hkask_ensemble::ImprovMode::CuratorLed => {
                                    println!("  \x1b[2m(Curator picks who speaks)\x1b[0m");
                                }
                                hkask_ensemble::ImprovMode::RoundRobin => {
                                    println!("  \x1b[2m(all agents speak in turn)\x1b[0m");
                                }
                            }
                        }
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
            None => {
                println!("  Unknown mode: \x1b[31m{}\x1b[0m", arg);
                println!("  Options: freeform, curator_led, round_robin");
            }
        }
    }
    println!();
}
