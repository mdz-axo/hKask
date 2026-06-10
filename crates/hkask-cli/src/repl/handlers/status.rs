//! REPL /status handler — system status display (CNS, agent, gas, loops)

pub(crate) fn handle_status(
    state: &mut super::super::ReplState,
    template_id: Option<&str>,
    rt: &tokio::runtime::Handle,
) {
    let agent_display = state.current_agent.clone();
    let tpl = template_id.unwrap_or("auto-select");
    let gas_remaining = state.inference_loop.gas_remaining();
    let gas_cap = state.inference_loop.gas_cap();
    let gas_pct = if gas_cap > 0 {
        (gas_remaining as f64 / gas_cap as f64) * 100.0
    } else {
        100.0
    };
    let gas_bar = if gas_pct > 60.0 {
        "\x1b[32m■\x1b[0m" // green
    } else if gas_pct > 20.0 {
        "\x1b[33m■\x1b[0m" // yellow
    } else {
        "\x1b[31m■\x1b[0m" // red
    };
    println!("  Agent:      \x1b[1m{}\x1b[0m", agent_display);
    println!("  Model:      \x1b[1m{}\x1b[0m", state.current_model);
    println!("  Template:   {}", tpl);
    println!(
        "  Gas:        {} {}/{} ({:.0}%)",
        gas_bar, gas_remaining, gas_cap, gas_pct
    );
    // Check CNS health
    let (cns_runtime, _, _, _) = state.service_context.cns();
    let cns_health = rt.block_on(cns_runtime.read());
    let cns_status = match rt.block_on(async { cns_health.health().await }) {
        health if health.critical_count > 0 => {
            format!(
                "\x1b[31m\u{26a0} CRITICAL\x1b[0m ({} critical, {} warnings)",
                health.critical_count, health.warning_count
            )
        }
        health if health.warning_count > 0 => {
            format!(
                "\x1b[33m\u{26a0} WARNING\x1b[0m ({} warnings)",
                health.warning_count
            )
        }
        _ => "\x1b[32mHEALTHY\x1b[0m (no alerts)".to_string(),
    };
    println!("  CNS:        {}", cns_status);
    // Show LoopSystem registered loops
    let (_, _, loops, _) = state.service_context.cns();
    let loop_count = rt.block_on(loops.registered_count());
    let loop_ids = rt.block_on(loops.registered_loop_ids());
    let ids_str = loop_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    println!("  Loops:      {} registered ({})", loop_count, ids_str);
    println!("  Turns:      {}", state.session_history.turns.len());
    match &state.active_session {
        Some(session) => {
            let config = rt.block_on(async {
                crate::commands::ensemble_improv_config(&state.service_context, session).await
            });
            match config {
                Ok(cfg) => {
                    println!(
                        "  Ensemble:   \x1b[33m{}\x1b[0m (mode: {}, threshold: {:.2})",
                        session,
                        cfg.mode.as_str(),
                        cfg.participation_threshold
                    );
                }
                Err(e) => {
                    println!(
                        "  Ensemble:   \x1b[33m{}\x1b[0m (config error: {})",
                        session, e
                    );
                }
            }
        }
        None => {
            println!("  Ensemble:   single-agent");
        }
    }
    println!();
}
