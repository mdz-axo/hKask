//! Scheduler — agent-native task scheduling.
//! Each agent owns its own scheduled tasks, stored in the registry DB.
//! The curation loop checks for due tasks each cycle.

use hkask_rsolidity::contract;

use hkask_storage::AgentRegistryStore;
use hkask_types::ScheduledTask;

use crate::ServiceError;

pub struct SchedulerService;

impl SchedulerService {
    /// Schedule a recurring task for an agent.
    ///
    pub fn schedule(
        store: &AgentRegistryStore,
        agent_name: &str,
        trigger: &str,
        action: &str,
        params: Option<&str>,
        next_run: &str,
    ) -> Result<(), ServiceError> {
        let task = ScheduledTask {
            agent_name: agent_name.to_string(),
            trigger: trigger.to_string(),
            action: action.to_string(),
            params: params.map(|s| s.to_string()),
            next_run: next_run.to_string(),
            enabled: true,
        };
        store
            .add_scheduled_task(&task)
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })
    }

    /// List all scheduled tasks for an agent.
    ///
    pub fn list(
        store: &AgentRegistryStore,
        agent_name: &str,
    ) -> Result<Vec<ScheduledTask>, ServiceError> {
        store
            .list_scheduled_tasks(agent_name)
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })
    }

    /// Get all due tasks across all agents (for the curation loop).
    ///
    pub fn due_tasks(
        store: &AgentRegistryStore,
        now: &str,
    ) -> Result<Vec<ScheduledTask>, ServiceError> {
        store
            .list_due_tasks(now)
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })
    }

    /// Update a task's next run time after it fires.
    ///
    pub fn reschedule(
        store: &AgentRegistryStore,
        agent_name: &str,
        trigger: &str,
        new_next_run: &str,
    ) -> Result<(), ServiceError> {
        store
            .update_next_run(agent_name, trigger, new_next_run)
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })
    }
}
