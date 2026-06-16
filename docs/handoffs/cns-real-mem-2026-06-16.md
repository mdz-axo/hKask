# Handoff — Contract ID Realignment (Memory Complete)

## Session Context

Continue converting every `/// REQ:` / `// REQ:` contract ID in the hKask workspace to the machine-parseable `P{N}-{domain}-{operation}` format, update `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` per crate, and keep every targeted crate building cleanly. This session completed the `hkask-memory` realignment (source was already aligned; work focused on verifying annotations and updating documentation), and prepared the workspace to start `hkask-inference` next.

## What Was Done

- **`hkask-memory` realigned / verified**
  - 9 source files verified: `src/recall_dedup.rs`, `src/consolidation.rs`, `src/consolidation_service.rs`, `src/episodic.rs`, `src/episodic_loop.rs`, `src/semantic.rs`, `src/semantic_loop.rs`, `src/salience.rs`, `src/ranking.rs`
  - 67 `P3-mem-*` occurrences (52 unique production IDs + 16 test IDs)
  - All production contracts carry `[P3] Motivating: Generative Space …` plus relevant `[P{N}] Constraining: …` annotations (P1, P4, P5, P7, P8, P9)
  - `cargo check -p hkask-memory` passes
  - `cargo test -p hkask-memory` passes — 16 tests, 0 failures

- **Documentation updated**
  - `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md`:
    - Domain Map: updated Memory contract count from 8 to 52
    - Section 3.3: replaced stub with full Memory section (52 production + 16 test contract rows)
    - Section 4.1: added Memory migration row
    - Appendix A metadata: updated contract count and build-status command
  - `docs/architecture/core/REQ_CONTRACT_INVENTORY.md`: regenerated (6,691 lines)

- **Helper scripts removed**
  - No Python helper scripts remain in `scripts/` after use.

## What Remains

| # | Priority | Task | Crate | Notes |
|---|----------|------|-------|-------|
| 1 | MEDIUM | Realign inference contracts | `hkask-inference` | Map `INFER-*` / `inf-cfg-*` / `chat-proto-*` to `P9-inf-*` and `P4-inf-*` per Domain Map (P9 + P4). ~87 REQ occurrences across 9 source files. |
| 2 | MEDIUM | Realign template engine | `hkask-templates` | P3 per Domain Map. |
| 3 | MEDIUM | Realign service layer | `hkask-services` | P5 + P7 per Domain Map. |
| 4 | MEDIUM | Realign MCP servers | `mcp-servers/` | P5 per Domain Map. |
| 5 | LOW | Realign type system | `hkask-types` | P8; large surface — defer if needed. |
| 6 | LOW | Realign communication | `hkask-comm` | P1 per Domain Map. |
| 7 | LOW | Realign keystore | `hkask-keystore` | P1 per Domain Map. |
| 8 | LOW | Realign API surface | `hkask-api` | P1 + P4 per Domain Map. |
| 9 | LOW | Realign CLI | `kask` | P3 per Domain Map. |

## Recommended Skills and Tools

- **coding-guidelines** — before editing; enforce surgical changes and goal-driven execution.
- **handoff** — if ending mid-crate, produce a handoff like this one.
- **tdd** — verify with tests; every test carries a `// REQ:` tag.
- Discovery command per crate: `grep -n "REQ:" crates/<crate>/src/**/*.rs`
- Verification commands:
  ```bash
  cargo check -p hkask-inference          # next
  cargo check -p hkask-memory
  bash scripts/gen-req-inventory.sh
  ```
- Prefer a small temporary Python script for one crate at a time, then review the diff and delete the script immediately.

## Key Decisions to Preserve

1. **Principle numbering is canonical** from `docs/architecture/core/PRINCIPLES.md`: P1 = User Sovereignty, P2 = Affirmative Consent, P3 = Generative Space, P4 = Clear Boundaries, P5 = Essentialism, P7 = Evolutionary Architecture, P8 = Semantic Grounding, P9 = Homeostatic Self-Regulation, P12 = Subscriber Consent.
2. **Memory domain is P3 motivating**. All `hkask-memory` contracts use a `P3-mem-*` prefix, even when constraining principles are P1/P4/P9/P8/P5/P7.
3. **One motivating principle per contract ID**. Constraining principles appear as body annotations.
4. **Build must pass per crate**. Run `cargo check -p <crate>` and `bash scripts/gen-req-inventory.sh` before declaring a crate done.
5. **Contract counts are discoverable, not fixed**. Realign every `REQ:` occurrence rather than trusting estimates.

## Current Workspace Notes

- `cargo check -p hkask-cns` currently fails due to pre-existing `CnsSpan`/`SpanNamespace` type mismatches in `crates/hkask-cns/src/wallet_gas_calibrator.rs` and `crates/hkask-cns/src/calibrated_energy_estimator.rs`. This is unrelated to the memory realignment and was not fixed in this session.
- `scripts/calibrate_contract_placement.py` exists in the workspace but is not from this session; do not delete unless explicitly asked.
