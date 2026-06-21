---
title: "Bug Hunter's Guide"
version: "0.30.0"
date: 2026-06-21
status: "Active"
domain: "Quality Assurance"
anchored_to:
  - "docs/research/bug-hunting-as-autopoietic-skill-unified.md"
  - "docs/research/bug-hunting-skill-corrected-design.md"
principles: [P1, P2, P5, P8, P9, P12]
---

# Bug Hunter's Guide

**A practical field manual for bug hunting expeditions in the hKask codebase.**

Grounded in two real expeditions against `hkask-wallet` (7 findings) and `hkask-cns` (6 findings). Each pattern listed below found actual bugs.

---

## 1. The Mindset

### 1.1 Weinberg's Lens

> Quality is value to some person who matters.

Before you start: **whose values are at stake?** A bug is not an objective property of code. It's a mismatch between behavior and someone's values. State the quality criteria explicitly before probing.

```
Wallet expedition: "All wallet operations preserve financial invariants —
no double-spend, no negative balances, all deposits credited, all withdrawals
debited correctly, encumbrance conservation."

CNS expedition: "No silent budget overruns, no undetected alert failures,
correct hold-settle enforcement."
```

### 1.2 Beizer's Taxonomy

Every bug fits into one of eight categories. Use this to guide where you look:

| Category | What to hunt for | Wallet example | CNS example |
|----------|-----------------|---------------|-------------|
| **requirements** | Missing or wrong specification | — | — |
| **structural** | Control flow, data flow, ownership errors | — | — |
| **data** | Types, boundaries, initialization, persistence | `balance_after: 0` (missing runtime value) | Circuit breaker unknown state |
| **coding** | Implementation logic, error handling, dead code | `let _ = reserved` (dead parameter) | — |
| **interface** | API contracts, parameter validation | Debit non-idempotent (missing contract) | `settle` doesn't check `actual ≤ reserved` |
| **integration** | Component interaction, ordering | — | — |
| **timing** | Concurrency, race conditions, TOCTOU | — | `WalletBackedBudget::reserve` TOCTOU |
| **configuration** | Environment, feature flags, deployment | — | — |

### 1.3 The Three Bug Tiers

Not every finding is a confirmed bug. Classify honestly:

| Tier | Confidence | Criteria |
|------|-----------|----------|
| **BUG** | 0.90–1.00 | Behavior violates a contract, invariant, or documented constraint |
| **POTENTIAL BUG** | 0.60–0.89 | Behavior matches a known pattern but no contract exists to confirm |
| **OBSERVATION** | <0.60 | Suspect behavior, no contract, flag as contract gap |

---

## 2. The Hunt Pattern

### 2.1 Expedition Structure

Every expedition follows five phases:

```
CHARTER → PROBE → ORACLE → TAXONOMIZE → REPORT
```

**Charter:** "Explore [target] using [strategy] to discover [quality threat]." Be specific. A charter like "look for bugs" finds nothing. A charter like "trace the credit→debit→settle path checking hold-settle enforcement at each step" finds BUG-1.

**Probe:** Read code. Search for patterns. Run tests. Do NOT just list patterns — actually use tools. Read real files. Run real commands.

**Oracle:** For each observation, ask: "Is this a bug?" Use the three-tier classification. Challenge your own verdicts (grill-me pattern).

**Taxonomize:** Classify into Beizer category. Assign severity. Generate pattern signature.

**Report:** Structured JSON with locations, evidence, and fix recommendations.

### 2.2 Probe Strategies (Hendrickson Tours)

| Tour | What to do | When to use |
|------|-----------|------------|
| **Money Trail Tour** | Follow the data flow: where does value enter, where does it leave, what checks exist at each boundary | Financial code, energy budgets, any value-bearing subsystem |
| **Contract Gap Tour** | List every public function. Check which have contracts (REQ tags, expect: annotations). Probe the uncontracted ones. | Any public API surface |
| **Hold-Settle Tour** | Find every reserve/allocate/hold pattern. Check that settle always verifies actual ≤ reserved. | Energy budgets, encumbrance systems, resource management |
| **Idempotency Tour** | For every state-changing operation: can it be called twice? What happens? Is the result correct? | Storage operations, payment operations, state transitions |
| **Fail-Safe Tour** | Find every default/fallback/unknown-handler path. Does it fail open or fail closed? Which is correct? | Circuit breakers, error handlers, state machines |
| **Audit Trail Tour** | Follow every transaction record. Is the balance recorded? Can you reconstruct the ledger? | Financial code, state changes, anything that needs traceability |

### 2.3 Bug Pattern Catalog

Patterns that actually found bugs in hKask. Search for these systematically:

#### PATTERN: Dead Parameter

```rust
// RED FLAG: parameter accepted but never used
pub fn settle(..., reserved: X, actual: X) {
    do_debit(actual);
    let _ = reserved;  // ← FIND THIS
}
```

**What it means:** The function accepts a parameter that should constrain behavior but doesn't use it. The constraint exists in the API signature but not in the implementation.

**Found in:** `WalletManager::settle_rjoules` — `reserved` parameter discarded, hold-settle broken.

#### PATTERN: Hardcoded Sentinel

```rust
// RED FLAG: zero or sentinel value where runtime data should go
record_transaction(&Transaction {
    balance_after: 0,  // ← FIND THIS
});
```

**What it means:** A field that should carry runtime state is hardcoded. The contract says "record the balance" but the implementation records 0.

**Found in:** `WalletManager::shield_assets` — `balance_after: 0` breaks audit trail.

#### PATTERN: Documented Gap

```rust
// RED FLAG: the test name IS the bug report
#[test]
fn credit_rjoules_is_not_idempotent_documents_gap() {
    // Test explicitly documents that duplicate credits double the balance
}
```

**What it means:** The developers know about the gap and documented it as a test. The test name describes the missing behavior. These are pre-triaged bugs waiting for implementation.

**Found in:** `wallet_store.rs` — credit and debit idempotency gaps.

#### PATTERN: Implicit Contract Violation

```rust
// RED FLAG: public API accepts two values but only enforces one constraint
pub fn settle(reserved: X, actual: X) {
    // Checks actual <= remaining ✓
    // Does NOT check actual <= reserved ✗
}
```

**What it means:** The function signature implies a constraint (`actual` should relate to `reserved`) but only partially enforces it. The API is under-constrained.

**Found in:** `EnergyBudget::settle` — checks `actual ≤ remaining` but not `actual ≤ reserved`.

#### PATTERN: Fail-Open Default

```rust
// RED FLAG: unknown/unexpected states default to permissive
match state {
    0 => Closed,
    1 => Open,
    2 => HalfOpen,
    _ => Closed,  // ← FIND THIS: unknown → allow all
}
```

**What it means:** When the system encounters an unexpected state, it defaults to the most permissive option. In safety-critical code, the default should be restrictive.

**Found in:** `CircuitBreaker::state()` — unknown state defaulted to `Closed` (allow all requests).

#### PATTERN: TOCTOU Without Documentation

```rust
// YELLOW FLAG: check and use are separated in time
pub fn reserve(&self, amount: X) {
    if !self.can_proceed(amount) { return Err(...); }
    // ... time passes ...
    Ok(amount)  // reservation is optimistic, settle() may fail
}
```

**What it means:** The check happens at time T1, the use happens at time T2, and the state can change between them. Acceptable if documented; dangerous if not.

**Found in:** `WalletBackedBudget::reserve` — TOCTOU between reserve and settle.

---

## 3. The Report Format

Every finding follows this structure:

```json
{
  "id": "BUG-001",
  "verdict": "BUG | POTENTIAL_BUG | OBSERVATION",
  "confidence": 0.92,
  "location": {
    "file": "crates/hkask-wallet/src/manager/budget.rs",
    "function": "settle_rjoules",
    "line": 82
  },
  "beizer_category": "coding",
  "severity": "CRITICAL | HIGH | MEDIUM | LOW",
  "summary": "One sentence describing the bug and its impact",
  "evidence": "Code snippet showing the issue",
  "pattern_signature": "How to detect this class of bug elsewhere",
  "fix_suggestion": "Specific code change or 'needs investigation'"
}
```

### Severity Guidelines

| Severity | Criteria |
|----------|----------|
| **CRITICAL** | Data loss, security breach, funds at risk, system-wide failure |
| **HIGH** | Feature broken, invariant violated, no workaround |
| **MEDIUM** | Workaround exists, edge case, audit/compliance issue |
| **LOW** | Cosmetic, documentation, unlikely to trigger |

---

## 4. Expedition Checklist

Before starting:
- [ ] State quality criteria (Weinberg: value to which person?)
- [ ] Choose a charter (Hendrickson format: "Explore X using Y to discover Z")
- [ ] Identify the Beizer categories most likely to contain bugs

During the hunt:
- [ ] Read the module's public API and contracts first
- [ ] Follow data flow end-to-end (credit → debit → settle)
- [ ] Search for each pattern from §2.3 systematically
- [ ] Run `cargo check`, `cargo test`, `cargo clippy` where applicable
- [ ] For every finding, challenge your own verdict (grill-me)

After the hunt:
- [ ] Classify every finding into Beizer taxonomy
- [ ] Assign severity per the guidelines
- [ ] Generate pattern signatures for each bug class
- [ ] Produce structured JSON report
- [ ] Recommend fixes with specific code changes

---

## 5. What We Learned from Two Expeditions

### 5.1 Effective Strategies

| Strategy | Yield | Example |
|----------|-------|---------|
| **Follow the money** | 3 bugs (HIGH, HIGH, MEDIUM) | Traced `credit → debit → settle` path, found broken hold-settle and idempotency gap |
| **Check contract enforcement** | 2 bugs (MEDIUM, MEDIUM) | Found `settle` API under-constrained and `shield_assets` recording wrong balance |
| **Search for documented gaps** | 1 bug (HIGH) | Found explicitly-named tests documenting known idempotency gaps |
| **Audit default behaviors** | 1 fix (MEDIUM) | Found circuit breaker fail-open on unknown states |

### 5.2 What Didn't Find Bugs (but was worth checking)

- **Unsafe blocks:** Wallet signing module was well-designed with Zeroizing and per-operation key loading
- **Atomic operations:** Encumbrance consume was correct with SQL-level atomicity
- **State machines:** Circuit breaker Closed→Open→HalfOpen→Closed transitions were correct
- **CNS span coverage:** All critical operations had CNS span emission

### 5.3 Bug Density by Beizer Category

| Category | Findings | Notes |
|----------|----------|-------|
| **coding** | 2 | Dead parameters, incomplete implementation |
| **data** | 2 | Missing runtime values, sentinel defaults |
| **interface** | 3 | Missing contracts, under-constrained APIs, non-idempotent operations |
| **timing** | 1 | TOCTOU window (documented, not exploited) |
| **requirements** | 0 | — |
| **structural** | 0 | — |
| **integration** | 0 | — |
| **configuration** | 0 | — |

**Pattern:** Interface and data bugs were most common. The code was structurally sound (correct state machines, atomic operations) but had gaps at API boundaries (missing validation, under-constrained parameters, non-idempotent operations).

---

## 6. Skill Composition

This guide is designed to be used with hKask's skill ecosystem:

| Skill | How to use during a bug hunt |
|-------|------------------------------|
| **pragmatic-semantics** | Classify each finding: IS vs OUGHT, declarative vs subjunctive. Never present speculation as fact. |
| **grill-me** | Challenge every HIGH-confidence verdict. "Could this be intentional? Edge case where it's correct?" |
| **diagnose** | For POTENTIAL_BUG findings: reproduce, hypothesize, isolate before confirming. |
| **essentialist** | After the expedition: which probe strategies found nothing? Deprecate them. Which found bugs? Strengthen them. |
| **coding-guidelines** | Fix recommendations must be surgical — touch only what must change. |
| **deep-module** | Audit fix complexity: is the fix deeper (small interface, much behavior) or shallow (thin delegation)? |

---

## 7. References

- Weinberg, G. M. (1992). *Quality Software Management, Volume 1: Systems Thinking*. Dorset House.
- Beizer, B. (1990). *Software Testing Techniques* (2nd ed.). Van Nostrand Reinhold.
- Bach, J. (2015). Heuristic Test Strategy Model. Satisfice, Inc.
- Hendrickson, E. (2013). *Explore It!: Reduce Risk and Increase Confidence with Exploratory Testing*. Pragmatic Bookshelf.
- Agans, D. J. (2002). *Debugging: The Nine Indispensable Rules*. AMACOM.
- Zeller, A. (2009). *Why Programs Fail: A Guide to Systematic Debugging* (2nd ed.). Morgan Kaufmann.
