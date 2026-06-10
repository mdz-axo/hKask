//! Goal coordination commands — delegates to GoalService.

use hkask_services::{CreateGoalRequest, GoalService};

use crate::cli::GoalAction;
use crate::errors::RegistryError;

pub fn create(text: &str, visibility: &str) -> Result<(), RegistryError> {
    let ctx = super::helpers::build_service_context();
    let owner = hkask_types::WebID::from_persona(b"cli-user");
    let goal = GoalService::create_goal(
        &ctx,
        CreateGoalRequest {
            text: text.to_string(),
            visibility: visibility.to_string(),
            owner,
        },
    )
    .map_err(|e| RegistryError::InitFailed(e.to_string()))?;
    println!("Created goal {}", goal.id);
    println!("  text:       {}", goal.text);
    println!("  state:      {}", goal.state);
    println!("  visibility: {}", goal.visibility);
    Ok(())
}

pub fn list(state: Option<&str>) -> Result<(), RegistryError> {
    let ctx = super::helpers::build_service_context();
    let owner = hkask_types::WebID::from_persona(b"cli-user");
    let goals = GoalService::list_goals(&ctx, &owner, state)
        .map_err(|e| RegistryError::InitFailed(e.to_string()))?;
    if goals.is_empty() {
        println!("No goals found.");
        return Ok(());
    }
    println!("Goals ({}):", goals.len());
    for g in goals {
        println!("  {} [{}] {}", g.id, g.state, g.text);
    }
    Ok(())
}

pub fn set_state(id: &str, state: &str) -> Result<(), RegistryError> {
    let ctx = super::helpers::build_service_context();
    let goal = GoalService::set_goal_state(&ctx, id, state)
        .map_err(|e| RegistryError::InitFailed(e.to_string()))?;
    println!("Goal {} -> {}", goal.id, goal.state);
    Ok(())
}

pub fn run_goal(action: crate::cli::GoalAction) {
    let result = match action {
        GoalAction::Create { text, visibility } => create(&text, &visibility),
        GoalAction::List { state } => list(state.as_deref()),
        GoalAction::SetState { id, state } => set_state(&id, &state),
    };
    super::helpers::or_exit(result, "Goal command failed");
}
