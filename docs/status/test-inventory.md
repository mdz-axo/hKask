---
title: "Test Inventory & Seam Analysis"
audience: [architects, developers, agents]
last_updated: 2026-06-08
version: "0.2.0"
status: "Active"
domain: "Quality"
---

# Test Inventory & Seam Analysis

**Purpose:** Enumerate every behavioral test in the workspace, map it to the seam it exercises, and track coverage status per crate.

**Verification:** `cargo test --workspace 2>&1 | tail -1` (must report 0 failures)

---

## 1. Summary

| Crate | Unit Tests | Doc Tests | Status |
|-------|----------:|----------:|--------|
| `hkask-services` | 138 | 6 (ignored) | ✅ Active |
| `hkask-templates` | 16 | 1 | ✅ Active |
| `hkask-mcp-condenser` | 53 | 0 | ✅ Active |
| `hkask-api` | 3 | 0 | ✅ Active |
| `hkask-storage` | 0 | 7 (ignored) | ✅ Active |
| `hkask-agents` | 0 | 3 (1 ignored) | ✅ Active |
| `hkask-mcp` | 0 | 3 (2 ignored) | ✅ Active |
| `hkask-cns` | 0 | 1 (ignored) | ✅ Active |
| **Total** | **210** | **24** | |

---

## 2. Service Layer — `hkask-services` (138 unit tests)

### 2.1 EnsembleService (17 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `create_chat_creates_session` | `EnsembleService` | New chat session is created with valid parameters |
| 2 | `list_chat_sessions_returns_ids` | `EnsembleService` | Session listing returns created session IDs |
| 3 | `list_participants_returns_empty_for_new_session` | `EnsembleService` | New session has no participants |
| 4 | `list_participants_returns_not_found_for_missing_session` | `EnsembleService` | Missing session returns error |
| 5 | `register_participant_succeeds_for_existing_session` | `EnsembleService` | Participant registration succeeds |
| 6 | `register_participant_returns_not_found_for_missing_session` | `EnsembleService` | Missing session returns error |
| 7 | `send_message_succeeds_for_existing_session` | `EnsembleService` | Message send succeeds |
| 8 | `send_message_returns_not_found_for_missing_session` | `EnsembleService` | Missing session returns error |
| 9 | `create_deliberation_creates_session` | `EnsembleService` | Deliberation is created |
| 10 | `start_deliberation_returns_not_found_for_missing_session` | `EnsembleService` | Missing deliberation returns error |
| 11 | `record_response_returns_not_found_for_missing_session` | `EnsembleService` | Missing deliberation returns error |
| 12 | `synthesize_deliberation_returns_not_found_for_missing_session` | `EnsembleService` | Missing deliberation returns error |
| 13 | `improv_config_succeeds_for_existing_session` | `EnsembleService` | Improv config returns for existing session |
| 14 | `improv_config_returns_not_found_for_missing_session` | `EnsembleService` | Missing session returns error |
| 15 | `set_improv_mode_returns_not_found_for_missing_session` | `EnsembleService` | Missing session returns error |
| 16 | `set_improv_threshold_returns_not_found_for_missing_session` | `EnsembleService` | Missing session returns error |
| 17 | `map_participant_role_normalizes_orchestrator` | `EnsembleService` | Orchestrator role is normalized |

### 2.2 SovereigntyService (13 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `parse_data_category_maps_known_categories` | `SovereigntyService` | Known category strings parse correctly |
| 2 | `parse_data_category_maps_unknown_to_custom` | `SovereigntyService` | Unknown category strings map to Custom |
| 3 | `get_boundary_returns_hkask_default` | `SovereigntyService` | Default boundary is returned for known categories |
| 4 | `requires_affirmative_consent_is_true_for_default` | `SovereigntyService` | Consent requirement is correctly determined |
| 5 | `grant_consent_allows_subsequent_check` | `SovereigntyService` | Consent is granted and permits access |
| 6 | `revoke_consent_removes_all_granted_consent` | `SovereigntyService` | Consent is revoked and access is denied |
| 7 | `has_consent_returns_false_for_unknown_webid` | `SovereigntyService` | Consent absence is correctly reported |
| 8 | `get_granted_categories_returns_granted_names` | `SovereigntyService` | Returns all categories with active consent |
| 9 | `check_access_public_always_accessible` | `SovereigntyService` | Public data is always accessible |
| 10 | `check_access_classifies_sovereign_correctly` | `SovereigntyService` | Sovereign data access check is correct |
| 11 | `check_access_classifies_shared_correctly` | `SovereigntyService` | Shared data access check is correct |
| 12 | `get_status_reflects_granted_consent` | `SovereigntyService` | Status reflects current consent state |
| 13 | `get_status_returns_boundary_and_consent_state` | `SovereigntyService` | Status includes boundary and consent |

### 2.3 GoalService (13 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `parse_goal_id_accepts_valid_uuid` | `GoalService` | Valid UUID parses to goal ID |
| 2 | `parse_goal_id_rejects_invalid_uuid` | `GoalService` | Invalid UUID returns error |
| 3 | `parse_goal_state_accepts_valid_strings` | `GoalService` | Valid state strings parse correctly |
| 4 | `parse_goal_state_rejects_invalid_string` | `GoalService` | Invalid state string returns error |
| 5 | `parse_visibility_accepts_valid_strings` | `GoalService` | Valid visibility strings parse correctly |
| 6 | `parse_visibility_rejects_invalid_string` | `GoalService` | Invalid visibility string returns error |
| 7 | `create_goal_creates_with_parsed_visibility` | `GoalService` | Goal is created with visibility |
| 8 | `create_goal_rejects_invalid_visibility` | `GoalService` | Invalid visibility returns error |
| 9 | `list_goals_returns_created_goals` | `GoalService` | Listing returns created goals |
| 10 | `list_goals_filters_by_state` | `GoalService` | Goals filter by state correctly |
| 11 | `set_goal_state_transitions_pending_to_active` | `GoalService` | State transition works |
| 12 | `set_goal_state_rejects_illegal_transition` | `GoalService` | Illegal transition returns error |
| 13 | `set_goal_state_rejects_invalid_goal_id` | `GoalService` | Invalid goal ID returns error |

### 2.4 ComposeService (11 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `cosine_distance_identical_vectors_is_zero` | `ComposeService` | Identical vectors have zero distance |
| 2 | `cosine_distance_opposite_vectors_is_two` | `ComposeService` | Opposite vectors have distance two |
| 3 | `cosine_distance_orthogonal_vectors_is_one` | `ComposeService` | Orthogonal vectors have distance one |
| 4 | `cosine_distance_mismatched_lengths_returns_two` | `ComposeService` | Mismatched lengths return max distance |
| 5 | `centroid_validation_passed_when_distance_within_threshold` | `ComposeService` | Validation passes within threshold |
| 6 | `centroid_validation_failed_when_distance_exceeds_threshold` | `ComposeService` | Validation fails beyond threshold |
| 7 | `cognition_config_deserializes_from_yaml` | `ComposeService` | YAML config deserialization works |
| 8 | `cognition_config_uses_default_retrieval_values` | `ComposeService` | Defaults are applied when not specified |
| 9 | `system_prompt_contains_exemplar_block` | `ComposeService` | System prompt includes exemplars |
| 10 | `system_prompt_omits_exemplar_block_when_empty` | `ComposeService` | Exemplars omitted when empty |
| 11 | `system_prompt_omits_centroid_note_when_no_validate` | `ComposeService` | Centroid note omitted without validation |

### 2.5 VerificationService (9 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `verification_service_has_three_operations` | `VerificationService` | Service exposes 3 operations |
| 2 | `verification_error_is_string_sentinel` | `VerificationService` | Error type is string-sentinel |
| 3 | `verify_returns_empty_report_without_manifests` | `VerificationService` | Empty report when no manifests |
| 4 | `verify_json_returns_valid_structure` | `VerificationService` | JSON verification produces valid structure |
| 5 | `manifest_deserializes_from_yaml` | `VerificationService` | YAML manifest deserialization works |
| 6 | `assertion_result_carries_status` | `VerificationService` | Assertion result includes status |
| 7 | `principle_result_carries_display_name` | `VerificationService` | Principle result includes display name |
| 8 | `principle_aliases_resolve` | `VerificationService` | Principle aliases resolve correctly |
| 9 | `verification_report_carries_totals` | `VerificationService` | Report includes total counts |

### 2.6 EmbedService (9 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `embed_service_has_operations` | `EmbedService` | Service exposes operations |
| 2 | `embed_error_is_string_sentinel` | `EmbedService` | Error type is string-sentinel |
| 3 | `embed_result_carries_pipeline_output` | `EmbedService` | Result includes pipeline output |
| 4 | `embedding_config_fields` | `EmbedService` | Config fields are accessible |
| 5 | `corpus_config_deserializes_from_yaml` | `EmbedService` | YAML config deserialization works |
| 6 | `chunking_config_fields` | `EmbedService` | Chunking config fields are accessible |
| 7 | `validation_config_is_cloneable` | `EmbedService` | Validation config is Clone |
| 8 | `foundational_rule_fields` | `EmbedService` | Foundational rule fields are accessible |
| 9 | `work_fields` | `EmbedService` | Work fields are accessible |

### 2.7 SkillService (8 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `skill_service_has_seven_operations` | `SkillService` | Service exposes 7 operations |
| 2 | `skill_error_is_string_sentinel` | `SkillService` | Error type is string-sentinel |
| 3 | `skill_info_carries_metadata` | `SkillService` | SkillInfo includes metadata |
| 4 | `publish_result_carries_metadata` | `SkillService` | PublishResult includes metadata |
| 5 | `resolve_replicant_name_falls_back_to_local` | `SkillService` | Name resolution falls back to local |
| 6 | `read_skill_namespace_returns_none_for_missing` | `SkillService` | Missing skill returns None |
| 7 | `read_skill_visibility_defaults_private` | `SkillService` | Default visibility is private |
| 8 | `compute_file_hash_returns_none_for_missing` | `SkillService` | Missing file returns None for hash |

### 2.8 CuratorService (6 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `list_escalations_delegates_to_queue` | `CuratorService` | Listing delegates to escalation queue |
| 2 | `get_escalation_returns_none_for_missing_id` | `CuratorService` | Missing escalation returns None |
| 3 | `resolve_escalation_returns_not_found_for_missing_id` | `CuratorService` | Missing escalation returns error |
| 4 | `dismiss_escalation_returns_not_found_for_missing_id` | `CuratorService` | Missing escalation returns error |
| 5 | `escalation_stats_returns_counts_after_adding_escalation` | `CuratorService` | Stats reflect escalation state |
| 6 | `run_metacognition_returns_error_without_cns_runtime` | `CuratorService` | Metacognition fails without CNS |

### 2.9 Infrastructure / Context (8 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `inference_context_from_service_context` | `ServiceContext` | Service context yields valid inference context |
| 2 | `pod_context_from_service_context` | `ServiceContext` | Service context yields valid pod context |
| 3 | `sovereignty_context_from_service_context` | `ServiceContext` | Service context yields valid sovereignty context |
| 4 | `curator_context_from_service_context_escalation_only` | `ServiceContext` | Service context yields curator context (escalation-only) |
| 5 | `curator_context_from_service_context_full` | `ServiceContext` | Service context yields curator context (full capabilities) |
| 6 | `ensemble_context_from_service_context` | `ServiceContext` | Service context yields valid ensemble context |
| 7 | `memory_stores_in_memory_when_config_says_in_memory` | `ServiceContext` | In-memory stores when config says in-memory |
| 8 | `memory_stores_persist_when_not_in_memory` | `ServiceContext` | Persistent stores when config says not in-memory |

### 2.10 PodService (6 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `parse_pod_id_accepts_valid_uuid` | `PodService` | Valid UUID parses to pod ID |
| 2 | `parse_pod_id_rejects_invalid_uuid` | `PodService` | Invalid UUID returns error |
| 3 | `list_pods_returns_empty_for_new_manager` | `PodService` | New manager has no pods |
| 4 | `activate_pod_returns_not_found_for_missing_pod` | `PodService` | Missing pod returns not-found |
| 5 | `deactivate_pod_returns_not_found_for_missing_pod` | `PodService` | Missing pod returns not-found |
| 6 | `get_pod_status_returns_not_found_for_missing_pod` | `PodService` | Missing pod returns not-found |

### 2.11 ArchivalService (7 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `archival_service_has_four_operations` | `ArchivalService` | Service exposes 4 operations |
| 2 | `archival_error_is_string_sentinel` | `ArchivalService` | Error type is string-sentinel |
| 3 | `archive_result_carries_path_and_commit` | `ArchivalService` | Archive result includes path and commit |
| 4 | `snapshot_result_carries_commit_sha` | `ArchivalService` | Snapshot result includes commit SHA |
| 5 | `default_registry_path_resolves_dot` | `ArchivalService` | Default registry path is `.` |
| 6 | `github_api_base_url` | `ArchivalService` | GitHub API base URL is correct |
| 7 | `build_client_fails_without_credentials` | `ArchivalService` | Client build fails without credentials |

### 2.12 UserService (6 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `validate_passphrase_accepts_valid` | `UserService` | Valid passphrase passes validation |
| 2 | `validate_passphrase_rejects_short` | `UserService` | Short passphrase is rejected |
| 3 | `validate_passphrase_rejects_no_uppercase` | `UserService` | Passphrase without uppercase is rejected |
| 4 | `validate_passphrase_rejects_non_alphanumeric` | `UserService` | Non-alphanumeric passphrase is rejected |
| 5 | `validate_registration_rejects_empty_name` | `UserService` | Empty name is rejected |
| 6 | `validate_registration_rejects_invalid_email` | `UserService` | Invalid email is rejected |

### 2.13 OnboardingService (6 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `derive_secrets_is_deterministic` | `OnboardingService` | Secret derivation is deterministic |
| 2 | `derive_secrets_differs_for_different_inputs` | `OnboardingService` | Different inputs produce different secrets |
| 3 | `derive_and_store_secrets_returns_secrets` | `OnboardingService` | Store+derive returns secrets |
| 4 | `try_list_returns_empty_for_missing_db` | `OnboardingService` | Missing DB returns empty list |
| 5 | `cleanup_does_not_panic_for_in_memory` | `OnboardingService` | Cleanup handles in-memory DB |
| 6 | `remove_orphaned_returns_false_for_in_memory` | `OnboardingService` | Orphan removal handles in-memory DB |

### 2.14 SpecService (5 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `build_spec_preserves_name` | `SpecService` | Spec name is preserved |
| 2 | `build_spec_parses_category_and_domain` | `SpecService` | Category and domain are parsed |
| 3 | `build_spec_falls_back_for_invalid_category` | `SpecService` | Invalid category falls back |
| 4 | `build_spec_applies_criteria` | `SpecService` | Criteria are applied |
| 5 | `list_categories_returns_all` | `SpecService` | All categories are listed |

### 2.15 InferenceService (4 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `resolve_port_returns_error_when_no_server` | `InferenceService` | No server returns error |
| 2 | `default_model_config_value` | `InferenceService` | Default model config is correct |
| 3 | `model_info_from_okapi_entry` | `InferenceService` | Full Okapi entry produces model info |
| 4 | `model_info_from_minimal_okapi_entry` | `InferenceService` | Minimal Okapi entry produces model info |

### 2.16 AgentService (4 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `list_returns_empty_for_fresh_context` | `AgentService` | Fresh context has no agents |
| 2 | `register_rejects_invalid_agent_type` | `AgentService` | Invalid agent type is rejected |
| 3 | `status_returns_not_found_for_unknown_agent` | `AgentService` | Unknown agent returns not-found |
| 4 | `unregister_returns_error_for_unknown_agent` | `AgentService` | Unknown agent returns error |

### 2.17 ServiceConfig (3 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `memory_db_path_derived_from_db_path` | `ServiceConfig` | Memory DB path derived from main DB path |
| 2 | `memory_db_path_explicit_overrides_derivation` | `ServiceConfig` | Explicit path overrides derivation |
| 3 | `memory_db_path_none_when_in_memory` | `ServiceConfig` | No path when in-memory mode |

### 2.18 ConsolidationService (2 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `consolidate_pipeline_constructs_from_fresh_db` | `ConsolidationService` | Pipeline constructs from fresh DB |
| 2 | `verify_passphrase_rejects_invalid_passphrase` | `ConsolidationService` | Invalid passphrase is rejected |

### 2.19 ChatService (1 test)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `token_usage_gas_cost_is_total` | `ChatService` | Gas cost equals total tokens |

---

## 3. Domain Layer — `hkask-agents` (0 unit tests, 3 doc tests)

No unit tests in `hkask-agents` currently. 3 doc tests exercise `PodManagerBuilder` and `pod` module
(1 ignored, 2 passing).

### Recent domain-layer changes (not yet covered by dedicated unit tests)

| Change | Session | Type |
|--------|---------|------|
| `RecalledEpisode` typed DTO for `EpisodicStoragePort::recall_episodic` | 24 (#73) | Port return type |
| `RecalledSemantic` typed DTO for `SemanticStoragePort::recall_semantic` | 25 (#75) | Port return type |
| `PodManager::new_mock()` deterministic test ACP secret | 24 (#74) | Test fixture |
| `triple_to_recalled_episode` helper in `MemoryLoopAdapter` | 24 | Adapter helper |
| `triple_to_recalled_semantic` helper in `MemoryLoopAdapter` | 25 | Adapter helper |

---

## 4. API Layer — `hkask-api` (3 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `from_service_context_produces_valid_state` | `AppState` | Service context conversion yields valid API state |
| 2 | `from_service_context_with_ensemble_inferencer` | `AppState` | Service context with ensemble inferencer yields valid state |
| 3 | `with_defaults_uses_service_context` | `AppState` | Default configuration yields valid API state |

---

## 5. Templates Layer — `hkask-templates` (16 unit tests)

### Skill Loader (16 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `parse_front_matter_with_visibility` | `SkillLoader` | Front matter with visibility parses |
| 2 | `parse_front_matter_without_visibility` | `SkillLoader` | Front matter without visibility defaults |
| 3 | `parse_front_matter_no_front_matter` | `SkillLoader` | Missing front matter is handled |
| 4 | `default_visibility_is_private` | `SkillLoader` | Default visibility is private |
| 5 | `parse_qualified_id_valid` | `SkillLoader` | Valid qualified ID parses |
| 6 | `parse_qualified_id_no_separator` | `SkillLoader` | Missing separator returns error |
| 7 | `parse_qualified_id_empty_parts` | `SkillLoader` | Empty parts return error |
| 8 | `qualified_id_with_namespace` | `SkillLoader` | Namespace is included in qualified ID |
| 9 | `qualified_id_without_namespace` | `SkillLoader` | Qualified ID works without namespace |
| 10 | `namespace_collision_prevention` | `SkillLoader` | Namespace collisions are prevented |
| 11 | `content_hash_includes_visibility_and_zone` | `SkillLoader` | Hash includes visibility and zone |
| 12 | `check_zone_visibility_public_zone_public_visibility_no_warn` | `SkillLoader` | Public zone + public visibility is OK |
| 13 | `check_zone_visibility_private_zone_any_visibility_no_warn` | `SkillLoader` | Private zone + any visibility is OK |
| 14 | `check_zone_visibility_public_zone_private_visibility_warns` | `SkillLoader` | Public zone + private visibility warns |
| 15 | `list_skills_visible_to_public_sees_only_public_and_shared` | `SkillLoader` | Public visibility filters correctly |
| 16 | `list_skills_visible_to_private_sees_all` | `SkillLoader` | Private visibility sees all |

---

## 6. MCP Server — `hkask-mcp-condenser` (53 unit tests)

### Algorithms (24 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|----------|
| 1 | `flashrank_relevance_matches_terms` | `FlashRankAlgorithm` | FlashRank relevance scoring |
| 2 | `flashrank_preserves_order` | `FlashRankAlgorithm` | FlashRank preserves input order |
| 3 | `flashrank_reduces_lines` | `FlashRankAlgorithm` | FlashRank reduces lines under budget |
| 4 | `flashrank_novelty_is_one_for_empty_selected` | `FlashRankAlgorithm` | FlashRank novelty for empty selection |
| 5 | `flashrank_brevity_favors_shorter_lines` | `FlashRankAlgorithm` | FlashRank brevity favors shorter lines |
| 6 | `flashrank_passthrough_when_under_budget` | `FlashRankAlgorithm` | FlashRank passthrough under budget |
| 7 | `rtk_style_always_produces_output` | `RtkStyleAlgorithm` | RTK always produces output |
| 8 | `rtk_style_includes_ellipsis_when_truncating` | `RtkStyleAlgorithm` | RTK includes ellipsis on truncation |
| 9 | `rtk_style_passthrough_when_under_budget` | `RtkStyleAlgorithm` | RTK passthrough under budget |
| 10 | `rtk_style_preserves_head_and_tail` | `RtkStyleAlgorithm` | RTK preserves head and tail |
| 11 | `rtk_style_reduces_lines_under_heavy` | `RtkStyleAlgorithm` | RTK reduces lines under heavy load |
| 12 | `saliency_rank_prioritizes_error_lines` | `SaliencyRankAlgorithm` | Saliency prioritizes error lines |
| 13 | `saliency_rank_preserves_order` | `SaliencyRankAlgorithm` | Saliency preserves order |
| 14 | `saliency_rank_reduces_lines` | `SaliencyRankAlgorithm` | Saliency reduces lines |
| 15 | `saliency_rank_passthrough_when_under_budget` | `SaliencyRankAlgorithm` | Saliency passthrough under budget |
| 16 | `registry_lists_three_algorithms` | `AlgorithmRegistry` | Registry lists three algorithms |
| 17 | `registry_selects_flashrank_for_file_and_structured` | `AlgorithmRegistry` | Registry selects FlashRank for file/structured |
| 18 | `registry_selects_for_all_categories` | `AlgorithmRegistry` | Registry selects for all categories |
| 19 | `registry_selects_rtk_for_shell` | `AlgorithmRegistry` | Registry selects RTK for shell category |
| 20 | `registry_selects_rtk_for_test_and_build` | `AlgorithmRegistry` | Registry selects RTK for test/build |
| 21 | `registry_selects_saliency_for_conv_log_unknown` | `AlgorithmRegistry` | Registry selects Saliency for conv/log/unknown |
| 22 | `all_algorithms_never_expand` | `AlgorithmRegistry` | No algorithm expands beyond input |
| 23 | `all_algorithms_non_empty_on_single_line` | `AlgorithmRegistry` | Non-empty output even on single-line input |
| 24 | `handles_consistent_with_default_for` | `AlgorithmRegistry` | Handles consistent with default |

### Engine (14 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|----------|
| 1 | `engine_default_profile_is_normal` | `CondenserEngine` | Default profile is Normal |
| 2 | `engine_starts_with_zero_stats` | `CondenserEngine` | Fresh engine has zero stats |
| 3–8 | `engine_compress_*` (6 tests) | `CondenserEngine` | Compression updates stats, auto-classifies, category, reduction |
| 9–11 | `engine_compress_*_counts` | `CondenserEngine` | Line and byte count accuracy |
| 12–14 | `engine_multiple_*`, `engine_set_profile_*` | `CondenserEngine` | Accumulation, profile change |

### Types (15 tests — in `types.rs`)

| # | Test Name | Seam | Invariant |
|---|-----------|------|----------|
| 1 | `profile_retention_pct_matches_spec` | `Profile` | Retention percentages match spec |
| 2 | `profile_max_lines_monotonic_with_light_unbounded` | `Profile` | Max lines decreases monotonically |
| 3 | `profile_round_trips_str` | `Profile` | Profile round-trips through FromStr/Display |
| 4 | `profile_parse_case_insensitive` | `Profile` | Profile parsing is case-insensitive |
| 5 | `profile_parse_rejects_unknown` | `Profile` | Unknown profile string is rejected |
| 6–8 | `context_category_*` (3 tests) | `ContextCategory` | Labels, round-trips, unknown fallback |
| 9 | `classify_tool_shell_variants` | `classify_tool` | Shell variants classified correctly |
| 10 | `classify_tool_all_categories` | `classify_tool` | All categories classified correctly |
| 11 | `classify_tool_unknown_fallback` | `classify_tool` | Unknown tools fall back correctly |
| 12 | `classify_tool_case_insensitive` | `classify_tool` | Classification is case-insensitive |
| 13 | `classify_tool_first_token_wins` | `classify_tool` | First token match wins (priority fix) |
| 14 | `classify_tool_splits_on_separators` | `classify_tool` | Splits on `-`, `_`, `:` separators |
| 15 | `condenser_stats_default` | `CondenserStats` | Default stats have zero counters |

---

## 7. Coverage Notes

- **Service layer extraction** — The 138 tests in `hkask-services` were extracted from CLI/API surfaces as part of the service layer refactoring (strangler fig pattern). They exercise the `*Service` structs through their public methods, independent of surface-specific adapters.
- **Context wiring** — The 8 infrastructure/context tests verify that `ServiceContext` correctly produces typed contexts for each service, including memory store persistence configuration.
- **Typed DTOs** — `RecalledEpisode` (F9, #73) and `RecalledSemantic` (F10, #75) replace `Vec<serde_json::Value>` return types from port traits. These DTOs live in `hkask-agents/src/ports/memory_storage.rs` (domain crate) because they're port trait return types.
- **Test fixture** — `PodManager::new_mock()` uses a deterministic 32-byte ACP secret so 4 pod tests pass without environment variables (F5, #74).
- **API state** — The 3 API tests validate `AppState` construction from `ServiceContext`, confirming the API layer receives a properly configured state.
- **MCP condenser** — 53 tests exercise compression algorithms (24), engine state management (14), and domain types (15) independently of the MCP runtime. The `classify_tool` tests include priority inversion fix (more-specific categories checked before ShellCommand catch-all) and separator splitting.

---

*Last updated: 2026-06-08 — Test inventory refresh (210 unit tests across workspace)*