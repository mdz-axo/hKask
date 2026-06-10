//! Loops command handlers for `kask loops`
//!
//! Implements the CLI display logic for starting the cybernetic loop system.
//! Uses `AgentService::build()` to assemble all shared infrastructure
//! (CNS, loop system, curation, episodic/semantic loops).

pub fn run(rt: &tokio::runtime::Runtime) {
    // Resolve configuration from keystore and environment
    let config = hkask_services::ServiceConfig::from_env().unwrap_or_else(|e| {
        eprintln!("Failed to resolve service config: {}", e);
        eprintln!("Using in-memory config for loop system.");
        hkask_services::ServiceConfig::in_memory()
    });

    // Build AgentService with all shared infrastructure
    let ctx = rt
        .block_on(hkask_services::AgentService::build(config))
        .expect("Failed to build service context for loop system");

    // Start the loop system
    println!("Starting Loop System (per-loop default tick intervals)");
    let (_, _, loops, _) = ctx.cns();
    println!("Registered loops:");
    let ids = rt.block_on(loops.registered_loop_ids());
    for id in &ids {
        println!("  • {:?}", id);
    }
    println!();
    let (inference, _, _, _) = ctx.coordination();
    if inference.is_none() {
        println!("Note: Inference Loop not registered (requires Okapi connection)");
    }
    println!();

    rt.block_on(loops.start())
        .expect("Failed to start loop system: lock poisoned");

    // Run until Ctrl+C
    println!("Loop system running. Press Ctrl+C to shutdown.");
    rt.block_on(async {
        tokio::signal::ctrl_c().await.ok();
    });

    loops.shutdown();
    println!("Loop system shut down.");
}
