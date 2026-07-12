//! Loops command handlers for `kask loops`
//!
//! Implements the CLI display logic for starting the cybernetic loop system.
//! Routes through `helpers::build_agent_service_from_secrets(None)` —
//! the canonical Result-returning entry point for one-shot CLI.

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; service config must be resolvable
/// post: starts the cybernetic loop system; prints registered loops; runs until Ctrl+C
pub fn run(rt: &tokio::runtime::Runtime) {
    // Build AgentService through the shared canonical helper
    let ctx = super::helpers::or_exit(
        super::helpers::build_agent_service_from_secrets(None),
        "Failed to build service context for loop system",
    );

    // Start the loop system
    println!("Starting Loop System (per-loop default tick intervals)");
    let loops = ctx.cns().loops.clone();
    println!("Registered loops:");
    let ids = rt.block_on(loops.registered_loop_ids());
    for id in &ids {
        println!("  • {:?}", id);
    }
    println!();
    let inference = ctx.infra().inference.clone();
    if inference.is_none() {
        println!("Note: Inference Loop not registered (requires inference connection)");
    }
    println!();

    rt.block_on(loops.start());

    // Run until Ctrl+C
    println!("Loop system running. Press Ctrl+C to shutdown.");
    rt.block_on(async {
        tokio::signal::ctrl_c().await.ok();
    });

    loops.shutdown();
    println!("Loop system shut down.");
}
