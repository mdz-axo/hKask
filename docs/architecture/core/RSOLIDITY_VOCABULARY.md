# rSolidity Contract Vocabulary

**Status:** Macro crate implemented — first migration target (`hkask-cns` energy budget contracts) complete  
**Source anchor:** [`FUNCTIONAL_SPECIFICATION.md`](./FUNCTIONAL_SPECIFICATION.md)  
**Version:** 0.27.0

---

## 1. Purpose

rSolidity is the runtime-contract layer for hKask. It makes the declarative `/// REQ:` contracts in Rust source executable via macros and attributes, while keeping the source comments authoritative for `scripts/contract-audit.sh`. This document defines the vocabulary and the rewrite pattern from spec contracts to rSolidity macros.

The design goal is **mechanical fidelity where the type system is insufficient**: every `pre:`, `post:`, and `inv:` clause becomes a `require!`, `assert!`, `revert!`, or `emit!` invocation. Principle annotations (`[P9] Motivating`, `[P4] Constraining`) are carried as compile-time metadata on `#[contract(...)]` for validation and future span namespace selection.

---

## 2. Macro Vocabulary

| Macro | Solidity Analog | Role in hKask | Emits CNS Span |
|-------|-----------------|---------------|----------------|
| `require!(condition, "contract-id", "msg")` | `require` | **Precondition** — must hold on entry; panics with the contract id if false | No |
| `assert!(condition, "contract-id", "msg")` | `assert` | **Postcondition / invariant** — must hold after execution; panics in debug if false | No |
| `revert!("contract-id", err)` | `revert` | **Explicit failure path** — returns `Err(err)` from the current function, tagged with a contract id | Optional |
| `emit!(span, verb, phase, payload)` | `emit` | **CNS span emission** — logs the event via `tracing` (`target: "rsolidity.emit"`); the CNS sink listens on the same target | Yes |
| `#[ocap(resource, operation)]` | modifier / capability check | **OCAP boundary gate** — injects `<Self as Ocap>::verify_ocap(...)` at the start of a method | Yes |
| `#[contract(id = "...", principle = "P9", pre = "...", post = "...", inv = "...")]` | n/a | **Compile-time contract metadata** — validates id/principle format and re-emits the item; source `/// REQ:` comments remain authoritative | No |

### 2.1 `require!`

```rust
require!(
    gas.0 <= self.available().0 || !self.hard_limit,
    "P9-cns-energy-budget-can-proceed",
    "gas exceeds available budget and hard_limit is enforced"
);
```

- Evaluates the condition.
- On failure, panics with a message containing the contract id and the supplied description.
- Does **not** emit a span by default; combine with `emit!` for algedonic feedback.
- Use only for programmer-error preconditions; recoverable domain errors should use `revert!`.

### 2.2 `assert!`

```rust
assert!(
    self.remaining.0 + self.reserved.0 <= self.cap.0,
    "P9-cns-energy-budget-invariant",
    "remaining + reserved must never exceed cap"
);
```

- Evaluates the condition after the guarded code runs.
- Panics on violation (all builds, via `core::assert!`).
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

- Logs the event via `tracing::info!` with `target: "rsolidity.emit"`.
- `span` is any type implementing `Display + Serialize` (e.g. `CnsSpan`).
- `verb` is any `Display` value (typically `"submitted"`, `"measured"`, etc.).
- `phase` is any `Debug + Serialize` value (e.g. `Phase::Act` from `hkask_types::event`).
- `payload` is any `Debug + Serialize` value (e.g. `serde_json::json!({...})`).
- The CNS sink subscribes to the same `rsolidity.emit` target, so emitted events are ingested without a direct crate dependency.

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
- Validates that `id` matches `P#-...` and `principle` is `P1`–`P12`.
- Re-emits the annotated item unchanged so the existing source `/// REQ:` comment remains the authoritative audit signal.
- Future releases may generate a build-time manifest from these attributes; for now `scripts/contract-audit.sh` continues to scan source comments.

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

Implemented:

```text
crates/hkask-rsolidity/
├── Cargo.toml              # re-exports macros and attributes from hkask-rsolidity-macros
├── src/
│   └── lib.rs              # Ocap trait, __private_emit helper, declarative macros
└── tests/
    └── rsolidity_smoke.rs  # smoke tests for all six vocabulary items

crates/hkask-rsolidity-macros/
├── Cargo.toml              # proc-macro crate
└── src/
    └── lib.rs              # #[ocap] and #[contract] attribute macros
```

### 5.1 Dependencies

- `hkask-rsolidity-macros` provides the proc-macro attributes.
- `proc-macro2`, `quote`, `syn` (in the macros crate only) for macro authoring.
- `serde` and `serde_json` for typed span payloads in `emit!`.
- `tracing` for the `emit!` sink.

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

- `#[contract(...)]` metadata is present above the function and matches the source `/// REQ:` id.
- `cargo test -p hkask-rsolidity` passes for the macro tests.
- `scripts/contract-audit.sh --summary` still reports the same number of contracts (source comments remain authoritative).
- `kask cns health` shows no new algedonic alerts.

## 8. Tooling Policy

rSolidity is a Rust layer. Python generator scripts are **not** retained in the repository; any ad-hoc Python used during exploration must be deleted before the work is considered complete. The contract inventory is maintained by `scripts/contract-audit.sh` scanning source `REQ:` comments.

## 9. References

- [`FUNCTIONAL_SPECIFICATION.md`](./FUNCTIONAL_SPECIFICATION.md) — spec anchor
- [`PRINCIPLES.md`](./PRINCIPLES.md) — P1–P12 definitions
- [`MDS.md`](./MDS.md) — Minimum Definition Specification
- [`crates/hkask-cns/src/energy.rs`](../../../crates/hkask-cns/src/energy.rs) — first migration target
- [`crates/hkask-rsolidity/src/lib.rs`](../../../crates/hkask-rsolidity/src/lib.rs) — macro and trait implementations
- [`crates/hkask-rsolidity-macros/src/lib.rs`](../../../crates/hkask-rsolidity-macros/src/lib.rs) — attribute proc-macro implementations
