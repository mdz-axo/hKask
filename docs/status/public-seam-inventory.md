# Public Seam Inventory

**Generated:** 2026-06-16T09:37:46Z
**Source:** `scripts/audit/public-seam-inventory.sh`
**Purpose:** P8 traceability — maps public API items to REQ-tagged test coverage.

Each public item is classified:
- 🟢 **Covered** — at least one `// REQ:` test in the same file or module
- 🔴 **Uncovered** — no REQ-tagged test found in the same file

---

## Summary

| Crate | Public Items | Covered | Uncovered | Coverage % | REQ Tests |
|-------|-------------|---------|-----------|------------|-----------|
| hkask-agents | 185 | 156 | 29 | 84% | 168 |

### hkask-agents

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `new` | hkask-agents::acp::audit | crates/hkask-agents/src/acp/audit.rs:23 | 🟢 Accessor/Constructor | 🟢 |
| enum | `A2AMessage` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:104 | 🟡 Type Declaration | 🟢 |
| enum | `AcpError` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:63 | 🟡 Type Declaration | 🟢 |
| fn | `correlation_id` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:280 | 🔴 Core Logic | 🟢 |
| fn | `from_webid` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:262 | 🟢 Accessor/Constructor | 🟢 |
| fn | `message_type` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:294 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:337 | 🟢 Accessor/Constructor | 🟢 |
| fn | `visit` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:215 | 🔴 Core Logic | 🟢 |
| struct | `AcpAgent` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:93 | 🟡 Type Declaration | 🟢 |
| struct | `AcpRuntime` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:322 | 🟡 Type Declaration | 🟢 |
| struct | `MemoryArtifact` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:148 | 🟡 Type Declaration | 🟢 |
| struct | `TemplateDispatch` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:134 | 🟡 Type Declaration | 🟢 |
| struct | `TemplateResponse` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:142 | 🟡 Type Declaration | 🟢 |
| trait | `A2AMessageVisitor` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:163 | 🟡 Type Declaration | 🟢 |
| type | `AgentSecret` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:48 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-agents::acp::root_authority | crates/hkask-agents/src/acp/root_authority.rs:46 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-agents::adapters::mcp_runtime | crates/hkask-agents/src/adapters/mcp_runtime.rs:142 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-agents::adapters::mcp_runtime | crates/hkask-agents/src/adapters/mcp_runtime.rs:85 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CapabilityOnlyAdapter` | hkask-agents::adapters::mcp_runtime | crates/hkask-agents/src/adapters/mcp_runtime.rs:74 | 🟡 Type Declaration | 🟢 |
| struct | `FullMcpAdapter` | hkask-agents::adapters::mcp_runtime | crates/hkask-agents/src/adapters/mcp_runtime.rs:124 | 🟡 Type Declaration | 🟢 |
| fn | `from_path` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:188 | 🟢 Accessor/Constructor | 🟢 |
| fn | `in_memory_unchecked` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:177 | 🔴 Core Logic | 🟢 |
| fn | `in_memory` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:162 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:152 | 🟢 Accessor/Constructor | 🟢 |
| struct | `MemoryLoopForwarder` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:136 | 🟡 Type Declaration | 🟢 |
| type | `MemoryLoopAdapter` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:143 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-agents::adapters::registry_source | crates/hkask-agents/src/adapters/registry_source.rs:24 | 🟢 Accessor/Constructor | 🟢 |
| struct | `FilesystemRegistrySource` | hkask-agents::adapters::registry_source | crates/hkask-agents/src/adapters/registry_source.rs:12 | 🟡 Type Declaration | 🟢 |
| enum | `ConsentError` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:25 | 🟡 Type Declaration | 🟢 |
| fn | `get_granted_categories` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:340 | 🟢 Accessor/Constructor | 🟢 |
| fn | `grant_consent` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:234 | 🔴 Core Logic | 🟢 |
| fn | `grant` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:66 | 🔴 Core Logic | 🟢 |
| fn | `has_category` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:92 | 🟢 Accessor/Constructor | 🟢 |
| fn | `has_consent` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:289 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_active` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:84 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:148 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:52 | 🟢 Accessor/Constructor | 🟢 |
| fn | `revoke_consent` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:265 | 🔴 Core Logic | 🟢 |
| fn | `revoke` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:76 | 🔴 Core Logic | 🟢 |
| fn | `with_event_sink` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:171 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ConsentManager` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:130 | 🟡 Type Declaration | 🟢 |
| enum | `EscalationSeverity` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:87 | 🟡 Type Declaration | 🟢 |
| enum | `EscalationTrigger` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:76 | 🟡 Type Declaration | 🟢 |
| enum | `MetacognitionError` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:45 | 🟡 Type Declaration | 🟢 |
| fn | `check_conditions` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:121 | 🔴 Core Logic | 🟢 |
| fn | `generate_summary` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:277 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:238 | 🟢 Accessor/Constructor | 🟢 |
| struct | `EscalationAlert` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:94 | 🟡 Type Declaration | 🟢 |
| struct | `EscalationPolicy` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:103 | 🟡 Type Declaration | 🟢 |
| struct | `HealthSnapshot` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:178 | 🟡 Type Declaration | 🟢 |
| struct | `MetacognitionConfig` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:198 | 🟡 Type Declaration | 🟢 |
| struct | `MetacognitionLoop` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:221 | 🟡 Type Declaration | 🟢 |
| fn | `context` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:176 | 🔴 Core Logic | 🟢 |
| fn | `curation_loop` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:158 | 🔴 Core Logic | 🟢 |
| fn | `metacognition` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:167 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:64 | 🟢 Accessor/Constructor | 🟢 |
| fn | `spec_curator` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:188 | 🔴 Core Logic | 🟢 |
| fn | `with_config` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:87 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_consolidation` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:122 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CuratorAgent` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:44 | 🟡 Type Declaration | 🟢 |
| fn | `calibrate_from_history` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:72 | 🔴 Core Logic | 🟢 |
| fn | `check_sovereignty` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:185 | 🔴 Core Logic | 🟢 |
| fn | `from_config` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:119 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:45 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_drift_threshold` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:140 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_event_sink` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:150 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_spec_channel` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:161 | 🟢 Accessor/Constructor | 🟢 |
| struct | `DefaultSpecCurator` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:30 | 🟡 Type Declaration | 🟢 |
| fn | `handle` | hkask-agents::curator::context | crates/hkask-agents/src/curator/context.rs:91 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-agents::curator::context | crates/hkask-agents/src/curator/context.rs:36 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_acp` | hkask-agents::curator::context | crates/hkask-agents/src/curator/context.rs:81 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_nu_event_store` | hkask-agents::curator::context | crates/hkask-agents/src/curator/context.rs:59 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CuratorContext` | hkask-agents::curator::context | crates/hkask-agents/src/curator/context.rs:13 | 🟡 Type Declaration | 🟢 |
| fn | `context` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:115 | 🔴 Core Logic | 🟢 |
| fn | `curator_handle` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:127 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:69 | 🟢 Accessor/Constructor | 🟢 |
| fn | `restore_cursor` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:141 | 🔴 Core Logic | 🟢 |
| fn | `with_consolidation` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:85 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_inbox` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:105 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CurationLoop` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:45 | 🟡 Type Declaration | 🟢 |
| fn | `check_persona_constraints` | hkask-agents::curator::persona_filter | crates/hkask-agents/src/curator/persona_filter.rs:32 | 🔴 Core Logic | 🟢 |
| fn | `strip_forbidden_patterns` | hkask-agents::curator::persona_filter | crates/hkask-agents/src/curator/persona_filter.rs:68 | 🔴 Core Logic | 🟢 |
| struct | `PersonaCheckResult` | hkask-agents::curator::persona_filter | crates/hkask-agents/src/curator/persona_filter.rs:12 | 🟡 Type Declaration | 🟢 |
| enum | `CoreError` | hkask-agents::error | crates/hkask-agents/src/error.rs:33 | 🟡 Type Declaration | 🔴 |
| enum | `McpError` | hkask-agents::error | crates/hkask-agents/src/error.rs:10 | 🟡 Type Declaration | 🔴 |
| enum | `MemoryError` | hkask-agents::error | crates/hkask-agents/src/error.rs:61 | 🟡 Type Declaration | 🔴 |
| enum | `RegistryError` | hkask-agents::error | crates/hkask-agents/src/error.rs:159 | 🟡 Type Declaration | 🔴 |
| fn | `gas_cap` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:105 | 🔴 Core Logic | 🔴 |
| fn | `gas_remaining` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:78 | 🔴 Core Logic | 🔴 |
| fn | `new` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:44 | 🟢 Accessor/Constructor | 🟢 |
| fn | `sync_gas_state` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:97 | 🔴 Core Logic | 🔴 |
| fn | `token_usage` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:87 | 🔴 Core Logic | 🔴 |
| fn | `with_energy_budget` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:65 | 🟢 Accessor/Constructor | 🔴 |
| fn | `with_model` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:72 | 🟢 Accessor/Constructor | 🔴 |
| struct | `InferenceLoop` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:31 | 🟡 Type Declaration | 🔴 |
| fn | `cancel_token` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:160 | 🔴 Core Logic | 🟢 |
| fn | `default_tick_interval` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:63 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:106 | 🟢 Accessor/Constructor | 🟢 |
| fn | `shutdown` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:265 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_tick_interval` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:129 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CyberneticsLoopHandle` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:19 | 🟡 Type Declaration | 🟢 |
| struct | `LoopSystem` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:89 | 🟡 Type Declaration | 🟢 |
| fn | `episodic_storage_budget` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:222 | 🔴 Core Logic | 🔴 |
| fn | `episodic_storage_usage` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:210 | 🔴 Core Logic | 🔴 |
| fn | `inference_port` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:152 | 🔴 Core Logic | 🟢 |
| fn | `invoke_tool` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:334 | 🔴 Core Logic | 🔴 |
| fn | `recall_episodic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:193 | 🔴 Core Logic | 🔴 |
| fn | `recall_semantic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:298 | 🔴 Core Logic | 🔴 |
| fn | `require_sovereignty` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:124 | 🔴 Core Logic | 🔴 |
| fn | `semantic_storage_usage` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:315 | 🔴 Core Logic | 🔴 |
| fn | `store_episodic_experience` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:235 | 🔴 Core Logic | 🔴 |
| fn | `store_episodic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:169 | 🔴 Core Logic | 🔴 |
| fn | `store_semantic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:275 | 🔴 Core Logic | 🔴 |
| struct | `PodContext` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:37 | 🟡 Type Declaration | 🔴 |
| fn | `acp_runtime` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:421 | 🔴 Core Logic | 🟢 |
| fn | `inference_port` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:191 | 🔴 Core Logic | 🟢 |
| fn | `new_mock` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:219 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:73 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_capability_checker` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:144 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_consent_port` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:123 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_governed_tool` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:158 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_inference` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:167 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_nu_event_sink` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:151 | 🟢 Accessor/Constructor | 🟢 |
| struct | `PodManager` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:23 | 🟡 Type Declaration | 🟢 |
| struct | `PodStatus` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:41 | 🟡 Type Declaration | 🟢 |
| type | `ActivationHook` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:21 | 🟡 Type Declaration | 🟢 |
| enum | `AgentPodError` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:122 | 🟡 Type Declaration | 🟢 |
| fn | `activate` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:322 | 🔴 Core Logic | 🟢 |
| fn | `check_sovereignty` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:610 | 🔴 Core Logic | 🟢 |
| fn | `deactivate` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:360 | 🔴 Core Logic | 🟢 |
| fn | `delegate` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:401 | 🔴 Core Logic | 🟢 |
| fn | `enter_chat_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:491 | 🔴 Core Logic | 🟢 |
| fn | `enter_server_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:458 | 🔴 Core Logic | 🟢 |
| fn | `exit_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:515 | 🔴 Core Logic | 🟢 |
| fn | `get_voice` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:561 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_active` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:428 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_in_chat_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:585 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_in_server_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:535 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:205 | 🟢 Accessor/Constructor | 🟢 |
| fn | `set_voice` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:546 | 🟢 Accessor/Constructor | 🟢 |
| fn | `state` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:437 | 🔴 Core Logic | 🟢 |
| fn | `voice_description` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:573 | 🔴 Core Logic | 🟢 |
| struct | `AgentPod` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:91 | 🟡 Type Declaration | 🟢 |
| type | `AgentPodResult` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:193 | 🟡 Type Declaration | 🟢 |
| fn | `emit_pod_activated` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:53 | 🔴 Core Logic | 🔴 |
| fn | `emit_pod_deactivated` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:65 | 🔴 Core Logic | 🔴 |
| fn | `emit_pod_event` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:20 | 🔴 Core Logic | 🔴 |
| fn | `emit_pod_registered` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:40 | 🔴 Core Logic | 🔴 |
| enum | `AgentMode` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:16 | 🟡 Type Declaration | 🟢 |
| enum | `PodLifecycleState` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:34 | 🟡 Type Declaration | 🟢 |
| fn | `can_transition_to` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:59 | 🔴 Core Logic | 🟢 |
| fn | `capability_resources` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:168 | 🔴 Core Logic | 🟢 |
| fn | `from_yaml` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:147 | 🟢 Accessor/Constructor | 🟢 |
| fn | `validate_fields` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:176 | 🔴 Core Logic | 🟢 |
| fn | `webid` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:159 | 🔴 Core Logic | 🟢 |
| struct | `AgentPersona` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:86 | 🟡 Type Declaration | 🟢 |
| trait | `AcpPort` | hkask-agents::ports::acp | crates/hkask-agents/src/ports/acp.rs:18 | 🟡 Type Declaration | 🔴 |
| trait | `MCPRuntimePort` | hkask-agents::ports::mcp_runtime | crates/hkask-agents/src/ports/mcp_runtime.rs:11 | 🟡 Type Declaration | 🔴 |
| fn | `classified_episodic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:130 | 🔴 Core Logic | 🟢 |
| fn | `episodic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:166 | 🔴 Core Logic | 🟢 |
| fn | `episodic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:77 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:52 | 🟢 Accessor/Constructor | 🟢 |
| fn | `semantic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:102 | 🔴 Core Logic | 🟢 |
| fn | `semantic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:180 | 🔴 Core Logic | 🟢 |
| struct | `RecallRequest` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:149 | 🟡 Type Declaration | 🟢 |
| struct | `RecalledEpisode` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:197 | 🟡 Type Declaration | 🟢 |
| struct | `RecalledSemantic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:228 | 🟡 Type Declaration | 🟢 |
| struct | `StorageRequest` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:29 | 🟡 Type Declaration | 🟢 |
| trait | `EpisodicStoragePort` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:253 | 🟡 Type Declaration | 🟢 |
| trait | `SemanticStoragePort` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:321 | 🟡 Type Declaration | 🟢 |
| trait | `RegistrySourcePort` | hkask-agents::ports::registry_source | crates/hkask-agents/src/ports/registry_source.rs:9 | 🟡 Type Declaration | 🔴 |
| fn | `decompose_prompt` | hkask-agents::prompt_analysis | crates/hkask-agents/src/prompt_analysis.rs:584 | 🔴 Core Logic | 🟢 |
| struct | `PromptAnalysis` | hkask-agents::prompt_analysis | crates/hkask-agents/src/prompt_analysis.rs:36 | 🟡 Type Declaration | 🟢 |
| struct | `SentenceDecomposition` | hkask-agents::prompt_analysis | crates/hkask-agents/src/prompt_analysis.rs:19 | 🟡 Type Declaration | 🟢 |
| enum | `RegistryLoaderError` | hkask-agents::registry_loader | crates/hkask-agents/src/registry_loader.rs:14 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-agents::registry_loader | crates/hkask-agents/src/registry_loader.rs:237 | 🟢 Accessor/Constructor | 🟢 |
| fn | `store` | hkask-agents::registry_loader | crates/hkask-agents/src/registry_loader.rs:385 | 🔴 Core Logic | 🟢 |
| struct | `AgentRegistryLoader` | hkask-agents::registry_loader | crates/hkask-agents/src/registry_loader.rs:223 | 🟡 Type Declaration | 🟢 |
| fn | `can_access` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:105 | 🔴 Core Logic | 🟢 |
| fn | `check_operation` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:123 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:86 | 🟢 Accessor/Constructor | 🟢 |
| struct | `AllowAllConsent` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:48 | 🟡 Type Declaration | 🟢 |
| struct | `DenyAllConsent` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:35 | 🟡 Type Declaration | 🟢 |
| struct | `SovereigntyChecker` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:61 | 🟡 Type Declaration | 🟢 |
| trait | `SovereigntyConsent` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:23 | 🟡 Type Declaration | 🟢 |

| hkask-api | 138 | 137 | 1 | 99% | 89 |

### hkask-api

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `create_openapi` | hkask-api | crates/hkask-api/src/lib.rs:262 | 🔴 API Route Handler | 🟢 |
| fn | `create_router` | hkask-api | crates/hkask-api/src/lib.rs:212 | 🔴 API Route Handler | 🟢 |
| fn | `shutdown_loops` | hkask-api | crates/hkask-api/src/lib.rs:198 | 🔴 API Route Handler | 🟢 |
| fn | `with_spec_store` | hkask-api | crates/hkask-api/src/lib.rs:159 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_wallet_service` | hkask-api | crates/hkask-api/src/lib.rs:169 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ApiState` | hkask-api | crates/hkask-api/src/lib.rs:67 | 🟡 Type Declaration | 🟢 |
| enum | `ApiError` | hkask-api::error | crates/hkask-api/src/error.rs:14 | 🟡 Type Declaration | 🟢 |
| struct | `ServiceErrorResponse` | hkask-api::error | crates/hkask-api/src/error.rs:94 | 🟡 Type Declaration | 🟢 |
| enum | `ApiKeyAuthError` | hkask-api::middleware::api_key_auth | crates/hkask-api/src/middleware/api_key_auth.rs:200 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-api::middleware::api_key_auth | crates/hkask-api/src/middleware/api_key_auth.rs:60 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ApiKeyAuthService` | hkask-api::middleware::api_key_auth | crates/hkask-api/src/middleware/api_key_auth.rs:49 | 🟡 Type Declaration | 🟢 |
| struct | `WalletContext` | hkask-api::middleware::api_key_auth | crates/hkask-api/src/middleware/api_key_auth.rs:36 | 🟡 Type Declaration | 🟢 |
| enum | `TokenVerification` | hkask-api::middleware::auth | crates/hkask-api/src/middleware/auth.rs:108 | 🟡 Type Declaration | 🟢 |
| fn | `from_config` | hkask-api::middleware::auth | crates/hkask-api/src/middleware/auth.rs:38 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_token_revoked` | hkask-api::middleware::auth | crates/hkask-api/src/middleware/auth.rs:61 | 🟢 Accessor/Constructor | 🟢 |
| fn | `revoke_token` | hkask-api::middleware::auth | crates/hkask-api/src/middleware/auth.rs:49 | 🔴 API Route Handler | 🟢 |
| fn | `verify_token` | hkask-api::middleware::auth | crates/hkask-api/src/middleware/auth.rs:76 | 🔴 API Route Handler | 🟢 |
| struct | `AuthService` | hkask-api::middleware::auth | crates/hkask-api/src/middleware/auth.rs:27 | 🟡 Type Declaration | 🟢 |
| type | `AuthContext` | hkask-api::middleware::auth | crates/hkask-api/src/middleware/auth.rs:124 | 🟡 Type Declaration | 🟢 |
| struct | `ApiDoc` | hkask-api::openapi | crates/hkask-api/src/openapi.rs:86 | 🟡 Type Declaration | 🔴 |
| fn | `acp_router` | hkask-api::routes::acp | crates/hkask-api/src/routes/acp.rs:82 | 🔴 API Route Handler | 🟢 |
| struct | `AcpAgentResponse` | hkask-api::routes::acp | crates/hkask-api/src/routes/acp.rs:63 | 🟡 Type Declaration | 🟢 |
| struct | `AcpRegisterRequest` | hkask-api::routes::acp | crates/hkask-api/src/routes/acp.rs:41 | 🟡 Type Declaration | 🟢 |
| struct | `AcpRegisterResponse` | hkask-api::routes::acp | crates/hkask-api/src/routes/acp.rs:52 | 🟡 Type Declaration | 🟢 |
| struct | `AgentListResponse` | hkask-api::routes::acp | crates/hkask-api/src/routes/acp.rs:73 | 🟡 Type Declaration | 🟢 |
| enum | `ApiBackupScope` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:31 | 🟡 Type Declaration | 🟢 |
| enum | `ApiRestoreScope` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:67 | 🟡 Type Declaration | 🟢 |
| fn | `backup_router` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:180 | 🔴 API Route Handler | 🟢 |
| struct | `BackupConfigResponse` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:153 | 🟡 Type Declaration | 🟢 |
| struct | `CommitInfo` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:59 | 🟡 Type Declaration | 🟢 |
| struct | `ListQuery` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:97 | 🟡 Type Declaration | 🟢 |
| struct | `ListResponse` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:112 | 🟡 Type Declaration | 🟢 |
| struct | `PruneRequest` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:118 | 🟡 Type Declaration | 🟢 |
| struct | `PruneResponse` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:129 | 🟡 Type Declaration | 🟢 |
| struct | `RepoVerifyReport` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:143 | 🟡 Type Declaration | 🟢 |
| struct | `RestoreRequest` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:78 | 🟡 Type Declaration | 🟢 |
| struct | `RestoreResponse` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:85 | 🟡 Type Declaration | 🟢 |
| struct | `RestoredArtifact` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:90 | 🟡 Type Declaration | 🟢 |
| struct | `RetentionConfigResponse` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:161 | 🟡 Type Declaration | 🟢 |
| struct | `SnapshotRequest` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:45 | 🟡 Type Declaration | 🟢 |
| struct | `SnapshotResponse` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:51 | 🟡 Type Declaration | 🟢 |
| struct | `UpdateConfigRequest` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:168 | 🟡 Type Declaration | 🟢 |
| struct | `VerifyResponse` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:138 | 🟡 Type Declaration | 🟢 |
| fn | `bots_router` | hkask-api::routes::bots | crates/hkask-api/src/routes/bots.rs:13 | 🔴 API Route Handler | 🟢 |
| fn | `bundles_router` | hkask-api::routes::bundles | crates/hkask-api/src/routes/bundles.rs:91 | 🔴 API Route Handler | 🟢 |
| struct | `ApplyBundleResponse` | hkask-api::routes::bundles | crates/hkask-api/src/routes/bundles.rs:59 | 🟡 Type Declaration | 🟢 |
| struct | `BundleListResponse` | hkask-api::routes::bundles | crates/hkask-api/src/routes/bundles.rs:76 | 🟡 Type Declaration | 🟢 |
| struct | `BundleSummary` | hkask-api::routes::bundles | crates/hkask-api/src/routes/bundles.rs:21 | 🟡 Type Declaration | 🟢 |
| struct | `ComposeBundleRequest` | hkask-api::routes::bundles | crates/hkask-api/src/routes/bundles.rs:32 | 🟡 Type Declaration | 🟢 |
| struct | `ComposeBundleResponse` | hkask-api::routes::bundles | crates/hkask-api/src/routes/bundles.rs:48 | 🟡 Type Declaration | 🟢 |
| struct | `DeactivateBundleResponse` | hkask-api::routes::bundles | crates/hkask-api/src/routes/bundles.rs:82 | 🟡 Type Declaration | 🟢 |
| struct | `EvolveBundleResponse` | hkask-api::routes::bundles | crates/hkask-api/src/routes/bundles.rs:68 | 🟡 Type Declaration | 🟢 |
| fn | `chat_router` | hkask-api::routes::chat | crates/hkask-api/src/routes/chat.rs:60 | 🔴 API Route Handler | 🟢 |
| struct | `ChatRequest` | hkask-api::routes::chat | crates/hkask-api/src/routes/chat.rs:32 | 🟡 Type Declaration | 🟢 |
| struct | `ChatResponse` | hkask-api::routes::chat | crates/hkask-api/src/routes/chat.rs:46 | 🟡 Type Declaration | 🟢 |
| fn | `cns_router` | hkask-api::routes::cns | crates/hkask-api/src/routes/cns.rs:24 | 🔴 API Route Handler | 🟢 |
| struct | `CnsHealthResponse` | hkask-api::routes::cns | crates/hkask-api/src/routes/cns.rs:242 | 🟡 Type Declaration | 🟢 |
| struct | `CnsVarietyResponse` | hkask-api::routes::cns | crates/hkask-api/src/routes/cns.rs:259 | 🟡 Type Declaration | 🟢 |
| struct | `VarietyCounterResponse` | hkask-api::routes::cns | crates/hkask-api/src/routes/cns.rs:251 | 🟡 Type Declaration | 🟢 |
| fn | `consolidation_router` | hkask-api::routes::consolidation | crates/hkask-api/src/routes/consolidation.rs:47 | 🔴 API Route Handler | 🟢 |
| struct | `ConsolidateRequest` | hkask-api::routes::consolidation | crates/hkask-api/src/routes/consolidation.rs:18 | 🟡 Type Declaration | 🟢 |
| struct | `ConsolidateResponse` | hkask-api::routes::consolidation | crates/hkask-api/src/routes/consolidation.rs:36 | 🟡 Type Declaration | 🟢 |
| fn | `curator_router` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:80 | 🔴 API Route Handler | 🟢 |
| struct | `BotStatusReportResponse` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:64 | 🟡 Type Declaration | 🟢 |
| struct | `DismissEscalationRequest` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:45 | 🟡 Type Declaration | 🟢 |
| struct | `DismissEscalationResponse` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:50 | 🟡 Type Declaration | 🟢 |
| struct | `EscalationEntryResponse` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:14 | 🟡 Type Declaration | 🟢 |
| struct | `EscalationStatsResponse` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:56 | 🟡 Type Declaration | 🟢 |
| struct | `ListEscalationsResponse` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:29 | 🟡 Type Declaration | 🟢 |
| struct | `MetacognitionStatusResponse` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:72 | 🟡 Type Declaration | 🟢 |
| struct | `ResolveEscalationRequest` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:34 | 🟡 Type Declaration | 🟢 |
| struct | `ResolveEscalationResponse` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:39 | 🟡 Type Declaration | 🟢 |
| fn | `episodic_router` | hkask-api::routes::episodic | crates/hkask-api/src/routes/episodic.rs:25 | 🔴 API Route Handler | 🟢 |
| struct | `EpisodeResponse` | hkask-api::routes::episodic | crates/hkask-api/src/routes/episodic.rs:63 | 🟡 Type Declaration | 🟢 |
| struct | `EpisodicUsageResponse` | hkask-api::routes::episodic | crates/hkask-api/src/routes/episodic.rs:82 | 🟡 Type Declaration | 🟢 |
| struct | `QueryEpisodesParams` | hkask-api::routes::episodic | crates/hkask-api/src/routes/episodic.rs:56 | 🟡 Type Declaration | 🟢 |
| struct | `QueryEpisodesResponse` | hkask-api::routes::episodic | crates/hkask-api/src/routes/episodic.rs:76 | 🟡 Type Declaration | 🟢 |
| struct | `StoreEpisodeRequest` | hkask-api::routes::episodic | crates/hkask-api/src/routes/episodic.rs:34 | 🟡 Type Declaration | 🟢 |
| struct | `StoreEpisodeResponse` | hkask-api::routes::episodic | crates/hkask-api/src/routes/episodic.rs:47 | 🟡 Type Declaration | 🟢 |
| fn | `git_router` | hkask-api::routes::git | crates/hkask-api/src/routes/git.rs:57 | 🔴 API Route Handler | 🟢 |
| struct | `ArchiveEntry` | hkask-api::routes::git | crates/hkask-api/src/routes/git.rs:31 | 🟡 Type Declaration | 🟢 |
| struct | `ArchiveRequest` | hkask-api::routes::git | crates/hkask-api/src/routes/git.rs:22 | 🟡 Type Declaration | 🟢 |
| struct | `ArchiveResponse` | hkask-api::routes::git | crates/hkask-api/src/routes/git.rs:40 | 🟡 Type Declaration | 🟢 |
| struct | `ResolveShaResponse` | hkask-api::routes::git | crates/hkask-api/src/routes/git.rs:48 | 🟡 Type Declaration | 🟢 |
| fn | `goal_router` | hkask-api::routes::goal | crates/hkask-api/src/routes/goal.rs:16 | 🔴 API Route Handler | 🟢 |
| struct | `CreateGoalRequest` | hkask-api::routes::goal | crates/hkask-api/src/routes/goal.rs:24 | 🟡 Type Declaration | 🟢 |
| struct | `GoalListResponse` | hkask-api::routes::goal | crates/hkask-api/src/routes/goal.rs:54 | 🟡 Type Declaration | 🟢 |
| struct | `GoalResponse` | hkask-api::routes::goal | crates/hkask-api/src/routes/goal.rs:35 | 🟡 Type Declaration | 🟢 |
| struct | `SetGoalStateRequest` | hkask-api::routes::goal | crates/hkask-api/src/routes/goal.rs:30 | 🟡 Type Declaration | 🟢 |
| fn | `mcp_router` | hkask-api::routes::mcp | crates/hkask-api/src/routes/mcp.rs:38 | 🔴 API Route Handler | 🟢 |
| struct | `McpInvokeRequest` | hkask-api::routes::mcp | crates/hkask-api/src/routes/mcp.rs:80 | 🟡 Type Declaration | 🟢 |
| struct | `McpInvokeResponse` | hkask-api::routes::mcp | crates/hkask-api/src/routes/mcp.rs:90 | 🟡 Type Declaration | 🟢 |
| fn | `models_router` | hkask-api::routes::models | crates/hkask-api/src/routes/models.rs:25 | 🔴 API Route Handler | 🟢 |
| struct | `ModelEntry` | hkask-api::routes::models | crates/hkask-api/src/routes/models.rs:36 | 🟡 Type Declaration | 🟢 |
| struct | `ModelListResponse` | hkask-api::routes::models | crates/hkask-api/src/routes/models.rs:55 | 🟡 Type Declaration | 🟢 |
| struct | `ModelSearchQuery` | hkask-api::routes::models | crates/hkask-api/src/routes/models.rs:68 | 🟡 Type Declaration | 🟢 |
| fn | `pods_router` | hkask-api::routes::pods | crates/hkask-api/src/routes/pods.rs:49 | 🔴 API Route Handler | 🟢 |
| struct | `CreatePodRequest` | hkask-api::routes::pods | crates/hkask-api/src/routes/pods.rs:19 | 🟡 Type Declaration | 🟢 |
| struct | `CreatePodResponse` | hkask-api::routes::pods | crates/hkask-api/src/routes/pods.rs:26 | 🟡 Type Declaration | 🟢 |
| struct | `ListPodsResponse` | hkask-api::routes::pods | crates/hkask-api/src/routes/pods.rs:42 | 🟡 Type Declaration | 🟢 |
| struct | `PodStatusResponse` | hkask-api::routes::pods | crates/hkask-api/src/routes/pods.rs:31 | 🟡 Type Declaration | 🟢 |
| fn | `settings_router` | hkask-api::routes::settings | crates/hkask-api/src/routes/settings.rs:86 | 🔴 API Route Handler | 🟢 |
| struct | `SettingsResponse` | hkask-api::routes::settings | crates/hkask-api/src/routes/settings.rs:15 | 🟡 Type Declaration | 🟢 |
| struct | `UpdateSettingsRequest` | hkask-api::routes::settings | crates/hkask-api/src/routes/settings.rs:67 | 🟡 Type Declaration | 🟢 |
| fn | `sovereignty_router` | hkask-api::routes::sovereignty | crates/hkask-api/src/routes/sovereignty.rs:25 | 🔴 API Route Handler | 🟢 |
| struct | `AccessCheckResponse` | hkask-api::routes::sovereignty | crates/hkask-api/src/routes/sovereignty.rs:57 | 🟡 Type Declaration | 🟢 |
| struct | `SovereigntyConsentRequest` | hkask-api::routes::sovereignty | crates/hkask-api/src/routes/sovereignty.rs:44 | 🟡 Type Declaration | 🟢 |
| struct | `SovereigntyConsentResponse` | hkask-api::routes::sovereignty | crates/hkask-api/src/routes/sovereignty.rs:50 | 🟡 Type Declaration | 🟢 |
| struct | `SovereigntyStatusResponse` | hkask-api::routes::sovereignty | crates/hkask-api/src/routes/sovereignty.rs:34 | 🟡 Type Declaration | 🟢 |
| fn | `spec_router` | hkask-api::routes::spec | crates/hkask-api/src/routes/spec.rs:72 | 🔴 API Route Handler | 🟢 |
| struct | `SpecCaptureRequestDto` | hkask-api::routes::spec | crates/hkask-api/src/routes/spec.rs:22 | 🟡 Type Declaration | 🟢 |
| struct | `SpecCoherenceResponse` | hkask-api::routes::spec | crates/hkask-api/src/routes/spec.rs:54 | 🟡 Type Declaration | 🟢 |
| struct | `SpecDetailResponse` | hkask-api::routes::spec | crates/hkask-api/src/routes/spec.rs:38 | 🟡 Type Declaration | 🟢 |
| struct | `SpecListQuery` | hkask-api::routes::spec | crates/hkask-api/src/routes/spec.rs:48 | 🟡 Type Declaration | 🟢 |
| struct | `SpecListResponse` | hkask-api::routes::spec | crates/hkask-api/src/routes/spec.rs:29 | 🟡 Type Declaration | 🟢 |
| struct | `SpecWritingQualityResponse` | hkask-api::routes::spec | crates/hkask-api/src/routes/spec.rs:62 | 🟡 Type Declaration | 🟢 |
| fn | `templates_router` | hkask-api::routes::templates | crates/hkask-api/src/routes/templates.rs:48 | 🔴 API Route Handler | 🟢 |
| struct | `GrantCapabilityRequest` | hkask-api::routes::templates | crates/hkask-api/src/routes/templates.rs:39 | 🟡 Type Declaration | 🟢 |
| struct | `TemplateResponse` | hkask-api::routes::templates | crates/hkask-api/src/routes/templates.rs:28 | 🟡 Type Declaration | 🟢 |
| fn | `wallet_router` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:30 | 🔴 API Route Handler | 🟢 |
| struct | `ApiKeyCreatedResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:111 | 🟡 Type Declaration | 🟢 |
| struct | `ApiKeyEntry` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:121 | 🟡 Type Declaration | 🟢 |
| struct | `ApiKeyListResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:132 | 🟡 Type Declaration | 🟢 |
| struct | `ApiKeyRevokedResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:137 | 🟡 Type Declaration | 🟢 |
| struct | `CreateKeyRequest` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:101 | 🟡 Type Declaration | 🟢 |
| struct | `DepositAddressQuery` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:324 | 🟡 Type Declaration | 🟢 |
| struct | `DepositAddressResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:60 | 🟡 Type Declaration | 🟢 |
| struct | `DepositReferenceRequest` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:67 | 🟡 Type Declaration | 🟢 |
| struct | `DepositReferenceResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:74 | 🟡 Type Declaration | 🟢 |
| struct | `FeeEstimateQuery` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:192 | 🟡 Type Declaration | 🟢 |
| struct | `TransactionListResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:96 | 🟡 Type Declaration | 🟢 |
| struct | `TransactionQuery` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:81 | 🟡 Type Declaration | 🟢 |
| struct | `TransactionResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:89 | 🟡 Type Declaration | 🟢 |
| struct | `WalletBalanceResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:44 | 🟡 Type Declaration | 🟢 |
| struct | `WalletIdQuery` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:278 | 🟡 Type Declaration | 🟢 |
| struct | `WithdrawRequest` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:143 | 🟡 Type Declaration | 🟢 |
| struct | `WithdrawalFeeEstimateResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:52 | 🟡 Type Declaration | 🟢 |
| struct | `WithdrawalResponse` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:153 | 🟡 Type Declaration | 🟢 |

| hkask-cli | 115 | 73 | 42 | 63% | 133 |

### hkask-cli

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `voice_preset_from_design` | hkask-cli | crates/hkask-cli/src/lib.rs:18 | 🔴 Core Logic | 🟢 |
| enum | `BootstrapError` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:47 | 🟡 Type Declaration | 🔴 |
| enum | `BootstrapPhase` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:21 | 🟡 Type Declaration | 🔴 |
| fn | `curator_webid` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:389 | 🔴 Core Logic | 🔴 |
| fn | `new` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:103 | 🟢 Accessor/Constructor | 🔴 |
| fn | `phase` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:66 | 🔴 Core Logic | 🔴 |
| fn | `r7_bot_identities` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:112 | 🔴 Core Logic | 🔴 |
| fn | `state` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:384 | 🔴 Core Logic | 🟢 |
| struct | `BootstrapSequence` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:90 | 🟡 Type Declaration | 🔴 |
| struct | `BootstrapState` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:81 | 🟡 Type Declaration | 🔴 |
| enum | `AgentAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:47 | 🟡 Type Declaration | 🔴 |
| enum | `BackupAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:267 | 🟡 Type Declaration | 🔴 |
| enum | `BotAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:35 | 🟡 Type Declaration | 🔴 |
| enum | `BundleAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:548 | 🟡 Type Declaration | 🔴 |
| enum | `CnsAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:127 | 🟡 Type Declaration | 🔴 |
| enum | `ConfigAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:308 | 🟡 Type Declaration | 🔴 |
| enum | `CuratorAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:327 | 🟡 Type Declaration | 🔴 |
| enum | `DocsAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:175 | 🟡 Type Declaration | 🔴 |
| enum | `GitAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:192 | 🟡 Type Declaration | 🔴 |
| enum | `GoalAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:578 | 🟡 Type Declaration | 🔴 |
| enum | `KataAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:615 | 🟡 Type Declaration | 🔴 |
| enum | `KeyAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:759 | 🟡 Type Declaration | 🔴 |
| enum | `KeystoreAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:385 | 🟡 Type Declaration | 🔴 |
| enum | `MatrixAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:795 | 🟡 Type Declaration | 🔴 |
| enum | `McpAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:109 | 🟡 Type Declaration | 🔴 |
| enum | `PodAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:68 | 🟡 Type Declaration | 🔴 |
| enum | `ReplicantAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:343 | 🟡 Type Declaration | 🔴 |
| enum | `SettingsAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:645 | 🟡 Type Declaration | 🔴 |
| enum | `SkillAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:600 | 🟡 Type Declaration | 🔴 |
| enum | `SovereigntyAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:152 | 🟡 Type Declaration | 🔴 |
| enum | `SpecAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:428 | 🟡 Type Declaration | 🔴 |
| enum | `StyleAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:483 | 🟡 Type Declaration | 🔴 |
| enum | `TemplateAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:7 | 🟡 Type Declaration | 🔴 |
| enum | `WalletAction` | hkask-cli::cli::actions | crates/hkask-cli/src/cli/actions.rs:658 | 🟡 Type Declaration | 🔴 |
| fn | `init_logging` | hkask-cli::cli::helpers | crates/hkask-cli/src/cli/helpers.rs:31 | 🔴 Core Logic | 🟢 |
| fn | `parse_data_category` | hkask-cli::cli::helpers | crates/hkask-cli/src/cli/helpers.rs:10 | 🔴 Core Logic | 🟢 |
| fn | `parse_template_type` | hkask-cli::cli::helpers | crates/hkask-cli/src/cli/helpers.rs:20 | 🔴 Core Logic | 🟢 |
| fn | `generate_cli_markdown` | hkask-cli::cli::markdown | crates/hkask-cli/src/cli/markdown.rs:12 | 🔴 Core Logic | 🟢 |
| enum | `Commands` | hkask-cli::cli::mod | crates/hkask-cli/src/cli/mod.rs:33 | 🟡 Type Declaration | 🔴 |
| struct | `Cli` | hkask-cli::cli::mod | crates/hkask-cli/src/cli/mod.rs:19 | 🟡 Type Declaration | 🟢 |
| fn | `run_agent` | hkask-cli::commands::agent | crates/hkask-cli/src/commands/agent.rs:181 | 🔴 Core Logic | 🟢 |
| fn | `run_bot` | hkask-cli::commands::agent | crates/hkask-cli/src/commands/agent.rs:106 | 🔴 Core Logic | 🟢 |
| struct | `AgentReceipt` | hkask-cli::commands::agent | crates/hkask-cli/src/commands/agent.rs:14 | 🟡 Type Declaration | 🟢 |
| fn | `run` | hkask-cli::commands::backup_cmd | crates/hkask-cli/src/commands/backup_cmd.rs:86 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run_bundle` | hkask-cli::commands::bundle | crates/hkask-cli/src/commands/bundle.rs:43 | 🔴 Core Logic | 🟢 |
| fn | `run_chat` | hkask-cli::commands::chat | crates/hkask-cli/src/commands/chat.rs:441 | 🔴 Core Logic | 🟢 |
| type | `ChatResponse` | hkask-cli::commands::chat | crates/hkask-cli/src/commands/chat.rs:62 | 🟡 Type Declaration | 🟢 |
| type | `TokenUsage` | hkask-cli::commands::chat | crates/hkask-cli/src/commands/chat.rs:67 | 🟡 Type Declaration | 🟢 |
| fn | `run` | hkask-cli::commands::cns | crates/hkask-cli/src/commands/cns.rs:15 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::compose | crates/hkask-cli/src/commands/compose.rs:17 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::consolidation | crates/hkask-cli/src/commands/consolidation.rs:11 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run_curator` | hkask-cli::commands::curator | crates/hkask-cli/src/commands/curator.rs:53 | 🔴 Core Logic | 🟢 |
| fn | `run` | hkask-cli::commands::discover | crates/hkask-cli/src/commands/discover.rs:21 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::docs | crates/hkask-cli/src/commands/docs.rs:10 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::embed_corpus | crates/hkask-cli/src/commands/embed_corpus.rs:44 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::git_cmd | crates/hkask-cli/src/commands/git_cmd.rs:46 | 🟢 Accessor/Constructor | 🟢 |
| fn | `create` | hkask-cli::commands::goal | crates/hkask-cli/src/commands/goal.rs:12 | 🔴 Core Logic | 🟢 |
| fn | `list` | hkask-cli::commands::goal | crates/hkask-cli/src/commands/goal.rs:35 | 🔴 Core Logic | 🟢 |
| fn | `run_goal` | hkask-cli::commands::goal | crates/hkask-cli/src/commands/goal.rs:66 | 🔴 Core Logic | 🟢 |
| fn | `set_state` | hkask-cli::commands::goal | crates/hkask-cli/src/commands/goal.rs:55 | 🟢 Accessor/Constructor | 🟢 |
| fn | `build_service_context` | hkask-cli::commands::helpers | crates/hkask-cli/src/commands/helpers.rs:27 | 🔴 Core Logic | 🟢 |
| fn | `or_exit` | hkask-cli::commands::helpers | crates/hkask-cli/src/commands/helpers.rs:12 | 🔴 Core Logic | 🟢 |
| fn | `write_or_print` | hkask-cli::commands::helpers | crates/hkask-cli/src/commands/helpers.rs:43 | 🔴 Core Logic | 🟢 |
| fn | `run` | hkask-cli::commands::kata | crates/hkask-cli/src/commands/kata.rs:26 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::keystore | crates/hkask-cli/src/commands/keystore.rs:12 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::loops | crates/hkask-cli/src/commands/loops.rs:10 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::magna_carta | crates/hkask-cli/src/commands/magna_carta.rs:11 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::matrix | crates/hkask-cli/src/commands/matrix.rs:12 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::mcp | crates/hkask-cli/src/commands/mcp.rs:42 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::models | crates/hkask-cli/src/commands/models.rs:10 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::onboard | crates/hkask-cli/src/commands/onboard.rs:12 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run_pod` | hkask-cli::commands::pod | crates/hkask-cli/src/commands/pod.rs:94 | 🔴 Core Logic | 🟢 |
| fn | `run_list` | hkask-cli::commands::registry | crates/hkask-cli/src/commands/registry.rs:21 | 🔴 Core Logic | 🟢 |
| fn | `run_rm` | hkask-cli::commands::registry | crates/hkask-cli/src/commands/registry.rs:44 | 🔴 Core Logic | 🟢 |
| fn | `run` | hkask-cli::commands::settings | crates/hkask-cli/src/commands/settings.rs:17 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run_skill` | hkask-cli::commands::skill | crates/hkask-cli/src/commands/skill.rs:23 | 🔴 Core Logic | 🟢 |
| fn | `run` | hkask-cli::commands::sovereignty | crates/hkask-cli/src/commands/sovereignty.rs:11 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::spec | crates/hkask-cli/src/commands/spec.rs:15 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::style | crates/hkask-cli/src/commands/style.rs:9 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_template` | hkask-cli::commands::template | crates/hkask-cli/src/commands/template.rs:84 | 🟢 Accessor/Constructor | 🟢 |
| fn | `list_templates_local` | hkask-cli::commands::template | crates/hkask-cli/src/commands/template.rs:30 | 🔴 Core Logic | 🟢 |
| fn | `list_templates` | hkask-cli::commands::template | crates/hkask-cli/src/commands/template.rs:17 | 🔴 Core Logic | 🟢 |
| fn | `register_template` | hkask-cli::commands::template | crates/hkask-cli/src/commands/template.rs:54 | 🔴 Core Logic | 🟢 |
| fn | `run_template` | hkask-cli::commands::template | crates/hkask-cli/src/commands/template.rs:164 | 🔴 Core Logic | 🟢 |
| fn | `search_templates` | hkask-cli::commands::template | crates/hkask-cli/src/commands/template.rs:95 | 🔴 Core Logic | 🟢 |
| fn | `change_passphrase` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:407 | 🔴 Core Logic | 🟢 |
| fn | `get_replicants` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:160 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_replicant` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:143 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_sessions` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:174 | 🟢 Accessor/Constructor | 🟢 |
| fn | `list_replicants` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:313 | 🔴 Core Logic | 🟢 |
| fn | `list_sessions` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:357 | 🔴 Core Logic | 🟢 |
| fn | `login_replicant` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:265 | 🔴 Core Logic | 🟢 |
| fn | `login_with_passphrase` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:125 | 🔴 Core Logic | 🟢 |
| fn | `logout` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:340 | 🔴 Core Logic | 🟢 |
| fn | `register_replicant_with_passphrase` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:89 | 🔴 Core Logic | 🟢 |
| fn | `register_replicant` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:202 | 🔴 Core Logic | 🟢 |
| fn | `revoke_session` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:185 | 🔴 Core Logic | 🟢 |
| fn | `run_replicant` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:377 | 🔴 Core Logic | 🟢 |
| fn | `show_replicant` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:292 | 🔴 Core Logic | 🟢 |
| fn | `run` | hkask-cli::commands::wallet | crates/hkask-cli/src/commands/wallet.rs:19 | 🟢 Accessor/Constructor | 🟢 |
| fn | `run` | hkask-cli::commands::web_search | crates/hkask-cli/src/commands/web_search.rs:31 | 🟢 Accessor/Constructor | 🟢 |
| enum | `OnboardingError` | hkask-cli::onboarding | crates/hkask-cli/src/onboarding.rs:22 | 🟡 Type Declaration | 🟢 |
| struct | `OnboardingOutcome` | hkask-cli::onboarding | crates/hkask-cli/src/onboarding.rs:32 | 🟡 Type Declaration | 🟢 |
| fn | `print_onboarding_banner` | hkask-cli::repl::display | crates/hkask-cli/src/repl/display.rs:6 | 🔴 Core Logic | 🔴 |
| fn | `settings_path` | hkask-cli::repl::handlers::repl_settings | crates/hkask-cli/src/repl/handlers/repl_settings.rs:256 | 🔴 Core Logic | 🟢 |
| fn | `run` | hkask-cli::repl::mod | crates/hkask-cli/src/repl/mod.rs:111 | 🟢 Accessor/Constructor | 🟢 |
| fn | `format_tool_prompt_section` | hkask-cli::repl::tool_augmented | crates/hkask-cli/src/repl/tool_augmented.rs:43 | 🔴 Core Logic | 🔴 |
| fn | `format_tool_results` | hkask-cli::repl::tool_augmented | crates/hkask-cli/src/repl/tool_augmented.rs:207 | 🔴 Core Logic | 🔴 |
| fn | `parse_tool_calls` | hkask-cli::repl::tool_augmented | crates/hkask-cli/src/repl/tool_augmented.rs:114 | 🔴 Core Logic | 🔴 |
| struct | `ParsedResponse` | hkask-cli::repl::tool_augmented | crates/hkask-cli/src/repl/tool_augmented.rs:100 | 🟡 Type Declaration | 🔴 |
| struct | `ProcessedResponse` | hkask-cli::repl::tool_augmented | crates/hkask-cli/src/repl/tool_augmented.rs:360 | 🟡 Type Declaration | 🔴 |
| struct | `ToolCall` | hkask-cli::repl::tool_augmented | crates/hkask-cli/src/repl/tool_augmented.rs:80 | 🟡 Type Declaration | 🔴 |
| fn | `from_file` | hkask-cli::transcript_viewer | crates/hkask-cli/src/transcript_viewer.rs:48 | 🟢 Accessor/Constructor | 🔴 |
| fn | `run` | hkask-cli::transcript_viewer | crates/hkask-cli/src/transcript_viewer.rs:72 | 🟢 Accessor/Constructor | 🟢 |
| struct | `TranscriptViewer` | hkask-cli::transcript_viewer | crates/hkask-cli/src/transcript_viewer.rs:26 | 🟡 Type Declaration | 🔴 |

| hkask-cns | 107 | 97 | 10 | 90% | 138 |

### hkask-cns

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| enum | `AlertSeverity` | hkask-cns::algedonic | crates/hkask-cns/src/algedonic.rs:32 | 🟡 Type Declaration | 🟢 |
| fn | `is_critical` | hkask-cns::algedonic | crates/hkask-cns/src/algedonic.rs:95 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_warning` | hkask-cns::algedonic | crates/hkask-cns/src/algedonic.rs:103 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-cns::algedonic | crates/hkask-cns/src/algedonic.rs:60 | 🟢 Accessor/Constructor | 🟢 |
| fn | `should_escalate` | hkask-cns::algedonic | crates/hkask-cns/src/algedonic.rs:87 | 🔴 Core Logic | 🟢 |
| struct | `RuntimeAlert` | hkask-cns::algedonic | crates/hkask-cns/src/algedonic.rs:43 | 🟡 Type Declaration | 🟢 |
| enum | `ApiMeteringAlert` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:260 | 🟡 Type Declaration | 🟢 |
| enum | `RateLimitStatus` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:102 | 🟡 Type Declaration | 🟢 |
| fn | `alert_type` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:287 | 🔴 Core Logic | 🟢 |
| fn | `as_str` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:116 | 🟢 Accessor/Constructor | 🟢 |
| fn | `check_and_record` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:163 | 🔴 Core Logic | 🟢 |
| fn | `current_rpm` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:196 | 🔴 Core Logic | 🟢 |
| fn | `endpoint_weight` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:36 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:142 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:237 | 🟢 Accessor/Constructor | 🟢 |
| fn | `severity` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:302 | 🔴 Core Logic | 🟢 |
| struct | `ApiMeter` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:132 | 🟡 Type Declaration | 🟢 |
| struct | `ApiRequestSpan` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:221 | 🟡 Type Declaration | 🟢 |
| struct | `EndpointWeight` | hkask-cns::api_metering | crates/hkask-cns/src/api_metering.rs:21 | 🟡 Type Declaration | 🟢 |
| fn | `allow_request` | hkask-cns::circuit_breaker | crates/hkask-cns/src/circuit_breaker.rs:86 | 🔴 Core Logic | 🟢 |
| fn | `default_for_inference` | hkask-cns::circuit_breaker | crates/hkask-cns/src/circuit_breaker.rs:78 | 🔴 Core Logic | 🟢 |
| fn | `record_failure` | hkask-cns::circuit_breaker | crates/hkask-cns/src/circuit_breaker.rs:148 | 🔴 Core Logic | 🟢 |
| fn | `record_success` | hkask-cns::circuit_breaker | crates/hkask-cns/src/circuit_breaker.rs:123 | 🔴 Core Logic | 🟢 |
| fn | `state` | hkask-cns::circuit_breaker | crates/hkask-cns/src/circuit_breaker.rs:172 | 🔴 Core Logic | 🟢 |
| struct | `CircuitBreaker` | hkask-cns::circuit_breaker | crates/hkask-cns/src/circuit_breaker.rs:43 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-cns::composite_energy_estimator | crates/hkask-cns/src/composite_energy_estimator.rs:26 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CompositeEnergyEstimator` | hkask-cns::composite_energy_estimator | crates/hkask-cns/src/composite_energy_estimator.rs:15 | 🟡 Type Declaration | 🟢 |
| fn | `emit_contract_coverage` | hkask-cns::contract_discipline | crates/hkask-cns/src/contract_discipline.rs:80 | 🔴 Core Logic | 🟢 |
| fn | `emit_contract_violated` | hkask-cns::contract_discipline | crates/hkask-cns/src/contract_discipline.rs:40 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-cns::cybernetics_loop | crates/hkask-cns/src/cybernetics_loop.rs:73 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_alerts_channel` | hkask-cns::cybernetics_loop | crates/hkask-cns/src/cybernetics_loop.rs:105 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_curator_directive_channel` | hkask-cns::cybernetics_loop | crates/hkask-cns/src/cybernetics_loop.rs:122 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_event_sink` | hkask-cns::cybernetics_loop | crates/hkask-cns/src/cybernetics_loop.rs:98 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_set_points` | hkask-cns::cybernetics_loop | crates/hkask-cns/src/cybernetics_loop.rs:77 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_tool_consumption_channel` | hkask-cns::cybernetics_loop | crates/hkask-cns/src/cybernetics_loop.rs:112 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CyberneticsLoop` | hkask-cns::cybernetics_loop | crates/hkask-cns/src/cybernetics_loop.rs:53 | 🟡 Type Declaration | 🟢 |
| enum | `EnergyError` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:486 | 🟡 Type Declaration | 🟢 |
| fn | `as_raw` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:117 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_raw` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:45 | 🟢 Accessor/Constructor | 🟢 |
| fn | `available` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:286 | 🔴 Core Logic | 🟢 |
| fn | `can_proceed` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:276 | 🔴 Core Logic | 🟢 |
| fn | `consume` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:375 | 🔴 Core Logic | 🟢 |
| fn | `from_raw` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:109 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_raw` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:37 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_ascending` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:136 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_descending` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:127 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:218 | 🟢 Accessor/Constructor | 🟢 |
| fn | `replenish_by_weighted` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:435 | 🔴 Core Logic | 🟢 |
| fn | `replenish_by` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:415 | 🔴 Core Logic | 🟢 |
| fn | `replenish` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:398 | 🔴 Core Logic | 🟢 |
| fn | `reserve` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:307 | 🔴 Core Logic | 🟢 |
| fn | `settle` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:340 | 🔴 Core Logic | 🟢 |
| fn | `unlimited` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:238 | 🔴 Core Logic | 🟢 |
| fn | `with_alert_threshold` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:256 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_hard_limit` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:265 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_replenish_rate` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:246 | 🟢 Accessor/Constructor | 🟢 |
| struct | `AgentEnergyStatus` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:453 | 🟡 Type Declaration | 🟢 |
| struct | `EnergyBudget` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:186 | 🟡 Type Declaration | 🟢 |
| struct | `EnergyCost` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:27 | 🟡 Type Declaration | 🟢 |
| struct | `EnergyDelta` | hkask-cns::energy | crates/hkask-cns/src/energy.rs:99 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-cns::energy_budget_management | crates/hkask-cns/src/energy_budget_management.rs:64 | 🟢 Accessor/Constructor | 🟢 |
| struct | `EnergyBudgetManager` | hkask-cns::energy_budget_management | crates/hkask-cns/src/energy_budget_management.rs:47 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-cns::governed_inference | crates/hkask-cns/src/governed_inference.rs:62 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_agent` | hkask-cns::governed_inference | crates/hkask-cns/src/governed_inference.rs:82 | 🟢 Accessor/Constructor | 🟢 |
| struct | `GovernedInference` | hkask-cns::governed_inference | crates/hkask-cns/src/governed_inference.rs:48 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-cns::governed_tool | crates/hkask-cns/src/governed_tool.rs:98 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_agent` | hkask-cns::governed_tool | crates/hkask-cns/src/governed_tool.rs:134 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_tool_consumption_channel` | hkask-cns::governed_tool | crates/hkask-cns/src/governed_tool.rs:121 | 🟢 Accessor/Constructor | 🟢 |
| struct | `GovernedTool` | hkask-cns::governed_tool | crates/hkask-cns/src/governed_tool.rs:81 | 🟡 Type Declaration | 🟢 |
| trait | `EnergyEstimator` | hkask-cns::governed_tool | crates/hkask-cns/src/governed_tool.rs:52 | 🟡 Type Declaration | 🟢 |
| fn | `blocking_variety_for_domain` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:381 | 🔴 Core Logic | 🟢 |
| fn | `calibrate_threshold_blocking` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:559 | 🔴 Core Logic | 🟢 |
| fn | `domains` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:217 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:194 | 🟢 Accessor/Constructor | 🟢 |
| fn | `subscribe` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:582 | 🔴 Core Logic | 🟢 |
| fn | `variety_for_domain` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:209 | 🔴 Core Logic | 🟢 |
| fn | `with_threshold` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:275 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CnsRuntime` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:264 | 🟡 Type Declaration | 🟢 |
| struct | `NoopEventSink` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:683 | 🟡 Type Declaration | 🟢 |
| struct | `VarietyMonitor` | hkask-cns::runtime | crates/hkask-cns/src/runtime.rs:185 | 🟡 Type Declaration | 🟢 |
| fn | `load` | hkask-cns::seam_watcher | crates/hkask-cns/src/seam_watcher.rs:114 | 🔴 Core Logic | 🟢 |
| fn | `refresh` | hkask-cns::seam_watcher | crates/hkask-cns/src/seam_watcher.rs:407 | 🔴 Core Logic | 🟢 |
| fn | `summary` | hkask-cns::seam_watcher | crates/hkask-cns/src/seam_watcher.rs:473 | 🔴 Core Logic | 🟢 |
| struct | `SeamDrift` | hkask-cns::seam_watcher | crates/hkask-cns/src/seam_watcher.rs:48 | 🟡 Type Declaration | 🟢 |
| struct | `SeamSummary` | hkask-cns::seam_watcher | crates/hkask-cns/src/seam_watcher.rs:68 | 🟡 Type Declaration | 🟢 |
| struct | `SeamWatcher` | hkask-cns::seam_watcher | crates/hkask-cns/src/seam_watcher.rs:99 | 🟡 Type Declaration | 🟢 |
| fn | `from_config` | hkask-cns::set_points | crates/hkask-cns/src/set_points.rs:125 | 🟢 Accessor/Constructor | 🔴 |
| fn | `from_yaml` | hkask-cns::set_points | crates/hkask-cns/src/set_points.rs:98 | 🟢 Accessor/Constructor | 🔴 |
| fn | `load_from_file` | hkask-cns::set_points | crates/hkask-cns/src/set_points.rs:103 | 🔴 Core Logic | 🔴 |
| fn | `load_set_points` | hkask-cns::set_points | crates/hkask-cns/src/set_points.rs:152 | 🔴 Core Logic | 🔴 |
| struct | `SetPointsConfig` | hkask-cns::set_points | crates/hkask-cns/src/set_points.rs:87 | 🟡 Type Declaration | 🔴 |
| struct | `SetPoints` | hkask-cns::set_points | crates/hkask-cns/src/set_points.rs:57 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-cns::snapshot_loop | crates/hkask-cns/src/snapshot_loop.rs:76 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_config` | hkask-cns::snapshot_loop | crates/hkask-cns/src/snapshot_loop.rs:85 | 🟢 Accessor/Constructor | 🔴 |
| struct | `SnapshotLoopConfig` | hkask-cns::snapshot_loop | crates/hkask-cns/src/snapshot_loop.rs:27 | 🟡 Type Declaration | 🔴 |
| struct | `SnapshotLoop` | hkask-cns::snapshot_loop | crates/hkask-cns/src/snapshot_loop.rs:66 | 🟡 Type Declaration | 🔴 |
| fn | `can_proceed` | hkask-cns::wallet_budget | crates/hkask-cns/src/wallet_budget.rs:79 | 🔴 Core Logic | 🟢 |
| fn | `check_key_health` | hkask-cns::wallet_budget | crates/hkask-cns/src/wallet_budget.rs:185 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-cns::wallet_budget | crates/hkask-cns/src/wallet_budget.rs:50 | 🟢 Accessor/Constructor | 🟢 |
| fn | `reserve` | hkask-cns::wallet_budget | crates/hkask-cns/src/wallet_budget.rs:135 | 🔴 Core Logic | 🟢 |
| fn | `settle` | hkask-cns::wallet_budget | crates/hkask-cns/src/wallet_budget.rs:152 | 🔴 Core Logic | 🟢 |
| fn | `with_api_key` | hkask-cns::wallet_budget | crates/hkask-cns/src/wallet_budget.rs:62 | 🟢 Accessor/Constructor | 🟢 |
| struct | `WalletBackedBudget` | hkask-cns::wallet_budget | crates/hkask-cns/src/wallet_budget.rs:32 | 🟡 Type Declaration | 🟢 |
| fn | `calibrate` | hkask-cns::wallet_energy_estimator | crates/hkask-cns/src/wallet_energy_estimator.rs:70 | 🔴 Core Logic | 🟢 |
| fn | `current_ratio` | hkask-cns::wallet_energy_estimator | crates/hkask-cns/src/wallet_energy_estimator.rs:97 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-cns::wallet_energy_estimator | crates/hkask-cns/src/wallet_energy_estimator.rs:46 | 🟢 Accessor/Constructor | 🟢 |
| struct | `WalletEnergyEstimator` | hkask-cns::wallet_energy_estimator | crates/hkask-cns/src/wallet_energy_estimator.rs:30 | 🟡 Type Declaration | 🟢 |

| hkask-communication | 17 | 17 | 0 | 100% | 50 |

### hkask-communication

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| enum | `AgentRegistrationError` | hkask-communication::agent_registration | crates/hkask-communication/src/agent_registration.rs:147 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-communication::agent_registration | crates/hkask-communication/src/agent_registration.rs:38 | 🟢 Accessor/Constructor | 🟢 |
| struct | `AgentRegistry` | hkask-communication::agent_registration | crates/hkask-communication/src/agent_registration.rs:26 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-communication::listener | crates/hkask-communication/src/listener.rs:39 | 🟢 Accessor/Constructor | 🟢 |
| struct | `SevenR7Listener` | hkask-communication::listener | crates/hkask-communication/src/listener.rs:23 | 🟡 Type Declaration | 🟢 |
| enum | `MatrixError` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:101 | 🟡 Type Declaration | 🟢 |
| fn | `as_str` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:40 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:63 | 🟢 Accessor/Constructor | 🟢 |
| fn | `healthy` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:454 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:137 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:32 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:55 | 🟢 Accessor/Constructor | 🟢 |
| struct | `MatrixMessage` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:87 | 🟡 Type Declaration | 🟢 |
| struct | `MatrixTransport` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:122 | 🟡 Type Declaration | 🟢 |
| struct | `RoomId` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:24 | 🟡 Type Declaration | 🟢 |
| struct | `Thread` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:70 | 🟡 Type Declaration | 🟢 |
| struct | `UserId` | hkask-communication::matrix | crates/hkask-communication/src/matrix.rs:47 | 🟡 Type Declaration | 🟢 |

| hkask-condenser | 35 | 30 | 5 | 85% | 36 |

### hkask-condenser

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `classify_tool` | hkask-condenser::algorithms | crates/hkask-condenser/src/algorithms.rs:434 | 🔴 Core Logic | 🟢 |
| fn | `list_algorithms` | hkask-condenser::algorithms | crates/hkask-condenser/src/algorithms.rs:398 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-condenser::algorithms | crates/hkask-condenser/src/algorithms.rs:377 | 🟢 Accessor/Constructor | 🟢 |
| fn | `select` | hkask-condenser::algorithms | crates/hkask-condenser/src/algorithms.rs:386 | 🔴 Core Logic | 🟢 |
| struct | `AlgorithmRegistry` | hkask-condenser::algorithms | crates/hkask-condenser/src/algorithms.rs:366 | 🟡 Type Declaration | 🟢 |
| struct | `FlashrankAlgorithm` | hkask-condenser::algorithms | crates/hkask-condenser/src/algorithms.rs:221 | 🟡 Type Declaration | 🟢 |
| struct | `RtkStyleAlgorithm` | hkask-condenser::algorithms | crates/hkask-condenser/src/algorithms.rs:47 | 🟡 Type Declaration | 🟢 |
| struct | `SaliencyRankAlgorithm` | hkask-condenser::algorithms | crates/hkask-condenser/src/algorithms.rs:112 | 🟡 Type Declaration | 🟢 |
| trait | `CondenserAlgorithm` | hkask-condenser::algorithms | crates/hkask-condenser/src/algorithms.rs:33 | 🟡 Type Declaration | 🟢 |
| fn | `check_global_health` | hkask-condenser::engine | crates/hkask-condenser/src/engine.rs:110 | 🔴 Core Logic | 🔴 |
| fn | `classify` | hkask-condenser::engine | crates/hkask-condenser/src/engine.rs:40 | 🔴 Core Logic | 🟢 |
| fn | `compress` | hkask-condenser::engine | crates/hkask-condenser/src/engine.rs:46 | 🔴 Core Logic | 🟢 |
| fn | `get_stats` | hkask-condenser::engine | crates/hkask-condenser/src/engine.rs:101 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-condenser::engine | crates/hkask-condenser/src/engine.rs:26 | 🟢 Accessor/Constructor | 🔴 |
| fn | `set_profile` | hkask-condenser::engine | crates/hkask-condenser/src/engine.rs:96 | 🟢 Accessor/Constructor | 🔴 |
| struct | `CondenserEngine` | hkask-condenser::engine | crates/hkask-condenser/src/engine.rs:13 | 🟡 Type Declaration | 🔴 |
| fn | `approx_token_count` | hkask-condenser::inference | crates/hkask-condenser/src/inference.rs:64 | 🔴 Core Logic | 🟢 |
| fn | `build_summarization_prompt` | hkask-condenser::inference | crates/hkask-condenser/src/inference.rs:27 | 🔴 Core Logic | 🟢 |
| fn | `build_summary_output` | hkask-condenser::inference | crates/hkask-condenser/src/inference.rs:40 | 🔴 Core Logic | 🟢 |
| fn | `format_conversation_text` | hkask-condenser::inference | crates/hkask-condenser/src/inference.rs:13 | 🔴 Core Logic | 🟢 |
| enum | `ContextCategory` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:121 | 🟡 Type Declaration | 🟢 |
| enum | `Profile` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:40 | 🟡 Type Declaration | 🟢 |
| fn | `action_threshold` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:73 | 🔴 Core Logic | 🟢 |
| fn | `label` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:133 | 🔴 Core Logic | 🟢 |
| fn | `max_lines` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:82 | 🔴 Core Logic | 🟢 |
| fn | `retention_pct` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:50 | 🔴 Core Logic | 🟢 |
| struct | `ClassifyRequest` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:24 | 🟡 Type Declaration | 🟢 |
| struct | `CompressRequest` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:12 | 🟡 Type Declaration | 🟢 |
| struct | `CompressedOutput` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:165 | 🟡 Type Declaration | 🟢 |
| struct | `CondenserHealthSignal` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:186 | 🟡 Type Declaration | 🟢 |
| struct | `CondenserStats` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:205 | 🟡 Type Declaration | 🟢 |
| struct | `PersistRequest` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:29 | 🟡 Type Declaration | 🟢 |
| struct | `SetProfileRequest` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:19 | 🟡 Type Declaration | 🟢 |
| struct | `ThreadSummaryOutput` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:244 | 🟡 Type Declaration | 🟢 |
| struct | `ThreadSummaryRequest` | hkask-condenser::types | crates/hkask-condenser/src/types.rs:229 | 🟡 Type Declaration | 🟢 |

| hkask-improv | 51 | 47 | 4 | 92% | 57 |

### hkask-improv

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `apply` | hkask-improv | crates/hkask-improv/src/lib.rs:50 | 🔴 Core Logic | 🔴 |
| fn | `descend` | hkask-improv | crates/hkask-improv/src/lib.rs:88 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-improv | crates/hkask-improv/src/lib.rs:78 | 🟢 Accessor/Constructor | 🟢 |
| fn | `register_with_cns` | hkask-improv | crates/hkask-improv/src/lib.rs:62 | 🔴 Core Logic | 🔴 |
| struct | `ConversationContext` | hkask-improv | crates/hkask-improv/src/lib.rs:69 | 🟡 Type Declaration | 🔴 |
| struct | `ImprovSkill` | hkask-improv | crates/hkask-improv/src/lib.rs:43 | 🟡 Type Declaration | 🔴 |
| enum | `ImprovError` | hkask-improv::cascade | crates/hkask-improv/src/cascade.rs:23 | 🟡 Type Declaration | 🟢 |
| fn | `execute` | hkask-improv::cascade | crates/hkask-improv/src/cascade.rs:71 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-improv::cascade | crates/hkask-improv/src/cascade.rs:50 | 🟢 Accessor/Constructor | 🟢 |
| fn | `step_count` | hkask-improv::cascade | crates/hkask-improv/src/cascade.rs:108 | 🔴 Core Logic | 🟢 |
| fn | `total_applications` | hkask-improv::cascade | crates/hkask-improv/src/cascade.rs:113 | 🔴 Core Logic | 🟢 |
| struct | `ImprovCascade` | hkask-improv::cascade | crates/hkask-improv/src/cascade.rs:40 | 🟡 Type Declaration | 🟢 |
| fn | `cascade_depth_span` | hkask-improv::cns | crates/hkask-improv/src/cns.rs:71 | 🔴 Core Logic | 🟢 |
| fn | `freestyle_coherence_span` | hkask-improv::cns | crates/hkask-improv/src/cns.rs:61 | 🔴 Core Logic | 🟢 |
| fn | `improv_span` | hkask-improv::cns | crates/hkask-improv/src/cns.rs:45 | 🔴 Core Logic | 🟢 |
| fn | `kata_improv_effectiveness_span` | hkask-improv::cns | crates/hkask-improv/src/cns.rs:66 | 🔴 Core Logic | 🟢 |
| fn | `mode_active_span` | hkask-improv::cns | crates/hkask-improv/src/cns.rs:51 | 🔴 Core Logic | 🟢 |
| fn | `plussing_ratio_span` | hkask-improv::cns | crates/hkask-improv/src/cns.rs:56 | 🔴 Core Logic | 🟢 |
| struct | `TracingImprovCns` | hkask-improv::cns | crates/hkask-improv/src/cns.rs:18 | 🟡 Type Declaration | 🟢 |
| trait | `ImprovCns` | hkask-improv::cns | crates/hkask-improv/src/cns.rs:9 | 🟡 Type Declaration | 🟢 |
| fn | `advance_speaker` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:65 | 🔴 Core Logic | 🟢 |
| fn | `cycle` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:84 | 🔴 Core Logic | 🟢 |
| fn | `is_expired` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:50 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:35 | 🟢 Accessor/Constructor | 🟢 |
| fn | `next_speaker` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:60 | 🔴 Core Logic | 🟢 |
| fn | `record_turn` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:70 | 🔴 Core Logic | 🟢 |
| fn | `time_remaining` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:55 | 🔴 Core Logic | 🟢 |
| fn | `to_context` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:111 | 🟢 Accessor/Constructor | 🟢 |
| fn | `turn_count` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:76 | 🔴 Core Logic | 🟢 |
| struct | `FreestyleSession` | hkask-improv::freestyling | crates/hkask-improv/src/freestyling.rs:17 | 🟡 Type Declaration | 🟢 |
| enum | `KataPhase` | hkask-improv::kata | crates/hkask-improv/src/kata.rs:20 | 🟡 Type Declaration | 🟢 |
| fn | `label` | hkask-improv::kata | crates/hkask-improv/src/kata.rs:48 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-improv::kata | crates/hkask-improv/src/kata.rs:74 | 🟢 Accessor/Constructor | 🟢 |
| fn | `recommended_mode` | hkask-improv::kata | crates/hkask-improv/src/kata.rs:38 | 🔴 Core Logic | 🟢 |
| struct | `KataImprovResult` | hkask-improv::kata | crates/hkask-improv/src/kata.rs:64 | 🟡 Type Declaration | 🟢 |
| enum | `ImprovMode` | hkask-improv::modes | crates/hkask-improv/src/modes.rs:18 | 🟡 Type Declaration | 🟢 |
| fn | `label` | hkask-improv::modes | crates/hkask-improv/src/modes.rs:53 | 🔴 Core Logic | 🟢 |
| fn | `respond` | hkask-improv::modes | crates/hkask-improv/src/modes.rs:69 | 🔴 Core Logic | 🟢 |
| fn | `build_on` | hkask-improv::plussing | crates/hkask-improv/src/plussing.rs:196 | 🔴 Core Logic | 🟢 |
| fn | `extract_agreeable` | hkask-improv::plussing | crates/hkask-improv/src/plussing.rs:54 | 🔴 Core Logic | 🟢 |
| fn | `process` | hkask-improv::plussing | crates/hkask-improv/src/plussing.rs:41 | 🔴 Core Logic | 🟢 |
| struct | `AgreeableComponent` | hkask-improv::plussing | crates/hkask-improv/src/plussing.rs:16 | 🟡 Type Declaration | 🟢 |
| struct | `PlussedResponse` | hkask-improv::plussing | crates/hkask-improv/src/plussing.rs:25 | 🟡 Type Declaration | 🟢 |
| enum | `ImprovResponse` | hkask-improv::protocol | crates/hkask-improv/src/protocol.rs:31 | 🟡 Type Declaration | 🟢 |
| fn | `content_text` | hkask-improv::protocol | crates/hkask-improv/src/protocol.rs:64 | 🔴 Core Logic | 🟢 |
| struct | `Contribution` | hkask-improv::protocol | crates/hkask-improv/src/protocol.rs:20 | 🟡 Type Declaration | 🟢 |
| trait | `ImprovProtocol` | hkask-improv::protocol | crates/hkask-improv/src/protocol.rs:13 | 🟡 Type Declaration | 🟢 |
| enum | `RiffOutcome` | hkask-improv::riffing | crates/hkask-improv/src/riffing.rs:26 | 🟡 Type Declaration | 🟢 |
| enum | `RiffReturn` | hkask-improv::riffing | crates/hkask-improv/src/riffing.rs:14 | 🟡 Type Declaration | 🟢 |
| fn | `diverge` | hkask-improv::riffing | crates/hkask-improv/src/riffing.rs:43 | 🔴 Core Logic | 🟢 |
| fn | `resolve` | hkask-improv::riffing | crates/hkask-improv/src/riffing.rs:55 | 🔴 Core Logic | 🟢 |

| hkask-inference | 49 | 49 | 0 | 100% | 97 |

### hkask-inference

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `infer_vision_support` | hkask-inference | crates/hkask-inference/src/lib.rs:81 | 🔴 Core Logic | 🟢 |
| struct | `RouterModelEntry` | hkask-inference | crates/hkask-inference/src/lib.rs:50 | 🟡 Type Declaration | 🟢 |
| fn | `build_chat_request` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:74 | 🔴 Core Logic | 🟢 |
| fn | `chat_response_to_result` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:255 | 🔴 Core Logic | 🟢 |
| fn | `map_token_probs` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:230 | 🔴 Core Logic | 🟢 |
| fn | `map_tool_calls` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:204 | 🔴 Core Logic | 🟢 |
| fn | `parse_sse_stream` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:290 | 🔴 Core Logic | 🟢 |
| fn | `validate_prompt` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:355 | 🔴 Core Logic | 🟢 |
| struct | `ChatChoice` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:115 | 🟡 Type Declaration | 🟢 |
| struct | `ChatMessage` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:57 | 🟡 Type Declaration | 🟢 |
| struct | `ChatRequest` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:32 | 🟡 Type Declaration | 🟢 |
| struct | `ChatResponseMessage` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:125 | 🟡 Type Declaration | 🟢 |
| struct | `ChatResponse` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:108 | 🟡 Type Declaration | 🟢 |
| struct | `ChatUsage` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:131 | 🟡 Type Declaration | 🟢 |
| struct | `RawFunctionCall` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:163 | 🟡 Type Declaration | 🟢 |
| struct | `RawTokenProbTopK` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:148 | 🟡 Type Declaration | 🟢 |
| struct | `RawTokenProb` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:140 | 🟡 Type Declaration | 🟢 |
| struct | `RawToolCall` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:156 | 🟡 Type Declaration | 🟢 |
| struct | `StreamChoice` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:180 | 🟡 Type Declaration | 🟢 |
| struct | `StreamChunk` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:172 | 🟡 Type Declaration | 🟢 |
| struct | `StreamDelta` | hkask-inference::chat_protocol | crates/hkask-inference/src/chat_protocol.rs:188 | 🟡 Type Declaration | 🟢 |
| enum | `ProviderId` | hkask-inference::config | crates/hkask-inference/src/config.rs:38 | 🟡 Type Declaration | 🟢 |
| fn | `as_str` | hkask-inference::config | crates/hkask-inference/src/config.rs:98 | 🟢 Accessor/Constructor | 🟢 |
| fn | `build_client` | hkask-inference::config | crates/hkask-inference/src/config.rs:229 | 🔴 Core Logic | 🟢 |
| fn | `from_env` | hkask-inference::config | crates/hkask-inference/src/config.rs:188 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse_from_model` | hkask-inference::config | crates/hkask-inference/src/config.rs:63 | 🔴 Core Logic | 🟢 |
| fn | `prefix_model` | hkask-inference::config | crates/hkask-inference/src/config.rs:90 | 🔴 Core Logic | 🟢 |
| struct | `InferenceConfig` | hkask-inference::config | crates/hkask-inference/src/config.rs:115 | 🟡 Type Declaration | 🟢 |
| fn | `generate_stream` | hkask-inference::deepinfra_backend | crates/hkask-inference/src/deepinfra_backend.rs:176 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-inference::deepinfra_backend | crates/hkask-inference/src/deepinfra_backend.rs:36 | 🟢 Accessor/Constructor | 🟢 |
| struct | `DeepInfraBackend` | hkask-inference::deepinfra_backend | crates/hkask-inference/src/deepinfra_backend.rs:22 | 🟡 Type Declaration | 🟢 |
| struct | `DeepInfraModelEntry` | hkask-inference::deepinfra_backend | crates/hkask-inference/src/deepinfra_backend.rs:480 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-inference::embedding_router | crates/hkask-inference/src/embedding_router.rs:29 | 🟢 Accessor/Constructor | 🟢 |
| struct | `EmbeddingRouter` | hkask-inference::embedding_router | crates/hkask-inference/src/embedding_router.rs:17 | 🟡 Type Declaration | 🟢 |
| fn | `generate_stream` | hkask-inference::fal_backend | crates/hkask-inference/src/fal_backend.rs:176 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-inference::fal_backend | crates/hkask-inference/src/fal_backend.rs:36 | 🟢 Accessor/Constructor | 🟢 |
| struct | `FalBackend` | hkask-inference::fal_backend | crates/hkask-inference/src/fal_backend.rs:22 | 🟡 Type Declaration | 🟢 |
| struct | `FalModelEntry` | hkask-inference::fal_backend | crates/hkask-inference/src/fal_backend.rs:591 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-inference::inference_router | crates/hkask-inference/src/inference_router.rs:47 | 🟢 Accessor/Constructor | 🟢 |
| struct | `InferenceRouter` | hkask-inference::inference_router | crates/hkask-inference/src/inference_router.rs:25 | 🟡 Type Declaration | 🟢 |
| fn | `generate_stream` | hkask-inference::ollama_backend | crates/hkask-inference/src/ollama_backend.rs:162 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-inference::ollama_backend | crates/hkask-inference/src/ollama_backend.rs:30 | 🟢 Accessor/Constructor | 🟢 |
| struct | `OllamaBackend` | hkask-inference::ollama_backend | crates/hkask-inference/src/ollama_backend.rs:19 | 🟡 Type Declaration | 🟢 |
| struct | `OllamaModelDetails` | hkask-inference::ollama_backend | crates/hkask-inference/src/ollama_backend.rs:262 | 🟡 Type Declaration | 🟢 |
| struct | `OllamaModelEntry` | hkask-inference::ollama_backend | crates/hkask-inference/src/ollama_backend.rs:252 | 🟡 Type Declaration | 🟢 |
| fn | `generate_stream` | hkask-inference::together_backend | crates/hkask-inference/src/together_backend.rs:125 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-inference::together_backend | crates/hkask-inference/src/together_backend.rs:47 | 🟢 Accessor/Constructor | 🟢 |
| struct | `TogetherBackend` | hkask-inference::together_backend | crates/hkask-inference/src/together_backend.rs:18 | 🟡 Type Declaration | 🟢 |
| struct | `TogetherModel` | hkask-inference::together_backend | crates/hkask-inference/src/together_backend.rs:26 | 🟡 Type Declaration | 🟢 |

| hkask-keystore | 44 | 37 | 7 | 84% | 41 |

### hkask-keystore

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| enum | `EncryptionError` | hkask-keystore::encryption | crates/hkask-keystore/src/encryption.rs:32 | 🟡 Type Declaration | 🔴 |
| fn | `decrypt` | hkask-keystore::encryption | crates/hkask-keystore/src/encryption.rs:88 | 🔴 Core Logic | 🔴 |
| fn | `derive_key` | hkask-keystore::encryption | crates/hkask-keystore/src/encryption.rs:116 | 🔴 Core Logic | 🔴 |
| fn | `encrypt` | hkask-keystore::encryption | crates/hkask-keystore/src/encryption.rs:70 | 🔴 Core Logic | 🔴 |
| fn | `generate_salt` | hkask-keystore::encryption | crates/hkask-keystore/src/encryption.rs:63 | 🔴 Core Logic | 🔴 |
| fn | `new` | hkask-keystore::encryption | crates/hkask-keystore/src/encryption.rs:50 | 🟢 Accessor/Constructor | 🟢 |
| struct | `EncryptionService` | hkask-keystore::encryption | crates/hkask-keystore/src/encryption.rs:44 | 🟡 Type Declaration | 🔴 |
| enum | `KeystoreError` | hkask-keystore::error | crates/hkask-keystore/src/error.rs:5 | 🟡 Type Declaration | 🔴 |
| enum | `KeychainError` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:14 | 🟡 Type Declaration | 🟢 |
| fn | `delete_by_key` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:129 | 🔴 Core Logic | 🟢 |
| fn | `delete` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:85 | 🔴 Core Logic | 🟢 |
| fn | `get_or_create_ocap_secret` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:310 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:44 | 🟢 Accessor/Constructor | 🟢 |
| fn | `resolve_acp_secret` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:197 | 🔴 Core Logic | 🟢 |
| fn | `resolve_capability_key` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:264 | 🔴 Core Logic | 🟢 |
| fn | `resolve_db_passphrase` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:290 | 🔴 Core Logic | 🟢 |
| fn | `resolve_mcp_secret` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:223 | 🔴 Core Logic | 🟢 |
| fn | `resolve_mcp_security_key` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:244 | 🔴 Core Logic | 🟢 |
| fn | `resolve_secret_chain` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:171 | 🔴 Core Logic | 🟢 |
| fn | `resolve_treasury_key` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:416 | 🔴 Core Logic | 🟢 |
| fn | `resolve_wallet_seed` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:443 | 🔴 Core Logic | 🟢 |
| fn | `resolve` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:353 | 🔴 Core Logic | 🟢 |
| fn | `retrieve_by_key` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:117 | 🔴 Core Logic | 🟢 |
| fn | `retrieve` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:72 | 🔴 Core Logic | 🟢 |
| fn | `sign_api_key_capability` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:463 | 🔴 Core Logic | 🟢 |
| fn | `store_by_key` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:101 | 🔴 Core Logic | 🟢 |
| fn | `store` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:56 | 🔴 Core Logic | 🟢 |
| struct | `Keychain` | hkask-keystore::keychain | crates/hkask-keystore/src/keychain.rs:35 | 🟡 Type Declaration | 🟢 |
| fn | `derive_all_internal_secrets_with_version` | hkask-keystore::master_key | crates/hkask-keystore/src/master_key.rs:108 | 🔴 Core Logic | 🟢 |
| fn | `derive_all_internal_secrets` | hkask-keystore::master_key | crates/hkask-keystore/src/master_key.rs:93 | 🔴 Core Logic | 🟢 |
| fn | `derive_sub_key_with_version` | hkask-keystore::master_key | crates/hkask-keystore/src/master_key.rs:190 | 🔴 Core Logic | 🟢 |
| fn | `derive_sub_key` | hkask-keystore::master_key | crates/hkask-keystore/src/master_key.rs:166 | 🔴 Core Logic | 🟢 |
| struct | `InternalSecrets` | hkask-keystore::master_key | crates/hkask-keystore/src/master_key.rs:56 | 🟡 Type Declaration | 🟢 |
| enum | `SpecSignatureError` | hkask-keystore::spec_signer | crates/hkask-keystore/src/spec_signer.rs:99 | 🟡 Type Declaration | 🟢 |
| fn | `from_master_secret` | hkask-keystore::spec_signer | crates/hkask-keystore/src/spec_signer.rs:30 | 🟢 Accessor/Constructor | 🟢 |
| fn | `sign_spec` | hkask-keystore::spec_signer | crates/hkask-keystore/src/spec_signer.rs:49 | 🔴 Core Logic | 🟢 |
| fn | `verify_spec` | hkask-keystore::spec_signer | crates/hkask-keystore/src/spec_signer.rs:63 | 🔴 Core Logic | 🟢 |
| fn | `verifying_key_hex` | hkask-keystore::spec_signer | crates/hkask-keystore/src/spec_signer.rs:92 | 🔴 Core Logic | 🟢 |
| fn | `verifying_key` | hkask-keystore::spec_signer | crates/hkask-keystore/src/spec_signer.rs:84 | 🔴 Core Logic | 🟢 |
| struct | `Ed25519SpecSigner` | hkask-keystore::spec_signer | crates/hkask-keystore/src/spec_signer.rs:16 | 🟡 Type Declaration | 🟢 |
| fn | `increment_key_version` | hkask-keystore::version_file | crates/hkask-keystore/src/version_file.rs:65 | 🔴 Core Logic | 🟢 |
| fn | `read_key_version` | hkask-keystore::version_file | crates/hkask-keystore/src/version_file.rs:34 | 🔴 Core Logic | 🟢 |
| fn | `version_file_path` | hkask-keystore::version_file | crates/hkask-keystore/src/version_file.rs:20 | 🔴 Core Logic | 🟢 |
| fn | `write_key_version` | hkask-keystore::version_file | crates/hkask-keystore/src/version_file.rs:49 | 🔴 Core Logic | 🟢 |

| hkask-mcp | 65 | 65 | 0 | 100% | 94 |

### hkask-mcp

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `configure_git_cas_port` | hkask-mcp::adapter_container | crates/hkask-mcp/src/adapter_container.rs:39 | 🔴 Core Logic | 🟢 |
| fn | `get_git_cas_port` | hkask-mcp::adapter_container | crates/hkask-mcp/src/adapter_container.rs:54 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-mcp::adapter_container | crates/hkask-mcp/src/adapter_container.rs:24 | 🟢 Accessor/Constructor | 🟢 |
| struct | `AdapterContainer` | hkask-mcp::adapter_container | crates/hkask-mcp/src/adapter_container.rs:14 | 🟡 Type Declaration | 🟢 |
| enum | `DaemonRequest` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:47 | 🟡 Type Declaration | 🟢 |
| enum | `DaemonResponse` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:70 | 🟡 Type Declaration | 🟢 |
| fn | `daemon_socket_path` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:34 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:109 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:251 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_path` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:120 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_path` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:263 | 🟢 Accessor/Constructor | 🟢 |
| struct | `DaemonClient` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:100 | 🟡 Type Declaration | 🟢 |
| struct | `DaemonListener` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:235 | 🟡 Type Declaration | 🟢 |
| trait | `DaemonHandler` | hkask-mcp::daemon | crates/hkask-mcp/src/daemon.rs:209 | 🟡 Type Declaration | 🟢 |
| fn | `issue_capability` | hkask-mcp::dispatch | crates/hkask-mcp/src/dispatch.rs:217 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-mcp::dispatch | crates/hkask-mcp/src/dispatch.rs:48 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_governed_tool` | hkask-mcp::dispatch | crates/hkask-mcp/src/dispatch.rs:200 | 🟢 Accessor/Constructor | 🟢 |
| struct | `McpDispatcher` | hkask-mcp::dispatch | crates/hkask-mcp/src/dispatch.rs:178 | 🟡 Type Declaration | 🟢 |
| struct | `RawMcpToolPort` | hkask-mcp::dispatch | crates/hkask-mcp/src/dispatch.rs:38 | 🟡 Type Declaration | 🟢 |
| fn | `from_env` | hkask-mcp::git_cas::gix_adapter | crates/hkask-mcp/src/git_cas/gix_adapter.rs:106 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-mcp::git_cas::gix_adapter | crates/hkask-mcp/src/git_cas/gix_adapter.rs:92 | 🟢 Accessor/Constructor | 🟢 |
| struct | `GixCasAdapter` | hkask-mcp::git_cas::gix_adapter | crates/hkask-mcp/src/git_cas/gix_adapter.rs:17 | 🟡 Type Declaration | 🟢 |
| fn | `from_path` | hkask-mcp::git_cas::mod | crates/hkask-mcp/src/git_cas/mod.rs:31 | 🟢 Accessor/Constructor | 🟢 |
| fn | `load_template_crate` | hkask-mcp::git_cas::mod | crates/hkask-mcp/src/git_cas/mod.rs:68 | 🔴 Core Logic | 🟢 |
| struct | `GitCasAdapter` | hkask-mcp::git_cas::mod | crates/hkask-mcp/src/git_cas/mod.rs:21 | 🟡 Type Declaration | 🟢 |
| enum | `ServerStartError` | hkask-mcp::runtime | crates/hkask-mcp/src/runtime.rs:89 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-mcp::runtime | crates/hkask-mcp/src/runtime.rs:116 | 🟢 Accessor/Constructor | 🟢 |
| fn | `validate_input` | hkask-mcp::runtime | crates/hkask-mcp/src/runtime.rs:43 | 🔴 Core Logic | 🟢 |
| struct | `McpRuntime` | hkask-mcp::runtime | crates/hkask-mcp/src/runtime.rs:100 | 🟡 Type Declaration | 🟢 |
| struct | `McpServer` | hkask-mcp::runtime | crates/hkask-mcp/src/runtime.rs:77 | 🟡 Type Declaration | 🟢 |
| struct | `McpTool` | hkask-mcp::runtime | crates/hkask-mcp/src/runtime.rs:24 | 🟡 Type Declaration | 🟢 |
| fn | `classify_http_error` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:494 | 🔴 Core Logic | 🟢 |
| fn | `cns_available` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:132 | 🔴 Core Logic | 🟢 |
| fn | `detect` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:101 | 🔴 Core Logic | 🟢 |
| fn | `error` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:237 | 🔴 Core Logic | 🟢 |
| fn | `failed_precondition` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:403 | 🔴 Core Logic | 🟢 |
| fn | `finish` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:266 | 🔴 Core Logic | 🟢 |
| fn | `internal_error` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:279 | 🔴 Core Logic | 🟢 |
| fn | `internal` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:354 | 🔴 Core Logic | 🟢 |
| fn | `invalid_argument` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:368 | 🔴 Core Logic | 🟢 |
| fn | `load_dotenv` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:566 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:211 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:343 | 🟢 Accessor/Constructor | 🟢 |
| fn | `not_found` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:361 | 🔴 Core Logic | 🟢 |
| fn | `ok_json` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:256 | 🔴 Core Logic | 🟢 |
| fn | `ok` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:225 | 🔴 Core Logic | 🟢 |
| fn | `open_database_with_extensions` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:176 | 🔴 Core Logic | 🟢 |
| fn | `open_database` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:157 | 🔴 Core Logic | 🟢 |
| fn | `optional` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:65 | 🔴 Core Logic | 🟢 |
| fn | `permission_denied` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:389 | 🔴 Core Logic | 🟢 |
| fn | `rate_limited` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:396 | 🔴 Core Logic | 🟢 |
| fn | `required` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:51 | 🔴 Core Logic | 🟢 |
| fn | `resolve_credential` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:599 | 🔴 Core Logic | 🟢 |
| fn | `timeout` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:382 | 🔴 Core Logic | 🟢 |
| fn | `to_json_string` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:410 | 🟢 Accessor/Constructor | 🟢 |
| fn | `tool_internal_error` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:432 | 🔴 Core Logic | 🟢 |
| fn | `unavailable` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:375 | 🔴 Core Logic | 🟢 |
| fn | `validate_identifier` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:449 | 🔴 Core Logic | 🟢 |
| fn | `validate_tool_url` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:482 | 🔴 Core Logic | 🟢 |
| struct | `CapabilityTier` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:85 | 🟡 Type Declaration | 🟢 |
| struct | `CredentialRequirement` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:34 | 🟡 Type Declaration | 🟢 |
| struct | `McpToolError` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:330 | 🟡 Type Declaration | 🟢 |
| struct | `ServerContext` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:138 | 🟡 Type Declaration | 🟢 |
| struct | `ToolSpanGuard` | hkask-mcp::server | crates/hkask-mcp/src/server.rs:198 | 🟡 Type Declaration | 🟢 |
| struct | `StartupGateResult` | hkask-mcp::startup | crates/hkask-mcp/src/startup.rs:42 | 🟡 Type Declaration | 🟢 |

| hkask-mcp-communication | 27 | 0 | 27 | 0% | 0 |

### hkask-mcp-communication

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `new` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:92 | 🟢 Accessor/Constructor | 🔴 |
| struct | `CommunicationServer` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:85 | 🟡 Type Declaration | 🔴 |
| struct | `CreateThreadRequest` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:59 | 🟡 Type Declaration | 🔴 |
| struct | `InviteAgentRequest` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:65 | 🟡 Type Declaration | 🔴 |
| struct | `ListVoicesRequest` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:48 | 🟡 Type Declaration | 🔴 |
| struct | `MonitorThreadRequest` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:71 | 🟡 Type Declaration | 🔴 |
| struct | `SendMessageRequest` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:53 | 🟡 Type Declaration | 🔴 |
| struct | `TagAgentRequest` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:77 | 🟡 Type Declaration | 🔴 |
| struct | `TtsGenerateRequest` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:41 | 🟡 Type Declaration | 🔴 |
| struct | `TtsSpeakRequest` | hkask-mcp-communication | mcp-servers/hkask-mcp-communication/src/main.rs:30 | 🟡 Type Declaration | 🔴 |
| enum | `AgentRegistrationError` | hkask-mcp-communication::agent_registration | mcp-servers/hkask-mcp-communication/src/agent_registration.rs:118 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-communication::agent_registration | mcp-servers/hkask-mcp-communication/src/agent_registration.rs:35 | 🟢 Accessor/Constructor | 🔴 |
| struct | `AgentRegistry` | hkask-mcp-communication::agent_registration | mcp-servers/hkask-mcp-communication/src/agent_registration.rs:26 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-communication::listener | mcp-servers/hkask-mcp-communication/src/listener.rs:34 | 🟢 Accessor/Constructor | 🔴 |
| struct | `SevenR7Listener` | hkask-mcp-communication::listener | mcp-servers/hkask-mcp-communication/src/listener.rs:23 | 🟡 Type Declaration | 🔴 |
| enum | `MatrixError` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:83 | 🟡 Type Declaration | 🔴 |
| fn | `as_str` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:31 | 🟢 Accessor/Constructor | 🔴 |
| fn | `as_str` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:45 | 🟢 Accessor/Constructor | 🔴 |
| fn | `healthy` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:381 | 🔴 MCP Tool Handler | 🔴 |
| fn | `new` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:115 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:27 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:41 | 🟢 Accessor/Constructor | 🔴 |
| struct | `MatrixMessage` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:69 | 🟡 Type Declaration | 🔴 |
| struct | `MatrixTransport` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:104 | 🟡 Type Declaration | 🔴 |
| struct | `RoomId` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:24 | 🟡 Type Declaration | 🔴 |
| struct | `Thread` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:52 | 🟡 Type Declaration | 🔴 |
| struct | `UserId` | hkask-mcp-communication::matrix | mcp-servers/hkask-mcp-communication/src/matrix.rs:38 | 🟡 Type Declaration | 🔴 |

| hkask-mcp-companies | 65 | 44 | 21 | 67% | 41 |

### hkask-mcp-companies

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `new` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:210 | 🟢 Accessor/Constructor | 🔴 |
| struct | `AttributionRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:114 | 🟡 Type Declaration | 🔴 |
| struct | `CharacteristicsRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:121 | 🟡 Type Declaration | 🔴 |
| struct | `CompaniesServer` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:197 | 🟡 Type Declaration | 🔴 |
| struct | `ExpectationsGapRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:127 | 🟡 Type Declaration | 🔴 |
| struct | `FileAttachRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:165 | 🟡 Type Declaration | 🔴 |
| struct | `FileDeleteRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:184 | 🟡 Type Declaration | 🔴 |
| struct | `FileListRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:178 | 🟡 Type Declaration | 🔴 |
| struct | `HistoricalRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:68 | 🟡 Type Declaration | 🔴 |
| struct | `LedgerExportRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:102 | 🟡 Type Declaration | 🔴 |
| struct | `LedgerImportRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:95 | 🟡 Type Declaration | 🔴 |
| struct | `NoteAddRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:140 | 🟡 Type Declaration | 🔴 |
| struct | `NoteDeleteRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:160 | 🟡 Type Declaration | 🔴 |
| struct | `NoteListRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:151 | 🟡 Type Declaration | 🔴 |
| struct | `PortfolioCompareRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:108 | 🟡 Type Declaration | 🔴 |
| struct | `PortfolioNameRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:83 | 🟡 Type Declaration | 🔴 |
| struct | `PortfolioReturnsRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:133 | 🟡 Type Declaration | 🔴 |
| struct | `SearchRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:75 | 🟡 Type Declaration | 🔴 |
| struct | `SymbolLimitRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:62 | 🟡 Type Declaration | 🔴 |
| struct | `SymbolRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:57 | 🟡 Type Declaration | 🔴 |
| struct | `TransactionNoteRequest` | hkask-mcp-companies | mcp-servers/hkask-mcp-companies/src/main.rs:88 | 🟡 Type Declaration | 🔴 |
| enum | `CeoRating` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:107 | 🟡 Type Declaration | 🟢 |
| enum | `MoatRating` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:47 | 🟡 Type Declaration | 🟢 |
| fn | `ceo_capital_allocation_score` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:121 | 🔴 MCP Tool Handler | 🟢 |
| fn | `classify_moat` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:54 | 🔴 MCP Tool Handler | 🟢 |
| fn | `extract_gross_margins` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:72 | 🔴 MCP Tool Handler | 🟢 |
| fn | `extract_invested_capital` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:176 | 🔴 MCP Tool Handler | 🟢 |
| fn | `extract_roic` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:157 | 🔴 MCP Tool Handler | 🟢 |
| fn | `extract_wc_days` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:93 | 🔴 MCP Tool Handler | 🟢 |
| fn | `gross_margin_stability` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:11 | 🔴 MCP Tool Handler | 🟢 |
| fn | `wc_signal_label` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:32 | 🔴 MCP Tool Handler | 🟢 |
| fn | `working_capital_spread` | hkask-mcp-companies::analysis | mcp-servers/hkask-mcp-companies/src/analysis.rs:27 | 🔴 MCP Tool Handler | 🟢 |
| fn | `add_note` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:872 | 🔴 MCP Tool Handler | 🟢 |
| fn | `add_transaction` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:285 | 🔴 MCP Tool Handler | 🟢 |
| fn | `append_note` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:310 | 🔴 MCP Tool Handler | 🟢 |
| fn | `attach_file` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:976 | 🔴 MCP Tool Handler | 🟢 |
| fn | `compare` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:798 | 🔴 MCP Tool Handler | 🟢 |
| fn | `create` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:231 | 🔴 MCP Tool Handler | 🟢 |
| fn | `delete_file` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:1051 | 🔴 MCP Tool Handler | 🟢 |
| fn | `delete_note` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:960 | 🔴 MCP Tool Handler | 🟢 |
| fn | `delete` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:244 | 🔴 MCP Tool Handler | 🟢 |
| fn | `export_csv` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:597 | 🔴 MCP Tool Handler | 🟢 |
| fn | `export_json` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:592 | 🔴 MCP Tool Handler | 🟢 |
| fn | `get_date_range` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:675 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_missing_price_dates` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:709 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_prices` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:768 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_symbols` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:624 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_transactions` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:334 | 🟢 Accessor/Constructor | 🟢 |
| fn | `import_csv` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:498 | 🔴 MCP Tool Handler | 🟢 |
| fn | `import_json` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:492 | 🔴 MCP Tool Handler | 🟢 |
| fn | `link_security` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:657 | 🔴 MCP Tool Handler | 🟢 |
| fn | `list_files` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:1015 | 🔴 MCP Tool Handler | 🟢 |
| fn | `list_notes` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:895 | 🔴 MCP Tool Handler | 🟢 |
| fn | `list` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:255 | 🔴 MCP Tool Handler | 🟢 |
| fn | `new` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:69 | 🟢 Accessor/Constructor | 🟢 |
| fn | `resolve_symbol` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:642 | 🔴 MCP Tool Handler | 🟢 |
| fn | `store_price` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:749 | 🔴 MCP Tool Handler | 🟢 |
| fn | `validate` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:396 | 🔴 MCP Tool Handler | 🟢 |
| fn | `with_dir` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:146 | 🟢 Accessor/Constructor | 🟢 |
| struct | `PortfolioManager` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:58 | 🟡 Type Declaration | 🟢 |
| struct | `PositionSummary` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:49 | 🟡 Type Declaration | 🟢 |
| struct | `Transaction` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:16 | 🟡 Type Declaration | 🟢 |
| struct | `ValidationReport` | hkask-mcp-companies::portfolio | mcp-servers/hkask-mcp-companies/src/portfolio.rs:40 | 🟡 Type Declaration | 🟢 |
| enum | `Provider` | hkask-mcp-companies::providers | mcp-servers/hkask-mcp-companies/src/providers.rs:13 | 🟡 Type Declaration | 🟢 |
| struct | `EndpointMapping` | hkask-mcp-companies::providers | mcp-servers/hkask-mcp-companies/src/providers.rs:25 | 🟡 Type Declaration | 🟢 |

| hkask-mcp-condenser | 1 | 0 | 1 | 0% | 0 |

### hkask-mcp-condenser

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| struct | `CondenserServer` | hkask-mcp-condenser | mcp-servers/hkask-mcp-condenser/src/main.rs:41 | 🟡 Type Declaration | 🔴 |

| hkask-mcp-docproc | 41 | 29 | 12 | 70% | 73 |

### hkask-mcp-docproc

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `detect_format` | hkask-mcp-docproc::convert | mcp-servers/hkask-mcp-docproc/src/convert.rs:10 | 🔴 MCP Tool Handler | 🔴 |
| fn | `is_format_supported` | hkask-mcp-docproc::convert | mcp-servers/hkask-mcp-docproc/src/convert.rs:52 | 🟢 Accessor/Constructor | 🔴 |
| fn | `strip_frontmatter` | hkask-mcp-docproc::convert | mcp-servers/hkask-mcp-docproc/src/convert.rs:57 | 🔴 MCP Tool Handler | 🔴 |
| fn | `strip_html` | hkask-mcp-docproc::convert | mcp-servers/hkask-mcp-docproc/src/convert.rs:74 | 🔴 MCP Tool Handler | 🔴 |
| fn | `analyze_threshold_drift` | hkask-mcp-docproc::ocr::calibration | mcp-servers/hkask-mcp-docproc/src/ocr/calibration.rs:55 | 🔴 MCP Tool Handler | 🟢 |
| fn | `emit_drift_alert` | hkask-mcp-docproc::ocr::calibration | mcp-servers/hkask-mcp-docproc/src/ocr/calibration.rs:102 | 🔴 MCP Tool Handler | 🟢 |
| struct | `DriftEvidence` | hkask-mcp-docproc::ocr::calibration | mcp-servers/hkask-mcp-docproc/src/ocr/calibration.rs:19 | 🟡 Type Declaration | 🟢 |
| struct | `ThresholdDriftAlert` | hkask-mcp-docproc::ocr::calibration | mcp-servers/hkask-mcp-docproc/src/ocr/calibration.rs:32 | 🟡 Type Declaration | 🟢 |
| fn | `score_page_complexity` | hkask-mcp-docproc::ocr::complexity | mcp-servers/hkask-mcp-docproc/src/ocr/complexity.rs:25 | 🔴 MCP Tool Handler | 🟢 |
| fn | `compute_cross_validation` | hkask-mcp-docproc::ocr::cross_validation | mcp-servers/hkask-mcp-docproc/src/ocr/cross_validation.rs:13 | 🔴 MCP Tool Handler | 🟢 |
| fn | `new` | hkask-mcp-docproc::ocr::llm_ocr | mcp-servers/hkask-mcp-docproc/src/ocr/llm_ocr.rs:34 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_max_tokens` | hkask-mcp-docproc::ocr::llm_ocr | mcp-servers/hkask-mcp-docproc/src/ocr/llm_ocr.rs:42 | 🟢 Accessor/Constructor | 🟢 |
| struct | `LlmOcrExecutor` | hkask-mcp-docproc::ocr::llm_ocr | mcp-servers/hkask-mcp-docproc/src/ocr/llm_ocr.rs:25 | 🟡 Type Declaration | 🟢 |
| trait | `OcrExecutor` | hkask-mcp-docproc::ocr::pipeline | mcp-servers/hkask-mcp-docproc/src/ocr/pipeline.rs:30 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-mcp-docproc::ocr::routing | mcp-servers/hkask-mcp-docproc/src/ocr/routing.rs:29 | 🟢 Accessor/Constructor | 🟢 |
| fn | `route_page` | hkask-mcp-docproc::ocr::routing | mcp-servers/hkask-mcp-docproc/src/ocr/routing.rs:68 | 🔴 MCP Tool Handler | 🟢 |
| fn | `set_force_fallback` | hkask-mcp-docproc::ocr::routing | mcp-servers/hkask-mcp-docproc/src/ocr/routing.rs:46 | 🟢 Accessor/Constructor | 🟢 |
| struct | `SamplingState` | hkask-mcp-docproc::ocr::routing | mcp-servers/hkask-mcp-docproc/src/ocr/routing.rs:12 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-mcp-docproc::ocr::tesseract | mcp-servers/hkask-mcp-docproc/src/ocr/tesseract.rs:27 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_language` | hkask-mcp-docproc::ocr::tesseract | mcp-servers/hkask-mcp-docproc/src/ocr/tesseract.rs:35 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_psm` | hkask-mcp-docproc::ocr::tesseract | mcp-servers/hkask-mcp-docproc/src/ocr/tesseract.rs:41 | 🟢 Accessor/Constructor | 🟢 |
| struct | `TesseractExecutor` | hkask-mcp-docproc::ocr::tesseract | mcp-servers/hkask-mcp-docproc/src/ocr/tesseract.rs:18 | 🟡 Type Declaration | 🟢 |
| fn | `estimate_word_count` | hkask-mcp-docproc::ocr::verification | mcp-servers/hkask-mcp-docproc/src/ocr/verification.rs:77 | 🔴 MCP Tool Handler | 🟢 |
| fn | `verify_output` | hkask-mcp-docproc::ocr::verification | mcp-servers/hkask-mcp-docproc/src/ocr/verification.rs:18 | 🔴 MCP Tool Handler | 🟢 |
| fn | `default_ocr_max_tokens` | hkask-mcp-docproc::server | mcp-servers/hkask-mcp-docproc/src/server.rs:25 | 🔴 MCP Tool Handler | 🔴 |
| fn | `has_ocr` | hkask-mcp-docproc::server | mcp-servers/hkask-mcp-docproc/src/server.rs:90 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-mcp-docproc::server | mcp-servers/hkask-mcp-docproc/src/server.rs:149 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-mcp-docproc::server | mcp-servers/hkask-mcp-docproc/src/server.rs:65 | 🟢 Accessor/Constructor | 🔴 |
| fn | `record_experience` | hkask-mcp-docproc::server | mcp-servers/hkask-mcp-docproc/src/server.rs:347 | 🔴 MCP Tool Handler | 🔴 |
| struct | `DocProcCnsObserver` | hkask-mcp-docproc::server | mcp-servers/hkask-mcp-docproc/src/server.rs:143 | 🟡 Type Declaration | 🔴 |
| struct | `DocProcServer` | hkask-mcp-docproc::server | mcp-servers/hkask-mcp-docproc/src/server.rs:31 | 🟡 Type Declaration | 🔴 |
| struct | `IndexedPassage` | hkask-mcp-docproc::server | mcp-servers/hkask-mcp-docproc/src/server.rs:57 | 🟡 Type Declaration | 🔴 |
| struct | `CacheRequest` | hkask-mcp-docproc::tools | mcp-servers/hkask-mcp-docproc/src/tools.rs:196 | 🟡 Type Declaration | 🟢 |
| struct | `ChunkRequest` | hkask-mcp-docproc::tools | mcp-servers/hkask-mcp-docproc/src/tools.rs:127 | 🟡 Type Declaration | 🟢 |
| struct | `ClearIndexRequest` | hkask-mcp-docproc::tools | mcp-servers/hkask-mcp-docproc/src/tools.rs:216 | 🟡 Type Declaration | 🟢 |
| struct | `ConvertRequest` | hkask-mcp-docproc::tools | mcp-servers/hkask-mcp-docproc/src/tools.rs:106 | 🟡 Type Declaration | 🟢 |
| struct | `EmbedRequest` | hkask-mcp-docproc::tools | mcp-servers/hkask-mcp-docproc/src/tools.rs:187 | 🟡 Type Declaration | 🟢 |
| struct | `ExtractTriplesRequest` | hkask-mcp-docproc::tools | mcp-servers/hkask-mcp-docproc/src/tools.rs:175 | 🟡 Type Declaration | 🟢 |
| struct | `GenerateQaRequest` | hkask-mcp-docproc::tools | mcp-servers/hkask-mcp-docproc/src/tools.rs:167 | 🟡 Type Declaration | 🟢 |
| struct | `OcrRequest` | hkask-mcp-docproc::tools | mcp-servers/hkask-mcp-docproc/src/tools.rs:115 | 🟡 Type Declaration | 🟢 |
| struct | `QueryRequest` | hkask-mcp-docproc::tools | mcp-servers/hkask-mcp-docproc/src/tools.rs:204 | 🟡 Type Declaration | 🟢 |

| hkask-mcp-media | 50 | 47 | 3 | 94% | 29 |

### hkask-mcp-media

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `new` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:460 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ApplyStyleRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:267 | 🟡 Type Declaration | 🟢 |
| struct | `AudioCaptureRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:402 | 🟡 Type Declaration | 🟢 |
| struct | `CreateCollageRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:274 | 🟡 Type Declaration | 🟢 |
| struct | `DescribeImageRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:88 | 🟡 Type Declaration | 🟢 |
| struct | `ExtractObjectRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:207 | 🟡 Type Declaration | 🟢 |
| struct | `FaceListRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:195 | 🟡 Type Declaration | 🟢 |
| struct | `FaceRegisterRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:181 | 🟡 Type Declaration | 🟢 |
| struct | `FaceRemoveRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:201 | 🟡 Type Declaration | 🟢 |
| struct | `FaceValidateRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:175 | 🟡 Type Declaration | 🟢 |
| struct | `GalleryAnalyzeRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:128 | 🟡 Type Declaration | 🟢 |
| struct | `GalleryFindSimilarRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:240 | 🟡 Type Declaration | 🟢 |
| struct | `GalleryNameFaceRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:163 | 🟡 Type Declaration | 🟢 |
| struct | `GalleryOrganizeRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:96 | 🟡 Type Declaration | 🟢 |
| struct | `GalleryRefreshRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:149 | 🟡 Type Declaration | 🟢 |
| struct | `GallerySearchRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:120 | 🟡 Type Declaration | 🟢 |
| struct | `GalleryTimelineRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:215 | 🟡 Type Declaration | 🟢 |
| struct | `GenerateImageRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:62 | 🟡 Type Declaration | 🟢 |
| struct | `GenerateSpeechRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:386 | 🟡 Type Declaration | 🟢 |
| struct | `GenerateVideoRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:82 | 🟡 Type Declaration | 🟢 |
| struct | `ImageToVideoRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:318 | 🟡 Type Declaration | 🟢 |
| struct | `MediaServer` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:39 | 🟡 Type Declaration | 🟢 |
| struct | `RecordAndTranscribeRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:410 | 🟡 Type Declaration | 🟢 |
| struct | `RemoveBackgroundRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:261 | 🟡 Type Declaration | 🟢 |
| struct | `TranscribeRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:394 | 🟡 Type Declaration | 🟢 |
| struct | `TransformImageRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:69 | 🟡 Type Declaration | 🟢 |
| struct | `UpscaleImageRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:76 | 🟡 Type Declaration | 🟢 |
| struct | `VideoAddCaptionRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:326 | 🟡 Type Declaration | 🟢 |
| struct | `VideoCaptionRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:354 | 🟡 Type Declaration | 🟢 |
| struct | `VideoClipRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:302 | 🟡 Type Declaration | 🟢 |
| struct | `VideoConcatRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:349 | 🟡 Type Declaration | 🟢 |
| struct | `VideoFromImagesRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:342 | 🟡 Type Declaration | 🟢 |
| struct | `VideoMemeRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:360 | 🟡 Type Declaration | 🟢 |
| struct | `VideoRemixRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:334 | 🟡 Type Declaration | 🟢 |
| struct | `VideoToGifRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:309 | 🟡 Type Declaration | 🟢 |
| struct | `VoiceDesignRequest` | hkask-mcp-media | mcp-servers/hkask-mcp-media/src/main.rs:381 | 🟡 Type Declaration | 🟢 |
| fn | `ensure_meta_dir` | hkask-mcp-media::gallery::state | mcp-servers/hkask-mcp-media/src/gallery/state.rs:109 | 🔴 MCP Tool Handler | 🟢 |
| fn | `new` | hkask-mcp-media::gallery::state | mcp-servers/hkask-mcp-media/src/gallery/state.rs:77 | 🟢 Accessor/Constructor | 🟢 |
| fn | `scan` | hkask-mcp-media::gallery::state | mcp-servers/hkask-mcp-media/src/gallery/state.rs:123 | 🔴 MCP Tool Handler | 🟢 |
| fn | `summary` | hkask-mcp-media::gallery::state | mcp-servers/hkask-mcp-media/src/gallery/state.rs:221 | 🔴 MCP Tool Handler | 🟢 |
| fn | `validate` | hkask-mcp-media::gallery::state | mcp-servers/hkask-mcp-media/src/gallery/state.rs:92 | 🔴 MCP Tool Handler | 🟢 |
| struct | `GalleryState` | hkask-mcp-media::gallery::state | mcp-servers/hkask-mcp-media/src/gallery/state.rs:23 | 🟡 Type Declaration | 🟢 |
| struct | `ImageEntry` | hkask-mcp-media::gallery::state | mcp-servers/hkask-mcp-media/src/gallery/state.rs:56 | 🟡 Type Declaration | 🟢 |
| struct | `ScanResult` | hkask-mcp-media::gallery::state | mcp-servers/hkask-mcp-media/src/gallery/state.rs:44 | 🟡 Type Declaration | 🟢 |
| struct | `FaceMatchResult` | hkask-mcp-media::gallery::vision | mcp-servers/hkask-mcp-media/src/gallery/vision.rs:45 | 🟡 Type Declaration | 🟢 |
| struct | `FaceValidationResult` | hkask-mcp-media::gallery::vision | mcp-servers/hkask-mcp-media/src/gallery/vision.rs:24 | 🟡 Type Declaration | 🟢 |
| fn | `create_env` | hkask-mcp-media::templates | mcp-servers/hkask-mcp-media/src/templates.rs:10 | 🔴 MCP Tool Handler | 🔴 |
| fn | `render` | hkask-mcp-media::templates | mcp-servers/hkask-mcp-media/src/templates.rs:28 | 🔴 MCP Tool Handler | 🔴 |
| fn | `detect` | hkask-mcp-media::video::ffmpeg | mcp-servers/hkask-mcp-media/src/video/ffmpeg.rs:19 | 🔴 MCP Tool Handler | 🟢 |
| struct | `FfmpegRunner` | hkask-mcp-media::video::ffmpeg | mcp-servers/hkask-mcp-media/src/video/ffmpeg.rs:11 | 🟡 Type Declaration | 🔴 |

| hkask-mcp-memory | 14 | 0 | 14 | 0% | 0 |

### hkask-mcp-memory

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `new` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:142 | 🟢 Accessor/Constructor | 🔴 |
| struct | `BackupRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:111 | 🟡 Type Declaration | 🔴 |
| struct | `BudgetRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:60 | 🟡 Type Declaration | 🔴 |
| struct | `CentroidRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:81 | 🟡 Type Declaration | 🔴 |
| struct | `ChunkTextRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:96 | 🟡 Type Declaration | 🔴 |
| struct | `ConsolidateStatusRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:63 | 🟡 Type Declaration | 🔴 |
| struct | `CountRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:106 | 🟡 Type Declaration | 🔴 |
| struct | `EmbedRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:68 | 🟡 Type Declaration | 🔴 |
| struct | `MemoryServer` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:130 | 🟡 Type Declaration | 🔴 |
| struct | `PurgeRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:91 | 🟡 Type Declaration | 🔴 |
| struct | `RecallRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:53 | 🟡 Type Declaration | 🔴 |
| struct | `RestoreRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:121 | 🟡 Type Declaration | 🔴 |
| struct | `SearchRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:75 | 🟡 Type Declaration | 🔴 |
| struct | `StoreRequest` | hkask-mcp-memory | mcp-servers/hkask-mcp-memory/src/main.rs:45 | 🟡 Type Declaration | 🔴 |

| hkask-mcp-research | 106 | 21 | 85 | 19% | 23 |

### hkask-mcp-research

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| struct | `ResearchServer` | hkask-mcp-research | mcp-servers/hkask-mcp-research/src/main.rs:45 | 🟡 Type Declaration | 🔴 |
| fn | `cache_key` | hkask-mcp-research::cache | mcp-servers/hkask-mcp-research/src/cache.rs:95 | 🔴 MCP Tool Handler | 🔴 |
| fn | `new` | hkask-mcp-research::cache | mcp-servers/hkask-mcp-research/src/cache.rs:36 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CacheKey` | hkask-mcp-research::cache | mcp-servers/hkask-mcp-research/src/cache.rs:27 | 🟡 Type Declaration | 🔴 |
| struct | `ResponseCache` | hkask-mcp-research::cache | mcp-servers/hkask-mcp-research/src/cache.rs:29 | 🟡 Type Declaration | 🔴 |
| fn | `build_entry_query` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:202 | 🔴 MCP Tool Handler | 🔴 |
| fn | `count_entries` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:302 | 🔴 MCP Tool Handler | 🔴 |
| fn | `edit_tags` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:341 | 🔴 MCP Tool Handler | 🔴 |
| fn | `export_opml` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:478 | 🔴 MCP Tool Handler | 🔴 |
| fn | `import_opml` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:532 | 🔴 MCP Tool Handler | 🔴 |
| fn | `insert_entries` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:123 | 🔴 MCP Tool Handler | 🔴 |
| fn | `list_subscriptions` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:430 | 🔴 MCP Tool Handler | 🔴 |
| fn | `mark_stream_read` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:320 | 🔴 MCP Tool Handler | 🔴 |
| fn | `query_entries` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:277 | 🔴 MCP Tool Handler | 🔴 |
| fn | `resolve_feed_url` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:189 | 🔴 MCP Tool Handler | 🔴 |
| fn | `search_entries` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:414 | 🔴 MCP Tool Handler | 🔴 |
| fn | `update_feed_cache_headers` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:176 | 🔴 MCP Tool Handler | 🔴 |
| fn | `upsert_feed` | hkask-mcp-research::db | mcp-servers/hkask-mcp-research/src/db.rs:93 | 🔴 MCP Tool Handler | 🔴 |
| fn | `new` | hkask-mcp-research::providers::arxiv | mcp-servers/hkask-mcp-research/src/providers/arxiv.rs:16 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ArxivProvider` | hkask-mcp-research::providers::arxiv | mcp-servers/hkask-mcp-research/src/providers/arxiv.rs:11 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-research::providers::brave | mcp-servers/hkask-mcp-research/src/providers/brave.rs:12 | 🟢 Accessor/Constructor | 🟢 |
| struct | `BraveProvider` | hkask-mcp-research::providers::brave | mcp-servers/hkask-mcp-research/src/providers/brave.rs:6 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-research::providers::browserbase | mcp-servers/hkask-mcp-research/src/providers/browserbase.rs:14 | 🟢 Accessor/Constructor | 🟢 |
| struct | `BrowserbaseProvider` | hkask-mcp-research::providers::browserbase | mcp-servers/hkask-mcp-research/src/providers/browserbase.rs:8 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-research::providers::exa | mcp-servers/hkask-mcp-research/src/providers/exa.rs:14 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ExaProvider` | hkask-mcp-research::providers::exa | mcp-servers/hkask-mcp-research/src/providers/exa.rs:8 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-research::providers::firecrawl | mcp-servers/hkask-mcp-research/src/providers/firecrawl.rs:16 | 🟢 Accessor/Constructor | 🟢 |
| struct | `FirecrawlProvider` | hkask-mcp-research::providers::firecrawl | mcp-servers/hkask-mcp-research/src/providers/firecrawl.rs:10 | 🟡 Type Declaration | 🔴 |
| fn | `browse_provider_kinds` | hkask-mcp-research::providers::mod | mcp-servers/hkask-mcp-research/src/providers/mod.rs:431 | 🔴 MCP Tool Handler | 🔴 |
| fn | `extract_provider_kinds` | hkask-mcp-research::providers::mod | mcp-servers/hkask-mcp-research/src/providers/mod.rs:424 | 🔴 MCP Tool Handler | 🔴 |
| fn | `provider_fingerprint` | hkask-mcp-research::providers::mod | mcp-servers/hkask-mcp-research/src/providers/mod.rs:438 | 🔴 MCP Tool Handler | 🔴 |
| fn | `search_provider_kinds` | hkask-mcp-research::providers::mod | mcp-servers/hkask-mcp-research/src/providers/mod.rs:417 | 🔴 MCP Tool Handler | 🔴 |
| fn | `validate_provider_url` | hkask-mcp-research::providers::mod | mcp-servers/hkask-mcp-research/src/providers/mod.rs:64 | 🔴 MCP Tool Handler | 🔴 |
| struct | `ProviderPool` | hkask-mcp-research::providers::mod | mcp-servers/hkask-mcp-research/src/providers/mod.rs:130 | 🟡 Type Declaration | 🔴 |
| struct | `ProviderSearchOutput` | hkask-mcp-research::providers::mod | mcp-servers/hkask-mcp-research/src/providers/mod.rs:42 | 🟡 Type Declaration | 🔴 |
| trait | `WebSearchPort` | hkask-mcp-research::providers::mod | mcp-servers/hkask-mcp-research/src/providers/mod.rs:80 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-research::providers::raw_fetch | mcp-servers/hkask-mcp-research/src/providers/raw_fetch.rs:20 | 🟢 Accessor/Constructor | 🟢 |
| fn | `truncate_str` | hkask-mcp-research::providers::raw_fetch | mcp-servers/hkask-mcp-research/src/providers/raw_fetch.rs:132 | 🔴 MCP Tool Handler | 🔴 |
| struct | `RawFetchProvider` | hkask-mcp-research::providers::raw_fetch | mcp-servers/hkask-mcp-research/src/providers/raw_fetch.rs:9 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-research::providers::semantic_scholar | mcp-servers/hkask-mcp-research/src/providers/semantic_scholar.rs:16 | 🟢 Accessor/Constructor | 🟢 |
| struct | `SemanticScholarProvider` | hkask-mcp-research::providers::semantic_scholar | mcp-servers/hkask-mcp-research/src/providers/semantic_scholar.rs:11 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-research::providers::serapi | mcp-servers/hkask-mcp-research/src/providers/serapi.rs:17 | 🟢 Accessor/Constructor | 🟢 |
| struct | `SerapiProvider` | hkask-mcp-research::providers::serapi | mcp-servers/hkask-mcp-research/src/providers/serapi.rs:11 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-mcp-research::providers::tavily | mcp-servers/hkask-mcp-research/src/providers/tavily.rs:14 | 🟢 Accessor/Constructor | 🟢 |
| struct | `TavilyProvider` | hkask-mcp-research::providers::tavily | mcp-servers/hkask-mcp-research/src/providers/tavily.rs:8 | 🟡 Type Declaration | 🔴 |
| struct | `Continuation` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:84 | 🟡 Type Declaration | 🔴 |
| struct | `DiscoverRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:59 | 🟡 Type Declaration | 🔴 |
| struct | `EditTagRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:64 | 🟡 Type Declaration | 🔴 |
| struct | `FetchRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:24 | 🟡 Type Declaration | 🔴 |
| struct | `FetchResult` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:76 | 🟡 Type Declaration | 🔴 |
| struct | `GetEntriesRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:29 | 🟡 Type Declaration | 🔴 |
| struct | `ImportOpmlRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:54 | 🟡 Type Declaration | 🔴 |
| struct | `ListSubscriptionsRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:19 | 🟡 Type Declaration | 🔴 |
| struct | `MarkReadRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:38 | 🟡 Type Declaration | 🔴 |
| struct | `SearchRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:48 | 🟡 Type Declaration | 🔴 |
| struct | `SubscribeRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:7 | 🟡 Type Declaration | 🔴 |
| struct | `UnreadCountRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:43 | 🟡 Type Declaration | 🔴 |
| struct | `UnsubscribeRequest` | hkask-mcp-research::rss_types | mcp-servers/hkask-mcp-research/src/rss_types.rs:14 | 🟡 Type Declaration | 🔴 |
| fn | `strip_html` | hkask-mcp-research::strip_html | mcp-servers/hkask-mcp-research/src/strip_html.rs:3 | 🔴 MCP Tool Handler | 🟢 |
| enum | `Freshness` | hkask-mcp-research::types::freshness | mcp-servers/hkask-mcp-research/src/types/freshness.rs:12 | 🟡 Type Declaration | 🟢 |
| fn | `freshness_brave` | hkask-mcp-research::types::freshness | mcp-servers/hkask-mcp-research/src/types/freshness.rs:63 | 🔴 MCP Tool Handler | 🟢 |
| fn | `freshness_serpapi` | hkask-mcp-research::types::freshness | mcp-servers/hkask-mcp-research/src/types/freshness.rs:73 | 🔴 MCP Tool Handler | 🟢 |
| fn | `normalize_freshness` | hkask-mcp-research::types::freshness | mcp-servers/hkask-mcp-research/src/types/freshness.rs:53 | 🔴 MCP Tool Handler | 🟢 |
| enum | `ProviderFilter` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:256 | 🟡 Type Declaration | 🔴 |
| enum | `RerankSignal` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:300 | 🟡 Type Declaration | 🔴 |
| enum | `SearchCapability` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:176 | 🟡 Type Declaration | 🔴 |
| enum | `SearchDepth` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:125 | 🟡 Type Declaration | 🔴 |
| enum | `SearchStrategy` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:238 | 🟡 Type Declaration | 🔴 |
| enum | `WebError` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:142 | 🟡 Type Declaration | 🔴 |
| fn | `allows` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:434 | 🔴 MCP Tool Handler | 🟢 |
| fn | `kind` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:156 | 🔴 MCP Tool Handler | 🔴 |
| fn | `matches` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:263 | 🔴 MCP Tool Handler | 🔴 |
| fn | `provider_filter` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:246 | 🔴 MCP Tool Handler | 🔴 |
| struct | `AnswerBox` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:202 | 🟡 Type Declaration | 🔴 |
| struct | `BrowseOutput` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:398 | 🟡 Type Declaration | 🔴 |
| struct | `BrowseRequest` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:78 | 🟡 Type Declaration | 🔴 |
| struct | `BrowseResult` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:106 | 🟡 Type Declaration | 🔴 |
| struct | `CapabilityContext` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:427 | 🟡 Type Declaration | 🔴 |
| struct | `CompoundSearchResult` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:221 | 🟡 Type Declaration | 🔴 |
| struct | `ExtractOptions` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:131 | 🟡 Type Declaration | 🔴 |
| struct | `ExtractOutput` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:389 | 🟡 Type Declaration | 🔴 |
| struct | `ExtractRequest` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:68 | 🟡 Type Declaration | 🔴 |
| struct | `ExtractedContent` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:98 | 🟡 Type Declaration | 🔴 |
| struct | `FindSimilarOutput` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:382 | 🟡 Type Declaration | 🔴 |
| struct | `FindSimilarRequest` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:62 | 🟡 Type Declaration | 🔴 |
| struct | `FindSimilarResultOutput` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:371 | 🟡 Type Declaration | 🔴 |
| struct | `PingOutput` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:417 | 🟡 Type Declaration | 🔴 |
| struct | `ProviderError` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:215 | 🟡 Type Declaration | 🔴 |
| struct | `ProviderHealthEntry` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:409 | 🟡 Type Declaration | 🔴 |
| struct | `ProviderInfo` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:209 | 🟡 Type Declaration | 🔴 |
| struct | `RankedResult` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:185 | 🟡 Type Declaration | 🔴 |
| struct | `SearchMetadata` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:346 | 🟡 Type Declaration | 🔴 |
| struct | `SearchOutput` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:336 | 🟡 Type Declaration | 🔴 |
| struct | `SearchQuery` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:114 | 🟡 Type Declaration | 🔴 |
| struct | `SearchRequest` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:52 | 🟡 Type Declaration | 🔴 |
| struct | `SearchResultOutput` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:309 | 🟡 Type Declaration | 🔴 |
| struct | `SearchResult` | hkask-mcp-research::types::mod | mcp-servers/hkask-mcp-research/src/types/mod.rs:87 | 🟡 Type Declaration | 🔴 |
| fn | `apply_rerank` | hkask-mcp-research::types::ranking | mcp-servers/hkask-mcp-research/src/types/ranking.rs:12 | 🔴 MCP Tool Handler | 🟢 |
| fn | `dedup_results` | hkask-mcp-research::types::ranking | mcp-servers/hkask-mcp-research/src/types/ranking.rs:47 | 🔴 MCP Tool Handler | 🟢 |
| fn | `check` | hkask-mcp-research::types::rate_limiter | mcp-servers/hkask-mcp-research/src/types/rate_limiter.rs:36 | 🔴 MCP Tool Handler | 🟢 |
| fn | `new` | hkask-mcp-research::types::rate_limiter | mcp-servers/hkask-mcp-research/src/types/rate_limiter.rs:26 | 🟢 Accessor/Constructor | 🟢 |
| struct | `RateLimiter` | hkask-mcp-research::types::rate_limiter | mcp-servers/hkask-mcp-research/src/types/rate_limiter.rs:14 | 🟡 Type Declaration | 🟢 |
| fn | `sanitize_health_error` | hkask-mcp-research::types::validation | mcp-servers/hkask-mcp-research/src/types/validation.rs:17 | 🔴 MCP Tool Handler | 🔴 |
| fn | `validate_browse_request` | hkask-mcp-research::types::validation | mcp-servers/hkask-mcp-research/src/types/validation.rs:89 | 🔴 MCP Tool Handler | 🔴 |
| fn | `validate_extract_request` | hkask-mcp-research::types::validation | mcp-servers/hkask-mcp-research/src/types/validation.rs:61 | 🔴 MCP Tool Handler | 🔴 |
| fn | `validate_search_request` | hkask-mcp-research::types::validation | mcp-servers/hkask-mcp-research/src/types/validation.rs:47 | 🔴 MCP Tool Handler | 🔴 |

| hkask-mcp-spec | 22 | 0 | 22 | 0% | 10 |

### hkask-mcp-spec

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `new` | hkask-mcp-spec | mcp-servers/hkask-mcp-spec/src/main.rs:73 | 🟢 Accessor/Constructor | 🔴 |
| struct | `SpecServer` | hkask-mcp-spec | mcp-servers/hkask-mcp-spec/src/main.rs:44 | 🟡 Type Declaration | 🔴 |
| fn | `meets_publication_standard` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:61 | 🔴 MCP Tool Handler | 🔴 |
| fn | `passes` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:43 | 🔴 MCP Tool Handler | 🔴 |
| struct | `DependencyEdge` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:83 | 🟡 Type Declaration | 🔴 |
| struct | `DimensionScore` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:27 | 🟡 Type Declaration | 🔴 |
| struct | `GoalCaptureRequest` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:147 | 🟡 Type Declaration | 🔴 |
| struct | `GoalCaptureResponse` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:69 | 🟡 Type Declaration | 🔴 |
| struct | `GoalDecomposeRequest` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:157 | 🟡 Type Declaration | 🔴 |
| struct | `GoalDecomposeResponse` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:76 | 🟡 Type Declaration | 🔴 |
| struct | `GraphCoherenceRequest` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:202 | 🟡 Type Declaration | 🔴 |
| struct | `GraphCoherenceResponse` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:138 | 🟡 Type Declaration | 🔴 |
| struct | `GraphEdge` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:118 | 🟡 Type Declaration | 🔴 |
| struct | `GraphNode` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:111 | 🟡 Type Declaration | 🔴 |
| struct | `GraphPath` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:125 | 🟡 Type Declaration | 🔴 |
| struct | `GraphQueryRequest` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:192 | 🟡 Type Declaration | 🔴 |
| struct | `GraphQueryResponse` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:131 | 🟡 Type Declaration | 🔴 |
| struct | `ReplicaRewriteRequest` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:215 | 🟡 Type Declaration | 🔴 |
| struct | `ReplicaRewriteResponse` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:241 | 🟡 Type Declaration | 🔴 |
| struct | `WritingQualityRequest` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:170 | 🟡 Type Declaration | 🔴 |
| struct | `WritingQualityResponse` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:89 | 🟡 Type Declaration | 🔴 |
| struct | `WritingQualityScore` | hkask-mcp-spec::types | mcp-servers/hkask-mcp-spec/src/types.rs:14 | 🟡 Type Declaration | 🔴 |

| hkask-mcp-training | 69 | 44 | 25 | 63% | 14 |

### hkask-mcp-training

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `new` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:337 | 🟢 Accessor/Constructor | 🟢 |
| struct | `AssembleDatasetRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:125 | 🟡 Type Declaration | 🟢 |
| struct | `GenerateTracesRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:151 | 🟡 Type Declaration | 🟢 |
| struct | `IngestQaRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:85 | 🟡 Type Declaration | 🟢 |
| struct | `QaItem` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:77 | 🟡 Type Declaration | 🟢 |
| struct | `TrainCancelRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:113 | 🟡 Type Declaration | 🟢 |
| struct | `TrainCurateFeedbackRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:273 | 🟡 Type Declaration | 🟢 |
| struct | `TrainDeleteAdapterRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:119 | 🟡 Type Declaration | 🟢 |
| struct | `TrainEvaluateRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:175 | 🟡 Type Declaration | 🟢 |
| struct | `TrainIngestDatasetRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:312 | 🟡 Type Declaration | 🟢 |
| struct | `TrainRecommendModelRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:234 | 🟡 Type Declaration | 🟢 |
| struct | `TrainRecordInvocationRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:252 | 🟡 Type Declaration | 🟢 |
| struct | `TrainRegisterAdapterRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:197 | 🟡 Type Declaration | 🟢 |
| struct | `TrainRetrainRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:292 | 🟡 Type Declaration | 🟢 |
| struct | `TrainStatusRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:107 | 🟡 Type Declaration | 🟢 |
| struct | `TrainSubmitRequest` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:96 | 🟡 Type Declaration | 🟢 |
| struct | `TrainingServer` | hkask-mcp-training | mcp-servers/hkask-mcp-training/src/main.rs:322 | 🟡 Type Declaration | 🟢 |
| enum | `AdapterStoreError` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:123 | 🟡 Type Declaration | 🟢 |
| fn | `get` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:553 | 🔴 MCP Tool Handler | 🟢 |
| fn | `list_all` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:582 | 🔴 MCP Tool Handler | 🟢 |
| fn | `migrate` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:243 | 🔴 MCP Tool Handler | 🟢 |
| fn | `new` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:145 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:234 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:499 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:64 | 🟢 Accessor/Constructor | 🟢 |
| fn | `store` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:510 | 🔴 MCP Tool Handler | 🟢 |
| fn | `update_status` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:540 | 🔴 MCP Tool Handler | 🟢 |
| struct | `AdapterMetrics` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:51 | 🟡 Type Declaration | 🟢 |
| struct | `InMemoryAdapterStore` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:138 | 🟡 Type Declaration | 🟢 |
| struct | `JobStore` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:494 | 🟡 Type Declaration | 🟢 |
| struct | `LoRAAdapter` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:24 | 🟡 Type Declaration | 🟢 |
| struct | `SqliteAdapterStore` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:228 | 🟡 Type Declaration | 🟢 |
| struct | `StoredJob` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:481 | 🟡 Type Declaration | 🟢 |
| trait | `AdapterStore` | hkask-mcp-training::adapters | mcp-servers/hkask-mcp-training/src/adapters.rs:97 | 🟡 Type Declaration | 🟢 |
| enum | `DatasetError` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:80 | 🟡 Type Declaration | 🟢 |
| enum | `DatasetFormat` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:32 | 🟡 Type Declaration | 🟢 |
| fn | `detect` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:45 | 🔴 MCP Tool Handler | 🟢 |
| fn | `ingest` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:130 | 🔴 MCP Tool Handler | 🟢 |
| fn | `new` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:119 | 🟢 Accessor/Constructor | 🟢 |
| fn | `to_axolotl_format` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:391 | 🟢 Accessor/Constructor | 🟢 |
| fn | `to_unsloth_format` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:399 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ChatConversation` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:25 | 🟡 Type Declaration | 🟢 |
| struct | `ChatMessage` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:18 | 🟡 Type Declaration | 🟢 |
| struct | `DatasetPipeline` | hkask-mcp-training::dataset | mcp-servers/hkask-mcp-training/src/dataset.rs:101 | 🟡 Type Declaration | 🟢 |
| enum | `ProviderError` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:150 | 🟡 Type Declaration | 🔴 |
| enum | `TrainingHarnessId` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:31 | 🟡 Type Declaration | 🔴 |
| enum | `TrainingHostId` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:59 | 🟡 Type Declaration | 🔴 |
| enum | `TrainingJobStatus` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:139 | 🟡 Type Declaration | 🔴 |
| fn | `create_host` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:1779 | 🔴 MCP Tool Handler | 🔴 |
| fn | `from_config` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:1933 | 🟢 Accessor/Constructor | 🔴 |
| fn | `from_str` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:41 | 🟢 Accessor/Constructor | 🔴 |
| fn | `from_str` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:71 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:1115 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:1390 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:280 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:524 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:785 | 🟢 Accessor/Constructor | 🔴 |
| struct | `AxolotlProvider` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:265 | 🟡 Type Declaration | 🔴 |
| struct | `BasetenProvider` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:1380 | 🟡 Type Declaration | 🔴 |
| struct | `CompletionMetadata` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:244 | 🟡 Type Declaration | 🔴 |
| struct | `CostEstimate` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:2099 | 🟡 Type Declaration | 🔴 |
| struct | `RunpodProvider` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:1105 | 🟡 Type Declaration | 🔴 |
| struct | `TogetherProvider` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:778 | 🟡 Type Declaration | 🔴 |
| struct | `TrainingHostConfig` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:1874 | 🟡 Type Declaration | 🔴 |
| struct | `TrainingHostRouter` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:1923 | 🟡 Type Declaration | 🔴 |
| struct | `TrainingJob` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:85 | 🟡 Type Declaration | 🔴 |
| struct | `TrainingParams` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:104 | 🟡 Type Declaration | 🔴 |
| struct | `UnslothProvider` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:513 | 🟡 Type Declaration | 🔴 |
| trait | `TrainingHost` | hkask-mcp-training::providers | mcp-servers/hkask-mcp-training/src/providers.rs:176 | 🟡 Type Declaration | 🔴 |

| hkask-memory | 66 | 66 | 0 | 100% | 68 |

### hkask-memory

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `consolidate` | hkask-memory::consolidation | crates/hkask-memory/src/consolidation.rs:168 | 🔴 Core Logic | 🟢 |
| fn | `consolidation_candidate_count` | hkask-memory::consolidation | crates/hkask-memory/src/consolidation.rs:204 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-memory::consolidation | crates/hkask-memory/src/consolidation.rs:52 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ConsolidationBridge` | hkask-memory::consolidation | crates/hkask-memory/src/consolidation.rs:26 | 🟡 Type Declaration | 🟢 |
| fn | `consolidate` | hkask-memory::consolidation_service | crates/hkask-memory/src/consolidation_service.rs:67 | 🔴 Core Logic | 🟢 |
| fn | `consolidation_candidate_count` | hkask-memory::consolidation_service | crates/hkask-memory/src/consolidation_service.rs:212 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-memory::consolidation_service | crates/hkask-memory/src/consolidation_service.rs:39 | 🟢 Accessor/Constructor | 🟢 |
| fn | `semantic_low_confidence_count` | hkask-memory::consolidation_service | crates/hkask-memory/src/consolidation_service.rs:222 | 🔴 Core Logic | 🟢 |
| fn | `semantic_triple_count` | hkask-memory::consolidation_service | crates/hkask-memory/src/consolidation_service.rs:231 | 🔴 Core Logic | 🟢 |
| struct | `ConsolidationService` | hkask-memory::consolidation_service | crates/hkask-memory/src/consolidation_service.rs:24 | 🟡 Type Declaration | 🟢 |
| enum | `EpisodicMemoryError` | hkask-memory::episodic | crates/hkask-memory/src/episodic.rs:18 | 🟡 Type Declaration | 🟢 |
| fn | `consolidation_candidate_count` | hkask-memory::episodic | crates/hkask-memory/src/episodic.rs:238 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-memory::episodic | crates/hkask-memory/src/episodic.rs:60 | 🟢 Accessor/Constructor | 🟢 |
| fn | `query_for_deduped` | hkask-memory::episodic | crates/hkask-memory/src/episodic.rs:106 | 🔴 Core Logic | 🟢 |
| fn | `storage_budget` | hkask-memory::episodic | crates/hkask-memory/src/episodic.rs:223 | 🔴 Core Logic | 🟢 |
| fn | `storage_usage` | hkask-memory::episodic | crates/hkask-memory/src/episodic.rs:152 | 🔴 Core Logic | 🟢 |
| fn | `store` | hkask-memory::episodic | crates/hkask-memory/src/episodic.rs:78 | 🔴 Core Logic | 🟢 |
| struct | `EpisodicMemory` | hkask-memory::episodic | crates/hkask-memory/src/episodic.rs:46 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-memory::episodic_loop | crates/hkask-memory/src/episodic_loop.rs:45 | 🟢 Accessor/Constructor | 🟢 |
| fn | `storage_budget` | hkask-memory::episodic_loop | crates/hkask-memory/src/episodic_loop.rs:85 | 🔴 Core Logic | 🟢 |
| fn | `with_consolidation` | hkask-memory::episodic_loop | crates/hkask-memory/src/episodic_loop.rs:65 | 🟢 Accessor/Constructor | 🟢 |
| struct | `EpisodicLoop` | hkask-memory::episodic_loop | crates/hkask-memory/src/episodic_loop.rs:25 | 🟡 Type Declaration | 🟢 |
| fn | `normalize_date_bucket` | hkask-memory::ranking | crates/hkask-memory/src/ranking.rs:172 | 🔴 Core Logic | 🟢 |
| fn | `parse_age_to_days` | hkask-memory::ranking | crates/hkask-memory/src/ranking.rs:35 | 🔴 Core Logic | 🟢 |
| fn | `rrf_score` | hkask-memory::ranking | crates/hkask-memory/src/ranking.rs:18 | 🔴 Core Logic | 🟢 |
| fn | `dedup_triples` | hkask-memory::recall_dedup | crates/hkask-memory/src/recall_dedup.rs:67 | 🔴 Core Logic | 🟢 |
| fn | `eav_hash` | hkask-memory::recall_dedup | crates/hkask-memory/src/recall_dedup.rs:24 | 🔴 Core Logic | 🟢 |
| enum | `BudgetConfig` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:809 | 🟡 Type Declaration | 🟢 |
| fn | `all_tags` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:648 | 🔴 Core Logic | 🟢 |
| fn | `compute_method_signals` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:89 | 🔴 Core Logic | 🟢 |
| fn | `compute_salience_batch` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:707 | 🔴 Core Logic | 🟢 |
| fn | `matches` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:562 | 🔴 Core Logic | 🟢 |
| fn | `resolve` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:852 | 🔴 Core Logic | 🟢 |
| fn | `tag_count` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:662 | 🔴 Core Logic | 🟢 |
| fn | `tag_entities` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:618 | 🔴 Core Logic | 🟢 |
| struct | `DeclaredMethod` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:491 | 🟡 Type Declaration | 🟢 |
| struct | `EntityTags` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:601 | 🟡 Type Declaration | 🟢 |
| struct | `MethodSignals` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:22 | 🟡 Type Declaration | 🟢 |
| struct | `MethodThresholds` | hkask-memory::salience | crates/hkask-memory/src/salience.rs:509 | 🟡 Type Declaration | 🟢 |
| enum | `SemanticMemoryError` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:21 | 🟡 Type Declaration | 🟢 |
| fn | `chunk_text` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:348 | 🔴 Core Logic | 🟢 |
| fn | `compute_centroid` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:229 | 🔴 Core Logic | 🟢 |
| fn | `delete_triple` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:496 | 🔴 Core Logic | 🟢 |
| fn | `embedding_count` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:188 | 🔴 Core Logic | 🟢 |
| fn | `embedding_store` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:197 | 🔴 Core Logic | 🟢 |
| fn | `low_confidence_count` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:531 | 🔴 Core Logic | 🟢 |
| fn | `low_confidence_triples` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:548 | 🔴 Core Logic | 🟢 |
| fn | `lowest_confidence_triples` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:516 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:68 | 🟢 Accessor/Constructor | 🟢 |
| fn | `purge_by_prefix` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:313 | 🔴 Core Logic | 🟢 |
| fn | `query_by_attribute` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:142 | 🔴 Core Logic | 🟢 |
| fn | `query_deduped` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:84 | 🔴 Core Logic | 🟢 |
| fn | `search_similar` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:176 | 🔴 Core Logic | 🟢 |
| fn | `store_embedding` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:157 | 🔴 Core Logic | 🟢 |
| fn | `store` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:101 | 🔴 Core Logic | 🟢 |
| fn | `strip_gutenberg_headers` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:458 | 🔴 Core Logic | 🟢 |
| fn | `triple_count_for_entity` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:133 | 🔴 Core Logic | 🟢 |
| fn | `triple_count` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:124 | 🔴 Core Logic | 🟢 |
| struct | `CentroidResult` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:38 | 🟡 Type Declaration | 🟢 |
| struct | `SemanticMemory` | hkask-memory::semantic | crates/hkask-memory/src/semantic.rs:57 | 🟡 Type Declaration | 🟢 |
| fn | `low_confidence_threshold` | hkask-memory::semantic_loop | crates/hkask-memory/src/semantic_loop.rs:108 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-memory::semantic_loop | crates/hkask-memory/src/semantic_loop.rs:52 | 🟢 Accessor/Constructor | 🟢 |
| fn | `storage_budget` | hkask-memory::semantic_loop | crates/hkask-memory/src/semantic_loop.rs:100 | 🔴 Core Logic | 🟢 |
| fn | `with_budget_and_threshold` | hkask-memory::semantic_loop | crates/hkask-memory/src/semantic_loop.rs:84 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_budget` | hkask-memory::semantic_loop | crates/hkask-memory/src/semantic_loop.rs:67 | 🟢 Accessor/Constructor | 🟢 |
| struct | `SemanticLoop` | hkask-memory::semantic_loop | crates/hkask-memory/src/semantic_loop.rs:37 | 🟡 Type Declaration | 🟢 |

| hkask-services | 288 | 268 | 20 | 93% | 319 |

### hkask-services

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| struct | `ArchivalService` | hkask-services::archival | crates/hkask-services/src/archival.rs:35 | 🟡 Type Declaration | 🟢 |
| struct | `ArchiveResult` | hkask-services::archival | crates/hkask-services/src/archival.rs:16 | 🟡 Type Declaration | 🟢 |
| struct | `SnapshotResult` | hkask-services::archival | crates/hkask-services/src/archival.rs:25 | 🟡 Type Declaration | 🟢 |
| fn | `backup_config_path` | hkask-services::backup::config | crates/hkask-services/src/backup/config.rs:175 | 🔴 Core Logic | 🟢 |
| fn | `from_duration_str` | hkask-services::backup::config | crates/hkask-services/src/backup/config.rs:140 | 🟢 Accessor/Constructor | 🟢 |
| fn | `load_backup_config` | hkask-services::backup::config | crates/hkask-services/src/backup/config.rs:186 | 🔴 Core Logic | 🟢 |
| fn | `save_backup_config` | hkask-services::backup::config | crates/hkask-services/src/backup/config.rs:199 | 🔴 Core Logic | 🟢 |
| fn | `should_keep` | hkask-services::backup::config | crates/hkask-services/src/backup/config.rs:109 | 🔴 Core Logic | 🟢 |
| struct | `BackupConfig` | hkask-services::backup::config | crates/hkask-services/src/backup/config.rs:13 | 🟡 Type Declaration | 🟢 |
| struct | `EncryptionConfig` | hkask-services::backup::config | crates/hkask-services/src/backup/config.rs:38 | 🟡 Type Declaration | 🟢 |
| struct | `RetentionPolicy` | hkask-services::backup::config | crates/hkask-services/src/backup/config.rs:73 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-services::backup::loop | crates/hkask-services/src/backup/loop.rs:50 | 🟢 Accessor/Constructor | 🟢 |
| struct | `BackupLoop` | hkask-services::backup::loop | crates/hkask-services/src/backup/loop.rs:39 | 🟡 Type Declaration | 🟢 |
| enum | `SnapshotTrigger` | hkask-services::backup::metadata | crates/hkask-services/src/backup/metadata.rs:11 | 🟡 Type Declaration | 🔴 |
| struct | `PruneReport` | hkask-services::backup::metadata | crates/hkask-services/src/backup/metadata.rs:41 | 🟡 Type Declaration | 🔴 |
| struct | `SnapshotMetadata` | hkask-services::backup::metadata | crates/hkask-services/src/backup/metadata.rs:25 | 🟡 Type Declaration | 🔴 |
| enum | `BackupError` | hkask-services::backup::mod | crates/hkask-services/src/backup/mod.rs:44 | 🟡 Type Declaration | 🟢 |
| fn | `config` | hkask-services::backup::mod | crates/hkask-services/src/backup/mod.rs:546 | 🔴 Core Logic | 🟢 |
| fn | `enable_encryption` | hkask-services::backup::mod | crates/hkask-services/src/backup/mod.rs:569 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-services::backup::mod | crates/hkask-services/src/backup/mod.rs:108 | 🟢 Accessor/Constructor | 🟢 |
| fn | `update_config` | hkask-services::backup::mod | crates/hkask-services/src/backup/mod.rs:555 | 🔴 Core Logic | 🟢 |
| fn | `with_config` | hkask-services::backup::mod | crates/hkask-services/src/backup/mod.rs:123 | 🟢 Accessor/Constructor | 🟢 |
| struct | `BackupService` | hkask-services::backup::mod | crates/hkask-services/src/backup/mod.rs:85 | 🟡 Type Declaration | 🟢 |
| enum | `ArtifactType` | hkask-services::backup::scope | crates/hkask-services/src/backup/scope.rs:20 | 🟡 Type Declaration | 🟢 |
| enum | `BackupScope` | hkask-services::backup::scope | crates/hkask-services/src/backup/scope.rs:89 | 🟡 Type Declaration | 🟢 |
| enum | `RestoreScope` | hkask-services::backup::scope | crates/hkask-services/src/backup/scope.rs:121 | 🟡 Type Declaration | 🟢 |
| fn | `description` | hkask-services::backup::scope | crates/hkask-services/src/backup/scope.rs:107 | 🔴 Core Logic | 🟢 |
| fn | `label` | hkask-services::backup::scope | crates/hkask-services/src/backup/scope.rs:62 | 🔴 Core Logic | 🟢 |
| fn | `repo_id` | hkask-services::backup::scope | crates/hkask-services/src/backup/scope.rs:41 | 🔴 Core Logic | 🟢 |
| struct | `ListFilter` | hkask-services::backup::scope | crates/hkask-services/src/backup/scope.rs:135 | 🟡 Type Declaration | 🟢 |
| fn | `artifact_git_path` | hkask-services::backup::serialization | crates/hkask-services/src/backup/serialization.rs:81 | 🔴 Core Logic | 🟢 |
| fn | `deserialize_artifact` | hkask-services::backup::serialization | crates/hkask-services/src/backup/serialization.rs:42 | 🔴 Core Logic | 🟢 |
| fn | `serialize_artifact` | hkask-services::backup::serialization | crates/hkask-services/src/backup/serialization.rs:21 | 🔴 Core Logic | 🟢 |
| struct | `ArtifactEnvelopeValue` | hkask-services::backup::serialization | crates/hkask-services/src/backup/serialization.rs:63 | 🟡 Type Declaration | 🟢 |
| fn | `deactivate` | hkask-services::bundle | crates/hkask-services/src/bundle.rs:312 | 🔴 Core Logic | 🟢 |
| struct | `BundleComposeResult` | hkask-services::bundle | crates/hkask-services/src/bundle.rs:33 | 🟡 Type Declaration | 🟢 |
| struct | `BundleService` | hkask-services::bundle | crates/hkask-services/src/bundle.rs:41 | 🟡 Type Declaration | 🟢 |
| enum | `MessageSource` | hkask-services::chat | crates/hkask-services/src/chat.rs:861 | 🟡 Type Declaration | 🟢 |
| fn | `apply_persona_filter` | hkask-services::chat | crates/hkask-services/src/chat.rs:584 | 🔴 Core Logic | 🟢 |
| fn | `gas_cost` | hkask-services::chat | crates/hkask-services/src/chat.rs:48 | 🔴 Core Logic | 🟢 |
| fn | `recall_raw_episodes` | hkask-services::chat | crates/hkask-services/src/chat.rs:481 | 🔴 Core Logic | 🟢 |
| fn | `recall_recent_turns` | hkask-services::chat | crates/hkask-services/src/chat.rs:439 | 🔴 Core Logic | 🟢 |
| fn | `recall_semantic` | hkask-services::chat | crates/hkask-services/src/chat.rs:363 | 🔴 Core Logic | 🟢 |
| fn | `store_episodic` | hkask-services::chat | crates/hkask-services/src/chat.rs:391 | 🔴 Core Logic | 🟢 |
| fn | `wrap_manifest_input` | hkask-services::chat | crates/hkask-services/src/chat.rs:572 | 🔴 Core Logic | 🟢 |
| struct | `ChatRequest` | hkask-services::chat | crates/hkask-services/src/chat.rs:76 | 🟡 Type Declaration | 🟢 |
| struct | `ChatResponse` | hkask-services::chat | crates/hkask-services/src/chat.rs:61 | 🟡 Type Declaration | 🟢 |
| struct | `ChatService` | hkask-services::chat | crates/hkask-services/src/chat.rs:125 | 🟡 Type Declaration | 🟢 |
| struct | `PreparedChat` | hkask-services::chat | crates/hkask-services/src/chat.rs:107 | 🟡 Type Declaration | 🟢 |
| struct | `TokenUsage` | hkask-services::chat | crates/hkask-services/src/chat.rs:36 | 🟡 Type Declaration | 🟢 |
| struct | `TurnRequest` | hkask-services::chat | crates/hkask-services/src/chat.rs:804 | 🟡 Type Declaration | 🟢 |
| struct | `TurnResult` | hkask-services::chat | crates/hkask-services/src/chat.rs:881 | 🟡 Type Declaration | 🟢 |
| fn | `from_def` | hkask-services::classify | crates/hkask-services/src/classify.rs:161 | 🟢 Accessor/Constructor | 🟢 |
| fn | `load_classifier_config` | hkask-services::classify | crates/hkask-services/src/classify.rs:117 | 🔴 Core Logic | 🟢 |
| struct | `ClassifierConfig` | hkask-services::classify | crates/hkask-services/src/classify.rs:147 | 🟡 Type Declaration | 🟢 |
| struct | `ClassifierDef` | hkask-services::classify | crates/hkask-services/src/classify.rs:67 | 🟡 Type Declaration | 🟢 |
| struct | `ClassifierYaml` | hkask-services::classify | crates/hkask-services/src/classify.rs:62 | 🟡 Type Declaration | 🟢 |
| struct | `ClassifyResult` | hkask-services::classify | crates/hkask-services/src/classify.rs:17 | 🟡 Type Declaration | 🟢 |
| struct | `TripleExtraction` | hkask-services::classify | crates/hkask-services/src/classify.rs:25 | 🟡 Type Declaration | 🟢 |
| fn | `get_set_points` | hkask-services::cns | crates/hkask-services/src/cns.rs:69 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-services::cns | crates/hkask-services/src/cns.rs:30 | 🟢 Accessor/Constructor | 🟢 |
| fn | `update_set_points` | hkask-services::cns | crates/hkask-services/src/cns.rs:81 | 🔴 Core Logic | 🟢 |
| struct | `CnsService` | hkask-services::cns | crates/hkask-services/src/cns.rs:20 | 🟡 Type Declaration | 🟢 |
| fn | `cosine_distance` | hkask-services::compose | crates/hkask-services/src/compose.rs:453 | 🔴 Core Logic | 🟢 |
| struct | `CentroidValidation` | hkask-services::compose | crates/hkask-services/src/compose.rs:140 | 🟡 Type Declaration | 🟢 |
| struct | `CognitionConfig` | hkask-services::compose | crates/hkask-services/src/compose.rs:38 | 🟡 Type Declaration | 🟢 |
| struct | `ComposeRequest` | hkask-services::compose | crates/hkask-services/src/compose.rs:114 | 🟡 Type Declaration | 🟢 |
| struct | `ComposeResult` | hkask-services::compose | crates/hkask-services/src/compose.rs:130 | 🟡 Type Declaration | 🟢 |
| struct | `ComposeService` | hkask-services::compose | crates/hkask-services/src/compose.rs:152 | 🟡 Type Declaration | 🟢 |
| struct | `EmbeddingSection` | hkask-services::compose | crates/hkask-services/src/compose.rs:60 | 🟡 Type Declaration | 🟢 |
| struct | `RetrievalSection` | hkask-services::compose | crates/hkask-services/src/compose.rs:69 | 🟡 Type Declaration | 🟢 |
| struct | `ValidationSection` | hkask-services::compose | crates/hkask-services/src/compose.rs:107 | 🟡 Type Declaration | 🟢 |
| fn | `effective_memory_db_path` | hkask-services::config | crates/hkask-services/src/config.rs:261 | 🔴 Core Logic | 🟢 |
| fn | `from_env` | hkask-services::config | crates/hkask-services/src/config.rs:122 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_secrets` | hkask-services::config | crates/hkask-services/src/config.rs:185 | 🟢 Accessor/Constructor | 🟢 |
| fn | `in_memory` | hkask-services::config | crates/hkask-services/src/config.rs:228 | 🔴 Core Logic | 🟢 |
| struct | `ServiceConfig` | hkask-services::config | crates/hkask-services/src/config.rs:37 | 🟡 Type Declaration | 🟢 |
| fn | `check_rate_limit` | hkask-services::consolidation | crates/hkask-services/src/consolidation.rs:32 | 🔴 Core Logic | 🟢 |
| fn | `consolidate` | hkask-services::consolidation | crates/hkask-services/src/consolidation.rs:77 | 🔴 Core Logic | 🟢 |
| fn | `db_path_for_agent` | hkask-services::consolidation | crates/hkask-services/src/consolidation.rs:52 | 🔴 Core Logic | 🟢 |
| fn | `verify_passphrase` | hkask-services::consolidation | crates/hkask-services/src/consolidation.rs:58 | 🔴 Core Logic | 🟢 |
| fn | `add` | hkask-services::contacts | crates/hkask-services/src/contacts.rs:18 | 🔴 Core Logic | 🟢 |
| fn | `find` | hkask-services::contacts | crates/hkask-services/src/contacts.rs:41 | 🔴 Core Logic | 🟢 |
| fn | `list` | hkask-services::contacts | crates/hkask-services/src/contacts.rs:56 | 🔴 Core Logic | 🟢 |
| struct | `ContactService` | hkask-services::contacts | crates/hkask-services/src/contacts.rs:10 | 🟡 Type Declaration | 🟢 |
| fn | `agent_registry_store` | hkask-services::context | crates/hkask-services/src/context.rs:422 | 🔴 Core Logic | 🟢 |
| fn | `build_per_agent_memory` | hkask-services::context | crates/hkask-services/src/context.rs:471 | 🔴 Core Logic | 🟢 |
| fn | `capability_checker` | hkask-services::context | crates/hkask-services/src/context.rs:311 | 🔴 Core Logic | 🟢 |
| fn | `cns_runtime` | hkask-services::context | crates/hkask-services/src/context.rs:266 | 🔴 Core Logic | 🟢 |
| fn | `config` | hkask-services::context | crates/hkask-services/src/context.rs:209 | 🔴 Core Logic | 🟢 |
| fn | `curation_inbox_tx` | hkask-services::context | crates/hkask-services/src/context.rs:390 | 🔴 Core Logic | 🟢 |
| fn | `cybernetics_loop` | hkask-services::context | crates/hkask-services/src/context.rs:274 | 🔴 Core Logic | 🟢 |
| fn | `daemon_handler` | hkask-services::context | crates/hkask-services/src/context.rs:441 | 🔴 Core Logic | 🟢 |
| fn | `escalation_queue` | hkask-services::context | crates/hkask-services/src/context.rs:327 | 🔴 Core Logic | 🟢 |
| fn | `event_sink` | hkask-services::context | crates/hkask-services/src/context.rs:290 | 🔴 Core Logic | 🟢 |
| fn | `goal_repo` | hkask-services::context | crates/hkask-services/src/context.rs:256 | 🔴 Core Logic | 🟢 |
| fn | `identity` | hkask-services::context | crates/hkask-services/src/context.rs:363 | 🔴 Core Logic | 🟢 |
| fn | `inference_port` | hkask-services::context | crates/hkask-services/src/context.rs:337 | 🔴 Core Logic | 🟢 |
| fn | `loop_system` | hkask-services::context | crates/hkask-services/src/context.rs:282 | 🔴 Core Logic | 🟢 |
| fn | `matrix_transport` | hkask-services::context | crates/hkask-services/src/context.rs:453 | 🔴 Core Logic | 🟢 |
| fn | `mcp_dispatcher` | hkask-services::context | crates/hkask-services/src/context.rs:319 | 🔴 Core Logic | 🟢 |
| fn | `mcp_runtime` | hkask-services::context | crates/hkask-services/src/context.rs:345 | 🔴 Core Logic | 🟢 |
| fn | `memory` | hkask-services::context | crates/hkask-services/src/context.rs:238 | 🔴 Core Logic | 🟢 |
| fn | `open_agent_registry` | hkask-services::context | crates/hkask-services/src/context.rs:557 | 🔴 Core Logic | 🟢 |
| fn | `open_consent_manager` | hkask-services::context | crates/hkask-services/src/context.rs:535 | 🔴 Core Logic | 🟢 |
| fn | `open_escalation_queue` | hkask-services::context | crates/hkask-services/src/context.rs:513 | 🔴 Core Logic | 🟢 |
| fn | `open_spec_store` | hkask-services::context | crates/hkask-services/src/context.rs:523 | 🔴 Core Logic | 🟢 |
| fn | `pod_manager` | hkask-services::context | crates/hkask-services/src/context.rs:353 | 🔴 Core Logic | 🟢 |
| fn | `registry` | hkask-services::context | crates/hkask-services/src/context.rs:248 | 🔴 Core Logic | 🟢 |
| fn | `seam_watcher` | hkask-services::context | crates/hkask-services/src/context.rs:300 | 🔴 Core Logic | 🟢 |
| fn | `sovereignty_boundary_store` | hkask-services::context | crates/hkask-services/src/context.rs:400 | 🔴 Core Logic | 🟢 |
| fn | `sovereignty` | hkask-services::context | crates/hkask-services/src/context.rs:374 | 🔴 Core Logic | 🟢 |
| fn | `spec_store` | hkask-services::context | crates/hkask-services/src/context.rs:412 | 🔴 Core Logic | 🟢 |
| fn | `user_store` | hkask-services::context | crates/hkask-services/src/context.rs:432 | 🔴 Core Logic | 🟢 |
| fn | `wallet_store` | hkask-services::context | crates/hkask-services/src/context.rs:227 | 🔴 Core Logic | 🟢 |
| fn | `wallet` | hkask-services::context | crates/hkask-services/src/context.rs:218 | 🔴 Core Logic | 🟢 |
| struct | `AgentService` | hkask-services::context | crates/hkask-services/src/context.rs:93 | 🟡 Type Declaration | 🟢 |
| struct | `PerAgentMemory` | hkask-services::context | crates/hkask-services/src/context.rs:195 | 🟡 Type Declaration | 🟢 |
| fn | `dismiss` | hkask-services::curator | crates/hkask-services/src/curator.rs:116 | 🔴 Core Logic | 🟢 |
| fn | `list_escalations` | hkask-services::curator | crates/hkask-services/src/curator.rs:65 | 🔴 Core Logic | 🟢 |
| fn | `resolve` | hkask-services::curator | crates/hkask-services/src/curator.rs:79 | 🔴 Core Logic | 🟢 |
| struct | `CuratorService` | hkask-services::curator | crates/hkask-services/src/curator.rs:55 | 🟡 Type Declaration | 🟢 |
| struct | `EscalationResponse` | hkask-services::curator | crates/hkask-services/src/curator.rs:22 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-services::daemon_handler | crates/hkask-services/src/daemon_handler.rs:59 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ServiceDaemonHandler` | hkask-services::daemon_handler | crates/hkask-services/src/daemon_handler.rs:46 | 🟡 Type Declaration | 🟢 |
| fn | `default_corpus_config` | hkask-services::discover | crates/hkask-services/src/discover.rs:527 | 🔴 Core Logic | 🟢 |
| fn | `generate_corpus_yaml` | hkask-services::discover | crates/hkask-services/src/discover.rs:457 | 🔴 Core Logic | 🟢 |
| fn | `slugify` | hkask-services::discover | crates/hkask-services/src/discover.rs:1429 | 🔴 Core Logic | 🟢 |
| struct | `DiscoverRequest` | hkask-services::discover | crates/hkask-services/src/discover.rs:36 | 🟡 Type Declaration | 🟢 |
| struct | `DiscoverResult` | hkask-services::discover | crates/hkask-services/src/discover.rs:91 | 🟡 Type Declaration | 🟢 |
| struct | `DiscoveredWork` | hkask-services::discover | crates/hkask-services/src/discover.rs:118 | 🟡 Type Declaration | 🟢 |
| struct | `DiscoveryService` | hkask-services::discover | crates/hkask-services/src/discover.rs:133 | 🟡 Type Declaration | 🟢 |
| enum | `EmbedPhase` | hkask-services::embed | crates/hkask-services/src/embed.rs:53 | 🟡 Type Declaration | 🔴 |
| fn | `format_full` | hkask-services::embed | crates/hkask-services/src/embed.rs:84 | 🔴 Core Logic | 🔴 |
| fn | `format_page_progress` | hkask-services::embed | crates/hkask-services/src/embed.rs:63 | 🔴 Core Logic | 🔴 |
| fn | `parse_config` | hkask-services::embed | crates/hkask-services/src/embed.rs:1148 | 🔴 Core Logic | 🔴 |
| fn | `strip_html_tags` | hkask-services::embed | crates/hkask-services/src/embed.rs:1431 | 🔴 Core Logic | 🔴 |
| struct | `ChunkingConfig` | hkask-services::embed | crates/hkask-services/src/embed.rs:287 | 🟡 Type Declaration | 🔴 |
| struct | `CorpusConfig` | hkask-services::embed | crates/hkask-services/src/embed.rs:111 | 🟡 Type Declaration | 🔴 |
| struct | `DimensionCentroidResult` | hkask-services::embed | crates/hkask-services/src/embed.rs:400 | 🟡 Type Declaration | 🔴 |
| struct | `DimensionCentroid` | hkask-services::embed | crates/hkask-services/src/embed.rs:303 | 🟡 Type Declaration | 🔴 |
| struct | `EmbedProgress` | hkask-services::embed | crates/hkask-services/src/embed.rs:43 | 🟡 Type Declaration | 🔴 |
| struct | `EmbedResult` | hkask-services::embed | crates/hkask-services/src/embed.rs:407 | 🟡 Type Declaration | 🔴 |
| struct | `EmbedService` | hkask-services::embed | crates/hkask-services/src/embed.rs:431 | 🟡 Type Declaration | 🟢 |
| struct | `EmbeddingConfig` | hkask-services::embed | crates/hkask-services/src/embed.rs:236 | 🟡 Type Declaration | 🔴 |
| struct | `EntityConfig` | hkask-services::embed | crates/hkask-services/src/embed.rs:182 | 🟡 Type Declaration | 🔴 |
| struct | `Entity` | hkask-services::embed | crates/hkask-services/src/embed.rs:213 | 🟡 Type Declaration | 🟢 |
| struct | `FoundationalRule` | hkask-services::embed | crates/hkask-services/src/embed.rs:274 | 🟡 Type Declaration | 🔴 |
| struct | `TagSet` | hkask-services::embed | crates/hkask-services/src/embed.rs:317 | 🟡 Type Declaration | 🔴 |
| struct | `ValidationConfig` | hkask-services::embed | crates/hkask-services/src/embed.rs:295 | 🟡 Type Declaration | 🔴 |
| struct | `Work` | hkask-services::embed | crates/hkask-services/src/embed.rs:244 | 🟡 Type Declaration | 🟢 |
| type | `ProgressFn` | hkask-services::embed | crates/hkask-services/src/embed.rs:39 | 🟡 Type Declaration | 🔴 |
| enum | `ServiceError` | hkask-services::error | crates/hkask-services/src/error.rs:59 | 🟡 Type Declaration | 🟢 |
| fn | `is_retryable` | hkask-services::error | crates/hkask-services/src/error.rs:443 | 🟢 Accessor/Constructor | 🟢 |
| fn | `message_key` | hkask-services::error | crates/hkask-services/src/error.rs:541 | 🔴 Core Logic | 🟢 |
| fn | `nu_event` | hkask-services::error | crates/hkask-services/src/error.rs:640 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-services::experience | crates/hkask-services/src/experience.rs:41 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CliExperienceRecorder` | hkask-services::experience | crates/hkask-services/src/experience.rs:29 | 🟡 Type Declaration | 🟢 |
| fn | `create_goal` | hkask-services::goal | crates/hkask-services/src/goal.rs:51 | 🔴 Core Logic | 🟢 |
| fn | `list_goals` | hkask-services::goal | crates/hkask-services/src/goal.rs:76 | 🔴 Core Logic | 🟢 |
| fn | `set_goal_state` | hkask-services::goal | crates/hkask-services/src/goal.rs:104 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CreateGoalRequest` | hkask-services::goal | crates/hkask-services/src/goal.rs:17 | 🟡 Type Declaration | 🟢 |
| struct | `GoalResponse` | hkask-services::goal | crates/hkask-services/src/goal.rs:24 | 🟡 Type Declaration | 🟢 |
| struct | `GoalService` | hkask-services::goal | crates/hkask-services/src/goal.rs:43 | 🟡 Type Declaration | 🟢 |
| fn | `from_parts` | hkask-services::inference | crates/hkask-services/src/inference.rs:47 | 🟢 Accessor/Constructor | 🟢 |
| fn | `resolve_port` | hkask-services::inference | crates/hkask-services/src/inference.rs:116 | 🔴 Core Logic | 🟢 |
| struct | `InferenceContext` | hkask-services::inference | crates/hkask-services/src/inference.rs:29 | 🟡 Type Declaration | 🟢 |
| struct | `InferenceService` | hkask-services::inference | crates/hkask-services/src/inference.rs:101 | 🟡 Type Declaration | 🟢 |
| struct | `ModelInfo` | hkask-services::inference | crates/hkask-services/src/inference.rs:72 | 🟡 Type Declaration | 🟢 |
| enum | `ImprovementDirection` | hkask-services::kata | crates/hkask-services/src/kata.rs:478 | 🟡 Type Declaration | 🟢 |
| enum | `KataError` | hkask-services::kata | crates/hkask-services/src/kata.rs:1693 | 🟡 Type Declaration | 🟢 |
| fn | `can_graduate_from_starter` | hkask-services::kata | crates/hkask-services/src/kata.rs:393 | 🔴 Core Logic | 🟢 |
| fn | `compute_automaticity` | hkask-services::kata | crates/hkask-services/src/kata.rs:357 | 🔴 Core Logic | 🟢 |
| fn | `current_streak` | hkask-services::kata | crates/hkask-services/src/kata.rs:320 | 🔴 Core Logic | 🟢 |
| fn | `days_since_last` | hkask-services::kata | crates/hkask-services/src/kata.rs:376 | 🔴 Core Logic | 🟢 |
| fn | `from_env` | hkask-services::kata | crates/hkask-services/src/kata.rs:646 | 🟢 Accessor/Constructor | 🟢 |
| fn | `load_manifest` | hkask-services::kata | crates/hkask-services/src/kata.rs:768 | 🔴 Core Logic | 🟢 |
| fn | `load` | hkask-services::kata | crates/hkask-services/src/kata.rs:270 | 🔴 Core Logic | 🟢 |
| fn | `load` | hkask-services::kata | crates/hkask-services/src/kata.rs:553 | 🔴 Core Logic | 🟢 |
| fn | `needs_habit_intervention` | hkask-services::kata | crates/hkask-services/src/kata.rs:402 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-services::kata | crates/hkask-services/src/kata.rs:624 | 🟢 Accessor/Constructor | 🟢 |
| fn | `record_history_entry` | hkask-services::kata | crates/hkask-services/src/kata.rs:737 | 🔴 Core Logic | 🟢 |
| fn | `record` | hkask-services::kata | crates/hkask-services/src/kata.rs:308 | 🔴 Core Logic | 🟢 |
| fn | `save` | hkask-services::kata | crates/hkask-services/src/kata.rs:290 | 🔴 Core Logic | 🟢 |
| fn | `save` | hkask-services::kata | crates/hkask-services/src/kata.rs:535 | 🔴 Core Logic | 🟢 |
| fn | `with_cns_runtime` | hkask-services::kata | crates/hkask-services/src/kata.rs:723 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_cns` | hkask-services::kata | crates/hkask-services/src/kata.rs:670 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_consent` | hkask-services::kata | crates/hkask-services/src/kata.rs:657 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_history_store` | hkask-services::kata | crates/hkask-services/src/kata.rs:697 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_history` | hkask-services::kata | crates/hkask-services/src/kata.rs:683 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_metrics` | hkask-services::kata | crates/hkask-services/src/kata.rs:707 | 🟢 Accessor/Constructor | 🟢 |
| struct | `AuditConfig` | hkask-services::kata | crates/hkask-services/src/kata.rs:215 | 🟡 Type Declaration | 🟢 |
| struct | `CnsConfig` | hkask-services::kata | crates/hkask-services/src/kata.rs:176 | 🟡 Type Declaration | 🟢 |
| struct | `CoachQuestion` | hkask-services::kata | crates/hkask-services/src/kata.rs:125 | 🟡 Type Declaration | 🟢 |
| struct | `ErrorHandling` | hkask-services::kata | crates/hkask-services/src/kata.rs:153 | 🟡 Type Declaration | 🟢 |
| struct | `GasConfig` | hkask-services::kata | crates/hkask-services/src/kata.rs:71 | 🟡 Type Declaration | 🟢 |
| struct | `ImprovementSignal` | hkask-services::kata | crates/hkask-services/src/kata.rs:466 | 🟡 Type Declaration | 🟢 |
| struct | `KataEngine` | hkask-services::kata | crates/hkask-services/src/kata.rs:600 | 🟡 Type Declaration | 🟢 |
| struct | `KataHistory` | hkask-services::kata | crates/hkask-services/src/kata.rs:249 | 🟡 Type Declaration | 🟢 |
| struct | `KataManifest` | hkask-services::kata | crates/hkask-services/src/kata.rs:35 | 🟡 Type Declaration | 🟢 |
| struct | `KataResult` | hkask-services::kata | crates/hkask-services/src/kata.rs:568 | 🟡 Type Declaration | 🟢 |
| struct | `KataState` | hkask-services::kata | crates/hkask-services/src/kata.rs:501 | 🟡 Type Declaration | 🟢 |
| struct | `KataStep` | hkask-services::kata | crates/hkask-services/src/kata.rs:98 | 🟡 Type Declaration | 🟢 |
| struct | `ManifestMeta` | hkask-services::kata | crates/hkask-services/src/kata.rs:58 | 🟡 Type Declaration | 🟢 |
| struct | `MetricDef` | hkask-services::kata | crates/hkask-services/src/kata.rs:200 | 🟡 Type Declaration | 🟢 |
| struct | `Outcome` | hkask-services::kata | crates/hkask-services/src/kata.rs:193 | 🟡 Type Declaration | 🟢 |
| struct | `PracticeEntry` | hkask-services::kata | crates/hkask-services/src/kata.rs:256 | 🟡 Type Declaration | 🟢 |
| struct | `PracticeRoutine` | hkask-services::kata | crates/hkask-services/src/kata.rs:137 | 🟡 Type Declaration | 🟢 |
| struct | `StarterOutcome` | hkask-services::kata | crates/hkask-services/src/kata.rs:208 | 🟡 Type Declaration | 🟢 |
| struct | `StepExperience` | hkask-services::kata | crates/hkask-services/src/kata.rs:487 | 🟡 Type Declaration | 🟢 |
| type | `CnsObserverFn` | hkask-services::kata | crates/hkask-services/src/kata.rs:590 | 🟡 Type Declaration | 🟢 |
| type | `ConsentCheckFn` | hkask-services::kata | crates/hkask-services/src/kata.rs:588 | 🟡 Type Declaration | 🟢 |
| type | `MetricCollectorFn` | hkask-services::kata | crates/hkask-services/src/kata.rs:592 | 🟡 Type Declaration | 🟢 |
| enum | `LifecycleError` | hkask-services::lifecycle | crates/hkask-services/src/lifecycle.rs:16 | 🟡 Type Declaration | 🟢 |
| enum | `ServerHealth` | hkask-services::lifecycle | crates/hkask-services/src/lifecycle.rs:29 | 🟡 Type Declaration | 🟢 |
| fn | `from_env` | hkask-services::lifecycle | crates/hkask-services/src/lifecycle.rs:120 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_healthy` | hkask-services::lifecycle | crates/hkask-services/src/lifecycle.rs:44 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ServerLifecycleConfig` | hkask-services::lifecycle | crates/hkask-services/src/lifecycle.rs:99 | 🟡 Type Declaration | 🟢 |
| trait | `ServerLifecycle` | hkask-services::lifecycle | crates/hkask-services/src/lifecycle.rs:63 | 🟡 Type Declaration | 🟢 |
| fn | `cleanup_failed_onboarding` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:364 | 🔴 Core Logic | 🟢 |
| fn | `derive_secrets` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:64 | 🔴 Core Logic | 🟢 |
| fn | `get_user_profile` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:221 | 🟢 Accessor/Constructor | 🟢 |
| fn | `remove_orphaned_db` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:318 | 🔴 Core Logic | 🟢 |
| fn | `store_user_profile` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:207 | 🔴 Core Logic | 🟢 |
| fn | `try_list_existing_replicants` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:283 | 🟢 Accessor/Constructor | 🟢 |
| struct | `MatrixRegistrationResult` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:538 | 🟡 Type Declaration | 🟢 |
| struct | `OnboardingService` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:53 | 🟡 Type Declaration | 🟢 |
| struct | `RegistryHandle` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:45 | 🟡 Type Declaration | 🟢 |
| struct | `ReplicantContactConfig` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:19 | 🟡 Type Declaration | 🟢 |
| struct | `ResolvedSecrets` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:29 | 🟡 Type Declaration | 🟢 |
| struct | `SignInOutcome` | hkask-services::onboarding | crates/hkask-services/src/onboarding.rs:36 | 🟡 Type Declaration | 🟢 |
| struct | `CreatePodRequest` | hkask-services::pods | crates/hkask-services/src/pods.rs:14 | 🟡 Type Declaration | 🟢 |
| struct | `PodResponse` | hkask-services::pods | crates/hkask-services/src/pods.rs:21 | 🟡 Type Declaration | 🟢 |
| struct | `PodService` | hkask-services::pods | crates/hkask-services/src/pods.rs:51 | 🟡 Type Declaration | 🟢 |
| struct | `PodStatusResponse` | hkask-services::pods | crates/hkask-services/src/pods.rs:26 | 🟡 Type Declaration | 🟢 |
| fn | `due_tasks` | hkask-services::scheduler | crates/hkask-services/src/scheduler.rs:58 | 🔴 Core Logic | 🟢 |
| fn | `list` | hkask-services::scheduler | crates/hkask-services/src/scheduler.rs:44 | 🔴 Core Logic | 🟢 |
| fn | `reschedule` | hkask-services::scheduler | crates/hkask-services/src/scheduler.rs:72 | 🔴 Core Logic | 🟢 |
| fn | `schedule` | hkask-services::scheduler | crates/hkask-services/src/scheduler.rs:18 | 🔴 Core Logic | 🟢 |
| struct | `SchedulerService` | hkask-services::scheduler | crates/hkask-services/src/scheduler.rs:10 | 🟡 Type Declaration | 🟢 |
| fn | `classifier_model` | hkask-services::settings | crates/hkask-services/src/settings.rs:158 | 🔴 Core Logic | 🟢 |
| fn | `embedding_model` | hkask-services::settings | crates/hkask-services/src/settings.rs:145 | 🔴 Core Logic | 🟢 |
| fn | `generation_model` | hkask-services::settings | crates/hkask-services/src/settings.rs:132 | 🔴 Core Logic | 🟢 |
| fn | `load_settings` | hkask-services::settings | crates/hkask-services/src/settings.rs:196 | 🔴 Core Logic | 🟢 |
| fn | `load` | hkask-services::settings | crates/hkask-services/src/settings.rs:94 | 🔴 Core Logic | 🟢 |
| fn | `ocr_model` | hkask-services::settings | crates/hkask-services/src/settings.rs:171 | 🔴 Core Logic | 🟢 |
| fn | `resolve_model` | hkask-services::settings | crates/hkask-services/src/settings.rs:114 | 🔴 Core Logic | 🟢 |
| fn | `save_settings` | hkask-services::settings | crates/hkask-services/src/settings.rs:218 | 🔴 Core Logic | 🟢 |
| fn | `save` | hkask-services::settings | crates/hkask-services/src/settings.rs:180 | 🔴 Core Logic | 🟢 |
| fn | `settings_path` | hkask-services::settings | crates/hkask-services/src/settings.rs:15 | 🔴 Core Logic | 🟢 |
| struct | `HkaskSettings` | hkask-services::settings | crates/hkask-services/src/settings.rs:27 | 🟡 Type Declaration | 🟢 |
| fn | `compute_file_hash` | hkask-services::skill | crates/hkask-services/src/skill.rs:139 | 🔴 Core Logic | 🟢 |
| fn | `discover_skills` | hkask-services::skill | crates/hkask-services/src/skill.rs:51 | 🔴 Core Logic | 🟢 |
| fn | `find_public_skill` | hkask-services::skill | crates/hkask-services/src/skill.rs:152 | 🔴 Core Logic | 🟢 |
| fn | `publish_skill` | hkask-services::skill | crates/hkask-services/src/skill.rs:183 | 🔴 Core Logic | 🟢 |
| fn | `read_skill_namespace` | hkask-services::skill | crates/hkask-services/src/skill.rs:128 | 🔴 Core Logic | 🟢 |
| fn | `read_skill_visibility` | hkask-services::skill | crates/hkask-services/src/skill.rs:99 | 🔴 Core Logic | 🟢 |
| fn | `resolve_replicant_name` | hkask-services::skill | crates/hkask-services/src/skill.rs:260 | 🔴 Core Logic | 🟢 |
| struct | `SkillInfo` | hkask-services::skill | crates/hkask-services/src/skill.rs:35 | 🟡 Type Declaration | 🟢 |
| struct | `SkillPublishResult` | hkask-services::skill | crates/hkask-services/src/skill.rs:22 | 🟡 Type Declaration | 🟢 |
| fn | `get_granted_categories` | hkask-services::sovereignty | crates/hkask-services/src/sovereignty.rs:48 | 🟢 Accessor/Constructor | 🟢 |
| fn | `grant_consent` | hkask-services::sovereignty | crates/hkask-services/src/sovereignty.rs:29 | 🔴 Core Logic | 🟢 |
| fn | `has_consent` | hkask-services::sovereignty | crates/hkask-services/src/sovereignty.rs:43 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-services::sovereignty | crates/hkask-services/src/sovereignty.rs:24 | 🟢 Accessor/Constructor | 🟢 |
| fn | `revoke_consent` | hkask-services::sovereignty | crates/hkask-services/src/sovereignty.rs:36 | 🔴 Core Logic | 🟢 |
| struct | `SovereigntyService` | hkask-services::sovereignty | crates/hkask-services/src/sovereignty.rs:18 | 🟡 Type Declaration | 🟢 |
| fn | `capture` | hkask-services::spec | crates/hkask-services/src/spec.rs:106 | 🔴 Core Logic | 🟢 |
| fn | `category_coverage` | hkask-services::spec | crates/hkask-services/src/spec.rs:224 | 🔴 Core Logic | 🟢 |
| fn | `get_by_id` | hkask-services::spec | crates/hkask-services/src/spec.rs:198 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_full` | hkask-services::spec | crates/hkask-services/src/spec.rs:187 | 🟢 Accessor/Constructor | 🟢 |
| fn | `list` | hkask-services::spec | crates/hkask-services/src/spec.rs:159 | 🔴 Core Logic | 🟢 |
| fn | `structural_quality_check` | hkask-services::spec | crates/hkask-services/src/spec.rs:269 | 🔴 Core Logic | 🟢 |
| fn | `validate` | hkask-services::spec | crates/hkask-services/src/spec.rs:303 | 🔴 Core Logic | 🟢 |
| struct | `CoherenceResult` | hkask-services::spec | crates/hkask-services/src/spec.rs:80 | 🟡 Type Declaration | 🟢 |
| struct | `SpecCaptureRequest` | hkask-services::spec | crates/hkask-services/src/spec.rs:26 | 🟡 Type Declaration | 🟢 |
| struct | `SpecCaptureResponse` | hkask-services::spec | crates/hkask-services/src/spec.rs:40 | 🟡 Type Declaration | 🟢 |
| struct | `SpecDetail` | hkask-services::spec | crates/hkask-services/src/spec.rs:71 | 🟡 Type Declaration | 🟢 |
| struct | `SpecListEntry` | hkask-services::spec | crates/hkask-services/src/spec.rs:49 | 🟡 Type Declaration | 🟢 |
| struct | `SpecService` | hkask-services::spec | crates/hkask-services/src/spec.rs:93 | 🟡 Type Declaration | 🟢 |
| struct | `WritingQualityResult` | hkask-services::spec | crates/hkask-services/src/spec.rs:87 | 🟡 Type Declaration | 🟢 |
| fn | `verify_json` | hkask-services::verification | crates/hkask-services/src/verification.rs:110 | 🔴 Core Logic | 🟢 |
| fn | `verify` | hkask-services::verification | crates/hkask-services/src/verification.rs:104 | 🔴 Core Logic | 🟢 |
| struct | `AssertionResult` | hkask-services::verification | crates/hkask-services/src/verification.rs:35 | 🟡 Type Declaration | 🟢 |
| struct | `Assertion` | hkask-services::verification | crates/hkask-services/src/verification.rs:23 | 🟡 Type Declaration | 🟢 |
| struct | `Manifest` | hkask-services::verification | crates/hkask-services/src/verification.rs:15 | 🟡 Type Declaration | 🟢 |
| struct | `PrincipleResult` | hkask-services::verification | crates/hkask-services/src/verification.rs:82 | 🟡 Type Declaration | 🟢 |
| struct | `VerificationReport` | hkask-services::verification | crates/hkask-services/src/verification.rs:89 | 🟡 Type Declaration | 🟢 |
| struct | `VerificationService` | hkask-services::verification | crates/hkask-services/src/verification.rs:98 | 🟡 Type Declaration | 🟢 |

| hkask-storage | 238 | 238 | 0 | 100% | 248 |

### hkask-storage

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| enum | `AgentRegistryError` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:10 | 🟡 Type Declaration | 🟢 |
| fn | `add_contact` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:272 | 🔴 Core Logic | 🟢 |
| fn | `add_scheduled_task` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:346 | 🔴 Core Logic | 🟢 |
| fn | `find_contacts` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:293 | 🔴 Core Logic | 🟢 |
| fn | `get_user_profile` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:255 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:98 | 🔴 Core Logic | 🟢 |
| fn | `initialize_schema` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:31 | 🔴 Core Logic | 🟢 |
| fn | `insert` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:74 | 🔴 Core Logic | 🟢 |
| fn | `list_by_kind` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:176 | 🔴 Core Logic | 🟢 |
| fn | `list_contacts` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:322 | 🔴 Core Logic | 🟢 |
| fn | `list_due_tasks` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:369 | 🔴 Core Logic | 🟢 |
| fn | `list_scheduled_tasks` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:395 | 🔴 Core Logic | 🟢 |
| fn | `list` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:133 | 🔴 Core Logic | 🟢 |
| fn | `remove` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:222 | 🔴 Core Logic | 🟢 |
| fn | `store_user_profile` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:240 | 🔴 Core Logic | 🟢 |
| fn | `update_next_run` | hkask-storage::agent_registry | crates/hkask-storage/src/agent_registry.rs:424 | 🔴 Core Logic | 🟢 |
| enum | `ConsentStoreError` | hkask-storage::consent_store | crates/hkask-storage/src/consent_store.rs:15 | 🟡 Type Declaration | 🟢 |
| fn | `delete` | hkask-storage::consent_store | crates/hkask-storage/src/consent_store.rs:141 | 🔴 Core Logic | 🟢 |
| fn | `get` | hkask-storage::consent_store | crates/hkask-storage/src/consent_store.rs:101 | 🔴 Core Logic | 🟢 |
| fn | `initialize_schema` | hkask-storage::consent_store | crates/hkask-storage/src/consent_store.rs:46 | 🔴 Core Logic | 🟢 |
| fn | `store` | hkask-storage::consent_store | crates/hkask-storage/src/consent_store.rs:69 | 🔴 Core Logic | 🟢 |
| struct | `StoredConsentRecord` | hkask-storage::consent_store | crates/hkask-storage/src/consent_store.rs:29 | 🟡 Type Declaration | 🟢 |
| enum | `DatabaseError` | hkask-storage::database | crates/hkask-storage/src/database.rs:55 | 🟡 Type Declaration | 🟢 |
| fn | `conn_arc` | hkask-storage::database | crates/hkask-storage/src/database.rs:221 | 🔴 Core Logic | 🟢 |
| fn | `in_memory_db` | hkask-storage::database | crates/hkask-storage/src/database.rs:258 | 🔴 Core Logic | 🟢 |
| fn | `in_memory_with_extensions` | hkask-storage::database | crates/hkask-storage/src/database.rs:193 | 🔴 Core Logic | 🟢 |
| fn | `in_memory` | hkask-storage::database | crates/hkask-storage/src/database.rs:175 | 🔴 Core Logic | 🟢 |
| fn | `open_database` | hkask-storage::database | crates/hkask-storage/src/database.rs:238 | 🔴 Core Logic | 🟢 |
| fn | `open_with_extensions` | hkask-storage::database | crates/hkask-storage/src/database.rs:149 | 🔴 Core Logic | 🟢 |
| fn | `open` | hkask-storage::database | crates/hkask-storage/src/database.rs:129 | 🟢 Accessor/Constructor | 🟢 |
| struct | `Database` | hkask-storage::database | crates/hkask-storage/src/database.rs:71 | 🟡 Type Declaration | 🟢 |
| enum | `EmbeddingError` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:25 | 🟡 Type Declaration | 🟢 |
| fn | `count` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:339 | 🔴 Core Logic | 🟢 |
| fn | `delete` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:286 | 🔴 Core Logic | 🟢 |
| fn | `get` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:197 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:73 | 🟢 Accessor/Constructor | 🟢 |
| fn | `query_by_prefix` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:353 | 🔴 Core Logic | 🟢 |
| fn | `search` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:235 | 🔴 Core Logic | 🟢 |
| fn | `store` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:137 | 🔴 Core Logic | 🟢 |
| fn | `with_dim` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:86 | 🟢 Accessor/Constructor | 🟢 |
| struct | `EmbeddingStore` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:50 | 🟡 Type Declaration | 🟢 |
| struct | `SimilarityResult` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:19 | 🟡 Type Declaration | 🟢 |
| struct | `StoredEmbedding` | hkask-storage::embeddings | crates/hkask-storage/src/embeddings.rs:11 | 🟡 Type Declaration | 🟢 |
| enum | `EscalationError` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:66 | 🟡 Type Declaration | 🟢 |
| enum | `EscalationStatus` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:55 | 🟡 Type Declaration | 🟢 |
| fn | `add` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:125 | 🔴 Core Logic | 🟢 |
| fn | `dismiss` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:279 | 🔴 Core Logic | 🟢 |
| fn | `get` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:203 | 🔴 Core Logic | 🟢 |
| fn | `list_pending` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:159 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:338 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:94 | 🟢 Accessor/Constructor | 🟢 |
| fn | `pending` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:36 | 🔴 Core Logic | 🟢 |
| fn | `resolve` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:262 | 🔴 Core Logic | 🟢 |
| fn | `stats` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:295 | 🔴 Core Logic | 🟢 |
| fn | `summary` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:352 | 🔴 Core Logic | 🟢 |
| struct | `EscalationBatch` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:324 | 🟡 Type Declaration | 🟢 |
| struct | `EscalationEntry` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:16 | 🟡 Type Declaration | 🟢 |
| struct | `EscalationQueue` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:61 | 🟡 Type Declaration | 🟢 |
| struct | `EscalationStats` | hkask-storage::escalation | crates/hkask-storage/src/escalation.rs:369 | 🟡 Type Declaration | 🟢 |
| enum | `GalleryMode` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:43 | 🟡 Type Declaration | 🟢 |
| enum | `GalleryStoreError` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:21 | 🟡 Type Declaration | 🟢 |
| fn | `add_image` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:266 | 🔴 Core Logic | 🟢 |
| fn | `as_str` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:70 | 🟢 Accessor/Constructor | 🟢 |
| fn | `create` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:218 | 🔴 Core Logic | 🟢 |
| fn | `get_all_tags` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:478 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_face` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:593 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_gallery` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:440 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_image` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:319 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_tags` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:415 | 🟢 Accessor/Constructor | 🟢 |
| fn | `init_tables` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:145 | 🔴 Core Logic | 🟢 |
| fn | `list_faces` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:558 | 🔴 Core Logic | 🟢 |
| fn | `register_face` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:519 | 🔴 Core Logic | 🟢 |
| fn | `remove_face` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:621 | 🔴 Core Logic | 🟢 |
| fn | `tag_image` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:371 | 🔴 Core Logic | 🟢 |
| fn | `update_face` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:641 | 🔴 Core Logic | 🟢 |
| struct | `FaceRegistryRecord` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:122 | 🟡 Type Declaration | 🟢 |
| struct | `GalleryRecord` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:81 | 🟡 Type Declaration | 🟢 |
| struct | `ImageRecord` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:93 | 🟡 Type Declaration | 🟢 |
| struct | `TagRecord` | hkask-storage::gallery | crates/hkask-storage/src/gallery.rs:108 | 🟡 Type Declaration | 🟢 |
| enum | `GoalRepositoryError` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:20 | 🟡 Type Declaration | 🟢 |
| fn | `add_artifact` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:312 | 🔴 Core Logic | 🟢 |
| fn | `add_criterion` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:289 | 🔴 Core Logic | 🟢 |
| fn | `create_goal` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:201 | 🔴 Core Logic | 🟢 |
| fn | `create_subgoal` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:402 | 🔴 Core Logic | 🟢 |
| fn | `delete_goal` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:448 | 🔴 Core Logic | 🟢 |
| fn | `get_artifacts` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:361 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_criteria` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:334 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_goal` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:220 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_subgoals` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:435 | 🟢 Accessor/Constructor | 🟢 |
| fn | `goal_from_row` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:145 | 🔴 Core Logic | 🟢 |
| fn | `list_goals` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:266 | 🔴 Core Logic | 🟢 |
| fn | `list_quarantined_goals` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:545 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:98 | 🟢 Accessor/Constructor | 🟢 |
| fn | `quarantine_goal` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:466 | 🔴 Core Logic | 🟢 |
| fn | `repair_quarantined_goal` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:491 | 🔴 Core Logic | 🟢 |
| fn | `try_goal_from_row` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:130 | 🟢 Accessor/Constructor | 🟢 |
| fn | `update_goal_state` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:235 | 🔴 Core Logic | 🟢 |
| fn | `with_telemetry` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:110 | 🟢 Accessor/Constructor | 🟢 |
| struct | `QuarantinedGoal` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:49 | 🟡 Type Declaration | 🟢 |
| struct | `SqliteGoalRepository` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:73 | 🟡 Type Declaration | 🟢 |
| type | `Result` | hkask-storage::goals | crates/hkask-storage/src/goals.rs:45 | 🟡 Type Declaration | 🟢 |
| enum | `KataHistoryError` | hkask-storage::kata_history | crates/hkask-storage/src/kata_history.rs:43 | 🟡 Type Declaration | 🟢 |
| fn | `count_entries_for_agent` | hkask-storage::kata_history | crates/hkask-storage/src/kata_history.rs:136 | 🔴 Core Logic | 🟢 |
| fn | `count_entries_on` | hkask-storage::kata_history | crates/hkask-storage/src/kata_history.rs:152 | 🔴 Core Logic | 🟢 |
| fn | `delete_entries_before` | hkask-storage::kata_history | crates/hkask-storage/src/kata_history.rs:267 | 🔴 Core Logic | 🟢 |
| fn | `entries_for_agent` | hkask-storage::kata_history | crates/hkask-storage/src/kata_history.rs:90 | 🔴 Core Logic | 🟢 |
| fn | `entries_in_range` | hkask-storage::kata_history | crates/hkask-storage/src/kata_history.rs:218 | 🔴 Core Logic | 🟢 |
| fn | `last_entry_for_agent` | hkask-storage::kata_history | crates/hkask-storage/src/kata_history.rs:172 | 🔴 Core Logic | 🟢 |
| fn | `record` | hkask-storage::kata_history | crates/hkask-storage/src/kata_history.rs:67 | 🔴 Core Logic | 🟢 |
| struct | `KataHistoryEntry` | hkask-storage::kata_history | crates/hkask-storage/src/kata_history.rs:22 | 🟡 Type Declaration | 🟢 |
| fn | `lock_mutex` | hkask-storage::lock_helpers | crates/hkask-storage/src/lock_helpers.rs:33 | 🔴 Core Logic | 🟢 |
| fn | `read_rwlock` | hkask-storage::lock_helpers | crates/hkask-storage/src/lock_helpers.rs:50 | 🔴 Core Logic | 🟢 |
| fn | `write_rwlock` | hkask-storage::lock_helpers | crates/hkask-storage/src/lock_helpers.rs:67 | 🔴 Core Logic | 🟢 |
| fn | `lambda_for` | hkask-storage::nu_event_store | crates/hkask-storage/src/nu_event_store.rs:117 | 🔴 Core Logic | 🟢 |
| fn | `load_cursor` | hkask-storage::nu_event_store | crates/hkask-storage/src/nu_event_store.rs:190 | 🔴 Core Logic | 🟢 |
| fn | `persist_cursor` | hkask-storage::nu_event_store | crates/hkask-storage/src/nu_event_store.rs:172 | 🔴 Core Logic | 🟢 |
| fn | `query_algedonic` | hkask-storage::nu_event_store | crates/hkask-storage/src/nu_event_store.rs:204 | 🔴 Core Logic | 🟢 |
| fn | `replay_weighted` | hkask-storage::nu_event_store | crates/hkask-storage/src/nu_event_store.rs:81 | 🔴 Core Logic | 🟢 |
| struct | `DecayConfig` | hkask-storage::nu_event_store | crates/hkask-storage/src/nu_event_store.rs:13 | 🟡 Type Declaration | 🟢 |
| struct | `WeightedEvent` | hkask-storage::nu_event_store | crates/hkask-storage/src/nu_event_store.rs:40 | 🟡 Type Declaration | 🟢 |
| fn | `sanitize_path` | hkask-storage::security | crates/hkask-storage/src/security.rs:17 | 🔴 Core Logic | 🟢 |
| enum | `SovereigntyStoreError` | hkask-storage::sovereignty | crates/hkask-storage/src/sovereignty.rs:17 | 🟡 Type Declaration | 🟢 |
| fn | `delete` | hkask-storage::sovereignty | crates/hkask-storage/src/sovereignty.rs:274 | 🔴 Core Logic | 🟢 |
| fn | `get` | hkask-storage::sovereignty | crates/hkask-storage/src/sovereignty.rs:223 | 🔴 Core Logic | 🟢 |
| fn | `initialize_schema` | hkask-storage::sovereignty | crates/hkask-storage/src/sovereignty.rs:54 | 🔴 Core Logic | 🟢 |
| fn | `store` | hkask-storage::sovereignty | crates/hkask-storage/src/sovereignty.rs:184 | 🔴 Core Logic | 🟢 |
| struct | `SovereigntyBoundaryEntry` | hkask-storage::sovereignty | crates/hkask-storage/src/sovereignty.rs:31 | 🟡 Type Declaration | 🟢 |
| fn | `init_schema` | hkask-storage::spec_store | crates/hkask-storage/src/spec_store.rs:135 | 🔴 Core Logic | 🟢 |
| fn | `init_schema` | hkask-storage::spec_store | crates/hkask-storage/src/spec_store.rs:156 | 🔴 Core Logic | 🟢 |
| fn | `list_curation_records_since` | hkask-storage::spec_store | crates/hkask-storage/src/spec_store.rs:216 | 🔴 Core Logic | 🟢 |
| fn | `load_all_curation_records` | hkask-storage::spec_store | crates/hkask-storage/src/spec_store.rs:243 | 🔴 Core Logic | 🟢 |
| fn | `load_curation_records` | hkask-storage::spec_store | crates/hkask-storage/src/spec_store.rs:196 | 🔴 Core Logic | 🟢 |
| fn | `save_curation_record` | hkask-storage::spec_store | crates/hkask-storage/src/spec_store.rs:174 | 🔴 Core Logic | 🟢 |
| trait | `SpecStore` | hkask-storage::spec_store | crates/hkask-storage/src/spec_store.rs:19 | 🟡 Type Declaration | 🟢 |
| enum | `DomainAnchor` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:160 | 🟡 Type Declaration | 🟢 |
| enum | `SpecCategory` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:81 | 🟡 Type Declaration | 🟢 |
| enum | `SpecError` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:390 | 🟡 Type Declaration | 🟢 |
| fn | `all` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:122 | 🔴 Core Logic | 🟢 |
| fn | `as_str` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:21 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:96 | 🟢 Accessor/Constructor | 🟢 |
| fn | `can_have_subgoals` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:220 | 🔴 Core Logic | 🟢 |
| fn | `coherence` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:230 | 🔴 Core Logic | 🟢 |
| fn | `coherence` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:337 | 🔴 Core Logic | 🟢 |
| fn | `collection_coherence` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:344 | 🔴 Core Logic | 🟢 |
| fn | `drift` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:310 | 🔴 Core Logic | 🟢 |
| fn | `from_string` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:54 | 🟢 Accessor/Constructor | 🟢 |
| fn | `infer_spec_category` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:141 | 🔴 Core Logic | 🟢 |
| fn | `is_complete` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:224 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_complete` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:333 | 🟢 Accessor/Constructor | 🟢 |
| fn | `mark_satisfied` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:181 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:174 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:198 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:271 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:371 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:46 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse_str` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:111 | 🔴 Core Logic | 🟢 |
| fn | `parse_str` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:28 | 🔴 Core Logic | 🟢 |
| fn | `with_criterion` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:215 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_declared_verb` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:288 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_display_name` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:210 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_goal` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:328 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_signature` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:296 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_valid_from` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:300 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_valid_to` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:304 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_version` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:292 | 🟢 Accessor/Constructor | 🟢 |
| struct | `Criterion` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:168 | 🟡 Type Declaration | 🟢 |
| struct | `DriftReport` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:248 | 🟡 Type Declaration | 🟢 |
| struct | `GoalSpec` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:187 | 🟡 Type Declaration | 🟢 |
| struct | `SpecCurationRecord` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:361 | 🟡 Type Declaration | 🟢 |
| struct | `SpecId` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:39 | 🟡 Type Declaration | 🟢 |
| struct | `Spec` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:255 | 🟡 Type Declaration | 🟢 |
| trait | `SpecCurator` | hkask-storage::spec_types | crates/hkask-storage/src/spec_types.rs:422 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-storage::store_macros | crates/hkask-storage/src/store_macros.rs:66 | 🟢 Accessor/Constructor | 🟢 |
| trait | `Store` | hkask-storage::store_macros | crates/hkask-storage/src/store_macros.rs:32 | 🟡 Type Declaration | 🟢 |
| enum | `TripleError` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:13 | 🟡 Type Declaration | 🟢 |
| fn | `close_by_id` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:419 | 🔴 Core Logic | 🟢 |
| fn | `count_by_perspective` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:403 | 🔴 Core Logic | 🟢 |
| fn | `count_semantic_below_confidence` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:330 | 🔴 Core Logic | 🟢 |
| fn | `count_semantic_by_entity` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:387 | 🔴 Core Logic | 🟢 |
| fn | `count_semantic` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:371 | 🔴 Core Logic | 🟢 |
| fn | `delete_by_entity_prefix` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:449 | 🔴 Core Logic | 🟢 |
| fn | `delete_by_id` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:435 | 🔴 Core Logic | 🟢 |
| fn | `get_by_id` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:283 | 🟢 Accessor/Constructor | 🟢 |
| fn | `insert` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:105 | 🔴 Core Logic | 🟢 |
| fn | `is_episodic` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:83 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_semantic` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:90 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:42 | 🟢 Accessor/Constructor | 🟢 |
| fn | `query_by_attribute` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:189 | 🔴 Core Logic | 🟢 |
| fn | `query_by_entity_attribute` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:148 | 🔴 Core Logic | 🟢 |
| fn | `query_by_entity` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:130 | 🔴 Core Logic | 🟢 |
| fn | `query_by_perspective` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:170 | 🔴 Core Logic | 🟢 |
| fn | `query_semantic_below_confidence` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:346 | 🔴 Core Logic | 🟢 |
| fn | `query_semantic_lowest_confidence` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:305 | 🔴 Core Logic | 🟢 |
| fn | `update` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:209 | 🔴 Core Logic | 🟢 |
| fn | `with_confidence` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:58 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_perspective` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:66 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_visibility` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:74 | 🟢 Accessor/Constructor | 🟢 |
| struct | `Triple` | hkask-storage::triples | crates/hkask-storage/src/triples.rs:26 | 🟡 Type Declaration | 🟢 |
| enum | `UserStoreError` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:18 | 🟡 Type Declaration | 🟢 |
| fn | `change_passphrase` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:225 | 🔴 Core Logic | 🟢 |
| fn | `check_passphrase_expiry` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:267 | 🔴 Core Logic | 🟢 |
| fn | `get_replicant` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:332 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_session` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:298 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_user` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:349 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_wallet_id` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:396 | 🟢 Accessor/Constructor | 🟢 |
| fn | `initialize_schema` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:84 | 🔴 Core Logic | 🟢 |
| fn | `list_replicants` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:382 | 🔴 Core Logic | 🟢 |
| fn | `list_sessions` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:315 | 🔴 Core Logic | 🟢 |
| fn | `login` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:174 | 🔴 Core Logic | 🟢 |
| fn | `logout` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:210 | 🔴 Core Logic | 🟢 |
| fn | `register_replicant` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:101 | 🔴 Core Logic | 🟢 |
| fn | `set_wallet_id` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:409 | 🟢 Accessor/Constructor | 🟢 |
| type | `UserResult` | hkask-storage::user_store | crates/hkask-storage/src/user_store.rs:42 | 🟡 Type Declaration | 🟢 |
| fn | `consume_deposit_reference` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:648 | 🔴 Core Logic | 🟢 |
| fn | `consume_encumbrance` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:801 | 🔴 Core Logic | 🟢 |
| fn | `credit_rjoules` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:181 | 🔴 Core Logic | 🟢 |
| fn | `debit_rjoules` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:208 | 🔴 Core Logic | 🟢 |
| fn | `enable_wal_mode` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:84 | 🔴 Core Logic | 🟢 |
| fn | `encumber_rjoules` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:698 | 🔴 Core Logic | 🟢 |
| fn | `ensure_wallet` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:152 | 🔴 Core Logic | 🟢 |
| fn | `get_api_key_by_public_key` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:406 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_api_key` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:369 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_balance` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:104 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_deposit_addresses` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:561 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_encumbrance` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:874 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_transactions` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:274 | 🟢 Accessor/Constructor | 🟢 |
| fn | `list_api_keys` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:446 | 🔴 Core Logic | 🟢 |
| fn | `list_wallet_ids` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:162 | 🔴 Core Logic | 🟢 |
| fn | `purge_expired_references` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:676 | 🔴 Core Logic | 🟢 |
| fn | `record_transaction` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:246 | 🔴 Core Logic | 🟢 |
| fn | `release_encumbrance` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:754 | 🔴 Core Logic | 🟢 |
| fn | `resolve_wallet_for_address` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:601 | 🔴 Core Logic | 🟢 |
| fn | `revoke_api_key` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:484 | 🔴 Core Logic | 🟢 |
| fn | `store_api_key` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:335 | 🔴 Core Logic | 🟢 |
| fn | `store_deposit_address` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:533 | 🔴 Core Logic | 🟢 |
| fn | `store_deposit_reference` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:626 | 🔴 Core Logic | 🟢 |
| fn | `transaction_exists_by_hash` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:317 | 🔴 Core Logic | 🟢 |
| fn | `update_spent_rj` | hkask-storage::wallet_store | crates/hkask-storage/src/wallet_store.rs:516 | 🔴 Core Logic | 🟢 |

| hkask-templates | 65 | 62 | 3 | 95% | 79 |

### hkask-templates

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `new` | hkask-templates::capability_validator | crates/hkask-templates/src/capability_validator.rs:28 | 🟢 Accessor/Constructor | 🟢 |
| fn | `validate_capabilities` | hkask-templates::capability_validator | crates/hkask-templates/src/capability_validator.rs:43 | 🔴 Core Logic | 🟢 |
| struct | `CapabilityAwareValidator` | hkask-templates::capability_validator | crates/hkask-templates/src/capability_validator.rs:21 | 🟡 Type Declaration | 🟢 |
| enum | `ValidationMode` | hkask-templates::contract_validator | crates/hkask-templates/src/contract_validator.rs:16 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-templates::contract_validator | crates/hkask-templates/src/contract_validator.rs:34 | 🟢 Accessor/Constructor | 🟢 |
| fn | `validate_terms` | hkask-templates::contract_validator | crates/hkask-templates/src/contract_validator.rs:69 | 🔴 Core Logic | 🟢 |
| fn | `with_lexicon` | hkask-templates::contract_validator | crates/hkask-templates/src/contract_validator.rs:46 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_mode` | hkask-templates::contract_validator | crates/hkask-templates/src/contract_validator.rs:57 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ContractValidator` | hkask-templates::contract_validator | crates/hkask-templates/src/contract_validator.rs:24 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-templates::executor | crates/hkask-templates/src/executor.rs:75 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ManifestExecutor` | hkask-templates::executor | crates/hkask-templates/src/executor.rs:54 | 🟡 Type Declaration | 🟢 |
| fn | `load_hlexicon_default` | hkask-templates::lexicon | crates/hkask-templates/src/lexicon.rs:93 | 🔴 Core Logic | 🟢 |
| fn | `load_hlexicon_from_file` | hkask-templates::lexicon | crates/hkask-templates/src/lexicon.rs:81 | 🔴 Core Logic | 🟢 |
| fn | `load_hlexicon_from_yaml` | hkask-templates::lexicon | crates/hkask-templates/src/lexicon.rs:54 | 🔴 Core Logic | 🟢 |
| fn | `parse_markdown_catalog` | hkask-templates::lexicon | crates/hkask-templates/src/lexicon.rs:117 | 🔴 Core Logic | 🟢 |
| fn | `regenerate_workspace_yaml` | hkask-templates::lexicon | crates/hkask-templates/src/lexicon.rs:244 | 🔴 Core Logic | 🟢 |
| fn | `render_workspace_yaml` | hkask-templates::lexicon | crates/hkask-templates/src/lexicon.rs:185 | 🔴 Core Logic | 🟢 |
| fn | `resolve_manifest` | hkask-templates::manifest_loader | crates/hkask-templates/src/manifest_loader.rs:171 | 🔴 Core Logic | 🟢 |
| enum | `TemplateError` | hkask-templates::ports | crates/hkask-templates/src/ports.rs:16 | 🟡 Type Declaration | 🔴 |
| trait | `McpPort` | hkask-templates::ports | crates/hkask-templates/src/ports.rs:54 | 🟡 Type Declaration | 🔴 |
| type | `Result` | hkask-templates::ports | crates/hkask-templates/src/ports.rs:41 | 🟡 Type Declaration | 🔴 |
| enum | `PromptStrategy` | hkask-templates::prompt_strategy | crates/hkask-templates/src/prompt_strategy.rs:13 | 🟡 Type Declaration | 🟢 |
| fn | `frame` | hkask-templates::prompt_strategy | crates/hkask-templates/src/prompt_strategy.rs:43 | 🔴 Core Logic | 🟢 |
| fn | `from_input` | hkask-templates::prompt_strategy | crates/hkask-templates/src/prompt_strategy.rs:28 | 🟢 Accessor/Constructor | 🟢 |
| fn | `name` | hkask-templates::prompt_strategy | crates/hkask-templates/src/prompt_strategy.rs:55 | 🔴 Core Logic | 🟢 |
| fn | `bootstrap` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:337 | 🔴 Core Logic | 🟢 |
| fn | `count` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:194 | 🔴 Core Logic | 🟢 |
| fn | `find_bundle_by_skills` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:318 | 🔴 Core Logic | 🟢 |
| fn | `get_bundle` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:290 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_skill` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:242 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:179 | 🔴 Core Logic | 🟢 |
| fn | `list_bundles` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:298 | 🔴 Core Logic | 🟢 |
| fn | `list_skills_by_visibility` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:211 | 🔴 Core Logic | 🟢 |
| fn | `list_skills` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:202 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:45 | 🟢 Accessor/Constructor | 🟢 |
| fn | `register_bundle` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:281 | 🔴 Core Logic | 🟢 |
| fn | `register_skill` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:233 | 🔴 Core Logic | 🟢 |
| fn | `register` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:151 | 🔴 Core Logic | 🟢 |
| fn | `reload` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:72 | 🔴 Core Logic | 🟢 |
| fn | `remove_bundle` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:307 | 🔴 Core Logic | 🟢 |
| fn | `remove_skill` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:224 | 🔴 Core Logic | 🟢 |
| fn | `set_lexicon` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:59 | 🟢 Accessor/Constructor | 🟢 |
| fn | `skills_by_domain` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:251 | 🔴 Core Logic | 🟢 |
| fn | `skills_referencing_template` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:264 | 🔴 Core Logic | 🟢 |
| fn | `validate_template_path` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:86 | 🔴 Core Logic | 🟢 |
| struct | `Registry` | hkask-templates::registry | crates/hkask-templates/src/registry.rs:30 | 🟡 Type Declaration | 🟢 |
| fn | `count` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:314 | 🔴 Core Logic | 🟢 |
| fn | `delete_entry` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:266 | 🔴 Core Logic | 🟢 |
| fn | `get_entry` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:244 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_skill_owned` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:562 | 🟢 Accessor/Constructor | 🟢 |
| fn | `list_skills_owned` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:616 | 🔴 Core Logic | 🟢 |
| fn | `new_with_conn` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:100 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:76 | 🟢 Accessor/Constructor | 🟢 |
| fn | `register` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:150 | 🔴 Core Logic | 🟢 |
| fn | `search_by_lexicon` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:291 | 🔴 Core Logic | 🟢 |
| fn | `set_lexicon` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:139 | 🟢 Accessor/Constructor | 🟢 |
| fn | `skills_by_domain_owned` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:625 | 🔴 Core Logic | 🟢 |
| fn | `skills_referencing_template_owned` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:637 | 🔴 Core Logic | 🟢 |
| struct | `SqliteRegistry` | hkask-templates::registry_sqlite | crates/hkask-templates/src/registry_sqlite.rs:65 | 🟡 Type Declaration | 🟢 |
| fn | `load_into` | hkask-templates::skill_loader | crates/hkask-templates/src/skill_loader.rs:64 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-templates::skill_loader | crates/hkask-templates/src/skill_loader.rs:52 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse_front_matter` | hkask-templates::skill_loader | crates/hkask-templates/src/skill_loader.rs:193 | 🔴 Core Logic | 🟢 |
| struct | `SkillFrontMatter` | hkask-templates::skill_loader | crates/hkask-templates/src/skill_loader.rs:21 | 🟡 Type Declaration | 🟢 |
| struct | `SkillLoadResult` | hkask-templates::skill_loader | crates/hkask-templates/src/skill_loader.rs:34 | 🟡 Type Declaration | 🟢 |
| struct | `SkillLoader` | hkask-templates::skill_loader | crates/hkask-templates/src/skill_loader.rs:41 | 🟡 Type Declaration | 🟢 |

| hkask-test-harness | 51 | 51 | 0 | 100% | 60 |

### hkask-test-harness

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| enum | `MockToolState` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:409 | 🟡 Type Declaration | 🟢 |
| enum | `SignalValence` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:251 | 🟡 Type Declaration | 🟢 |
| fn | `advance_time` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:328 | 🔴 Core Logic | 🟢 |
| fn | `alice` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:166 | 🔴 Core Logic | 🟢 |
| fn | `bob` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:174 | 🔴 Core Logic | 🟢 |
| fn | `carol` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:182 | 🔴 Core Logic | 🟢 |
| fn | `conn_arc` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:78 | 🔴 Core Logic | 🟢 |
| fn | `conn` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:70 | 🔴 Core Logic | 🟢 |
| fn | `execute_batch` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:87 | 🔴 Core Logic | 🟢 |
| fn | `from_persona` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:199 | 🟢 Accessor/Constructor | 🟢 |
| fn | `homeostatic` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:220 | 🔴 Core Logic | 🟢 |
| fn | `inject` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:312 | 🔴 Core Logic | 🟢 |
| fn | `is_homeostatic` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:369 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_negative_valence` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:262 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_positive_valence` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:270 | 🟢 Accessor/Constructor | 🟢 |
| fn | `key_path` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:133 | 🔴 Core Logic | 🟢 |
| fn | `master_key` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:141 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:109 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:290 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:57 | 🟢 Accessor/Constructor | 🟢 |
| fn | `path` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:125 | 🔴 Core Logic | 🟢 |
| fn | `perturbed` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:234 | 🔴 Core Logic | 🟢 |
| fn | `random` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:190 | 🔴 Core Logic | 🟢 |
| fn | `recent_signals` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:347 | 🔴 Core Logic | 🟢 |
| fn | `record_variety` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:378 | 🔴 Core Logic | 🟢 |
| fn | `temp_dir` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:428 | 🔴 Core Logic | 🟢 |
| fn | `test_event_with_observer` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:464 | 🔴 Core Logic | 🟢 |
| fn | `test_event` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:449 | 🔴 Core Logic | 🟢 |
| fn | `test_triple_with_owner` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:492 | 🔴 Core Logic | 🟢 |
| fn | `test_triple` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:483 | 🔴 Core Logic | 🟢 |
| fn | `tool_state` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:356 | 🔴 Core Logic | 🟢 |
| fn | `variety_for_domain` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:391 | 🔴 Core Logic | 🟢 |
| fn | `with_state` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:301 | 🟢 Accessor/Constructor | 🟢 |
| struct | `MockAlgedonicSignal` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:244 | 🟡 Type Declaration | 🟢 |
| struct | `MockCnsRuntime` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:281 | 🟡 Type Declaration | 🟢 |
| struct | `MockCnsState` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:208 | 🟡 Type Declaration | 🟢 |
| struct | `TestDb` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:48 | 🟡 Type Declaration | 🟢 |
| struct | `TestKeystore` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:98 | 🟡 Type Declaration | 🟢 |
| struct | `TestWebId` | hkask-test-harness | crates/hkask-test-harness/src/lib.rs:159 | 🟡 Type Declaration | 🟢 |
| fn | `clear_error` | hkask-test-harness::mocks | crates/hkask-test-harness/src/mocks.rs:109 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-test-harness::mocks | crates/hkask-test-harness/src/mocks.rs:47 | 🟢 Accessor/Constructor | 🟢 |
| fn | `set_error` | hkask-test-harness::mocks | crates/hkask-test-harness/src/mocks.rs:101 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_default` | hkask-test-harness::mocks | crates/hkask-test-harness/src/mocks.rs:79 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_model` | hkask-test-harness::mocks | crates/hkask-test-harness/src/mocks.rs:91 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_response` | hkask-test-harness::mocks | crates/hkask-test-harness/src/mocks.rs:64 | 🟢 Accessor/Constructor | 🟢 |
| struct | `MockInferencePort` | hkask-test-harness::mocks | crates/hkask-test-harness/src/mocks.rs:31 | 🟡 Type Declaration | 🟢 |
| fn | `any_capability_spec` | hkask-test-harness::strategies | crates/hkask-test-harness/src/strategies.rs:118 | 🔴 Core Logic | 🟢 |
| fn | `any_goal` | hkask-test-harness::strategies | crates/hkask-test-harness/src/strategies.rs:148 | 🔴 Core Logic | 🟢 |
| fn | `any_nu_event` | hkask-test-harness::strategies | crates/hkask-test-harness/src/strategies.rs:78 | 🔴 Core Logic | 🟢 |
| fn | `any_transcript_segment` | hkask-test-harness::strategies | crates/hkask-test-harness/src/strategies.rs:187 | 🔴 Core Logic | 🟢 |
| fn | `any_triple` | hkask-test-harness::strategies | crates/hkask-test-harness/src/strategies.rs:99 | 🔴 Core Logic | 🟢 |

| hkask-types | 490 | 415 | 75 | 84% | 308 |

### hkask-types

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| enum | `AgentKind` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:12 | 🟡 Type Declaration | 🟢 |
| fn | `as_persona_kind` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:36 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:22 | 🟢 Accessor/Constructor | 🟢 |
| fn | `compose_system_prompt` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:138 | 🔴 Core Logic | 🟢 |
| fn | `has_capability` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:190 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:47 | 🔴 Core Logic | 🟢 |
| fn | `responsibilities_flat` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:127 | 🔴 Core Logic | 🟢 |
| fn | `rights_flat` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:119 | 🔴 Core Logic | 🟢 |
| struct | `AgentDefinition` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:89 | 🟡 Type Declaration | 🟢 |
| struct | `Charter` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:64 | 🟡 Type Declaration | 🟢 |
| struct | `PersonaConstraints` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:74 | 🟡 Type Declaration | 🟢 |
| struct | `RegisteredAgent` | hkask-types::agent::definition | crates/hkask-types/src/agent/definition.rs:197 | 🟡 Type Declaration | 🟢 |
| enum | `Responsibility` | hkask-types::agent::profile | crates/hkask-types/src/agent/profile.rs:34 | 🟡 Type Declaration | 🟢 |
| enum | `Right` | hkask-types::agent::profile | crates/hkask-types/src/agent/profile.rs:7 | 🟡 Type Declaration | 🟢 |
| fn | `replicant_display_name` | hkask-types::agent::profile | crates/hkask-types/src/agent/profile.rs:90 | 🔴 Core Logic | 🟢 |
| fn | `to_display_string` | hkask-types::agent::profile | crates/hkask-types/src/agent/profile.rs:19 | 🟢 Accessor/Constructor | 🟢 |
| fn | `to_display_string` | hkask-types::agent::profile | crates/hkask-types/src/agent/profile.rs:52 | 🟢 Accessor/Constructor | 🟢 |
| struct | `Contact` | hkask-types::agent::profile | crates/hkask-types/src/agent/profile.rs:98 | 🟡 Type Declaration | 🟢 |
| struct | `ScheduledTask` | hkask-types::agent::profile | crates/hkask-types/src/agent/profile.rs:114 | 🟡 Type Declaration | 🟢 |
| struct | `UserProfile` | hkask-types::agent::profile | crates/hkask-types/src/agent/profile.rs:75 | 🟡 Type Declaration | 🟢 |
| enum | `AuditOutcome` | hkask-types::audit | crates/hkask-types/src/audit.rs:36 | 🟡 Type Declaration | 🟢 |
| fn | `new` | hkask-types::audit | crates/hkask-types/src/audit.rs:92 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_correlation_id` | hkask-types::audit | crates/hkask-types/src/audit.rs:114 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_metadata` | hkask-types::audit | crates/hkask-types/src/audit.rs:134 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_recipient` | hkask-types::audit | crates/hkask-types/src/audit.rs:124 | 🟢 Accessor/Constructor | 🟢 |
| struct | `AuditEntry` | hkask-types::audit | crates/hkask-types/src/audit.rs:15 | 🟡 Type Declaration | 🟢 |
| enum | `CascadePhase` | hkask-types::bundle::cascade | crates/hkask-types/src/bundle/cascade.rs:28 | 🟡 Type Declaration | 🟢 |
| fn | `as_str` | hkask-types::bundle::cascade | crates/hkask-types/src/bundle/cascade.rs:9 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse_str` | hkask-types::bundle::cascade | crates/hkask-types/src/bundle/cascade.rs:15 | 🔴 Core Logic | 🟢 |
| enum | `ComplementarityType` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:75 | 🟡 Type Declaration | 🟢 |
| enum | `ConflictResolution` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:51 | 🟡 Type Declaration | 🟢 |
| enum | `ConflictType` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:29 | 🟡 Type Declaration | 🟢 |
| fn | `as_str` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:11 | 🟢 Accessor/Constructor | 🟢 |
| fn | `complementarity_type_str` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:129 | 🔴 Core Logic | 🟢 |
| fn | `conflict_type_str` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:106 | 🔴 Core Logic | 🟢 |
| fn | `parse_str` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:16 | 🔴 Core Logic | 🟢 |
| fn | `resolution_str` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:112 | 🔴 Core Logic | 🟢 |
| struct | `BundleComplementarity` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:119 | 🟡 Type Declaration | 🟢 |
| struct | `BundleConflict` | hkask-types::bundle::composition | crates/hkask-types/src/bundle/composition.rs:94 | 🟡 Type Declaration | 🟢 |
| struct | `AuditConfig` | hkask-types::bundle::config | crates/hkask-types/src/bundle/config.rs:114 | 🟡 Type Declaration | 🔴 |
| struct | `CnsConfig` | hkask-types::bundle::config | crates/hkask-types/src/bundle/config.rs:92 | 🟡 Type Declaration | 🔴 |
| struct | `ConvergenceConfig` | hkask-types::bundle::config | crates/hkask-types/src/bundle/config.rs:11 | 🟡 Type Declaration | 🔴 |
| struct | `ErrorHandlingConfig` | hkask-types::bundle::config | crates/hkask-types/src/bundle/config.rs:50 | 🟡 Type Declaration | 🔴 |
| struct | `GasConfig` | hkask-types::bundle::config | crates/hkask-types/src/bundle/config.rs:30 | 🟡 Type Declaration | 🔴 |
| struct | `OcapConfig` | hkask-types::bundle::config | crates/hkask-types/src/bundle/config.rs:72 | 🟡 Type Declaration | 🔴 |
| enum | `SkillPolarity` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:41 | 🟡 Type Declaration | 🟢 |
| fn | `as_str` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:23 | 🟢 Accessor/Constructor | 🟢 |
| fn | `has_warnings` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:300 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_convergent` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:69 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_divergent` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:63 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_valid` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:295 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse_str` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:28 | 🔴 Core Logic | 🟢 |
| fn | `phase_str` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:107 | 🔴 Core Logic | 🟢 |
| fn | `skill_ids` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:280 | 🔴 Core Logic | 🟢 |
| fn | `skills_in_phase` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:267 | 🔴 Core Logic | 🟢 |
| fn | `total_step_gas` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:261 | 🔴 Core Logic | 🟢 |
| fn | `validate` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:143 | 🔴 Core Logic | 🟢 |
| struct | `BundleManifestStep` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:86 | 🟡 Type Declaration | 🟢 |
| struct | `BundleManifest` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:114 | 🟡 Type Declaration | 🟢 |
| struct | `BundleSkill` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:76 | 🟡 Type Declaration | 🟢 |
| struct | `ValidationResult` | hkask-types::bundle::manifest | crates/hkask-types/src/bundle/manifest.rs:287 | 🟡 Type Declaration | 🟢 |
| fn | `derive_signing_key` | hkask-types::capability::auth | crates/hkask-types/src/capability/auth.rs:22 | 🔴 Core Logic | 🔴 |
| struct | `AuthContext` | hkask-types::capability::auth | crates/hkask-types/src/capability/auth.rs:12 | 🟡 Type Declaration | 🔴 |
| enum | `CapabilityParseError` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:40 | 🟡 Type Declaration | 🔴 |
| enum | `DelegationAction` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:79 | 🟡 Type Declaration | 🔴 |
| enum | `DelegationResource` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:50 | 🟡 Type Declaration | 🔴 |
| fn | `as_str` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:59 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:86 | 🟢 Accessor/Constructor | 🟢 |
| fn | `capabilities_match` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:121 | 🔴 Core Logic | 🟢 |
| fn | `capability_from_server_id` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:112 | 🔴 Core Logic | 🔴 |
| fn | `parse_str` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:67 | 🔴 Core Logic | 🟢 |
| fn | `parse_str` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:93 | 🔴 Core Logic | 🟢 |
| fn | `parse` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:17 | 🔴 Core Logic | 🟢 |
| fn | `permits_read` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:106 | 🔴 Core Logic | 🟢 |
| fn | `permits_write` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:102 | 🔴 Core Logic | 🟢 |
| struct | `CapabilitySpec` | hkask-types::capability::resources | crates/hkask-types/src/capability/resources.rs:8 | 🟡 Type Declaration | 🔴 |
| fn | `expected_issuer` | hkask-types::capability::tokens | crates/hkask-types/src/capability/tokens.rs:33 | 🔴 Core Logic | 🟢 |
| fn | `issuer` | hkask-types::capability::tokens | crates/hkask-types/src/capability/tokens.rs:43 | 🔴 Core Logic | 🟢 |
| fn | `verify_issuer` | hkask-types::capability::tokens | crates/hkask-types/src/capability/tokens.rs:38 | 🔴 Core Logic | 🟢 |
| struct | `ConsolidationToken` | hkask-types::capability::tokens | crates/hkask-types/src/capability/tokens.rs:22 | 🟡 Type Declaration | 🟢 |
| fn | `allows_read` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:576 | 🔴 Core Logic | 🟢 |
| fn | `allows_write` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:569 | 🔴 Core Logic | 🟢 |
| fn | `as_u8` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:52 | 🟢 Accessor/Constructor | 🟢 |
| fn | `attenuate_with_expiry` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:418 | 🔴 Core Logic | 🟢 |
| fn | `attenuate` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:401 | 🔴 Core Logic | 🟢 |
| fn | `attenuation` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:190 | 🔴 Core Logic | 🟢 |
| fn | `can_attenuate` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:390 | 🔴 Core Logic | 🟢 |
| fn | `caveat_ids` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:531 | 🔴 Core Logic | 🟢 |
| fn | `context_nonce` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:198 | 🔴 Core Logic | 🟢 |
| fn | `expires_at` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:183 | 🔴 Core Logic | 🟢 |
| fn | `fingerprint` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:554 | 🔴 Core Logic | 🟢 |
| fn | `from_base64` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:383 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_caveat_data` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:544 | 🟢 Accessor/Constructor | 🟢 |
| fn | `grants_resource` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:470 | 🔴 Core Logic | 🟢 |
| fn | `has_caveat_type` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:537 | 🟢 Accessor/Constructor | 🟢 |
| fn | `holder` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:362 | 🔴 Core Logic | 🟢 |
| fn | `is_compatible_with` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:583 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_expired` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:354 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_valid_for` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:459 | 🟢 Accessor/Constructor | 🟢 |
| fn | `issuer` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:368 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:158 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:259 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:31 | 🟢 Accessor/Constructor | 🟢 |
| fn | `root_context_nonce` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:486 | 🔴 Core Logic | 🟢 |
| fn | `signature_bytes` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:346 | 🔴 Core Logic | 🟢 |
| fn | `sign` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:211 | 🔴 Core Logic | 🟢 |
| fn | `to_base64` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:376 | 🟢 Accessor/Constructor | 🟢 |
| fn | `unchecked` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:46 | 🔴 Core Logic | 🟢 |
| fn | `validate_context_nonce` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:477 | 🔴 Core Logic | 🟢 |
| fn | `verify_attenuation_chain` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:501 | 🔴 Core Logic | 🟢 |
| fn | `verify_cryptographic` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:525 | 🔴 Core Logic | 🟢 |
| fn | `verify` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:320 | 🔴 Core Logic | 🟢 |
| struct | `DelegationTokenBuilder` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:137 | 🟡 Type Declaration | 🟢 |
| struct | `DelegationToken` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:105 | 🟡 Type Declaration | 🟢 |
| struct | `TokenSignature` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:97 | 🟡 Type Declaration | 🟢 |
| type | `CapabilityToken` | hkask-types::capability::token_types | crates/hkask-types/src/capability/token_types.rs:592 | 🟡 Type Declaration | 🟢 |
| fn | `attenuate` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:261 | 🔴 Core Logic | 🟢 |
| fn | `check_resource` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:93 | 🔴 Core Logic | 🟢 |
| fn | `check` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:73 | 🔴 Core Logic | 🟢 |
| fn | `grant_cascade` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:212 | 🔴 Core Logic | 🟢 |
| fn | `grant_manifest` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:157 | 🔴 Core Logic | 🟢 |
| fn | `grant_registry` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:185 | 🔴 Core Logic | 🟢 |
| fn | `grant_spec` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:240 | 🔴 Core Logic | 🟢 |
| fn | `grant_template` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:129 | 🔴 Core Logic | 🟢 |
| fn | `grant_tool` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:110 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:30 | 🟢 Accessor/Constructor | 🟢 |
| fn | `verify_with_time` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:61 | 🔴 Core Logic | 🟢 |
| fn | `verify` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:50 | 🔴 Core Logic | 🟢 |
| fn | `with_signing_key` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:39 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CapabilityChecker` | hkask-types::capability::verification::checker | crates/hkask-types/src/capability/verification/checker.rs:16 | 🟡 Type Declaration | 🟢 |
| enum | `VerificationOutcome` | hkask-types::capability::verification::types | crates/hkask-types/src/capability/verification/types.rs:22 | 🟡 Type Declaration | 🟢 |
| fn | `token_err_insufficient_access` | hkask-types::capability::verification::types | crates/hkask-types/src/capability/verification/types.rs:44 | 🔴 Core Logic | 🟢 |
| fn | `token_err_tool_access_denied` | hkask-types::capability::verification::types | crates/hkask-types/src/capability/verification/types.rs:53 | 🔴 Core Logic | 🟢 |
| fn | `require_read_access` | hkask-types::capability::verification::verify | crates/hkask-types/src/capability/verification/verify.rs:137 | 🔴 Core Logic | 🟢 |
| fn | `require_write_access` | hkask-types::capability::verification::verify | crates/hkask-types/src/capability/verification/verify.rs:114 | 🔴 Core Logic | 🟢 |
| fn | `verify_delegation_token_now` | hkask-types::capability::verification::verify | crates/hkask-types/src/capability/verification/verify.rs:22 | 🔴 Core Logic | 🟢 |
| fn | `verify_delegation_token` | hkask-types::capability::verification::verify | crates/hkask-types/src/capability/verification/verify.rs:63 | 🔴 Core Logic | 🟢 |
| enum | `CircuitState` | hkask-types::cns | crates/hkask-types/src/cns.rs:53 | 🟡 Type Declaration | 🟢 |
| enum | `CnsSpan` | hkask-types::cns | crates/hkask-types/src/cns.rs:84 | 🟡 Type Declaration | 🟢 |
| enum | `ToolSubsystem` | hkask-types::cns | crates/hkask-types/src/cns.rs:209 | 🟡 Type Declaration | 🟢 |
| fn | `as_raw` | hkask-types::cns | crates/hkask-types/src/cns.rs:35 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-types::cns | crates/hkask-types/src/cns.rs:226 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-types::cns | crates/hkask-types/src/cns.rs:257 | 🟢 Accessor/Constructor | 🟢 |
| fn | `delay_for_attempt` | hkask-types::cns | crates/hkask-types/src/cns.rs:499 | 🔴 Core Logic | 🟢 |
| fn | `is_retryable_status` | hkask-types::cns | crates/hkask-types/src/cns.rs:507 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::cns | crates/hkask-types/src/cns.rs:27 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CnsHealth` | hkask-types::cns | crates/hkask-types/src/cns.rs:66 | 🟡 Type Declaration | 🟢 |
| struct | `QueueDepth` | hkask-types::cns | crates/hkask-types/src/cns.rs:23 | 🟡 Type Declaration | 🟢 |
| struct | `RetryConfig` | hkask-types::cns | crates/hkask-types/src/cns.rs:481 | 🟡 Type Declaration | 🟢 |
| struct | `SeamCoverage` | hkask-types::cns | crates/hkask-types/src/cns.rs:441 | 🟡 Type Declaration | 🟢 |
| struct | `SeamInventory` | hkask-types::cns | crates/hkask-types/src/cns.rs:467 | 🟡 Type Declaration | 🟢 |
| enum | `CurationDecision` | hkask-types::curation | crates/hkask-types/src/curation.rs:16 | 🟡 Type Declaration | 🔴 |
| enum | `OcapTokenKind` | hkask-types::curation | crates/hkask-types/src/curation.rs:64 | 🟡 Type Declaration | 🔴 |
| fn | `parse_ocap_token_kind` | hkask-types::curation | crates/hkask-types/src/curation.rs:89 | 🔴 Core Logic | 🔴 |
| fn | `parse_token` | hkask-types::curation | crates/hkask-types/src/curation.rs:143 | 🔴 Core Logic | 🔴 |
| fn | `token` | hkask-types::curation | crates/hkask-types/src/curation.rs:134 | 🔴 Core Logic | 🟢 |
| struct | `CurationThresholdConfig` | hkask-types::curation | crates/hkask-types/src/curation.rs:160 | 🟡 Type Declaration | 🔴 |
| struct | `OCAPBoundary` | hkask-types::curation | crates/hkask-types/src/curation.rs:124 | 🟡 Type Declaration | 🔴 |
| struct | `OcapCapability` | hkask-types::curation | crates/hkask-types/src/curation.rs:106 | 🟡 Type Declaration | 🔴 |
| enum | `InfrastructureError` | hkask-types::error | crates/hkask-types/src/error.rs:34 | 🟡 Type Declaration | 🟢 |
| enum | `McpErrorKind` | hkask-types::error | crates/hkask-types/src/error.rs:92 | 🟡 Type Declaration | 🟢 |
| fn | `is_retryable` | hkask-types::error | crates/hkask-types/src/error.rs:122 | 🟢 Accessor/Constructor | 🟢 |
| fn | `requires_intervention` | hkask-types::error | crates/hkask-types/src/error.rs:133 | 🔴 Core Logic | 🟢 |
| struct | `CapabilityDenied` | hkask-types::error | crates/hkask-types/src/error.rs:169 | 🟡 Type Declaration | 🟢 |
| struct | `DimensionMismatch` | hkask-types::error | crates/hkask-types/src/error.rs:181 | 🟡 Type Declaration | 🟢 |
| struct | `NotFound` | hkask-types::error | crates/hkask-types/src/error.rs:156 | 🟡 Type Declaration | 🟢 |
| enum | `Phase` | hkask-types::event | crates/hkask-types/src/event.rs:439 | 🟡 Type Declaration | 🟢 |
| enum | `SpanCategory` | hkask-types::event | crates/hkask-types/src/event.rs:250 | 🟡 Type Declaration | 🟢 |
| enum | `SpanKind` | hkask-types::event | crates/hkask-types/src/event.rs:375 | 🟡 Type Declaration | 🟢 |
| fn | `as_str` | hkask-types::event | crates/hkask-types/src/event.rs:204 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-types::event | crates/hkask-types/src/event.rs:351 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-types::event | crates/hkask-types/src/event.rs:447 | 🟢 Accessor/Constructor | 🟢 |
| fn | `category` | hkask-types::event | crates/hkask-types/src/event.rs:227 | 🔴 Core Logic | 🟢 |
| fn | `from_kind` | hkask-types::event | crates/hkask-types/src/event.rs:364 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_short_name` | hkask-types::event | crates/hkask-types/src/event.rs:269 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_str` | hkask-types::event | crates/hkask-types/src/event.rs:459 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::event | crates/hkask-types/src/event.rs:172 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::event | crates/hkask-types/src/event.rs:342 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::event | crates/hkask-types/src/event.rs:36 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse` | hkask-types::event | crates/hkask-types/src/event.rs:188 | 🔴 Core Logic | 🟢 |
| fn | `short_name` | hkask-types::event | crates/hkask-types/src/event.rs:211 | 🔴 Core Logic | 🟢 |
| fn | `with_outcome` | hkask-types::event | crates/hkask-types/src/event.rs:62 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_parent` | hkask-types::event | crates/hkask-types/src/event.rs:80 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_regulation` | hkask-types::event | crates/hkask-types/src/event.rs:71 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_visibility` | hkask-types::event | crates/hkask-types/src/event.rs:89 | 🟢 Accessor/Constructor | 🟢 |
| struct | `NuEvent` | hkask-types::event | crates/hkask-types/src/event.rs:16 | 🟡 Type Declaration | 🟢 |
| struct | `SpanNamespace` | hkask-types::event | crates/hkask-types/src/event.rs:101 | 🟡 Type Declaration | 🟢 |
| struct | `Span` | hkask-types::event | crates/hkask-types/src/event.rs:326 | 🟡 Type Declaration | 🟢 |
| trait | `NuEventSink` | hkask-types::event | crates/hkask-types/src/event.rs:473 | 🟡 Type Declaration | 🟢 |
| enum | `GoalState` | hkask-types::goal | crates/hkask-types/src/goal.rs:47 | 🟡 Type Declaration | 🟢 |
| fn | `as_str` | hkask-types::goal | crates/hkask-types/src/goal.rs:60 | 🟢 Accessor/Constructor | 🟢 |
| fn | `can_have_subgoals` | hkask-types::goal | crates/hkask-types/src/goal.rs:274 | 🔴 Core Logic | 🟢 |
| fn | `can_transition_to` | hkask-types::goal | crates/hkask-types/src/goal.rs:108 | 🔴 Core Logic | 🟢 |
| fn | `is_terminal` | hkask-types::goal | crates/hkask-types/src/goal.rs:89 | 🟢 Accessor/Constructor | 🟢 |
| fn | `mark_satisfied` | hkask-types::goal | crates/hkask-types/src/goal.rs:156 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-types::goal | crates/hkask-types/src/goal.rs:142 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::goal | crates/hkask-types/src/goal.rs:177 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::goal | crates/hkask-types/src/goal.rs:209 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse_str` | hkask-types::goal | crates/hkask-types/src/goal.rs:74 | 🔴 Core Logic | 🟢 |
| fn | `transition` | hkask-types::goal | crates/hkask-types/src/goal.rs:254 | 🔴 Core Logic | 🟢 |
| fn | `with_display_name` | hkask-types::goal | crates/hkask-types/src/goal.rs:228 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_parent` | hkask-types::goal | crates/hkask-types/src/goal.rs:237 | 🟢 Accessor/Constructor | 🟢 |
| struct | `GoalArtifact` | hkask-types::goal | crates/hkask-types/src/goal.rs:163 | 🟡 Type Declaration | 🟢 |
| struct | `GoalCriterion` | hkask-types::goal | crates/hkask-types/src/goal.rs:128 | 🟡 Type Declaration | 🟢 |
| struct | `Goal` | hkask-types::goal | crates/hkask-types/src/goal.rs:190 | 🟡 Type Declaration | 🟢 |
| struct | `IllegalGoalTransition` | hkask-types::goal | crates/hkask-types/src/goal.rs:26 | 🟡 Type Declaration | 🟢 |
| enum | `ApiKeyKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:177 | 🟡 Type Declaration | 🟢 |
| enum | `BotKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:141 | 🟡 Type Declaration | 🟢 |
| enum | `EmbeddingKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:157 | 🟡 Type Declaration | 🟢 |
| enum | `EscalationKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:181 | 🟡 Type Declaration | 🟢 |
| enum | `EventKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:149 | 🟡 Type Declaration | 🟢 |
| enum | `GoalKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:153 | 🟡 Type Declaration | 🟢 |
| enum | `PodKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:169 | 🟡 Type Declaration | 🟢 |
| enum | `TemplateKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:137 | 🟡 Type Declaration | 🟢 |
| enum | `TripleKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:145 | 🟡 Type Declaration | 🟢 |
| enum | `UserKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:161 | 🟡 Type Declaration | 🟢 |
| enum | `WalletKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:173 | 🟡 Type Declaration | 🟢 |
| fn | `as_uuid` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:111 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_name` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:102 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_uuid` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:85 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:75 | 🟢 Accessor/Constructor | 🟢 |
| struct | `Id` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:24 | 🟡 Type Declaration | 🟢 |
| trait | `IdKind` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:17 | 🟡 Type Declaration | 🟢 |
| trait | `Sealed` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:12 | 🟡 Type Declaration | 🟢 |
| type | `ApiKeyId` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:197 | 🟡 Type Declaration | 🟢 |
| type | `BotID` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:188 | 🟡 Type Declaration | 🟢 |
| type | `EmbeddingID` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:192 | 🟡 Type Declaration | 🟢 |
| type | `EscalationID` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:198 | 🟡 Type Declaration | 🟢 |
| type | `EventID` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:190 | 🟡 Type Declaration | 🟢 |
| type | `GoalID` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:191 | 🟡 Type Declaration | 🟢 |
| type | `PodID` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:195 | 🟡 Type Declaration | 🟢 |
| type | `TemplateID` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:187 | 🟡 Type Declaration | 🟢 |
| type | `TripleID` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:189 | 🟡 Type Declaration | 🟢 |
| type | `UserID` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:193 | 🟡 Type Declaration | 🟢 |
| type | `WalletId` | hkask-types::id::core | crates/hkask-types/src/id/core.rs:196 | 🟡 Type Declaration | 🟢 |
| enum | `RegistrationError` | hkask-types::identity | crates/hkask-types/src/identity.rs:147 | 🟡 Type Declaration | 🟢 |
| fn | `derive_webid` | hkask-types::identity | crates/hkask-types/src/identity.rs:71 | 🔴 Core Logic | 🟢 |
| fn | `is_expired` | hkask-types::identity | crates/hkask-types/src/identity.rs:122 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::identity | crates/hkask-types/src/identity.rs:27 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::identity | crates/hkask-types/src/identity.rs:81 | 🟢 Accessor/Constructor | 🟢 |
| struct | `HumanUser` | hkask-types::identity | crates/hkask-types/src/identity.rs:14 | 🟡 Type Declaration | 🟢 |
| struct | `RegistrationRequest` | hkask-types::identity | crates/hkask-types/src/identity.rs:130 | 🟡 Type Declaration | 🟢 |
| struct | `ReplicantIdentity` | hkask-types::identity | crates/hkask-types/src/identity.rs:51 | 🟡 Type Declaration | 🟢 |
| struct | `UserSession` | hkask-types::identity | crates/hkask-types/src/identity.rs:106 | 🟡 Type Declaration | 🟢 |
| fn | `as_uuid` | hkask-types::id::webid | crates/hkask-types/src/id/webid.rs:34 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_persona_with_namespace` | hkask-types::id::webid | crates/hkask-types/src/id/webid.rs:66 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_persona` | hkask-types::id::webid | crates/hkask-types/src/id/webid.rs:50 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_uuid` | hkask-types::id::webid | crates/hkask-types/src/id/webid.rs:27 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::id::webid | crates/hkask-types/src/id/webid.rs:20 | 🟢 Accessor/Constructor | 🟢 |
| fn | `redacted_display` | hkask-types::id::webid | crates/hkask-types/src/id/webid.rs:88 | 🔴 Core Logic | 🟢 |
| struct | `WebID` | hkask-types::id::webid | crates/hkask-types/src/id/webid.rs:14 | 🟡 Type Declaration | 🟢 |
| enum | `MdsCategory` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:108 | 🟡 Type Declaration | 🟢 |
| enum | `TemplateType` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:28 | 🟡 Type Declaration | 🟢 |
| fn | `add` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:194 | 🔴 Core Logic | 🟢 |
| fn | `as_spec_name` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:75 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:120 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_str` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:41 | 🟢 Accessor/Constructor | 🟢 |
| fn | `bootstrap` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:237 | 🔴 Core Logic | 🟢 |
| fn | `contains` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:208 | 🔴 Core Logic | 🟢 |
| fn | `file_extension` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:64 | 🔴 Core Logic | 🟢 |
| fn | `get` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:201 | 🔴 Core Logic | 🟢 |
| fn | `infer_from_extension` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:86 | 🔴 Core Logic | 🟢 |
| fn | `is_empty` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:231 | 🟢 Accessor/Constructor | 🟢 |
| fn | `len` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:225 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:149 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:185 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse_str` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:52 | 🔴 Core Logic | 🟢 |
| fn | `validate` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:215 | 🔴 Core Logic | 🟢 |
| fn | `with_citation` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:162 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_mds_category` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:170 | 🟢 Accessor/Constructor | 🟢 |
| struct | `HLexicon` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:178 | 🟡 Type Declaration | 🟢 |
| struct | `LexiconTerm` | hkask-types::lexicon | crates/hkask-types/src/lexicon.rs:133 | 🟡 Type Declaration | 🟢 |
| enum | `ActionType` | hkask-types::loops::actions | crates/hkask-types/src/loops/actions.rs:25 | 🟡 Type Declaration | 🔴 |
| fn | `new` | hkask-types::loops::actions | crates/hkask-types/src/loops/actions.rs:14 | 🟢 Accessor/Constructor | 🟢 |
| struct | `LoopAction` | hkask-types::loops::actions | crates/hkask-types/src/loops/actions.rs:7 | 🟡 Type Declaration | 🔴 |
| enum | `CurationInput` | hkask-types::loops::channels | crates/hkask-types/src/loops/channels.rs:76 | 🟡 Type Declaration | 🔴 |
| struct | `GoalTransitionEvent` | hkask-types::loops::channels | crates/hkask-types/src/loops/channels.rs:61 | 🟡 Type Declaration | 🔴 |
| struct | `RuntimeAlert` | hkask-types::loops::channels | crates/hkask-types/src/loops/channels.rs:19 | 🟡 Type Declaration | 🔴 |
| struct | `SpecEvent` | hkask-types::loops::channels | crates/hkask-types/src/loops/channels.rs:47 | 🟡 Type Declaration | 🔴 |
| struct | `ToolConsumptionEvent` | hkask-types::loops::channels | crates/hkask-types/src/loops/channels.rs:33 | 🟡 Type Declaration | 🔴 |
| enum | `LoopId` | hkask-types::loops::core | crates/hkask-types/src/loops/core.rs:11 | 🟡 Type Declaration | 🔴 |
| fn | `from_cycle` | hkask-types::loops::core | crates/hkask-types/src/loops/core.rs:101 | 🟢 Accessor/Constructor | 🟢 |
| struct | `LoopQuality` | hkask-types::loops::core | crates/hkask-types/src/loops/core.rs:73 | 🟡 Type Declaration | 🟢 |
| trait | `Loop` | hkask-types::loops::core | crates/hkask-types/src/loops/core.rs:41 | 🟡 Type Declaration | 🟢 |
| enum | `CuratorDirective` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:85 | 🟡 Type Declaration | 🔴 |
| fn | `agent_target` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:156 | 🔴 Core Logic | 🔴 |
| fn | `can_read` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:57 | 🔴 Core Logic | 🔴 |
| fn | `can_write` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:62 | 🔴 Core Logic | 🔴 |
| fn | `curator_id` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:52 | 🔴 Core Logic | 🔴 |
| fn | `is_metacognitive` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:173 | 🟢 Accessor/Constructor | 🔴 |
| fn | `issue_consolidation_token` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:71 | 🔴 Core Logic | 🔴 |
| fn | `new_test` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:35 | 🟢 Accessor/Constructor | 🔴 |
| fn | `system` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:46 | 🔴 Core Logic | 🟢 |
| fn | `variant_name` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:141 | 🔴 Core Logic | 🔴 |
| struct | `CuratorHandle` | hkask-types::loops::curation | crates/hkask-types/src/loops/curation.rs:30 | 🟡 Type Declaration | 🔴 |
| enum | `ExperienceClassification` | hkask-types::loops::episodic | crates/hkask-types/src/loops/episodic.rs:25 | 🟡 Type Declaration | 🔴 |
| fn | `default_confidence` | hkask-types::loops::episodic | crates/hkask-types/src/loops/episodic.rs:31 | 🔴 Core Logic | 🔴 |
| enum | `DeviationDirection` | hkask-types::loops::signals | crates/hkask-types/src/loops/signals.rs:173 | 🟡 Type Declaration | 🔴 |
| enum | `SignalMetric` | hkask-types::loops::signals | crates/hkask-types/src/loops/signals.rs:12 | 🟡 Type Declaration | 🔴 |
| fn | `as_str` | hkask-types::loops::signals | crates/hkask-types/src/loops/signals.rs:87 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_signal` | hkask-types::loops::signals | crates/hkask-types/src/loops/signals.rs:154 | 🟢 Accessor/Constructor | 🔴 |
| fn | `new` | hkask-types::loops::signals | crates/hkask-types/src/loops/signals.rs:134 | 🟢 Accessor/Constructor | 🟢 |
| struct | `Deviation` | hkask-types::loops::signals | crates/hkask-types/src/loops/signals.rs:147 | 🟡 Type Declaration | 🟢 |
| struct | `Signal` | hkask-types::loops::signals | crates/hkask-types/src/loops/signals.rs:125 | 🟡 Type Declaration | 🔴 |
| enum | `ComplexityTier` | hkask-types::ocr::config | crates/hkask-types/src/ocr/config.rs:26 | 🟡 Type Declaration | 🟢 |
| enum | `OcrBackend` | hkask-types::ocr::config | crates/hkask-types/src/ocr/config.rs:53 | 🟡 Type Declaration | 🟢 |
| fn | `classify` | hkask-types::ocr::config | crates/hkask-types/src/ocr/config.rs:125 | 🔴 Core Logic | 🟢 |
| fn | `label` | hkask-types::ocr::config | crates/hkask-types/src/ocr/config.rs:63 | 🔴 Core Logic | 🟢 |
| struct | `ComplexityScore` | hkask-types::ocr::config | crates/hkask-types/src/ocr/config.rs:41 | 🟡 Type Declaration | 🔴 |
| struct | `ThresholdConfig` | hkask-types::ocr::config | crates/hkask-types/src/ocr/config.rs:94 | 🟡 Type Declaration | 🔴 |
| enum | `PipelineError` | hkask-types::ocr::document | crates/hkask-types/src/ocr/document.rs:56 | 🟡 Type Declaration | 🟢 |
| fn | `compute_passed` | hkask-types::ocr::document | crates/hkask-types/src/ocr/document.rs:118 | 🔴 Core Logic | 🔴 |
| fn | `new` | hkask-types::ocr::document | crates/hkask-types/src/ocr/document.rs:126 | 🟢 Accessor/Constructor | 🟢 |
| struct | `CrossValidation` | hkask-types::ocr::document | crates/hkask-types/src/ocr/document.rs:30 | 🟡 Type Declaration | 🟢 |
| struct | `OcrResult` | hkask-types::ocr::document | crates/hkask-types/src/ocr/document.rs:9 | 🟡 Type Declaration | 🔴 |
| struct | `PageVerificationDetail` | hkask-types::ocr::document | crates/hkask-types/src/ocr/document.rs:148 | 🟡 Type Declaration | 🔴 |
| struct | `PipelineOutcome` | hkask-types::ocr::document | crates/hkask-types/src/ocr/document.rs:164 | 🟡 Type Declaration | 🟢 |
| struct | `VerificationReport` | hkask-types::ocr::document | crates/hkask-types/src/ocr/document.rs:97 | 🟡 Type Declaration | 🟢 |
| struct | `BackpressureSignal` | hkask-types::ports::cns | crates/hkask-types/src/ports/cns.rs:53 | 🟡 Type Declaration | 🔴 |
| struct | `ConsolidationOutcome` | hkask-types::ports::cns | crates/hkask-types/src/ports/cns.rs:38 | 🟡 Type Declaration | 🔴 |
| struct | `ConsolidationRequest` | hkask-types::ports::cns | crates/hkask-types/src/ports/cns.rs:21 | 🟡 Type Declaration | 🔴 |
| struct | `DepletionSignal` | hkask-types::ports::cns | crates/hkask-types/src/ports/cns.rs:45 | 🟡 Type Declaration | 🔴 |
| trait | `CircuitBreakerPort` | hkask-types::ports::cns | crates/hkask-types/src/ports/cns.rs:12 | 🟡 Type Declaration | 🔴 |
| trait | `CnsObserver` | hkask-types::ports::cns | crates/hkask-types/src/ports/cns.rs:61 | 🟡 Type Declaration | 🔴 |
| enum | `EmbeddingGenerationError` | hkask-types::ports::embedding | crates/hkask-types/src/ports/embedding.rs:5 | 🟡 Type Declaration | 🔴 |
| enum | `GitCasError` | hkask-types::ports::git_cas::error | crates/hkask-types/src/ports/git_cas/error.rs:15 | 🟡 Type Declaration | 🔴 |
| fn | `blob_count` | hkask-types::ports::git_cas::port | crates/hkask-types/src/ports/git_cas/port.rs:135 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-types::ports::git_cas::port | crates/hkask-types/src/ports/git_cas/port.rs:113 | 🟢 Accessor/Constructor | 🟢 |
| fn | `snapshot_history` | hkask-types::ports::git_cas::port | crates/hkask-types/src/ports/git_cas/port.rs:126 | 🔴 Core Logic | 🟢 |
| struct | `LogEntry` | hkask-types::ports::git_cas::port | crates/hkask-types/src/ports/git_cas/port.rs:34 | 🟡 Type Declaration | 🟢 |
| struct | `MockGitCas` | hkask-types::ports::git_cas::port | crates/hkask-types/src/ports/git_cas/port.rs:102 | 🟡 Type Declaration | 🟢 |
| struct | `VerificationReport` | hkask-types::ports::git_cas::port | crates/hkask-types/src/ports/git_cas/port.rs:18 | 🟡 Type Declaration | 🟢 |
| trait | `GitCASPort` | hkask-types::ports::git_cas::port | crates/hkask-types/src/ports/git_cas/port.rs:54 | 🟡 Type Declaration | 🟢 |
| enum | `SnapshotTrigger` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:157 | 🟡 Type Declaration | 🟢 |
| fn | `default_for` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:93 | 🔴 Core Logic | 🟢 |
| fn | `disabled` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:119 | 🔴 Core Logic | 🟢 |
| fn | `effective_policy` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:133 | 🔴 Core Logic | 🟢 |
| fn | `with_policy` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:106 | 🟢 Accessor/Constructor | 🟢 |
| struct | `RepoSnapshotPolicy` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:77 | 🟡 Type Declaration | 🟢 |
| struct | `RetentionPolicy` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:37 | 🟡 Type Declaration | 🟢 |
| struct | `RetentionTier` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:16 | 🟡 Type Declaration | 🟢 |
| struct | `SnapshotMetadata` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:142 | 🟡 Type Declaration | 🟢 |
| struct | `TripleEntry` | hkask-types::ports::git_cas::snapshot | crates/hkask-types/src/ports/git_cas/snapshot.rs:176 | 🟡 Type Declaration | 🟢 |
| enum | `DiffKind` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:219 | 🟡 Type Declaration | 🟢 |
| enum | `RepoId` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:131 | 🟡 Type Declaration | 🟢 |
| enum | `TreeEntryKind` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:201 | 🟡 Type Declaration | 🟢 |
| fn | `all` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:173 | 🔴 Core Logic | 🟢 |
| fn | `as_bytes` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:34 | 🟢 Accessor/Constructor | 🟢 |
| fn | `as_bytes` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:86 | 🟢 Accessor/Constructor | 🟢 |
| fn | `dir_name` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:155 | 🔴 Core Logic | 🟢 |
| fn | `from_blake3` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:25 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_bytes` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:77 | 🟢 Accessor/Constructor | 🟢 |
| fn | `null` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:95 | 🔴 Core Logic | 🟢 |
| struct | `CommitHash` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:69 | 🟡 Type Declaration | 🟢 |
| struct | `ContentHash` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:16 | 🟡 Type Declaration | 🟢 |
| struct | `FileDiff` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:208 | 🟡 Type Declaration | 🟢 |
| struct | `TreeEntry` | hkask-types::ports::git_cas::types | crates/hkask-types/src/ports/git_cas/types.rs:190 | 🟡 Type Declaration | 🟢 |
| struct | `InferenceStreamChunk` | hkask-types::ports::inference_port | crates/hkask-types/src/ports/inference_port.rs:88 | 🟡 Type Declaration | 🔴 |
| trait | `InferencePort` | hkask-types::ports::inference_port | crates/hkask-types/src/ports/inference_port.rs:12 | 🟡 Type Declaration | 🔴 |
| enum | `InferenceError` | hkask-types::ports::inference_types | crates/hkask-types/src/ports/inference_types.rs:7 | 🟡 Type Declaration | 🟢 |
| fn | `compute_confidence` | hkask-types::ports::inference_types | crates/hkask-types/src/ports/inference_types.rs:49 | 🔴 Core Logic | 🟢 |
| struct | `InferenceResult` | hkask-types::ports::inference_types | crates/hkask-types/src/ports/inference_types.rs:76 | 🟡 Type Declaration | 🟢 |
| struct | `InferenceUsage` | hkask-types::ports::inference_types | crates/hkask-types/src/ports/inference_types.rs:22 | 🟡 Type Declaration | 🟢 |
| struct | `StructuredToolCall` | hkask-types::ports::inference_types | crates/hkask-types/src/ports/inference_types.rs:67 | 🟡 Type Declaration | 🟢 |
| struct | `TokenProbability` | hkask-types::ports::inference_types | crates/hkask-types/src/ports/inference_types.rs:30 | 🟡 Type Declaration | 🟢 |
| struct | `TokenProb` | hkask-types::ports::inference_types | crates/hkask-types/src/ports/inference_types.rs:38 | 🟡 Type Declaration | 🟢 |
| enum | `RegistryError` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:267 | 🟡 Type Declaration | 🟢 |
| enum | `SkillZone` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:61 | 🟡 Type Declaration | 🟢 |
| fn | `as_str` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:72 | 🟢 Accessor/Constructor | 🟢 |
| fn | `can_nest` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:51 | 🔴 Core Logic | 🟢 |
| fn | `compute_content_hash` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:243 | 🔴 Core Logic | 🟢 |
| fn | `directory` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:93 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:129 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse_qualified_id` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:229 | 🔴 Core Logic | 🟢 |
| fn | `parse_str` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:82 | 🔴 Core Logic | 🟢 |
| fn | `qualified_id` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:218 | 🔴 Core Logic | 🟢 |
| fn | `validate` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:28 | 🔴 Core Logic | 🟢 |
| fn | `with_content_hash` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:181 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_flow_def` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:157 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_know_act` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:165 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_namespace` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:208 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_polarity` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:173 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_visibility` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:190 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_word_act` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:149 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_zone` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:199 | 🟢 Accessor/Constructor | 🟢 |
| struct | `RegistryEntry` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:11 | 🟡 Type Declaration | 🟢 |
| struct | `Skill` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:102 | 🟡 Type Declaration | 🟢 |
| trait | `BundleRegistryIndex` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:297 | 🟡 Type Declaration | 🟢 |
| trait | `RegistryIndex` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:307 | 🟡 Type Declaration | 🟢 |
| trait | `SkillRegistryIndex` | hkask-types::ports::registry | crates/hkask-types/src/ports/registry.rs:275 | 🟡 Type Declaration | 🟢 |
| enum | `ToolPortError` | hkask-types::ports::tool | crates/hkask-types/src/ports/tool.rs:7 | 🟡 Type Declaration | 🔴 |
| struct | `ToolInfo` | hkask-types::ports::tool | crates/hkask-types/src/ports/tool.rs:41 | 🟡 Type Declaration | 🔴 |
| trait | `ToolPort` | hkask-types::ports::tool | crates/hkask-types/src/ports/tool.rs:21 | 🟡 Type Declaration | 🔴 |
| fn | `default_r7_bots` | hkask-types::r7 | crates/hkask-types/src/r7.rs:90 | 🔴 Core Logic | 🟢 |
| fn | `webid` | hkask-types::r7 | crates/hkask-types/src/r7.rs:50 | 🔴 Core Logic | 🟢 |
| struct | `R7BotIdentity` | hkask-types::r7 | crates/hkask-types/src/r7.rs:17 | 🟡 Type Declaration | 🟢 |
| enum | `SecretRef` | hkask-types::secret | crates/hkask-types/src/secret.rs:22 | 🟡 Type Declaration | 🔴 |
| fn | `as_bytes` | hkask-types::secret | crates/hkask-types/src/secret.rs:137 | 🟢 Accessor/Constructor | 🟢 |
| fn | `derived` | hkask-types::secret | crates/hkask-types/src/secret.rs:69 | 🔴 Core Logic | 🟢 |
| fn | `env` | hkask-types::secret | crates/hkask-types/src/secret.rs:54 | 🔴 Core Logic | 🔴 |
| fn | `generated` | hkask-types::secret | crates/hkask-types/src/secret.rs:79 | 🔴 Core Logic | 🔴 |
| fn | `keychain` | hkask-types::secret | crates/hkask-types/src/secret.rs:59 | 🔴 Core Logic | 🔴 |
| fn | `new` | hkask-types::secret | crates/hkask-types/src/secret.rs:133 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ZeroizingSecret` | hkask-types::secret | crates/hkask-types/src/secret.rs:130 | 🟡 Type Declaration | 🔴 |
| enum | `BoundaryClassification` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:152 | 🟡 Type Declaration | 🟢 |
| enum | `DataCategory` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:21 | 🟡 Type Declaration | 🟢 |
| fn | `access_required` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:179 | 🔴 Core Logic | 🟢 |
| fn | `as_str` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:48 | 🟢 Accessor/Constructor | 🟢 |
| fn | `classify` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:289 | 🔴 Core Logic | 🟢 |
| fn | `default_visibility` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:120 | 🔴 Core Logic | 🟢 |
| fn | `grant_consent` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:335 | 🔴 Core Logic | 🟢 |
| fn | `hkask_default` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:210 | 🔴 Core Logic | 🟢 |
| fn | `is_category_public` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:267 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_category_shared` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:255 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_sovereign` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:240 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_typically_sovereign` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:90 | 🟢 Accessor/Constructor | 🟢 |
| fn | `label` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:165 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:322 | 🟢 Accessor/Constructor | 🟢 |
| fn | `parse` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:71 | 🔴 Core Logic | 🟢 |
| fn | `requires_affirmative_consent` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:276 | 🔴 Core Logic | 🟢 |
| fn | `revoke_consent` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:344 | 🔴 Core Logic | 🟢 |
| struct | `DataSovereigntyBoundary` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:135 | 🟡 Type Declaration | 🟢 |
| struct | `UserSovereigntyState` | hkask-types::sovereignty | crates/hkask-types/src/sovereignty.rs:309 | 🟡 Type Declaration | 🟢 |
| struct | `LLMParameters` | hkask-types::template | crates/hkask-types/src/template.rs:14 | 🟡 Type Declaration | 🔴 |
| struct | `TemplateCrate` | hkask-types::template | crates/hkask-types/src/template.rs:116 | 🟡 Type Declaration | 🔴 |
| struct | `TemplateFile` | hkask-types::template | crates/hkask-types/src/template.rs:107 | 🟡 Type Declaration | 🔴 |
| struct | `TemplateInvocation` | hkask-types::template | crates/hkask-types/src/template.rs:137 | 🟡 Type Declaration | 🔴 |
| fn | `blake3_hash` | hkask-types::text | crates/hkask-types/src/text.rs:17 | 🔴 Core Logic | 🟢 |
| fn | `now_rfc3339` | hkask-types::time | crates/hkask-types/src/time.rs:18 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-types::transcript | crates/hkask-types/src/transcript.rs:82 | 🟢 Accessor/Constructor | 🟢 |
| fn | `segment_at_ms` | hkask-types::transcript | crates/hkask-types/src/transcript.rs:122 | 🔴 Core Logic | 🟢 |
| fn | `word_at_ms` | hkask-types::transcript | crates/hkask-types/src/transcript.rs:110 | 🔴 Core Logic | 🟢 |
| fn | `word_count` | hkask-types::transcript | crates/hkask-types/src/transcript.rs:100 | 🔴 Core Logic | 🟢 |
| struct | `TimedWord` | hkask-types::transcript | crates/hkask-types/src/transcript.rs:15 | 🟡 Type Declaration | 🟢 |
| struct | `TranscriptBundle` | hkask-types::transcript | crates/hkask-types/src/transcript.rs:43 | 🟡 Type Declaration | 🟢 |
| struct | `TranscriptSegment` | hkask-types::transcript | crates/hkask-types/src/transcript.rs:29 | 🟡 Type Declaration | 🟢 |
| enum | `Visibility` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:33 | 🟡 Type Declaration | 🟢 |
| fn | `as_str` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:44 | 🟢 Accessor/Constructor | 🟢 |
| fn | `decay` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:287 | 🔴 Core Logic | 🟢 |
| fn | `episodic` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:110 | 🔴 Core Logic | 🟢 |
| fn | `full` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:253 | 🔴 Core Logic | 🟢 |
| fn | `is_current` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:351 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_episodic` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:150 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_semantic` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:159 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:244 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:339 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:96 | 🟢 Accessor/Constructor | 🟢 |
| fn | `now` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:327 | 🔴 Core Logic | 🟢 |
| fn | `parse_str` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:55 | 🔴 Core Logic | 🟢 |
| fn | `semantic` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:124 | 🔴 Core Logic | 🟢 |
| fn | `superseded` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:360 | 🔴 Core Logic | 🟢 |
| fn | `to_semantic` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:137 | 🟢 Accessor/Constructor | 🟢 |
| fn | `value` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:274 | 🔴 Core Logic | 🟢 |
| fn | `with_perspective` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:168 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_visibility` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:185 | 🟢 Accessor/Constructor | 🟢 |
| fn | `without_perspective` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:222 | 🔴 Core Logic | 🟢 |
| struct | `AccessControl` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:83 | 🟡 Type Declaration | 🟢 |
| struct | `Confidence` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:235 | 🟡 Type Declaration | 🟢 |
| struct | `TemporalBounds` | hkask-types::visibility | crates/hkask-types/src/visibility.rs:316 | 🟡 Type Declaration | 🟢 |
| fn | `to_elevenlabs_voice` | hkask-types::voice | crates/hkask-types/src/voice.rs:121 | 🟢 Accessor/Constructor | 🟢 |
| fn | `to_tts_description` | hkask-types::voice | crates/hkask-types/src/voice.rs:76 | 🟢 Accessor/Constructor | 🟢 |
| struct | `VoiceDesign` | hkask-types::voice | crates/hkask-types/src/voice.rs:15 | 🟡 Type Declaration | 🟢 |
| enum | `ChainId` | hkask-types::wallet::chain | crates/hkask-types/src/wallet/chain.rs:19 | 🟡 Type Declaration | 🟢 |
| enum | `PrivacyMode` | hkask-types::wallet::chain | crates/hkask-types/src/wallet/chain.rs:60 | 🟡 Type Declaration | 🟢 |
| fn | `as_bytes` | hkask-types::wallet::chain | crates/hkask-types/src/wallet/chain.rs:103 | 🟢 Accessor/Constructor | 🟢 |
| fn | `from_bytes` | hkask-types::wallet::chain | crates/hkask-types/src/wallet/chain.rs:99 | 🟢 Accessor/Constructor | 🟢 |
| struct | `DepositAddress` | hkask-types::wallet::chain | crates/hkask-types/src/wallet/chain.rs:130 | 🟡 Type Declaration | 🔴 |
| struct | `DepositReference` | hkask-types::wallet::chain | crates/hkask-types/src/wallet/chain.rs:154 | 🟡 Type Declaration | 🔴 |
| struct | `Ed25519PublicKey` | hkask-types::wallet::chain | crates/hkask-types/src/wallet/chain.rs:96 | 🟡 Type Declaration | 🔴 |
| struct | `TxHash` | hkask-types::wallet::chain | crates/hkask-types/src/wallet/chain.rs:118 | 🟡 Type Declaration | 🔴 |
| enum | `WalletError` | hkask-types::wallet::error | crates/hkask-types/src/wallet/error.rs:17 | 🟡 Type Declaration | 🟢 |
| enum | `EncumbranceStatus` | hkask-types::wallet::keys | crates/hkask-types/src/wallet/keys.rs:100 | 🟡 Type Declaration | 🟢 |
| fn | `is_active` | hkask-types::wallet::keys | crates/hkask-types/src/wallet/keys.rs:160 | 🟢 Accessor/Constructor | 🟢 |
| fn | `is_expired` | hkask-types::wallet::keys | crates/hkask-types/src/wallet/keys.rs:57 | 🟢 Accessor/Constructor | 🟢 |
| fn | `remaining_rj` | hkask-types::wallet::keys | crates/hkask-types/src/wallet/keys.rs:155 | 🔴 Core Logic | 🟢 |
| fn | `remaining_rj` | hkask-types::wallet::keys | crates/hkask-types/src/wallet/keys.rs:62 | 🔴 Core Logic | 🟢 |
| struct | `ApiKeyCapability` | hkask-types::wallet::keys | crates/hkask-types/src/wallet/keys.rs:36 | 🟡 Type Declaration | 🟢 |
| struct | `ApiKeyMaterial` | hkask-types::wallet::keys | crates/hkask-types/src/wallet/keys.rs:77 | 🟡 Type Declaration | 🟢 |
| struct | `Encumbrance` | hkask-types::wallet::keys | crates/hkask-types/src/wallet/keys.rs:140 | 🟡 Type Declaration | 🟢 |
| struct | `RateLimitConfig` | hkask-types::wallet::keys | crates/hkask-types/src/wallet/keys.rs:16 | 🟡 Type Declaration | 🟢 |
| enum | `PriceFeedConfig` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:67 | 🟡 Type Declaration | 🟢 |
| enum | `TransactionType` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:169 | 🟡 Type Declaration | 🟢 |
| fn | `as_u64` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:37 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:32 | 🟢 Accessor/Constructor | 🟢 |
| fn | `saturating_add` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:42 | 🔴 Core Logic | 🟢 |
| fn | `saturating_sub` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:47 | 🔴 Core Logic | 🟢 |
| struct | `RJoule` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:25 | 🟡 Type Declaration | 🟢 |
| struct | `WalletBalance` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:143 | 🟡 Type Declaration | 🟢 |
| struct | `WalletConfig` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:107 | 🟡 Type Declaration | 🟢 |
| struct | `WalletTransaction` | hkask-types::wallet::types | crates/hkask-types/src/wallet/types.rs:215 | 🟡 Type Declaration | 🟢 |

| hkask-wallet | 66 | 62 | 4 | 93% | 104 |

### hkask-wallet

| Kind | Item | Module | Location | Risk Tier | REQ |
|------|------|--------|----------|-----------|-----|
| fn | `new` | hkask-wallet::chain | crates/hkask-wallet/src/chain.rs:33 | 🟢 Accessor/Constructor | 🟢 |
| struct | `DepositEvent` | hkask-wallet::chain | crates/hkask-wallet/src/chain.rs:21 | 🟡 Type Declaration | 🔴 |
| trait | `ChainPort` | hkask-wallet::chain | crates/hkask-wallet/src/chain.rs:65 | 🟡 Type Declaration | 🔴 |
| fn | `new_mainnet` | hkask-wallet::hedera | crates/hkask-wallet/src/hedera.rs:225 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new_testnet` | hkask-wallet::hedera | crates/hkask-wallet/src/hedera.rs:215 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-wallet::hedera | crates/hkask-wallet/src/hedera.rs:149 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_event_sink` | hkask-wallet::hedera | crates/hkask-wallet/src/hedera.rs:176 | 🟢 Accessor/Constructor | 🟢 |
| struct | `HederaPort` | hkask-wallet::hedera | crates/hkask-wallet/src/hedera.rs:128 | 🟡 Type Declaration | 🟢 |
| fn | `in_cooldown` | hkask-wallet::hinkal | crates/hkask-wallet/src/hinkal.rs:659 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-wallet::hinkal | crates/hkask-wallet/src/hinkal.rs:189 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_default_base` | hkask-wallet::hinkal | crates/hkask-wallet/src/hinkal.rs:235 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_event_sink` | hkask-wallet::hinkal | crates/hkask-wallet/src/hinkal.rs:241 | 🟢 Accessor/Constructor | 🟢 |
| struct | `HinkalPort` | hkask-wallet::hinkal | crates/hkask-wallet/src/hinkal.rs:156 | 🟡 Type Declaration | 🟢 |
| fn | `create_key` | hkask-wallet::issuer | crates/hkask-wallet/src/issuer.rs:100 | 🔴 Core Logic | 🟢 |
| fn | `list_keys` | hkask-wallet::issuer | crates/hkask-wallet/src/issuer.rs:197 | 🔴 Core Logic | 🟢 |
| fn | `new` | hkask-wallet::issuer | crates/hkask-wallet/src/issuer.rs:55 | 🟢 Accessor/Constructor | 🟢 |
| fn | `revoke_key` | hkask-wallet::issuer | crates/hkask-wallet/src/issuer.rs:176 | 🔴 Core Logic | 🟢 |
| fn | `with_event_sink` | hkask-wallet::issuer | crates/hkask-wallet/src/issuer.rs:70 | 🟢 Accessor/Constructor | 🟢 |
| struct | `ApiKeyIssuer` | hkask-wallet::issuer | crates/hkask-wallet/src/issuer.rs:38 | 🟡 Type Declaration | 🟢 |
| fn | `build` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:57 | 🟢 Accessor/Constructor | 🟢 |
| fn | `can_afford` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:849 | 🔴 Core Logic | 🟢 |
| fn | `consume` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:997 | 🔴 Core Logic | 🟢 |
| fn | `emit_chain_error_for_actor` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:165 | 🔴 Core Logic | 🟢 |
| fn | `emit_chain_error` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:186 | 🔴 Core Logic | 🟢 |
| fn | `emit_key_alert` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:136 | 🔴 Core Logic | 🟢 |
| fn | `encumber` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:947 | 🔴 Core Logic | 🟢 |
| fn | `ensure_wallet` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:228 | 🔴 Core Logic | 🟢 |
| fn | `gas_to_rjoules` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:789 | 🔴 Core Logic | 🟢 |
| fn | `generate_deposit_reference` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:905 | 🔴 Core Logic | 🟢 |
| fn | `get_api_key` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:220 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_balance` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:200 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_deposit_address` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:752 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_encumbrance` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:1008 | 🟢 Accessor/Constructor | 🟢 |
| fn | `get_transactions` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:233 | 🟢 Accessor/Constructor | 🟢 |
| fn | `price_feed` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:95 | 🔴 Core Logic | 🟢 |
| fn | `release_encumbrance` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:975 | 🔴 Core Logic | 🟢 |
| fn | `reserve_rjoules` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:861 | 🔴 Core Logic | 🟢 |
| fn | `rjoules_to_gas` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:801 | 🔴 Core Logic | 🟢 |
| fn | `settle_rjoules` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:881 | 🔴 Core Logic | 🟢 |
| fn | `with_event_sink` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:82 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_price_feed` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:89 | 🟢 Accessor/Constructor | 🟢 |
| struct | `WalletManager` | hkask-wallet::manager | crates/hkask-wallet/src/manager.rs:35 | 🟡 Type Declaration | 🟢 |
| fn | `estimate_withdrawal_fee` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:512 | 🔴 Core Logic | 🟢 |
| fn | `from_env` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:109 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:119 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:203 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:312 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:56 | 🟢 Accessor/Constructor | 🟢 |
| fn | `resolve_price_feed` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:435 | 🔴 Core Logic | 🟢 |
| struct | `CoinGeckoPriceFeed` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:197 | 🟡 Type Declaration | 🟢 |
| struct | `CompositePriceFeed` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:302 | 🟡 Type Declaration | 🟢 |
| struct | `EodhdPriceFeed` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:99 | 🟡 Type Declaration | 🟢 |
| struct | `ExchangeRate` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:29 | 🟡 Type Declaration | 🟢 |
| struct | `StaticPriceFeed` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:52 | 🟡 Type Declaration | 🟢 |
| struct | `WithdrawalFee` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:494 | 🟡 Type Declaration | 🟢 |
| trait | `PriceFeed` | hkask-wallet::price_feed | crates/hkask-wallet/src/price_feed.rs:42 | 🟡 Type Declaration | 🟢 |
| struct | `ShieldedTransfer` | hkask-wallet::privacy | crates/hkask-wallet/src/privacy.rs:17 | 🟡 Type Declaration | 🔴 |
| trait | `PrivacyPort` | hkask-wallet::privacy | crates/hkask-wallet/src/privacy.rs:45 | 🟡 Type Declaration | 🔴 |
| fn | `sign_capability` | hkask-wallet::signing | crates/hkask-wallet/src/signing.rs:117 | 🔴 Core Logic | 🟢 |
| fn | `sign_message` | hkask-wallet::signing | crates/hkask-wallet/src/signing.rs:87 | 🔴 Core Logic | 🟢 |
| fn | `sign_withdrawal` | hkask-wallet::signing | crates/hkask-wallet/src/signing.rs:77 | 🔴 Core Logic | 🟢 |
| fn | `new_devnet` | hkask-wallet::solana | crates/hkask-wallet/src/solana.rs:165 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new_mainnet` | hkask-wallet::solana | crates/hkask-wallet/src/solana.rs:174 | 🟢 Accessor/Constructor | 🟢 |
| fn | `new` | hkask-wallet::solana | crates/hkask-wallet/src/solana.rs:85 | 🟢 Accessor/Constructor | 🟢 |
| fn | `with_event_sink` | hkask-wallet::solana | crates/hkask-wallet/src/solana.rs:123 | 🟢 Accessor/Constructor | 🟢 |
| struct | `SolanaPort` | hkask-wallet::solana | crates/hkask-wallet/src/solana.rs:64 | 🟡 Type Declaration | 🟢 |


---

## Totals

| Metric | Value |
|--------|-------|
| Total public items | 2465 |
| Covered (🟢) | 2055 |
| Uncovered (🔴) | 410 |
| Overall coverage | 83% |
| Total REQ-tagged tests | 2279 |
| Crates analyzed | 26 |
