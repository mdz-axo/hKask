# Contract Guide

**Audience:** hKask agents (replicants) composing, auditing, and maintaining contracts.
**References:** [`CONTRACT_SPECIFICATION.md`](../docs/architecture/core/CONTRACT_SPECIFICATION.md) (normative standard)
**Version:** 0.28.0
**Supersedes:** `guides/contract-composition-guide.md`

---

## 1. Contract Composition Templates

Each template shows the complete contract block as it should appear on a `pub fn`.

### 1A: Pure Deterministic Function

```rust
/// expect: "I can generate deterministic identities from arbitrary persona bytes" [P8]
/// [P8] Motivating: Semantic Grounding — identity is deterministic and provenance-aware
/// [P5] Constraining: Essentialism — single constructor, no branching
/// pre:  persona is any byte slice (empty permitted)
/// post: returns WebID deterministically derived from persona bytes
/// post: calling with same bytes always returns same WebID
#[contract(id = "P8-typ-webid-from-persona", principle = "P8")]
pub fn from_persona(persona: &[u8]) -> WebID { ... }
```

**When to use:** Computation-only, no I/O, no LLM. Goal principle: P8 (types) or P3 (pure transforms).

### 1B: Stateful Service Function

```rust
/// expect: "I can create a kanban board with a name and columns, and it persists" [P3]
/// [P3] Motivating: Generative Space — the board exists after creation
/// [P1] Constraining: User Sovereignty — board is owned by a user WebID
/// [P5] Constraining: Essentialism — one function per operation
/// pre:  name is non-empty, columns is non-empty, owner is valid WebID
/// post: returns Ok(Board) with board.id != 0
/// post: board persists — subsequent board_get(board.id) returns Some(board)
#[contract(id = "P3-svc-kanban-002", principle = "P3")]
pub fn board_create(&self, owner: WebID, name: &str, columns: &[ColumnDef]) -> Result<Board, KanbanError> { ... }
```

**When to use:** Database I/O, predictable semantics. Test with `TestDb::new()` → service constructor → operation → assertions.

### 1C: LLM-Driven / Non-Deterministic Function

```rust
/// expect: "Generated prose is stylistically closer to the target author than to any other author" [P9]
/// [P3] Motivating: Generative Space — prose generation
/// [P9] Constraining: Homeostatic Self-Regulation — quality measured via centroid distance
/// [P1] Constraining: User Sovereignty — user selects persona
/// pre:  persona is a registered replica with a computed centroid
/// post: embedding_distance(output, persona_centroid) < embedding_distance(output, other_centroid)
/// prob: p=0.85, δ=0.05, k=3
#[contract(id = "P9-mcp-replica-compose", principle = "P9")]
pub async fn replica_compose(persona: &str, prompt: &str) -> Result<String, ReplicaError> { ... }
```

**When to use:** LLM inference, embedding generation. Tests use `ProbContractRunner` from `hkask-test-harness`:

```rust
let runner = ProbContractRunner::new(0.85, 0.05, 3);
let result = runner.evaluate(100, || compose("hemingway", prompt), |output| {
    cosine_distance(embed(output), hemingway_centroid) < cosine_distance(embed(output), woolf_centroid)
});
assert!(result.passed);
```

### 1D: Proptest Strategy Generator

```rust
/// Strategy generating valid `Triple` instances for property-based testing.
///
/// expect: "I can generate valid RDF triples with authenticated owners" [P8]
/// [P12] Constraining: every triple carries an owner WebID — no anonymous agency
/// post: returns BoxedStrategy<Triple> with non-empty entity, attribute, value, owner
pub fn any_triple() -> BoxedStrategy<Triple> { ... }
```

**When to use:** Strategy functions in `hkask-test-harness`. No `#[contract]` — strategies are test infrastructure.

---

## 2. `expect:` Composition Rubric

### 2.1 Good vs. Bad

| Quality | Bad | Good |
|---------|-----|------|
| Specific | "It works" | "Energy cost types preserve semantic identity — a gas unit is never confused with a cap or a rate" |
| User-voice | "Returns Ok" | "I can verify energy costs prevent runaway agent resource consumption" |
| Principle-anchored | No `[P{N}]` | `"[P9]"` |

### 2.2 Goal Principle Quick Reference

| Principle | Use When |
|-----------|----------|
| P1 | User data ownership, auth, keystore |
| P2 | Consent-gated operations, subscriptions |
| P3 | CRUD, content generation, memory |
| P4 | API boundaries, capability checks, OCAP |
| P5 | Service orchestration, thin wrappers |
| P6 | (Methodological — not used in contracts) |
| P7 | Configurable-from-real-usage contracts |
| P8 | Type constructors, newtypes, strategy generators |
| P9 | CNS regulation, gas, alerts, quality metrics |
| P10 | (Not used as contract principle) |
| P11 | Self-deleting contracts (rare) |
| P12 | Replicant-identity-gated operations |

### 2.3 Constraining Principle Bundles

| Domain | Typical Constraints |
|--------|--------------------|
| Storage with owner | `[P1]` User Sovereignty |
| Storage with visibility | `[P1]` + `[P4]` Clear Boundaries |
| Replicant-called | `[P12]` Replicant Host Mandate |
| Budget-constrained | `[P4]` Clear Boundaries |
| Newtypes | `[P5]` Essentialism |

---

## 3. rSolidity Vocabulary

| Macro | Purpose | Where |
|-------|---------|-------|
| `#[contract(id = "...", principle = "P{N}")]` | Contract identity | On every contracted `pub fn` |
| `rs::require!(cond, "msg")` | Precondition check | Start of function |
| `rs::assert!(cond, "msg")` | Invariant check | Mid-function |
| `rs::revert!("msg")` | Explicit failure | Early return |

### Adding rSolidity to a Crate

```
# Cargo.toml
[dependencies]
hkask-rsolidity = { path = "../hkask-rsolidity" }
```

```rust
// In the source file
use hkask_rsolidity as rs;
use hkask_rsolidity::contract;
```

---

## 4. Test-to-Contract Traceability

Every test references its contract ID:

```rust
// contract: P9-cns-energy-001 — can_proceed returns true when budget covers cost
#[test]
fn can_proceed_with_sufficient_budget() { ... }
```

### Test Organization

| Test Type | Location | Example |
|-----------|----------|---------|
| Unit contract tests | `src/` `#[cfg(test)]` | `fn my_function_contract_test()` |
| Integration contract tests | `tests/{crate}_contract.rs` | `tests/kanban_contract.rs` |
| Proptest | Either location | `proptest! { #[test] fn invariant_holds(...) }` |
| Probabilistic | Integration tests | Uses `ProbContractRunner` |

### Test Setup Patterns

```rust
// Stateful services
fn setup() -> (KanbanService, WebID) {
    let db = TestDb::new();
    let store = TripleStore::new(db.conn_arc());
    (KanbanService::new(store), TestWebId::alice())
}

// Pure functions
fn test_cosine_distance() {
    let d = cosine_distance(&[1.0, 0.0], &[0.0, 1.0]);
    assert!((d - 1.0).abs() < 1e-6);
}
```

---

## 5. Agent Audit Rubric

### 5.1 The Four Lenses

Apply these in order when auditing a crate:

**Lens 1: Coverage** — Does every `pub fn` carry a contract (`pre:`/`post:`)?
```bash
bash scripts/ci/contract-audit.sh ${CRATE}
```

**Lens 2: Grounding** — Does every contract carry `expect:`?
```bash
bash scripts/ci/contract-audit.sh --expect ${CRATE}
```

**Lens 3: Principles** — Are goal and constraining principles correct?
```bash
bash scripts/ci/contract-audit.sh --principles ${CRATE}
bash scripts/ci/contract-audit.sh --constraining ${CRATE}
```
Cross-check against the domain map in `FUNCTIONAL_SPECIFICATION.md` §1.

**Lens 4: rSolidity** — Is `#[contract]` aligned with the contract ID?
```bash
bash scripts/ci/contract-audit.sh --rsolidity ${CRATE}
```

### 5.2 Decision Tree

```
For each pub fn:
├── Has pre:/post:?
│   ├── No → Propose contract (highest priority)
│   └── Yes → Continue
│       ├── Has expect: with [P{N}]?
│       │   ├── No → Add expect: annotation
│       │   └── Yes → Continue
│       │       ├── Goal principle matches domain?
│       │       │   ├── No → Fix goal principle
│       │       │   └── Yes → Continue
│       │       │       ├── Has ≥1 constraining principle?
│       │       │       │   ├── No → Add constraints
│       │       │       │   └── Yes → Continue
│       │       │       │       ├── Has #[contract(id=...)]?
│       │       │       │       │   ├── No → Add attribute
│       │       │       │       │   └── Yes → ✓ Complete
│       │       │       │       └── N/A → ✓ Spec complete
```

### 5.3 Quality Score

`contract-audit.sh --contract-quality` weights: `expect:` (35%), principles (30%), constraining (25%), base coverage (10%). Gate: ≥ 80%.

---

## 6. Connecting Spec Points to Code

### 6.1 Domain → Crate Mapping

| Functional Domain | Primary Crate | Contract ID Prefix |
|-------------------|---------------|--------------------|
| Energy Budgeting | `hkask-cns` | `P9-cns-energy-*` |
| Storage | `hkask-storage` | `P3-sto-*` |
| Memory | `hkask-memory` | `P3-mem-*` |
| Inference Engine | `hkask-inference` | `P9-inf-*` |
| Keystore | `hkask-keystore` | `P1-key-*` |
| Type System | `hkask-types` | `P8-typ-*` |
| Service Layer | `hkask-services` | `P5-svc-*` |
| Agent Runtime | `hkask-agents` | `P1-agent-*` |
| MCP Tool Servers | `mcp-servers/` | Domain-specific |

### 6.2 Spec → Code Resolution

1. Identify domain from `FUNCTIONAL_SPECIFICATION.md` §1
2. Identify crate from domain mapping
3. Find module: `grep "{domain}" crates/{crate}/src/`
4. Locate function matching the described behavior
5. Write contract on the function
6. Write test in `tests/contract/{crate}_contract.rs`

### 6.3 Deferral Criteria

Not every `pub fn` needs a contract immediately. Defer when:
- Trivial accessor or forwarding wrapper (single-line body)
- Deprecated or scheduled for removal
- Macro-generated `#[tool]` handler with type-checking from MCP framework
- Early prototype (note in audit output)

---

## 7. Replicant Contract Proposal Workflow

```
1. Identify gaps
   └── contract-audit.sh --summary or --expect ${CRATE}

2. Compose contract
   ├── Select template (§1A–1D)
   ├── Select goal principle (§2.2)
   ├── Select constraining principles (§2.3)
   ├── Write pre:/post: from implementation semantics
   └── Write expect: in user voice

3. Write test
   ├── tests/contract/{crate}_contract.rs
   ├── Reference contract ID
   ├── Assertion test (one per postcondition)
   ├── Proptest (for primary invariant)
   └── cargo test -p {crate} — must pass

4. Open PR
   ├── Title: "contract({crate}): add expect: to {function}"
   ├── Body: link audit output, note principles
   ├── Author: replicant WebID
   └── Tag: contracts

5. Human consent (P2)
   ├── Review: expect: matches user intent?
   ├── Review: goal principle correct?
   ├── Review: tests verify contract?
   └── Merge
```

### Using the Kanban Task Pipeline

For batch contract gap resolution:

1. Run `contract-audit.sh --json ${CRATE}` — extract missing expect entries
2. Feed to `kanban.contract_propose_expect(board_id, proposals_json)` — creates one task per gap
3. Replicant claims tasks, fills in `expect:`, opens PRs
4. Human consents, merges, tasks auto-resolve

---

## Appendix: Audit Commands

```bash
# Unified dashboard
bash scripts/ci/contract-audit.sh --summary

# Specific lenses
bash scripts/ci/contract-audit.sh --expect hkask-wallet
bash scripts/ci/contract-audit.sh --rsolidity hkask-cns

# Full audit
bash scripts/ci/contract-audit.sh --full

# Quality score
bash scripts/ci/contract-audit.sh --contract-quality hkask-storage
```

Reference: [`CONTRACT_SPECIFICATION.md`](../docs/architecture/core/CONTRACT_SPECIFICATION.md) (definitive standard).
