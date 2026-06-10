//! REPL /ensemble handler — multi-agent session management

use hkask_services::AgentService;

pub(crate) fn handle_ensemble(
    subcmd: &str,
    rest: &str,
    active_session: &mut Option<String>,
    svc_ctx: &AgentService,
    rt: &tokio::runtime::Handle,
) {
    match subcmd {
        "sessions" | "list" | "" => {
            rt.block_on(async {
                match crate::commands::ensemble_chat_list(svc_ctx).await {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            println!("  No active ensemble sessions.");
                            println!("  Use \x1b[36m/ensemble create <id>\x1b[0m to start one.");
                        } else {
                            println!("  \x1b[1mEnsemble sessions:\x1b[0m");
                            for s in &sessions {
                                let active = match &active_session {
                                    Some(a) if a == s => " \x1b[1;33m← active\x1b[0m",
                                    _ => "",
                                };
                                println!("    \x1b[36m•\x1b[0m {}{}", s, active);
                            }
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            });
        }
        "create" => {
            if rest.is_empty() {
                println!("  Usage: \x1b[36m/ensemble create <session-id>\x1b[0m");
            } else {
                let session = rest.split_whitespace().next().unwrap_or(rest);
                rt.block_on(async {
                    match crate::commands::ensemble_chat_create(svc_ctx, session.to_string()).await
                    {
                        Ok(msg) => println!("  \x1b[32m✓\x1b[0m {}", msg),
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
        }
        "join" | "register" => {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() < 3 {
                println!("  Usage: \x1b[36m/ensemble join <session> <bot> <role>\x1b[0m");
                println!("  Roles: memory_bot, spandrel_bot, okapi_bot, scholar_bot");
            } else {
                rt.block_on(async {
                    match crate::commands::ensemble_chat_register(
                        svc_ctx,
                        parts[0].to_string(),
                        parts[1].to_string(),
                        parts[2].to_string(),
                    )
                    .await
                    {
                        Ok(msg) => println!("  \x1b[32m✓\x1b[0m {}", msg),
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
        }
        "invite" => match &active_session {
            Some(session) => {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.is_empty() {
                    println!("  Usage: \x1b[36m/ensemble invite <bot> [role]\x1b[0m");
                    println!(
                        "  Roles: memory_bot, spandrel_bot, okapi_bot, scholar_bot (default: custom)"
                    );
                } else {
                    let bot = parts[0];
                    let role = parts
                        .get(1)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "custom".to_string());
                    rt.block_on(async {
                        match crate::commands::ensemble_chat_register(
                            svc_ctx,
                            session.clone(),
                            bot.to_string(),
                            role,
                        )
                        .await
                        {
                            Ok(msg) => println!("  \x1b[32m✓\x1b[0m {}", msg),
                            Err(e) => println!("  Error: {}", e),
                        }
                    });
                }
            }
            None => {
                println!(
                    "  \x1b[31mNo active session.\x1b[0m Use \x1b[36m/into <session>\x1b[0m first."
                );
            }
        },
        "participants" | "who" => match &active_session {
            Some(session) => {
                rt.block_on(async {
                    match crate::commands::ensemble_participants(svc_ctx, session).await {
                        Ok(participants) => {
                            if participants.is_empty() {
                                println!("  No participants in session.");
                            } else {
                                println!("  \x1b[1mParticipants ({}):\x1b[0m", participants.len());
                                for (name, role, caps) in &participants {
                                    println!(
                                        "    \x1b[36m{}\x1b[0m ({}) caps: {}",
                                        name, role, caps
                                    );
                                }
                            }
                        }
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
            None => {
                println!(
                    "  \x1b[31mNo active session.\x1b[0m Use \x1b[36m/into <session>\x1b[0m first."
                );
            }
        },
        "send" | "say" => {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() < 2 {
                println!("  Usage: \x1b[36m/ensemble send <session> <message>\x1b[0m");
            } else {
                rt.block_on(async {
                    match crate::commands::ensemble_chat_send(
                        svc_ctx,
                        parts[0].to_string(),
                        parts[1].to_string(),
                    )
                    .await
                    {
                        Ok(_) => println!("  \x1b[32m✓\x1b[0m Message sent to {}", parts[0]),
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
        }
        other => {
            println!("  Unknown ensemble subcommand: \x1b[31m{}\x1b[0m", other);
            println!("  Use: sessions, create, join, invite, participants, send");
            println!("  Type \x1b[36m/help ensemble\x1b[0m for details.");
        }
    }
    println!();
}

/// Handle /filter — ensemble participation threshold
pub(crate) fn handle_filter(
    arg: &str,
    active_session: &Option<String>,
    svc_ctx: &AgentService,
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

/// Handle /mode — ensemble orchestration mode
pub(crate) fn handle_mode(
    arg: &str,
    active_session: &Option<String>,
    svc_ctx: &AgentService,
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
        match hkask_agents::ensemble::ImprovMode::parse_mode(arg.trim()) {
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
                                hkask_agents::ensemble::ImprovMode::Freeform => {
                                    println!("  \x1b[2m(agents self-select by relevance)\x1b[0m");
                                }
                                hkask_agents::ensemble::ImprovMode::CuratorLed => {
                                    println!("  \x1b[2m(Curator picks who speaks)\x1b[0m");
                                }
                                hkask_agents::ensemble::ImprovMode::RoundRobin => {
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

/// Handle /into — enter/leave ensemble sessions
pub(crate) fn handle_into(
    arg: &str,
    active_session: &mut Option<String>,
    svc_ctx: &AgentService,
    rt: &tokio::runtime::Handle,
) {
    if arg.is_empty() {
        match active_session {
            Some(_) => {
                let leaving = active_session.take().expect("active session exists");
                println!(
                    "  Left ensemble session \x1b[33m{}\x1b[0m. Back to single-agent mode.",
                    leaving
                );
            }
            None => {
                println!("  Not in an ensemble session.");
                println!("  Use \x1b[36m/into <session-id>\x1b[0m to enter one.");
                println!("  Use \x1b[36m/ensemble create <id>\x1b[0m to create one first.");
            }
        }
    } else {
        let session = arg.trim().to_string();
        let exists = rt.block_on(async {
            match crate::commands::ensemble_chat_list(svc_ctx).await {
                Ok(sessions) => sessions.contains(&session),
                Err(_) => false,
            }
        });

        if exists {
            *active_session = Some(session.clone());
            let config_result = rt.block_on(async {
                crate::commands::ensemble_improv_config(svc_ctx, &session).await
            });
            match config_result {
                Ok(config) => {
                    println!("  Entered ensemble session \x1b[33m{}\x1b[0m", session);
                    println!(
                        "  Mode: \x1b[1m{}\x1b[0m  Threshold: \x1b[1m{:.2}\x1b[0m",
                        config.mode.as_str(),
                        config.participation_threshold
                    );
                    println!("  Messages now go to the ensemble. \x1b[2m/into\x1b[0m to leave.");
                }
                Err(e) => {
                    println!(
                        "  Entered ensemble session \x1b[33m{}\x1b[0m (config error: {})",
                        session, e
                    );
                }
            }
        } else {
            println!(
                "  Session \x1b[31m{}\x1b[0m not found. Create it first with \x1b[36m/ensemble create {}\x1b[0m",
                session, session
            );
        }
    }
    println!();
}
