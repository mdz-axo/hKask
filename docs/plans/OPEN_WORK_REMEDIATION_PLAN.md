# Open Work Remediation Plan

**Version:** 1.0  
**Created:** 2026-05-23  
**Status:** Pending  
**Priority:** P0 (Critical) → P1 (Important)

---

## Overview

This plan addresses the three remaining open work items for hKask v0.21.0 MVP completion:

| ID | Task | Priority | Estimated Effort | Dependencies |
|----|------|----------|------------------|--------------|
| **P0-01** | Fix `goals.rs` trait implementation mismatches | P0 | 2-3 hours | None |
| **P0-02** | Integration tests for inference pipeline | P0 | 4-6 hours | P0-01 |
| **P1-01** | Phase 4 Production documentation | P1 | 3-4 hours | P0-01, P0-02 |

**Total Estimated Effort:** 9-13 hours

---

## P0-01: Fix goals.rs Trait Implementation Mismatches

### Problem Statement

The `SqliteGoalRepository` implementation in `crates/hkask-storage/src/goals.rs` has trait implementation mismatches:

1. **Return type mismatch**: Trait uses generic `Result<T>` but implementation uses `rusqlite::Result<T>`
2. **Missing error type definition**: No custom error type for goal repository operations
3. **Chrono import at EOF**: `use chrono::Utc;` appears at line 688 (end of file) instead of top of file

### Root Cause Analysis

The `GoalRepositoryPort` trait was defined with a generic `Result` return type without specifying the error type:

```rust
pub trait GoalRepositoryPort {
    fn create_goal(...) -> Result<Goal>;  // Ambiguous - which Result?
    fn get_goal(...) -> Result<Option<Goal>>;
    // ...
}
```

The implementation uses `rusqlite::Result` directly, which conflicts with the trait's ambiguous return type.

### Solution

**Step 1: Define custom error type**

Create `GoalRepositoryError` enum that wraps rusqlite errors and domain-specific errors:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoalRepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    
    #[error("Visibility denied: {0}")]
    VisibilityDenied(String),
    
    #[error("Goal not found: {0}")]
    NotFound(String),
    
    #[error("Invalid goal state transition: {0}")]
    InvalidTransition(String),
    
    #[error("Subgoal depth exceeded (max 7): {0}")]
    MaxDepthExceeded(String),
}

pub type Result<T> = std::result::Result<T, GoalRepositoryError>;
```

**Step 2: Update trait definition**

```rust
pub trait GoalRepositoryPort {
    fn create_goal(
        &self,
        token: &GoalCapabilityToken,
        webid: &WebID,
        text: &str,
        visibility: Visibility,
    ) -> Result<Goal>;
    // ... update all methods
}
```

**Step 3: Update implementation**

Convert all `rusqlite::Result` returns to `GoalRepositoryError`:

```rust
fn verify_capability(&self, token: &GoalCapabilityToken, required_op: GoalOp) -> Result<()> {
    if !token.is_valid() {
        return Err(GoalRepositoryError::CapabilityDenied(
            "Token invalid or expired".to_string(),
        ));
    }
    if !token.can_perform(required_op) {
        return Err(GoalRepositoryError::CapabilityDenied(
            format!("Missing capability for operation: {:?}", required_op),
        ));
    }
    Ok(())
}
```

**Step 4: Fix imports**

Move `use chrono::Utc;` to top of file with other imports.

**Step 5: Update tests**

Update test assertions to handle new error type:

```rust
#[test]
fn visibility_enforcement() {
    // ...
    let result = repo.get_goal(&other_token, goal.id);
    assert!(matches!(result, Err(GoalRepositoryError::VisibilityDenied(_))));
}
```

### Files to Modify

| File | Changes |
|------|---------|
| `crates/hkask-storage/src/goals.rs` | Add error type, fix trait impl, fix imports |
| `crates/hkask-storage/src/lib.rs` | Export `GoalRepositoryError` |

### Acceptance Criteria

- [ ] `cargo check -p hkask-storage` passes
- [ ] `cargo test -p hkask-storage` passes (8 tests)
- [ ] `cargo clippy -p hkask-storage -- -D warnings` passes
- [ ] All error variants have meaningful messages
- [ ] Tests verify error conditions explicitly

---

## P0-02: Integration Tests for Inference Pipeline

### Problem Statement

No integration tests exist for the full Okapi inference pipeline with mock server. Current tests are unit-only.

### Test Requirements

**Test Categories:**

1. **End-to-End Inference** — Full generate request/response cycle
2. **Rate Limiting** — Verify rate limiter blocks excess requests
3. **Circuit Breaker** — Verify circuit breaker opens after failures
4. **Multi-Okapi Failover** — Verify automatic failover to healthy instance
5. **Prompt Caching** — Verify cache hits/misses and TTL behavior
6. **Token Probabilities** — Verify `n_probs` extraction in response
7. **CNS Span Emission** — Verify spans are emitted for all operations
8. **OCAP Enforcement** — Verify capability checks at inference boundary

### Test Architecture

```
hkask-testing/
├── test-harnesses/
│   ├── mock_okapi.rs          # Existing mock Okapi server
│   ├── mocks.rs               # Mock adapters
│   ├── fixtures.rs            # Test fixtures (personas, templates)
│   └── temp_dirs.rs           # Temp directory management
├── integration-tests/
│   ├── inference_pipeline.rs  # NEW: Full pipeline tests
│   ├── rate_limiting.rs       # NEW: Rate limit tests
│   ├── circuit_breaker.rs     # NEW: Circuit breaker tests
│   ├── multi_okapi.rs         # NEW: Failover tests
│   ├── prompt_cache.rs        # NEW: Cache tests
│   └── cns_spans.rs           # NEW: CNS span verification
└── unit-tests/                # Existing unit tests
```

### Test Implementation Plan

**Test 1: End-to-End Inference**

```rust
#[tokio::test]
async fn test_end_to_end_inference() {
    // Arrange
    let mock_server = MockOkapiServer::start().await;
    let client = OkapiHttpClient::new(&mock_server.base_url());
    let request = GenerateRequest {
        model: "qwen3:8b".to_string(),
        prompt: "What is 2+2?".to_string(),
        options: Some(GenerateOptions {
            temperature: Some(0.2),
            max_tokens: Some(100),
            n_probs: Some(5),
        }),
    };

    // Act
    let response = client.generate(&request).await.unwrap();

    // Assert
    assert!(!response.response.is_empty());
    assert_eq!(response.model, "qwen3:8b");
    assert!(response.latency_ms > 0);
}
```

**Test 2: Rate Limiting**

```rust
#[tokio::test]
async fn test_rate_limiting_blocks_excess_requests() {
    // Arrange
    let rate_limiter = RateLimiter::new(RateLimitConfig {
        max_tokens: 5,
        refill_interval: Duration::from_secs(60),
    });
    let webid = WebID::new();

    // Act: Make 6 requests (5 should pass, 6th should fail)
    let mut results = Vec::new();
    for _ in 0..6 {
        results.push(rate_limiter.check(&webid));
    }

    // Assert
    assert!(results[..5].iter().all(|&r| r));
    assert!(!results[5]);
}
```

**Test 3: Circuit Breaker**

```rust
#[tokio::test]
async fn test_circuit_breaker_opens_after_failures() {
    // Arrange
    let mut circuit_breaker = CircuitBreaker::new(
        5,  // failure_threshold
        Duration::from_secs(60),  // timeout
    );
    let mock_server = MockOkapiServer::failing().await;

    // Act: Simulate 5 failures
    for _ in 0..5 {
        circuit_breaker.record_failure();
    }

    // Assert: Circuit should be open
    assert!(matches!(circuit_breaker.state(), CircuitState::Open));
    
    // Act: Try to execute (should fail fast)
    let result = circuit_breaker.execute(async { mock_server.generate().await });
    
    // Assert
    assert!(matches!(result, Err(CircuitBreakerError::CircuitOpen)));
}
```

**Test 4: CNS Span Emission**

```rust
#[tokio::test]
async fn test_cns_spans_emitted_for_inference() {
    // Arrange
    let span_collector = SpanCollector::new();
    let cns_integration = CnsIntegration::with_collector(span_collector.clone());
    let mock_server = MockOkapiServer::start().await;

    // Act
    let response = mock_server.generate().await;
    cns_integration.emit_tool_span("inference", true, json!({
        "model": response.model,
        "tokens": response.usage.total_tokens,
    }));

    // Assert
    let spans = span_collector.get_spans().await;
    assert!(spans.iter().any(|s| s.category == "cns.tool"));
    assert!(spans.iter().any(|s| s.category == "cns.connector.llm"));
}
```

### Files to Create

| File | Purpose | LOC Estimate |
|------|---------|--------------|
| `hkask-testing/integration-tests/inference_pipeline.rs` | E2E tests | ~200 |
| `hkask-testing/integration-tests/rate_limiting.rs` | Rate limit tests | ~100 |
| `hkask-testing/integration-tests/circuit_breaker.rs` | Circuit breaker tests | ~150 |
| `hkask-testing/integration-tests/multi_okapi.rs` | Failover tests | ~150 |
| `hkask-testing/integration-tests/prompt_cache.rs` | Cache tests | ~100 |
| `hkask-testing/integration-tests/cns_spans.rs` | CNS span tests | ~150 |
| `hkask-testing/integration-tests/mod.rs` | Module exports | ~20 |

**Total New LOC:** ~870 (excluded from 30k budget — test crate)

### Acceptance Criteria

- [ ] All 6 integration test files created
- [ ] `cargo test -p hkask-testing` passes (6+ new tests)
- [ ] Tests run in parallel without interference
- [ ] Mock server properly simulates success/failure scenarios
- [ ] CNS span collector captures all emitted spans
- [ ] Tests documented with `///` comments

---

## P1-01: Phase 4 Production Documentation

### Problem Statement

Production documentation is incomplete for deployment and operations.

### Required Documents

| Document | Purpose | Location | Priority |
|----------|---------|----------|----------|
| **Deployment Guide** | How to deploy hKask to production | `docs/DEPLOYMENT.md` | P1 |
| **Operations Runbook** | Day-2 operations, monitoring, alerts | `docs/operations/RUNBOOK.md` | P1 |
| **Performance Tuning** | Configuration for optimal performance | `docs/PERFORMANCE.md` | P2 |
| **Security Hardening** | Production security checklist | `docs/SECURITY-HARDENING.md` | P1 |
| **Backup & Recovery** | Database backup and restore procedures | `docs/operations/BACKUP-RECOVERY.md` | P1 |

### Document Templates

**Deployment Guide Structure:**

```markdown
# hKask Deployment Guide

## Prerequisites
- Rust 1.85+
- SQLite 3.40+
- Okapi instance (optional)

## Quick Start
```bash
cargo install --path crates/hkask-cli
kask --version
```

## Production Deployment

### 1. Environment Variables
| Variable | Description | Default |
|----------|-------------|---------|
| `OKAPI_BASE_URL` | Okapi API endpoint | `http://localhost:8080` |
| `OKAPI_API_KEY` | Okapi API key | — |
| `HKASK_DATABASE_URL` | SQLite database path | `./hkask.db` |
| `HKASK_CAPABILITY_SECRET` | 32-byte hex secret | — |

### 2. Build for Production
```bash
cargo build --release -p hkask-cli
cargo build --release -p hkask-api
```

### 3. Configure Systemd (Linux)
[service file content]

### 4. Health Checks
- `GET /api/cns/health` — CNS health status
- `GET /api/sovereignty/status` — User sovereignty status

## Troubleshooting
[Common issues and solutions]
```

**Operations Runbook Structure:**

```markdown
# hKask Operations Runbook

## Monitoring

### Key Metrics
| Metric | Alert Threshold | Action |
|--------|-----------------|--------|
| CNS variety deficit | >100 | Investigate tool usage |
| Algedonic alerts | >5/hour | Escalate to on-call |
| API latency p99 | >500ms | Scale horizontally |
| Database size | >10GB | Archive old data |

### CNS Health Dashboard
[Dashboard configuration]

## Incident Response

### P1: CNS Variety Deficit Critical
1. Check `kask cns variety`
2. Review tool invocation logs
3. Identify missing tool categories
4. Enable additional MCP servers if needed

### P2: Algedonic Alert Storm
1. Check `kask cns alerts`
2. Identify root cause domain
3. Adjust thresholds if needed
4. Escalate to architecture team

## Maintenance Windows

### Weekly
- Review CNS variety counters
- Archive completed goals
- Rotate capability secrets

### Monthly
- Database vacuum
- Log rotation
- Security audit
```

### Files to Create

| File | LOC Estimate | Priority |
|------|--------------|----------|
| `docs/DEPLOYMENT.md` | ~300 | P1 |
| `docs/operations/RUNBOOK.md` | ~400 | P1 |
| `docs/SECURITY-HARDENING.md` | ~250 | P1 |
| `docs/operations/BACKUP-RECOVERY.md` | ~200 | P1 |
| `docs/PERFORMANCE.md` | ~200 | P2 |

**Total New LOC:** ~1,350 (documentation budget: 10,000 lines)

### Acceptance Criteria

- [ ] All 5 documents created with complete content
- [ ] Documents follow `DOCUMENTATION_STANDARDS.md`
- [ ] Metadata headers present (Version, Last-Updated, Audience)
- [ ] Code examples tested and working
- [ ] Cross-references to architecture docs verified
- [ ] Documents archived after superseded

---

## Execution Timeline

### Week 1 (2026-05-23 to 2026-05-29)

| Day | Task | Owner | Deliverable |
|-----|------|-------|-------------|
| **Day 1** | P0-01: goals.rs trait fixes | Storage bot | Compiling `hkask-storage` |
| **Day 2** | P0-01: Test updates | Storage bot | 8 tests passing |
| **Day 3** | P0-02: Mock server enhancements | Testing bot | Enhanced `mock_okapi.rs` |
| **Day 4** | P0-02: E2E inference tests | Testing bot | `inference_pipeline.rs` |
| **Day 5** | P0-02: Rate limit/circuit breaker tests | Testing bot | 2 test files |

### Week 2 (2026-05-30 to 2026-06-05)

| Day | Task | Owner | Deliverable |
|-----|------|-------|-------------|
| **Day 6** | P0-02: Multi-Okapi/cache tests | Testing bot | 2 test files |
| **Day 7** | P0-02: CNS span tests | Testing bot | `cns_spans.rs` |
| **Day 8** | P0-02: Integration test verification | Testing bot | All tests passing |
| **Day 9** | P1-01: Deployment guide | Curator | `DEPLOYMENT.md` |
| **Day 10** | P1-01: Operations runbook | Curator | `RUNBOOK.md` |

### Week 3 (2026-06-06 to 2026-06-12)

| Day | Task | Owner | Deliverable |
|-----|------|-------|-------------|
| **Day 11** | P1-01: Security hardening guide | Security bot | `SECURITY-HARDENING.md` |
| **Day 12** | P1-01: Backup/recovery procedures | DevOps bot | `BACKUP-RECOVERY.md` |
| **Day 13** | P1-01: Performance tuning guide | Performance bot | `PERFORMANCE.md` |
| **Day 14** | Documentation review | Curator | All docs verified |
| **Day 15** | MVP completion verification | Architect | v0.21.0 release candidate |

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Trait fix breaks existing tests | Low | Medium | Run tests after each change |
| Mock server doesn't match Okapi API | Medium | High | Use OpenAPI spec for mock generation |
| Integration tests flaky | Medium | Medium | Add retry logic, isolate test state |
| Documentation becomes stale | High | Low | Add last-reviewed date, schedule quarterly review |

---

## Success Criteria

**MVP Complete when:**

1. ✅ `cargo check --workspace` passes
2. ✅ `cargo test --workspace` passes (40+ tests)
3. ✅ `cargo clippy --workspace -- -D warnings` passes
4. ✅ LOC ≤ 30,000 (excluding tests and docs)
5. ✅ Documentation ≤ 10,000 lines (working docs only)
6. ✅ All P0 tasks complete
7. ✅ All P1 tasks complete or deferred with justification

---

## References

- [`PROJECT_STATUS.md`](status/PROJECT_STATUS.md) — Current project status
- [`DOCUMENTATION_STANDARDS.md`](standards/DOCUMENTATION_STANDARDS.md) — Documentation standards
- [`AGENTS.md`](../../AGENTS.md) — Agent operating guide
- [`hKask-architecture-master.md`](architecture/hKask-architecture-master.md) — Architecture specification

---

*Plan created 2026-05-23. Next review: 2026-05-30.*

**ℏKask — Planck's Constant of Agent Systems**
