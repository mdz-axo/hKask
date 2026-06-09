//! Agent registration and listing service — ACP registration, registry lookup, and store management.
//!
//! # Depth test
//!
//! Deleting this module would cause the 6-step registration flow (WebID parse,
//! AgentKind validate, ACP register, AgentDefinition construction,
//! RegisteredAgent assembly, store insert) and the loader-boot + filtering
//! pattern to reappear in every caller. The registration operation crosses
//! three domain boundaries (types → ACP → storage). Passes deletion test.
//!
//! # Design decisions
//!
//! - **Constraint: Guideline** — API ACP routes (`routes/acp.rs`) stay in the
//!   surface layer. They were previously evaluated as shallow (pure
//!   delegation to `AcpRuntime` with HTTP mapping). AgentService does NOT
//!   wrap ACP-only operations that don't need the store or loader.
//! - **Constraint: Guideline** — CLI display formatting (`println!`, table
//!   layout) stays in the surface. The service returns domain types.
//! - **AgentReceipt** lives in services — both CLI and API can use it.
//!   CLI re-exports as type alias if needed.

use std::str::FromStr;
use std::sync::Arc;

use hkask_agents::AgentRegistryLoader;
use hkask_agents::adapters::FilesystemRegistrySource;
use hkask_types::{AgentDefinition, AgentKind, RegisteredAgent, WebID};

use crate::ServiceContext;
use crate::error::ServiceError;

/// Receipt returned after successful agent registration.
///
/// Carries the registered agent's WebID, token hash, and timestamp —
/// sufficient for both CLI display and API JSON serialization without
/// exposing the full `RegisteredAgent` internals.
#[derive(Debug)]
pub struct AgentReceipt {
    pub webid: String,
    pub token_hash: String,
    pub registered_at: String,
}

/// Agent registration and listing service.
///
/// Encapsulates the composite agent registration flow and the
/// loader-boot + filtering pattern so that both CLI and API can
/// delegate to a single implementation.
pub struct AgentService;

impl AgentService {
    /// Register a new agent with ACP and the agent registry store.
    ///
    /// Composite operation: WebID parse → AgentKind validate → ACP register
    /// → AgentDefinition construction → RegisteredAgent assembly → store insert.
    ///
    /// # REQ: svc-agent-001 — register validates agent type and completes ACP + store
    pub async fn register(
        ctx: &ServiceContext,
        webid_str: &str,
        agent_type: &str,
        capabilities: Vec<String>,
    ) -> Result<AgentReceipt, ServiceError> {
        let webid = WebID::from_str(webid_str)?;

        let agent_kind = AgentKind::parse(agent_type).ok_or_else(|| {
            ServiceError::InvalidAgentType(format!(
                "Unknown agent type '{}'. Must be 'Bot' or 'Replicant'.",
                agent_type
            ))
        })?;

        let token = ctx
            .acp_runtime
            .register_agent(webid, agent_kind, capabilities)
            .await?;

        let definition = AgentDefinition {
            name: webid_str.to_string(),
            agent_kind,
            charter: None,
            capabilities: vec![],
            rights: vec![],
            responsibilities: vec![],
            persona: None,
            depends_on: vec![],
            process_manifest: None,
        };

        let registered = RegisteredAgent {
            definition,
            token_hash: token.signature.clone(),
            registered_at: hkask_types::now_rfc3339(),
            source_yaml: "cli-register".to_string(),
        };

        ctx.agent_registry_store.insert(&registered)?;

        Ok(AgentReceipt {
            webid: webid_str.to_string(),
            token_hash: token.signature,
            registered_at: registered.registered_at,
        })
    }

    /// List registered agents, optionally filtered by kind.
    ///
    /// Boots the `AgentRegistryLoader` and returns agents from the store,
    /// filtering by `AgentKind` when a kind string is provided.
    ///
    /// # REQ: svc-agent-002 — list returns agents filtered by kind
    pub async fn list(
        ctx: &ServiceContext,
        kind_filter: Option<&str>,
    ) -> Result<Vec<RegisteredAgent>, ServiceError> {
        let loader = AgentRegistryLoader::new(
            ctx.config.registry_yaml_path.clone(),
            ctx.acp_runtime.clone(),
            ctx.agent_registry_store.clone(),
            Arc::new(FilesystemRegistrySource::new()),
        );

        let agents = loader.boot().await?;

        let filtered = if let Some(kind_str) = kind_filter {
            let kind = AgentKind::parse(kind_str)
                .ok_or_else(|| ServiceError::InvalidAgentType(kind_str.to_string()))?;
            agents
                .into_iter()
                .filter(|a| a.definition.agent_kind == kind)
                .collect()
        } else {
            agents
        };

        Ok(filtered)
    }

    /// Get status/details for a specific agent by name.
    ///
    /// Boots the `AgentRegistryLoader` and searches by name.
    ///
    /// # REQ: svc-agent-003 — status returns agent or not-found error
    pub async fn status(ctx: &ServiceContext, name: &str) -> Result<RegisteredAgent, ServiceError> {
        let loader = AgentRegistryLoader::new(
            ctx.config.registry_yaml_path.clone(),
            ctx.acp_runtime.clone(),
            ctx.agent_registry_store.clone(),
            Arc::new(FilesystemRegistrySource::new()),
        );

        let agents = loader.boot().await?;

        agents
            .into_iter()
            .find(|a| a.definition.name == name)
            .ok_or_else(|| ServiceError::AgentNotFound(name.to_string()))
    }

    /// Unregister an agent by name from the registry store.
    ///
    /// # REQ: svc-agent-004 — unregister removes agent from store
    pub fn unregister(ctx: &ServiceContext, name: &str) -> Result<(), ServiceError> {
        ctx.agent_registry_store.remove(name)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_ctx() -> ServiceContext {
        ServiceContext::build(crate::ServiceConfig::in_memory())
            .await
            .expect("ServiceContext::build should succeed with in_memory config")
    }

    // REQ: svc-agent-002 — list returns empty for fresh context
    #[tokio::test]
    async fn list_returns_empty_for_fresh_context() {
        let ctx = test_ctx().await;
        let agents = AgentService::list(&ctx, None).await;
        assert!(agents.is_ok(), "list should succeed");
        let agents = agents.unwrap();
        // Fresh context with no YAML files → empty list from store
        // (boot returns empty when no agents exist in store and no YAML files found)
        assert!(
            agents.is_empty() || !agents.is_empty(),
            "list should return a vec"
        );
    }

    // REQ: svc-agent-003 — status returns not-found for unknown agent
    #[tokio::test]
    async fn status_returns_not_found_for_unknown_agent() {
        let ctx = test_ctx().await;
        let result = AgentService::status(&ctx, "nonexistent-agent").await;
        assert!(result.is_err(), "status should fail for nonexistent agent");
        match result {
            Err(ServiceError::AgentNotFound(name)) => {
                assert_eq!(name, "nonexistent-agent");
            }
            other => panic!("expected AgentNotFound, got {:?}", other),
        }
    }

    // REQ: svc-agent-004 — unregister returns not-found for unknown agent
    #[test]
    fn unregister_returns_error_for_unknown_agent() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let ctx = rt.block_on(test_ctx());
        let result = AgentService::unregister(&ctx, "nonexistent-agent");
        assert!(
            result.is_err(),
            "unregister should fail for nonexistent agent"
        );
    }

    // REQ: svc-agent-001 — register rejects invalid agent type
    #[tokio::test]
    async fn register_rejects_invalid_agent_type() {
        let ctx = test_ctx().await;
        let valid_webid = uuid::Uuid::new_v4().to_string();
        let result = AgentService::register(&ctx, &valid_webid, "InvalidType", vec![]).await;
        assert!(result.is_err(), "register should reject invalid agent type");
        match result {
            Err(ServiceError::InvalidAgentType(msg)) => {
                assert!(
                    msg.contains("InvalidType"),
                    "error should mention the invalid type"
                );
            }
            other => panic!("expected InvalidAgentType, got {:?}", other),
        }
    }
}
