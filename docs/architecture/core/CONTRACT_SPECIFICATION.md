# Contract Specification

**Version:** v0.28.0
**Created:** 2026-06-16
**Consolidated:** 2026-06-18
**Status:** Active — definitive specification of the hKask contract standard
**Supersedes:** `FUNCTIONAL_SPECIFICATION.md` §5, `PRINCIPLES.md` §1.5–1.6, `TESTING_DISCIPLINE.md` §1, `RSOLIDITY_VOCABULARY.md`

---

## 1. The Contract Traceability Chain

Every behavioral guarantee in hKask is expressed as a **contract** — a structured annotation on a `pub fn` that traces through five layers of content connected by a single **contract ID**.

```
                        contract_id: "P9-cns-energy-001"
┌──────────────────┐   ┌─────────────┐   ┌───────────┐   ┌──────────────┐
│    EXPECTATION   │   │   CONTRACT   │   │   CODE     │   │    TEST      │
│ expect: "..."    │   │ pre:/post:   │   │ pub fn     │   │ proptest     │
│ [P{N}]           │   │ conditions   │   │ impl       │   │ verifies     │
└────────┬─────────┘   └──────┬──────┘   └─────┬──────┘   └──────┬───────┘
         │                    │                │                  │
         └────────────────────┴────────────────┴──────────────────┘
                    all joined by the contract ID
```

### 1.1 The Five Content Layers

| Layer | What It Is | What Work It Does | Canonical Form |
|-------|-----------|-------------------|----------------|
| **Expectation** | `expect: "user-voice statement" [P{N}]` | Expresses WHY the contract matters in the user's voice. Grounds the contract in a goal principle (P1–P12). | `/// expect: "I can verify gas budgets prevent runaway agents" [P9]` |
| **Contract** | `pre: condition / post: condition` | Defines HOW the code guarantees behavior. Input constraints and output guarantees. | `/// pre: budget.remaining >= cost` / `/// post: returns true iff sufficient gas remains` |
| **Code** | `pub fn can_proceed(...)` | The implementation that satisfies the contract. | Rust source |
| **Test** | `proptest!(|inputs| assert(postcondition))` | Verifies the contract holds for all valid inputs. | `tests/contract/{crate}_contract.rs` |
| **Specification** | `Spec { id: UUID, criteria: [...] }` | Defines WHAT must exist — the functional requirement. An optional superset: many contracts exist without a formal spec. | `spec/goal/capture` → MDS spec store |

The first four layers are mandatory. The specification layer is recommended but not required — a contract can exist without a formal spec.

### 1.2 The Contract ID (Identity)

The **contract ID** is the stable identity that joins all layers. It appears in two forms:

| Form | Status | Example |
|------|--------|---------|
| `#[contract(id = "P9-cns-energy-001", principle = "P9")]` | **Target** (rSolidity attribute) | The canonical contract identity |
| `/// REQ: P9-cns-energy-001` | **Transitional** (doc-comment convention) | Present until full rSolidity migration completes |

Both forms carry the same contract ID. When `#[contract]` is present, the `/// REQ:` line is redundant — delete it. The `#[contract].id` field is the contract ID.

### 1.3 Contract ID Format

```
P{N}-{crate_prefix}-{domain}-{NNN}
```

| Component | Meaning | Example |
|-----------|---------|---------|
| `P{N}` | Goal principle number (1–12) | `P9` |
| `{crate_prefix}` | Crate short name | `cns`, `sto`, `wallet`, `inf`, `svc`, `typ` |
| `{domain}` | Domain within crate | `energy`, `triple`, `board` |
| `{NNN}` | Sequence number (001–999) | `001` |

Validation rules:
- `P{N}` must be 1–12 and match the goal principle
- No hyphens in `{domain}` beyond the separator
- `{NNN}` must be three digits, zero-padded
- The full ID must be unique within the workspace

### 1.4 Relationship to `spec_id`

A **spec_id** is a UUID from the MDS spec store. A contract **optionally** references a spec_id. The contract_id is the core — it names the contract in code. The spec_id is an edge — it connects to an external specification.

```
contract_id: "P9-cns-energy-001"
    │
    └── optionally references ──► spec_id: "a1b2c3d4-..."
```

Not every contract needs a spec. A contract is valid if it has `expect:`, `pre:`, and `post:` — regardless of whether a formal spec exists.

---

## 2. Contract Content Specification

### 2.1 The Expectation Layer (`expect:`)

**Mandatory for every contract.** Expresses what the user expects from this function in the user's natural voice. Always tagged with exactly one goal principle.

```
/// expect: "Energy cost types preserve semantic identity — a gas unit is never confused with a cap or a rate" [P8]
```

Rules:
- Must read as a first-person user statement ("I can...", "I can verify that...")
- Must reference the observable behavior, not the implementation
- Must carry exactly one `[P{N}]` goal principle tag
- The goal principle must be the single principle that the expectation most directly expresses

### 2.2 Goal Principle Selection

Every contract has exactly one **goal principle** — the Magna Carta principle (P1–P12) that the user's expectation most directly expresses.

| Contract Domain | Goal Principle | Reasoning |
|----------------|---------------|-----------|
| CNS regulation, gas budgets, alerts, metering | **P9** (Homeostatic Self-Regulation) | The contract IS the regulation boundary |
| Storage CRUD, content generation, memory | **P3** (Generative Space) | Creates or persists new entities |
| User data ownership, auth, keystore | **P1** (User Sovereignty) | Protects user ownership boundaries |
| Type constructors, newtypes, conversions | **P8** (Semantic Grounding) | Preserves type-level identity |
| Service orchestration, CLI thin wrappers | **P5** (Essentialism) | Minimizes abstraction layers |
| API boundaries, capability checks, OCAP membranes | **P4** (Clear Boundaries) | Enforces permission boundaries |
| Consent-gated operations, subscriptions | **P2** (Affirmative Consent) | Requires explicit user approval |
| Test harness generators, strategy functions | **P8** (Semantic Grounding) | Generates well-formed test data |

### 2.3 Constraining Principles

Every contract also carries 1–11 **constraining principles** — the other Magna Carta principles that constrain how the goal is achieved.

```
/// [P5] Constraining: Essentialism — minimal newtype, no validation or transformation
```

Constraining principles appear as `[P{N}] Constraining:` annotations. Common bundles:

| Domain | Typical Constraints |
|--------|--------------------|
| Storage with owner WebID | `[P1]`, `[P4]` |
| Storage with visibility | `[P1]`, `[P4]` |
| Replicant-called functions | `[P12]` |
| Budget-constrained functions | `[P4]` |
| API endpoints | `[P4]`, `[P1]` |
| Newtypes | `[P5]` |

### 2.4 The Contract Layer (`pre:` / `post:` / `inv:`)

**Mandatory for every contract.** At least one `pre:` or `post:` condition.

```
/// pre:  owner is a valid, non-nil WebID
/// pre:  name is non-empty
/// post: returns Ok(Board) where board.id != 0
/// post: board persists — subsequent board_get(board.id) returns Some(board)
```

Rules:
- `pre:` conditions describe what must be true before the call
- `post:` conditions describe what must be true after the call
- Multiple `pre:` and `post:` lines are permitted
- `inv:` describes a cross-operation invariant (rare, for stateful types)
- Conditions are English prose, not formal logic — the test verifies them

### 2.5 Probabilistic Contracts

For LLM-driven functions where exact-match postconditions are impossible, contracts may carry a `prob:` line:

```
/// prob: p=0.85, δ=0.05, k=3
```

| Parameter | Meaning |
|-----------|---------|
| `p` | Probability threshold (e.g., 0.85 = postcondition must hold in 85% of trials) |
| `δ` | Tolerance bound (how far from the postcondition is acceptable) |
| `k` | Recovery window (how many retries per trial before counting failure) |

Probabilistic contracts are verified by `ProbContractRunner` in `hkask-test-harness`.

---

## 3. rSolidity Enforcement Layer

rSolidity is the formally adopted contracting language for hKask (2026-06-18). It provides runtime enforcement of contract conditions through macros.

### 3.1 The `#[contract]` Attribute

The `#[contract(id=..., principle=...)]` attribute carries the contract identity and principle anchoring as structured metadata.

```rust
/// expect: "Gas budgets prevent runaway agents" [P9]
/// pre:  budget.remaining >= cost
/// post: returns true iff sufficient gas remains
#[contract(id = "P9-cns-energy-001", principle = "P9")]
pub fn can_proceed(&self, cost: EnergyCost) -> bool {
    // implementation
}
```

### 3.2 Runtime Macros

| Macro | Purpose | When |
|-------|---------|------|
| `rs::require!(cond, "msg")` | Precondition check — reverts if false | Start of function |
| `rs::assert!(cond, "msg")` | Invariant check — reverts if false | Mid-function |
| `rs::revert!("msg")` | Explicit failure with reason | Early return |

### 3.3 Migration Status

The `#[contract]` attribute is the target. `/// REQ:` is the transitional convention. Migration is tracked by `scripts/ci/contract-audit.sh --summary`.

| State | Contract Count | Percentage |
|-------|---------------|------------|
| `#[contract]` attributes | 390 | 15.1% |
| `/// REQ:` (transitional) | 2,194 | 84.9% |
| Total contracts | 2,584 | 100% |

### 3.4 Dependencies

Crates using `#[contract]` require:
- `hkask-rsolidity = { path = "../hkask-rsolidity" }` in `[dependencies]`
- `use hkask_rsolidity as rs;` (for `rs::require!` / `rs::assert!` / `rs::revert!`)
- `use hkask_rsolidity::contract;` (for bare `#[contract]` attribute)

---

## 4. The Contract Audit Standard

`scripts/ci/contract-audit.sh` is the single source of truth for contract metrics.

### 4.1 Dashboard

```
bash scripts/ci/contract-audit.sh --summary
```

Reports per-crate: PubFns, Contracted, Cover%, `expect:` count, Grounding%, `#[contract]` count.

### 4.2 Quality Metrics

| Metric | Meaning | Gate |
|--------|---------|------|
| **Cover%** | Contracted / PubFns × 100 | ≥ 90% (trend, not hard) |
| **Grounding%** | `expect:` count / Contracted × 100 | ≥ 80% (trend) |
| **rSolidity%** | `#[contract]` count / Contracted × 100 | Migration progress |
| **Quality score** | Weighted (expect: 35%, principles 30%, constraining 25%, base 10%) | ≥ 80% |

---

## 5. Contract Rules

| Rule | Description |
|------|-------------|
| C1 | Every `pub fn` should carry exactly one contract block (`expect:` + `pre:`/`post:`) |
| C2 | Every contract must have `expect:` with exactly one `[P{N}]` goal principle |
| C3 | Every contract must have at least one `pre:` or `post:` condition |
| C4 | Goal principle must match the domain map (§2.2) |
| C5 | Every test must reference its contract ID (`// contract:P9-cns-energy-001`) |
| C6 | `#[contract].id` is canonical when present; `/// REQ:` is transitional |

---

## Appendix: Complete Example

```rust
/// expect: "Energy costs prevent runaway agent resource consumption" [P9]
/// [P9] Motivating: Homeostatic Self-Regulation — this IS the regulation boundary
/// [P4] Constraining: Clear Boundaries — OCAP budget caps prevent runaway consumption
/// [P5] Constraining: Essentialism — single predicate, no branching
/// pre:  cost is a valid EnergyCost
/// post: returns true iff budget.remaining >= cost
/// prob: p=0.99, δ=0.01, k=0 (deterministic in practice)
#[contract(id = "P9-cns-energy-001", principle = "P9")]
pub fn can_proceed(&self, cost: EnergyCost) -> bool {
    self.remaining >= cost.0
}
```

References: [`FUNCTIONAL_SPECIFICATION.md`](FUNCTIONAL_SPECIFICATION.md) (domain map, §5 realignment), [`PRINCIPLES.md`](PRINCIPLES.md) (principle hierarchy), [`TESTING_DISCIPLINE.md`](TESTING_DISCIPLINE.md) (test verification).
