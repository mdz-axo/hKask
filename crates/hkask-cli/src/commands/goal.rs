//! Goal coordination commands.
//!
//! This module wires the goal subsystem into the CLI via `ServiceContext`,
//! which provides the `goal_repo` field. Business logic (parsing, validation)
//! is delegated to `GoalService` in the shared service layer.

use hkask_services::{GoalContext, GoalService};
use hkask_types::id::WebID;

use crate::cli::GoalAction;
use crate::errors::RegistryError;

/// Build a ServiceContext for goal subcommands.
fn build_service_context() -> Result<hkask_services::ServiceContext, RegistryError> {
    let config = hkask_services::ServiceConfig::from_env()
        .map_err(|e| RegistryError::InitFailed(e.to_string()))?;
    let rt =
        tokio::runtime::Runtime::new().map_err(|e| RegistryError::InitFailed(e.to_string()))?;
    rt.block_on(hkask_services::ServiceContext::build(config))
        .map_err(|e| RegistryError::InitFailed(e.to_string()))
}

/// `kask goal create <text> [--visibility ...]`
pub fn create(text: &str, visibility: &str) -> Result<(), RegistryError> {
    let ctx = build_service_context()?;
    let goal_ctx = GoalContext::from(&ctx);
    let webid = WebID::from_persona(b"cli-user");

    let goal = GoalService::create_goal(&goal_ctx, &webid, text, visibility)
        .map_err(|e| RegistryError::InitFailed(format!("Goal creation failed: {e}")))?;

    println!("Created goal {}", goal.id);
    println!("  text:       {}", goal.text);
    println!("  state:      {}", goal.state.as_str());
    println!("  visibility: {}", goal.visibility.as_str());
    Ok(())
}

/// `kask goal list [--state ...]`
pub fn list(state: Option<&str>) -> Result<(), RegistryError> {
    let ctx = build_service_context()?;
    let goal_ctx = GoalContext::from(&ctx);
    let webid = WebID::from_persona(b"cli-user");

    let goals = GoalService::list_goals(&goal_ctx, &webid, state)
        .map_err(|e| RegistryError::InitFailed(format!("Goal list failed: {e}")))?;

    if goals.is_empty() {
        println!("No goals found.");
        return Ok(());
    }
    println!("Goals ({}):", goals.len());
    for g in goals {
        println!("  {} [{}] {}", g.id, g.state.as_str(), g.text);
    }
    Ok(())
}

/// `kask goal set-state <id> <state>`
pub fn set_state(id: &str, state: &str) -> Result<(), RegistryError> {
    let ctx = build_service_context()?;
    let goal_ctx = GoalContext::from(&ctx);

    // Parse goal ID first for the success message
    let goal_id =
        GoalService::parse_goal_id(id).map_err(|e| RegistryError::InitFailed(e.to_string()))?;

    GoalService::set_goal_state(&goal_ctx, id, state)
        .map_err(|e| RegistryError::InitFailed(format!("Goal state change failed: {e}")))?;

    println!("Goal {} -> {}", goal_id, state);
    Ok(())
}

/// CLI handler for `kask goal` subcommand
pub fn run_goal(action: crate::cli::GoalAction) {
    let result = match action {
        GoalAction::Create { text, visibility } => create(&text, &visibility),
        GoalAction::List { state } => list(state.as_deref()),
        GoalAction::SetState { id, state } => set_state(&id, &state),
    };
    super::helpers::or_exit(result, "Goal command failed");
}
