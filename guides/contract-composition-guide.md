# Contract Composition Guide

**Audience:** hKask agents (replicants) auditing and maintaining contract-spec alignment.
**Domain:** `architecture`, `lifecycle`
**Last Updated:** 2026-06-18
**Version:** 0.28.0

---

## 1. The Contract Traceability Chain

hKask contracts trace through **five layers of content** connected by a **contract ID** — a stable identifier that runs through every layer.

```
                 Contract ID: P9-cns-energy-001
                 ┌───────────────────────────────────┐
                 │  appears in three places today:    │
                 │  /// REQ:  (transitional)          │
                 │  #[contract(id=...)]  (target)     │
                 │  // REQ: on tests  (transitional)  │
                 └───────────────┬───────────────────┘
                                 │
┌──────────────────┐   ┌────────┴┐   ┌──────────┐   ┌──────────────────┐
│ SPECIFICATION    │──►│EXPECT   │──►│ CONTRACT │──►│ CODE             │
│ spec_id (UUID)   │   │user voice│   │pre:/post:│   │ pub fn impl      │
│ "what must exist"│   │"why"     │   │"how"     │   │ satisfies it     │
└──────────────────┘   └─────────┘   └──────────┘   └────────┬─────────┘
                                                              │
                                              ┌───────────────┘
                                              ▼
                                    ┌──────────────────┐
                                    │ TEST             │
                                    │ proptest verifies│
                                    │ contract holds   │
                                    └──────────────────┘
```

### What Each Layer Does (Content)

| Layer | What It Is | What Work It Does | Where It Lives |
|-------|-----------|-------------------|----------------|
| **Specification** | `Spec { id: UUID, criteria: [...] }` | Defines WHAT must exist. Every contract traces to exactly one spec. | `spec/goal/capture` → MDS spec store |
| **Expectation** | `expect: "I can verify energy costs prevent runaway agents" [P9]` | Expresses WHY this matters to the user. Grounds the contract in a goal principle. | `/// expect:` on the `pub fn` |
| **Contract** | `pre: ...post: ...` | Defines HOW the code guarantees behavior. Input constraints and output guarantees. | `/// pre:` / `/// post:` on the `pub fn` |
| **Code** | `pub fn can_proceed(...)` | Satisfies the contract. Can change entirely; the contract shouldn't. | Rust source files |
| **Test** | `proptest!(|inputs| assert(postcondition))` | Verifies the contract holds for all valid inputs. | `tests/contract/{crate}_contract.rs` |

### How the Contract ID Connects Layers (Transitional → Target)

The contract ID (`P9-cns-energy-001`) is the stable identity. How it gets marked is currently in transition:

| Place | Today (transitional) | Target (after migration) |
|-------|---------------------|--------------------------|
| On the function | `/// REQ: P9-cns-energy-001` (doc-comment convention) | `#[contract(id = "P9-cns-energy-001", principle = "P9")]` (rSolidity attribute) |
| On the test | `// REQ: P9-cns-energy-001 — summary` (doc-comment convention) | `// contract:P9-cns-energy-001 — summary` or test attribute |
| Audit counting | `grep "/// REQ:"` and `grep "///* REQ:"` | `grep '#\[contract'` |

The migration is a marker swap. A script can do it:

```bash
# Phase 1: Add hkask-rsolidity to every crate with contracts
# Phase 2: Swap markers
find crates/ mcp-servers/ -name '*.rs' -exec sed -i \
  's|/// REQ: \(P[0-9]*-[a-z-]*\)|#[contract(id = "\1")]|g' {} +
find crates/ mcp-servers/ -name '*.rs' -exec sed -i \
  's|// REQ: \(P[0-9]*-[a-z-]*\)|// contract:\1|g' {} +
# Phase 3: Update contract-audit.sh to count #[contract] instead of /// REQ:
```

**Why hasn't this happened yet?** `#[contract]` requires `hkask-rsolidity` as a dependency. Only `hkask-cns`, `hkask-test-harness`, `hkask-wallet`, `hkask-storage`, `hkask-mcp-spec`, and `hkask-agents` have it. Once every crate that carries contracts adds the dependency, the swap is mechanical.

**Until then:** The guide uses `/// REQ:` as the convention because it's the universal marker that works everywhere. But the logical architecture treats the contract ID as primary — `REQ:` is just how we currently spell it.

### The Full Chain, Collapsed

```
Spec (UUID: a1b2...) ──► ID: P9-cns-energy-001 ──┬──► expect: "..." [P9]
                                                  ├──► pre: budget >= cost
                                                  ├──► post: returns true iff sufficient
                                                  ├──► pub fn can_proceed(...)
                                                  └──► proptest!(|cost| assert!(invariant))
```

Five layers of content. One contract ID joining them.

---

## 2. rSolidity Smart Contract Framework

rSolidity is the contracting language. The contract ID lives in `#[contract]`. The doc-comment block (`///`) carries the content layers (expectation, pre/post). They're two facets of one contract.

### 2.1 The Doc-Comment Block (Content Layers 2 + 3)

Every contracted `pub fn` carries expectation and pre/post conditions as doc-comments:

```rust
/// expect: "<user expectation>" [P{N}]      ← Layer 2: WHY
/// [P{N}] Motivating: <rationale>           ← principle anchoring
/// [P{N}] Constraining: <rationale>         ← constraining forces
/// pre:  <condition>                         ← Layer 3: HOW
/// post: <condition>                         ← Layer 3: HOW
pub fn my_function(...) -> ... { }            ← Layer 4: CODE
```

### 2.2 The `#[contract]` Attribute (Identity + Enforcement)

The `#[contract(id=..., principle=...)]` attribute is the contract ID — it names this contract so tests, audit scripts, and runtime enforcement can reference it:

```rust
/// expect: "Gas budgets prevent runaway agents" [P9]
/// pre:  budget.remaining >= cost
/// post: returns true iff sufficient gas remains
#[contract(id = "P9-cns-energy-001", principle = "P9")]
pub fn can_proceed(&self, cost: EnergyCost) -> bool {
    rs::require!(cost.0 > 0, "energy cost must be positive");
    self.remaining >= cost.0
}
```

**Key rule:** `#[contract(id)]` is the contract ID. `/// pre:`/`/// post:` is the contract content. They describe the same contract. Only `#[contract]` enables runtime enforcement via `rs::require!` / `rs::assert!` / `rs::revert!`.

### 2.3 Transitional Convention: `/// REQ:`

Where `#[contract]` is not yet present (84.9% of contracts), the contract ID is carried by a doc-comment:

```rust
/// REQ: P9-cns-energy-001                   ← transitional contract ID
/// expect: "Gas budgets prevent runaway agents" [P9]
/// pre:  budget.remaining >= cost
/// post: returns true iff sufficient gas remains
#[contract(id = "P9-cns-energy-001", principle = "P9")]  ← target contract ID
pub fn can_proceed(...) { }
```

When `#[contract]` is present, the `/// REQ:` line is redundant — delete it. The `id` field is the contract ID.

### 2.4 rSolidity Vocabulary

| Macro | Purpose | When |
|-------|---------|------|
| `#[contract(id = "...", principle = "P{N}")]` | Contract identity + principle anchoring | On every contracted `pub fn` |
| `rs::require!(cond, "msg")` | Precondition check — reverts if false | Start of function |
| `rs::assert!(cond, "msg")` | Invariant check — reverts if false | Mid-function |
| `rs::revert!("msg")` | Explicit failure with reason | Early return on error |

### 2.5 Drift Detection

`contract-audit.sh --rsolidity` detects when the global marker and the `#[contract]` attribute disagree:

| Drift Type | Meaning | Fix |
|-----------|---------|-----|
| `UNMIGRATED` | `/// REQ:` exists but no `#[contract(id=...)]` | Add `hkask-rsolidity` dep, add `#[contract]`, delete `/// REQ:` |
| `ORPHAN_RSOLIDITY` | `#[contract]` exists but the global marker is missing | Add `/// REQ:` (transitional) — will be deleted after full migration |
| `ID_MISMATCH` | `/// REQ:` ID differs from `#[contract].id` | `#[contract].id` is canonical — fix `/// REQ:` to match
```

### 2.3 rSolidity Vocabulary

| Macro | Purpose | When |
|-------|---------|------|
| `rs::require!(cond, "msg")` | Precondition check | Start of function |
| `rs::assert!(cond, "msg")` | Invariant check | Mid-function |
| `rs::revert!("msg")` | Explicit failure with reason | Early return on error |

### 2.4 Drift Detection

`contract-audit.sh --rsolidity` detects three drift types:

| Drift Type | Meaning | Fix |
|-----------|---------|-----|
| `UNMIGRATED` | `/// REQ:` exists but no `#[contract(id=...)]` | Add the attribute |
| `ORPHAN_RSOLIDITY` | `#[contract]` exists but no `/// REQ:` | Add the doc-comment contract |
| `ID_MISMATCH` | `/// REQ:` and `#[contract]` have different IDs | Align them |

---

## 3. Contract Composition Templates

### 3A: Pure Deterministic Function

**Use when:** Computation-only, no I/O, no LLM dependency.

```rust
/// expect: "I can generate deterministic identities from arbitrary persona bytes" [P8]
/// [P8] Motivating: Semantic Grounding — identity is deterministic and provenance-aware
/// [P5] Constraining: Essentialism — single constructor, no branching
/// pre:  persona is any byte slice (empty permitted)
/// post: returns WebID deterministically derived from persona bytes
/// post: calling with same bytes always returns same WebID
#[contract(id = "P8-typ-webid-from-persona", principle = "P8")]
pub fn from_persona(persona: &[u8]) -> WebID {
    // implementation
}
```

**Template rules:**
- Goal principle is **P8** (Semantic Grounding) for type constructors, **P3** (Generative Space) for pure transforms
- Constraining principles: **P5** (Essentialism — minimal API surface)
- Pre/post conditions are mathematical equalities
- Can be tested with deterministic assertion tests + proptest
- Test references: `// contract:P8-typ-webid-from-persona`

### 3B: Stateful Service Function

**Use when:** Database I/O, filesystem, network access with predictable semantics.

```rust
/// expect: "I can create a kanban board with a name and columns, and it persists" [P3]
/// [P3] Motivating: Generative Space — the board exists after creation
/// [P1] Constraining: User Sovereignty — board is owned by a user WebID
/// [P5] Constraining: Essentialism — one function per operation
/// pre:  name is non-empty, columns is non-empty, owner is valid WebID
/// post: returns Ok(Board) with board.id != 0
/// post: board is persisted — subsequent board_get(board.id) returns Some(board)
/// post: board appears in board_list(owner)
#[contract(id = "P3-svc-kanban-002", principle = "P3")]
pub fn board_create(
    &self,
    owner: WebID,
    name: &str,
    columns: &[ColumnDef],
) -> Result<Board, KanbanError> {
```

**Template rules:**
- Goal principle is **P3** (Generative Space) for CRUD, **P1** (User Sovereignty) for ownership-gated ops
- Constraining principles: **P1** (sovereignty), **P12** (replicant identity), **P5** (essentialism)
- `post:` conditions describe persistence state and error conditions
- Test setup: `TestDb::new()` → service constructor → operation → assertion chain
- Every error path gets a separate `post:` variant

### 3C: LLM-Driven / Non-Deterministic Function

**Use when:** LLM inference, embedding generation, style composition. Postconditions are probabilistic.

```rust
/// expect: "Generated prose is stylistically closer to the target author than to any other author" [P9]
/// [P3] Motivating: Generative Space — prose generation
/// [P9] Constraining: Homeostatic Self-Regulation — quality measured via centroid distance
/// [P1] Constraining: User Sovereignty — user selects persona
/// pre:  persona is a registered replica with a computed centroid
/// pre:  prompt is non-empty
/// post: embedding_distance(output, persona_centroid) < embedding_distance(output, other_centroid)
/// prob: p=0.85, δ=0.05, k=3
#[contract(id = "P9-mcp-replica-compose", principle = "P9")]
pub async fn replica_compose(
    persona: &str,
    prompt: &str,
) -> Result<String, ReplicaError> {
```

**Template rules:**
- Goal principle is **P9** (Homeostatic) or **P3** (Generative)
- Postconditions carry a `prob:` line with `(p, δ, k)` parameters (§7.6 of Testing Discipline)
- Tests use `ProbContractRunner` from `hkask-test-harness`:
  ```rust
  let runner = ProbContractRunner::new(0.85, 0.05, 3);
  let result = runner.evaluate(100, || compose("hemingway", prompt), |output| {
      cosine_distance(embed(output), hemingway_centroid) < cosine_distance(embed(output), woolf_centroid)
  });
  assert!(result.passed);
  ```
- The `k` recovery window allows per-trial retries before counting failure
- `expect:` describes qualitative intent ("stylistically closer"), not exact match

### 3D: Proptest Strategy Generator

**Use when:** Defining a strategy function in `hkask-test-harness::strategies`.

```rust
/// Strategy generating valid `Triple` instances.
///
/// Produces triples with random entity/attribute strings,
/// string JSON values, and random owner WebIDs.
///
/// expect: "I can generate valid RDF triples with non-empty entities, attributes, and authenticated owners for property-based testing" [P8]
/// [P12] Constraining: every triple carries an owner WebID — no anonymous agency
/// post: returns `BoxedStrategy<Triple>` with non-empty entity, attribute, value, owner
pub fn any_triple() -> BoxedStrategy<Triple> {
```

**Template rules:**
- Goal principle is **P8** (Semantic Grounding) — the strategy generates semantically valid data
- `expect:` describes what property test authors expect from the generator
- `[P{N}] Constraining:` annotations explain design decisions (e.g., "triples always carry owners" → P12)
- The `///` doc comment above `expect:` describes the generator's behavior (not part of the contract chain)
- Strategies don't carry `#[contract]` — they're test infrastructure, not production contracts

---

## 4. `expect:` Composition Rubric

### 4.1 What Makes a Good `expect:` Statement

| Quality | Bad | Good |
|---------|-----|------|
| Specific | "It works" | "Energy cost types preserve semantic identity — a gas unit is never confused with a cap or a rate" |
| User-voice | "Returns Ok" | "I can verify energy costs prevent runaway agent resource consumption" |
| Principle-anchored | No [P{N}] | `"[P9]"` — explicit goal principle tag |
| Testable | "Is fast" | "Distance to self is zero — the identity invariant" |

### 4.2 Goal Principle Selection

| Contract Type | Dominant Principle | Reasoning |
|---------------|-------------------|-----------|
| CNS regulation / gas / alerts / metering | **P9** (Homeostatic Self-Regulation) | The contract IS the regulation boundary |
| Storage CRUD / content generation / memory | **P3** (Generative Space) | Creates or persists new entities |
| User data ownership / auth / keystore | **P1** (User Sovereignty) | Protects user ownership boundaries |
| Type constructors / newtypes / conversions | **P8** (Semantic Grounding) | Preserves type-level identity |
| Service-layer orchestration / CLI thin wrappers | **P5** (Essentialism) | Minimizes abstraction layers |
| API boundaries / capability checks / OCAP | **P4** (Clear Boundaries) | Enforces permission membrane |
| Consent-gated operations / subscriptions | **P2** (Affirmative Consent) | Requires explicit user approval |
| Strategy generators / test harness fixtures | **P8** (Semantic Grounding) | Generates well-formed test data |

### 4.3 Constraining Principle Selection

Every contract is governed by **exactly one goal principle** and **1–11 constraining principles**. Constraining principles appear as `[P{N}] Constraining:` annotations.

**Common constraint bundles:**

| Domain | Typical Constraints |
|--------|--------------------|
| Storage function with owner WebID | `[P1] Constraining: User Sovereignty — owner_webid carries ownership` |
| Storage function with visibility | `[P1] Constraining: ...`, `[P4] Constraining: Clear Boundaries — visibility gates read access` |
| Function called by replicants | `[P12] Constraining: Replicant Host Mandate — every action has an authenticated author` |
| Budget-constrained function | `[P4] Constraining: Clear Boundaries — OCAP budget caps prevent runaway consumption` |
| API endpoint | `[P4] Constraining: Clear Boundaries — API key scoping`, `[P1] Constraining: User Sovereignty — per-user rate limits` |
| Newtype | `[P5] Constraining: Essentialism — minimal wrapper, no validation or transformation` |

---

## 5. Test-to-Spec Traceability

### 5.1 Test-to-Contract Traceability

Every test carries a comment referencing the contract ID it verifies — currently `// REQ:`, soon `// contract:`:

```rust
// REQ: P9-cns-energy-001 — can_proceed returns true when budget covers cost
#[test]
fn can_proceed_with_sufficient_budget() {
    // ...
}
```

The contract ID (`P9-cns-energy-001`) is the `id` field from `#[contract(id = "P9-cns-energy-001")]` on the function. Every test references exactly one contract.

### 5.2 MDS Category Mapping

| MDS Category | What It Covers | Contract ID Prefix Examples |
|-------------|----------------|---------------------------|
| `domain` | Core domain entities and invariants | `P8-typ-*`, `P3-sto-*` |
| `composition` | Service wiring, orchestration | `P5-svc-*`, `P7-*` |
| `trust` | Sovereignty, consent, keystore | `P1-*`, `P2-*`, `P4-*` |
| `lifecycle` | CRUD, deployment, backup | `P3-svc-*`, `P3-sto-*` |
| `curation` | Spec management, contract quality | N/A (meta-category) |

### 5.3 Verifying Alignment with `contract-audit.sh`

```
bash scripts/ci/contract-audit.sh --summary     # unified dashboard
bash scripts/ci/contract-audit.sh --expect      # which contracts lack expect:
bash scripts/ci/contract-audit.sh --rsolidity    # drift detection
bash scripts/ci/contract-audit.sh --full        # all modes on all crates
```

The dashboard reads:

```
Crate                 PubFns Contracted Cover% expect: Ground% #[contract]
hkask-cns                 153       197  129.4%     125   63.1%          69
```

- **Cover% > 100%** = more REQ: tags than pub fns (helpers, constructors, trait impls also carry contracts)
- **Ground% < 100%** = contracts without `expect:` — replicant work queue
- **#[contract] = 0** = no rSolidity migration started for this crate

---

## 6. Agent Audit Rubric

### 6.1 The Four Audit Lenses

When auditing contract-spec alignment, apply these four lenses in order:

#### Lens 1: Coverage — Does Every `pub fn` Carry a Contract?

```bash
bash scripts/ci/contract-audit.sh "${CRATE}"  # lists uncontracted functions
```

**Red flag:** Any `pub fn` with 0% coverage. Prioritize by function importance, not alphabetical.

#### Lens 2: Grounding — Does Every Contract Carry `expect:`?

```bash
bash scripts/ci/contract-audit.sh --expect "${CRATE}"  # lists missing expect:
```

**Red flag:** `expect:` grounding below 80%. These contracts have formal pre/post conditions but no user-voice expectation. Replicants cannot verify Link 2 (Contract → UserExpectation) without `expect:`.

#### Lens 3: Principles — Are Goal and Constraining Principles Correct?

```bash
bash scripts/ci/contract-audit.sh --principles "${CRATE}"
bash scripts/ci/contract-audit.sh --constraining "${CRATE}"
```

**Red flag:** A contract with `[P9]` goal principle that is not a CNS regulation function. Cross-check against the domain map in `FUNCTIONAL_SPECIFICATION.md` §1.

#### Lens 4: rSolidity — Are Both Layers Aligned?

```bash
bash scripts/ci/contract-audit.sh --rsolidity "${CRATE}"
```

**Red flag:** `ID_MISMATCH` — the `#[contract].id` and the doc-comment contract ID disagree. `#[contract].id` is canonical.

### 6.2 Audit Decision Tree

```
For each pub fn in crate:
├── Has /// REQ:?
│   ├── No → Propose contract (highest priority)
│   └── Yes → Continue
│       ├── Has expect: on contract?
│       │   ├── No → Add expect: annotation
│       │   └── Yes → Continue
│       │       ├── Has valid [P{N}] goal principle?
│       │       │   ├── No → Fix goal principle anchor
│       │       │   └── Yes → Continue
│       │       │       ├── Has ≥1 constraining principle?
│       │       │       │   ├── No → Add constraining principle
│       │       │       │   └── Yes → Continue
│       │       │       │       ├── Has #[contract(..)]?
│       │       │       │       │   ├── No → Add rSolidity attribute
│       │       │       │       │   └── Yes → Check IDs match
│       │       │       │       │       ├── Mismatch → Align IDs
│       │       │       │       │       └── Match → ✓ Complete
│       │       │       │       └── N/A (not migrated yet) → ✓ Spec layer complete
│       │       │       └── N/A (constr. optional for this domain)
│       │       └── Continue
│       └── Continue
└── Continue
```

### 6.3 Quality Score

`contract-audit.sh --contract-quality` computes a 4-layer score:

| Layer | Weight | What It Measures |
|-------|--------|-----------------|
| expect: (user voice) | 35% | How many contracts carry user-facing expectation language |
| [P{N}] goal-principle | 30% | How many contracts are principle-anchored |
| [P{N}] Constraining: | 25% | How many contracts acknowledge constraining forces |
| pre:/post: (base coverage) | 10% | Baseline contract density |

**Quality gate:** ≥ 80%. Below 80%, the crate lacks sufficient contract grounding for autonomous replicant maintenance.

---

## 7. Connecting Spec Points to Code

### 7.1 Spec → Crate Mapping

| Functional Specification Domain (§1) | Primary Crate(s) | Contract ID Prefix |
|--------------------------------------|------------------|--------------------|
| Energy Budgeting | `hkask-cns` | `P9-cns-energy-*` |
| Algedonic Signalling | `hkask-cns` | `P9-cns-algedonic-*` |
| Runtime Observability | `hkask-cns` | `P9-cns-*` |
| Wallet | `hkask-wallet` | `P9-wallet-*` |
| Storage | `hkask-storage` | `P3-sto-*` |
| Memory | `hkask-memory` | `P3-mem-*` |
| Inference Engine | `hkask-inference` | `P9-inf-*` |
| Template Engine | `hkask-templates` | `P3-tpl-*` |
| Keystore | `hkask-keystore` | `P1-key-*` |
| Type System | `hkask-types` | `P8-typ-*` |
| Service Layer | `hkask-services` | `P5-svc-*` |
| Agent Runtime | `hkask-agents` | `P1-agent-*` |
| MCP Tool Servers | `mcp-servers/` | Domain-specific prefix |
| Test Harness | `hkask-test-harness` | `HARN-*` |

### 7.2 Spec → Code Resolution Protocol

When given a specification requirement, resolve to the right code location:

1. **Identify the domain** from `FUNCTIONAL_SPECIFICATION.md` §1 domain table
2. **Identify the crate** from the domain → crate mapping above
3. **Identify the module** by searching for the domain tag in crate source (e.g., `grep "energy" crates/hkask-cns/src/`)
4. **Locate the function** by matching the specification's described behavior to `pub fn` signatures
5. **Write the contract** on the function (or propose one if the function exists but is uncontracted)
6. **Write the test** in `tests/contract/{crate}_contract.rs` with a comment referencing the contract ID

### 7.3 When to Defer

Not every `pub fn` needs a contract immediately. Defer when:

- The function is a trivial accessor or forwarding wrapper (single-line body)
- The function is deprecated or scheduled for removal
- The function is a macro-generated `#[tool]` handler with type-checking provided by the MCP framework
- The crate is in early prototype phase (note the deferral in contract-audit.sh output)

---

## 8. Replicant Contract Proposal Workflow

When a replicant proposes a new contract or strengthens an existing one:

```
1. Identify the gap
   ├── contract-audit.sh --summary shows Ground% < 80% for a crate
   ├── contract-audit.sh --expect lists MISSING_EXPECTATION lines
   └── Select ONE function to focus on

2. Compose the contract
   ├── Follow the template for the function type (§3A–3D)
   ├── Select goal principle from rubric (§4.2)
   ├── Select constraining principles (§4.3)
   ├── Write pre:/post: conditions from the implementation's semantics
   └── Write expect: in user-voice language (§4.1)

3. Write the test
   ├── Create or extend tests/contract/{crate}_contract.rs
   ├── Add a comment referencing the contract ID (e.g., `// contract:P3-svc-kanban-002`)
   ├── Write deterministic assertion test (one per postcondition)
   ├── Write proptest for the function's primary invariant (if applicable)
   └── cargo test -p {crate} — must pass

4. Open a PR
   ├── PR title: "contract({crate}): add expect: to {function_name}"
   ├── PR body: link to contract-audit.sh output, note principle selections
   ├── Author: replicant WebID
   └── Tag: contracts

5. Human consent (P2)
   ├── Review: does expect: match user intent?
   ├── Review: is goal principle correct?
   ├── Review: do tests verify the contract?
   └── Merge
```

---

## Appendix: Quick Reference

### Contract ID Format

```
P{N}-{crate_prefix}-{domain}-{NNN}
```

Examples: `P9-cns-energy-001`, `P3-sto-triple-new`, `P8-typ-webid-from-persona`, `HARN-008`

### Script Commands

```bash
# Unified dashboard (all crates + mcp-servers)
scripts/ci/contract-audit.sh --summary

# Full audit with violations listed
scripts/ci/contract-audit.sh --full

# Specific audit lenses
scripts/ci/contract-audit.sh --expect hkask-wallet
scripts/ci/contract-audit.sh --rsolidity hkask-cns
scripts/ci/contract-audit.sh --contract-quality hkask-storage

# Coverage only (original mode)
scripts/ci/contract-audit.sh hkask-types

# Machine-readable
scripts/ci/contract-audit.sh --json
scripts/ci/contract-audit.sh --csv
```

### Principle Quick Reference

| # | Principle | Audit Concern |
|---|-----------|--------------|
| P1 | User Sovereignty | Does the contract protect user data boundaries? |
| P2 | Affirmative Consent | Does the contract require explicit user approval? |
| P3 | Generative Space | Does the contract enable creation/persistence? |
| P4 | Clear Boundaries | Does the contract enforce permission membranes? |
| P5 | Essentialism | Is the contract minimal? Does it do one thing? |
| P6 | Testability | (Not used as contract principle — methodological) |
| P7 | Evolutionary Architecture | Is the contract calibrateable from real usage? |
| P8 | Semantic Grounding | Does the contract preserve type-level identity? |
| P9 | Homeostatic Self-Regulation | Does the contract enforce resource/quality bounds? |
| P10 | Plausible Deniability | Does the contract protect replicant identity? |
| P11 | Debt Consciousness | Does the contract self-delete on mutation coverage failure? |
| P12 | Replicant Host Mandate | Does the contract carry authenticated actor identity? |
