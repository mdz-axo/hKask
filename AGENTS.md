# Agent Operating Guide — hKask

**hKask** (ℏKask) — A Minimal Viable Container for Replicants | `kask` binary | `hkask-` crate prefix | v0.31.0

---

## Capability Catalog

**48 Skills** (PDCA), **2 Templates** (one-shot), **1 Bundle** (kata). **51 capabilities total.**

| Type | Behavior |
|------|----------|
| **Skill** | PDCA FlowDef with convergence threshold + energy budget + loop action |
| **Template** | One-shot prompt execution, no registry manifest |
| **Bundle** | Composition orchestrator, delegates to sub-skills (non-PDCA) |

### Guardrails (activate first)

| Skill | When to Activate |
|-------|-----------------|
| **coding-guidelines** | Before writing or reviewing any code |

### Core Development

| Skill | When to Activate |
|-------|-----------------|
| **bug-hunt** | Bug hunting. Run expeditions against target crates. |
| **task-breakdown** | Decompose work into verifiable vertical-slice tasks with acceptance criteria. Convergent planning PDCA. |
| **tdd** | Building features or fixing bugs. RED→GREEN→REFACTOR. |
| **diagnose** | Debugging hard bugs or performance regressions. |
| **deep-module** | Module design. Deletion test, interface minimalism. |
| **refactor-service-layer** | Extracting duplicated logic from CLI/API/MCP surfaces. |
| **improve-codebase-architecture** | Finding deepening opportunities in the codebase. |
| **strangler-fig** | Incremental architectural migration. |
| **idiomatic-rust** | Type-driven Rust design through Hoare's principles. |
| **diataxis-diagram** | Generate Mermaid diagrams (ERD, flowchart, state, sequence, class) from code with Diataxis quality evaluation. |
| **wardley-mapper** | Strategic mapping of any system's components on evolution × value chain. |
| **codegraph** | Code understanding: query, traverse, analyze, and assemble context from the code graph. |

### Reasoning & Analysis

| Skill | When to Activate |
|-------|-----------------|
| **pragmatic-semantics** | Classify statements by certainty, constraint force, provenance. |
| **pragmatic-cybernetics** | Feedback loops, variety engineering, system homeostasis. |
| **pragmatic-laziness** | Find the path of least action through meaning-space. |
| **essentialist** | Recursive eliminative interrogation (Exist → Surface → Contract). |
| **review** | Self-critique for contradictions, unsupported claims, logical gaps. |
| **grill-me** | Socratic questioning to stress-test understanding. |
| **zoom-out** | Broader context on unfamiliar code. |
| **sequential-inquiry** | Dynamic chain-of-thought with automatic deep-dive delegation. |
| **falsifiability** | Domain-agnostic eliminative inference (Popper/Platt/Chamberlin + Pearl counterfactuals). Rule out the untestable, eliminate the falsified, corroborate survivors. Delegation target. |
| **metacognition** | Self-reflective goal decomposition, progress assessment, and strategy calibration. Any replicant. |
| **semantic-graph-audit** | Domain-agnostic semantic dependency graph analysis. Detects cycles, redundancies, gaps, orphans. |

### Kata & Coaching

| Skill | When to Activate |
|-------|-----------------|
| **kata** (Bundle — registry manifest, not a `.agents/skills/` directory) | Toyota Kata system — starter + improvement + coaching. |
| **kata-coaching** | 5-question Coaching Kata dialogue. |
| **kata-improvement** | 4-step Improvement Kata PDCA pattern. |
| **kata-starter** | Foundational kata practice routines. |
| **improv** | Agent interaction grammar (Plussing, Yes And, Freestyling, Riffing). |

### Meta & Maintenance

| Skill | When to Activate |
|-------|-----------------|
| **skill-maintenance** | Audit skill architecture for staleness, coverage gaps. |
| **skill-logic-audit** | Audit .j2 template logic against stated goals. |
| **skill-bundler** | Compose multiple skills into a cohesive bundle. |
| **skill-discovery** | Find, evaluate, and install skills for hKask. Registry crate is canonical source of truth. |
| **handoff** | Session handoff — capture what was done, what remains. |

### Specialized

| Skill | When to Activate |
|-------|-----------------|
| **superforecasting** | Calibrated probability forecasting (Tetlock). |
| **mcda** | Multi-Criteria Decision Analysis. |
| **scenario-builder** | Schwartz scenario planning. |
| **hypothesis-framer** | Research question framing (FINER + PICO). |
| **adversarial-red-team** | Adversarial robustness testing with defense-layer awareness. Probes hKask's 8-layer defense-in-depth stack and reports which layers were bypassed. |
| **kali-audit** | Security review of Rust code, Jinja2 templates, YAML manifests, supply chain, MCP surfaces, and LLM I/O boundaries. Forward-adaptable: consumes regression library at runtime, checks 8-layer defense coverage, discovers surfaces dynamically. |
| **supply-chain-sentinel** | Dependency manifest audit (Cargo.toml, deny.toml, lockfiles). Deeper than kali-audit's supply-chain surface: version pinning, registry verification, SBOM tracking, license conflicts. Emits `cns.supply_chain.*` spans. |
| **runtime-posture-monitor** | Runtime security posture monitoring. Observes CNS telemetry (`hkask.*` spans, `cns.guard.*` violations, `cns.regulation` events) for runtime threats: endpoint abuse, bot traffic, LLM usage anomalies. Emits `cns.runtime.*` spans. |
| **attack-taxonomy-mapper** | Maps supply chain findings to OSC&R attack taxonomy (verified against `github.com/pbom-dev/OSCAR`). Consumes findings from supply-chain-sentinel and kali-audit. Adds `taxonomy_mapping` field to regression YAML. Emits `cns.taxonomy.*` spans. |
| **goal-analysis** | Goal specification and completion verification. |
| **magna-carta-verifier** | Verify Magna Carta principles enforcement. |
| **structured-extraction** | Extract structured data from unstructured text. |
| **caveman** | Multi-mode text compression.
| **self-critique-revision** | Iterative self-critique and revision cycle.
| **gpa-evolution** | Genetic-Pareto evolutionary optimization over text artifacts (prompts). NL reflection as gradient. |
| **media-workflow** | Multi-step media pipeline skill. Compose Fal.ai workflow DAGs from natural-language intent. |
| **logo-builder** (Template) | Pragmatic logo design. |
| **qa-script-builder** (Template) | Design autonomous QA pipeline manifests. |

---

## Prohibitions

From Magna Carta (P1–P4) and P12. Violations **must be deleted**.

| # | Prohibition | Principle |
|---|-------------|----------|
| 1 | No `todo!()`, `unimplemented!()`, `#[deprecated]`, unused traits, or stubs | P5 · P3 |
| 2 | No anonymous agency — every action has an authenticated author | P12 · P1 |
| 3 | No hidden parameters or admin-gated settings | P3 |
| 4 | No pass-through abstractions (deep-module discipline) | P5 · P7 |

### CI-Enforced Gates

Mechanical CI enforcement (inline steps in the `ci` job in `.github/workflows/ci.yml`, not a separate job):

| Gate | Enforcement | Script |
|------|-------------|--------|
| Visual-UI/monitoring infrastructure prohibited | `grep` scan for grafana/prometheus/dashboard imports | inline in `ci.yml` |
| No hardcoded secrets | Environment variables or keystore only | inline in `ci.yml` |
| No `Result<_, String>` | Use `thiserror` enums for library errors | `scripts/check-string-errors.sh` |
| No unused crate dependencies | `nightly -D unused_crate_dependencies` | separate nightly job |
| MCP servers must have tool-behavior contract tests | `Parameters(` seam present in `mcp-servers/*/tests/` | `scripts/check-mcp-tool-tests.sh` (ratcheted allowlist) |
| CNS canonical namespace invariant | every `cns.*` tracing target registered in `CANONICAL_NAMESPACES` (performative telemetry uses `hkask.*`) | `scripts/check-cns-canonical.sh` |
| Security regression library enforced | every `status: enforced` regression in `security/regressions/` is checked | `scripts/check-kali-regressions.sh` (ratcheted) |

Prohibitions #1–#4 above are design discipline. Only #1 is partially CI-gated via `clippy -D warnings` (catches `todo!()`/`unimplemented!()`). Prohibitions #2–#4 are enforced by code review, not mechanical gates.

---

## Tooling Policy

hKask is a Rust project. Python is **not** an acceptable project dependency. Ad-hoc Python scripts are permitted during exploration but must be deleted before work is complete. Ad-hoc generated artifacts (one-off JSON inventories, scratch manifests) must also be removed. Permanent generated docs under `docs/generated/` and skill `manifest.yaml` files are part of the system and excluded.

Preferred auxiliary tooling: shell (`bash`) under `scripts/`, Rust binaries or `build.rs` for source/Cargo metadata.

---

## Key Docs

- `.github/workflows/ci.yml` — CI pipeline (fmt, clippy, unused-deps, build, test, doc, invariants)
- `.github/workflows/audit.yml` — Weekly dependency audit (cargo-deny + cargo-audit)
- `scripts/check-string-errors.sh` — CI guard: `Result<_, String>` anti-pattern detector
- `scripts/check-mcp-tool-tests.sh` — CI guard: MCP tool-behavior test coverage (ratcheted)
- `scripts/check-cns-canonical.sh` — CI guard: `cns.*` prefix reserved for canonical CNS spans (performative telemetry uses `hkask.*`)
- `scripts/check-kali-regressions.sh` — CI guard: security regression library enforcement (ratcheted)
- `crates/hkask-types/src/observable_span.rs` — `ObservableSpan` trait and domain span enums
- `crates/hkask-types/src/lib.rs` — Foundation types
- `crates/hkask-types/src/macros.rs` — Shared `enum_str_ops!` and `enum_snake_str!` macros (canonical location)
- `crates/hkask-types/src/error.rs` — `InfrastructureError`, `DatabaseErrorKind`, `McpErrorKind`
- `crates/hkask-ports/src/lib.rs` — Hexagonal port traits
- `crates/hkask-ports/src/federation.rs` — `FederationDispatch`, `FederationDispatchError`
- `crates/hkask-cns/src/types/loops/mod.rs` — `LoopAction`, `LoopActionParams`, `ActionType`, `ImpactReport`, `ActionDecision`, `LoopQuality` (re-exported from `hkask-types`)
- `crates/hkask-cns/src/types/loops/loop_trait.rs` — `Loop` trait, `HkaskLoop`, `ExperienceClassification`
- `crates/hkask-cns/src/regulation_policy.rs` — `RegulationPolicy`, `RegulationRule` (`pub(crate)` — internal regulation rules)
- `crates/hkask-cns/src/sensor_provider.rs` — `SensorProvider` trait, `SensorRegistry` (`pub(crate)` — internal sensor registration)
- `crates/hkask-cns/src/tool_stats.rs` — Statistical learning: LogNormal cost distributions, Beta reliability tracking (`ToolStats` re-exported at crate root)
- `crates/hkask-mcp/src/lib.rs` — `bootstrap_mcp_server`, `impl_tool_context!`, `MCPBootstrap`
- `crates/hkask-codegraph/src/lib.rs` — Code understanding engine: types, graph, indexer, context assembly
- `crates/hkask-codegraph/src/types.rs` — Core types: Symbol, Edge, SymbolKind, EdgeKind, Visibility, Complexity
- `mcp-servers/hkask-mcp-codegraph/src/lib.rs` — CodeGraph MCP server: 11 tools (query, traverse, impact, analysis, context, structure, stats, reindex, feedback, embed, dead_code)
- `crates/hkask-agents/src/curator_agent/metacognition/mod.rs` — Curator metacognition
- Dependency governance: CI unused-deps job (`nightly -D unused_crate_dependencies`)
- Feature gating: `hkask-communication` matrix feature, `hkask-cli` communication/tui/api features
- Coding conventions: `enum_str_ops!` for PascalCase/snake_case enum conversion (dual-form `parse_str`); `enum_snake_str!` for snake_case-primary enums (`as_str` + `FromStr`); `thiserror` enums for library errors; `impl_tool_context!` for MCP server ToolContext impls

> Architecture docs canonical location: `docs/architecture/`. See `docs/reference/` for API reference, `docs/explanation/` for design decisions, `docs/how-to/` for procedures, and `docs/tutorial/` for getting started. Documentation health is mechanically verified by `docs/ci/verify-docs.sh`.


