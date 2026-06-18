//! `/invoke <tool> [args]` — direct tool invocation from the REPL.
//!
//! Mints a capability token from the session's A2A secret, then routes
//! the invocation through GovernedTool (OCAP + energy budget + CNS observability).

use hkask_types::capability::derive_signing_key;
use hkask_types::ports::ToolPort;
use hkask_types::{DelegationAction, DelegationResource, DelegationToken, WebID};

/// Handle `/invoke <tool> [args]` or `/invoke <server>/<tool> [args]`.
///
/// Format:
///   /invoke tool_name                    — invoke with no arguments
///   /invoke tool_name '{"key":"val"}'    — invoke with JSON arguments
///   /invoke server/tool_name             — specify server explicitly
///   /invoke server/tool_name '{"k":"v"}' — both
pub(crate) fn handle_invoke(
    arg1: &str,
    arg2: &str,
    state: &mut crate::repl::ReplState,
    rt: &tokio::runtime::Handle,
) {
    if arg1.is_empty() {
        println!("  Usage: \x1b[36m/invoke <tool> [args]\x1b[0m");
        println!("         \x1b[36m/invoke <server>/<tool> [args]\x1b[0m");
        println!();
        println!("  \x1b[2mInvoke an MCP tool through the GovernedTool membrane.\x1b[0m");
        println!("  \x1b[2mArguments should be valid JSON.\x1b[0m");
        println!();
        return;
    }

    // Parse server/tool — if no '/', assume the tool is the first arg and
    // server is determined by the tool registry (use "" to let discover_tools
    // find it).
    let (server, tool_name) = if let Some(pos) = arg1.find('/') {
        (&arg1[..pos], &arg1[pos + 1..])
    } else {
        ("", arg1)
    };

    if tool_name.is_empty() {
        println!("  \x1b[31mError:\x1b[0m Tool name cannot be empty");
        println!("  Usage: \x1b[36m/invoke <tool> [args]\x1b[0m");
        println!();
        return;
    }

    // Parse arguments — if arg2 is provided, parse as JSON; otherwise empty object
    let args: serde_json::Value = if arg2.is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str(arg2) {
            Ok(v) => v,
            Err(e) => {
                println!("  \x1b[31mInvalid JSON arguments:\x1b[0m {}", e);
                println!("  \x1b[2mExpected valid JSON, e.g.: {{\"key\": \"value\"}}\x1b[0m");
                println!();
                return;
            }
        }
    };

    // Mint a capability token for this tool invocation using the A2A secret
    // from onboarding. Capability tokens are signed with the same secret that
    // governs OCAP authority throughout the system.
    let a2a_secret = match state.resolved_secrets {
        Some(ref secrets) => secrets.a2a_secret.as_bytes(),
        None => {
            eprintln!(
                "Error: No A2A secret resolved. Run `kask chat` to complete onboarding or set HKASK_MASTER_KEY."
            );
            return;
        }
    };

    let from_webid = crate::commands::helpers::resolve_user_webid(); // system
    let to_webid = state.agent_webid;
    let token = DelegationToken::new(
        DelegationResource::Tool,
        tool_name.to_string(),
        DelegationAction::Execute,
        from_webid,
        to_webid,
        &derive_signing_key(a2a_secret),
    );

    // Invoke through the GovernedTool membrane — all governance
    // (OCAP verification, energy budget, CNS observability) applies.
    print!("  \x1b[2mInvoking \x1b[36m{}\x1b[0m", tool_name);
    if !server.is_empty() {
        print!(" on \x1b[36m{}\x1b[0m", server);
    }
    println!("...");

    let result = rt.block_on(async {
        state
            .governed_tool
            .invoke(server, tool_name, args, &token)
            .await
    });

    match result {
        Ok(value) => {
            println!("  \x1b[32m✓\x1b[0m {}", tool_name);
            // Pretty-print the JSON result
            match serde_json::to_string_pretty(&value) {
                Ok(formatted) => {
                    for line in formatted.lines() {
                        println!("    {}", line);
                    }
                }
                Err(_) => println!("    {}", value),
            }
        }
        Err(e) => {
            println!("  \x1b[31m✗\x1b[0m {} — {}", tool_name, e);
        }
    }
    println!();
}
