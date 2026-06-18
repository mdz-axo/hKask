# Public Seam Priority List

**Generated:** 2026-06-18T09:17:52Z
**Source:** `scripts/public-seam-inventory.sh`
**Purpose:** Top high-risk uncovered public items requiring REQ-tagged tests.

Items are classified as **high risk** when they are:
- API route handlers (`hkask-api`)
- MCP tool handlers (`hkask-mcp-*` servers)
- Core logic functions in other crates (non-accessor/constructor patterns)

Accessors, constructors, and type declarations are excluded — they are low/medium
risk and typically covered by struct-level or integration tests.

---

## Top High-Risk Uncovered Items (top 100)

| # | Crate | Kind | Item | Module | Location | Category |
|---|-------|------|------|--------|----------|----------|
| 1 | hkask-acp | fn | `for_testing` | hkask-acp::main_impl | crates/hkask-acp/src/main_impl.rs:143 | Core Logic |
| 2 | hkask-adapter | fn | `accrue_cost` | hkask-adapter::endpoint_lifecycle | crates/hkask-adapter/src/endpoint_lifecycle.rs:159 | Core Logic |
| 3 | hkask-adapter | fn | `baseten` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:143 | Core Logic |
| 4 | hkask-adapter | fn | `baseten` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:184 | Core Logic |
| 5 | hkask-adapter | fn | `can_compose` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:96 | Core Logic |
| 6 | hkask-adapter | fn | `cost_accrued` | hkask-adapter::adapter_port | crates/hkask-adapter/src/adapter_port.rs:134 | Core Logic |
| 7 | hkask-adapter | fn | `count` | hkask-adapter::adapter_store | crates/hkask-adapter/src/adapter_store.rs:409 | Core Logic |
| 8 | hkask-adapter | fn | `deepinfra` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:193 | Core Logic |
| 9 | hkask-adapter | fn | `delete` | hkask-adapter::adapter_store | crates/hkask-adapter/src/adapter_store.rs:394 | Core Logic |
| 10 | hkask-adapter | fn | `drain_all_owner` | hkask-adapter::adapter_router | crates/hkask-adapter/src/adapter_router.rs:841 | Core Logic |
| 11 | hkask-adapter | fn | `elapsed_seconds` | hkask-adapter::endpoint_lifecycle | crates/hkask-adapter/src/endpoint_lifecycle.rs:171 | Core Logic |
| 12 | hkask-adapter | fn | `endpoint_id` | hkask-adapter::adapter_router | crates/hkask-adapter/src/adapter_router.rs:1264 | Core Logic |
| 13 | hkask-adapter | fn | `estimated_cost_for_hours` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:59 | Core Logic |
| 14 | hkask-adapter | fn | `estimated_setup_cost` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:64 | Core Logic |
| 15 | hkask-adapter | fn | `list_owner` | hkask-adapter::adapter_store | crates/hkask-adapter/src/adapter_store.rs:349 | Core Logic |
| 16 | hkask-adapter | fn | `migrate` | hkask-adapter::adapter_store | crates/hkask-adapter/src/adapter_store.rs:167 | Core Logic |
| 17 | hkask-adapter | fn | `parse` | hkask-adapter::expertise | crates/hkask-adapter/src/expertise.rs:50 | Core Logic |
| 18 | hkask-adapter | fn | `phase` | hkask-adapter::adapter_port | crates/hkask-adapter/src/adapter_port.rs:126 | Core Logic |
| 19 | hkask-adapter | fn | `repository_id` | hkask-adapter::adapter_store | crates/hkask-adapter/src/adapter_store.rs:39 | Core Logic |
| 20 | hkask-adapter | fn | `runpod` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:132 | Core Logic |
| 21 | hkask-adapter | fn | `runpod` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:170 | Core Logic |
| 22 | hkask-adapter | fn | `select_provider` | hkask-adapter::adapter_router | crates/hkask-adapter/src/adapter_router.rs:787 | Core Logic |
| 23 | hkask-adapter | fn | `store` | hkask-adapter::adapter_store | crates/hkask-adapter/src/adapter_store.rs:215 | Core Logic |
| 24 | hkask-adapter | fn | `teardown` | hkask-adapter::adapter_router | crates/hkask-adapter/src/adapter_router.rs:1243 | Core Logic |
| 25 | hkask-adapter | fn | `time_until_budget_exceeded` | hkask-adapter::endpoint_lifecycle | crates/hkask-adapter/src/endpoint_lifecycle.rs:186 | Core Logic |
| 26 | hkask-adapter | fn | `together` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:121 | Core Logic |
| 27 | hkask-adapter | fn | `together` | hkask-adapter::provider_cost | crates/hkask-adapter/src/provider_cost.rs:157 | Core Logic |
| 28 | hkask-adapter | fn | `transition` | hkask-adapter::endpoint_lifecycle | crates/hkask-adapter/src/endpoint_lifecycle.rs:130 | Core Logic |
| 29 | hkask-adapter | fn | `validate_base_model` | hkask-adapter::adapter_config | crates/hkask-adapter/src/adapter_config.rs:69 | Core Logic |
| 30 | hkask-agents | fn | `a2a_runtime` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:439 | Core Logic |
| 31 | hkask-agents | fn | `activate` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:327 | Core Logic |
| 32 | hkask-agents | fn | `calibrate_from_history` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:76 | Core Logic |
| 33 | hkask-agents | fn | `can_access` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:108 | Core Logic |
| 34 | hkask-agents | fn | `can_transition_to` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:61 | Core Logic |
| 35 | hkask-agents | fn | `cancel_token` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:167 | Core Logic |
| 36 | hkask-agents | fn | `capability_resources` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:170 | Core Logic |
| 37 | hkask-agents | fn | `check_conditions` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:124 | Core Logic |
| 38 | hkask-agents | fn | `check_operation` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:127 | Core Logic |
| 39 | hkask-agents | fn | `check_persona_constraints` | hkask-agents::curator::persona_filter | crates/hkask-agents/src/curator/persona_filter.rs:34 | Core Logic |
| 40 | hkask-agents | fn | `check_sovereignty` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:194 | Core Logic |
| 41 | hkask-agents | fn | `check_sovereignty` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:632 | Core Logic |
| 42 | hkask-agents | fn | `classified_episodic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:135 | Core Logic |
| 43 | hkask-agents | fn | `context` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:121 | Core Logic |
| 44 | hkask-agents | fn | `context` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:184 | Core Logic |
| 45 | hkask-agents | fn | `correlation_id` | hkask-agents::a2a::mod | crates/hkask-agents/src/a2a/mod.rs:254 | Core Logic |
| 46 | hkask-agents | fn | `curation_loop` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:164 | Core Logic |
| 47 | hkask-agents | fn | `curator_handle` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:134 | Core Logic |
| 48 | hkask-agents | fn | `deactivate` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:366 | Core Logic |
| 49 | hkask-agents | fn | `decompose_prompt` | hkask-agents::prompt_analysis | crates/hkask-agents/src/prompt_analysis.rs:585 | Core Logic |
| 50 | hkask-agents | fn | `default_tick_interval` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:64 | Core Logic |
| 51 | hkask-agents | fn | `delegate` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:409 | Core Logic |
| 52 | hkask-agents | fn | `emit_pod_activated` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:53 | Core Logic |
| 53 | hkask-agents | fn | `emit_pod_deactivated` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:65 | Core Logic |
| 54 | hkask-agents | fn | `emit_pod_event` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:20 | Core Logic |
| 55 | hkask-agents | fn | `emit_pod_registered` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:40 | Core Logic |
| 56 | hkask-agents | fn | `enter_chat_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:505 | Core Logic |
| 57 | hkask-agents | fn | `enter_server_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:470 | Core Logic |
| 58 | hkask-agents | fn | `episodic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:173 | Core Logic |
| 59 | hkask-agents | fn | `episodic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:80 | Core Logic |
| 60 | hkask-agents | fn | `episodic_storage_budget` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:222 | Core Logic |
| 61 | hkask-agents | fn | `episodic_storage_usage` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:210 | Core Logic |
| 62 | hkask-agents | fn | `exit_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:530 | Core Logic |
| 63 | hkask-agents | fn | `gas_cap` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:105 | Core Logic |
| 64 | hkask-agents | fn | `gas_remaining` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:78 | Core Logic |
| 65 | hkask-agents | fn | `generate_summary` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:325 | Core Logic |
| 66 | hkask-agents | fn | `grant` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:69 | Core Logic |
| 67 | hkask-agents | fn | `grant_consent` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:243 | Core Logic |
| 68 | hkask-agents | fn | `handle` | hkask-agents::curator::context | crates/hkask-agents/src/curator/context.rs:95 | Core Logic |
| 69 | hkask-agents | fn | `in_memory` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:164 | Core Logic |
| 70 | hkask-agents | fn | `in_memory_unchecked` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:180 | Core Logic |
| 71 | hkask-agents | fn | `inference_port` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:152 | Core Logic |
| 72 | hkask-agents | fn | `inference_port` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:200 | Core Logic |
| 73 | hkask-agents | fn | `invoke_tool` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:334 | Core Logic |
| 74 | hkask-agents | fn | `message_type` | hkask-agents::a2a::mod | crates/hkask-agents/src/a2a/mod.rs:269 | Core Logic |
| 75 | hkask-agents | fn | `metacognition` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:174 | Core Logic |
| 76 | hkask-agents | fn | `recall_episodic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:193 | Core Logic |
| 77 | hkask-agents | fn | `recall_semantic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:298 | Core Logic |
| 78 | hkask-agents | fn | `require_sovereignty` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:124 | Core Logic |
| 79 | hkask-agents | fn | `restore_cursor` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:149 | Core Logic |
| 80 | hkask-agents | fn | `revoke` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:80 | Core Logic |
| 81 | hkask-agents | fn | `revoke_consent` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:275 | Core Logic |
| 82 | hkask-agents | fn | `semantic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:106 | Core Logic |
| 83 | hkask-agents | fn | `semantic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:189 | Core Logic |
| 84 | hkask-agents | fn | `semantic_storage_usage` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:315 | Core Logic |
| 85 | hkask-agents | fn | `spec_curator` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:197 | Core Logic |
| 86 | hkask-agents | fn | `state` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:447 | Core Logic |
| 87 | hkask-agents | fn | `store` | hkask-agents::registry_loader | crates/hkask-agents/src/registry_loader.rs:389 | Core Logic |
| 88 | hkask-agents | fn | `store_episodic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:169 | Core Logic |
| 89 | hkask-agents | fn | `store_episodic_experience` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:235 | Core Logic |
| 90 | hkask-agents | fn | `store_semantic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:275 | Core Logic |
| 91 | hkask-agents | fn | `strip_forbidden_patterns` | hkask-agents::curator::persona_filter | crates/hkask-agents/src/curator/persona_filter.rs:71 | Core Logic |
| 92 | hkask-agents | fn | `sync_gas_state` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:97 | Core Logic |
| 93 | hkask-agents | fn | `token_usage` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:87 | Core Logic |
| 94 | hkask-agents | fn | `validate_fields` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:178 | Core Logic |
| 95 | hkask-agents | fn | `visit` | hkask-agents::a2a::mod | crates/hkask-agents/src/a2a/mod.rs:186 | Core Logic |
| 96 | hkask-agents | fn | `voice_description` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:592 | Core Logic |
| 97 | hkask-agents | fn | `webid` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:161 | Core Logic |
| 98 | hkask-api | fn | `a2a_router` | hkask-api::routes::a2a | crates/hkask-api/src/routes/a2a.rs:86 | API Route Handler |
| 99 | hkask-api | fn | `backup_router` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:211 | API Route Handler |
| 100 | hkask-api | fn | `bots_router` | hkask-api::routes::bots | crates/hkask-api/src/routes/bots.rs:13 | API Route Handler |

---

## Per-Crate High-Risk Uncovered Count

| Crate | High-Risk Uncovered |
|-------|--------------------|
| hkask-types | 157 |
| hkask-storage | 135 |
| hkask-agents | 68 |
| hkask-cns | 55 |
| hkask-memory | 43 |
| hkask-cli | 43 |
| hkask-test-harness | 37 |
| hkask-services | 35 |
| hkask-mcp-research | 35 |
| hkask-services-kanban | 32 |
| hkask-keystore | 32 |
| hkask-templates | 31 |
| hkask-services-context | 30 |
| hkask-mcp | 30 |
| hkask-adapter | 28 |
| hkask-mcp-companies | 26 |
| hkask-wallet | 24 |
| hkask-api | 24 |
| hkask-improv | 21 |
| hkask-mcp-training | 16 |
| hkask-services-core | 14 |
| hkask-inference | 14 |
| hkask-condenser | 14 |
| hkask-services-backup | 13 |
| hkask-services-kata | 12 |
| hkask-mcp-docproc | 12 |
| hkask-services-skill | 7 |
| hkask-mcp-media | 7 |
| hkask-services-onboarding | 4 |
| hkask-services-embed | 4 |
| hkask-services-discover | 3 |
| hkask-services-verification | 2 |
| hkask-services-sovereignty | 2 |
| hkask-rsolidity-macros | 2 |
| hkask-mcp-spec | 2 |
| hkask-services-inference-svc | 1 |
| hkask-services-classify | 1 |
| hkask-rsolidity | 1 |
| hkask-mcp-communication | 1 |
| hkask-communication | 1 |
| hkask-acp | 1 |

**Total high-risk uncovered:** 1020
