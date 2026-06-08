//! REPL /ensemble handler — multi-agent session management

use hkask_services::ServiceContext;

pub(crate) fn handle_ensemble(
    subcmd: &str,
    rest: &str,
    active_session: &mut Option<String>,
    svc_ctx: &ServiceContext,
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
