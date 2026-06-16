# Handoff: CNS Contract Realignment + rSolidity Vocabulary

**Date:** 2026-06-16  
**Session scope:** Continue CNS contract realignment (Work Package 1 verification), realign `hkask-wallet` contracts (Work Package 2), and derive the rSolidity contract vocabulary (Work Package 3).  
**Status:** Work Packages 1–3 complete. Tooling-policy question unresolved.

---

## 1. What Was Done

### CNS contracts — verified
- `cargo check -p hkask-cns` passes.
- AGENTS.md constraint checks pass: no headless/monitoring violations, no `todo!` / `unimplemented!` / `#[deprecated]`.
- `scripts/contract-audit.sh --summary` passes: total coverage 124.6%, `hkask-cns` at 112.4%.

### `hkask-wallet` contracts — realigned
- All wallet `REQ:` tags migrated from legacy `P9-wlt-*`, `WALLET-*`, `HINKAL-*`, `MUST-*`, `wallet-price-*`, etc. to `P9-wallet-*` namespace.
- Duplicate IDs split into unique per-function/per-test contracts where semantics differ; intentional single-contract/multiple-site duplicates remain (e.g., `P9-wallet-mgr-fee-estimate`, `P9-wallet-mgr-balance-conservation-pbt`).
- Updated source files:
  - `crates/hkask-wallet/src/manager.rs`
  - `crates/hkask-wallet/src/issuer.rs`
  - `crates/hkask-wallet/src/signing.rs`
  - `crates/hkask-wallet/src/hinkal.rs`
  - `crates/hkask-wallet/src/hedera.rs`
  - `crates/hkask-wallet/src/solana.rs`
  - `crates/hkask-wallet/src/price_feed.rs`
  - `crates/hkask-wallet/tests/hinkal_adapter.rs`
- Updated documentation:
  - `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` — Section 3.1 wallet table, Section 4 counts, Appendix A/B/C.
  - `docs/architecture/core/REQ_CONTRACT_INVENTORY.md` — two stale `P9-wlt-*` IDs corrected.
- `cargo check -p hkask-wallet` and `cargo test -p hkask-wallet --lib` pass (44 tests).
- No legacy `P9-wlt-*`, `MUST-*`, `WALLET-*`, `HINKAL-*`, `P4-signing`, etc. remain in wallet source or docs.

### rSolidity vocabulary — derived
- Created `docs/architecture/core/RSOLIDITY_VOCABULARY.md` defining macros `require!`, `assert!`, `revert!`, `emit!`, attributes `#[ocap]`, `#[contract]`, design rules, example rewrites for `P9-cns-energy-budget-can-proceed` and `P9-cns-energy-budget-reserve`, and migration order.
- Created `data/rsolidity_contract_manifest.json` — 406 contract IDs extracted from `FUNCTIONAL_SPECIFICATION.md`.
- Created `scripts/generate_rsolidity_manifest.py` to regenerate the manifest.
- Marked rSolidity derivation complete in `FUNCTIONAL_SPECIFICATION.md` Appendix B.

---

## 2. What Remains

### HIGH — tooling policy decision
- **Question:** Is Python acceptable for auxiliary generator scripts, or should the project stay Rust/shell-only?
- **Context:** Several Python scripts already exist in `scripts/` (`audit-skills.py`, `calibrate-frontmatter.py`, `fix-active-hlexicon.py`, `calibrate_contract_placement.py`). The rSolidity manifest generator adds one more. User explicitly challenged this and asked the agent not to set policy.
- **Next step:** User must decide. If Python is rejected, delete `scripts/generate_rsolidity_manifest.py` and `data/rsolidity_contract_manifest.json`, and rewrite the generator in Rust or shell. If accepted, document the convention in `AGENTS.md` or `scripts/README.md`.

### MEDIUM — implement rSolidity macro crate
- Scaffold `crates/hkask-rsolidity/` as a proc-macro crate with `require!`, `assert!`, `revert!`, `emit!`, `#[ocap]`, `#[contract]`.
- Add crate to workspace `Cargo.toml`.
- Implement first migration target: `crates/hkask-cns/src/energy.rs` `P9-cns-energy-budget-*` contracts.
- Run `cargo test -p hkask-rsolidity` and verify `scripts/contract-audit.sh --summary` still reports the same contract count.

### LOW — continue non-CNS realignment
- `hkask-agents` (30 contracts), `hkask-inference` (15), `hkask-storage` (12), `hkask-services` (14), etc. are still stubs per `FUNCTIONAL_SPECIFICATION.md` Section 3.

---

## 3. Key Decisions to Preserve

1. **Wallet contract namespace is `P9-wallet-*`, not `P9-wlt-*`.** `FUNCTIONAL_SPECIFICATION.md` Section 5.5 specifies `P9-wallet-*`; source and docs now match.
2. **Contract IDs must be unique per semantic contract, but one contract may be asserted at multiple sites.** Duplicates like `P9-wallet-mgr-fee-estimate` (function + test) and `P9-wallet-mgr-balance-conservation-pbt` (attribute + inner comment) are intentional.
3. **rSolidity is a Rust macro/runtime layer, not a Solidity transpiler.** It does not generate `.sol` files; it provides `require!`, `assert!`, `revert!`, `emit!`, `#[ocap]`, `#[contract]`.
4. **Strangler fig migration.** Old `/// REQ:` comments stay during migration; macros are added alongside and only replace comments after a release cycle without regressions.
5. **No tool or language policy was set by this agent.** The Python generator was added pending user confirmation; user must explicitly approve or reject it.

---

## 4. Recommended Skills for Next Session

- **coding-guidelines** — before writing any code.
- **tdd** — for macro crate implementation.
- **strangler-fig** — for migrating `/// REQ:` to rSolidity macros.
- **deep-module** — for keeping `hkask-rsolidity` surface ≤7 public items.
- **document-update** — if the tooling-policy decision requires doc changes.

---

## 5. Verification Commands

```bash
# Headless + monitoring stack violation (P3 + §5 anti-patterns)
grep -rn "grafana\|prometheus\|dashboard\|visual.*ui\|web.*frontend" crates/ --include="*.rs"

# Stub / dead-code violation (P5)
grep -rn "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"

# Contract completeness audit
scripts/contract-audit.sh --summary

# CNS + wallet build
cargo check -p hkask-cns -p hkask-wallet

# Wallet unit tests
cargo test -p hkask-wallet --lib

# Legacy wallet ID check
grep -R "P9-wlt-\|MUST-[0-9]\|WALLET-[0-9]\|HINKAL-[0-9]\|P4-signing" crates/hkask-wallet/src crates/hkask-wallet/tests --include="*.rs"
```

---

## 6. Sensitive Data

None present in this handoff.
