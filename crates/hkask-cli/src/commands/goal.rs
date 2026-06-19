//! Goal coordination commands — delegates to GoalService.

use hkask_services::{CreateGoalRequest, GoalService, ServiceError};

use crate::cli::GoalAction;

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  text is non-empty
/// pre:  visibility is a valid visibility string
/// post: returns Ok(()) and prints created goal to stdout
/// post: delegates to GoalService::create_goal
pub fn create(text: &str, visibility: &str) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    let owner = hkask_types::WebID::from_persona(b"cli-user");
    let goal = GoalService::create_goal(
        &ctx,
        CreateGoalRequest {
            text: text.to_string(),
            visibility: visibility.to_string(),
            owner,
        },
    )?;
    println!("Created goal {}", goal.id);
    println!("  text:       {}", goal.text);
    println!("  state:      {}", goal.state);
    println!("  visibility: {}", goal.visibility);
    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  state is an optional state filter string
/// post: returns Ok(()) and prints goals to stdout
/// post: if no goals found → prints "No goals found."
/// post: delegates to GoalService::list_goals
pub fn list(state: Option<&str>) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    let owner = hkask_types::WebID::from_persona(b"cli-user");
    let goals = GoalService::list_goals(&ctx, &owner, state)?;
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

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  id is a valid goal identifier
/// pre:  state is a valid state string
/// post: returns Ok(()) and prints updated goal state to stdout
/// post: delegates to GoalService::set_goal_state
pub fn set_state(id: &str, state: &str) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    let owner = hkask_types::WebID::from_persona(b"cli-user");
    let goal = GoalService::set_goal_state(&ctx, id, state, &owner)?;
    println!("Goal {} -> {}", goal.id, goal.state);
    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  action is a valid GoalAction variant
/// post: dispatches to create/list/set_state based on action variant
/// post: exits with error message on failure (via or_exit)
pub fn run_goal(action: crate::cli::GoalAction) {
    let result = match action {
        GoalAction::Create { text, visibility } => create(&text, &visibility),
        GoalAction::List { state } => list(state.as_deref()),
        GoalAction::SetState { id, state } => set_state(&id, &state),
    };
    super::helpers::or_exit(result, "Goal command failed");
}
