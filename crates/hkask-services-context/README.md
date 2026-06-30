# hkask-services-context — AgentService Context

Central `AgentService` struct — the runtime context aggregating all service dependencies, CNS runtime, cybernetic loops, and subsystem handles. Every CLI command, API route, MCP tool, and ACP handler receives this context.

**Version:** v0.31.0 | **Crate:** `hkask-services-context`

## Exports

| Type | Purpose |
|------|---------|
| `AgentService` | Central runtime context (34 pub fields after getter refactoring) |
| `PerAgentMemory` | Episodic + semantic memory handle per agent |

## AgentService Fields (pub — v0.30.0 refactored)

| Field | Type | Purpose |
|-------|------|---------|
| `registry` | `Arc<Mutex<SqliteRegistry>>` | Template/skill registry |
| `mcp_runtime` | `Arc<McpRuntime>` | MCP tool dispatch |
| `mcp_dispatcher` | `Arc<McpDispatcher>` | Multi-server MCP management |
| `cns_runtime` | `Arc<RwLock<CnsRuntime>>` | CNS span emission |
| `cybernetics_loop` | `Arc<RwLock<CyberneticsLoop>>` | Homeostasis regulation |
| `loop_system` | `Arc<LoopSystem>` | Four-loop authority model |
| `backup_loop` | `Arc<BackupLoop>` | Git CAS backup |
| `escalation_queue` | `Arc<EscalationQueue>` | Algedonic escalation |
| `goal_repo` | `Arc<SqliteGoalRepository>` | Goal state persistence |
| `pod_manager` | `Arc<ActivePods>` | Multi-pod orchestration |
| `capability_checker` | `Arc<CapabilityChecker>` | OCAP verification |
| `event_sink` | `Arc<dyn NuEventSink>` | Event emission |
| `energy_estimator` | `Arc<CalibratedEnergyEstimator>` | Gas/energy accounting |
| `seam_watcher` | `Arc<RwLock<Option<SeamWatcher>>>` | Public seam coverage |
| `config` | `ServiceConfig` | System configuration |
| `sovereignty_boundary_store` | `SovereigntyBoundaryStore` | Sovereignty enforcement |
| `spec_store` | `SqliteSpecStore` | Specification persistence |
| `a2a_runtime` | `Arc<A2ARuntime>` | Agent-to-agent communication |
| `agent_registry_store` | `AgentRegistryStore` | Agent registry |
| `user_store` | `Arc<Mutex<UserStore>>` | User management |
| `daemon_handler` | `Arc<ServiceDaemonHandler>` | Daemon lifecycle |

## Dependencies

Directly depends on: `hkask-types`, `hkask-cns`, `hkask-mcp`, `hkask-services-core`, `hkask-services-runtime`, `hkask-services-backup`, `hkask-services-kata-kanban`, `hkask-services-skill`, `hkask-services-wallet`, `hkask-storage`, `hkask-memory`, `hkask-agents`, `hkask-keystore`, `hkask-capability`, `hkask-communication`, `hkask-federation`
