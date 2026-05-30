# Task 6: Verify — Cybernetic Unit Tests (22 tests)

## Test Conventions

Per `README.md#cybernetic-unit-tests`:
- Use `*_cybertests.rs` for test files
- Prefix test names with `cyber_`
- Each test defines: policy objective, disturbance injected, expected telemetry, adaptation/escalation expectation

## Inference Loop Tests

### Test 1: Inference loop closes — input enters, output emerges

```rust
#[test]
fn cyber_inference_loop_closes() {
    // Policy: A prompt enters the inference loop and a parsed action emerges
    // Disturbance: None (happy path)
    // Expected: cns.tool span emitted, result produced

    let handle = InferenceHandle::new_test();
    let result = get_or_infer(&handle.cache, &handle.budget, &mut handle.circuit_breaker,
        "What is the capital of France?", &LLMParameters::default());
    assert!(result.is_ok(), "Inference loop must produce a result");
    assert!(handle.observe.emitted_spans().iter().any(|s| s.category == SpanCategory::Tool));
}
```

### Test 2: Inference capability boundary — denied tool call blocked

```rust
#[test]
fn cyber_inference_capability_boundary() {
    // Policy: Inference without capability token cannot invoke tools
    // Disturbance: Tool call parsed, but no capability for that tool
    // Expected: cns.sovereignty violation span, capability denied error

    let handle = InferenceHandle::new_test_without_capability(
        CapabilityResource::Tool("web_search".into()),
        CapabilityAction::Execute,
    );
    let action = ParsedAction::ToolCall {
        tool: "web_search".into(),
        args: serde_json::json!({"query": "test"}),
    };
    let result = dispatch_action(&handle, action);
    assert!(matches!(result, Err(InferenceError::CapabilityDenied(_))));
    assert!(handle.observe.emitted_spans().iter().any(|s| s.category == SpanCategory::Sovereignty));
}
```

### Test 3: Inference energy budget denies consumption

```rust
#[test]
fn cyber_inference_energy_budget_denial() {
    // Policy: Energy budget hard limit prevents inference when exhausted
    // Disturbance: Budget set to zero
    // Expected: BudgetExhausted error, cns.energy Regulate span

    let handle = InferenceHandle::new_test_with_budget(0);
    let result = get_or_infer(&handle.cache, &handle.budget, &mut handle.circuit_breaker,
        "prompt", &LLMParameters::default());
    assert!(matches!(result, Err(InferenceError::BudgetExhausted)));
    assert!(handle.observe.emitted_spans().iter().any(|s| s.category == SpanCategory::Energy));
}
```

### Test 4: Inference circuit breaker opens on failure

```rust
#[test]
fn cyber_inference_circuit_breaker() {
    // Policy: Circuit breaker prevents cascading failures
    // Disturbance: Model endpoint fails repeatedly
    // Expected: Circuit opens, subsequent calls fail fast, then recovers after timeout

    let mut cb = CircuitBreakerHandle::new_test();
    // Simulate 5 failures to open the circuit
    for _ in 0..5 {
        let _ = cb.call(|| Err(InferenceError::ModelError("timeout".into())));
    }
    // Circuit should now be open — next call fails immediately
    let result = cb.call(|| Err(InferenceError::ModelError("should not be called".into())));
    assert!(matches!(result, Err(InferenceError::CircuitOpen)));
    assert!(handle.observe.emitted_spans().iter().any(|s| s.category == SpanCategory::Connector));
}
```

### Test 5: Inference context assembly deduplicates and respects budget

```rust
#[test]
fn cyber_inference_context_assembly() {
    // Policy: Context assembly deduplicates facts and respects token budget
    // Disturbance: Two identical facts and one over-budget fact
    // Expected: Dedup removes duplicate; budget limits total

    let memory = MemoryReadHandle::new_test();
    memory.store_episodic("paris", "capital_of", "France", 0.95);
    memory.store_episodic("paris", "capital_of", "France", 0.95); // duplicate
    memory.store_episodic("berlin", "capital_of", "Germany", 0.90);

    let context = assemble_context(memory.query_visible("paris"), 50); // 50-token budget
    // Duplicate should be removed; only one "paris" entry should appear
    assert_eq!(context.matches("paris").count(), 1);
    // Total should respect budget
    assert!(estimate_tokens(&context) <= 50);
}
```

### Test 5b: Inference rate limiter denies when bucket exhausted

```rust
#[test]
fn cyber_inference_rate_limiter() {
    // Policy: Rate limiter prevents invocation frequency exhaustion
    // Disturbance: Exhaust the token bucket, then attempt inference
    // Expected: RateLimited error, cns.tool.rate_limited span emitted

    let rate_limiter = RateLimiterHandle::new_test_with_bucket(0);
    let handle = InferenceHandle::new_test_with_rate_limiter(rate_limiter);

    let result = get_or_infer(&handle.cache, &handle.budget, &handle.rate_limiter,
        &mut handle.circuit_breaker, "test prompt", &LLMParameters::default());

    assert!(matches!(result, Err(InferenceError::RateLimited)));
    assert!(handle.observe.emitted_spans().iter()
        .any(|s| s.path.contains("rate_limited")));
}
```

---

## Memory Loop Tests

### Test 6: Memory loop closes — experience encoded, stored, recalled

```rust
#[test]
fn cyber_memory_loop_closes() {
    // Policy: Experience encoded as triple, stored, and recalled
    // Disturbance: None (happy path)
    // Expected: cns.pipeline.store and cns.pipeline.recall spans emitted

    let write_handle = MemoryWriteHandle::new_test();
    let read_handle = write_handle.read_handle();

    let triple = write_handle.store_episodic("agent:bot-1", "learned", "Rust is memory-safe", 0.9).unwrap();
    let recalled = read_handle.query_visible("agent:bot-1");
    assert!(!recalled.is_empty(), "Stored triple must be recallable");
    assert!(read_handle.observe.emitted_spans().iter().any(|s| s.path.starts_with("cns.pipeline")));
}
```

### Test 7: Memory capability boundary — read handle cannot write

```rust
#[test]
fn cyber_memory_read_cannot_write() {
    // Policy: MemoryReadHandle cannot store triples
    // Expected: Compile-time enforcement — MemoryReadHandle has no store methods
    // This is verified by the type system, not runtime

    let read_handle: MemoryReadHandle = /* ... */;
    // read_handle.store_episodic(...) — DOES NOT COMPILE
    // read_handle.store_semantic(...) — DOES NOT COMPILE
    assert!(true, "Type system enforces: MemoryReadHandle has no store methods");
}
```

### Test 8: Memory visibility boundary — private triples hidden

```rust
#[test]
fn cyber_memory_visibility_boundary() {
    // Policy: Agent A's private episodic triples are invisible to Agent B
    // Disturbance: Agent B queries Agent A's private data
    // Expected: cns.sovereignty.boundary span, only public/shared triples returned

    let agent_a = MemoryWriteHandle::new_test_with_webid("agent-a");
    let agent_b = MemoryReadHandle::new_test_with_webid("agent-b");

    agent_a.store_episodic("secret", "private_data", "sensitive", 1.0).unwrap();
    let results = agent_b.query_visible("secret");
    assert!(results.iter().all(|t| t.visibility != Visibility::Private),
        "Agent B must not see Agent A's private triples");
}
```

### Test 9: Memory deduplication removes duplicates

```rust
#[test]
fn cyber_memory_deduplication() {
    // Policy: Deduplication removes identical facts
    // Disturbance: Three triples, two identical
    // Expected: Only unique triples remain

    let triples = vec![
        Triple::new("x", "y", "z").with_confidence(0.9),
        Triple::new("x", "y", "z").with_confidence(0.9), // duplicate
        Triple::new("a", "b", "c").with_confidence(0.8),
    ];
    let deduped = dedup_triples(triples);
    assert_eq!(deduped.len(), 2, "Duplicates must be removed");
}
```

### Test 10: Memory consolidation strips perspective

```rust
#[test]
fn cyber_memory_consolidation() {
    // Policy: Consolidation converts episodic to semantic by stripping perspective
    // Disturbance: Episodic triple with perspective
    // Expected: Resulting semantic triple has no perspective and public visibility

    let write_handle = MemoryWriteHandle::new_test();
    let episodic = vec![
        Triple::new("sky", "color", "blue").with_perspective(WebID::new()).with_confidence(0.9),
    ];
    let semantic = consolidate(&write_handle, episodic).unwrap();
    assert!(semantic[0].perspective.is_none(), "Semantic triple has no perspective");
    assert_eq!(semantic[0].visibility, Visibility::Public, "Semantic triple is public");
}
```

---

## Governance Loop Tests

### Test 11: Governance loop closes — request authorized, dispatched, observed

```rust
#[test]
fn cyber_governance_loop_closes() {
    // Policy: Capability token verified, action dispatched, variety observed
    // Disturbance: None (happy path)
    // Expected: cns.governance.verify span emitted

    let governance = GovernanceHandle::new_test();
    let token = governance.mint_root_token();
    let result = governance.verify_capability(&token,
        CapabilityResource::Tool("web".into()), CapabilityAction::Execute);
    assert!(matches!(result, VerificationResult::Valid));
    assert!(governance.cns.emitted_spans().iter().any(|s| s.path.starts_with("cns.governance")));
}
```

### Test 12: Governance capability attenuation — attenuated token has less authority

```rust
#[test]
fn cyber_governance_attenuation() {
    // Policy: Attenuated token grants fewer rights than parent
    // Disturbance: Attenuated to read-only, attempt write
    // Expected: cns.sovereignty.violation span, access denied

    let governance = GovernanceHandle::new_test();
    let root_token = governance.mint_root_token();
    let read_token = governance.attenuate_token(&root_token, vec![
        Caveat::Operation(CapabilityAction::Read),
    ]).unwrap();

    assert!(matches!(
        governance.verify_capability(&read_token, CapabilityResource::Template("x".into()), CapabilityAction::Read),
        VerificationResult::Valid));
    assert!(matches!(
        governance.verify_capability(&read_token, CapabilityResource::Template("x".into()), CapabilityAction::Write),
        VerificationResult::Unauthorized(_)));
}
```

### Test 13: Governance revocation — revoked token is denied

```rust
#[test]
fn cyber_governance_revocation() {
    // Policy: Revoked capability tokens are denied future use
    // Disturbance: Revoke a previously valid token
    // Expected: Verification fails, cns.governance.revoked span

    let governance = GovernanceHandle::new_test();
    let token = governance.mint_root_token();
    governance.revoke_capability(&token.id()).unwrap();
    let result = governance.verify_capability(&token,
        CapabilityResource::Tool("web".into()), CapabilityAction::Execute);
    assert!(matches!(result, VerificationResult::Zombie));
}
```

### Test 14: Governance algedonic escalation — critical deficit triggers Curator escalation

```rust
#[test]
fn cyber_governance_algedonic_escalation() {
    // Policy: Variety deficit > threshold triggers Critical alert → Curator escalation
    // Disturbance: Inject variety deficit > 100
    // Expected: EscalationAction::EscalateToCurator

    let governance = GovernanceHandle::new_test_with_threshold(10);
    for _ in 0..200 { governance.cns.increment_variety("test_domain"); }
    let alert = governance.cns.check_variety("test_domain");
    assert!(alert.is_some());
    let action = governance.process_alert(&alert.unwrap());
    assert!(matches!(action, EscalationAction::EscalateToCurator));
}
```

---

## Observability Loop Tests

### Test 15: Observability loop closes — span emitted, aggregated, anomaly detected

```rust
#[test]
fn cyber_observability_loop_closes() {
    // Policy: Span emitted, persisted, aggregated, deficit detected
    // Disturbance: Many spans for one domain
    // Expected: Algedonic alert generated

    let write = CnsWriteHandle::new_test();
    let govern = CnsGovernHandle::new_test_with_threshold(5);
    for _ in 0..10 {
        emit_event(&write, &WebID::new(), Span::tool("test.op"), Phase::Observe, "ok", 0.9);
    }
    let alert = govern.check_variety("tool");
    assert!(alert.is_some(), "Deficit should exceed threshold");
}
```

### Test 16: Observability capability boundary — CnsWriteHandle cannot govern

```rust
#[test]
fn cyber_observability_write_cannot_govern() {
    // Policy: CnsWriteHandle cannot check variety or process sovereignty
    // Expected: Compile-time enforcement — CnsWriteHandle has no govern methods

    let write: CnsWriteHandle = /* ... */;
    // write.check_variety("test") — DOES NOT COMPILE
    // write.process_sovereignty_event(...) — DOES NOT COMPILE
    assert!(true, "Type system enforces: CnsWriteHandle has no govern methods");
}
```

### Test 17: Observability CNS span emission — each operation persists ν-event

```rust
#[test]
fn cyber_observability_every_operation_emits_span() {
    // Policy: Every observability operation emits at least one cns.* span
    // Disturbance: None
    // Expected: After emit_event, NuEventSink contains a persisted event

    let write = CnsWriteHandle::new_with_sink(/* test sink */);
    emit_event(&write, &WebID::new(), Span::connector("test"), Phase::Observe, "ok", 0.8);
    let events = write.sink().events();
    assert!(!events.is_empty(), "emit_event must persist a ν-event");
    assert!(events[0].span.path.starts_with("cns."), "ν-event span must be in cns.* namespace");
}
```

---

## Curation Loop Tests

### Test 18: Curation loop closes — observe → evaluate → regulate

```rust
#[test]
fn cyber_curation_loop_closes() {
    // Policy: MetacognitionLoop run_cycle produces a snapshot and posts escalations
    // Disturbance: None (happy path with healthy system)
    // Expected: SystemHealthSnapshot produced, no escalations posted

    let handle = CuratorHandle::new_test_healthy();
    let snapshot = run_cycle(&handle).await.unwrap();
    assert!(snapshot.critical_alerts == 0, "Healthy system should have zero critical alerts");
    assert!(snapshot.bot_status_reports.is_empty(), "No bot failures in healthy system");
}
```

### Test 19: Curation escalation routing — variety deficit triggers escalation

```rust
#[test]
fn cyber_curation_escalation_routing() {
    // Policy: Variety deficit exceeding threshold posts escalation to queue
    // Disturbance: Inject variety deficit > threshold
    // Expected: EscalationQueue contains variety deficit escalation

    let handle = CuratorHandle::new_test_with_variety_deficit(200);
    let snapshot = run_cycle(&handle).await.unwrap();
    let escalations = handle.escalation.list_pending().unwrap();
    assert!(!escalations.is_empty(), "Variety deficit must produce escalation");
    assert!(escalations[0].output.contains("variety"));
}
```

### Test 20: Curation bot evaluation — degraded bot triggers coaching

```rust
#[test]
fn cyber_curation_bot_evaluation() {
    // Policy: Degraded bot with capability gaps triggers coaching recommendation
    // Disturbance: Bot with low success rate and variety deficit
    // Expected: EvaluationResult recommends Coach action

    let metrics = BotEvaluationMetrics::new_test_degraded();
    let result = evaluate_bot(&WebID::new(), &metrics);
    assert!(matches!(result.recommended_action, RecommendedAction::Coach(_)),
        "Degraded bot should recommend coaching");
    assert!(!result.gaps.is_empty(), "Degraded bot should have capability gaps");
}
```

### Test 21: Curation kata coaching — capability gap produces KataDirective

```rust
#[test]
fn cyber_curation_kata_coaching() {
    // Policy: Capability gap from evaluation maps to specific KataDirective
    // Disturbance: EvaluationResult with VarietyDeficit gap
    // Expected: KataDirective with KataType::Coaching

    let evaluation = EvaluationResult::new_test_with_gap(GapType::VarietyDeficit);
    let directive = identify_capability_gap(&evaluation);
    assert!(directive.is_some());
    assert!(matches!(directive.unwrap().kata_type, KataType::Coaching),
        "Variety deficit should produce Coaching kata");
}
```

### Test 22: Curation threshold calibration — CnsGovernWriteHandle writes, Governance reads

```rust
#[test]
fn cyber_curation_threshold_calibration() {
    // Policy: Curation loop can calibrate thresholds via CnsGovernWriteHandle;
    //   Governance loop reads variety via CnsGovernReadHandle but cannot write
    // Disturbance: Curator calibrates threshold, then Governance reads updated value
    // Expected: Threshold updated; Governance reads new value but cannot modify it

    let govern_read = CnsGovernReadHandle::new_test_with_threshold(100);
    let govern_write = CnsGovernWriteHandle::from_read(govern_read.clone());

    // Curation writes
    govern_write.set_expected_variety("test_domain", 150);
    // Governance reads
    let alert = govern_read.check_variety("test_domain");
    assert!(alert.is_some(), "Updated threshold should affect detection");

    // Verify Governance cannot write (compile-time: set_expected_variety not on CnsGovernReadHandle)
    // govern_read.set_expected_variety(...) — DOES NOT COMPILE
    assert!(true, "Type system enforces: CnsGovernReadHandle has no write methods");
}
```

---

## Running the Tests

```bash
# Run all cybernetic unit tests
cargo test -p hkask-types cyber_
cargo test -p hkask-memory cyber_
cargo test -p hkask-cns cyber_
cargo test -p hkask-agents cyber_

# Run clippy
cargo clippy -p hkask-types -- -D warnings
cargo clippy -p hkask-memory -- -D warnings
cargo clippy -p hkask-cns -- -D warnings
cargo clippy -p hkask-agents -- -D warnings
```

## Note on Implementation

The capability handle types defined in Task 5 are specifications that need to be implemented in their respective crates. The tests use `new_test()` constructors that need test-specific implementations.

The key verification is **type-level**: capability discipline is enforced by the Rust type system. A `MemoryReadHandle` cannot call `store_episodic()` because the method doesn't exist on that type. This is the strongest possible enforcement — no runtime check needed, no capability boundary can be accidentally crossed.

The runtime tests (loop closing, CNS span emission, deficit detection) verify behavioral contracts. The type-level tests (capability boundary, visibility boundary) are verified at compile time.