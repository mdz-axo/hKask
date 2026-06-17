# Public Seam Priority List

**Generated:** 2026-06-17T03:44:21Z
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
| 1 | hkask-agents | fn | `acp_runtime` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:439 | Core Logic |
| 2 | hkask-agents | fn | `activate` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:327 | Core Logic |
| 3 | hkask-agents | fn | `calibrate_from_history` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:76 | Core Logic |
| 4 | hkask-agents | fn | `can_access` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:108 | Core Logic |
| 5 | hkask-agents | fn | `cancel_token` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:167 | Core Logic |
| 6 | hkask-agents | fn | `can_transition_to` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:61 | Core Logic |
| 7 | hkask-agents | fn | `capability_resources` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:170 | Core Logic |
| 8 | hkask-agents | fn | `check_conditions` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:124 | Core Logic |
| 9 | hkask-agents | fn | `check_operation` | hkask-agents::sovereignty | crates/hkask-agents/src/sovereignty.rs:127 | Core Logic |
| 10 | hkask-agents | fn | `check_persona_constraints` | hkask-agents::curator::persona_filter | crates/hkask-agents/src/curator/persona_filter.rs:34 | Core Logic |
| 11 | hkask-agents | fn | `check_sovereignty` | hkask-agents::curator_agent::spec_curator | crates/hkask-agents/src/curator_agent/spec_curator.rs:194 | Core Logic |
| 12 | hkask-agents | fn | `check_sovereignty` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:632 | Core Logic |
| 13 | hkask-agents | fn | `classified_episodic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:135 | Core Logic |
| 14 | hkask-agents | fn | `context` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:121 | Core Logic |
| 15 | hkask-agents | fn | `context` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:184 | Core Logic |
| 16 | hkask-agents | fn | `correlation_id` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:284 | Core Logic |
| 17 | hkask-agents | fn | `curation_loop` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:164 | Core Logic |
| 18 | hkask-agents | fn | `curator_handle` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:134 | Core Logic |
| 19 | hkask-agents | fn | `deactivate` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:366 | Core Logic |
| 20 | hkask-agents | fn | `decompose_prompt` | hkask-agents::prompt_analysis | crates/hkask-agents/src/prompt_analysis.rs:585 | Core Logic |
| 21 | hkask-agents | fn | `default_tick_interval` | hkask-agents::loop_system | crates/hkask-agents/src/loop_system.rs:64 | Core Logic |
| 22 | hkask-agents | fn | `delegate` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:409 | Core Logic |
| 23 | hkask-agents | fn | `emit_pod_activated` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:53 | Core Logic |
| 24 | hkask-agents | fn | `emit_pod_deactivated` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:65 | Core Logic |
| 25 | hkask-agents | fn | `emit_pod_event` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:20 | Core Logic |
| 26 | hkask-agents | fn | `emit_pod_registered` | hkask-agents::pod::nu_event | crates/hkask-agents/src/pod/nu_event.rs:40 | Core Logic |
| 27 | hkask-agents | fn | `enter_chat_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:505 | Core Logic |
| 28 | hkask-agents | fn | `enter_server_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:470 | Core Logic |
| 29 | hkask-agents | fn | `episodic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:173 | Core Logic |
| 30 | hkask-agents | fn | `episodic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:80 | Core Logic |
| 31 | hkask-agents | fn | `episodic_storage_budget` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:222 | Core Logic |
| 32 | hkask-agents | fn | `episodic_storage_usage` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:210 | Core Logic |
| 33 | hkask-agents | fn | `exit_mode` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:530 | Core Logic |
| 34 | hkask-agents | fn | `gas_cap` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:105 | Core Logic |
| 35 | hkask-agents | fn | `gas_remaining` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:78 | Core Logic |
| 36 | hkask-agents | fn | `generate_summary` | hkask-agents::curator_agent::metacognition | crates/hkask-agents/src/curator_agent/metacognition.rs:325 | Core Logic |
| 37 | hkask-agents | fn | `grant` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:69 | Core Logic |
| 38 | hkask-agents | fn | `grant_consent` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:243 | Core Logic |
| 39 | hkask-agents | fn | `handle` | hkask-agents::curator::context | crates/hkask-agents/src/curator/context.rs:95 | Core Logic |
| 40 | hkask-agents | fn | `inference_port` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:152 | Core Logic |
| 41 | hkask-agents | fn | `inference_port` | hkask-agents::pod::manager | crates/hkask-agents/src/pod/manager.rs:200 | Core Logic |
| 42 | hkask-agents | fn | `in_memory` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:164 | Core Logic |
| 43 | hkask-agents | fn | `in_memory_unchecked` | hkask-agents::adapters::memory_loop_adapter | crates/hkask-agents/src/adapters/memory_loop_adapter.rs:180 | Core Logic |
| 44 | hkask-agents | fn | `invoke_tool` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:334 | Core Logic |
| 45 | hkask-agents | fn | `message_type` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:299 | Core Logic |
| 46 | hkask-agents | fn | `metacognition` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:174 | Core Logic |
| 47 | hkask-agents | fn | `recall_episodic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:193 | Core Logic |
| 48 | hkask-agents | fn | `recall_semantic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:298 | Core Logic |
| 49 | hkask-agents | fn | `require_sovereignty` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:124 | Core Logic |
| 50 | hkask-agents | fn | `restore_cursor` | hkask-agents::curator::curation_loop | crates/hkask-agents/src/curator/curation_loop.rs:149 | Core Logic |
| 51 | hkask-agents | fn | `revoke` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:80 | Core Logic |
| 52 | hkask-agents | fn | `revoke_consent` | hkask-agents::consent | crates/hkask-agents/src/consent.rs:275 | Core Logic |
| 53 | hkask-agents | fn | `semantic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:106 | Core Logic |
| 54 | hkask-agents | fn | `semantic` | hkask-agents::ports::memory_storage | crates/hkask-agents/src/ports/memory_storage.rs:189 | Core Logic |
| 55 | hkask-agents | fn | `semantic_storage_usage` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:315 | Core Logic |
| 56 | hkask-agents | fn | `spec_curator` | hkask-agents::curator_agent::mod | crates/hkask-agents/src/curator_agent/mod.rs:197 | Core Logic |
| 57 | hkask-agents | fn | `state` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:447 | Core Logic |
| 58 | hkask-agents | fn | `store` | hkask-agents::registry_loader | crates/hkask-agents/src/registry_loader.rs:389 | Core Logic |
| 59 | hkask-agents | fn | `store_episodic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:169 | Core Logic |
| 60 | hkask-agents | fn | `store_episodic_experience` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:235 | Core Logic |
| 61 | hkask-agents | fn | `store_semantic` | hkask-agents::pod::context | crates/hkask-agents/src/pod/context.rs:275 | Core Logic |
| 62 | hkask-agents | fn | `strip_forbidden_patterns` | hkask-agents::curator::persona_filter | crates/hkask-agents/src/curator/persona_filter.rs:71 | Core Logic |
| 63 | hkask-agents | fn | `sync_gas_state` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:97 | Core Logic |
| 64 | hkask-agents | fn | `token_usage` | hkask-agents::inference_loop | crates/hkask-agents/src/inference_loop.rs:87 | Core Logic |
| 65 | hkask-agents | fn | `validate_fields` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:178 | Core Logic |
| 66 | hkask-agents | fn | `visit` | hkask-agents::acp::mod | crates/hkask-agents/src/acp/mod.rs:216 | Core Logic |
| 67 | hkask-agents | fn | `voice_description` | hkask-agents::pod::mod | crates/hkask-agents/src/pod/mod.rs:592 | Core Logic |
| 68 | hkask-agents | fn | `webid` | hkask-agents::pod::types | crates/hkask-agents/src/pod/types.rs:161 | Core Logic |
| 69 | hkask-api | fn | `acp_router` | hkask-api::routes::acp | crates/hkask-api/src/routes/acp.rs:82 | API Route Handler |
| 70 | hkask-api | fn | `backup_router` | hkask-api::routes::backup | crates/hkask-api/src/routes/backup.rs:180 | API Route Handler |
| 71 | hkask-api | fn | `bots_router` | hkask-api::routes::bots | crates/hkask-api/src/routes/bots.rs:13 | API Route Handler |
| 72 | hkask-api | fn | `bundles_router` | hkask-api::routes::bundles | crates/hkask-api/src/routes/bundles.rs:91 | API Route Handler |
| 73 | hkask-api | fn | `chat_router` | hkask-api::routes::chat | crates/hkask-api/src/routes/chat.rs:60 | API Route Handler |
| 74 | hkask-api | fn | `cns_router` | hkask-api::routes::cns | crates/hkask-api/src/routes/cns.rs:24 | API Route Handler |
| 75 | hkask-api | fn | `consolidation_router` | hkask-api::routes::consolidation | crates/hkask-api/src/routes/consolidation.rs:47 | API Route Handler |
| 76 | hkask-api | fn | `create_openapi` | hkask-api | crates/hkask-api/src/lib.rs:262 | API Route Handler |
| 77 | hkask-api | fn | `create_router` | hkask-api | crates/hkask-api/src/lib.rs:212 | API Route Handler |
| 78 | hkask-api | fn | `curator_router` | hkask-api::routes::curator | crates/hkask-api/src/routes/curator.rs:80 | API Route Handler |
| 79 | hkask-api | fn | `episodic_router` | hkask-api::routes::episodic | crates/hkask-api/src/routes/episodic.rs:25 | API Route Handler |
| 80 | hkask-api | fn | `git_router` | hkask-api::routes::git | crates/hkask-api/src/routes/git.rs:57 | API Route Handler |
| 81 | hkask-api | fn | `goal_router` | hkask-api::routes::goal | crates/hkask-api/src/routes/goal.rs:16 | API Route Handler |
| 82 | hkask-api | fn | `mcp_router` | hkask-api::routes::mcp | crates/hkask-api/src/routes/mcp.rs:38 | API Route Handler |
| 83 | hkask-api | fn | `models_router` | hkask-api::routes::models | crates/hkask-api/src/routes/models.rs:25 | API Route Handler |
| 84 | hkask-api | fn | `pods_router` | hkask-api::routes::pods | crates/hkask-api/src/routes/pods.rs:49 | API Route Handler |
| 85 | hkask-api | fn | `revoke_token` | hkask-api::middleware::auth | crates/hkask-api/src/middleware/auth.rs:49 | API Route Handler |
| 86 | hkask-api | fn | `settings_router` | hkask-api::routes::settings | crates/hkask-api/src/routes/settings.rs:86 | API Route Handler |
| 87 | hkask-api | fn | `shutdown_loops` | hkask-api | crates/hkask-api/src/lib.rs:198 | API Route Handler |
| 88 | hkask-api | fn | `sovereignty_router` | hkask-api::routes::sovereignty | crates/hkask-api/src/routes/sovereignty.rs:25 | API Route Handler |
| 89 | hkask-api | fn | `spec_router` | hkask-api::routes::spec | crates/hkask-api/src/routes/spec.rs:72 | API Route Handler |
| 90 | hkask-api | fn | `templates_router` | hkask-api::routes::templates | crates/hkask-api/src/routes/templates.rs:48 | API Route Handler |
| 91 | hkask-api | fn | `verify_token` | hkask-api::middleware::auth | crates/hkask-api/src/middleware/auth.rs:76 | API Route Handler |
| 92 | hkask-api | fn | `wallet_router` | hkask-api::routes::wallet | crates/hkask-api/src/routes/wallet.rs:30 | API Route Handler |
| 93 | hkask-cli | fn | `build_service_context` | hkask-cli::commands::helpers | crates/hkask-cli/src/commands/helpers.rs:27 | Core Logic |
| 94 | hkask-cli | fn | `change_passphrase` | hkask-cli::commands::user | crates/hkask-cli/src/commands/user.rs:424 | Core Logic |
| 95 | hkask-cli | fn | `create` | hkask-cli::commands::goal | crates/hkask-cli/src/commands/goal.rs:12 | Core Logic |
| 96 | hkask-cli | fn | `curator_webid` | hkask-cli::bootstrap | crates/hkask-cli/src/bootstrap.rs:389 | Core Logic |
| 97 | hkask-cli | fn | `format_tool_prompt_section` | hkask-cli::repl::tool_augmented | crates/hkask-cli/src/repl/tool_augmented.rs:43 | Core Logic |
| 98 | hkask-cli | fn | `format_tool_results` | hkask-cli::repl::tool_augmented | crates/hkask-cli/src/repl/tool_augmented.rs:207 | Core Logic |
| 99 | hkask-cli | fn | `generate_cli_markdown` | hkask-cli::cli::markdown | crates/hkask-cli/src/cli/markdown.rs:12 | Core Logic |
| 100 | hkask-cli | fn | `init_logging` | hkask-cli::cli::helpers | crates/hkask-cli/src/cli/helpers.rs:31 | Core Logic |

---

## Per-Crate High-Risk Uncovered Count

| Crate | High-Risk Uncovered |
|-------|--------------------|
| hkask-types | 148 |
| hkask-storage | 135 |
| hkask-services | 131 |
| hkask-agents | 68 |
| hkask-cns | 50 |
| hkask-cli | 45 |
| hkask-memory | 43 |
| hkask-templates | 36 |
| hkask-mcp-research | 35 |
| hkask-keystore | 32 |
| hkask-mcp | 30 |
| hkask-test-harness | 29 |
| hkask-mcp-companies | 29 |
| hkask-improv | 28 |
| hkask-wallet | 24 |
| hkask-api | 24 |
| hkask-inference | 14 |
| hkask-condenser | 14 |
| hkask-mcp-training | 12 |
| hkask-mcp-docproc | 12 |
| hkask-mcp-media | 7 |
| hkask-rsolidity-macros | 2 |
| hkask-mcp-spec | 2 |
| hkask-rsolidity | 1 |
| hkask-mcp-communication | 1 |
| hkask-communication | 1 |

**Total high-risk uncovered:** 953
