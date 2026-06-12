//! REPL /ask handler — force a specific agent to respond

pub(crate) fn handle_ask(
    arg1: &str,
    arg2: &str,
    rt: &tokio::runtime::Handle,
    state: &mut super::super::ReplState,
) {
    if arg1.is_empty() || arg2.is_empty() {
        println!("  Usage: \x1b[36m/ask <agent> <message>\x1b[0m");
        return;
    }

    match &state.active_session {
        Some(session) => {
            let chat_response = rt.block_on(crate::commands::chat_with_agent(
                arg2,
                Some(arg1),
                None,
                Some(state.inference_port.clone()),
                state.resolved_secrets.as_ref(),
                Some(state.episodic_storage.clone()),
                Some(state.semantic_storage.clone()),
                Some(state.agent_webid),
                Some(state.tool_prompt_section.as_str()),
            ));
            println!("\x1b[1m{}\x1b[0m: {}\n", arg1, chat_response.text);
            if let Some(ref usage) = chat_response.usage {
                println!(
                    "  \x1b[2m{} tokens ({} prompt + {} completion)\x1b[0m",
                    usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
                );
            }

            let manager_session = session.clone();
            rt.block_on(async {
                let _ = crate::commands::ensemble_chat_send(
                    &state.service_context,
                    manager_session,
                    format!("[direct to {}] {}", arg1, arg2),
                )
                .await;
            });
        }
        None => {
            let chat_response = rt.block_on(crate::commands::chat_with_agent(
                arg2,
                Some(arg1),
                None,
                Some(state.inference_port.clone()),
                state.resolved_secrets.as_ref(),
                Some(state.episodic_storage.clone()),
                Some(state.semantic_storage.clone()),
                Some(state.agent_webid),
                Some(state.tool_prompt_section.as_str()),
            ));
            println!("\x1b[1m{}\x1b[0m: {}\n", arg1, chat_response.text);
            if let Some(ref usage) = chat_response.usage {
                println!(
                    "  \x1b[2m{} tokens ({} prompt + {} completion)\x1b[0m",
                    usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
                );
            }
        }
    }
}
