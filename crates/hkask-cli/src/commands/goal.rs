//! Goal coordination commands — delegates to GoalService.

use hkask_services::{AgentService, CreateGoalRequest, GoalService, ServiceConfig, ServiceError};

use crate::cli::GoalAction;
use crate::errors::RegistryError;

impl From<ServiceError> for RegistryError {
    fn from(e: ServiceError) -> Self {
        RegistryError::InitFailed(e.to_string())
    }
}

fn build_service_context() -> Result<AgentService, RegistryError> {
    let config = ServiceConfig::from_env().map_err(RegistryError::from)?;
    let rt = tokio::runtime::Runtime::new().expect("runtime should start");
    let svc = rt
        .block_on(AgentService::build(config))
        .map_err(RegistryError::from)?;
    Ok(svc)
}

pub fn create(text: &str, visibility: &str) -> Result<(), RegistryError> {
    let ctx = build_service_context()?;
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

pub fn list(state: Option<&str>) -> Result<(), RegistryError> {
    let ctx = build_service_context()?;
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

pub fn set_state(id: &str, state: &str) -> Result<(), RegistryError> {
    let ctx = build_service_context()?;
    let goal = GoalService::set_goal_state(&ctx, id, state)?;
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
