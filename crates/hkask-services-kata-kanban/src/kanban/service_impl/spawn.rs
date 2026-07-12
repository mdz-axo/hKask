use super::*;

impl KanbanService {
    pub fn spawn_task(
        &self,
        task_id: TaskId,
        spawn_spec: super::SpawnSpec,
        actor: WebID,
    ) -> Result<String, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        Self::require_task_owner(&task, actor)?;

        if let Some(ref pm) = self.pod_manager {
            let pod_name = Self::build_pod_name(&task.title);
            let persona_yaml = Self::build_persona_yaml(&pod_name, &task.title, &spawn_spec);
            match hkask_agents::pod::AgentPersona::from_yaml(&persona_yaml) {
                Ok(persona) => {
                    let pod_note =
                        Self::activate_pod(pm, "kanban-agent", &persona, &pod_name, &spawn_spec)?;
                    let comment = super::Comment::new(task_id, task.owner, pod_note);
                    task.comments.push(comment);
                    task.updated_at = chrono::Utc::now();
                    self.update_task_triple(&task)?;
                    return Ok(format!(
                        "Pod {} activated (webid: {}). Use /kanban note {} to communicate.",
                        pod_name,
                        persona.webid().redacted_display(),
                        task_id
                    ));
                }
                Err(e) => return Ok(format!("Persona parse failed: {}", e)),
            }
        }

        let spawn_note = format!(
            "Spawn configured (no ActivePods): level={}, skills={:?}, memory={}, tools={:?}",
            spawn_spec.delegation_level,
            spawn_spec.delegated_skills,
            spawn_spec.memory_scope,
            spawn_spec.tool_servers,
        );
        let comment = super::Comment::new(task_id, task.owner, spawn_note);
        task.comments.push(comment);
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(format!(
            "Spawn configured for '{}' (no ActivePods — string mode). Skills: {:?}",
            task.title, spawn_spec.delegated_skills
        ))
    }

    fn build_pod_name(title: &str) -> String {
        format!(
            "kanban-{}",
            title.chars().take(20).collect::<String>().replace(' ', "-")
        )
    }

    fn build_persona_yaml(pod_name: &str, title: &str, spec: &super::SpawnSpec) -> String {
        let skills = spec
            .delegated_skills
            .iter()
            .map(|s| format!("  - {}", s))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "agent:\n  name: {name}\n  type: bot\n  version: 0.1.0\n  charter:\n    description: Task: {title}\n    editor: kanban\n  capabilities:\n{skills}\n",
            name = pod_name,
            title = title,
            skills = skills,
        )
    }

    fn activate_pod(
        pm: &hkask_agents::pod::ActivePods,
        agent_type: &str,
        persona: &hkask_agents::pod::AgentPersona,
        pod_name: &str,
        spec: &super::SpawnSpec,
    ) -> Result<String, KanbanError> {
        let rt = tokio::runtime::Handle::current();
        let pod_id = rt
            .block_on(pm.create_pod(
                agent_type,
                persona,
                Some(pod_name.to_string()),
                hkask_agents::pod::PodKind::Team,
            ))
            .map_err(|e| KanbanError::Internal(format!("Pod creation failed: {}", e)))?;
        rt.block_on(pm.activate_pod(&pod_id))
            .map_err(|e| KanbanError::Internal(format!("Pod activation failed: {}", e)))?;
        let webid = persona.webid();
        Ok(format!(
            "Pod activated: id={}, webid={}, skills={:?}, tools={:?}",
            pod_id,
            webid.redacted_display(),
            spec.delegated_skills,
            spec.tool_servers
        ))
    }
}
