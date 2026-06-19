# cargo-bolero QA Plan — Lightweight Autonomous Testing for hKask

**Principle:** Zero production-code annotations. All QA infrastructure lives in `fuzz/`, `tests/`, CI scripts, and classifier configs. If any piece becomes unmanageable, delete it — the system still works.

**Classifier model:** `google/gemma-4-26B-A4B-it` (3.8B active, Apache 2.0, via DeepInfra in `hkask-services-classify`).

**Foundation:** `cargo-bolero` (unified fuzz + property testing + formal verification front-end) + `cargo-mutants` (mutation testing) + `proptest` (existing property-based testing).

**Process:** Toyota Kata — 4-step scientific pattern.

**Version:** 0.28.0 | **Status:** Implemented | **Last updated:** 2026-06-19

---

## Implementation Status

| Capability | Status | Details |
|-----------|--------|---------|
| Fuzz targets | ✅ | 9 crates, 18 test functions |
| CNS QA spans | ✅ | 5 spans in `CnsSpan` enum |
| LLM triage | ✅ | `kask qa triage` — classify via Gemma 4, route by confidence |
| Auto-repair PRs | ✅ | `gh pr create` on high-confidence diagnoses |
| Issue creation | ✅ | `gh issue create` for medium/low/unparseable |
| Mutant → fuzz suggestions | ✅ | `kask qa suggest-fuzz` via qa-feedback classifier |
| Feedback loops | ✅ | Correction passages + mutant-to-fuzz suggestions |
| CI integration | ✅ | Property fuzz on every push, deep fuzz + triage on schedule |
| Property-based fuzzing | ✅ | Stable Rust via `cargo test` (`cargo bolero test` works on stable) |
| Coverage-guided fuzzing | ✅ | Nightly via `cargo +nightly bolero test -e libfuzzer` |
| Mutation testing | ⚠️ | `cargo-mutants` installed; local run needed (987 mutants per crate) |

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    hKask Lightweight QA Stack                     │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────┐   ┌───────────────┐   ┌──────────────┐            │
│  │ EXISTING │   │ cargo-bolero  │   │ cargo-mutants│            │
│  │ 938 TESTS│   │               │   │              │            │
│  │          │   │ 9 crates      │   │ Installed    │            │
│  │ Baseline │   │ 18 fuzz tests │   │ Run locally  │            │
│  │ for mut  │   │ Property +    │   │              │            │
│  │ scoring  │   │ libFuzzer     │   │              │            │
│  └────┬─────┘   └──────┬────────┘   └──────┬───────┘            │
│       │                 │                   │                     │
│       │    ┌────────────┘                   │                     │
│       │    ▼                                ▼                     │
│       │  ┌──────────────────────────────────────────┐            │
│       │  │         kask qa triage (CLI)              │            │
│       │  │                                          │            │
│       │  │  Reads bolero output from stdin           │            │
│       │  │  → classify_batch (Gemma 4 26B)          │            │
│       │  │  → routes by confidence (Rust-enforced)   │            │
│       │  │  → ≥ 0.95: auto-PR via gh CLI             │            │
│       │  │  → 0.70–0.94: issue with suggestion       │            │
│       │  │  → < 0.70: issue for investigation        │            │
│       │  │  → unparseable: issue with raw output      │            │
│       │  └────────────────┬─────────────────────────┘            │
│       │                   │                                       │
│       │    ┌──────────────┴──────────────┐                       │
│       │    ▼                              ▼                       │
│       │  ┌──────────────────┐   ┌────────────────────┐           │
│       │  │  FEEDBACK LOOP   │   │  FEEDBACK LOOP     │           │
│       │  │                  │   │                    │           │
│       │  │ Rejected PRs →   │   │ Surviving mutants  │           │
│       │  │ correction       │   │ → fuzz target      │           │
│       │  │ passage →        │   │ suggestions →      │           │
│       │  │ qa-feedback      │   │ append to fuzz/*   │           │
│       │  │ classifier       │   │                    │           │
│       │  └──────────────────┘   └────────────────────┘           │
│       │                                                           │
│       ▼                                                           │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │                    CNS QA Spans (5)                       │     │
│  │  cns.qa.bolero_failure   cns.qa.repair_attempted         │     │
│  │  cns.qa.repair_verified  cns.qa.repair_exhausted         │     │
│  │  cns.qa.mutant_survived                                  │     │
│  └─────────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────────┘
```

---

## Module Map

| Component | Location | Responsibility |
|-----------|----------|----------------|
| CNS QA spans | `crates/hkask-types/src/cns.rs` | 5 `CnsSpan` variants for QA observability |
| Bolero fuzz targets | `crates/hkask-{types,cns,inference,wallet,storage,templates,memory,services-core,improv}/fuzz/` | 9 crates, 18 property-based + libfuzzer targets |
| Triage library | `crates/hkask-test-harness/src/triage.rs` | Bolero output parser, git helpers, auto-repair, CNS span emission, `gh` CLI integration |
| Feedback library | `crates/hkask-test-harness/src/feedback.rs` | Correction passages, mutant parsing, fuzz target suggestions |
| CLI subcommand | `crates/hkask-cli/src/commands/qa.rs` | `kask qa triage` — stdin reader, classifier orchestration, confidence routing |
| Classifier config | `registry/classify/qa-triage.yaml` | Gemma 4 triage prompt (failure diagnosis) |
| Feedback config | `registry/classify/qa-feedback.yaml` | Gemma 4 feedback prompt (correction learning + mutant suggestions) |
| CI (push/PR) | `.github/workflows/ci.yml` | Property fuzz on 9 crates + triage on failure |
| CI (nightly) | `.github/workflows/mutants.yml` | Deep fuzz (libfuzzer) + triage + mutation testing |
| Plan doc | `docs/architecture/qa/QA_PLAN.md` | This document |

---

## Fuzz Target Inventory

### P0 (Substrate)

| Crate | Targets | Tests | Surfaces Covered |
|-------|---------|-------|-----------------|
| `hkask-types` | 1 file | 4 | QueueDepth, CnsHealth, CnsSpan parse/roundtrip |
| `hkask-cns` | 1 file | 3 | CnsSpan parse, EnergyCost, EnergyBudget |
| `hkask-inference` | 1 file | 3 | ProviderId parse, model prefix, prompt validation |

### P1 (Services + Storage)

| Crate | Targets | Tests | Surfaces Covered |
|-------|---------|-------|-----------------|
| `hkask-wallet` | 1 file | 2 | WalletConfig, key parsing |
| `hkask-storage` | 1 file | 1 | Triple construction |
| `hkask-templates` | 1 file | 2 | Skill front matter, capability validation |
| `hkask-memory` | 1 file | 1 | Salience computation |
| `hkask-services-core` | 1 file | 1 | Settings model resolution |
| `hkask-improv` | 1 file | 1 | Riffing pattern matching |

**Total: 9 crates, 9 fuzz target files, 18 test functions.**

---

## CI Pipeline

```
Push/PR to main:
  fmt → clippy + build → test + doc → invariants → fuzz (property, 9 crates)
                                                      └─ failure → kask qa triage → CNS span + gh issue/PR

Nightly (03:00 UTC) + v* tags:
  cargo-mutants (workspace, timeout 10s)
  deep-fuzz (libfuzzer, 3 targets × 300s) → triage on failure

Nightly only:
  exhaustive-fuzz (libfuzzer, all 10 targets × 600s)
```

---

## Triage Routing

| Confidence | Action | CNS Span |
|-----------|--------|----------|
| ≥ 0.95 | `gh pr create` with proposed fix | `cns.qa.repair_verified` |
| 0.70–0.94 | `gh issue create` with suggestion | `cns.qa.bolero_failure` |
| < 0.70 | `gh issue create` for investigation | `cns.qa.bolero_failure` |
| Unparseable | `gh issue create` with raw output | `cns.qa.bolero_failure` |
| is_flake | Skip (no action) | `cns.qa.bolero_failure` |
| No API key | Parse-only mode, print failures | `cns.qa.bolero_failure` |
| No classifier config | Parse-only mode, print failures | `cns.qa.bolero_failure` |

---

## Running Locally

```bash
# Property-based fuzzing (stable Rust, 1s per target)
cargo test -p hkask-types-fuzz -p hkask-cns-fuzz -p hkask-inference-fuzz \
           -p hkask-wallet-fuzz -p hkask-storage-fuzz -p hkask-templates-fuzz \
           -p hkask-memory-fuzz -p hkask-services-core-fuzz -p hkask-improv-fuzz

# Coverage-guided fuzzing (nightly Rust, 60s per target)
cargo +nightly bolero test -p hkask-types-fuzz fuzz_cns_span_parse_never_panics -T 60s -e libfuzzer

# Triage with LLM classifier
export DEEPINFRA_API_KEY="your-deepinfra-key"
cargo test -p hkask-types-fuzz 2>&1 | cargo run --bin kask -- qa triage

# Mutation testing
cargo mutants -p hkask-types --timeout 120

# Surviving mutants → fuzz target suggestions
export DEEPINFRA_API_KEY="your-deepinfra-key"
cargo mutants -p hkask-types --timeout 120 2>&1 | grep "Uncaught" \
  | cargo run --bin kask -- qa suggest-fuzz
```

### API Key

The QA classifiers use `DEEPINFRA_API_KEY` — a dedicated environment variable for the
classification pipeline. This is separate from the inference router keys (`DI_API_KEY`,
`TOGETHER_API_KEY`, etc.) because `classify_batch` in `hkask-services-classify` uses its
own HTTP client path, not the inference router.

If `DEEPINFRA_API_KEY` is not set, both `kask qa triage` and `kask qa suggest-fuzz`
fall back to parse-only mode — they detect and report failures/mutants but skip
LLM-powered classification and suggestions.

---

## Cost Profile

| Operation | Model | Tokens | DeepInfra Cost | Frequency |
|-----------|-------|--------|---------------|-----------|
| Classify one bolero failure | Gemma 4 26B A4B | ~400 in, ~300 out | ~$0.00030 | Per failure |
| Feedback: rejected repair | Gemma 4 26B A4B | ~200 in, ~100 out | ~$0.00010 | Per rejection |
| Feedback: surviving mutant → fuzz suggestion | Gemma 4 26B A4B | ~200 in, ~200 out | ~$0.00016 | Per mutant |
| cargo-bolero (property) | — | — | $0 | Every push/PR |
| cargo-bolero (libFuzzer) | — | — | $0 | Nightly |
| cargo-mutants | — | — | $0 | Nightly |

---

## What This Does NOT Do

| Anti-pattern | Why Avoided |
|-------------|-------------|
| No `#[contract]` annotations | Removed — suffocated the code |
| No pre/post/invariant DSL | Same reason as above |
| No new model deployment | Uses existing Gemma 4 26B via `classify_batch` |
| No new binary | `kask qa triage` is a CLI subcommand |
| No visual QA dashboard | P3 Prohibition #1 — CNS spans + CLI only |
| No Python QA scripts | AGENTS.md tooling policy — Rust only |
| No auto-merge to main | P1 User Sovereignty — human always reviews the PR |

---

## References

| Source | Relevance |
|--------|-----------|
| `hkask-services-classify` (`classify_impl.rs`) | `classify_batch` API |
| `registry/classify/qa-triage.yaml` | Triage classifier config |
| `registry/classify/qa-feedback.yaml` | Feedback classifier config |
| `hkask-test-harness/src/triage.rs` | Triage pipeline library |
| `hkask-test-harness/src/feedback.rs` | Feedback loop library |
| `hkask-types/src/cns.rs` (`CnsSpan`) | CNS span registry — 5 QA spans |
| `.github/workflows/ci.yml` | CI: property fuzz + triage on every push |
| `.github/workflows/mutants.yml` | CI: deep fuzz + mutation on nightly |
| `docs/architecture/core/TESTING_DISCIPLINE.md` | Testing philosophy |
| bolero (`camshaft/bolero`, v0.13, MIT) | Unified fuzz + property + Kani front-end |
| cargo-mutants (v27.1.0) | Zero-config mutation testing |
