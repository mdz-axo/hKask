# Agent Operating Guide — hKask

**hKask** (ℏKask) — A Minimal Viable Container for Agents | `kask` binary | `hkask-` crate prefix | v0.27.0

---

## Skills (Must Load Before Work)

Activate the relevant skill via `skill` tool when its conditions are met:

| Skill | When to Activate |
|-------|-----------------|
| **coding-guidelines** | Before writing or reviewing any code. Surfaces assumptions, enforces simplicity, surgical changes, goal-driven execution. |
| **tdd** | Building features or fixing bugs. Vertical tracer-bullet RED→GREEN→REFACTOR. Every test carries `// REQ:` tag from spec. Anchored on the Testing Discipline (Design by Contract + Property-Based Testing). |
| **refactor-service-layer** | Extracting duplicated business logic from CLI/API/MCP surfaces into `hkask-services`. Strangler fig pattern, deep-module discipline. |
| **improve-codebase-architecture** | Finding deepening opportunities. Walk codebase for shallow modules, tight coupling, untested seams. |
| **condenser-continuation** | Resuming condenser implementation after context reset. Restores session state, prioritizes remaining tasks, verifies build health. |
| **improv** | Agent interaction grammar — Plussing, Yes And, Yes But, Freestyling, Riffing. Sets replicant posture in dual-presence chat, ensemble sessions, and kata coaching. Use `/improv` in REPL. |
| **pragmatics** | Meta-cognitive codebase review. Composes pragmatic-semantics, pragmatic-cybernetics, pragmatic-laziness, essentialist, and coding-guidelines into a unified architecture analysis discipline. Use when reviewing codebase patterns, auditing principle compliance, or analyzing architecture. |
| **document-update** | Systematic documentation corpus maintenance. 7-task workflow: inventory, metadata alignment, writing quality, cross-corpus coherence, spec-code drift, archive, portal refresh. Use when updating docs, aligning specs, or auditing documentation. |

---

## Prohibitions

These derive from the Magna Carta (P1–P4) and P12 of [`docs/architecture/core/PRINCIPLES.md`](docs/architecture/core/PRINCIPLES.md). Violations compromise the system's core identity and **must be deleted**.

| # | Prohibition | Principle | Rationale |
|---|-------------|-----------|-----------|
| 1 | No visual UI, dashboards, Grafana, Prometheus, or monitoring stacks | P3 · §5 | Headless system — CLI/MCP/API only. CNS provides all observability programmatically. |
| 2 | No `todo!()`, `unimplemented!()`, `#[deprecated]`, unused traits, stubs, or feature flags | P5 · P3 | Stubs are debt against the Generative Space. Deprecated code earns deletion, not annotation. |
| 3 | No anonymous agency — every action has an authenticated author | P12 · P1 | Every operation carries a replicant host; every triple stores an `owner` WebID. No root, no `sudo`. |
| 4 | No hidden parameters or admin-gated settings | P3 | All generative settings are user-visible through CLI, API, and REPL. No privileged engineer access. |
| 5 | No pass-through abstractions (deep-module discipline) | P5 · P7 | Modules earn existence by the deletion test. Public surface ≤ 7 items; extras justified or removed. |

These are **Prohibitions**, not guidelines. See PRINCIPLES.md §2.1–2.4 for the full principle hierarchy (Prohibition → Guardrail → Guideline) and constraint force classification.

---

## Tooling Policy

hKask is a Rust project. Python is **not** an acceptable project dependency. Ad-hoc Python scripts may be used during exploration, but they must be deleted before the work is considered complete. Any generated artifacts produced by Python scripts (JSON manifests, inventories, etc.) must also be removed. The repository will be periodically scanned for Python files and purged as a coding violation.

Preferred auxiliary tooling:

- Shell (`bash`) for repository-level scripts under `scripts/`.
- Rust binaries or `build.rs` for anything that needs to parse source or Cargo metadata.
- `scripts/contract-audit.sh` remains the source-of-truth for contract coverage; it scans `/// REQ:` and `// REQ:` comments directly.

---

## Key Docs

| Topic | Location |
|-------|----------|
| Architecture master | `docs/architecture/hKask-architecture-master.md` |
| Principles (P1–P12) | `docs/architecture/core/PRINCIPLES.md` |
| MDS Specification | `docs/architecture/core/MDS.md` |
| Canonical CNS span registry | `crates/hkask-types/src/cns.rs` (`CnsSpan`) |
| Testing Discipline | `docs/architecture/core/TESTING_DISCIPLINE.md` |

---

## Constraint Verification

```bash
# Headless + monitoring stack violation (P3 + §5 anti-patterns)
grep -r "grafana\|prometheus\|dashboard\|visual.*ui\|web.*frontend" crates/ --include="*.rs"

# Stub / dead-code violation (P5)
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"

# Magna Carta compliance
kask sovereignty verify

# CNS span health
kask cns health

# Contract completeness audit (Testing Discipline §9.2)
scripts/contract-audit.sh --summary

# Or the raw grep one-liner:
grep -rn "pub fn\|pub async fn" crates/ mcp-servers/ --include="*.rs" | grep -v "cfg(test)" | grep -v "/tests/" | wc -l  # public functions
grep -rn "// REQ:.*pre:" crates/ mcp-servers/ --include="*.rs" | wc -l  # contracted functions
```
