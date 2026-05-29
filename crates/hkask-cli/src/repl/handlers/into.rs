//! REPL /into handler — enter/leave ensemble sessions

pub(crate) fn handle_into(
    arg: &str,
    active_session: &mut Option<String>,
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
            match crate::commands::ensemble_chat_list().await {
                Ok(sessions) => sessions.contains(&session),
                Err(_) => false,
            }
        });

        if exists {
            *active_session = Some(session.clone());
            let config_result =
                rt.block_on(async { crate::commands::ensemble_improv_config(&session).await });
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
