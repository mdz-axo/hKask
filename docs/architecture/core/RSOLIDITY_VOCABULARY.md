# rSolidity Contract Vocabulary

**Status:** Design complete — awaiting macro crate implementation  
**Source anchor:** [`FUNCTIONAL_SPECIFICATION.md`](./FUNCTIONAL_SPECIFICATION.md)  
**Machine-readable manifest:** [`data/rsolidity_contract_manifest.json`](../../data/rsolidity_contract_manifest.json)  
**Version:** 0.27.0

---

## 1. Purpose

rSolidity is the runtime-contract layer for hKask. It translates the declarative `/// REQ:` contracts in Rust source into executable pre/post/invariant checks, OCAP boundary enforcement, and CNS span emission. This document defines the vocabulary and establishes the rewrite pattern from spec contracts to rSolidity macros.

The design goal is **mechanical fidelity**: every `pre:`, `post:`, and `inv:` line in a `/// REQ:` block becomes one rSolidity macro invocation. Principle annotations (`[P9] Motivating`, `[P4] Constraining`) become compile-time metadata used for span namespace selection and algedonic escalation.

---

## 2. Macro Vocabulary

| Macro | Solidity Analog | Role in hKask | Emits CNS Span |
|-------|-----------------|---------------|----------------|
| `require!(condition, "contract-id", "msg")` | `require` | **Precondition** — must hold on entry; panics/reverts if false | No |
| `assert!(condition, "contract-id", "msg")` | `assert` | **Postcondition / invariant** — must hold after execution; panics if false | No |
| `revert!("contract-id", "msg")` | `revert` | **Explicit failure path** — returns an `EnergyError` or domain error | Optional |
| `emit!(span, verb, phase, payload)` | `emit` | **CNS span emission** — Sense/Act/Decide events | Yes |
| `#[ocap(resource, operation)]` | modifier / capability check | **OCAP boundary gate** — verifies caller owns the resource before executing the annotated function | Yes |
| `#[contract(id = "...", principle = "P9", pre = "...", post = "...", inv = "...")]` | n/a | **Compile-time contract registration** — generates the `/// REQ:` doc comment and links to the manifest | No |

### 2.1 `require!`

```rust
require!(
    gas.0 <= self.available().0 || !self.hard_limit,
    "P9-cns-energy-budget-can-proceed",
    "gas exceeds available budget and hard_limit is enforced"
);
```

- Evaluates the condition.
- On failure, returns `EnergyError::BudgetExceeded` (or a principle-mapped error type).
- Does **not** emit a span by default; combine with `emit!` for algedonic feedback.

### 2.2 `assert!`

```rust
assert!(
    self.remaining.0 + self.reserved.0 <= self.cap.0,
    "P9-cns-energy-budget-invariant",
    "remaining + reserved must never exceed cap"
);
```

- Evaluates the condition after the guarded code runs.
- Panics in debug builds; logs and continues in release builds (configurable).
- Used for invariants and postconditions that cannot be enforced by construction.

### 2.3 `revert!`

```rust
if self.hard_limit && gas.0 > available.0 {
    revert!(
        "P9-cns-energy-budget-reserve",
        EnergyError::BudgetExceeded { requested: gas, remaining: available }
    );
}
```

- Returns the provided error value.
- Equivalent to `return Err(...)` but tagged with a contract ID for tracing.

### 2.4 `emit!`

```rust
emit!(
    CnsSpan::WalletWithdrawal,
    "submitted",
    Phase::Act,
    json!({ "actor": actor.to_string(), "tx_hash": tx_hash.0 })
);
```

- Emits a `NuEvent` to the configured sink.
- `phase` is `Sense | Act | Decide` from [`MDS.md`](./MDS.md).
- Optional `actor` parameter defaults to the caller’s `WebID`.

### 2.5 `#[ocap(resource, operation)]`

```rust
#[ocap(resource = "wallet_balance", operation = "debit")]
pub fn debit_rjoules(&self, wallet_id: WalletId, amount: RJoule) -> Result<WalletBalance, WalletError> {
    // ...
}
```

- Injects a capability check before the function body.
- Verifies the caller owns `resource` and is authorized for `operation`.
- On failure, returns a principle-mapped authorization error and emits `cns.auth.denied`.

### 2.6 `#[contract(...)]`

```rust
#[contract(
    id = "P9-cns-energy-budget-can-proceed",
    principle = "P9",
    pre = "gas is a valid EnergyCost",
    post = "returns true iff gas <= available OR hard_limit is false"
)]
pub fn can_proceed(&self, gas: EnergyCost) -> bool {
    // ...
}
```

- Purely compile-time metadata.
- Generates the `/// REQ:` doc comment from the spec.
- Registers the contract in the build-time manifest so `scripts/contract-audit.sh` can validate it.

---

## 3. Design Rules

### 3.1 One macro per contract clause

Every clause in a `/// REQ:` block maps to exactly one macro call:

| Spec clause | Macro |
|-------------|-------|
| `pre: condition` | `require!(condition, ...)` |
| `post: condition` | `assert!(condition, ...)` |
| `inv: condition` | `assert!(condition, ...)` at function entry and exit |
| failure path described in post | `revert!(...)` |
| CNS span described in post | `emit!(...)` |
| ownership / authorization | `#[ocap(...)]` |

### 3.2 Principle-driven error and span mapping

| Principle | Error family | Default span namespace | Algedonic |
|-----------|--------------|------------------------|-----------|
| P1 User Sovereignty | `SovereigntyError` | `cns.sovereignty.*` | Yes |
| P2 Affirmative Consent | `ConsentError` | `cns.consent.*` | Yes |
| P3 Generative Space | `StateError` | `cns.state.*` | No |
| P4 Clear Boundaries | `BoundaryError` | `cns.boundary.*` | Yes |
| P5 Essentialism | `ComplexityError` | `cns.essentialism.*` | No |
| P7 Evolutionary Architecture | `CalibrationError` | `cns.calibrate.*` | No |
| P8 Semantic Grounding | `TypeError` | `cns.semantic.*` | No |
| P9 Homeostatic Self-Regulation | `EnergyError` | `cns.energy.*` | Yes |
| P12 Subscriber Consent | `SubscriptionError` | `cns.subscription.*` | Yes |

### 3.3 Strangler fig migration

Old `/// REQ:` comments remain in source during migration. New rSolidity macros are added alongside them. A contract is considered migrated when:

1. Every `pre:` has a corresponding `require!`.
2. Every `post:` has a corresponding `assert!` or `emit!`.
3. Every `inv:` has an entry and exit `assert!`.
4. The old `/// REQ:` comment is removed only after a full release cycle with no regressions.

---

## 4. Example Rewrite

### 4.1 Source: `P9-cns-energy-budget-can-proceed`

**Current Rust contract comment:**

```rust
/// REQ: P9-cns-energy-budget-can-proceed
/// [P9] Motivating: Homeostatic Self-Regulation — the check-before-execute gateway
/// [P4] Constraining: Clear Boundaries — hard_limit enforces the boundary
/// pre:  gas is a valid EnergyCost
/// post: returns true iff gas <= available OR hard_limit is false
pub fn can_proceed(&self, gas: EnergyCost) -> bool {
    let available = self.available();
    gas.0 <= available.0 || !self.hard_limit
}
```

**rSolidity rewrite:**

```rust
use hkask_rsolidity::{contract, require, assert};

#[contract(
    id = "P9-cns-energy-budget-can-proceed",
    principle = "P9",
    pre = "gas is a valid EnergyCost",
    post = "returns true iff gas <= available OR hard_limit is false"
)]
pub fn can_proceed(&self, gas: EnergyCost) -> bool {
    require!(
        gas.0 <= self.available().0 || !self.hard_limit,
        "P9-cns-energy-budget-can-proceed",
        "postcondition: gas must fit available budget or hard_limit must be disabled"
    );
    let available = self.available();
    let result = gas.0 <= available.0 || !self.hard_limit;
    assert!(
        result == (gas.0 <= available.0 || !self.hard_limit),
        "P9-cns-energy-budget-can-proceed",
        "result must match the budget gate formula"
    );
    result
}
```

For this trivial predicate the `require!` and `assert!` are the same expression; in practice the `require!` is omitted when the function body itself is the check, and only the `#[contract]` metadata plus an `assert!` on the return value remain.

### 4.2 Source: `P9-cns-energy-budget-reserve`

**Current Rust:**

```rust
pub fn reserve(&mut self, gas: EnergyCost) -> Result<EnergyCost, EnergyError> {
    let available = self.available();
    if self.hard_limit && gas.0 > available.0 {
        return Err(EnergyError::BudgetExceeded { requested: gas, remaining: available });
    }
    self.reserved = EnergyCost(self.reserved.0.saturating_add(gas.0));
    Ok(gas)
}
```

**rSolidity rewrite:**

```rust
#[contract(
    id = "P9-cns-energy-budget-reserve",
    principle = "P9",
    pre = "gas is a valid EnergyCost",
    post = "if hard_limit && gas > available → Err(BudgetExceeded)",
    post2 = "if Ok → reserved increased by gas, remaining unchanged",
    inv = "remaining + reserved ≤ cap (maintained)"
)]
pub fn reserve(&mut self, gas: EnergyCost) -> Result<EnergyCost, EnergyError> {
    let available = self.available();
    if self.hard_limit && gas.0 > available.0 {
        revert!(
            "P9-cns-energy-budget-reserve",
            EnergyError::BudgetExceeded { requested: gas, remaining: available }
        );
    }
    self.reserved = EnergyCost(self.reserved.0.saturating_add(gas.0));
    assert!(
        self.remaining.0 + self.reserved.0 <= self.cap.0,
        "P9-cns-energy-budget-reserve",
        "invariant: remaining + reserved ≤ cap"
    );
    Ok(gas)
}
```

---

## 5. Crate Structure

Future implementation: `crates/hkask-rsolidity/`

```text
crates/hkask-rsolidity/
├── Cargo.toml
├── src/
│   ├── lib.rs          # re-exports macros and attributes
│   ├── macros/
│   │   ├── require.rs
│   │   ├── assert.rs
│   │   ├── revert.rs
│   │   └── emit.rs
│   └── attr/
│       ├── contract.rs
│       └── ocap.rs
└── tests/
    └── energy_budget.rs
```

### 5.1 Dependencies

- `proc-macro2`, `quote`, `syn` for macro authoring.
- `hkask-types` for `CnsSpan`, `Phase`, `NuEvent`, and error types.
- `serde_json` for span payloads.

### 5.2 Non-goals

- rSolidity is **not** a full EVM compiler or Solidity transpiler.
- It does not generate `.sol` files.
- It does not replace the Rust type system; it adds runtime contracts where types cannot express the constraint.

---

## 6. Migration Order

1. `hkask-cns` — smallest, most stable contracts; establishes the pattern.
2. `hkask-wallet` — financial boundaries; high-value OCAP use case.
3. `hkask-agents` — sovereignty and consent boundaries.
4. `hkask-storage` — generative-space CRUD contracts.
5. `hkask-inference`, `hkask-services`, `hkask-api`, etc. — follow as needed.

---

## 7. Validation

A contract is correctly migrated when:

- `grep "REQ:" crates/<crate>/src/*.rs` shows only `#[contract(...)]` registrations (no free-form pre/post/inv comments).
- `cargo test -p hkask-rsolidity` passes for the macro tests.
- `scripts/contract-audit.sh --summary` still reports the same number of contracts.
- `kask cns health` shows no new algedonic alerts.

---

## 8. References

- [`FUNCTIONAL_SPECIFICATION.md`](./FUNCTIONAL_SPECIFICATION.md) — spec anchor
- [`data/rsolidity_contract_manifest.json`](../../data/rsolidity_contract_manifest.json) — 406 extracted contract IDs
- [`PRINCIPLES.md`](./PRINCIPLES.md) — P1–P12 definitions
- [`MDS.md`](./MDS.md) — Minimum Definition Specification
- [`crates/hkask-cns/src/energy.rs`](../../crates/hkask-cns/src/energy.rs) — starting migration target
