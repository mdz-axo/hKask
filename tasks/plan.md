# Codegraph Consolidation Plan — Zed-Informed, Surface-Protected

## Target Condition (OUGHT)

Consolidate hKask's codegraph toward Zed's structural efficiency while
provably retaining all four protected surfaces (S4):

1. MCP server tool surface (hkask-mcp-codegraph and siblings)
2. Skill manifest (50 skills in .agents/skills/)
3. Chat / REPL functionality
4. Inference access to every provider

## F1 Baseline (2026-07-22)

| Metric | Value |
|--------|-------|
| Workspace members | 69 (53 crates + 16 MCP servers) |
| Skill directories | 51 |
| Total LOC | 233,385 |
| Total .rs files | 829 |
| `cargo build --workspace` | ✅ |
| `cargo clippy --workspace -- -D warnings` | ✅ (after lifetime fix) |

### Fix applied in baseline
- `mcp-servers/hkask-mcp-codegraph/src/lib.rs:44` — lifetime elision fix
  (`MutexGuard<IndexPipeline>` → `MutexGuard<'_, IndexPipeline>`)

## Final State (2026-07-22)

| Metric | Value | Delta from F1 |
|--------|-------|---------------|
| Workspace members | 60 (44 crates + 16 MCP servers) | -9 (13%) |
| Skill directories | 51 | 0 |
| Total LOC | 232,665 | -720 |
| Total .rs files | 829 | 0 |
| `cargo build --workspace` | ✅ | — |
| `cargo clippy --workspace -- -D warnings` | ✅ | — |
| S4.1 MCP tools | 16 servers, 238 tools | 0 (all preserved) |
| S4.2 Skills | 51 | 0 |
| S4.3 Chat/REPL | ✅ (incl. tui feature) | 0 |
| S4.4 Inference | 9 providers | 0 |

## Zed Reference Analysis (domain_supplement, confidence 0.7)

Zed has ~220 crates for a full IDE — more total crates than hKask, but
its closely-coupled functionality uses modules within crates, not
separate workspace members. Key comparison:

| Area | Zed | hKask | Pattern |
|------|-----|-------|---------|
| MCP/context | 1 `context_server` crate | 16 MCP server crates + `hkask-mcp` | hKask splits more |
| Inference | 16+ per-provider crates | 2 (`hkask-inference`, `hkask-services-inference`) | Zed splits more |
| REPL | 1 `repl` crate | 3 (`hkask-repl`, `hkask-tui`, `hkask-services-chat`) | hKask splits more |
| CLI | 1 `cli` crate | 2 (`hkask-cli`, `hkask-api`) | hKask splits more |

**Transferability hypothesis** (confidence 0.4): Zed's approach of using
modules within crates for single-consumer domain code applies to hKask.

### Falsifiability Admission Gate

- **Testable?** Yes — each merge operation can be tested for S4 preservation.
- **Falsifier?** If merging breaks any S4 surface, the hypothesis is falsified for that pair.
- **Multiple hypotheses:**
  - H1: Single-consumer domain crates can be merged into their consumers without S4 regression.
  - H2: hKask's domain requires more crate seams than Zed's (null hypothesis).
  - H3: Some crates are mergeable, others are not (mixed).

H1 is tested empirically per-pair. H3 is the expected outcome.

## Consolidation Strategy

Merge single-consumer domain crates into their sole consumer. This is
the Zed-informed pattern: don't create a separate workspace member for
code that has only one consumer. The consumer becomes deeper
(deep-module principle), the graph loses a node, and the S4 surface is
preserved because the merge is behind the surface, not at it.

### Single-Consumer Crates (regular deps only)

| Crate | Consumer | LOC | S4 Impact | Phase |
|-------|----------|-----|-----------|-------|
| `hkask-codegraph` | `hkask-mcp-codegraph` | 3157 | None (behind S4.1) | T1 |
| `hkask-bridge-eso` | `hkask-mcp-docproc` | 79 | None | T2 |
| `hkask-bridge-fibo` | `hkask-mcp-docproc` | 68 | None | T2 |
| `hkask-bridge-golem` | `hkask-mcp-docproc` | 80 | None | T2 |
| `hkask-storage-guard` | `hkask-services-context` | 310 | None | T3 |
| `hkask-services-verification` | `hkask-cli` | 537 | None | T4 |
| `hkask-services-research` | `hkask-mcp-research` | 4697 | None (behind S4.1) | T5 |
| `hkask-tui` | `hkask-repl` | 4277 | None (behind S4.3) | T6 |
| `hkask-adapter` | `hkask-mcp-training` | 3628 | None (behind S4.1) | T7 |

### Deferred (needs deeper analysis)

| Crate | Consumer | LOC | Reason |
|-------|----------|-----|--------|
| `hkask-api` | `hkask-cli` | 10836 | Large; different surface types (HTTP API vs CLI) |
| `hkask-repl` | `hkask-cli` | 10414 | S4.3; would create 20K LOC mega-crate |

## Task Sequence (T1–T11)

### Phase 1: Safe single-consumer merges (T1–T5)

- **T1**: Merge `hkask-codegraph` → `hkask-mcp-codegraph`
- **T2**: Merge `hkask-bridge-eso` + `hkask-bridge-fibo` + `hkask-bridge-golem` → `hkask-mcp-docproc`
- **T3**: Merge `hkask-storage-guard` → `hkask-services-context`
- **T4**: Merge `hkask-services-verification` → `hkask-cli`
- **T5**: Merge `hkask-services-research` → `hkask-mcp-research`

### Phase 2: Larger single-consumer merges (T6–T7)

- **T6**: Merge `hkask-tui` → `hkask-repl`
- **T7**: Merge `hkask-adapter` → `hkask-mcp-training`

### Phase 3: Structural evaluation (T8–T10)

- **T8**: Evaluate 2-consumer crates for consolidation potential
- **T9**: Evaluate `hkask-api` → `hkask-cli` (large, different surface types)
- **T10**: Evaluate `hkask-mcp-cloud-gateway` (0 regular consumers, binary)

### Phase 4: Convergence (T11)

- **T11**: Final metrics, S4 retention proof, PDCA log

## PDCA Cycle (per task)

Each task follows:

1. **PLAN**: Identify the merge target and consumer. Map all imports.
2. **DO**: Move source files, update Cargo.toml, fix imports.
3. **CHECK**: `cargo build --workspace` + `cargo clippy --workspace -- -D warnings`.
4. **ACT**: Record before/after delta. If S4 regressed, revert.

## Convergence Gate

- Confidence ≥ 0.7 that no S4 surface regressed
- No pending branches
- S4 fully green (all 4 surfaces verified)
- Codegraph node count reduced vs F1 baseline
- `cargo build --workspace` + `cargo clippy --workspace -- -D warnings` green