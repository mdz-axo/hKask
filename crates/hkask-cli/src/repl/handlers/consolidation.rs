//! REPL /consolidate handler — user-triggered episodic→semantic consolidation

pub(crate) fn handle_consolidate(
    arg: &str,
    state: &mut super::super::ReplState,
    _rt: &tokio::runtime::Handle,
) {
    // Parse optional sub-arguments: "status" for pre-consolidation view, otherwise run consolidation
    let trimmed = arg.trim();

    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("status") {
        // Show pre-consolidation state using REPL's existing memory infrastructure
        println!("  \x1b[1mConsolidation Status\x1b[0m");
        println!("  Agent: \x1b[36m{}\x1b[0m", state.current_agent);
        println!("  Agent WebID: {}", state.agent_webid);
        println!();
        println!("  Use \x1b[36m/consolidate run\x1b[0m to trigger consolidation");
        println!(
            "  Use \x1b[36mkask consolidate\x1b[0m for full options (limit, confidence floor, etc.)"
        );
        println!();
        return;
    }

    // "run" or other — execute consolidation with defaults
    // Open the database via the standard config path
    use crate::commands::config::{registry_db_path, resolve_db_passphrase};
    use crate::commands::helpers::or_exit;

    let db_path = registry_db_path();
    let db_passphrase = or_exit(resolve_db_passphrase(), "Failed to resolve DB passphrase");
    let db = or_exit(
        if db_path == ":memory:" {
            hkask_storage::Database::in_memory()
        } else {
            hkask_storage::Database::open(&db_path, &db_passphrase)
        },
        "Failed to open database",
    );

    crate::commands::consolidation::run(&db, Some(&state.current_agent), 100, None, None, None);
}
