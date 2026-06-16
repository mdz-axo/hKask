---
title: "REQ Contract Inventory"
audience: [architects, developers, agents, curators]
last_updated: 2026-06-16
version: "0.27.0"
status: "Draft"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle]
---

# REQ Contract Inventory

**Purpose:** Catalog of every `/// REQ:` contract on public functions across crates and MCP servers. Each entry shows the REQ ID, its contract terms (pre/post/inv), and the function it annotates. This is the raw material for designing the rSolidity contract vocabulary.

## Summary by Crate

| Crate | Count | Domain |
|-------|-------|--------|
| hkask-agents | 30 | Agent runtime |
| hkask-api | 8 | API surface |
| hkask-cli | 2 | CLI surface |
| hkask-cns | 91 | CNS observability |
| hkask-communication | 25 | Communication |
| hkask-inference | 86 | Inference |
| hkask-keystore | 28 | Keystore |
| hkask-mcp | 41 | MCP framework |
| hkask-memory | 52 | Memory |
| hkask-services | 208 | Service layer |
| hkask-storage | 195 | Storage |
| hkask-templates | 52 | Templates |
| hkask-test-harness | 42 | Test harness |
| hkask-types | 99 | Type system |
| hkask-wallet | 27 | Wallet |

## Per-Crate Contract Detail

### hkask-agents (30 contracts)

#### P2-agt-consent-record-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `webid` is a non-empty string.
- **Post:** Returns a new `ConsentRecord` with empty granted categories,
- **File:** crates/hkask-agents/src/consent.rs:47

#### P2-agt-consent-record-grant (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `category` is a non-empty string.
- **Post:** `category` is added to `granted_categories`; `active` is set
- **File:** crates/hkask-agents/src/consent.rs:64

#### P2-agt-consent-record-revoke (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  (none — revoke is always valid).
- **Post:** `revoked_at` is set to the current UTC timestamp;
- **File:** crates/hkask-agents/src/consent.rs:75

#### P2-agt-consent-record-is-active (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  (none).
- **Post:** Returns `true` iff `active == true` AND `revoked_at` is `None`.
- **File:** crates/hkask-agents/src/consent.rs:85

#### P2-agt-consent-record-has-category (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `category` is a non-empty string.
- **Post:** Returns `true` iff the record is active AND `category` is
- **File:** crates/hkask-agents/src/consent.rs:93

#### P2-agt-consent-manager-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `store` is a valid, initialized `ConsentStore`.
- **Post:** Returns a `ConsentManager` with an empty in-memory cache;
- **File:** crates/hkask-agents/src/consent.rs:149

#### P2-agt-consent-manager-with-sink (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `sink` is a valid `Arc<dyn NuEventSink>`.
- **Post:** Returns `self` with `event_sink` set to `Some(sink)`.
- **File:** crates/hkask-agents/src/consent.rs:175

#### P2-agt-consent-manager-grant (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `webid` is a non-empty string; `category` is a valid
- **Post:** If a record exists for `webid`, the category is granted and
- **File:** crates/hkask-agents/src/consent.rs:236

#### P2-agt-consent-manager-revoke (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `webid` is a non-empty string.
- **Post:** If a record exists for `webid`, it is revoked and persisted;
- **File:** crates/hkask-agents/src/consent.rs:269

#### P2-agt-consent-manager-check (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `webid` is a non-empty string; `category` is a valid
- **Post:** Returns `Ok(true)` if an active record for `webid` has the
- **File:** crates/hkask-agents/src/consent.rs:293

#### P2-agt-consent-manager-granted-categories (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `webid` is a non-empty string.
- **Post:** Returns `Ok(Vec<String>)` containing all granted category
- **File:** crates/hkask-agents/src/consent.rs:347

#### P9-agt-loop-id (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `loop_id` is one of `Inference`, `Memory`, `Cybernetics`, or
- **Post:** Returns the default tick `Duration` for the given loop:
- **File:** crates/hkask-agents/src/loop_system.rs:58

#### P9-agt-loop-system-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  (none).
- **Post:** Returns a `LoopSystem` with an empty loop registry, a fresh
- **File:** crates/hkask-agents/src/loop_system.rs:102

#### P9-agt-loop-system-interval (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `loop_id` is a valid `LoopId`; `interval` is a positive
- **Post:** Returns `self` with the tick interval for `loop_id` updated
- **File:** crates/hkask-agents/src/loop_system.rs:127

#### P9-agt-loop-system-register (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `loop_instance` is a valid `Arc<dyn HkaskLoop>`.
- **Post:** The loop is added to the registry under its `LoopId`;
- **File:** crates/hkask-agents/src/loop_system.rs:144

#### P9-agt-loop-system-cancel-token (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  (none — accessor).
- **Post:** Returns a clone of the inner `CancellationToken`.
- **File:** crates/hkask-agents/src/loop_system.rs:163

#### P9-agt-loop-system-run (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  Loops have been registered via `register_loop`.
- **Post:** Spawns a tokio task per loop instance; each task ticks
- **File:** crates/hkask-agents/src/loop_system.rs:176

#### P9-agt-loop-system-tick (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  Loops have been registered.
- **Post:** Each registered loop is ticked once in authority order;
- **File:** crates/hkask-agents/src/loop_system.rs:234

#### P9-agt-loop-system-run-ticks (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `max_ticks` > 0.
- **Post:** Calls `tick()` `max_ticks` times sequentially; logs each
- **File:** crates/hkask-agents/src/loop_system.rs:252

#### P9-agt-loop-system-stop (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  (none — idempotent).
- **Post:** The cancellation token is triggered; all spawned tick tasks
- **File:** crates/hkask-agents/src/loop_system.rs:271

#### P9-agt-loop-system-count (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  (none).
- **Post:** Returns the sum of `Vec::len()` across all entries in the
- **File:** crates/hkask-agents/src/loop_system.rs:283

#### P9-agt-loop-system-ids (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  (none).
- **Post:** Returns a `Vec<LoopId>` containing all keys currently in
- **File:** crates/hkask-agents/src/loop_system.rs:294

#### P9-agt-prompt-classify (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `prompt` is a valid UTF-8 string (may be empty).
- **Post:** Returns a `PromptAnalysis` with sentence decompositions, clause
- **File:** crates/hkask-agents/src/prompt_analysis.rs:578

#### P3-agt-registry-loader-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `registry_path` is a valid `PathBuf`; `acp_runtime` is a
- **Post:** Returns an `AgentRegistryLoader` with all fields set.
- **File:** crates/hkask-agents/src/registry_loader.rs:231

#### P3-agt-registry-loader-restore (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  The store schema has been initialized.
- **Post:** If existing agents are found in the store, returns them
- **File:** crates/hkask-agents/src/registry_loader.rs:252

#### P3-agt-registry-loader-load (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  The registry path contains valid YAML agent definitions.
- **Post:** Returns `Ok(Vec<RegisteredAgent>)` with all successfully
- **File:** crates/hkask-agents/src/registry_loader.rs:274

#### P3-agt-registry-loader-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  (none — accessor).
- **Post:** Returns a reference to the inner `AgentRegistryStore`.
- **File:** crates/hkask-agents/src/registry_loader.rs:385

#### P1-agt-sovereignty-checker-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `owner_webid` is a valid `WebID`; `consent` is a valid
- **Post:** Returns a `SovereigntyChecker` with a fresh
- **File:** crates/hkask-agents/src/sovereignty.rs:81

#### P1-agt-sovereignty-checker-can-access (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `data_category` is a valid `DataCategory`; `requester` is a
- **Post:** Returns `true` iff the requester is permitted to access the
- **File:** crates/hkask-agents/src/sovereignty.rs:101

#### P1-agt-sovereignty-checker-can-perform (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  `operation` is a non-empty string; `data_category` is a
- **Post:** For "acquisition", returns `true` iff affirmative consent is
- **File:** crates/hkask-agents/src/sovereignty.rs:120


### hkask-api (8 contracts)

#### API-027 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  environment variables and keystore are configured
- **Post:** if config/secrets available → Ok(ApiState) with full infrastructure;if config/secrets missing → Err(ApiError::Internal)
- **File:** crates/hkask-api/src/lib.rs:97

#### API-028 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx is a fully-built AgentService
- **Post:** returns Ok(ApiState) with all shared infra from ctx;git_cas initialized from ctx or defaults;api_key_auth_service initialized if wallet_store + wallet_service available
- **File:** crates/hkask-api/src/lib.rs:121

#### API-029 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store is a valid Arc<SqliteSpecStore>
- **Post:** self.spec_store = Some(store); returns self
- **File:** crates/hkask-api/src/lib.rs:156

#### API-030 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  svc is a valid Arc<WalletService>
- **Post:** self.wallet_service = Some(svc); returns self
- **File:** crates/hkask-api/src/lib.rs:166

#### API-031 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self.agent_service.loop_system() is initialized
- **Post:** all registered loops begin tick cycles;returns Ok(()) on success, Err(InfrastructureError) on failure
- **File:** crates/hkask-api/src/lib.rs:179

#### API-032 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self.agent_service.loop_system() is initialized
- **Post:** loop system shutdown signal sent; background tasks begin winding down
- **File:** crates/hkask-api/src/lib.rs:195

#### API-033 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  state is a valid ApiState
- **Post:** returns Ok(OpenApiRouter) with all route modules merged;auth middleware layer applied;api_key_auth middleware layer applied if available
- **File:** crates/hkask-api/src/lib.rs:207

#### API-034 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none
- **Post:** returns OpenApi with all route paths documented
- **File:** crates/hkask-api/src/lib.rs:259


### hkask-cli (2 contracts)

#### CLI-006 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  vd_json is a JSON string (may be invalid)
- **Post:** returns voice preset name from JSON fields (elevenlabs_voice, preset, name);if no voice field found → returns "custom";if JSON parse fails → returns "Rachel"
- **File:** crates/hkask-cli/src/lib.rs:13

#### CLI-ONBOARDING-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  user must not cancel at any interactive prompt
- **Post:** returns OnboardingOutcome with signed_in_agent, resolved_secrets, selected_model, is_first_run=true; all secrets derived and stored in keychain; replicant registered in ACP; user profile stored; matrix registration attempted (non-blocking)
- **Inv:**  does not modify any external state before derive_secrets; cancellation at any prompt returns OnboardingError::Cancelled with zero side effects
- **File:** crates/hkask-cli/src/onboarding.rs:225


### hkask-cns (91 contracts)

#### P9-cns-algedonic-alert-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty, threshold > 0
- **Post:** returns RuntimeAlert with severity based on deficit vs threshold
- **File:** crates/hkask-cns/src/algedonic.rs:57

#### P9-cns-algedonic-alert-should-escalate (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns true iff severity is Critical
- **File:** crates/hkask-cns/src/algedonic.rs:88

#### P9-cns-algedonic-alert-is-critical (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns true iff severity == Critical
- **File:** crates/hkask-cns/src/algedonic.rs:98

#### P9-cns-algedonic-alert-is-warning (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns true iff severity == Warning
- **File:** crates/hkask-cns/src/algedonic.rs:108

#### P9-cns-api-meter-endpoint-weight (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  path is non-empty
- **Post:** returns EndpointWeight based on path pattern
- **File:** crates/hkask-cns/src/api_metering.rs:33

#### P9-cns-api-meter-rate-limit-status (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns lowercase alert type string
- **File:** crates/hkask-cns/src/api_metering.rs:116

#### P9-cns-api-meter-new (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns ApiMeter with empty buckets
- **File:** crates/hkask-cns/src/api_metering.rs:144

#### P9-cns-api-meter-check-and-record (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is valid
- **Post:** returns Ok if within limit, Err if rate limited
- **File:** crates/hkask-cns/src/api_metering.rs:167

#### P9-cns-api-meter-current-rpm (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is valid
- **Post:** returns current requests per minute
- **File:** crates/hkask-cns/src/api_metering.rs:202

#### P9-cns-api-meter-span-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  path and method are non-empty
- **Post:** returns ApiRequestSpan
- **File:** crates/hkask-cns/src/api_metering.rs:245

#### P9-cns-api-meter-alert-type (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns alert type label
- **File:** crates/hkask-cns/src/api_metering.rs:298

#### P9-cns-api-meter-alert-severity (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns severity label
- **File:** crates/hkask-cns/src/api_metering.rs:315

#### GAS-CALIB-004—runtimecalibrationloopwiredtoproductionestimator (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-cns/src/calibrated_energy_estimator.rs:37

#### GAS-CALIB-004 (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-cns/src/calibrated_energy_estimator.rs:42

#### GAS-CALIB-004—runtimecalibrationloopwiredtoproductionestimator (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store is a valid NuEventStore
- **Post:** returns CalibratedEnergyEstimator with default table and no observations;first calibration will look back `DEFAULT_INITIAL_LOOKBACK`;no event sink attached until `with_event_sink` is called
- **File:** crates/hkask-cns/src/calibrated_energy_estimator.rs:67

#### GAS-CALIB-004 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  lookback is a positive duration
- **Post:** first calibration will search [Utc::now() - lookback, Utc::now()]
- **File:** crates/hkask-cns/src/calibrated_energy_estimator.rs:86

#### GAS-CALIB-004-obs—calibrationadjustmentsemitcns.gasspans (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  sink is a valid NuEventSink
- **Post:** subsequent successful calibrations that adjust costs emit a span
- **File:** crates/hkask-cns/src/calibrated_energy_estimator.rs:99

#### GAS-CALIB-004 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  `self.store` is a valid NuEventStore
- **Post:** all settled gas events since the last calibration are fed into
- **File:** crates/hkask-cns/src/calibrated_energy_estimator.rs:110

#### GAS-CALIB-004 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  interval > 0
- **Post:** a Tokio task is spawned; it calls `calibrate()` every `interval`
- **File:** crates/hkask-cns/src/calibrated_energy_estimator.rs:193

#### GAS-CALIB-004 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns a copy of the internal server_costs map
- **File:** crates/hkask-cns/src/calibrated_energy_estimator.rs:215

#### P9-cns-circuit-default-for-inference (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  name is non-empty
- **Post:** returns CircuitBreaker with default thresholds
- **File:** crates/hkask-cns/src/circuit_breaker.rs:75

#### P9-cns-circuit-allow-request (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns true if circuit is closed or half-open, false if open
- **File:** crates/hkask-cns/src/circuit_breaker.rs:86

#### P9-cns-circuit-record-success (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** success counted, may transition circuit to closed
- **File:** crates/hkask-cns/src/circuit_breaker.rs:125

#### P9-cns-est-composite-new (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns CompositeEnergyEstimator with empty estimators
- **File:** crates/hkask-cns/src/composite_energy_estimator.rs:24

#### GAS-CALIB-003—calibratedtablereplaceshardcodedTableEnergyEstimatorcosts (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  table was calibrated (or default) via DynamicGasTable::calibrate()
- **Post:** estimate_cost(server, ...) uses table.report_table()[server] for non-inference servers
- **File:** crates/hkask-cns/src/composite_energy_estimator.rs:40

#### GAS-CALIB-001 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  `server_costs` contains known servers with initialized costs
- **Post:** after `calibrate()`, costs reflect EMA-smoothed actual/estimated ratios
- **File:** crates/hkask-cns/src/dynamic_gas_table.rs:41

#### GAS-CALIB-001 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns DynamicGasTable with default server costs and no observations
- **File:** crates/hkask-cns/src/dynamic_gas_table.rs:78

#### GAS-CALIB-002—singleobservationinitializesEMAperserver (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  estimated_gas > 0 (no division by zero)
- **Post:** ema_ratios[server] updated with EMA of actual/estimated ratio;observation_counts[server] incremented
- **File:** crates/hkask-cns/src/dynamic_gas_table.rs:101

#### GAS-CALIB-001 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** server_costs[server] is updated if its EMA ratio exceeds tolerance;returns the number of servers whose costs were adjusted
- **File:** crates/hkask-cns/src/dynamic_gas_table.rs:135

#### GAS-CALIB-001 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns a HashMap<String, u64> of server → cost mappings
- **File:** crates/hkask-cns/src/dynamic_gas_table.rs:168

#### GAS-CALIB-002 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns a HashMap<String, f64> of server → EMA ratio mappings
- **File:** crates/hkask-cns/src/dynamic_gas_table.rs:179

#### GAS-CALIB-002 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns the count of recorded observations for `server`, or 0 if unobserved
- **File:** crates/hkask-cns/src/dynamic_gas_table.rs:189

#### P8-cns-energy-cost-from-raw (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** result.0 == value
- **File:** crates/hkask-cns/src/energy.rs:35

#### P8-cns-energy-cost-as-raw (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** result == self.0
- **File:** crates/hkask-cns/src/energy.rs:45

#### P8-cns-energy-delta-from-raw (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** result.0 == value
- **File:** crates/hkask-cns/src/energy.rs:111

#### P8-cns-energy-delta-as-raw (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** result == self.0
- **File:** crates/hkask-cns/src/energy.rs:121

#### P9-cns-energy-delta-descending (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** result == (self.0 <= 0.0)
- **File:** crates/hkask-cns/src/energy.rs:133

#### P9-cns-energy-delta-ascending (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** result == (self.0 > 0.0);is_ascending() == !is_descending()  self.0 == 0.0
- **File:** crates/hkask-cns/src/energy.rs:143

#### P9-cns-energy-budget-invariant (🟡 partial)

- **Principle:** ✅ anchored
- **Inv:** remaining + reserved ≤ cap (budget cap invariant);remaining ≥ 0, reserved ≥ 0
- **File:** crates/hkask-cns/src/energy.rs:194

#### P9-cns-energy-budget-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  cap > 0
- **Post:** remaining == cap, reserved == 0, hard_limit == true;replenish_rate == cap / 10, alert_threshold == DEFAULT_ENERGY_ALERT_THRESHOLD
- **File:** crates/hkask-cns/src/energy.rs:227

#### P9-cns-energy-budget-unlimited (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** cap == u64::MAX, hard_limit == false
- **File:** crates/hkask-cns/src/energy.rs:250

#### P9-cns-energy-budget-with-replenish-rate (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** self.replenish_rate == rate
- **File:** crates/hkask-cns/src/energy.rs:263

#### P9-cns-energy-budget-with-alert-threshold (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  threshold is a valid ratio
- **Post:** self.alert_threshold == threshold.clamp(0.0, 1.0)
- **File:** crates/hkask-cns/src/energy.rs:274

#### P9-cns-energy-budget-with-hard-limit (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** self.hard_limit == hard
- **File:** crates/hkask-cns/src/energy.rs:286

#### P9-cns-energy-budget-can-proceed (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gas is a valid EnergyCost
- **Post:** returns true iff gas <= available OR hard_limit is false
- **File:** crates/hkask-cns/src/energy.rs:297

#### P9-cns-energy-budget-available (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** [NORMATIVE] post: result >= 0 (available never negative) (P9 — Homeostatic Self-Regulation);result == remaining.saturating_sub(reserved)
- **File:** crates/hkask-cns/src/energy.rs:310

#### P9-cns-energy-budget-reserve (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gas is a valid EnergyCost
- **Post:** if hard_limit && gas > available → Err(BudgetExceeded);if Ok → reserved increased by gas, remaining unchanged
- **Inv:**  remaining + reserved ≤ cap (maintained)
- **File:** crates/hkask-cns/src/energy.rs:321

#### P9-cns-energy-budget-settle (🟢 full)

- **Principle:** ✅ anchored
- **Pre:** [NORMATIVE] pre:  reserved_gas ≤ self.reserved (caller must track reservations) (P9 — Homeostatic Self-Regulation)
- **Post:** reserved decreased by reserved_gas;if hard_limit && actual > remaining → Err(BudgetExceeded);if Ok → remaining decreased by actual
- **File:** crates/hkask-cns/src/energy.rs:346

#### P9-cns-energy-budget-consume (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gas is a valid EnergyCost
- **Post:** if hard_limit && gas > remaining → Err(BudgetExceeded);if Ok → remaining decreased by gas
- **Inv:**  remaining + reserved ≤ cap (maintained)
- **File:** crates/hkask-cns/src/energy.rs:383

#### P9-cns-energy-budget-replenish (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** remaining ≤ cap (never exceeds cap);if replenish_rate > 0 → remaining increased by up to replenish_rate
- **File:** crates/hkask-cns/src/energy.rs:406

#### P9-cns-energy-budget-replenish-by (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  amount is a valid EnergyCost
- **Post:** remaining ≤ cap (never exceeds cap);remaining increased by up to amount
- **File:** crates/hkask-cns/src/energy.rs:427

#### P9-cns-energy-budget-replenish-by-weighted (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  amount is a valid EnergyCost, priority in [0.0, 1.0]
- **Post:** remaining ≤ cap (never exceeds cap);returns the actual amount replenished (≥ 1 if amount * priority > 0)
- **File:** crates/hkask-cns/src/energy.rs:439

#### GAS-CALIB-003—GasReportsettledeventsfeedDynamicGasTable (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  `table` is a valid DynamicGasTable
- **Post:** every `cns.gas.settled` event in [since, until) with a server field
- **File:** crates/hkask-cns/src/gas_report.rs:263

#### P9-cns-gov-inf-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  inference is valid, cns is valid
- **Post:** returns GovernedInference
- **File:** crates/hkask-cns/src/governed_inference.rs:59

#### P12-cns-gov-inf-with-agent (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Self with agent set (builder pattern)
- **File:** crates/hkask-cns/src/governed_inference.rs:83

#### P9-cns-gov-tool-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  inner is valid, cns is valid
- **Post:** returns GovernedTool
- **File:** crates/hkask-cns/src/governed_tool.rs:94

#### P9-cns-gov-tool-consumption-channel (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Self with channel set (builder pattern)
- **File:** crates/hkask-cns/src/governed_tool.rs:123

#### P12-cns-gov-tool-with-agent (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Self with agent set (builder pattern)
- **File:** crates/hkask-cns/src/governed_tool.rs:139

#### P9-cns-runtime-variety-monitor-new (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns VarietyMonitor with empty counters
- **File:** crates/hkask-cns/src/runtime.rs:192

#### P9-cns-runtime-variety-for-domain (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty
- **Post:** returns variety count, 0 if domain not tracked
- **File:** crates/hkask-cns/src/runtime.rs:208

#### P9-cns-runtime-variety-monitor-domains (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of domain name strings
- **File:** crates/hkask-cns/src/runtime.rs:219

#### P9-cns-runtime-with-threshold (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  threshold > 0
- **Post:** returns CnsRuntime with configured threshold
- **File:** crates/hkask-cns/src/runtime.rs:278

#### P9-cns-runtime-health (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns CnsHealth with current state
- **File:** crates/hkask-cns/src/runtime.rs:294

#### P9-cns-runtime-alerts (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of RuntimeAlert
- **File:** crates/hkask-cns/src/runtime.rs:308

#### P9-cns-runtime-default-threshold (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns threshold value from algedonic manager
- **File:** crates/hkask-cns/src/runtime.rs:320

#### P9-cns-runtime-critical-alerts (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of critical RuntimeAlert
- **File:** crates/hkask-cns/src/runtime.rs:331

#### P9-cns-runtime-variety (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns HashMap of namespace → variety count
- **File:** crates/hkask-cns/src/runtime.rs:352

#### P9-cns-runtime-variety-for-domain (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty
- **Post:** returns variety count for domain
- **File:** crates/hkask-cns/src/runtime.rs:383

#### P3-cns-runtime-blocking-variety-for-domain (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty
- **Post:** returns variety count
- **File:** crates/hkask-cns/src/runtime.rs:398

#### P9-cns-runtime-record-outcome (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty
- **Post:** outcome tracked for domain
- **File:** crates/hkask-cns/src/runtime.rs:418

#### P9-cns-runtime-check-outcome (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty
- **Post:** returns Some(alert) if success rate below threshold, None if healthy
- **File:** crates/hkask-cns/src/runtime.rs:443

#### P9-cns-runtime-outcome-success-rate (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty
- **Post:** returns Some(rate) if domain tracked, None otherwise
- **File:** crates/hkask-cns/src/runtime.rs:478

#### P9-cns-runtime-increment-variety (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain and state_name are non-empty
- **Post:** variety counter incremented
- **File:** crates/hkask-cns/src/runtime.rs:493

#### P9-cns-runtime-check-variety (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty
- **Post:** returns Some(alert) if variety below threshold, None if healthy
- **File:** crates/hkask-cns/src/runtime.rs:533

#### P7-cns-runtime-calibrate-threshold (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty, new_threshold > 0
- **Post:** threshold updated for domain
- **File:** crates/hkask-cns/src/runtime.rs:569

#### P3-cns-runtime-calibrate-threshold-blocking (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty, new_threshold > 0
- **Post:** threshold updated
- **File:** crates/hkask-cns/src/runtime.rs:591

#### P12-cns-runtime-subscribe (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  observer is valid
- **Post:** observer added to subscribers
- **File:** crates/hkask-cns/src/runtime.rs:617

#### P12-cns-runtime-subscribe-async (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  observer is valid
- **Post:** observer added to subscribers
- **File:** crates/hkask-cns/src/runtime.rs:633

#### P9-cns-runtime-emit-backpressure (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  signal is valid
- **Post:** backpressure signal emitted to subscribers
- **File:** crates/hkask-cns/src/runtime.rs:649

#### P9-cns-runtime-register-energy-budget (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent is valid, budget is valid
- **Post:** budget registered for agent
- **File:** crates/hkask-cns/src/runtime.rs:666

#### P9-cns-runtime-replenish-agent-budget (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent is registered, amount > 0
- **Post:** budget replenished, returns actual amount added
- **File:** crates/hkask-cns/src/runtime.rs:683

#### P9-cns-runtime-agent-gas-status (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent is valid
- **Post:** returns Some(status) if budget exists, None otherwise
- **File:** crates/hkask-cns/src/runtime.rs:713

#### GAS-CALIB-003—calibratedtablereplaceshardcodedTableEnergyEstimatorcosts (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  `server_costs` contains the desired server → cost mappings
- **Post:** per-tool overrides (e.g. condenser_thread_summary) are still applied
- **File:** crates/hkask-cns/src/table_energy_estimator.rs:114

#### GAS-CALIB-006—walletestimatorusescalibratedper-servercosts (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  gas_per_rjoule > 0
- **Post:** returns WalletEnergyEstimator with the supplied inner estimator
- **File:** crates/hkask-cns/src/wallet_energy_estimator.rs:48

#### P9-cns-wallet-est-calibrate (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  observed_ratio > 0.0 (actual_gas / estimated_gas)
- **Post:** ema_ratio updated via exponential moving average;if ema_ratio deviates significantly from 1.0, gas_per_rjoule adjusted
- **File:** crates/hkask-cns/src/wallet_energy_estimator.rs:63

#### GAS-CALIB-005—runtimecalibrationofwalletgasconversionrate (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-cns/src/wallet_gas_calibrator.rs:23

#### GAS-CALIB-005 (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-cns/src/wallet_gas_calibrator.rs:28

#### GAS-CALIB-005—runtimecalibrationofwalletgasconversionrate (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store is a valid NuEventStore; wallet_manager is valid
- **Post:** returns WalletGasCalibrator seeded with the manager's current gas_per_rjoule rate;first calibration will look back `DEFAULT_WALLET_INITIAL_LOOKBACK`
- **File:** crates/hkask-cns/src/wallet_gas_calibrator.rs:49

#### GAS-CALIB-005 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  lookback is a positive duration
- **Post:** first calibration will search [Utc::now() - lookback, Utc::now()]
- **File:** crates/hkask-cns/src/wallet_gas_calibrator.rs:67

#### GAS-CALIB-005 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  `self.store` is a valid NuEventStore; `self.wallet_manager` is valid
- **Post:** if settled events exist and the aggregate ratio exceeds tolerance,;returns true if the rate was adjusted
- **File:** crates/hkask-cns/src/wallet_gas_calibrator.rs:84

#### GAS-CALIB-005 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  interval > 0
- **Post:** a Tokio task is spawned; it calls `calibrate()` every `interval`
- **File:** crates/hkask-cns/src/wallet_gas_calibrator.rs:138


### hkask-communication (25 contracts)

#### COMM-013 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns AgentRegistry with empty entries and watchlists
- **File:** crates/hkask-communication/src/agent_registration.rs:36

#### COMM-014 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid is a valid WebID, user_id is a valid Matrix UserId
- **Post:** mapping stored in entries;idempotent — overwrites existing mapping for same webid
- **File:** crates/hkask-communication/src/agent_registration.rs:48

#### COMM-015 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid is a valid WebID
- **Post:** mapping removed from entries if present;idempotent — removing non-existent entry is Ok(())
- **File:** crates/hkask-communication/src/agent_registration.rs:67

#### COMM-016 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid is a valid WebID
- **Post:** returns Some(UserId) if mapping exists;returns None if no mapping for webid
- **File:** crates/hkask-communication/src/agent_registration.rs:86

#### COMM-017 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid is registered (record_mapping called); room_id is a valid RoomId
- **Post:** room_id added to agent's watchlist;returns Err(NotRegistered) if webid not in entries
- **File:** crates/hkask-communication/src/agent_registration.rs:96

#### COMM-018 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  room_id is a valid RoomId
- **Post:** returns Vec of WebID strings watching this thread;returns empty Vec if no watchers
- **File:** crates/hkask-communication/src/agent_registration.rs:130

#### COMM-019 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  matrix is a valid MatrixTransport (authenticated); poll_interval_secs > 0
- **Post:** returns SevenR7Listener with active=false
- **File:** crates/hkask-communication/src/listener.rs:35

#### COMM-020 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  matrix transport is authenticated
- **Post:** background polling task spawned;idempotent — calling start() on already-active listener is no-op
- **File:** crates/hkask-communication/src/listener.rs:54

#### COMM-021 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** active flag set to false;idempotent — calling stop() on already-stopped listener is no-op
- **File:** crates/hkask-communication/src/listener.rs:127

#### COMM-022 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is a valid Matrix room ID (e.g., "!abc123:localhost")
- **Post:** returns RoomId wrapping the string
- **File:** crates/hkask-communication/src/matrix.rs:29

#### COMM-023 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns &str of the inner room ID
- **File:** crates/hkask-communication/src/matrix.rs:38

#### COMM-024 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is a valid Matrix user ID (e.g., "@agent:localhost")
- **Post:** returns UserId wrapping the string
- **File:** crates/hkask-communication/src/matrix.rs:52

#### COMM-025 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns &str of the inner user ID
- **File:** crates/hkask-communication/src/matrix.rs:61

#### COMM-001 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  homeserver_url is a valid URL string
- **Post:** returns MatrixTransport with client=None, homeserver_url set
- **File:** crates/hkask-communication/src/matrix.rs:134

#### COMM-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  homeserver_url is set
- **Post:** returns Ok(true) if homeserver responds;returns Err(Unavailable) if homeserver is unreachable
- **File:** crates/hkask-communication/src/matrix.rs:148

#### COMM-003 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  username and password are non-empty
- **Post:** if successful, self.client is set to authenticated client;returns Err(Auth) if credentials are invalid;returns Err(Unavailable) if homeserver is unreachable
- **File:** crates/hkask-communication/src/matrix.rs:174

#### COMM-004 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  client is authenticated (login() called); room_id is a valid Matrix room ID; limit > 0
- **Post:** returns Vec<MatrixMessage> with at most `limit` messages;returns Err(NotLoggedIn) if not authenticated;returns Err(Room) if room not found
- **File:** crates/hkask-communication/src/matrix.rs:210

#### COMM-005 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  client is authenticated (login() called); room_id is a valid Matrix room ID; body is non-empty
- **Post:** message sent to room;returns Err(NotLoggedIn) if not authenticated;returns Err(Network) if send fails
- **File:** crates/hkask-communication/src/matrix.rs:290

#### COMM-006 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  client is authenticated (login() called); name is non-empty
- **Post:** returns Ok(RoomId) for the newly created room;room name is set to `name`;returns Err(NotLoggedIn) if not authenticated
- **File:** crates/hkask-communication/src/matrix.rs:339

#### COMM-007 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  client is authenticated (login() called); room_id is a valid Matrix room ID; user_id is a valid Matrix user ID
- **Post:** user invited to room;returns Err(NotLoggedIn) if not authenticated;returns Err(Room) if room not found or invite fails
- **File:** crates/hkask-communication/src/matrix.rs:379

#### COMM-008 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  client is authenticated (login() called)
- **Post:** returns Vec<Thread> with all joined rooms;each Thread has room_id, title, and participants populated;returns Err(NotLoggedIn) if not authenticated
- **File:** crates/hkask-communication/src/matrix.rs:415

#### COMM-009 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff client is Some (login() succeeded)
- **File:** crates/hkask-communication/src/matrix.rs:452

#### COMM-010 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  HKASK_MATRIX_AGENT_USERNAME and HKASK_MATRIX_AGENT_PASSWORD env vars are set
- **Post:** self.client is reset and re-authenticated;returns Err(Auth) if env vars are missing or credentials invalid
- **File:** crates/hkask-communication/src/matrix.rs:465

#### COMM-011 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff client is authenticated and whoami succeeds;does not mutate self
- **File:** crates/hkask-communication/src/matrix.rs:493

#### COMM-012 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Some(user_id) if authenticated;returns None if not authenticated or whoami fails
- **File:** crates/hkask-communication/src/matrix.rs:508


### hkask-inference (86 contracts)

#### INFER-026 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is non-empty, prompt is non-empty
- **Post:** returns serde_json::Value with model, messages, and parameters
- **File:** crates/hkask-inference/src/chat_protocol.rs:71

#### INFER-027 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  calls is a valid slice of RawToolCall
- **Post:** returns Vec<StructuredToolCall> with parsed arguments
- **File:** crates/hkask-inference/src/chat_protocol.rs:201

#### INFER-028 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  probs is a valid slice of RawTokenProb
- **Post:** returns Vec<TokenProbability> with mapped fields
- **File:** crates/hkask-inference/src/chat_protocol.rs:227

#### INFER-029 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  response is a valid ChatResponse
- **Post:** returns Ok(InferenceResult) with text, usage, finish_reason;returns Err if no choices in response
- **File:** crates/hkask-inference/src/chat_protocol.rs:251

#### INFER-030 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  stream is a valid SSE byte stream
- **Post:** returns stream of InferenceStreamChunk parsed from SSE data lines
- **File:** crates/hkask-inference/src/chat_protocol.rs:287

#### INFER-001 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  prompt is a valid &str
- **Post:** returns Err(Generation) if prompt is empty;returns Err(Generation) if prompt.len() > 1_000_000
- **File:** crates/hkask-inference/src/chat_protocol.rs:351

#### chat-proto-001—ChatResponsedeserializesOpenAI-compatibleformat (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/chat_protocol.rs:369

#### chat-proto-002—build_chat_requestproducesvalidJSONwithstream:false (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/chat_protocol.rs:405

#### chat-proto-003—validate_promptrejectsemptyandoverlongprompts (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/chat_protocol.rs:435

#### chat-proto-004—disable_thinkingmapstoenable_thinking:falseinwireformat (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/chat_protocol.rs:442

#### chat-proto-005—enable_thinkingisomittedfromJSONwhentrue(default) (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/chat_protocol.rs:463

#### INFER-021 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is non-empty
- **Post:** returns Some((ProviderId, stripped_model)) for OM/, DI/, FA/, TG/ prefixes;returns None for unrecognized or missing prefix
- **File:** crates/hkask-inference/src/config.rs:59

#### INFER-022 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is non-empty
- **Post:** returns "{prefix}/{model}" string
- **File:** crates/hkask-inference/src/config.rs:87

#### INFER-023 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns "OM", "DI", "FA", or "TG"
- **File:** crates/hkask-inference/src/config.rs:96

#### INFER-024 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns InferenceConfig resolved from env vars and keychain;defaults to Ollama localhost if env vars unset
- **File:** crates/hkask-inference/src/config.rs:185

#### INFER-025 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns reqwest::Client with timeout and pool settings from config
- **File:** crates/hkask-inference/src/config.rs:227

#### inf-cfg-001—ProviderId::parse_from_modelparsesallthreeprefixes (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:299

#### inf-cfg-002—unprefixedmodelnamesreturnNone (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:316

#### inf-cfg-003—emptymodelafterprefixreturnsNone (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:323

#### inf-cfg-004—too-shortstringsreturnNone (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:331

#### inf-cfg-005—unknownprefixreturnsNone (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:339

#### inf-cfg-006—prefix_modelformatscorrectlyforallproviders (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:346

#### inf-cfg-007—FA/prefixparsescorrectly (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:361

#### inf-cfg-008—parse_provider_codeparsesallfourprovidercodes (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:376

#### inf-cfg-009—unknownoremptyprovidercodedefaultstoOllama (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:385

#### inf-cfg-010—resolve_api_keyreadsfromprimaryenvvar (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:396

#### inf-cfg-011—resolve_api_keyfallsbacktolegacyenvvarnames (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:409

#### inf-cfg-012—resolve_api_keyreturnsemptywhennokeyfound (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:422

#### inf-cfg-013—resolve_api_keyprefersprimaryoverfallback (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/config.rs:436

#### INFER-010 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config.deepinfra_api_key is set
- **Post:** returns DeepInfraBackend with configured HTTP client
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:33

#### INFER-033 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid DeepInfra model name; prompt is non-empty (validated by validate_prompt); params is a valid LLMParameters
- **Post:** returns Ok(InferenceResult) with generated text, model, usage stats;if connection fails → Err(InferenceError::Connection);if prompt is empty → Err(InferenceError::Generation)
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:55

#### INFER-034 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid DeepInfra vision-capable model name; prompt is non-empty; images is non-empty (at least one base64-encoded image); params is a valid LLMParameters
- **Post:** returns Ok(InferenceResult) with vision-generated text;if images is empty → Err(InferenceError::Generation("No images provided"))
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:108

#### INFER-011 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid DeepInfra model name
- **Post:** returns stream of inference chunks
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:173

#### INFER-035 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self.client and self.base_url are initialized
- **Post:** returns Ok(Vec<DeepInfraModelEntry>) with models updated in last 180 days;if API returns non-success → Ok(Vec::new()) (graceful degradation);if connection fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:238

#### INFER-036 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL
- **Post:** returns Ok(serde_json::Value) with background-removed image data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:316

#### INFER-037 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  prompt is a non-empty text description
- **Post:** returns Ok(serde_json::Value) with generated image data (1024x1024);if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:331

#### INFER-038 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL; prompt is a non-empty edit instruction
- **Post:** returns Ok(serde_json::Value) with edited image data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:352

#### INFER-039 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  text is non-empty; voice_id is a valid voice identifier
- **Post:** returns Ok(serde_json::Value) with base64-encoded MP3 audio;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:374

#### INFER-040 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  audio_url is a valid, accessible audio file URL
- **Post:** returns Ok(serde_json::Value) with verbose_json transcription (word+segment timestamps);if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/deepinfra_backend.rs:433

#### INFER-018 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config is a valid InferenceConfig
- **Post:** returns EmbeddingRouter with configured backends
- **File:** crates/hkask-inference/src/embedding_router.rs:26

#### INFER-031 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid provider-prefixed model name; sentences is non-empty
- **Post:** returns Vec<Vec<f32>> with one vector per sentence, same order;if sentences is empty → Err(EmptyResponse);if provider is Fal → Err(Connection) (fal.ai does not support embeddings)
- **File:** crates/hkask-inference/src/embedding_router.rs:81

#### INFER-032 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid provider-prefixed model name; sentence is a non-empty string
- **Post:** returns Vec<f32> — the first (only) embedding vector;delegates to embed_sentences, inherits its error conditions
- **File:** crates/hkask-inference/src/embedding_router.rs:147

#### INFER-012 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config.fal_api_key is set
- **Post:** returns FalBackend with configured HTTP client
- **File:** crates/hkask-inference/src/fal_backend.rs:33

#### INFER-047 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid fal.ai model name; prompt is non-empty (validated by validate_prompt); params is a valid LLMParameters
- **Post:** returns Ok(InferenceResult) with generated text, model, usage stats;if connection fails → Err(InferenceError::Connection);if prompt is empty → Err(InferenceError::Generation)
- **File:** crates/hkask-inference/src/fal_backend.rs:55

#### INFER-048 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid fal.ai vision-capable model name; prompt is non-empty; images is non-empty (at least one base64-encoded image); params is a valid LLMParameters
- **Post:** returns Ok(InferenceResult) with vision-generated text;if images is empty → Err(InferenceError::Generation("No images provided"))
- **File:** crates/hkask-inference/src/fal_backend.rs:108

#### INFER-013 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid Fal model name
- **Post:** returns stream of inference chunks
- **File:** crates/hkask-inference/src/fal_backend.rs:173

#### INFER-049 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (static catalog, no API call)
- **Post:** returns Ok(Vec<FalModelEntry>) with curated model list
- **File:** crates/hkask-inference/src/fal_backend.rs:242

#### INFER-050 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  prompt is a non-empty text description
- **Post:** returns Ok(serde_json::Value) with generated image data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/fal_backend.rs:405

#### INFER-051 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL; prompt is a non-empty transformation instruction
- **Post:** returns Ok(serde_json::Value) with transformed image data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/fal_backend.rs:426

#### INFER-052 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL
- **Post:** returns Ok(serde_json::Value) with background-removed image data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/fal_backend.rs:451

#### INFER-053 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL
- **Post:** returns Ok(serde_json::Value) with upscaled image data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/fal_backend.rs:466

#### INFER-054 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  prompt is a non-empty text description
- **Post:** returns Ok(serde_json::Value) with generated video data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/fal_backend.rs:485

#### INFER-055 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL
- **Post:** returns Ok(serde_json::Value) with generated video data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/fal_backend.rs:505

#### INFER-056 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL; object_description is a non-empty description of the object to segment
- **Post:** returns Ok(serde_json::Value) with segmented object data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/fal_backend.rs:529

#### INFER-057 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  text is non-empty; voice is a valid voice preset name
- **Post:** returns Ok(serde_json::Value) with generated speech audio data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/fal_backend.rs:556

#### INFER-058 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  audio_url is a valid, accessible audio file URL
- **Post:** returns Ok(serde_json::Value) with transcription data;if API call fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/fal_backend.rs:577

#### inf-fal-01—ConstructionfailswithoutAPIkey (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/fal_backend.rs:601

#### inf-fal-02—ConstructionsucceedswithAPIkey (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/fal_backend.rs:616

#### inf-fal-03—Staticcatalogreturnsknownvisionmodels (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/fal_backend.rs:631

#### inf-fal-04—Visionsupportheuristicrecognizesfal.aimodels (🔴 bare)

- **Principle:** ⚠ unanchored
- **File:** crates/hkask-inference/src/fal_backend.rs:653

#### INFER-019 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config is a valid InferenceConfig
- **Post:** returns InferenceRouter with backends for configured providers
- **File:** crates/hkask-inference/src/inference_router.rs:44

#### INFER-059 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  backends are initialized (may be None)
- **Post:** returns Vec<RouterModelEntry> with all available models across providers;if a backend fails → its models are omitted (graceful degradation)
- **File:** crates/hkask-inference/src/inference_router.rs:107

#### INFER-060 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  query may be empty (returns all models)
- **Post:** returns Vec<RouterModelEntry> filtered by case-insensitive substring match;if query is empty → returns all models (delegates to list_models)
- **File:** crates/hkask-inference/src/inference_router.rs:197

#### INFER-061 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (delegates to list_models)
- **Post:** returns Vec<RouterModelEntry> filtered to supports_vision == Some(true)
- **File:** crates/hkask-inference/src/inference_router.rs:217

#### INFER-062 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  prompt is non-empty; images is non-empty; params is a valid LLMParameters
- **Post:** dispatches to provider-resolved backend's generate_vision;returns Ok(InferenceResult) on success;if provider resolution fails → Err(InferenceError)
- **File:** crates/hkask-inference/src/inference_router.rs:230

#### INFER-063 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  prompt is a non-empty text description
- **Post:** returns Ok(serde_json::Value) with generated image data;if fal backend unavailable → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/inference_router.rs:299

#### INFER-064 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL; prompt is a non-empty transformation instruction
- **Post:** returns Ok(serde_json::Value) with transformed image data;if fal backend unavailable → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/inference_router.rs:321

#### INFER-065 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL
- **Post:** tries DeepInfra first, falls back to fal.ai on failure;returns Ok(serde_json::Value) with background-removed image data;if no backend available → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/inference_router.rs:341

#### INFER-066 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL
- **Post:** returns Ok(serde_json::Value) with upscaled image data;if fal backend unavailable → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/inference_router.rs:369

#### INFER-067 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  prompt is a non-empty text description
- **Post:** returns Ok(serde_json::Value) with generated video data;if fal backend unavailable → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/inference_router.rs:387

#### INFER-068 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL
- **Post:** returns Ok(serde_json::Value) with generated video data;if fal backend unavailable → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/inference_router.rs:407

#### INFER-069 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  text is non-empty; voice is a valid voice preset name
- **Post:** tries DeepInfra first, falls back to fal.ai on failure;returns Ok(serde_json::Value) with generated speech audio data;if no backend available → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/inference_router.rs:428

#### INFER-070 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  image_url is a valid, accessible image URL; object_description is a non-empty description of the object to segment
- **Post:** returns Ok(serde_json::Value) with segmented object data;if fal backend unavailable → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/inference_router.rs:458

#### INFER-071 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  audio_url is a valid, accessible audio file URL
- **Post:** tries DeepInfra first, falls back to fal.ai on failure;returns Ok(serde_json::Value) with transcription data;if no backend available → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/inference_router.rs:479

#### INFER-072 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  _text may be any string (currently ignored)
- **Post:** always returns Err(EmbeddingGenerationError::Connection) — not yet implemented
- **File:** crates/hkask-inference/src/inference_router.rs:750

#### INFER-020 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is non-empty
- **Post:** returns Some(true) if model/family matches known vision families;returns None if unknown
- **File:** crates/hkask-inference/src/lib.rs:77

#### INFER-014 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config.ollama_base_url is set
- **Post:** returns OllamaBackend with configured HTTP client
- **File:** crates/hkask-inference/src/ollama_backend.rs:27

#### INFER-044 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid Ollama model name; prompt is non-empty (validated by validate_prompt); params is a valid LLMParameters
- **Post:** returns Ok(InferenceResult) with generated text, model, usage stats;if connection fails → Err(InferenceError::Connection);if prompt is empty → Err(InferenceError::Generation)
- **File:** crates/hkask-inference/src/ollama_backend.rs:43

#### INFER-045 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid Ollama vision-capable model name; prompt is non-empty; images is non-empty (at least one base64-encoded image); params is a valid LLMParameters
- **Post:** returns Ok(InferenceResult) with vision-generated text;if images is empty → Err(InferenceError::Generation("No images provided"))
- **File:** crates/hkask-inference/src/ollama_backend.rs:95

#### INFER-015 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid Ollama model name
- **Post:** returns stream of inference chunks
- **File:** crates/hkask-inference/src/ollama_backend.rs:159

#### INFER-046 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self.client and self.base_url are initialized
- **Post:** returns Ok(Vec<OllamaModelEntry>) with locally available models;if API returns non-success → Ok(Vec::new()) (graceful degradation);if connection fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/ollama_backend.rs:222

#### INFER-016 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config.together_api_key is set
- **Post:** returns TogetherBackend with configured HTTP client
- **File:** crates/hkask-inference/src/together_backend.rs:44

#### INFER-041 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid Together AI model name; prompt is non-empty (validated by validate_prompt); params is a valid LLMParameters
- **Post:** returns Ok(InferenceResult) with generated text, model, usage stats;if connection fails → Err(InferenceError::Connection);if prompt is empty → Err(InferenceError::Generation)
- **File:** crates/hkask-inference/src/together_backend.rs:66

#### INFER-017 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid Together model name
- **Post:** returns stream of inference chunks
- **File:** crates/hkask-inference/src/together_backend.rs:122

#### INFER-042 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is a valid Together AI vision-capable model name; prompt is non-empty; images is non-empty (at least one base64-encoded image); params is a valid LLMParameters
- **Post:** returns Ok(InferenceResult) with vision-generated text;if connection fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/together_backend.rs:187

#### INFER-043 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self.client and self.base_url are initialized
- **Post:** returns Ok(Vec<TogetherModel>) with all available models;if API returns non-success → Err(InferenceError::Connection);if connection fails → Err(InferenceError::Connection)
- **File:** crates/hkask-inference/src/together_backend.rs:241


### hkask-keystore (28 contracts)

#### KEYSTORE-001 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Inv:** secrets are stored in OS keychain, never in plaintext files
- **File:** crates/hkask-keystore/src/keychain.rs:33

#### KEYSTORE-001 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** service_name is set
- **File:** crates/hkask-keystore/src/keychain.rs:42

#### KEYSTORE-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid is a valid WebID, secret is non-empty
- **Post:** secret stored in OS keychain under service_name + webid.uuid;returns Err(Platform) if keychain is unavailable
- **File:** crates/hkask-keystore/src/keychain.rs:52

#### KEYSTORE-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid is a valid WebID
- **Post:** returns Ok(secret) if stored, Err(NotFound) if not
- **File:** crates/hkask-keystore/src/keychain.rs:69

#### KEYSTORE-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid is a valid WebID
- **Post:** secret removed from OS keychain;idempotent — deleting non-existent entry is no-op (platform-dependent)
- **File:** crates/hkask-keystore/src/keychain.rs:81

#### KEYSTORE-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  key is non-empty, secret is non-empty
- **Post:** secret stored in OS keychain under service_name + key
- **File:** crates/hkask-keystore/src/keychain.rs:98

#### KEYSTORE-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  key is non-empty
- **Post:** returns Ok(secret) if stored, Err(NotFound) if not
- **File:** crates/hkask-keystore/src/keychain.rs:114

#### KEYSTORE-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  key is non-empty
- **Post:** secret removed from OS keychain
- **File:** crates/hkask-keystore/src/keychain.rs:126

#### KEYSTORE-003 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  derivation_context, env_var, keychain_key are valid
- **Post:** tries derivation → env → keychain in order;returns Ok(Zeroizing<Vec<u8>>) on first success;returns Err if all three sources fail
- **File:** crates/hkask-keystore/src/keychain.rs:166

#### KEY-010 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Zeroizing<Vec<u8>> from first successful resolution step
- **File:** crates/hkask-keystore/src/keychain.rs:195

#### KEY-011 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Zeroizing<Vec<u8>> from first successful resolution step;falls back to ACP secret if MCP key unavailable
- **File:** crates/hkask-keystore/src/keychain.rs:220

#### KEY-012 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Zeroizing<Vec<u8>> from first successful resolution step
- **File:** crates/hkask-keystore/src/keychain.rs:242

#### KEY-013 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Zeroizing<Vec<u8>> from first successful resolution step
- **File:** crates/hkask-keystore/src/keychain.rs:262

#### KEY-014 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Zeroizing<Vec<u8>> from env var or keychain
- **File:** crates/hkask-keystore/src/keychain.rs:288

#### KEY-015 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Zeroizing<Vec<u8>> from derivation, keychain, or random generation
- **File:** crates/hkask-keystore/src/keychain.rs:302

#### KEYSTORE-003 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  secret_ref is a valid SecretRef variant
- **Post:** Env → reads from environment variable, Err(NotFound) if unset;Keychain → reads from OS keychain, Err(NotFound) if absent;Derived → resolves master key (env→keychain), HKDF-SHA256 derives sub-key;Generated → random bytes (debug only, not reproducible);all returned secrets wrapped in Zeroizing
- **File:** crates/hkask-keystore/src/keychain.rs:340

#### KEYSTORE-004 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  chain is a valid ChainId (Solana, Hedera, or Hinkal)
- **Post:** returns Ok(Zeroizing<Vec<u8>>) — 32-byte HKDF-derived seed;same master key → same treasury key for given chain (deterministic)
- **File:** crates/hkask-keystore/src/keychain.rs:394

#### KEYSTORE-005 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Ok(Zeroizing<Vec<u8>>) — 32-byte HKDF-derived seed;same master key → same wallet seed (deterministic)
- **File:** crates/hkask-keystore/src/keychain.rs:424

#### KEYSTORE-006 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  capability is a valid, fully-populated ApiKeyCapability
- **Post:** returns Ok(hex_signature) — 128-char hex-encoded Ed25519 signature;wallet seed loaded, used for signing, zeroized within this call
- **File:** crates/hkask-keystore/src/keychain.rs:446

#### KEY-020 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  master_secret is non-empty
- **Post:** returns Ed25519SpecSigner with derived signing key
- **File:** crates/hkask-keystore/src/spec_signer.rs:27

#### KEY-021 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  canonical_json is non-empty
- **Post:** returns 128-char hex-encoded Ed25519 signature
- **File:** crates/hkask-keystore/src/spec_signer.rs:46

#### KEY-022 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  canonical_json is non-empty, hex_signature is 128 hex chars
- **Post:** returns Ok(()) if signature valid, Err otherwise
- **File:** crates/hkask-keystore/src/spec_signer.rs:60

#### KEY-023 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Ed25519 VerifyingKey
- **File:** crates/hkask-keystore/src/spec_signer.rs:82

#### KEY-024 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns 64-char hex-encoded verifying key
- **File:** crates/hkask-keystore/src/spec_signer.rs:90

#### KEY-016 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns PathBuf to ~/.config/hkask/version
- **File:** crates/hkask-keystore/src/version_file.rs:18

#### KEY-017 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns u32 version from file, or DEFAULT_KEY_VERSION if missing
- **File:** crates/hkask-keystore/src/version_file.rs:32

#### KEY-018 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  version is a valid u32
- **Post:** version written to version file
- **File:** crates/hkask-keystore/src/version_file.rs:46

#### KEY-019 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** version incremented by 1 and written to disk;returns new version number
- **File:** crates/hkask-keystore/src/version_file.rs:62


### hkask-mcp (41 contracts)

#### MCP-010 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns AdapterContainer with no configured ports
- **File:** crates/hkask-mcp/src/adapter_container.rs:22

#### MCP-011 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  port is a valid GitCASPort
- **Post:** git_cas_port set
- **File:** crates/hkask-mcp/src/adapter_container.rs:36

#### MCP-012 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Some(port) if configured, None otherwise
- **File:** crates/hkask-mcp/src/adapter_container.rs:52

#### MCP-016 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns PathBuf to the daemon socket (config dir or /tmp fallback)
- **File:** crates/hkask-mcp/src/daemon.rs:32

#### MCP-017 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns DaemonClient with default socket path
- **File:** crates/hkask-mcp/src/daemon.rs:107

#### MCP-018 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  path is a valid filesystem path
- **Post:** returns DaemonClient with custom socket path
- **File:** crates/hkask-mcp/src/daemon.rs:117

#### MCP-019 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns DaemonListener with default socket path, listener=None
- **File:** crates/hkask-mcp/src/daemon.rs:249

#### MCP-020 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  path is a valid filesystem path
- **Post:** returns DaemonListener with custom socket path
- **File:** crates/hkask-mcp/src/daemon.rs:260

#### MCP-013 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  runtime is initialized
- **Post:** returns RawMcpToolPort
- **File:** crates/hkask-mcp/src/dispatch.rs:45

#### MCP-014 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  runtime is initialized, secret is non-empty
- **Post:** returns McpDispatcher with GovernedTool membrane
- **File:** crates/hkask-mcp/src/dispatch.rs:197

#### MCP-015 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  tool_name is non-empty, from and to are valid WebIDs
- **Post:** returns DelegationToken granting tool access from → to
- **File:** crates/hkask-mcp/src/dispatch.rs:214

#### MCP-SCHEMA-001 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  input is a valid JSON Value
- **Post:** returns Ok(()) if input conforms to self.input_schema;returns Err with validation errors if input violates schema;returns Ok(()) if input_schema is empty or not a valid JSON Schema (graceful)
- **File:** crates/hkask-mcp/src/runtime.rs:38

#### MCP-025 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns McpRuntime with empty servers, tool_registry, connections
- **File:** crates/hkask-mcp/src/runtime.rs:114

#### MCP-026 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  env_var and description are non-empty
- **Post:** returns CredentialDecl with required=true
- **File:** crates/hkask-mcp/src/server.rs:48

#### MCP-027 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  env_var and description are non-empty
- **Post:** returns CredentialDecl with required=false
- **File:** crates/hkask-mcp/src/server.rs:62

#### MCP-028 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  resolved_credentials is a valid map
- **Post:** returns CredentialStatus with available/missing counts
- **File:** crates/hkask-mcp/src/server.rs:98

#### MCP-029 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff all required credentials are available
- **File:** crates/hkask-mcp/src/server.rs:130

#### MCP-030 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  db_env_var is set and contains a valid passphrase
- **Post:** returns opened Database
- **File:** crates/hkask-mcp/src/server.rs:154

#### MCP-031 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  db_env_var is set, extensions is valid SQL DDL
- **Post:** returns opened Database with extensions applied
- **File:** crates/hkask-mcp/src/server.rs:173

#### MCP-032 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  tool_name is non-empty, caller is valid
- **Post:** returns ToolSpanGuard with start time recorded
- **File:** crates/hkask-mcp/src/server.rs:208

#### MCP-033 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** CNS tool span emitted with "ok" status;returns output unchanged
- **File:** crates/hkask-mcp/src/server.rs:222

#### MCP-034 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** CNS tool span emitted with "error" status and error kind;returns output unchanged
- **File:** crates/hkask-mcp/src/server.rs:234

#### MCP-035 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** CNS tool span emitted with "ok" status;returns JSON string of value
- **File:** crates/hkask-mcp/src/server.rs:253

#### MCP-036 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** CNS tool span emitted with appropriate status;returns JSON string of Ok value or error
- **File:** crates/hkask-mcp/src/server.rs:263

#### MCP-037 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** CNS tool span emitted with "error" status;returns JSON error string
- **File:** crates/hkask-mcp/src/server.rs:276

#### MCP-038 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  kind is a valid McpErrorKind, message is non-empty
- **Post:** returns McpToolError
- **File:** crates/hkask-mcp/src/server.rs:340

#### MCP-039 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns McpToolError with Internal kind
- **File:** crates/hkask-mcp/src/server.rs:352

#### MCP-040 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns McpToolError with NotFound kind
- **File:** crates/hkask-mcp/src/server.rs:359

#### MCP-041 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns McpToolError with InvalidArgument kind
- **File:** crates/hkask-mcp/src/server.rs:366

#### MCP-042 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns McpToolError with Unavailable kind
- **File:** crates/hkask-mcp/src/server.rs:373

#### MCP-043 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns McpToolError with Timeout kind
- **File:** crates/hkask-mcp/src/server.rs:380

#### MCP-044 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns McpToolError with PermissionDenied kind
- **File:** crates/hkask-mcp/src/server.rs:387

#### MCP-045 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns McpToolError with RateLimited kind
- **File:** crates/hkask-mcp/src/server.rs:394

#### MCP-046 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns McpToolError with FailedPrecondition kind
- **File:** crates/hkask-mcp/src/server.rs:401

#### MCP-047 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns JSON string with "error" object
- **File:** crates/hkask-mcp/src/server.rs:408

#### MCP-048 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  message is non-empty
- **Post:** returns JSON string with error object
- **File:** crates/hkask-mcp/src/server.rs:429

#### MCP-049 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  name and value are non-empty, max_len > 0
- **Post:** returns Ok(()) if valid (non-empty, ≤max_len, alphanumeric+hyphen+underscore);returns Err if invalid
- **File:** crates/hkask-mcp/src/server.rs:445

#### MCP-050 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  url is non-empty
- **Post:** returns Ok(()) if valid http/https URL;returns Err if invalid scheme or format
- **File:** crates/hkask-mcp/src/server.rs:478

#### MCP-051 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  service is non-empty, status is valid
- **Post:** returns McpToolError with appropriate kind based on status code
- **File:** crates/hkask-mcp/src/server.rs:491

#### MCP-052 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns HashMap of env vars from .env file;returns empty map if .env not found
- **File:** crates/hkask-mcp/src/server.rs:563

#### MCP-053 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  env_var is non-empty
- **Post:** returns credential value from env or keychain
- **File:** crates/hkask-mcp/src/server.rs:596


### hkask-memory (52 contracts)

#### P3-mem-consolidation-bridge-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  episodic and semantic are initialized memory stores
- **Post:** returns ConsolidationBridge linking the two stores
- **File:** crates/hkask-memory/src/consolidation.rs:49

#### P3-mem-consolidation-bridge-consolidate (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  token.issuer() == expected curator WebID; perspective is a valid WebID
- **Post:** episodic triples stripped of perspective, stored in semantic memory
- **File:** crates/hkask-memory/src/consolidation.rs:163

#### P3-mem-consolidation-candidate-count (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  perspective is a valid WebID
- **Post:** returns count of triples in episodic storage for this perspective;returns 0 on error (graceful degradation)
- **File:** crates/hkask-memory/src/consolidation.rs:205

#### P3-mem-consolidation-service-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  bridge and semantic are initialized; token.issuer() == expected curator
- **Post:** returns ConsolidationService ready for consolidation operations
- **File:** crates/hkask-memory/src/consolidation_service.rs:35

#### P3-mem-consolidation-service-consolidate (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  perspective is a valid WebID; request.limit > 0
- **Post:** episodic triples consolidated into semantic memory
- **File:** crates/hkask-memory/src/consolidation_service.rs:62

#### P3-mem-consolidation-service-candidate-count (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  perspective is a valid WebID
- **Post:** returns count of episodic triples available for consolidation
- **File:** crates/hkask-memory/src/consolidation_service.rs:214

#### P3-mem-consolidation-service-low-confidence-count (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  threshold in [0.0, 1.0]
- **Post:** returns count of semantic triples with confidence ≤ threshold;returns 0 on error (graceful degradation)
- **File:** crates/hkask-memory/src/consolidation_service.rs:225

#### P3-mem-consolidation-service-triple-count (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns total count of triples in semantic memory;returns 0 on error (graceful degradation)
- **File:** crates/hkask-memory/src/consolidation_service.rs:237

#### P3-mem-episodic-loop-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  memory is initialized, perspective is valid, storage_budget > 0
- **Post:** returns EpisodicLoop without consolidation bridge
- **File:** crates/hkask-memory/src/episodic_loop.rs:42

#### P3-mem-episodic-loop-with-consolidation (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  memory is initialized, perspective is valid, storage_budget > 0; consolidation_token.issuer() == expected curator
- **Post:** returns EpisodicLoop with consolidation bridge and token
- **File:** crates/hkask-memory/src/episodic_loop.rs:63

#### P3-mem-episodic-loop-storage-budget (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns the storage_budget value set at construction
- **File:** crates/hkask-memory/src/episodic_loop.rs:87

#### P3-mem-episodic-memory-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  triple_store is initialized
- **Post:** returns EpisodicMemory with DEFAULT_DECAY_RATE and DEFAULT_EPISODIC_BUDGET
- **File:** crates/hkask-memory/src/episodic.rs:57

#### P3-mem-episodic-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  triple.access.visibility != Public (episodic is sovereign); triple.access.perspective is Some (must have owner)
- **Post:** triple inserted into triple_store
- **File:** crates/hkask-memory/src/episodic.rs:74

#### P3-mem-episodic-query-deduped (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity is non-empty, perspective is valid
- **Post:** returns Vec<Triple> filtered by perspective, decayed, deduped, sorted by recency;confidence decayed via e^(-λt) for each triple
- **File:** crates/hkask-memory/src/episodic.rs:107

#### P3-mem-episodic-storage-usage (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  perspective is a valid WebID
- **Post:** returns count of triples for this perspective
- **File:** crates/hkask-memory/src/episodic.rs:156

#### P3-mem-episodic-storage-budget (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns the storage_budget value set at construction
- **File:** crates/hkask-memory/src/episodic.rs:230

#### P3-mem-episodic-candidate-count (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  perspective is a valid WebID
- **Post:** returns count of triples eligible for consolidation;returns 0 on error (graceful degradation)
- **File:** crates/hkask-memory/src/episodic.rs:245

#### P3-mem-ranking-rrf-score (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  k > 0, ranks contains valid 0-based positions
- **Post:** returns sum of 1/(k + rank + 1) for each rank;result is always ≥ 0.0
- **File:** crates/hkask-memory/src/ranking.rs:14

#### P3-mem-ranking-parse-age (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  age is a valid &str
- **Post:** returns days as f64 (≥ 0.0 for valid dates);returns -1.0 for unparseable or empty input
- **File:** crates/hkask-memory/src/ranking.rs:33

#### P3-mem-ranking-normalize-date-bucket (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  published is a valid &str
- **Post:** returns one of five bucket labels based on age in days;returns "unknown" for unparseable input
- **File:** crates/hkask-memory/src/ranking.rs:172

#### P3-mem-recall-eav-hash (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  triple is a valid Triple with entity, attribute, value
- **Post:** returns deterministic 32-byte BLAKE3 hash of canonical EAV content;same EAV content → same hash (metadata-independent)
- **File:** crates/hkask-memory/src/recall_dedup.rs:20

#### P3-mem-recall-dedup-triples (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  triples is a Vec of valid Triples
- **Post:** returns Vec with duplicates removed (by EAV hash);preserves original ordering (first occurrence kept);result.len() ≤ triples.len()
- **File:** crates/hkask-memory/src/recall_dedup.rs:64

#### P3-mem-salience-method-signals (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  text is a valid &str
- **Post:** returns MethodSignals with computed linguistic features;returns MethodSignals::default() for empty text
- **File:** crates/hkask-memory/src/salience.rs:85

#### P3-mem-salience-declared-method-matches (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  signals is a valid MethodSignals
- **Post:** returns true iff all configured min/max thresholds are satisfied;unconfigured thresholds (None) are always satisfied
- **File:** crates/hkask-memory/src/salience.rs:560

#### P3-mem-salience-tag-entities (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  text is non-empty, entity lists are valid
- **Post:** returns EntityTags with matched entities per category;methods field is empty (filled separately)
- **File:** crates/hkask-memory/src/salience.rs:618

#### P3-mem-salience-all-tags (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns iterator over all tag strings across all categories
- **File:** crates/hkask-memory/src/salience.rs:652

#### P3-mem-salience-tag-count (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns sum of lengths of all tag category vectors
- **File:** crates/hkask-memory/src/salience.rs:668

#### P3-mem-salience-compute-batch (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  all_tags is a slice of EntityTags
- **Post:** returns Vec<f32> with one salience score per passage;passages with zero tags get salience 0.0;returns empty Vec for empty input
- **File:** crates/hkask-memory/src/salience.rs:712

#### P3-mem-salience-budget-resolve (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  passage_count ≥ 0
- **Post:** returns computed absolute triple budget;Flat variant caps at total_passages if set and smaller
- **File:** crates/hkask-memory/src/salience.rs:860

#### P3-mem-semantic-loop-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  memory is initialized
- **Post:** returns SemanticLoop with DEFAULT_SEMANTIC_STORAGE_BUDGET and DEFAULT_LOW_CONFIDENCE_THRESHOLD
- **File:** crates/hkask-memory/src/semantic_loop.rs:49

#### P3-mem-semantic-loop-with-budget (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  memory is initialized, storage_budget > 0
- **Post:** returns SemanticLoop with custom budget, default threshold
- **File:** crates/hkask-memory/src/semantic_loop.rs:66

#### P3-mem-semantic-loop-with-budget-threshold (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  memory is initialized, storage_budget > 0; low_confidence_threshold in [0.0, 1.0]
- **Post:** returns SemanticLoop with custom budget and threshold
- **File:** crates/hkask-memory/src/semantic_loop.rs:84

#### P3-mem-semantic-loop-storage-budget (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns the storage_budget value set at construction
- **File:** crates/hkask-memory/src/semantic_loop.rs:104

#### P3-mem-semantic-loop-low-confidence-threshold (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns the low_confidence_threshold value set at construction
- **File:** crates/hkask-memory/src/semantic_loop.rs:114

#### P3-mem-semantic-memory-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  triple_store and embedding_store are initialized
- **Post:** returns SemanticMemory wrapping both stores
- **File:** crates/hkask-memory/src/semantic.rs:65

#### P3-mem-semantic-query-deduped (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity is non-empty
- **Post:** returns Vec<Triple> filtered to Public visibility, deduplicated by EAV hash
- **File:** crates/hkask-memory/src/semantic.rs:83

#### P3-mem-semantic-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  triple.access.visibility == Public; triple.access.perspective is None
- **Post:** triple inserted into triple_store;returns Err(InvalidVisibility) if not Public
- **File:** crates/hkask-memory/src/semantic.rs:99

#### P3-mem-semantic-triple-count (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns total count of semantic triples in store
- **File:** crates/hkask-memory/src/semantic.rs:128

#### P3-mem-semantic-triple-count-entity (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity is non-empty
- **Post:** returns count of semantic triples for this entity
- **File:** crates/hkask-memory/src/semantic.rs:138

#### P3-mem-semantic-query-attribute (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  attribute is non-empty
- **Post:** returns Vec<Triple> with matching attribute
- **File:** crates/hkask-memory/src/semantic.rs:149

#### P3-mem-semantic-store-embedding (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity_ref is non-empty, vector is non-empty, model is valid
- **Post:** embedding stored and indexed by entity_ref;returns embedding ID
- **File:** crates/hkask-memory/src/semantic.rs:165

#### P3-mem-semantic-search-similar (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  query_vector is non-empty, limit > 0
- **Post:** returns Vec<SimilarityResult> ordered by ascending distance
- **File:** crates/hkask-memory/src/semantic.rs:187

#### P3-mem-semantic-embedding-count (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns total count of embeddings in store
- **File:** crates/hkask-memory/src/semantic.rs:202

#### P3-mem-semantic-embedding-store (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns reference to the EmbeddingStore
- **File:** crates/hkask-memory/src/semantic.rs:213

#### P3-mem-semantic-compute-centroid (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  prefix is non-empty, dim > 0
- **Post:** returns CentroidResult with mean vector and passage count;returns Err(NoEmbeddingsForCentroid) if no matching embeddings;centroid stored if store_as and model are provided
- **File:** crates/hkask-memory/src/semantic.rs:244

#### P3-mem-semantic-purge-prefix (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  prefix is non-empty
- **Post:** all embeddings with matching prefix deleted;returns count of deleted embeddings
- **File:** crates/hkask-memory/src/semantic.rs:331

#### P3-mem-semantic-chunk-text (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  text is non-empty, entity_ref_prefix is non-empty; min_words > 0, max_words >= min_words
- **Post:** returns Vec of (entity_ref, text) chunks;each chunk has word count between min_words and max_words (best-effort)
- **File:** crates/hkask-memory/src/semantic.rs:367

#### P3-mem-semantic-strip-gutenberg (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  text is a valid &str
- **Post:** returns text between START OF and END OF markers;returns full text if markers not found
- **File:** crates/hkask-memory/src/semantic.rs:480

#### P3-mem-semantic-delete-triple (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  id is a valid TripleID
- **Post:** triple deleted from store;returns Err if triple not found
- **File:** crates/hkask-memory/src/semantic.rs:520

#### P3-mem-semantic-lowest-confidence (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  limit > 0
- **Post:** returns up to `limit` triples ordered by confidence ascending
- **File:** crates/hkask-memory/src/semantic.rs:543

#### P3-mem-semantic-low-confidence-count (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  threshold in [0.0, 1.0]
- **Post:** returns count of triples with confidence ≤ threshold
- **File:** crates/hkask-memory/src/semantic.rs:560

#### P3-mem-semantic-low-confidence-triples (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  threshold in [0.0, 1.0], limit > 0
- **Post:** returns up to `limit` triples with confidence ≤ threshold
- **File:** crates/hkask-memory/src/semantic.rs:579


### hkask-services (208 contracts)

#### SVC-217 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  repo_owner, repo_name, branch, path, content must be non-empty; GitHub credentials must be in keychain
- **Post:** returns ArchiveResult with path and commit_sha; file created or updated on GitHub; Err(Archival) on API failure
- **File:** crates/hkask-services/src/archival.rs:43

#### SVC-218 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  repo_owner, repo_name, git_ref must be non-empty; target_path defaults to "registry" if "."
- **Post:** returns decoded file content as String; Err(Archival) on API failure, missing content, or decode error
- **File:** crates/hkask-services/src/archival.rs:107

#### SVC-219 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  repo_owner, repo_name must be non-empty; GitHub credentials must be in keychain
- **Post:** returns Vec<String> of commit SHAs; empty Vec if no commits; Err(Archival) on API failure
- **File:** crates/hkask-services/src/archival.rs:166

#### SVC-220 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  repo_owner, repo_name, message must be non-empty; agent_registry_store must be initialized
- **Post:** returns SnapshotResult with commit_sha; registry content pushed to GitHub; Err(Archival) on API or serialization failure
- **File:** crates/hkask-services/src/archival.rs:205

#### SVC-200 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  skill_ids must have at least 2 entries; ctx.registry() must be initialized; inference_port must be valid
- **Post:** returns BundleComposeResult with validated manifest and warnings; Err(Compose) if <2 skills, skills not found, or validation fails
- **File:** crates/hkask-services/src/bundle.rs:51

#### SVC-201 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.registry() must be initialized
- **Post:** returns Vec<BundleManifest> of all registered bundles; empty Vec if none
- **File:** crates/hkask-services/src/bundle.rs:228

#### SVC-202 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.registry() must be initialized; id must be non-empty
- **Post:** returns Some(BundleManifest) if found; None if not found
- **File:** crates/hkask-services/src/bundle.rs:239

#### SVC-203 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.registry() must be initialized; id must be non-empty
- **Post:** returns BundleManifest if found; Err(Compose) if bundle not found
- **File:** crates/hkask-services/src/bundle.rs:252

#### SVC-204 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.registry() must be initialized; id must reference an existing bundle; inference_port must be valid
- **Post:** returns BundleComposeResult with evolved manifest; old bundle removed, new one registered; Err(Compose) if bundle not found
- **File:** crates/hkask-services/src/bundle.rs:269

#### SVC-205 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** always returns Ok(())
- **File:** crates/hkask-services/src/bundle.rs:309

#### SVC-206 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.registry() must be initialized
- **Post:** returns Vec<Skill> of all registered skills; empty Vec if none
- **File:** crates/hkask-services/src/bundle.rs:318

#### SVC-234 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self.total_tokens must be set
- **Post:** returns total_tokens as u64 gas cost
- **File:** crates/hkask-services/src/chat.rs:45

#### SVC-235 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx must be fully built; req.input must be non-empty; agent must be registered
- **Post:** returns PreparedChat with prompt, model, agent_webid, capability_token, inference_port, episodic_port, and agent_name; Err(AgentNotFound) if agent not registered
- **File:** crates/hkask-services/src/chat.rs:352

#### SVC-236 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx must be fully built; req.input must be non-empty
- **Post:** returns ChatResponse with text, usage, finish_reason, and tool_calls; CNS spans emitted; episodic trace stored; Err on agent lookup or inference failure
- **File:** crates/hkask-services/src/chat.rs:474

#### SVC-237 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  semantic_port must be initialized; input must be non-empty; token must be valid
- **Post:** returns Some(String) of concatenated triple values if matches found; None if no matches or recall fails
- **File:** crates/hkask-services/src/chat.rs:578

#### SVC-238 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  episodic_port must be initialized; input and response must be non-empty; agent_webid must be valid; token must be valid
- **Post:** chat exchange is stored as episodic triple with confidence 0.7; failures are logged but not returned (best-effort)
- **File:** crates/hkask-services/src/chat.rs:606

#### SVC-239 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  episodic_port must be initialized; agent_webid must be valid; token must be valid; limit must be > 0
- **Post:** returns Some(String) of formatted recent turns; None if no episodes or recall fails
- **File:** crates/hkask-services/src/chat.rs:652

#### SVC-240 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  episodic_port must be initialized; agent_webid must be valid; token must be valid; limit must be > 0
- **Post:** returns Vec<Value> of {role, content} messages; empty Vec if no episodes or recall fails
- **File:** crates/hkask-services/src/chat.rs:696

#### SVC-241 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  executor must be initialized; manifest must be valid; input and agent_name must be non-empty
- **Post:** returns Some(String) of concatenated step outputs if cascade completes; None if no manifest or execution fails
- **File:** crates/hkask-services/src/chat.rs:731

#### SVC-242 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  input and manifest_context must be non-empty
- **Post:** returns formatted string with [Manifest Context] block prepended to input
- **File:** crates/hkask-services/src/chat.rs:787

#### SVC-243 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  response must be non-empty; constraints if Some must be valid PersonaConstraints
- **Post:** returns cleaned response with forbidden patterns stripped; violations logged; returns original if constraints is None
- **File:** crates/hkask-services/src/chat.rs:799

#### SVC-244 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx must be fully built; req.input must be non-empty; req.agent_name must be registered
- **Post:** returns TurnResult with response text, token usage, tool calls, and iteration count; manifest cascade and history suffix applied; persona filter applied; Err on inference failure
- **File:** crates/hkask-services/src/chat.rs:907

#### SVC-277 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  texts must be non-empty; config must have valid timeout and concurrency
- **Post:** returns Vec<ClassifyResult> in input order; failed classifications fall back to config.fallback_category; all fallback if no API key
- **File:** crates/hkask-services/src/classify.rs:269

#### SVC-278 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  texts must be non-empty; config must have valid timeout and concurrency
- **Post:** returns Vec<TripleExtraction> in input order; failed extractions fall back to empty; all empty if no API key
- **File:** crates/hkask-services/src/classify.rs:351

#### SVC-136 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  runtime must be a valid Arc<RwLock<CnsRuntime>>
- **Post:** returns CnsService wrapping the runtime
- **File:** crates/hkask-services/src/cns.rs:27

#### SVC-137 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  runtime must be initialized
- **Post:** returns CnsHealth with healthy flag, alert count, and deficit summary
- **File:** crates/hkask-services/src/cns.rs:36

#### SVC-138 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  runtime must be initialized
- **Post:** returns Vec<RuntimeAlert> of currently active alerts; empty Vec if none
- **File:** crates/hkask-services/src/cns.rs:45

#### SVC-139 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  runtime must be initialized
- **Post:** returns HashMap<SpanNamespace, u64> of variety counters; empty map if no counters
- **File:** crates/hkask-services/src/cns.rs:54

#### SVC-140 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns SetPoints from env config or hard-coded defaults
- **File:** crates/hkask-services/src/cns.rs:66

#### SVC-141 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must be a valid SetPointsConfig; missing fields use defaults
- **Post:** returns SetPoints computed from config merged with defaults
- **File:** crates/hkask-services/src/cns.rs:78

#### SVC-095 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  request.db_path must point to a valid database; request.prompt must be non-empty; request.cognition must have valid embedding config
- **Post:** returns ComposeResult with generated_prose, exemplar_count, and optional CentroidValidation; Err on DB open failure, embedding failure, or inference failure
- **File:** crates/hkask-services/src/compose.rs:157

#### SVC-096 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  a and b must be non-empty f32 slices of equal length; mismatched or empty returns 2.0
- **Post:** returns f64 in range [0.0, 2.0]; 0.0 = identical, 1.0 = orthogonal, 2.0 = opposite or degenerate
- **File:** crates/hkask-services/src/compose.rs:450

#### SVC-221 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  keystore must have acp_secret, db_passphrase, and mcp_secret configured
- **Post:** returns ServiceConfig with env-derived values and keystore secrets; Err(Keystore) on secret resolution failure
- **File:** crates/hkask-services/src/config.rs:119

#### SVC-222 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  acp_secret, db_passphrase, mcp_secret, agent_name must be non-empty
- **Post:** returns ServiceConfig with provided secrets and env-derived or default values
- **File:** crates/hkask-services/src/config.rs:182

#### SVC-223 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns ServiceConfig with :memory: DB, zeroed secrets, and test agent name
- **File:** crates/hkask-services/src/config.rs:225

#### SVC-224 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns Some(path) if not in_memory; None if in_memory; derives from db_path if memory_db_path not set
- **File:** crates/hkask-services/src/config.rs:258

#### SVC-174 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds or returns rate-limit error)
- **Post:** Ok(()) if rate limit not exceeded; Err(RateLimited) with remaining seconds if within 30s window
- **File:** crates/hkask-services/src/consolidation.rs:29

#### SVC-175 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid must be a valid WebID
- **Post:** returns "hkask-memory-agent-{webid}.db" path string
- **File:** crates/hkask-services/src/consolidation.rs:49

#### SVC-176 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  passphrase must be non-empty; server passphrase must be configured in keystore
- **Post:** returns the expected passphrase string on match; Err(Keystore) if not configured; Err(InvalidPassphrase) if mismatch
- **File:** crates/hkask-services/src/consolidation.rs:55

#### SVC-177 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid must be a valid WebID; db_passphrase must be correct; db_path must point to a valid database; request must be a valid ConsolidationRequest
- **Post:** returns ConsolidationOutcome with consolidated_count, deleted_count, failed_count; Err on DB open failure or consolidation failure
- **File:** crates/hkask-services/src/consolidation.rs:74

#### SVC-122 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be initialized; agent_name and contact_name must be non-empty
- **Post:** contact is persisted to the registry store; Err(AgentRegistryStore) on store failure
- **File:** crates/hkask-services/src/contacts.rs:15

#### SVC-123 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be initialized; agent_name and query must be non-empty
- **Post:** returns Vec<Contact> matching the query; empty Vec if no matches; Err(AgentRegistryStore) on store failure
- **File:** crates/hkask-services/src/contacts.rs:38

#### SVC-124 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be initialized; agent_name must be non-empty
- **Post:** returns Vec<Contact> for the agent; empty Vec if no contacts; Err(AgentRegistryStore) on store failure
- **File:** crates/hkask-services/src/contacts.rs:53

#### SVC-245 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns reference to ServiceConfig
- **File:** crates/hkask-services/src/context.rs:212

#### SVC-246 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns Some(&Arc<WalletService>) if wallet configured; None otherwise
- **File:** crates/hkask-services/src/context.rs:221

#### SVC-247 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns Some(&Arc<WalletStore>) if wallet store configured; None otherwise
- **File:** crates/hkask-services/src/context.rs:230

#### GAS-CALIB-005—runtimecalibrationofwalletgasconversionrate (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns Some(&Arc<WalletGasCalibrator>) if wallet is configured; None otherwise
- **File:** crates/hkask-services/src/context.rs:239

#### SVC-248 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns (&episodic_storage, &semantic_storage) tuple
- **File:** crates/hkask-services/src/context.rs:250

#### SVC-249 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<Mutex<SqliteRegistry>>
- **File:** crates/hkask-services/src/context.rs:260

#### SVC-250 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<SqliteGoalRepository>
- **File:** crates/hkask-services/src/context.rs:268

#### SVC-251 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<RwLock<CnsRuntime>>
- **File:** crates/hkask-services/src/context.rs:278

#### SVC-252 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<RwLock<CyberneticsLoop>>
- **File:** crates/hkask-services/src/context.rs:286

#### SVC-253 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<LoopSystem>
- **File:** crates/hkask-services/src/context.rs:294

#### SVC-254 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<dyn NuEventSink>
- **File:** crates/hkask-services/src/context.rs:302

#### GAS-CALIB-004—runtimecalibrationloopwiredtoproductionestimator (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<CalibratedEnergyEstimator> sharing the same background
- **File:** crates/hkask-services/src/context.rs:311

#### SVC-255 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<RwLock<Option<SeamWatcher>>>
- **File:** crates/hkask-services/src/context.rs:323

#### SVC-256 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<CapabilityChecker>
- **File:** crates/hkask-services/src/context.rs:333

#### SVC-257 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<McpDispatcher>
- **File:** crates/hkask-services/src/context.rs:342

#### SVC-258 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<EscalationQueue>
- **File:** crates/hkask-services/src/context.rs:350

#### SVC-259 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns Some(Arc<dyn InferencePort>) if configured; None otherwise
- **File:** crates/hkask-services/src/context.rs:360

#### SVC-260 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<McpRuntime>
- **File:** crates/hkask-services/src/context.rs:368

#### SVC-261 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<PodManager>
- **File:** crates/hkask-services/src/context.rs:376

#### SVC-262 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns (&WebID, &Arc<AcpRuntime>) tuple
- **File:** crates/hkask-services/src/context.rs:386

#### SVC-263 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns SovereigntyService wrapping the consent manager
- **File:** crates/hkask-services/src/context.rs:396

#### SVC-264 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Option<UnboundedSender<CurationInput>>
- **File:** crates/hkask-services/src/context.rs:413

#### SVC-265 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &SovereigntyBoundaryStore
- **File:** crates/hkask-services/src/context.rs:423

#### SVC-266 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &SqliteSpecStore
- **File:** crates/hkask-services/src/context.rs:435

#### SVC-267 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &AgentRegistryStore
- **File:** crates/hkask-services/src/context.rs:445

#### SVC-268 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<Mutex<UserStore>>
- **File:** crates/hkask-services/src/context.rs:455

#### SVC-269 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns &Arc<ServiceDaemonHandler>
- **File:** crates/hkask-services/src/context.rs:464

#### SVC-270 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be fully built
- **Post:** returns Some(&Arc<Mutex<MatrixTransport>>) if connected; None otherwise
- **File:** crates/hkask-services/src/context.rs:476

#### SVC-271 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  db must be a valid opened Database
- **Post:** returns PerAgentMemory with episodic_storage, semantic_storage, and consolidation_service all sharing the same DB
- **File:** crates/hkask-services/src/context.rs:494

#### SVC-272 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must have valid db_path and db_passphrase
- **Post:** returns Arc<EscalationQueue> initialized from DB; Err on DB open or schema init failure
- **File:** crates/hkask-services/src/context.rs:536

#### SVC-273 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must have valid db_path and db_passphrase
- **Post:** returns SqliteSpecStore with schema initialized; Err on DB open or schema init failure
- **File:** crates/hkask-services/src/context.rs:546

#### SVC-274 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must have valid db_path and db_passphrase
- **Post:** returns (Arc<ConsentManager>, SovereigntyBoundaryStore) with schemas initialized; Err on DB open or schema init failure
- **File:** crates/hkask-services/src/context.rs:558

#### SVC-275 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must have valid db_path, db_passphrase, and acp_secret
- **Post:** returns (Arc<AcpRuntime>, AgentRegistryStore) with schema initialized; Err on DB open or schema init failure
- **File:** crates/hkask-services/src/context.rs:580

#### SVC-276 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must be a valid ServiceConfig with resolved secrets
- **Post:** returns fully assembled AgentService with all infrastructure wired; Err on any construction step failure
- **File:** crates/hkask-services/src/context.rs:610

#### SVC-213 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.escalation_queue() must be initialized
- **Post:** returns Vec<EscalationResponse> of pending escalations; empty Vec if none; Err(Escalation) on queue error
- **File:** crates/hkask-services/src/curator.rs:60

#### SVC-214 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.escalation_queue() must be initialized; id must be a valid escalation ID; resolved_by must be non-empty
- **Post:** escalation is resolved; CNS event emitted; Ok(()) on success; Err(EscalationNotFound) if ID not found; Err(Escalation) on queue error
- **File:** crates/hkask-services/src/curator.rs:73

#### SVC-215 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.escalation_queue() must be initialized; id must be a valid escalation ID; dismissed_by must be non-empty
- **Post:** escalation is dismissed; CNS event emitted; Ok(()) on success; Err(EscalationNotFound) if ID not found; Err(Escalation) on queue error
- **File:** crates/hkask-services/src/curator.rs:110

#### SVC-216 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.escalation_queue() and ctx.cns_runtime() must be initialized
- **Post:** returns human-readable summary string from metacognition cycle; Err(Metacognition) on cycle failure; Err(Cns) if CNS runtime unavailable
- **File:** crates/hkask-services/src/curator.rs:150

#### SVC-135 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  pod_manager must be a valid Arc<PodManager>; user_store must be a valid Arc<Mutex<UserStore>>
- **Post:** returns ServiceDaemonHandler with all fields initialized; inference_port may be None
- **File:** crates/hkask-services/src/daemon_handler.rs:56

#### SVC-166 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  req.author_name must be non-empty; mcp must be connected; token must be valid
- **Post:** returns DiscoverResult with discovered works, sources, and academic works; output and cache directories created; Err on MCP or I/O failure
- **File:** crates/hkask-services/src/discover.rs:142

#### SVC-167 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  author_slug must be non-empty; works must be non-empty; output_dir must exist
- **Post:** corpus.yaml is written to output_dir; returns PathBuf to the written file; Err on serialization or I/O failure
- **File:** crates/hkask-services/src/discover.rs:454

#### SVC-168 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  author_slug must be non-empty
- **Post:** returns CorpusConfig with default embedding, chunking, validation, and budget settings
- **File:** crates/hkask-services/src/discover.rs:524

#### SVC-169 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  url must be a valid HTTP/HTTPS URL; cache_path's parent directory must exist
- **Post:** content is downloaded, PDFs are text-extracted (with OCR fallback), HTML is stripped, and result is written to cache_path; Err on HTTP failure, empty content, or I/O error
- **File:** crates/hkask-services/src/discover.rs:1293

#### SVC-170 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  s may be any string (including empty)
- **Post:** returns lowercase, alphanumeric-only slug with hyphens; empty string becomes empty slug
- **File:** crates/hkask-services/src/discover.rs:1426

#### SVC-225 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be a valid ServiceError variant
- **Post:** returns true for retryable errors (network, rate-limit, keystore); false for non-retryable (not-found, validation, permission)
- **File:** crates/hkask-services/src/error.rs:440

#### SVC-226 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be a valid ServiceError variant
- **Post:** returns &'static str i18n key (e.g., "error.curator.escalation_not_found")
- **File:** crates/hkask-services/src/error.rs:538

#### SVC-227 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be a valid ServiceError variant
- **Post:** returns Some(NuEvent) for system-level errors (inference, CNS, storage, infra); None for user-input errors (not-found, validation)
- **File:** crates/hkask-services/src/error.rs:637

#### SVC-211 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns CliExperienceRecorder; daemon is Some if socket exists, None otherwise
- **File:** crates/hkask-services/src/experience.rs:38

#### SVC-212 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  replicant, tool, input_summary, outcome must be non-empty; detail must be valid JSON
- **Post:** experience is sent to daemon for dual encoding; silently skipped if daemon unavailable
- **File:** crates/hkask-services/src/experience.rs:67

#### SVC-125 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.goal_repo() must be initialized; req.text must be non-empty; req.visibility must be "private" or "public"
- **Post:** goal is persisted and returned as GoalResponse; Err(ValidationError) on invalid visibility; Err(GoalRepo) on store failure
- **File:** crates/hkask-services/src/goal.rs:48

#### SVC-126 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.goal_repo() must be initialized; owner must be a valid WebID; state_filter if Some must be a valid GoalState string
- **Post:** returns Vec<GoalResponse> for matching goals; empty Vec if none; Err(ValidationError) on invalid state filter; Err(GoalRepo) on store failure
- **File:** crates/hkask-services/src/goal.rs:73

#### SVC-127 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.goal_repo() must be initialized; goal_id_str must be a valid GoalID; new_state_str must be a valid GoalState
- **Post:** goal state is updated and returned as GoalResponse; Err(ValidationError) on invalid ID or state; Err(GoalRepo) on store failure
- **File:** crates/hkask-services/src/goal.rs:101

#### SVC-228 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  default_model must be non-empty; inference_config must be valid
- **Post:** returns InferenceContext with provided parts; shared_port may be None
- **File:** crates/hkask-services/src/inference.rs:44

#### SVC-229 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx must have valid inference_config; model must be non-empty
- **Post:** returns Arc<dyn InferencePort> — shared port if model matches default, else fresh InferenceRouter; Err on connection failure
- **File:** crates/hkask-services/src/inference.rs:110

#### SVC-230 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx must have valid inference_config
- **Post:** returns Vec<ModelInfo> from all configured providers; empty Vec if none
- **File:** crates/hkask-services/src/inference.rs:134

#### SVC-231 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx must have valid inference_config; query must be non-empty
- **Post:** returns Vec<ModelInfo> matching query; empty Vec if no matches
- **File:** crates/hkask-services/src/inference.rs:146

#### SVC-097 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  path may or may not exist; if missing, returns default empty history
- **Post:** returns KataHistory from JSON file; Err(LoadFailed) on I/O error; Err(ParseFailed) on invalid JSON
- **File:** crates/hkask-services/src/kata.rs:267

#### SVC-098 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be a valid KataHistory; path's parent directory must exist
- **Post:** history is serialized as pretty JSON and written to path; Err(LoadFailed) on serialization or I/O error
- **File:** crates/hkask-services/src/kata.rs:287

#### SVC-099 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  agent must be non-empty; entry must have valid date and kata_type
- **Post:** entry is appended to the agent's practice history; creates agent entry if not present
- **File:** crates/hkask-services/src/kata.rs:305

#### SVC-100 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  agent may or may not have entries; today must be ISO 8601 date (YYYY-MM-DD)
- **Post:** returns u32 streak count; 0 if no entries or today not practiced; counts consecutive days backward from today
- **File:** crates/hkask-services/src/kata.rs:317

#### SVC-101 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  agent may or may not have entries; today must be ISO 8601 date
- **Post:** returns f64 in [0.0, 1.0]; 0.0 = no practice; 1.0 = 21+ day streak; decay applied after 3+ days gap
- **File:** crates/hkask-services/src/kata.rs:354

#### SVC-102 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  agent may or may not have entries; today must be ISO 8601 date
- **Post:** returns u32 days since last practice; u32::MAX if no entries or parse failure
- **File:** crates/hkask-services/src/kata.rs:373

#### SVC-103 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  agent may or may not have entries; today must be ISO 8601 date
- **Post:** returns true if compute_automaticity > 0.5; false otherwise
- **File:** crates/hkask-services/src/kata.rs:390

#### SVC-104 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  agent may or may not have entries; today must be ISO 8601 date
- **Post:** returns true if days_since_last is in range [3, u32::MAX); false otherwise
- **File:** crates/hkask-services/src/kata.rs:399

#### SVC-105 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be a valid KataState; path's parent directory must exist
- **Post:** state is serialized as pretty JSON and written to path; Err(LoadFailed) on serialization or I/O error
- **File:** crates/hkask-services/src/kata.rs:532

#### SVC-106 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  path must exist and contain valid JSON
- **Post:** returns KataState deserialized from file; Err(LoadFailed) on I/O error; Err(ParseFailed) on invalid JSON
- **File:** crates/hkask-services/src/kata.rs:550

#### SVC-107 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  inference must be a valid InferencePort; registry must be initialized
- **Post:** returns KataEngine with inference and registry wired; all optional components (consent, CNS, history, metrics) default to None
- **File:** crates/hkask-services/src/kata.rs:621

#### SVC-108 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  registry must be initialized; inference env vars must be set or defaults used
- **Post:** returns KataEngine with InferenceRouter built from env config
- **File:** crates/hkask-services/src/kata.rs:643

#### SVC-109 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  check must be a valid Fn(&str, &str) -> Result<(), KataError>
- **Post:** returns self with consent_check set; kata execution will call check before running
- **File:** crates/hkask-services/src/kata.rs:654

#### SVC-110 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  observer must be a valid Fn(&str, u32, &str)
- **Post:** returns self with cns_observer set; observer is called after each kata step
- **File:** crates/hkask-services/src/kata.rs:667

#### SVC-111 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  history must be a valid KataHistory
- **Post:** returns self with history set; starter kata uses it for automaticity computation
- **File:** crates/hkask-services/src/kata.rs:680

#### SVC-112 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be a valid Arc<KataHistoryStore>
- **Post:** returns self with history_store set; record_history_entry will persist to SQLite
- **File:** crates/hkask-services/src/kata.rs:694

#### SVC-113 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  collector must be a valid Fn(&str, &str) -> Result<Value, KataError>
- **Post:** returns self with metric_collector set; improvement kata captures before/after metrics
- **File:** crates/hkask-services/src/kata.rs:704

#### SVC-114 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  cns must be a valid Arc<RwLock<CnsRuntime>>
- **Post:** returns self with cns_runtime set; kata cycles will increment variety and check alerts
- **File:** crates/hkask-services/src/kata.rs:720

#### SVC-115 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  agent_name, date, kata_type, practice_name must be non-empty
- **Post:** returns Some(row_id) if history_store is set and record succeeds; None if store not configured; Err on store failure
- **File:** crates/hkask-services/src/kata.rs:734

#### SVC-116 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  path must exist and contain valid YAML
- **Post:** returns KataManifest deserialized from file; Err(LoadFailed) on I/O error; Err(ParseFailed) on invalid YAML
- **File:** crates/hkask-services/src/kata.rs:765

#### SVC-117 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  manifest must have at least one step for selector; learner_bot must be non-empty
- **Post:** returns KataResult from the selected kata execution; Err on selector failure or kata execution error
- **File:** crates/hkask-services/src/kata.rs:783

#### SVC-118 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  manifest.manifest.kata_type must be "improvement", "coaching", or "starter"; learner_bot must be non-empty
- **Post:** returns KataResult with steps_completed, gas_consumed, and kata-type-specific outputs; Err(UnknownType) on invalid kata_type
- **File:** crates/hkask-services/src/kata.rs:852

#### SVC-119 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  manifest must have at least one step; state.learner_bot must be non-empty
- **Post:** returns KataResult with steps_completed, gas_consumed, and step_experiences; Err(NoSteps) if manifest has no steps; Err(GasExceeded) if gas cap exceeded
- **File:** crates/hkask-services/src/kata.rs:1140

#### SVC-120 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  manifest must have at least one question; state.learner_bot must be non-empty
- **Post:** returns KataResult with steps_completed (question count), gas_consumed, and step_experiences; Err(NoSteps) if no questions; Err(GasExceeded) if gas cap exceeded
- **File:** crates/hkask-services/src/kata.rs:1309

#### SVC-121 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  manifest must have at least one practice; state.learner_bot must be non-empty
- **Post:** returns KataResult with steps_completed (practice count), automaticity_delta, and step_experiences; Err(NoSteps) if no practices
- **File:** crates/hkask-services/src/kata.rs:1450

#### SVC-171 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be a valid ServerHealth variant
- **Post:** returns true for Healthy; false for Degraded or Stopped
- **File:** crates/hkask-services/src/lifecycle.rs:41

#### SVC-172 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  name and version must be non-empty; env vars HKASK_DB_PATH, HKASK_DB_PASSPHRASE, HKASK_MEMORY_DB_PATH, HKASK_MEMORY_DB_PASSPHRASE are read if set
- **Post:** returns ServerLifecycleConfig with env-derived or default values
- **File:** crates/hkask-services/src/lifecycle.rs:117

#### SVC-173 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must be a valid ServerLifecycleConfig; server must implement ServerLifecycle
- **Post:** server is initialized, started, and result returned; CNS spans emitted for start/stop/failure
- **File:** crates/hkask-services/src/lifecycle.rs:147

#### SVC-188 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  passphrase must be non-empty; store=true requires writable keychain
- **Post:** returns ResolvedSecrets with acp_secret and db_passphrase; if store=true, secrets are persisted to keychain; Err(Keystore) on keychain failure
- **File:** crates/hkask-services/src/onboarding.rs:61

#### SVC-189 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must have valid db_path, db_passphrase, and acp_secret
- **Post:** returns RegistryHandle with ACP runtime and initialized AgentRegistryStore; registered agents restored into ACP; Err on DB open or schema init failure
- **File:** crates/hkask-services/src/onboarding.rs:93

#### SVC-190 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  acp must be initialized; store must be initialized; name and description must be non-empty
- **Post:** replicant is registered in ACP with default capabilities and persisted to store; Err(Acp) on registration failure; Err(AgentRegistryStore) on persistence failure
- **File:** crates/hkask-services/src/onboarding.rs:139

#### SVC-191 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be initialized; profile must be a valid UserProfile
- **Post:** profile is persisted to the registry store; Err(AgentRegistryStore) on store failure
- **File:** crates/hkask-services/src/onboarding.rs:204

#### SVC-192 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be initialized
- **Post:** returns Some(UserProfile) if stored; None if no profile; Err(AgentRegistryStore) on store failure
- **File:** crates/hkask-services/src/onboarding.rs:218

#### SVC-193 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must be valid; agent_name must match a registered replicant; resolved_secrets must be valid
- **Post:** returns SignInOutcome on success; secrets stored in keychain; Err(AgentNotFound) if replicant missing; Err on registry init failure
- **File:** crates/hkask-services/src/onboarding.rs:235

#### SVC-194 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config.db_path must be set; returns empty Vec on any failure
- **Post:** returns Vec<RegisteredAgent> of replicants; empty Vec if DB inaccessible or no replicants
- **File:** crates/hkask-services/src/onboarding.rs:280

#### SVC-195 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config.db_path must be set; :memory: paths are never orphaned
- **Post:** returns true if orphaned DB was cleaned up; false if DB has replicants or doesn't exist
- **File:** crates/hkask-services/src/onboarding.rs:315

#### SVC-196 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must be valid; best-effort cleanup (errors are silently ignored)
- **Post:** keychain entries (acp-secret, hkask-db-passphrase) are removed; DB and salt files deleted if not :memory:
- **File:** crates/hkask-services/src/onboarding.rs:361

#### SVC-197 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  user_profile must have first_name and last_name; replicant_display_name must be non-empty; passphrase must be non-empty; homeserver_url must be valid
- **Post:** returns MatrixRegistrationResult with human and replicant user IDs; credentials stored in keychain; Err(Matrix) on registration failure
- **File:** crates/hkask-services/src/onboarding.rs:397

#### SVC-198 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  homeserver_url must be valid and reachable
- **Post:** returns HashMap<String, String> of bot_name → user_id for successfully registered bots; failed registrations are silently skipped
- **File:** crates/hkask-services/src/onboarding.rs:481

#### SVC-199 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  homeserver_url must be a valid HTTP URL
- **Post:** returns true if server responds with 2xx; false on connection error or non-2xx status
- **File:** crates/hkask-services/src/onboarding.rs:648

#### SVC-128 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.pod_manager() must be initialized; req.template must be non-empty; req.persona_yaml must be valid YAML
- **Post:** pod is created and returns PodResponse with pod_id; Err(ValidationError) on invalid persona YAML; Err(Pod) on upstream error
- **File:** crates/hkask-services/src/pods.rs:56

#### SVC-129 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.pod_manager() must be initialized
- **Post:** returns Vec<PodStatusResponse> for all pods; empty Vec if none; Err(Pod) on upstream error
- **File:** crates/hkask-services/src/pods.rs:85

#### SVC-130 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.pod_manager() must be initialized; pod_id must be a valid UUID
- **Post:** pod is activated; Ok(()) on success; Err(PodNotFound) on invalid UUID; Err(Pod) on upstream error
- **File:** crates/hkask-services/src/pods.rs:96

#### SVC-131 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.pod_manager() must be initialized; pod_id must be a valid UUID
- **Post:** pod is deactivated; Ok(()) on success; Err(PodNotFound) on invalid UUID; Err(Pod) on upstream error
- **File:** crates/hkask-services/src/pods.rs:110

#### SVC-132 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.pod_manager() must be initialized; pod_id must be a valid UUID
- **Post:** returns PodStatusResponse with pod state, webid, agent_type, template, etc.; Err(PodNotFound) on invalid UUID; Err(Pod) on upstream error
- **File:** crates/hkask-services/src/pods.rs:124

#### SVC-133 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.pod_manager() must be initialized; name and role must be non-empty
- **Post:** role is assigned to the replicant; Ok(()) on success; Err(Pod) on upstream error
- **File:** crates/hkask-services/src/pods.rs:153

#### SVC-134 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.pod_manager() must be initialized; name and mode must be non-empty; mode must be "server", "chat", or "exit"
- **Post:** agent mode is set; Ok(()) on success; Err(Pod) on upstream error
- **File:** crates/hkask-services/src/pods.rs:170

#### SVC-207 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be initialized; agent_name, trigger, action, next_run must be non-empty
- **Post:** task is persisted to the registry store; Err(AgentRegistryStore) on store failure
- **File:** crates/hkask-services/src/scheduler.rs:15

#### SVC-208 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be initialized; agent_name must be non-empty
- **Post:** returns Vec<ScheduledTask> for the agent; empty Vec if none; Err(AgentRegistryStore) on store failure
- **File:** crates/hkask-services/src/scheduler.rs:41

#### SVC-209 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be initialized; now must be a valid timestamp string
- **Post:** returns Vec<ScheduledTask> of all due tasks; empty Vec if none; Err(AgentRegistryStore) on store failure
- **File:** crates/hkask-services/src/scheduler.rs:55

#### SVC-210 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  store must be initialized; agent_name, trigger, new_next_run must be non-empty
- **Post:** task's next_run is updated in the store; Err(AgentRegistryStore) on store failure
- **File:** crates/hkask-services/src/scheduler.rs:69

#### SVC-178 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns PathBuf to ~/.config/hkask/settings.json; parent directory created if missing
- **File:** crates/hkask-services/src/settings.rs:12

#### SVC-179 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns HkaskSettings from disk; HkaskSettings::default() if file missing or unparseable
- **File:** crates/hkask-services/src/settings.rs:91

#### SVC-180 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  env_var name must be valid; settings_value and default must be non-empty strings
- **Post:** returns env var value if set and non-empty; else settings_value if non-empty; else default
- **File:** crates/hkask-services/src/settings.rs:111

#### SVC-181 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns effective generation model string (env > settings > default)
- **File:** crates/hkask-services/src/settings.rs:129

#### SVC-182 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns effective embedding model string (env > settings > default)
- **File:** crates/hkask-services/src/settings.rs:142

#### SVC-183 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns effective classifier model string (env > settings > default)
- **File:** crates/hkask-services/src/settings.rs:155

#### SVC-184 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns effective OCR model string (env > settings > default)
- **File:** crates/hkask-services/src/settings.rs:168

#### SVC-185 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be a valid HkaskSettings
- **Post:** settings are written as pretty JSON to settings_path(); Err on serialization or I/O failure
- **File:** crates/hkask-services/src/settings.rs:177

#### SVC-186 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  T must implement DeserializeOwned + Default
- **Post:** returns T from disk; T::default() if file missing or unparseable
- **File:** crates/hkask-services/src/settings.rs:193

#### SVC-187 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  settings must implement Serialize
- **Post:** settings are written as pretty JSON to settings_path(); Err(ServiceError::Infra) on serialization or I/O failure
- **File:** crates/hkask-services/src/settings.rs:215

#### SVC-088 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  zone_dir must be a readable directory; each subdirectory with SKILL.md is treated as a skill
- **Post:** returns Vec<SkillInfo> sorted by name, each with path, name, visibility, namespace, and content_hash; Err on I/O failure
- **File:** crates/hkask-services/src/skill.rs:48

#### SVC-089 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  skill_md_path may or may not exist; if unreadable, defaults to Private
- **Post:** returns Visibility parsed from front matter; defaults to Private on any parse failure
- **File:** crates/hkask-services/src/skill.rs:96

#### SVC-090 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  skill_md_path may or may not exist; returns None if unreadable or no namespace in front matter
- **Post:** returns Some(namespace) if front matter has a namespace field; None otherwise
- **File:** crates/hkask-services/src/skill.rs:125

#### SVC-091 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  path must be a readable file; returns None if unreadable
- **Post:** returns Some(hex-encoded BLAKE3 hash) on success; None on I/O failure
- **File:** crates/hkask-services/src/skill.rs:136

#### SVC-092 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  root must be a valid skill zone root; name must be non-empty
- **Post:** returns Some(PathBuf) to the matching skill directory if found; None if no match or public zone missing
- **File:** crates/hkask-services/src/skill.rs:149

#### SVC-093 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  root must be a valid skill zone root; name must exist in the private zone
- **Post:** skill directory is copied to public zone with namespaced name; visibility set to public; namespace set to replicant name; Err if private skill not found
- **File:** crates/hkask-services/src/skill.rs:180

#### SVC-094 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  none (always succeeds)
- **Post:** returns a non-empty String — env var, git user.name, or "local" fallback
- **File:** crates/hkask-services/src/skill.rs:257

#### SVC-096 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  registry and skill_index are valid; project_root points to hKask root
- **Post:** returns an auditor configured for both layers
- **File:** crates/hkask-services/src/skills.rs:22

#### SVC-097 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns a report with a health score and defects per skill
- **File:** crates/hkask-services/src/skills.rs:51

#### SVC-098 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  name is non-empty
- **Post:** returns the skill's health score or an error if audit fails
- **File:** crates/hkask-services/src/skills.rs:67

#### SVC-099 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns a JSON string representation of the report
- **File:** crates/hkask-services/src/skills.rs:85

#### SVC-100 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff health_score >= 0.8
- **File:** crates/hkask-services/src/skills.rs:108

#### SVC-081 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.spec_store() must be initialized; req.name_or_description must be non-empty
- **Post:** spec is persisted to the spec store; returns SpecCaptureResponse with spec_id, name, category, domain_anchor, and complete flag
- **File:** crates/hkask-services/src/spec.rs:103

#### SVC-082 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.spec_store() must be initialized; category_filter if Some must be a valid SpecCategory string
- **Post:** returns Vec<SpecListEntry> for all matching specs; Err(ValidationError) on invalid category
- **File:** crates/hkask-services/src/spec.rs:156

#### SVC-083 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
- **Post:** returns the full Spec with goals on success; Err(ValidationError) on invalid UUID; Err(Spec) on store error
- **File:** crates/hkask-services/src/spec.rs:184

#### SVC-084 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
- **Post:** returns SpecDetail with spec_id, name, category, domain_anchor, and flattened requirements; Err on invalid ID or store error
- **File:** crates/hkask-services/src/spec.rs:195

#### SVC-085 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ctx.spec_store() must be initialized
- **Post:** returns CoherenceResult with coherence_score (0.0–1.0), missing category violations, and suggestions; score=0.0 when store is empty
- **File:** crates/hkask-services/src/spec.rs:221

#### SVC-086 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
- **Post:** returns WritingQualityResult with dimensions_passing count and meets_publication_standard flag (true when all 4 dimensions pass)
- **File:** crates/hkask-services/src/spec.rs:266

#### SVC-087 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
- **Post:** returns SpecCurationRecord from DefaultSpecCurator evaluation; Err on invalid ID or store/curation error
- **File:** crates/hkask-services/src/spec.rs:300

#### SVC-232 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  filter if Some must be a valid principle name; manifests must be loadable
- **Post:** returns VerificationReport with principle results, pass/fail/gap/skip counts, and total assertions
- **File:** crates/hkask-services/src/verification.rs:101

#### SVC-233 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  filter if Some must be a valid principle name
- **Post:** returns serde_json::Value with principles array, totals, and escalation_required flag
- **File:** crates/hkask-services/src/verification.rs:107

#### SVC-279 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  manager must be a valid Arc<WalletManager>; issuer must be a valid Arc<ApiKeyIssuer>
- **Post:** returns WalletService with manager and issuer wired; cybernetics and consent_manager default to None
- **File:** crates/hkask-services/src/wallet.rs:57

#### SVC-280 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  loop_ must be a valid Arc<RwLock<CyberneticsLoop>>
- **Post:** returns self with cybernetics set
- **File:** crates/hkask-services/src/wallet.rs:71

#### SVC-281 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  cm must be a valid Arc<ConsentManager>
- **Post:** returns self with consent_manager set
- **File:** crates/hkask-services/src/wallet.rs:86

#### SVC-282 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self must be constructed
- **Post:** returns &Arc<WalletManager>
- **File:** crates/hkask-services/src/wallet.rs:97

#### SVC-283 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  config must be valid; store must be initialized; event_sink must be valid; cybernetics must be valid
- **Post:** returns Arc<WalletService> with chain ports, price feed, WalletManager, and ApiKeyIssuer all wired; Err on construction failure
- **File:** crates/hkask-services/src/wallet.rs:112

#### SVC-284 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid
- **Post:** returns WalletBalance; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:291

#### SVC-285 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid; cost_rj must be >= 0
- **Post:** returns true if balance >= cost_rj; false otherwise; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:306

#### SVC-286 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid
- **Post:** wallet row exists in store; Ok(()) on success; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:321

#### SVC-287 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid; chain must be a configured ChainId; privacy must be a valid PrivacyMode
- **Post:** returns DepositAddress; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:338

#### SVC-288 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid; chain must be configured; validity_hours must be > 0
- **Post:** returns DepositReference with expiry; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:360

#### SVC-289 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid; limit must be > 0
- **Post:** returns Vec<WalletTransaction>; empty Vec if no transactions; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:383

#### MUST-4—requiresP2affirmativeconsentwhenConsentManagerisconfigured. (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid identifies the user requesting the withdrawal
- **Post:** if consent_manager is Some and consent denied → Err(ConsentDenied);if consent_manager is None → proceeds without consent check (backward compat)
- **File:** crates/hkask-services/src/wallet.rs:407

#### SVC-290 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid must be valid; chain must be configured
- **Post:** returns WithdrawalFee estimate; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:466

#### SVC-291 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid; amount_usdc_micro must be > 0; chain must support shielding
- **Post:** returns TxHash of shield transaction; Err(Wallet) on failure
- **File:** crates/hkask-services/src/wallet.rs:490

#### SVC-292 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid; spending_limit_rj must be >= 0; purpose must be non-empty
- **Post:** returns ApiKeyMaterial with key secret; Err(Wallet) on issuer error
- **File:** crates/hkask-services/src/wallet.rs:516

#### SVC-293 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  key_id must be a valid, non-revoked key
- **Post:** key is revoked; unspent rJoules returned to wallet; Err(Wallet) on issuer error
- **File:** crates/hkask-services/src/wallet.rs:553

#### SVC-294 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid
- **Post:** returns Vec<ApiKeyCapability> of active keys; empty Vec if none; Err(Wallet) on issuer error
- **File:** crates/hkask-services/src/wallet.rs:568

#### SVC-295 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  key_id must be valid
- **Post:** returns Some(ApiKeyCapability) if found; None if not found; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:583

#### SVC-296 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  gas must be >= 0
- **Post:** returns RJoule equivalent using manager's conversion rate
- **File:** crates/hkask-services/src/wallet.rs:600

#### SVC-297 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  rj must be >= 0
- **Post:** returns u64 gas equivalent using manager's conversion rate
- **File:** crates/hkask-services/src/wallet.rs:609

#### SVC-298 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  cybernetics must be attached via with_cybernetics(); agent must be a valid WebID; wallet_id must be valid
- **Post:** wallet-backed budget is registered in CNS for the agent; Err(Wallet) if cybernetics not attached
- **File:** crates/hkask-services/src/wallet.rs:624

#### SVC-299 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  cybernetics must be attached; agent must be valid; wallet_id and key_id must be valid; spending_limit_rj must be >= 0
- **Post:** wallet-backed budget with API key tracking is registered in CNS; Err(Wallet) if cybernetics not attached
- **File:** crates/hkask-services/src/wallet.rs:654

#### SVC-300 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  wallet_id must be valid with sufficient balance; key_id must be valid; amount must be > 0
- **Post:** rJoules are encumbered from wallet to key; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:685

#### SVC-301 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  key_id must have an active encumbrance
- **Post:** encumbrance is released; unspent rJoules returned to wallet; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:707

#### SVC-302 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  key_id must have sufficient encumbered balance; gas_rj must be > 0
- **Post:** rJoules are atomically debited from key's encumbrance; Err(Wallet) on manager error or insufficient balance
- **File:** crates/hkask-services/src/wallet.rs:722

#### SVC-303 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  key_id must be valid
- **Post:** returns Some(Encumbrance) if key has active encumbrance; None if none; Err(Wallet) on manager error
- **File:** crates/hkask-services/src/wallet.rs:737

#### SVC-304 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  key_id must be valid; exhausted and expired are boolean flags
- **Post:** CNS alert emitted if event sink configured; no-op otherwise
- **File:** crates/hkask-services/src/wallet.rs:758


### hkask-storage (195 contracts)

#### P3-sto-agent-registry-schema (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** agents, user_profiles, contacts, scheduled_tasks tables created
- **File:** crates/hkask-storage/src/agent_registry.rs:29

#### P3-sto-agent-registry-insert (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent.name is non-empty
- **Post:** agent inserted into agents table
- **File:** crates/hkask-storage/src/agent_registry.rs:72

#### P3-sto-agent-registry-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  name is non-empty
- **Post:** returns RegisteredAgent if found
- **File:** crates/hkask-storage/src/agent_registry.rs:97

#### P3-sto-agent-registry-list (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of all RegisteredAgent
- **File:** crates/hkask-storage/src/agent_registry.rs:134

#### P3-sto-agent-registry-list-by-kind (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  kind is a valid AgentKind
- **Post:** returns Vec of agents matching kind
- **File:** crates/hkask-storage/src/agent_registry.rs:177

#### P3-sto-agent-registry-remove (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  name is non-empty
- **Post:** agent deleted if existed
- **File:** crates/hkask-storage/src/agent_registry.rs:224

#### P3-sto-agent-registry-profile-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  profile has valid fields
- **Post:** profile upserted
- **File:** crates/hkask-storage/src/agent_registry.rs:243

#### P3-sto-agent-registry-profile-get (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Some(profile) if exists, None otherwise
- **File:** crates/hkask-storage/src/agent_registry.rs:260

#### P3-sto-agent-registry-contact-add (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  contact has valid fields
- **Post:** contact inserted
- **File:** crates/hkask-storage/src/agent_registry.rs:277

#### P3-sto-agent-registry-contact-find (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of matching contacts
- **File:** crates/hkask-storage/src/agent_registry.rs:300

#### P3-sto-agent-registry-contact-list (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent_name is non-empty
- **Post:** returns Vec of contacts
- **File:** crates/hkask-storage/src/agent_registry.rs:329

#### P3-sto-agent-registry-task-add (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  task has valid fields
- **Post:** task inserted
- **File:** crates/hkask-storage/src/agent_registry.rs:354

#### P3-sto-agent-registry-task-list-due (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  now is a valid timestamp
- **Post:** returns Vec of due tasks
- **File:** crates/hkask-storage/src/agent_registry.rs:378

#### P3-sto-agent-registry-task-list-agent (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent_name is non-empty
- **Post:** returns Vec of tasks
- **File:** crates/hkask-storage/src/agent_registry.rs:405

#### P3-sto-agent-registry-task-update (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  task_id is valid, next_run is valid
- **Post:** next_run updated
- **File:** crates/hkask-storage/src/agent_registry.rs:435

#### P2-sto-consent-schema (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** consent_records table created if not exists
- **File:** crates/hkask-storage/src/consent_store.rs:44

#### P2-sto-consent-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  record.webid is non-empty
- **Post:** record inserted or replaced in consent_records
- **File:** crates/hkask-storage/src/consent_store.rs:67

#### P2-sto-consent-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  webid is non-empty
- **Post:** returns Some(record) if found, None otherwise
- **File:** crates/hkask-storage/src/consent_store.rs:100

#### P2-sto-consent-delete (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  webid is non-empty
- **Post:** record deleted if existed
- **File:** crates/hkask-storage/src/consent_store.rs:141

#### P4-sto-database-open (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  path is valid, passphrase is non-empty
- **Post:** returns Database with SQLCipher encryption
- **File:** crates/hkask-storage/src/database.rs:126

#### P4-sto-database-open-ext (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  path is valid, passphrase is non-empty, extensions is valid SQL
- **Post:** returns Database with extensions applied
- **File:** crates/hkask-storage/src/database.rs:149

#### P4-sto-database-in-memory (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns in-memory Database
- **File:** crates/hkask-storage/src/database.rs:177

#### P4-sto-database-in-memory-ext (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  extensions is valid SQL DDL
- **Post:** returns in-memory Database with extensions
- **File:** crates/hkask-storage/src/database.rs:196

#### P4-sto-database-conn-arc (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Arc<Mutex<Connection>> for Store constructors
- **File:** crates/hkask-storage/src/database.rs:226

#### P4-sto-database-open-unwrap (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  path is valid, passphrase is non-empty
- **Post:** returns Database (in-memory if path is ":memory:")
- **File:** crates/hkask-storage/src/database.rs:243

#### P4-sto-database-in-memory-unwrap (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns in-memory Database (panics on failure)
- **File:** crates/hkask-storage/src/database.rs:265

#### P3-sto-embedding-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  conn is a valid SQLite connection
- **Post:** returns EmbeddingStore with default dimension
- **File:** crates/hkask-storage/src/embeddings.rs:70

#### P3-sto-embedding-new-with-dim (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  conn is valid, dim > 0
- **Post:** returns EmbeddingStore with specified dimension
- **File:** crates/hkask-storage/src/embeddings.rs:84

#### P3-sto-embedding-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity_ref is non-empty, vector matches store dimension, model is non-empty
- **Post:** embedding stored and indexed by entity_ref;returns embedding ID
- **File:** crates/hkask-storage/src/embeddings.rs:135

#### P3-sto-embedding-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity_ref is non-empty
- **Post:** returns StoredEmbedding if found;returns Err(NotFound) if not found
- **File:** crates/hkask-storage/src/embeddings.rs:196

#### P3-sto-embedding-search (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  query_vector matches store dimension, limit > 0
- **Post:** returns Vec<SimilarityResult> ordered by ascending distance
- **File:** crates/hkask-storage/src/embeddings.rs:236

#### P3-sto-embedding-delete (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity_ref is non-empty
- **Post:** embedding deleted if existed
- **File:** crates/hkask-storage/src/embeddings.rs:288

#### P3-sto-embedding-count (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns total count of embeddings
- **File:** crates/hkask-storage/src/embeddings.rs:343

#### P3-sto-embedding-prefix (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  prefix is non-empty
- **Post:** returns Vec of entity_refs matching prefix
- **File:** crates/hkask-storage/src/embeddings.rs:357

#### P3-sto-escalation-pending (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns EscalationSignal with Pending status
- **File:** crates/hkask-storage/src/escalation.rs:34

#### P3-sto-escalation-queue-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  conn is a valid SQLite connection
- **Post:** returns EscalationQueue with schema initialized
- **File:** crates/hkask-storage/src/escalation.rs:92

#### P3-sto-escalation-add (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entry has valid domain and output
- **Post:** entry inserted into escalations
- **File:** crates/hkask-storage/src/escalation.rs:124

#### P3-sto-escalation-list-pending (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of pending EscalationEntry
- **File:** crates/hkask-storage/src/escalation.rs:160

#### P3-sto-escalation-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  id is non-empty
- **Post:** returns Some(entry) if found, None otherwise
- **File:** crates/hkask-storage/src/escalation.rs:204

#### P3-sto-escalation-resolve (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  id is non-empty, resolved_by is non-empty
- **Post:** escalation status set to Resolved
- **File:** crates/hkask-storage/src/escalation.rs:264

#### P3-sto-escalation-dismiss (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  id is non-empty, resolved_by is non-empty
- **Post:** escalation status set to Dismissed
- **File:** crates/hkask-storage/src/escalation.rs:282

#### P3-sto-escalation-stats (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns EscalationStats with counts by status
- **File:** crates/hkask-storage/src/escalation.rs:300

#### P3-sto-escalation-summary-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  domain is non-empty, threshold > 0
- **Post:** returns EscalationSummary
- **File:** crates/hkask-storage/src/escalation.rs:343

#### P3-sto-escalation-summary-text (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns summary string with counts and threshold info
- **File:** crates/hkask-storage/src/escalation.rs:359

#### P3-sto-gallery-mode-str (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns "active" or "inactive"
- **File:** crates/hkask-storage/src/gallery.rs:68

#### P3-sto-gallery-schema (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  conn is a valid SQLite connection
- **Post:** gallery tables created if not exists
- **File:** crates/hkask-storage/src/gallery.rs:143

#### P3-sto-gallery-create-test (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  name is non-empty
- **Post:** gallery created and returned
- **File:** crates/hkask-storage/src/gallery.rs:214

#### P3-sto-gallery-create (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  name is non-empty
- **Post:** gallery created and returned
- **File:** crates/hkask-storage/src/gallery.rs:217

#### P3-sto-gallery-add-image-test (🟡 partial)

- **Principle:** ✅ anchored
- **Pre:**  gallery_id is valid, image data is non-empty
- **File:** crates/hkask-storage/src/gallery.rs:262

#### P3-sto-gallery-add-image (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gallery_id is valid, image data is non-empty
- **Post:** image stored in gallery
- **File:** crates/hkask-storage/src/gallery.rs:266

#### P3-sto-gallery-get-image-index-test (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gallery_id is valid
- **Post:** returns GalleryImage if found
- **File:** crates/hkask-storage/src/gallery.rs:317

#### P3-sto-gallery-get-image (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gallery_id is valid
- **Post:** returns GalleryImage if found
- **File:** crates/hkask-storage/src/gallery.rs:320

#### P3-sto-gallery-tag-image-test (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gallery_id and image_hash are valid, tag is non-empty
- **Post:** tag added to image
- **File:** crates/hkask-storage/src/gallery.rs:370

#### P3-sto-gallery-tag-image (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gallery_id and image_hash are valid, tag is non-empty
- **Post:** tag added to image
- **File:** crates/hkask-storage/src/gallery.rs:373

#### P3-sto-gallery-get-tags-test (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gallery_id and image_hash are valid
- **Post:** returns Vec of tags
- **File:** crates/hkask-storage/src/gallery.rs:415

#### P3-sto-gallery-get-tags (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gallery_id and image_hash are valid
- **Post:** returns Vec of tags
- **File:** crates/hkask-storage/src/gallery.rs:418

#### P3-sto-gallery-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gallery_id is valid
- **Post:** returns Gallery if found
- **File:** crates/hkask-storage/src/gallery.rs:444

#### P3-sto-gallery-all-tags (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of all unique tags
- **File:** crates/hkask-storage/src/gallery.rs:481

#### P3-sto-gallery-all-tags (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of all unique tags
- **File:** crates/hkask-storage/src/gallery.rs:484

#### P3-sto-gallery-face-register-test (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  face data is valid
- **Post:** face registered and returned
- **File:** crates/hkask-storage/src/gallery.rs:522

#### P3-sto-gallery-face-register (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  face data is valid
- **Post:** face registered and returned
- **File:** crates/hkask-storage/src/gallery.rs:525

#### P3-sto-gallery-face-list-test (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of faces, optionally filtered by status
- **File:** crates/hkask-storage/src/gallery.rs:563

#### P3-sto-gallery-face-list (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of faces, optionally filtered by status
- **File:** crates/hkask-storage/src/gallery.rs:566

#### P3-sto-gallery-face-get-test (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  face_id is non-empty
- **Post:** returns Face if found
- **File:** crates/hkask-storage/src/gallery.rs:598

#### P3-sto-gallery-face-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  face_id is non-empty
- **Post:** returns Face if found
- **File:** crates/hkask-storage/src/gallery.rs:601

#### P3-sto-gallery-face-remove-test (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  face_id is non-empty
- **Post:** face deleted
- **File:** crates/hkask-storage/src/gallery.rs:627

#### P3-sto-gallery-face-remove (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  face_id is non-empty
- **Post:** face deleted
- **File:** crates/hkask-storage/src/gallery.rs:630

#### P3-sto-gallery-face-update-test (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  face_id is valid, status is valid
- **Post:** face status updated
- **File:** crates/hkask-storage/src/gallery.rs:648

#### P3-sto-gallery-face-update (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  face_id is valid, status is valid
- **Post:** face status updated
- **File:** crates/hkask-storage/src/gallery.rs:651

#### P3-sto-gallery-create-test—creategalleryreturnsvalidrecord (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:740

#### P3-sto-gallery-create-dup-test—duplicatepathisrejected (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:752

#### P3-sto-gallery-add-image-test—add_imagestoresrecord (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:763

#### P3-sto-gallery-get-image-index-test—getbyindex (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:789

#### P3-sto-gallery-get-image-hash-test—getbyhash (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:829

#### P3-sto-gallery-tag-image-test—tag_imagestorestag (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:854

#### P3-sto-gallery-get-tags-test—get_tagsreturnsalltags (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:889

#### P3-sto-gallery-tag-dedup-test—tag_imageignoresduplicate(image_id,tag_type,value) (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:920

#### P3-sto-gallery-face-register-test—register_facecreatesavalidrecord (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:957

#### P3-sto-gallery-face-list-test—list_facesreturnsallregisteredfaces (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:995

#### P3-sto-gallery-face-list-filter-test—list_facesfiltersbystatus (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:1038

#### P3-sto-gallery-face-get-test—get_facereturnscorrectrecord (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:1086

#### P3-sto-gallery-face-get-missing-test—get_faceerrorsonunknownID (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:1115

#### P3-sto-gallery-face-remove-test—remove_facedeletesrecord (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:1123

#### P3-sto-gallery-face-update-test—update_facechangesstatusandnotes (🔴 bare)

- **Principle:** ✅ anchored
- **File:** crates/hkask-storage/src/gallery.rs:1153

#### P3-sto-goal-repo-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  conn is a valid SQLite connection
- **Post:** returns SqliteGoalRepository with schema initialized
- **File:** crates/hkask-storage/src/goals.rs:95

#### P3-sto-goal-repo-telemetry (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Self with telemetry sink configured
- **File:** crates/hkask-storage/src/goals.rs:109

#### P3-sto-goal-try-row (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Goal if row is valid
- **File:** crates/hkask-storage/src/goals.rs:130

#### P3-sto-goal-row-parse (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Goal from row columns
- **File:** crates/hkask-storage/src/goals.rs:146

#### P3-sto-goal-create (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  webid is valid, text is non-empty
- **Post:** goal created and returned
- **File:** crates/hkask-storage/src/goals.rs:202

#### P3-sto-goal-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  goal_id is valid
- **Post:** returns Some(Goal) if found, None otherwise
- **File:** crates/hkask-storage/src/goals.rs:222

#### P3-sto-goal-update-state (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  goal_id is valid, state is valid
- **Post:** goal state updated
- **File:** crates/hkask-storage/src/goals.rs:238

#### P3-sto-goal-list (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  webid is valid
- **Post:** returns Vec of goals, optionally filtered by state
- **File:** crates/hkask-storage/src/goals.rs:270

#### P3-sto-goal-criterion-add (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  goal_id is valid, criterion has description
- **Post:** criterion added to goal
- **File:** crates/hkask-storage/src/goals.rs:294

#### P3-sto-goal-artifact-add (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  goal_id is valid, artifact has content
- **Post:** artifact added to goal
- **File:** crates/hkask-storage/src/goals.rs:318

#### P3-sto-goal-criteria-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  goal_id is valid
- **Post:** returns Vec of GoalCriterion
- **File:** crates/hkask-storage/src/goals.rs:341

#### P3-sto-goal-artifacts-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  goal_id is valid
- **Post:** returns Vec of GoalArtifact
- **File:** crates/hkask-storage/src/goals.rs:369

#### P3-sto-goal-subgoal-create (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  parent_id is valid, text is non-empty
- **Post:** subgoal created with depth = parent.depth + 1
- **File:** crates/hkask-storage/src/goals.rs:411

#### P3-sto-goal-subgoal-list (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  parent_id is valid
- **Post:** returns Vec of child goals
- **File:** crates/hkask-storage/src/goals.rs:445

#### P3-sto-goal-delete (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  goal_id is valid
- **Post:** goal and subgoals deleted
- **File:** crates/hkask-storage/src/goals.rs:459

#### P3-sto-goal-quarantine (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  goal_id is valid, reason is non-empty
- **Post:** goal moved to quarantine
- **File:** crates/hkask-storage/src/goals.rs:478

#### P3-sto-goal-repair (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  goal_id is valid
- **Post:** goal restored from quarantine
- **File:** crates/hkask-storage/src/goals.rs:504

#### P3-sto-goal-quarantine-list (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of QuarantinedGoal
- **File:** crates/hkask-storage/src/goals.rs:560

#### P3-sto-kata-record (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entry.agent_name is non-empty
- **Post:** entry inserted into kata_history
- **File:** crates/hkask-storage/src/kata_history.rs:65

#### P3-sto-kata-list-agent (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent_name is non-empty
- **Post:** returns Vec of entries for this agent
- **File:** crates/hkask-storage/src/kata_history.rs:89

#### P3-sto-kata-count-agent (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent_name is non-empty
- **Post:** returns count of entries
- **File:** crates/hkask-storage/src/kata_history.rs:136

#### P3-sto-kata-count-date (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent_name is non-empty, date is valid ISO date
- **Post:** returns count of entries on that date
- **File:** crates/hkask-storage/src/kata_history.rs:153

#### P3-sto-kata-last (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent_name is non-empty
- **Post:** returns Some(entry) if exists, None otherwise
- **File:** crates/hkask-storage/src/kata_history.rs:174

#### P3-sto-kata-range (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  agent_name is non-empty, from/to are valid ISO dates
- **Post:** returns Vec of entries in range
- **File:** crates/hkask-storage/src/kata_history.rs:221

#### P3-sto-kata-delete-before (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  before_date is a valid ISO date
- **Post:** entries before date deleted;returns count of deleted entries
- **File:** crates/hkask-storage/src/kata_history.rs:270

#### P4-sto-lock-mutex (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Ok(MutexGuard) if lock acquired;returns Err(LockPoisoned) if mutex is poisoned
- **File:** crates/hkask-storage/src/lock_helpers.rs:30

#### P4-sto-lock-read (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Ok(RwLockReadGuard) if lock acquired;returns Err(LockPoisoned) if lock is poisoned
- **File:** crates/hkask-storage/src/lock_helpers.rs:48

#### P4-sto-lock-write (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Ok(RwLockWriteGuard) if lock acquired;returns Err(LockPoisoned) if lock is poisoned
- **File:** crates/hkask-storage/src/lock_helpers.rs:66

#### P3-sto-nu-event-replay (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  observer is valid, category is valid, lookback_secs > 0
- **Post:** returns Vec<NuEvent> within lookback window, weighted by recency
- **File:** crates/hkask-storage/src/nu_event_store.rs:79

#### P3-sto-nu-event-decay (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  category is a valid SpanCategory
- **Post:** returns decay lambda from config or default
- **File:** crates/hkask-storage/src/nu_event_store.rs:116

#### P3-sto-nu-event-cursor-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key is non-empty
- **Post:** cursor value stored
- **File:** crates/hkask-storage/src/nu_event_store.rs:172

#### P3-sto-nu-event-cursor-load (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key is non-empty
- **Post:** returns Some(value) if cursor exists, None otherwise
- **File:** crates/hkask-storage/src/nu_event_store.rs:191

#### P3-sto-nu-event-algedonic-query (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of algedonic signal events
- **File:** crates/hkask-storage/src/nu_event_store.rs:207

#### P4-sto-path-safe-join (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  base is a valid directory, input is a relative path
- **Post:** returns Ok(PathBuf) if path is safe (no traversal, no null bytes);returns Err if path contains traversal or null bytes
- **File:** crates/hkask-storage/src/security.rs:13

#### P1-sto-sovereignty-schema (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** sovereignty_boundaries table created if not exists
- **File:** crates/hkask-storage/src/sovereignty.rs:52

#### P1-sto-sovereignty-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entry.webid is non-empty
- **Post:** entry inserted or replaced
- **File:** crates/hkask-storage/src/sovereignty.rs:182

#### P1-sto-sovereignty-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  webid is non-empty
- **Post:** returns Vec of entries for this WebID
- **File:** crates/hkask-storage/src/sovereignty.rs:222

#### P1-sto-sovereignty-delete (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  webid is non-empty
- **Post:** entries deleted for this WebID
- **File:** crates/hkask-storage/src/sovereignty.rs:274

#### P3-sto-spec-schema (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** specs table created if not exists
- **File:** crates/hkask-storage/src/spec_store.rs:133

#### P3-sto-spec-curation-schema (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** spec_curation_records table created if not exists
- **File:** crates/hkask-storage/src/spec_store.rs:155

#### P3-sto-spec-curation-save (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  record.spec_id is non-empty
- **Post:** record inserted into spec_curation_records
- **File:** crates/hkask-storage/src/spec_store.rs:173

#### P3-sto-spec-curation-load (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  spec_id is non-empty
- **Post:** returns Vec of curation records for this spec
- **File:** crates/hkask-storage/src/spec_store.rs:196

#### P3-sto-spec-curation-since (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of records created after since_ts
- **File:** crates/hkask-storage/src/spec_store.rs:218

#### P3-sto-spec-curation-all (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of all curation records
- **File:** crates/hkask-storage/src/spec_store.rs:246

#### P8-sto-spec-str-enum-as-str (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns lowercase string
- **File:** crates/hkask-storage/src/spec_types.rs:19

#### P8-sto-spec-str-enum-parse (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Some if valid, None otherwise
- **File:** crates/hkask-storage/src/spec_types.rs:27

#### P8-sto-spec-id-new (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns new random SpecId
- **File:** crates/hkask-storage/src/spec_types.rs:46

#### P8-sto-spec-id-from-str (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  s is a valid UUID string
- **Post:** returns SpecId
- **File:** crates/hkask-storage/src/spec_types.rs:54

#### P8-sto-spec-category-as-str (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns snake_case string
- **File:** crates/hkask-storage/src/spec_types.rs:98

#### P8-sto-spec-category-parse (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Some(SpecCategory) if valid, None otherwise
- **File:** crates/hkask-storage/src/spec_types.rs:114

#### P3-sto-triple-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity and attribute are non-empty, owner_webid is valid
- **Post:** returns Triple with defaults for temporal, confidence, access
- **File:** crates/hkask-storage/src/triples.rs:39

#### P3-sto-triple-with-confidence (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Self with confidence set (builder pattern)
- **File:** crates/hkask-storage/src/triples.rs:58

#### P3-sto-triple-with-perspective (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Self with perspective set (builder pattern)
- **File:** crates/hkask-storage/src/triples.rs:67

#### P3-sto-triple-with-visibility (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Self with visibility set (builder pattern)
- **File:** crates/hkask-storage/src/triples.rs:76

#### P3-sto-triple-is-episodic (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns true iff perspective is Some
- **File:** crates/hkask-storage/src/triples.rs:86

#### P3-sto-triple-is-semantic (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns true iff visibility is Public and perspective is None
- **File:** crates/hkask-storage/src/triples.rs:94

#### P3-sto-triple-insert (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  triple has valid entity, attribute, value
- **Post:** triple inserted
- **File:** crates/hkask-storage/src/triples.rs:109

#### P3-sto-triple-query-entity (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity is non-empty
- **Post:** returns Vec of triples matching entity
- **File:** crates/hkask-storage/src/triples.rs:135

#### P3-sto-triple-query-entity-attribute (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity and attribute are non-empty
- **Post:** returns Vec of matching triples
- **File:** crates/hkask-storage/src/triples.rs:154

#### P3-sto-triple-query-perspective (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  perspective is valid
- **Post:** returns Vec of triples for this perspective
- **File:** crates/hkask-storage/src/triples.rs:177

#### P3-sto-triple-query-attribute (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  attribute is non-empty
- **Post:** returns Vec of triples matching attribute
- **File:** crates/hkask-storage/src/triples.rs:197

#### P3-sto-triple-update (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  id is valid
- **Post:** triple value and confidence updated
- **File:** crates/hkask-storage/src/triples.rs:218

#### P3-sto-triple-get-id (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  id is valid
- **Post:** returns Some(Triple) if found, None otherwise
- **File:** crates/hkask-storage/src/triples.rs:293

#### P3-sto-triple-low-confidence (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  limit > 0
- **Post:** returns up to limit triples ordered by confidence ascending
- **File:** crates/hkask-storage/src/triples.rs:316

#### P3-sto-triple-count-below (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  threshold in [0.0, 1.0]
- **Post:** returns count of triples with confidence ≤ threshold
- **File:** crates/hkask-storage/src/triples.rs:342

#### P3-sto-triple-query-below (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  threshold in [0.0, 1.0], limit > 0
- **Post:** returns up to limit triples with confidence ≤ threshold
- **File:** crates/hkask-storage/src/triples.rs:359

#### P3-sto-triple-count-semantic (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns total count of semantic triples
- **File:** crates/hkask-storage/src/triples.rs:386

#### P3-sto-triple-count-entity (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  entity is non-empty
- **Post:** returns count for entity
- **File:** crates/hkask-storage/src/triples.rs:402

#### P3-sto-triple-count-perspective (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  perspective is valid
- **Post:** returns count for perspective
- **File:** crates/hkask-storage/src/triples.rs:419

#### P3-sto-triple-soft-delete (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  id is valid
- **Post:** triple's valid_to set to now (soft-delete)
- **File:** crates/hkask-storage/src/triples.rs:436

#### P3-sto-triple-hard-delete (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  id is valid
- **Post:** triple permanently deleted
- **File:** crates/hkask-storage/src/triples.rs:453

#### P3-sto-triple-delete-prefix (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  prefix is non-empty
- **Post:** matching triples deleted;returns count of deleted triples
- **File:** crates/hkask-storage/src/triples.rs:467

#### P1-sto-user-schema (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** users, replicants, sessions tables created if not exists
- **File:** crates/hkask-storage/src/user_store.rs:79

#### P1-sto-user-register (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  replicant_name is non-empty, passphrase meets requirements
- **Post:** replicant and user records created
- **File:** crates/hkask-storage/src/user_store.rs:96

#### P1-sto-user-login (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  replicant_name is registered, passphrase is correct
- **Post:** returns UserSession on success;returns Err if credentials invalid
- **File:** crates/hkask-storage/src/user_store.rs:170

#### P1-sto-user-logout (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  session_id is valid
- **Post:** session invalidated
- **File:** crates/hkask-storage/src/user_store.rs:208

#### P1-sto-user-passphrase-change (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  replicant_name is registered, old_passphrase is correct
- **Post:** passphrase updated
- **File:** crates/hkask-storage/src/user_store.rs:224

#### P1-sto-user-passphrase-expired (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  replicant_name is registered
- **Post:** returns true if passphrase needs rotation
- **File:** crates/hkask-storage/src/user_store.rs:267

#### P1-sto-user-session-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  session_id is non-empty
- **Post:** returns Some(session) if valid, None otherwise
- **File:** crates/hkask-storage/src/user_store.rs:299

#### P1-sto-user-session-list (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  replicant_name is non-empty
- **Post:** returns Vec of active sessions
- **File:** crates/hkask-storage/src/user_store.rs:317

#### P1-sto-user-replicant-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  replicant_name is non-empty
- **Post:** returns Some(identity) if found, None otherwise
- **File:** crates/hkask-storage/src/user_store.rs:335

#### P1-sto-user-human-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  user_id is valid
- **Post:** returns HumanUser
- **File:** crates/hkask-storage/src/user_store.rs:353

#### P1-sto-user-replicant-list (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  user_id is valid
- **Post:** returns Vec of replicants owned by user
- **File:** crates/hkask-storage/src/user_store.rs:387

#### P1-sto-user-wallet-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  replicant_name is non-empty
- **Post:** returns Some(WalletId) if set, None otherwise
- **File:** crates/hkask-storage/src/user_store.rs:402

#### P1-sto-user-wallet-set (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  replicant_name is registered, wallet_id is valid
- **Post:** wallet_id stored for replicant
- **File:** crates/hkask-storage/src/user_store.rs:416

#### P3-sto-wallet-wal-test—WALmodeforwalletstoreconcurrency (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** journal_mode set to WAL;synchronous set to NORMAL (balance durability vs performance)
- **File:** crates/hkask-storage/src/wallet_store.rs:75

#### P3-sto-wallet-wal-mode (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** journal_mode set to WAL, synchronous set to NORMAL
- **File:** crates/hkask-storage/src/wallet_store.rs:82

#### P3-sto-wallet-balance-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is valid
- **Post:** returns Some(WalletBalance) if wallet exists, None otherwise
- **File:** crates/hkask-storage/src/wallet_store.rs:103

#### P3-sto-wallet-ensure (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is valid
- **Post:** wallet row exists (created if missing)
- **File:** crates/hkask-storage/src/wallet_store.rs:152

#### P3-sto-wallet-list-ids (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns Vec of all WalletId
- **File:** crates/hkask-storage/src/wallet_store.rs:164

#### P3-sto-wallet-credit (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id exists, amount > 0
- **Post:** balance increased by amount, transaction recorded
- **File:** crates/hkask-storage/src/wallet_store.rs:183

#### P3-sto-wallet-debit (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id exists, amount > 0, balance >= amount
- **Post:** balance decreased by amount, transaction recorded;returns Err if insufficient balance
- **File:** crates/hkask-storage/src/wallet_store.rs:210

#### P3-sto-wallet-tx-record (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  tx has valid wallet_id and rjoules_delta
- **Post:** transaction inserted into ledger
- **File:** crates/hkask-storage/src/wallet_store.rs:250

#### P3-sto-wallet-tx-list (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is valid
- **Post:** returns Vec of transactions, optionally limited
- **File:** crates/hkask-storage/src/wallet_store.rs:279

#### P3-sto-wallet-tx-hash-exists (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  tx_hash is non-empty
- **Post:** returns true if hash exists (anti-replay)
- **File:** crates/hkask-storage/src/wallet_store.rs:323

#### P3-sto-wallet-api-key-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  capability has valid key_id and wallet_id
- **Post:** API key stored
- **File:** crates/hkask-storage/src/wallet_store.rs:342

#### P3-sto-wallet-api-key-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is valid
- **Post:** returns Some(capability) if found, None otherwise
- **File:** crates/hkask-storage/src/wallet_store.rs:377

#### P3-sto-wallet-api-key-by-pubkey (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  public_key is valid
- **Post:** returns Some(capability) if found, None otherwise
- **File:** crates/hkask-storage/src/wallet_store.rs:415

#### P3-sto-wallet-api-key-list (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is valid
- **Post:** returns Vec of API key capabilities
- **File:** crates/hkask-storage/src/wallet_store.rs:456

#### P3-sto-wallet-api-key-revoke (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is valid
- **Post:** API key revoked, unspent rJ returned to wallet
- **File:** crates/hkask-storage/src/wallet_store.rs:495

#### P3-sto-wallet-spent-rj-update (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is valid
- **Post:** spent_rj updated
- **File:** crates/hkask-storage/src/wallet_store.rs:528

#### P3-sto-wallet-address-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  address has valid wallet_id and chain
- **Post:** deposit address stored
- **File:** crates/hkask-storage/src/wallet_store.rs:546

#### P3-sto-wallet-address-list (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is valid
- **Post:** returns Vec of deposit addresses
- **File:** crates/hkask-storage/src/wallet_store.rs:575

#### P3-sto-wallet-address-resolve (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  chain is valid, address is non-empty
- **Post:** returns Some(WalletId) if found, None otherwise
- **File:** crates/hkask-storage/src/wallet_store.rs:616

#### P3-sto-wallet-reference-store (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  reference has valid fields
- **Post:** deposit reference stored
- **File:** crates/hkask-storage/src/wallet_store.rs:642

#### P3-sto-wallet-reference-consume (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  reference is valid and not expired
- **Post:** reference consumed, wallet credited;returns Err if already consumed or expired
- **File:** crates/hkask-storage/src/wallet_store.rs:664

#### P3-sto-wallet-reference-purge (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** expired references deleted;returns count of deleted references
- **File:** crates/hkask-storage/src/wallet_store.rs:694

#### P3-sto-wallet-encumber (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id exists, key_id is valid, amount > 0, balance >= amount
- **Post:** rJoules encumbered, balance decreased
- **File:** crates/hkask-storage/src/wallet_store.rs:717

#### P3-sto-wallet-encumbrance-release (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id has active encumbrance
- **Post:** encumbrance released, unspent rJ returned to wallet
- **File:** crates/hkask-storage/src/wallet_store.rs:774

#### P3-sto-wallet-encumbrance-consume (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id has active encumbrance with sufficient remaining
- **Post:** consumed_rj increased, api_keys.spent_rj synced;returns Err if insufficient or not active
- **File:** crates/hkask-storage/src/wallet_store.rs:821

#### P3-sto-wallet-encumbrance-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is valid
- **Post:** returns Some(Encumbrance) if found, None otherwise
- **File:** crates/hkask-storage/src/wallet_store.rs:896


### hkask-templates (52 contracts)

#### TPL-001 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns CapabilityAwareValidator
- **File:** crates/hkask-templates/src/capability_validator.rs:26

#### TPL-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  template_id is non-empty
- **Post:** returns Ok(()) if all required capabilities are satisfied;returns Ok(()) if required_capabilities is empty;returns Err(CapabilityDenied) for first unsatisfied requirement
- **File:** crates/hkask-templates/src/capability_validator.rs:38

#### TPL-011 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns ContractValidator with no lexicon, Warn mode
- **File:** crates/hkask-templates/src/contract_validator.rs:32

#### TPL-012 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  hlexicon is a valid HLexicon
- **Post:** returns ContractValidator with lexicon, Warn mode
- **File:** crates/hkask-templates/src/contract_validator.rs:43

#### TPL-013 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Self with mode updated (builder pattern)
- **File:** crates/hkask-templates/src/contract_validator.rs:55

#### TPL-014 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  template_id is non-empty
- **Post:** returns (Ok(()), unknown_terms) in Warn mode;returns (Err, unknown_terms) in Reject mode if unknown terms found;returns (Ok(()), vec![]) if no lexicon configured
- **File:** crates/hkask-templates/src/contract_validator.rs:64

#### TPL-003 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  inference and mcp are initialized, acp_secret is non-empty
- **Post:** returns ManifestExecutor with default template_base_path
- **File:** crates/hkask-templates/src/executor.rs:72

#### TPL-015 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  content is valid YAML in hlexicon-workspace format
- **Post:** returns HLexicon populated with WordAct, FlowDef, KnowAct terms
- **File:** crates/hkask-templates/src/lexicon.rs:51

#### TPL-016 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  path points to a valid hlexicon-workspace YAML file
- **Post:** returns HLexicon parsed from file contents
- **File:** crates/hkask-templates/src/lexicon.rs:78

#### TPL-017 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns HLexicon from registry/hlexicon/hlexicon-workspace.yaml;respects HKASK_TEMPLATES_PATH env var for path resolution
- **File:** crates/hkask-templates/src/lexicon.rs:90

#### TPL-018 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  markdown is valid hLexicon markdown content
- **Post:** returns Vec of (term, definition, TemplateType) tuples
- **File:** crates/hkask-templates/src/lexicon.rs:114

#### TPL-019 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  terms is a valid catalog from parse_markdown_catalog
- **Post:** returns YAML string in hlexicon-workspace format;terms sorted alphabetically within each domain
- **File:** crates/hkask-templates/src/lexicon.rs:181

#### TPL-020 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  markdown is valid hLexicon markdown content
- **Post:** returns YAML string ready to write to hlexicon-workspace.yaml
- **File:** crates/hkask-templates/src/lexicon.rs:241

#### TPL-004 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  reference is non-empty, registry is initialized
- **Post:** returns Some(BundleManifest) if found via registry or file path;returns None if not found (graceful degradation)
- **File:** crates/hkask-templates/src/manifest_loader.rs:167

#### TPL-005 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  input is non-empty
- **Post:** returns Answer for questions, Instruct for creation, Assist otherwise
- **File:** crates/hkask-templates/src/prompt_strategy.rs:25

#### TPL-006 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  input is non-empty
- **Post:** returns framed prompt string with strategy-specific prefix
- **File:** crates/hkask-templates/src/prompt_strategy.rs:40

#### TPL-007 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns lowercase strategy name
- **File:** crates/hkask-templates/src/prompt_strategy.rs:53

#### TPL-033 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Registry with empty templates, skills, bundles, no lexicon
- **File:** crates/hkask-templates/src/registry.rs:43

#### TPL-034 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  lexicon is a valid HLexicon
- **Post:** hlexicon set — subsequent register() calls validate terms
- **File:** crates/hkask-templates/src/registry.rs:56

#### TPL-035 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** templates cache cleared and reloaded from bootstrap
- **File:** crates/hkask-templates/src/registry.rs:70

#### TPL-036 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  template_id is non-empty
- **Post:** returns Ok(()) if path is safe (no traversal, null bytes, non-ASCII);returns Err(PathTraversal) for unsafe paths
- **File:** crates/hkask-templates/src/registry.rs:82

#### TPL-037 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  entry.id is non-empty, entry.template_type is valid
- **Post:** entry inserted into templates map;validates terms against hlexicon if set (warnings logged)
- **File:** crates/hkask-templates/src/registry.rs:147

#### TPL-038 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is non-empty
- **Post:** returns Some(&RegistryEntry) if found, None otherwise
- **File:** crates/hkask-templates/src/registry.rs:176

#### TPL-039 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns count of templates in registry
- **File:** crates/hkask-templates/src/registry.rs:192

#### TPL-040 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Vec<Skill> with all registered skills
- **File:** crates/hkask-templates/src/registry.rs:200

#### TPL-041 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  visibility is a valid Visibility variant
- **Post:** returns Vec<Skill> filtered by visibility
- **File:** crates/hkask-templates/src/registry.rs:208

#### TPL-042 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is non-empty
- **Post:** returns Some(Skill) if removed, None if not found
- **File:** crates/hkask-templates/src/registry.rs:221

#### TPL-043 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  skill.id is non-empty
- **Post:** skill inserted into skills map
- **File:** crates/hkask-templates/src/registry.rs:230

#### TPL-044 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is non-empty
- **Post:** returns Some(Skill) if found, None otherwise
- **File:** crates/hkask-templates/src/registry.rs:239

#### TPL-045 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  domain is a valid TemplateType
- **Post:** returns Vec<Skill> filtered by domain
- **File:** crates/hkask-templates/src/registry.rs:248

#### TPL-046 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  template_id is non-empty
- **Post:** returns Vec<Skill> referencing the given template
- **File:** crates/hkask-templates/src/registry.rs:261

#### TPL-047 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  bundle.id is non-empty
- **Post:** bundle inserted into bundles map
- **File:** crates/hkask-templates/src/registry.rs:278

#### TPL-048 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is non-empty
- **Post:** returns Some(&BundleManifest) if found, None otherwise
- **File:** crates/hkask-templates/src/registry.rs:287

#### TPL-049 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Vec<&BundleManifest> with all registered bundles
- **File:** crates/hkask-templates/src/registry.rs:296

#### TPL-050 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is non-empty
- **Post:** returns Some(BundleManifest) if removed, None if not found
- **File:** crates/hkask-templates/src/registry.rs:304

#### TPL-051 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  skill_ids is non-empty
- **Post:** returns Some(&BundleManifest) if exact skill set match found;returns None if no exact match
- **File:** crates/hkask-templates/src/registry.rs:314

#### TPL-052 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Registry populated from bootstrap-registry.yaml;all entries have matroshka_limit set to SYSTEM_MAX_RECURSION
- **File:** crates/hkask-templates/src/registry.rs:334

#### TPL-021 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  path is None (in-memory) or a valid filesystem path
- **Post:** returns SqliteRegistry with schema initialized
- **File:** crates/hkask-templates/src/registry_sqlite.rs:73

#### TPL-022 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  conn is a valid SQLite connection
- **Post:** returns SqliteRegistry with schema initialized on the given connection
- **File:** crates/hkask-templates/src/registry_sqlite.rs:97

#### TPL-023 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  lexicon is a valid HLexicon
- **Post:** hlexicon set — subsequent register() calls validate terms
- **File:** crates/hkask-templates/src/registry_sqlite.rs:136

#### TPL-024 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  entry.id is non-empty, entry.template_type is valid
- **Post:** entry inserted or replaced in templates table;lexicon_terms and capabilities synced;validates terms against hlexicon if set
- **File:** crates/hkask-templates/src/registry_sqlite.rs:145

#### TPL-025 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is non-empty
- **Post:** returns RegistryEntry if found;returns Err(NotFound) if not found
- **File:** crates/hkask-templates/src/registry_sqlite.rs:240

#### TPL-026 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is non-empty
- **Post:** template and associated data deleted;returns Some(entry) if existed, None otherwise
- **File:** crates/hkask-templates/src/registry_sqlite.rs:262

#### TPL-027 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  term is non-empty
- **Post:** returns Vec<RegistryEntry> for templates declaring this term
- **File:** crates/hkask-templates/src/registry_sqlite.rs:288

#### TPL-028 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns count of templates in registry;returns 0 on lock error (graceful degradation)
- **File:** crates/hkask-templates/src/registry_sqlite.rs:311

#### TPL-029 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  id is non-empty
- **Post:** returns Some(Skill) if found, None otherwise
- **File:** crates/hkask-templates/src/registry_sqlite.rs:559

#### TPL-030 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Vec<Skill> with all registered skills
- **File:** crates/hkask-templates/src/registry_sqlite.rs:614

#### TPL-031 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  domain is a valid TemplateType
- **Post:** returns Vec<Skill> filtered by domain
- **File:** crates/hkask-templates/src/registry_sqlite.rs:622

#### TPL-032 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  tid is non-empty
- **Post:** returns Vec<Skill> referencing the given template ID
- **File:** crates/hkask-templates/src/registry_sqlite.rs:634

#### TPL-008 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  project_root is a valid directory path
- **Post:** returns SkillLoader configured for the given root
- **File:** crates/hkask-templates/src/skill_loader.rs:49

#### TPL-009 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  registry is initialized
- **Post:** skills from private and public zones loaded and registered;returns SkillLoadResult with loaded skills and any warnings
- **File:** crates/hkask-templates/src/skill_loader.rs:60

#### TPL-010 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  content is a valid SKILL.md file content
- **Post:** returns SkillFrontMatter parsed from YAML front matter;returns default SkillFrontMatter if no front matter present
- **File:** crates/hkask-templates/src/skill_loader.rs:189


### hkask-test-harness (42 contracts)

#### HARN-012 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns TestDb with in-memory SQLite connection and full schema initialized
- **File:** crates/hkask-test-harness/src/lib.rs:55

#### HARN-013 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns MutexGuard<Connection> for direct SQL access
- **File:** crates/hkask-test-harness/src/lib.rs:68

#### HARN-014 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Arc<Mutex<Connection>> clone for Store::new()
- **File:** crates/hkask-test-harness/src/lib.rs:76

#### HARN-015 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  sql is valid SQL
- **Post:** batch executed on the connection
- **File:** crates/hkask-test-harness/src/lib.rs:84

#### HARN-016 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns TestKeystore with temp dir, key file written, 32-byte master key
- **File:** crates/hkask-test-harness/src/lib.rs:107

#### HARN-017 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns &Path to the temp directory
- **File:** crates/hkask-test-harness/src/lib.rs:123

#### HARN-018 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns &Path to the master.key file
- **File:** crates/hkask-test-harness/src/lib.rs:131

#### HARN-019 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns &[u8; 32] reference to the master key
- **File:** crates/hkask-test-harness/src/lib.rs:139

#### HARN-020 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns deterministic WebID from persona b"alice"
- **File:** crates/hkask-test-harness/src/lib.rs:164

#### HARN-021 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns deterministic WebID from persona b"bob"
- **File:** crates/hkask-test-harness/src/lib.rs:172

#### HARN-022 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns deterministic WebID from persona b"carol"
- **File:** crates/hkask-test-harness/src/lib.rs:180

#### HARN-023 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns new random WebID
- **File:** crates/hkask-test-harness/src/lib.rs:188

#### HARN-024 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  bytes is non-empty
- **Post:** returns deterministic WebID from persona bytes
- **File:** crates/hkask-test-harness/src/lib.rs:196

#### HARN-025 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns MockCnsState with homeostatic=true, no throttled tools, empty signals
- **File:** crates/hkask-test-harness/src/lib.rs:218

#### HARN-026 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  throttled_tool is non-empty
- **Post:** returns MockCnsState with homeostatic=false, tool throttled
- **File:** crates/hkask-test-harness/src/lib.rs:231

#### HARN-027 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff valence == Negative
- **File:** crates/hkask-test-harness/src/lib.rs:260

#### HARN-028 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff valence == Positive
- **File:** crates/hkask-test-harness/src/lib.rs:268

#### HARN-029 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns MockCnsRuntime with homeostatic state
- **File:** crates/hkask-test-harness/src/lib.rs:288

#### HARN-030 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  state is a valid MockCnsState
- **Post:** returns MockCnsRuntime with the given state
- **File:** crates/hkask-test-harness/src/lib.rs:298

#### HARN-031 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  event is a valid NuEvent
- **Post:** homeostatic set to false, negative signal appended
- **File:** crates/hkask-test-harness/src/lib.rs:309

#### HARN-032 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** if duration >= 5s, homeostatic restored, throttled tools cleared, positive signal appended
- **File:** crates/hkask-test-harness/src/lib.rs:326

#### HARN-033 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns clone of recent_signals vector
- **File:** crates/hkask-test-harness/src/lib.rs:345

#### HARN-034 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  tool_name is non-empty
- **Post:** returns Throttled if tool in throttled_tools, Active otherwise
- **File:** crates/hkask-test-harness/src/lib.rs:353

#### HARN-035 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff homeostatic flag is true
- **File:** crates/hkask-test-harness/src/lib.rs:367

#### HARN-036 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  domain is non-empty
- **Post:** variety counter for domain incremented by 1
- **File:** crates/hkask-test-harness/src/lib.rs:375

#### HARN-037 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  domain is non-empty
- **Post:** returns variety count for domain, 0 if never recorded
- **File:** crates/hkask-test-harness/src/lib.rs:388

#### HARN-038 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns TempDir that auto-cleans on drop
- **File:** crates/hkask-test-harness/src/lib.rs:426

#### HARN-039 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  span is a valid Span, phase is a valid Phase
- **Post:** returns NuEvent with random observer, depth=0, test observation
- **File:** crates/hkask-test-harness/src/lib.rs:446

#### HARN-040 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  observer is a valid WebID, span is valid, phase is valid
- **Post:** returns NuEvent with specified observer, depth=0, test observation
- **File:** crates/hkask-test-harness/src/lib.rs:461

#### HARN-041 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  entity and attribute are non-empty, value is valid JSON
- **Post:** returns Triple with random owner, sensible defaults
- **File:** crates/hkask-test-harness/src/lib.rs:480

#### HARN-042 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  entity and attribute are non-empty, value is valid JSON, owner is valid
- **Post:** returns Triple with specified owner
- **File:** crates/hkask-test-harness/src/lib.rs:489

#### HARN-001 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns MockInferencePort with empty responses, default="Mock response", model="mock-model"
- **File:** crates/hkask-test-harness/src/mocks.rs:45

#### HARN-002 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  prompt_prefix and response are non-empty
- **Post:** response registered for prefix matching;returns Self for builder chaining
- **File:** crates/hkask-test-harness/src/mocks.rs:59

#### HARN-003 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  response is non-empty
- **Post:** default_response updated;returns Self for builder chaining
- **File:** crates/hkask-test-harness/src/mocks.rs:74

#### HARN-004 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  model is non-empty
- **Post:** model_name updated;returns Self for builder chaining
- **File:** crates/hkask-test-harness/src/mocks.rs:86

#### HARN-005 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** error_override set — subsequent generate() calls return Err
- **File:** crates/hkask-test-harness/src/mocks.rs:99

#### HARN-006 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** error_override cleared — subsequent generate() calls return Ok
- **File:** crates/hkask-test-harness/src/mocks.rs:107

#### HARN-007 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns BoxedStrategy<NuEvent> with valid observer, span, phase, observation, depth 0–7
- **File:** crates/hkask-test-harness/src/strategies.rs:76

#### HARN-008 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns BoxedStrategy<Triple> with non-empty entity, attribute, value, owner
- **File:** crates/hkask-test-harness/src/strategies.rs:97

#### HARN-009 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns BoxedStrategy<CapabilitySpec> with valid resource, action, resource_id
- **File:** crates/hkask-test-harness/src/strategies.rs:116

#### HARN-010 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns BoxedStrategy<Goal> with valid webid, text, state, visibility, depth 0–7
- **File:** crates/hkask-test-harness/src/strategies.rs:146

#### HARN-011 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns BoxedStrategy<TranscriptSegment> with non-empty text, start_ms 0–1hr, duration 100ms–30s
- **File:** crates/hkask-test-harness/src/strategies.rs:185


### hkask-types (99 contracts)

#### TYP-194 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  actor is a valid WebID; action and resource are non-empty strings;
- **Post:** returns an AuditEntry with a new v4 UUID id, current Utc timestamp,
- **File:** crates/hkask-types/src/audit.rs:87

#### TYP-195 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid AuditEntry; correlation_id is a non-empty string
- **Post:** returns self with context.correlation_id set to Some(correlation_id)
- **File:** crates/hkask-types/src/audit.rs:111

#### TYP-196 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid AuditEntry; recipient is a valid WebID
- **Post:** returns self with context.recipient set to Some(recipient)
- **File:** crates/hkask-types/src/audit.rs:121

#### TYP-197 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid AuditEntry; metadata is any valid serde_json::Value
- **Post:** returns self with context.metadata set to the given value
- **File:** crates/hkask-types/src/audit.rs:131

#### TYP-208 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid CnsSpan variant
- **Post:** returns the canonical namespace string (e.g. "cns.tool.web_search"); output matches CANONICAL_NAMESPACES byte-for-byte
- **File:** crates/hkask-types/src/cns.rs:250

#### TYP-209 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  s is a string matching a canonical CnsSpan namespace
- **Post:** returns Ok(CnsSpan) for canonical strings; Err(()) for unknown strings
- **File:** crates/hkask-types/src/cns.rs:335

#### TYP-210 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  attempt >= 0; self.initial_delay_ms, self.multiplier, self.max_delay_ms are valid
- **Post:** returns the exponential backoff delay in ms, capped at self.max_delay_ms
- **File:** crates/hkask-types/src/cns.rs:665

#### TYP-211 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  status is a valid HTTP status code (u16)
- **Post:** returns true if status is in the retryable_status list
- **File:** crates/hkask-types/src/cns.rs:673

#### TYP-198 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is any McpErrorKind variant
- **Post:** returns true only for Unavailable, Timeout, and RateLimited;
- **File:** crates/hkask-types/src/error.rs:117

#### TYP-199 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is any McpErrorKind variant
- **Post:** returns true only for PermissionDenied and FailedPrecondition;
- **File:** crates/hkask-types/src/error.rs:128

#### TYP-170 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  observer is valid, span is valid, phase is valid
- **Post:** returns NuEvent
- **File:** crates/hkask-types/src/event.rs:33

#### TYP-200 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  outcome is a valid serde_json::Value
- **Post:** returns self with outcome set to Some(outcome)
- **File:** crates/hkask-types/src/event.rs:58

#### TYP-201 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  regulation is a valid serde_json::Value
- **Post:** returns self with regulation set to Some(regulation)
- **File:** crates/hkask-types/src/event.rs:67

#### TYP-202 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  parent is a valid EventID
- **Post:** returns self with parent_event set to Some(parent)
- **File:** crates/hkask-types/src/event.rs:76

#### TYP-203 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  visibility is a non-empty string (e.g. "private", "public")
- **Post:** returns self with visibility set to visibility.to_string()
- **File:** crates/hkask-types/src/event.rs:85

#### TYP-171 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  namespace is non-empty
- **Post:** returns SpanNamespace
- **File:** crates/hkask-types/src/event.rs:169

#### TYP-172 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Some(SpanNamespace) if valid, None otherwise
- **File:** crates/hkask-types/src/event.rs:186

#### TYP-204 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid SpanNamespace (canonical)
- **Post:** returns the full namespace string (e.g. "cns.tool")
- **File:** crates/hkask-types/src/event.rs:201

#### TYP-205 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid SpanNamespace (canonical, starts with "cns.")
- **Post:** returns the short name after the "cns." prefix (e.g. "tool")
- **File:** crates/hkask-types/src/event.rs:208

#### TYP-206 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid SpanNamespace (canonical)
- **Post:** returns the SpanCategory for this namespace; unknown prefixes return SpanCategory::Unknown
- **File:** crates/hkask-types/src/event.rs:215

#### TYP-207 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  s is a short_name() string (e.g. "variety", "variety.sensor")
- **Post:** returns the matching SpanCategory; unrecognised prefixes return SpanCategory::Unknown
- **File:** crates/hkask-types/src/event.rs:266

#### TYP-173 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  namespace is valid, path is non-empty
- **Post:** returns Span
- **File:** crates/hkask-types/src/event.rs:339

#### TYP-174 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  kind is valid
- **Post:** returns Span with canonical namespace and path
- **File:** crates/hkask-types/src/event.rs:361

#### TYP-158 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns snake_case state name
- **File:** crates/hkask-types/src/goal.rs:58

#### TYP-159 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Some(GoalState) if valid, None otherwise
- **File:** crates/hkask-types/src/goal.rs:72

#### TYP-160 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true for Completed, Abandoned, Quarantined
- **File:** crates/hkask-types/src/goal.rs:87

#### TYP-161 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  next is a valid GoalState
- **Post:** returns true iff transition is allowed
- **File:** crates/hkask-types/src/goal.rs:105

#### TYP-162 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  goal_id is valid, description is non-empty
- **Post:** returns GoalCriterion
- **File:** crates/hkask-types/src/goal.rs:139

#### TYP-163 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** satisfied set to true
- **File:** crates/hkask-types/src/goal.rs:154

#### TYP-164 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  goal_id is valid, artifact_ref and artifact_type are non-empty
- **Post:** returns GoalArtifact
- **File:** crates/hkask-types/src/goal.rs:174

#### TYP-165 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  webid is valid, text is non-empty
- **Post:** returns Goal with Pending state
- **File:** crates/hkask-types/src/goal.rs:206

#### TYP-166 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Self with display_name set
- **File:** crates/hkask-types/src/goal.rs:226

#### TYP-167 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Self with parent_goal_id and depth set
- **File:** crates/hkask-types/src/goal.rs:235

#### TYP-168 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  transition is valid per can_transition_to
- **Post:** state updated, completed_at set if terminal;returns Err if illegal transition
- **File:** crates/hkask-types/src/goal.rs:250

#### TYP-169 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true for non-terminal states with depth < 7
- **File:** crates/hkask-types/src/goal.rs:272

#### TYP-188 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  replicant_name is a non-empty string (1–64 alphanumeric/hyphen/underscore chars)
- **Post:** returns a deterministic WebID with the "replicant" namespace;
- **File:** crates/hkask-types/src/identity.rs:67

#### TYP-189 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  replicant_name is non-empty; user_id is a valid UserID;
- **Post:** returns a ReplicantIdentity with derived webid, wallet_id=None,
- **File:** crates/hkask-types/src/identity.rs:75

#### TYP-190 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  now is a Unix timestamp (i64); self.expires_at is a valid
- **Post:** returns true if now > self.expires_at (session has expired);
- **File:** crates/hkask-types/src/identity.rs:117

#### TYP-212 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid TemplateType variant
- **Post:** returns the canonical PascalCase string ("WordAct", "KnowAct", "FlowDef")
- **File:** crates/hkask-types/src/lexicon.rs:38

#### TYP-213 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  s is a string in PascalCase or lowercase ("WordAct"/"wordact", "KnowAct"/"knowact", "FlowDef"/"flowdef")
- **Post:** returns Some(TemplateType) if s matches a known variant; None otherwise
- **File:** crates/hkask-types/src/lexicon.rs:49

#### TYP-214 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid TemplateType variant
- **Post:** returns the file extension: "j2" for WordAct/KnowAct, "yaml" for FlowDef
- **File:** crates/hkask-types/src/lexicon.rs:61

#### TYP-215 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid TemplateType variant
- **Post:** returns the MDS specification name: WordAct→"Prompt", KnowAct→"Cognition", FlowDef→"Process"
- **File:** crates/hkask-types/src/lexicon.rs:72

#### TYP-216 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ext is a file extension string (e.g. "j2", "yaml", "yml")
- **Post:** returns Some(KnowAct) for "j2", Some(FlowDef) for "yaml"/"yml"; None for unknown extensions
- **File:** crates/hkask-types/src/lexicon.rs:83

#### TYP-217 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid MdsCategory variant
- **Post:** returns the lowercase category string ("domain", "composition", "trust", "lifecycle", "curation")
- **File:** crates/hkask-types/src/lexicon.rs:117

#### TYP-218 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  term is non-empty, domain is a valid TemplateType, definition is non-empty
- **Post:** returns LexiconTerm with academic_citation=None, mds_category=None
- **File:** crates/hkask-types/src/lexicon.rs:146

#### TYP-219 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  citation is a non-empty string
- **Post:** returns self with academic_citation set to Some(citation.to_string())
- **File:** crates/hkask-types/src/lexicon.rs:159

#### TYP-220 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  cat is a valid MdsCategory variant
- **Post:** returns self with mds_category set to Some(cat)
- **File:** crates/hkask-types/src/lexicon.rs:167

#### TYP-221 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns an empty HLexicon
- **File:** crates/hkask-types/src/lexicon.rs:183

#### TYP-222 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  term is a valid LexiconTerm with a non-empty term field
- **Post:** inserts term into the lexicon keyed by term.term; replaces existing entry if term already present
- **File:** crates/hkask-types/src/lexicon.rs:191

#### TYP-223 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  term is a non-empty string key
- **Post:** returns Some(&LexiconTerm) if term exists in lexicon; None otherwise
- **File:** crates/hkask-types/src/lexicon.rs:198

#### TYP-224 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  term is a non-empty string key
- **Post:** returns true if term exists in lexicon; false otherwise
- **File:** crates/hkask-types/src/lexicon.rs:205

#### TYP-225 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  terms is a slice of String keys to validate
- **Post:** returns Vec<String> of terms not found in the lexicon (empty if all present)
- **File:** crates/hkask-types/src/lexicon.rs:212

#### TYP-226 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns the number of terms in the lexicon
- **File:** crates/hkask-types/src/lexicon.rs:223

#### TYP-227 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true if the lexicon contains no terms; false otherwise
- **File:** crates/hkask-types/src/lexicon.rs:229

#### TYP-228 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns a bootstrap HLexicon with 17 minimal startup terms covering KnowAct, FlowDef, and WordAct domains
- **File:** crates/hkask-types/src/lexicon.rs:235

#### TYP-186 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid R7BotIdentity (constructed via new or deserialized
- **Post:** returns the deterministic WebID derived from the bot's id at
- **File:** crates/hkask-types/src/r7.rs:45

#### TYP-187 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  (none — always callable)
- **Post:** returns a &'static [R7BotIdentity] slice of exactly 7 entries
- **File:** crates/hkask-types/src/r7.rs:86

#### TYP-143 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns category name as &str
- **File:** crates/hkask-types/src/sovereignty.rs:46

#### TYP-144 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns DataCategory (defaults to Episodic for unknown)
- **File:** crates/hkask-types/src/sovereignty.rs:69

#### TYP-145 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true for Episodic, Goals, Wallet, Identity
- **File:** crates/hkask-types/src/sovereignty.rs:88

#### TYP-146 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Private for sovereign, Public for shared categories
- **File:** crates/hkask-types/src/sovereignty.rs:119

#### TYP-147 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns label string
- **File:** crates/hkask-types/src/sovereignty.rs:164

#### TYP-148 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns "sovereign", "shared", or "public"
- **File:** crates/hkask-types/src/sovereignty.rs:178

#### TYP-149 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns UserSovereigntyState with all categories sovereign
- **File:** crates/hkask-types/src/sovereignty.rs:209

#### TYP-150 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  category is valid
- **Post:** returns true iff category is in sovereign set
- **File:** crates/hkask-types/src/sovereignty.rs:238

#### TYP-151 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  category is valid
- **Post:** returns true iff category is in shared set
- **File:** crates/hkask-types/src/sovereignty.rs:253

#### TYP-152 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  category is valid
- **Post:** returns true iff category is in public set
- **File:** crates/hkask-types/src/sovereignty.rs:265

#### TYP-153 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true (always required under Magna Carta)
- **File:** crates/hkask-types/src/sovereignty.rs:275

#### TYP-154 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  category is valid
- **Post:** returns BoundaryClassification (Sovereign, Shared, or Public)
- **File:** crates/hkask-types/src/sovereignty.rs:287

#### TYP-155 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns ConsentState with consent=false
- **File:** crates/hkask-types/src/sovereignty.rs:321

#### TYP-156 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** consent set to true
- **File:** crates/hkask-types/src/sovereignty.rs:334

#### TYP-157 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** consent set to false
- **File:** crates/hkask-types/src/sovereignty.rs:343

#### TYP-183 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  data is any byte slice, including empty
- **Post:** returns a deterministic 32-byte BLAKE3 hash; same input always
- **File:** crates/hkask-types/src/text.rs:13

#### TYP-178 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  (none — always callable, no arguments)
- **Post:** returns a valid RFC 3339 timestamp string for the current UTC moment
- **File:** crates/hkask-types/src/time.rs:15

#### TYP-179 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  audio_path is a non-empty file path string; audio_duration_secs >= 0.0;
- **Post:** returns a TranscriptBundle with format "hkask-transcript-v1",
- **File:** crates/hkask-types/src/transcript.rs:77

#### TYP-180 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid TranscriptBundle
- **Post:** returns the number of TimedWord entries in self.words (usize)
- **File:** crates/hkask-types/src/transcript.rs:97

#### TYP-181 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ms is any u64 millisecond offset
- **Post:** returns Some(&TimedWord) if a word spans ms (start_ms <= ms < end_ms);
- **File:** crates/hkask-types/src/transcript.rs:106

#### TYP-182 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  ms is any u64 millisecond offset
- **Post:** returns Some(&TranscriptSegment) if a segment spans ms
- **File:** crates/hkask-types/src/transcript.rs:118

#### TYP-124 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns "private", "shared", or "public"
- **File:** crates/hkask-types/src/visibility.rs:42

#### TYP-125 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Some(Visibility) if valid, None otherwise
- **File:** crates/hkask-types/src/visibility.rs:53

#### TYP-126 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  owner is valid
- **Post:** returns AccessControl with Private visibility, no perspective
- **File:** crates/hkask-types/src/visibility.rs:93

#### TYP-127 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  perspective and owner are valid
- **Post:** returns AccessControl with Private visibility and perspective
- **File:** crates/hkask-types/src/visibility.rs:107

#### TYP-128 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  owner is valid
- **Post:** returns AccessControl with Public visibility, no perspective
- **File:** crates/hkask-types/src/visibility.rs:121

#### TYP-129 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns AccessControl with Public visibility, no perspective
- **File:** crates/hkask-types/src/visibility.rs:135

#### TYP-130 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff perspective is Some
- **File:** crates/hkask-types/src/visibility.rs:148

#### TYP-131 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff visibility is Public and perspective is None
- **File:** crates/hkask-types/src/visibility.rs:157

#### TYP-132 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Self with perspective set
- **File:** crates/hkask-types/src/visibility.rs:166

#### TYP-133 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Self with visibility set
- **File:** crates/hkask-types/src/visibility.rs:183

#### TYP-134 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Self with perspective set to None
- **File:** crates/hkask-types/src/visibility.rs:220

#### TYP-135 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  value in [0.0, 1.0]
- **Post:** returns Confidence
- **File:** crates/hkask-types/src/visibility.rs:241

#### TYP-136 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Confidence(1.0)
- **File:** crates/hkask-types/src/visibility.rs:251

#### TYP-137 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns f64 value
- **File:** crates/hkask-types/src/visibility.rs:272

#### TYP-138 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  rate >= 0, time >= 0
- **Post:** returns decayed Confidence
- **File:** crates/hkask-types/src/visibility.rs:284

#### TYP-139 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns TemporalContext with valid_from=now, valid_to=None
- **File:** crates/hkask-types/src/visibility.rs:325

#### TYP-140 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns TemporalContext
- **File:** crates/hkask-types/src/visibility.rs:337

#### TYP-141 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns true iff valid_to is None or in the future
- **File:** crates/hkask-types/src/visibility.rs:349

#### TYP-142 (🟡 partial)

- **Principle:** ⚠ unanchored
- **Post:** returns Self with valid_to=now
- **File:** crates/hkask-types/src/visibility.rs:358

#### TYP-184 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid VoiceDesign with all fields populated
- **Post:** returns a prose string describing the voice's gender, age, timbre,
- **File:** crates/hkask-types/src/voice.rs:72

#### TYP-185 (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  self is a valid VoiceDesign with gender_presentation, age_range,
- **Post:** returns a &'static str naming one of the known ElevenLabs voice
- **File:** crates/hkask-types/src/voice.rs:116


### hkask-wallet (27 contracts)

#### P9-wallet-hinkal-port-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  api_base_url is a valid absolute URL; treasury_pubkey is a non-empty account/public key string
- **Post:** HTTP client initialized with rustls TLS;circuit breaker initialized with zero failures
- **File:** crates/hkask-wallet/src/hinkal.rs:184

#### P9-wallet-issuer-struct (🟡 partial)

- **Principle:** ✅ anchored
- **Inv:** private keys are never stored (only public keys persisted);wallet_seed is zeroized on drop
- **File:** crates/hkask-wallet/src/issuer.rs:31

#### P9-wallet-issuer-new (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  store is initialized
- **Post:** returns Ok(ApiKeyIssuer) with resolved wallet_seed in Zeroizing
- **File:** crates/hkask-wallet/src/issuer.rs:55

#### P9-wallet-issuer-create-key (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is valid, spending_limit_rj > 0, purpose is non-empty
- **Post:** returns Ok(ApiKeyMaterial) with fresh Ed25519 keypair
- **File:** crates/hkask-wallet/src/issuer.rs:98

#### P9-wallet-issuer-revoke-key (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is a valid ApiKeyId
- **Post:** key marked as revoked in store
- **File:** crates/hkask-wallet/src/issuer.rs:182

#### P9-wallet-issuer-list-keys (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is a valid WalletId
- **Post:** returns Ok(Vec<ApiKeyCapability>) containing only non-revoked keys
- **File:** crates/hkask-wallet/src/issuer.rs:210

#### P9-wallet-mgr-struct (🟡 partial)

- **Principle:** ✅ anchored
- **Inv:** wallet_seed is zeroized on drop (Zeroizing wrapper);chains map is non-empty after successful build
- **File:** crates/hkask-wallet/src/manager.rs:39

#### P9-wallet-mgr-build (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  config is valid, store is initialized, chains is non-empty; price_feed is a resolved PriceFeed implementation
- **Post:** returns Ok(WalletManager) with resolved wallet_seed;returns Err if wallet_seed resolution fails
- **File:** crates/hkask-wallet/src/manager.rs:64

#### P9-wallet-mgr-key-alert-span(algedonicfeedbackclosure) (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is a valid ApiKeyId
- **Post:** if key is expired → emits cns.wallet.key_expired span (Sense phase);if key is exhausted → emits cns.wallet.key_exhausted span (Sense phase);if event_sink is None → no-op (graceful degradation)
- **File:** crates/hkask-wallet/src/manager.rs:144

#### P9-wallet-mgr-chain-error-span (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  chain is a valid ChainId
- **Post:** emits cns.wallet.chain_error span with error details (Sense phase);if event_sink is None → no-op (graceful degradation)
- **File:** crates/hkask-wallet/src/manager.rs:177

#### P9-wallet-mgr-balance (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is a valid WalletId
- **Post:** returns Ok(balance) with rjoules, gas_equivalent, usdc_equivalent_micro;gas_equivalent == rjoules * config.gas_per_rjoule;balance.rjoules >= 0 (balances are never negative)
- **File:** crates/hkask-wallet/src/manager.rs:213

#### P9-wallet-mgr-api-key-get (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is a valid ApiKeyId
- **Post:** returns Ok(Some(capability)) if key exists and is active;returns Ok(None) if key doesn't exist or is revoked
- **File:** crates/hkask-wallet/src/manager.rs:236

#### P9-wallet-mgr-gas-to-rjoules (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  gas is a non-negative integer
- **Post:** returns RJoule equivalent using the current gas_per_rjoule rate
- **File:** crates/hkask-wallet/src/manager.rs:812

#### P9-wallet-mgr-rjoules-to-gas (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  rj is a non-negative RJoule
- **Post:** returns gas equivalent using the current gas_per_rjoule rate
- **File:** crates/hkask-wallet/src/manager.rs:829

#### P9-wallet-mgr-gas-per-rjoule (🟡 partial)

- **Principle:** ✅ anchored
- **Post:** returns the manager's current gas_per_rjoule rate
- **File:** crates/hkask-wallet/src/manager.rs:838

#### GAS-CALIB-005—runtimecalibrationofwalletgasconversionrate (🟢 full)

- **Principle:** ⚠ unanchored
- **Pre:**  rate > 0
- **Post:** subsequent gas_to_rjoules/rjoules_to_gas use the new rate
- **File:** crates/hkask-wallet/src/manager.rs:846

#### P9-wallet-mgr-fee-estimate (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  chain is a valid ChainId
- **Post:** returns fee estimate derived from live/native USD rate when available;returns Err if configured price feed cannot provide a rate
- **File:** crates/hkask-wallet/src/manager.rs:856

#### P9-wallet-mgr-can-afford (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is a valid WalletId, cost_rj is a valid RJoule
- **Post:** returns Ok(true) iff balance.rjoules >= cost_rj;returns Ok(false) iff balance.rjoules < cost_rj
- **File:** crates/hkask-wallet/src/manager.rs:895

#### P9-wallet-mgr-reserve (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is a valid WalletId, amount is a valid RJoule
- **Post:** if can_afford → Ok(()), reservation is optimistic (no debit);if !can_afford → Err(InsufficientBalance)
- **File:** crates/hkask-wallet/src/manager.rs:909

#### P9-wallet-mgr-settle (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is a valid WalletId, reserved and actual are valid RJoule
- **Post:** wallet balance debited by actual (not reserved);if actual < reserved, difference is implicitly refunded
- **File:** crates/hkask-wallet/src/manager.rs:931

#### P9-wallet-mgr-encumber (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  wallet_id is a valid WalletId, key_id is a valid ApiKeyId, amount > 0
- **Post:** amount rJoules locked against wallet for key_id;emits cns.wallet.encumbered span if event_sink configured
- **File:** crates/hkask-wallet/src/manager.rs:996

#### P9-wallet-mgr-release-encumbrance (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is a valid ApiKeyId
- **Post:** unspent rJoules returned to wallet;idempotent — releasing already-released/consumed encumbrance is no-op
- **File:** crates/hkask-wallet/src/manager.rs:1028

#### P9-wallet-mgr-consume (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is a valid ApiKeyId, gas_rj > 0
- **Post:** gas_rj deducted from key's active encumbrance (atomic);if encumbrance fully consumed → status transitions to 'consumed'
- **File:** crates/hkask-wallet/src/manager.rs:1052

#### P9-wallet-mgr-get-encumbrance (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  key_id is a valid ApiKeyId
- **Post:** returns Ok(Some(encumbrance)) if key has active encumbrance;returns Ok(None) if key has no encumbrance
- **File:** crates/hkask-wallet/src/manager.rs:1069

#### P9-wallet-sign-withdrawal (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  chain is a valid ChainId, tx_bytes is non-empty
- **Post:** returns Ok(signature) — 64-byte Ed25519 signature;treasury key loaded, used, and zeroized within this call
- **File:** crates/hkask-wallet/src/signing.rs:63

#### P9-wallet-sign-hinkal-message (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  message is any byte slice (including empty)
- **Post:** returns Ok(signature) — 64-byte Ed25519 signature;treasury key loaded, used, and zeroized within this call
- **File:** crates/hkask-wallet/src/signing.rs:86

#### P9-wallet-sign-capability (🟢 full)

- **Principle:** ✅ anchored
- **Pre:**  capability is a valid, fully-populated ApiKeyCapability
- **Post:** returns Ok(hex_signature) — 128-char hex-encoded Ed25519 signature;delegates to hkask_keystore::sign_api_key_capability (isolated boundary)
- **File:** crates/hkask-wallet/src/signing.rs:111



---

## Next Steps

1. **Review the inventory** — identify patterns, gaps, and inconsistencies
2. **Design rSolidity vocabulary** — how `require!()`, `assert!()`, `revert!()`, `emit!()`, `#[ocap]` map to these contracts
3. **Pick a starting contract** — rewrite one well-formed contract in rSolidity to establish the pattern
4. **Write the rSolidity crate** — `crates/hkask-rsolidity/` with macro implementations
5. **Migrate contracts one crate at a time** — strangler fig: old `/// REQ:` stays, new `#[rSolidity]` replaces

Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
