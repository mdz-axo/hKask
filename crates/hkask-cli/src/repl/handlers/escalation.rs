//! REPL escalation handlers — /escalations, /resolve, /dismiss

pub(crate) fn handle_escalations(rt: &tokio::runtime::Handle) {
    rt.block_on(async {
        match crate::commands::curator_escalations().await {
            Ok(escalations) => {
                if escalations.is_empty() {
                    println!("  No pending escalations.");
                } else {
                    println!("  {:<20} {:<15} {:<10} CONTEXT", "ID", "BOT", "CONFIDENCE");
                    println!("  {}", "-".repeat(70));
                    for esc in &escalations {
                        println!(
                            "  {:<20} {:<15} {:<10.2} {}",
                            &esc.id[..std::cmp::min(20, esc.id.len())],
                            esc.bot_id
                                .as_uuid()
                                .to_string()
                                .split('-')
                                .next()
                                .unwrap_or("?"),
                            esc.confidence,
                            &esc.error_context[..std::cmp::min(40, esc.error_context.len())],
                        );
                    }
                    println!("\n  Total: {} pending", escalations.len());
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
    });
    println!();
}

pub(crate) fn handle_resolve(arg1: &str, rt: &tokio::runtime::Handle) {
    if arg1.is_empty() {
        println!("  Usage: /resolve <ID>");
    } else {
        rt.block_on(async {
            match crate::commands::curator_resolve(arg1).await {
                Ok(()) => println!("  Escalation \x1b[32m{}\x1b[0m resolved.", arg1),
                Err(e) => println!("  Error: {}", e),
            }
        });
    }
    println!();
}

pub(crate) fn handle_dismiss(arg1: &str, rt: &tokio::runtime::Handle) {
    if arg1.is_empty() {
        println!("  Usage: /dismiss <ID>");
    } else {
        rt.block_on(async {
            match crate::commands::curator_dismiss(arg1).await {
                Ok(()) => println!("  Escalation \x1b[33m{}\x1b[0m dismissed.", arg1),
                Err(e) => println!("  Error: {}", e),
            }
        });
    }
    println!();
}
