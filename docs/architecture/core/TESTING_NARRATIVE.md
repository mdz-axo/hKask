---
title: "Testing Discipline — Narrative Companion"
audience: [developers, agents]
last_updated: 2026-06-17
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# Testing Discipline — Narrative Companion

Welcome. If you're new to hKask, this document is for you. It tells the story of how we test code here — not with bullet-point checklists or dry specification tables, but as an actual explanation of why things work the way they do.

By the time you finish reading, you should understand the architecture of hKask's testing program well enough to write your first contract-anchored test. For the formal rules and reference tables, see [`TESTING_DISCIPLINE.md`](TESTING_DISCIPLINE.md) — but read this first.

---

## 1. The Architecture in Narrative Form

### 1.1 Where It All Starts: The Functional Spec

Every feature in hKask begins life as a functional specification. These specs follow the **Minimal Domain Specification** (MDS) methodology — a capability-driven framework where specifications are *grants* ("this agent CAN verify sovereignty via this interface"), not *fences* ("MUST NOT do X").

When you run `spec/goal/capture` through the MCP spec server, you get back a goal with a set of criteria. Something like:

> "The system shall verify sovereignty for any valid WebID"

That's the **OUGHT** — the statement of intent. But intent doesn't build software. We need to bridge from OUGHT to IS.

### 1.2 The Bridge: `// REQ:` Contracts

This is where contracts come in. Every OUGHT statement gets translated into one or more `// REQ:` behavioral contracts — explicit preconditions, postconditions, and invariants written directly in the source code above every public function.

A contract answers three questions:

- **Precondition:** What must the caller guarantee before calling this function?
- **Postcondition:** What does the function guarantee to return?
- **Invariant:** What must always remain true across all operations on this type?

In code, it looks like this:

```rust
/// REQ: sovereignty-verify-001
/// pre:  webid is a valid, non-nil WebID
/// post: returns Ok(sovereignty_state) where state.webid == webid
///       OR returns Err(NotFound) if webid has no sovereignty record
/// inv:  does not modify any stored state (read-only)
pub fn verify_sovereignty(webid: &WebID) -> Result<SovereigntyState, SovereigntyError> {
    // ...
}
```

This contract is simultaneously three things:
1. **Documentation** — anyone reading the code knows exactly what the function does
2. **Specification** — there is no separate "spec document" for this function's behavior; the contract IS the specification
3. **Test Oracle** — the contract tells us what the tests should verify

This is not a hKask invention. It's Design by Contract, formulated by Bertrand Meyer in 1986 and battle-tested across four decades in Eiffel, Ada 2012, SPARK, and Racket. We've just applied it to a Rust codebase with agent co-developers.

### 1.3 From Contract to Property-Based Test

A contract says *what* must be true. But how do we know it IS true, for all possible inputs, not just the examples we thought to test?

Enter **property-based testing** (PBT). Instead of writing individual test cases with hardcoded values, you write a *property* — an invariant — and the test framework generates thousands of random inputs, checks the property against each one, and shrinks any failures to the smallest counterexample.

```rust
// REQ: condenser-idempotency-001
// pre:  input is any non-empty string
// post: compress(compress(input)) == compress(input) for all inputs
proptest! {
    #[test]
    fn compression_is_idempotent(input in any::<String>()) {
        prop_assume!(!input.is_empty());  // enforce precondition
        let once = compress(&input);
        let twice = compress(&once);
        prop_assert_eq!(once, twice);     // verify postcondition
    }
}
```

Notice the two critical operations:
- `prop_assume!` enforces the **precondition** — inputs that don't satisfy it are silently skipped
- `prop_assert_eq!` verifies the **postcondition** — if it fails for any generated input, the test framework shrinks to the minimal counterexample

This pipeline — Contract → Proptest — is the heartbeat of hKask's testing program.

### 1.4 The Contract-Completeness Invariant

Here is the central invariant of the hKask codebase:

> **Every `pub fn` must carry at least one `// REQ: pre:` contract.**

This is not aspirational. It is the actual state of the codebase. As of today:

| Metric | Value |
|--------|-------|
| Public functions | 1,775 |
| REQ: contract tags | 2,274 |
| Coverage percentage | 128.1% |
| Crates at ≥100% | 21/21 |

The coverage exceeds 100% because some functions carry multiple contracts (builder methods, functions with multiple distinct behaviors), and struct fields also carry REQ tags for type-level invariants.

This invariant is enforced by `scripts/contract-audit.sh` — a straightforward shell script that counts `pub fn` declarations and `// REQ:` tags across every crate, computes percentage coverage, and flags any uncontracted functions as candidates for remediation. It runs as a trend monitor, not a hard build gate — the goal is observability, not punishment. But the expectation is clear: **100% is the floor, not the ceiling**.

The audit script provides multiple output modes:
- `--summary` for a dashboard view across all crates
- `--json` for machine-readable output (used by MCP tools and CNS)
- Per-crate detail mode to list specific uncontracted functions
- `--csv` for spreadsheet import

When a new developer opens a PR that adds a `pub fn` without a contract, the audit catches it before merge.

### 1.5 A Concrete Example: `WalletBalance`

Let's walk through a type-level contract, because structs need contracts too:

```rust
/// REQ: wallet-balance-001
/// inv: balance_rj >= 0 (balances are never negative)
/// inv: balance_rj + sum(encumbrances) <= original_deposit_total
pub struct WalletBalance {
    pub balance_rj: u64,
    // ...
}
```

These invariants are cross-operation guarantees. Every function that mutates `WalletBalance` must preserve both invariants on exit. A property-based test for this doesn't test one function — it generates random *sequences* of wallet operations (deposit, withdraw, convert, encumber) and asserts that after every operation, the invariants still hold.

---

## 2. The Red-Green-Refactor Loop in hKask

If you've done TDD before, you know the loop: Red → Green → Refactor. hKask uses the same cycle, but with contracts inserted at the front.

### 2.1 RED: Write the Contract and Test

Step one is not writing a test. It's writing the contract:

```rust
/// REQ: my-feature-001
/// pre:  input is a non-empty string with valid UTF-8
/// post: returns the reversed string; length is preserved
pub fn reverse(input: &str) -> String {
    todo!()  // not implemented yet, but the contract IS the specification
}
```

Then write the property-based test anchored to that contract:

```rust
// REQ: my-feature-001
proptest! {
    #[test]
    fn reverse_preserves_length(input in "[a-zA-Z0-9]+") {
        let result = reverse(&input);
        prop_assert_eq!(input.len(), result.len());
    }

    #[test]
    fn reverse_is_its_own_inverse(input in "[a-zA-Z0-9]+") {
        let result = reverse(&reverse(&input));
        prop_assert_eq!(input, result);
    }
}
```

Run the tests. They fail — because `todo!()` is a placeholder. This is the RED phase. But it's not just "the test fails" — it's "the test fails *because the contract has not been satisfied*." The contract is the missing piece.

### 2.2 GREEN: Implement Minimal Code

Now write just enough code to satisfy the contract:

```rust
pub fn reverse(input: &str) -> String {
    input.chars().rev().collect()
}
```

Run the tests. They pass — for all thousands of generated inputs, not just the one you would have written by hand. This is GREEN. The contract is satisfied.

### 2.3 REFACTOR: Improve While the Contract Holds

Now you can improve the implementation. Maybe `chars().rev().collect()` allocates unnecessarily for ASCII-only inputs. Maybe a `Vec<u8>` reversal with unsafe would be faster for known-ASCII. Doesn't matter — as long as the contract holds, the tests pass.

During refactoring, you can also improve the contract itself:

- **Weaken the precondition** to accept more inputs
- **Strengthen the postcondition** to guarantee more about the output
- **Add invariants** for cross-operation guarantees

The key insight: contracts make refactoring fearless. If the tests pass, the contract holds. If the contract holds, the behavior is correct. You can rewrite the entire internals of a module and know — with mathematical certainty, not just hope — that you haven't broken anything the contract promised.

### 2.4 Vertical Slicing, Not Horizontal

One critical rule: never write multiple tests before implementing. Each RED→GREEN is a single vertical slice through the system:

```
RIGHT:  Contract-1 → Test-1 → Impl-1  (vertical slice, complete)
        Contract-2 → Test-2 → Impl-2
        Contract-3 → Test-3 → Impl-3

WRONG:  Contract-1, Contract-2, Contract-3  (horizontal, speculative)
        Test-1, Test-2, Test-3
        Impl-1, Impl-2, Impl-3
```

Horizontal slicing produces tests that verify *imagined* behavior. Vertical slicing produces tests that verify *actual* behavior. The difference matters.

---

## 3. CNS, Kata, and Kanban Integration

The testing program doesn't exist in isolation. It's woven into hKask's three self-regulating systems.

### 3.1 CNS Observes Test Runs

hKask's **Cybernetic Nervous System** (CNS) watches everything. When tests execute — whether locally during development, in CI, or on the background contract monitor — they emit CNS spans that feed into the system's homeostatic regulation.

Five spans are specific to contract discipline:

| Span | When It Fires | What It Means |
|------|-------------|---------------|
| `ContractCoverage` | Background monitor runs (every 30 min) | Tracks the variety of contracted functions per domain. A drop in coverage triggers an algedonic alert. |
| `ContractViolated` | A contract test fails | An algedonic signal — something the system promised is now broken. Triggered by `cargo test` failures on contract tests. |
| `ContractProposed` | A replicant opens a PR with a new contract | An agent has volunteered a behavioral specification for its own actions. |
| `ContractAccepted` | A human merges the PR | Affirmative consent (P2) has been given. The contract is now part of the regulatory model. |
| `ContractRejected` | A human closes the PR without merge | The contract was examined and found wanting. Rationale is documented as a curation decision. |

These spans feed into the CNS health dashboard (`kask cns health`), which tracks overall system state including contract discipline as a regulatory metric.

The background contract monitor runs `cargo test` on priority crates every 60 minutes. When it detects a contract violation, it doesn't just log it — it creates a kanban task labeled "Contract Violations" so the breach can't be ignored or forgotten. Every contract violation is assigned an owner (the replicant or developer who last touched the function, tracked by `git blame`), enforcing P12's rule that every action has an accountable host.

### 3.2 Kata Coaches Improvement

The **Toyota Kata** system in hKask uses scientific thinking to drive continuous improvement. The kata-improvement skill frames capability gaps as structured experiments.

When the contract audit reveals test debt — uncontracted functions, functions with weak postconditions, or failing contract tests — that debt becomes a kata experiment:

1. **Understand the Direction:** "All pub fns must have contracts at 100% coverage"
2. **Grasp the Current Condition:** "`hkask-wallet` has 57 pub fns, 104 contracts — but 3 functions have weak postconditions"
3. **Establish the Target Condition:** "Strengthen postconditions on wallet withdraw/convert operations to exclude underflow bugs"
4. **PDCA Cycle:**
   - **Plan:** Write the strengthened contract, write the proptest, predict it will catch the underflow edge case
   - **Do:** Implement the contract and test
   - **Check:** Run contract-audit.sh, run the test suite, verify the postcondition holds for generated inputs
   - **Act:** Merge if passing; if failing, the contract caught a real bug — fix the implementation

The PDCA cycle maps directly onto contract writing. A contract is a hypothesis ("this function, when given X, will return Y"). The proptest is the experiment that tests the hypothesis across thousands of inputs. The result either confirms the hypothesis (test passes) or falsifies it (test fails with a shrunk counterexample).

### 3.3 Kanban Tracks Test Debt

Uncontracted functions don't hide in the shadows — they appear as kanban tasks on the contract discipline board.

When `contract-audit.sh` finds a `pub fn` without a contract, the system can generate a task:
- **Title:** "Add contract to `hkask-wallet::shield_balance`"
- **Description:** The function signature and a template contract to fill in
- **Acceptance Criteria:** The contract audit must show ≥100% coverage with the new contract
- **Owner:** Assigned with consent (P2)

Contract writing is a *verifiable* task with clear pre/post conditions. The acceptance criteria are machine-checkable: run the audit, check the coverage. There's no ambiguity about whether the task is "done."

The full loop:

```
contract-audit.sh → detects gap → kanban task created (TaskCreated span)
    → replicant proposes contract (ContractProposed span)
    → human reviews and consents (ContractAccepted span)
    → coverage updated (ContractCoverage span)
    → task verified and closed (TaskVerified span)
```

Every step is observable. Every decision is attributable. Nothing falls through the cracks.

---

## 4. The Contract-Audit Pipeline

Let's trace the complete path of a single requirement from spec to CNS span, using `verify_sovereignty` as our running example.

```
Functional Spec ──→ // REQ: contract ──→ proptest ──→ contract-audit.sh ──→ coverage report ──→ CNS span
```

### Stage 1: Functional Spec

A sovereignty verification requirement enters through MDS:

```
spec/goal/capture {
    description: "Verify WebID sovereignty state",
    context: "P1 User Sovereignty enforcement"
}
→ goal_id: "sovereignty-verify"
→ criteria: ["system shall verify sovereignty for any valid WebID"]
```

### Stage 2: Contract

The spec criterion generates a `// REQ:` contract on the function:

```rust
/// REQ: sovereignty-verify-001
/// pre:  webid is a valid, non-nil WebID
/// post: returns Ok(sovereignty_state) where state.webid == webid
///       OR returns Err(NotFound) if webid has no sovereignty record
/// inv:  does not modify any stored state (read-only)
pub fn verify_sovereignty(webid: &WebID) -> Result<SovereigntyState, SovereigntyError> {
```

The `REQ: sovereignty-verify-001` tag is the traceability link back to the spec. Anyone can grep for that tag and find both the spec requirement and the contract. The contract IS the specification for this function — there is no separate document to drift out of sync.

### Stage 3: Property-Based Test

The contract's preconditions and postconditions are translated directly into a proptest:

```rust
// REQ: sovereignty-verify-001
proptest! {
    #[test]
    fn verify_sovereignty_returns_state_for_valid_webid(
        webid in any_valid_webid()
    ) {
        let result = verify_sovereignty(&webid);
        prop_assert!(result.is_ok());
        if let Ok(state) = result {
            prop_assert_eq!(state.webid, webid);
        }
    }

    #[test]
    fn verify_sovereignty_returns_not_found_for_unknown(
        webid in any_non_existent_webid()
    ) {
        let result = verify_sovereignty(&webid);
        prop_assert!(matches!(result, Err(SovereigntyError::NotFound)));
    }
}
```

Proptest generates thousands of valid and non-existent WebIDs. For each one, it verifies that the function either returns the correct state or returns `NotFound`. If any input produces a different result, proptest shrinks it to the minimal failing WebID and reports: "Here is exactly the input that breaks your contract."

### Stage 4: Contract Audit

`scripts/contract-audit.sh` scans the entire codebase. It greps for `pub fn` and `// REQ:` patterns, computes per-crate coverage, and produces a report. Because `verify_sovereignty` carries its `REQ:` tag, it contributes +1 to the contracted count for its crate.

If someone were to add a new public function *without* a contract, the audit would flag it:

```
── Uncontracted public functions ──
  crates/hkask-types/src/new_module.rs L42 pub fn do_stuff(...)
```

### Stage 5: Coverage Report

The summary output shows the health of every crate:

```
Crate                           Pub Fns Contracted Coverage %
------------------------------ -------- ---------- ----------
hkask-types                        340        375     110.2%
...
TOTAL                              1775       2274     128.1%
```

### Stage 6: CNS Span

The audit's results are consumed by the CNS background monitor. Every 30 minutes, the monitor emits a `ContractCoverage` span with the current variety (contracted function count per domain). If coverage drops — a function loses its contract during a refactor, or a new `pub fn` is merged without one — the drop is detected, an alert fires, and a kanban task is created.

Meanwhile, if a contract test fails during `cargo test`, a `ContractViolated` span fires immediately. That span carries the function name, the failed postcondition, and the minimal counterexample that broke it. The developer (or replicant) who caused the violation is identified and assigned the fix.

---

## 5. Key Principles at Work

The testing program isn't arbitrary. Every design decision traces back to one or more of the twelve architecture principles.

### 5.1 P4 — Clear Boundaries (OCAP)

> "P1–P3 are enforced through explicit capability boundaries. No ambient authority and no admin bypass."

**Contracts ARE OCAP membranes made testable.** An OCAP boundary is a capability gate — "you may call this function only if you possess capability X." A contract is the behavioral specification of that gate:

- The **precondition** defines what capabilities the caller must possess (or what state must hold)
- The **postcondition** defines what capabilities are returned or exercised
- The **invariant** defines what must remain true across the boundary

Contract tests at crate boundaries detect **semantic drift** — when a type changes in a way that's type-compatible but behaviorally different. The Rust compiler can verify that types match. It cannot verify that semantics match. Contracts can.

### 5.2 P8 — Semantic Grounding

> "System claims must be grounded in traceable, provenance-aware representations."

**Every contract is an IS statement about behavior.** Preconditions, postconditions, and invariants are declarative claims about what the system actually does. They are not OUGHT statements ("the system should...") — they are IS statements ("the system, when called with X, returns Y").

The `// REQ:` tag traces the contract to a specification requirement:
- The **spec** is the OUGHT (intent)
- The **contract** is the IS (declared behavior)
- The **proptest** verifies that IS matches OUGHT

This three-way traceability means you can always answer the question "why does this function behave this way?" by following the chain: test → contract → spec.

### 5.3 P9 — Homeostatic Self-Regulation

> "The system must remain observable and self-correcting through cybernetic feedback loops."

**The test suite is a feedback loop.** Under the Good Regulator Theorem (Conant & Ashby, 1970), every good regulator must be a model of the system it regulates. The test suite IS that model. If the tests don't model the system's actual failure modes, they're not a good regulator — they're just noise.

This is why CNS spans are critical:

- `ContractViolated` is an **algedonic signal** — pain that demands attention
- `ContractCoverage` is a **variety counter** — ensuring the regulator (tests) has enough variety (Ashby's Law) to match the regulated system (codebase)
- `ContractProposed` / `ContractAccepted` / `ContractRejected` make the **contract lifecycle observable** — you can see when agents are improving their own behavioral specifications and when humans are consenting

---

## 6. Getting Started: Your First Contract-Anchored Test

Let's walk through the complete process of writing your first contract-anchored test in hKask. We'll write a function that computes a checksum for a byte slice.

### Step 1: Understand the Feature

You're implementing a simple checksum: given any byte slice, compute a u32 hash using a non-cryptographic algorithm. The hash must be deterministic (same input → same output) and must have good bit dispersion.

### Step 2: Write the Contract (RED)

The contract goes above the function signature — before any implementation exists:

```rust
/// REQ: checksum-compute-001
/// pre:  data is any byte slice (including empty)
/// post: returns a deterministic u32; same input always produces same output
///       checksum([]) != checksum([0]) (empty and zero-differ)
/// post: bit_flip(data, i) produces different checksum for at least 50% of bit positions
pub fn checksum(data: &[u8]) -> u32 {
    todo!()
}
```

Note: you write the contract first, then `todo!()`. The contract IS the specification. The `todo!()` is just a placeholder that says "not implemented yet" — it will be replaced in the GREEN phase.

### Step 3: Write the Proptest (RED, continued)

Now write property-based tests that verify each postcondition:

```rust
// REQ: checksum-compute-001
proptest! {
    /// Determinism: same input → same output
    #[test]
    fn checksum_is_deterministic(data in any::<Vec<u8>>()) {
        let first = checksum(&data);
        let second = checksum(&data);
        prop_assert_eq!(first, second);
    }

    /// Empty and zero-differ: empty input ≠ single zero byte
    #[test]
    fn checksum_distinguishes_empty_from_zero() {
        let empty_hash = checksum(&[]);
        let zero_hash = checksum(&[0u8]);
        prop_assert_ne!(empty_hash, zero_hash);
    }

    /// Bit dispersion: flipping any bit changes hash
    #[test]
    fn checksum_has_good_dispersion(
        data in any::<Vec<u8>>(),
        bit_pos in 0usize..8000usize
    ) {
        prop_assume!(!data.is_empty());
        let original = checksum(&data);

        // Flip one bit
        let byte_idx = (bit_pos / 8) % data.len();
        let bit_in_byte = bit_pos % 8;
        let mut mutated = data.clone();
        mutated[byte_idx] ^= 1u8 << bit_in_byte;

        let mutated_hash = checksum(&mutated);
        prop_assert_ne!(original, mutated_hash,
            "hash should differ after a single-bit change");
    }
}
```

Run `cargo test`. All three tests fail — RED. The contract is not satisfied.

### Step 4: Implement the Function (GREEN)

Now write the minimal implementation that satisfies the contract. A CRC-32 is simple and effective:

```rust
/// REQ: checksum-compute-001
/// pre:  data is any byte slice (including empty)
/// post: returns a deterministic u32; same input always produces same output
///       checksum([]) != checksum([0]) (empty and zero-differ)
/// post: bit_flip(data, i) produces different checksum for at least 50% of bit positions
pub fn checksum(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
```

Run `cargo test`. All three pass — GREEN. Proptest has verified determinism against thousands of random inputs, confirmed empty ≠ zero, and confirmed that single-bit flips change the hash.

### Step 5: Refactor (REFACTOR)

Now that the contract holds, you can improve the implementation:

- Use a lookup table for the CRC polynomial to speed up the inner loop
- Add SIMD intrinsics for larger throughput
- Add a streaming version for incremental hashing

As long as the contract holds, the refactor is safe. If you accidentally break the bit-dispersion property, the proptest catches it with a shrunk counterexample: "Here is the exact byte slice and bit position where your new implementation produces the same hash as the original."

### Step 6: Verify Coverage

Run the contract audit to confirm your new function is counted:

```bash
bash scripts/contract-audit.sh hkask-<your-crate>
```

If the coverage is ≥100%, you're done. If not, you have more contracts to write — but you know exactly which functions need them.

---

## Summary

That's the hKask testing program in narrative form. The key takeaway:

1. **Every `pub fn` has a contract.** No exceptions.
2. **Every contract has a property-based test.** Proptest verifies the contract against thousands of inputs.
3. **Every test traces to a spec.** The `// REQ:` tag links test → contract → specification.
4. **CNS watches everything.** Coverage drops and contract violations trigger observable events.
5. **Kata and kanban close the loop.** Improvement is structured, measurable, and verifiable.

Write the contract. Verify with proptest. Let CNS watch. Repeat.

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.0*
