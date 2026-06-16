//! Scheduler — agent-native task scheduling.
//! Each agent owns its own scheduled tasks, stored in the registry DB.
//! The curation loop checks for due tasks each cycle.

use hkask_storage::AgentRegistryStore;
use hkask_types::ScheduledTask;

use crate::error::ServiceError;

pub struct SchedulerService;

impl SchedulerService {
    /// Schedule a recurring task for an agent.
    ///
    /// REQ: P7-svc-scheduler-207
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be initialized; agent_name, trigger, action, next_run must be non-empty
    /// post: task is persisted to the registry store; Err(AgentRegistryStore) on store failure
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
            .map_err(ServiceError::AgentRegistryStore)
    }

    /// List all scheduled tasks for an agent.
    ///
    /// REQ: P7-svc-scheduler-208
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be initialized; agent_name must be non-empty
    /// post: returns Vec<ScheduledTask> for the agent; empty Vec if none; Err(AgentRegistryStore) on store failure
    pub fn list(
        store: &AgentRegistryStore,
        agent_name: &str,
    ) -> Result<Vec<ScheduledTask>, ServiceError> {
        store
            .list_scheduled_tasks(agent_name)
            .map_err(ServiceError::AgentRegistryStore)
    }

    /// Get all due tasks across all agents (for the curation loop).
    ///
    /// REQ: P7-svc-scheduler-209
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be initialized; now must be a valid timestamp string
    /// post: returns Vec<ScheduledTask> of all due tasks; empty Vec if none; Err(AgentRegistryStore) on store failure
    pub fn due_tasks(
        store: &AgentRegistryStore,
        now: &str,
    ) -> Result<Vec<ScheduledTask>, ServiceError> {
        store
            .list_due_tasks(now)
            .map_err(ServiceError::AgentRegistryStore)
    }

    /// Update a task's next run time after it fires.
    ///
    /// REQ: P7-svc-scheduler-210
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be initialized; agent_name, trigger, new_next_run must be non-empty
    /// post: task's next_run is updated in the store; Err(AgentRegistryStore) on store failure
    pub fn reschedule(
        store: &AgentRegistryStore,
        agent_name: &str,
        trigger: &str,
        new_next_run: &str,
    ) -> Result<(), ServiceError> {
        store
            .update_next_run(agent_name, trigger, new_next_run)
            .map_err(ServiceError::AgentRegistryStore)
    }
}
