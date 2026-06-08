---
title: "Test Inventory & Seam Analysis"
audience: [architects, developers, agents]
last_updated: 2026-06-08
version: "0.1.0"
status: "Active"
domain: "Quality"
---

# Test Inventory & Seam Analysis

**Purpose:** Enumerate every behavioral test in the workspace, map it to the seam it exercises, and track coverage status per crate.

**Verification:** `cargo test --workspace 2>&1 | tail -1` (must report 0 failures)

---

## 1. Summary

| Crate | Tests | Status |
|-------|------:|--------|
| `hkask-services` | 46 | ✅ Active |
| `hkask-api` | 3 | ✅ Active |
| **Total** | **49** | |

---

## 2. Service Layer — `hkask-services` (46 tests)

### 2.1 InferenceService (4 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `resolve_port_with_default` | `InferenceService` | Default port is returned when no override is set |
| 2 | `resolve_port_with_non_default` | `InferenceService` | Custom port override takes precedence over default |
| 3 | `list_models` | `InferenceService` | Model listing returns available models from context |
| 4 | `search_models` | `InferenceService` | Model search filters results by query |

### 2.2 CuratorService (6 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `list_escalations` | `CuratorService` | Returns escalation list from context |
| 2 | `get_escalation` | `CuratorService` | Returns single escalation by ID |
| 3 | `resolve_escalation` | `CuratorService` | Escalation transitions to resolved state |
| 4 | `dismiss_escalation` | `CuratorService` | Escalation transitions to dismissed state |
| 5 | `escalation_stats` | `CuratorService` | Returns aggregate escalation statistics |
| 6 | `run_metacognition` | `CuratorService` | Metacognition cycle executes and reports outcome |

### 2.3 EnsembleService (11 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `create_chat` | `EnsembleService` | New chat session is created with valid parameters |
| 2 | `list_chat_sessions` | `EnsembleService` | Session listing returns created sessions |
| 3 | `register_participant` | `EnsembleService` | Participant is registered to a session |
| 4 | `send_message` | `EnsembleService` | Message is sent and persisted in session |
| 5 | `create_deliberation` | `EnsembleService` | Deliberation is created within a session |
| 6 | `start_deliberation` | `EnsembleService` | Deliberation transitions to started state |
| 7 | `record_deliberation_response` | `EnsembleService` | Participant response is recorded |
| 8 | `synthesize_deliberation` | `EnsembleService` | Deliberation synthesis produces aggregate result |
| 9 | `parse_data_category` | `EnsembleService` | Data category string parses to correct enum variant |
| 10 | `map_participant_role` | `EnsembleService` | Participant role maps to correct enum variant |
| 11 | `session_not_found` | `EnsembleService` | Missing session returns appropriate error |

### 2.4 PodService (6 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `parse_pod_id` | `PodService` | Pod ID string parses to valid identifier |
| 2 | `get_pod_status` | `PodService` | Returns current pod lifecycle status |
| 3 | `list_pods` | `PodService` | Returns all pods with their statuses |
| 4 | `create_pod` | `PodService` | New pod is created in inactive state |
| 5 | `activate_pod` | `PodService` | Pod transitions from inactive to active |
| 6 | `deactivate_pod` | `PodService` | Pod transitions from active to inactive |

### 2.5 SovereigntyService (13 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `parse_data_category` | `SovereigntyService` | Data category string parses to correct enum variant |
| 2 | `get_boundary` | `SovereigntyService` | Boundary is returned for a valid data category |
| 3 | `requires_affirmative_consent` | `SovereigntyService` | Consent requirement is correctly determined |
| 4 | `grant_consent` | `SovereigntyService` | Consent is granted and recorded |
| 5 | `revoke_consent` | `SovereigntyService` | Consent is revoked and no longer active |
| 6 | `has_consent` | `SovereigntyService` | Consent presence is correctly reported |
| 7 | `get_granted_categories` | `SovereigntyService` | Returns all categories with active consent |
| 8 | `check_access` | `SovereigntyService` | Access check returns correct permit/deny result |
| 9 | `get_status` | `SovereigntyService` | Sovereignty status reflects current consent state |
| 10 | `access_check_type` | `SovereigntyService` | Access check result type is correctly shaped |
| 11 | `sovereignty_status_type` | `SovereigntyService` | Status result type is correctly shaped |
| 12 | `consent_not_found_returns_err` | `SovereigntyService` | Missing consent returns error, not panic |
| 13 | `invalid_uuid_returns_pod_not_found` | `SovereigntyService` | Invalid UUID returns pod-not-found error |

### 2.6 Infrastructure / Context (6 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `inference_context_from_service_context` | `ServiceContext` | Service context yields valid inference context |
| 2 | `pod_context_from_service_context` | `ServiceContext` | Service context yields valid pod context |
| 3 | `sovereignty_context_from_service_context` | `ServiceContext` | Service context yields valid sovereignty context |
| 4 | `curator_context_from_service_context_escalation_only` | `ServiceContext` | Service context yields curator context (escalation-only) |
| 5 | `curator_context_from_service_context_full` | `ServiceContext` | Service context yields curator context (full capabilities) |
| 6 | `ensemble_context_from_service_context` | `ServiceContext` | Service context yields valid ensemble context |

---

## 3. API Layer — `hkask-api` (3 tests)

| # | Test Name | Seam | Invariant |
|---|-----------|------|-----------|
| 1 | `from_service_context_produces_valid_state` | `AppState` | Service context conversion yields valid API state |
| 2 | `with_defaults_produces_valid_state` | `AppState` | Default configuration yields valid API state |
| 3 | `from_service_context_has_required_fields` | `AppState` | Converted state contains all required fields |

---

## 4. Coverage Notes

- **Service layer extraction** — The 46 tests in `hkask-services` were extracted from CLI/API surfaces as part of the service layer refactoring (strangler fig pattern). They exercise the `*Service` structs through their public methods, independent of surface-specific adapters.
- **Context wiring** — The 6 infrastructure/context tests verify that `ServiceContext` correctly produces typed contexts for each service, ensuring the dependency injection layer is wired correctly.
- **API state** — The 3 API tests validate `AppState` construction from `ServiceContext`, confirming the API layer receives a properly configured state without duplicating service-layer logic.

---

*Last updated: 2026-06-08 — Service layer extraction phase (46 services + 3 API = 49 total)*