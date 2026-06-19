# cargo-bolero QA Plan — Lightweight Autonomous Testing for hKask

**Principle:** Zero production-code annotations. All QA infrastructure lives in `fuzz/`, `tests/`, CI scripts, and one classifier config. If any piece becomes unmanageable, delete it — the system still works.

**Classifier model:** `google/gemma-4-26B-A4B-it` (3.8B active, Apache 2.0, already deployed via DeepInfra in `hkask-services-classify`).

**Foundation:** `cargo-bolero` (unified fuzz + property testing + formal verification front-end) + `cargo-mutants` (mutation testing). `proptest` already integrated in `hkask-test-harness` for property-based testing.

**Process:** Toyota Kata — 4-step scientific pattern. Navigate with a compass, not a map.

**Version:** 0.28.0 | **Status:** Active | **Last updated:** 2026-06-19

---

## Kata Step 1: Understand the Direction

**Challenge:** hKask has zero automated regression detection via fuzzing. A panic in `hkask-cns` or `hkask-types` cascades through every downstream crate with no automated fuzz signal until a human notices. The prior contract system was removed because it suffocated the code with annotations. The QA system must find real bugs without annotating production code.

**Capability gap:** No fuzzing, no mutation testing, no automated failure triage. The system is blind to fuzz-discoverable regressions.

---

## Kata Step 2: Grasp the Current Condition (Audit 2026-06-19)

```bash
# Test count
grep -rn '#\[test\]' crates/ --include='*.rs' | wc -l        # → 805
grep -rn '#\[tokio::test\]' crates/ --include='*.rs' | wc -l # → 133
# Total: 938 tests

# Per-P0-crate test count
hkask-types:     96 tests
hkask-cns:       89 tests
hkask-inference: 34 tests
# P0 total:      219 tests

# Unsafe surface
grep -rn 'unsafe' crates/ --include='*.rs' | wc -l           # → 53
  hkask-types:     0
  hkask-cns:       2 (test-only env var manipulation)
  hkask-inference: 7 (test-only env var manipulation)

# Public function count
grep -rn 'pub fn' crates/ --include='*.rs' | grep -v '/tests/' | wc -l  # → 1,546
```

**Existing infrastructure:**

| Component | Status |
|-----------|--------|
| `hkask-test-harness` crate | Exists — TestDb, TestKeystore, MockCnsRuntime, fuzz seeds, proptest strategies |
| `proptest` dependency | Already in `hkask-test-harness/Cargo.toml` |
| `registry/classify/` | Exists — gemma-classifier.yaml pattern |
| `classify_batch` API | Exists in `hkask-services-classify` |
| `CnsSpan` enum | 37 variants, no QA spans |
| `cargo-bolero` | Not installed |
| `cargo-mutants` | Not installed |
| `fuzz/` directories | None |
| `qa-triage.yaml` | Not created |

**Key insight:** Unlike the initial plan assumption, hKask has substantial test coverage (938 tests across 30 files). The gap is not "zero tests" — it's "zero fuzz coverage, zero mutation testing, zero automated triage."

---

## Kata Step 3: Establish the Next Target Condition

**Target (achieve by: 1 week):**

| Metric | Current | Target |
|--------|---------|--------|
| QA CNS spans in `CnsSpan` | 0 | 5 |
| Fuzz targets on P0 crates | 0 | ≥ 3 (hkask-types, hkask-cns, hkask-inference) |
| `cargo bolero test --all` exits clean | N/A | Yes |
| `cargo mutants` score on hkask-cns | 0% | > 0% |
| `qa-triage.yaml` classifier config | None | Created and loadable |
| `kask qa triage` classifies a failure | N/A | CNS span `cns.qa.bolero_failure` emitted |

**Target (achieve by: 3 weeks):**

| Metric | Target |
|--------|--------|
| Fuzz targets on all P1 crates | ≥ 6 |
| Auto-repair PR opened for a real bolero failure | ≥ 1 |
| Feedback loop: rejected repair feeds back to classifier | 1 complete cycle |
| `cargo mutants` score on P0 crates | > 25% |

**Obstacles Parking Lot** (address one at a time during Step 4):

1. bolero requires system libraries on Linux (binutils-dev, libunwind-dev)
2. bolero failure output format unknown — needs reverse-engineering
3. Classifier may return malformed JSON → needs fallback path
4. `git apply` can fail on stale diffs → needs check + rollback
5. Auto-repair PRs can duplicate → needs dedup logic
6. `cargo mutants --in-place` corrupts working tree if CI is killed
7. Surviving mutants are wasted signal if not fed back to the system

---

## Kata Step 4: Iterate Toward the Target Condition

### Obstacle 1: No QA CNS spans

**Plan:** Add 5 QA spans to `CnsSpan` enum. All unit variants (no struct data — data goes in tracing fields). Follow existing pattern exactly.

**Do:**

```rust
// crates/hkask-types/src/cns.rs — add to CnsSpan enum
QaBoleroFailure,      // → "cns.qa.bolero_failure"
QaRepairAttempted,    // → "cns.qa.repair_attempted"
QaRepairVerified,     // → "cns.qa.repair_verified"
QaRepairExhausted,    // → "cns.qa.repair_exhausted"
QaMutantSurvived,     // → "cns.qa.mutant_survived"
```

**Deletion test:** QaMetric was proposed as a struct variant `{ name, value }` — rejected because no existing CnsSpan variant carries data (except `Tool { subsystem }` which uses an enum, not arbitrary strings). Metric name/value go in tracing field attributes instead.

**Check:** `cargo test -p hkask-types` passes with new variants. Exhaustive match test updated.

---

### Obstacle 2: No fuzzing infrastructure

**Plan:** Add `bolero` dependency. Create `fuzz/fuzz_targets/` directories in P0 crates with bolero targets. Leverage existing `proptest` strategies from `hkask-test-harness::strategies` where possible.

**Fuzz target priority heuristic:**

| Priority | Surface | Rationale |
|----------|---------|-----------|
| 1 | Every `pub fn` containing `unsafe` | Highest bug density |
| 2 | Every parser/deserializer (`FromStr`, `Deserialize`, `from_bytes`) | Input boundary |
| 3 | Every function with `Vec`, `HashMap`, or indexing | Panic surface |
| 4 | Every function with arithmetic operations | Silent overflow/corruption |
| 5 | Everything else | Diminishing returns |

**Do:**

```bash
# System deps (Debian/Ubuntu)
sudo apt install binutils-dev libunwind-dev
cargo install cargo-bolero
```

Add to workspace `Cargo.toml`:
```toml
[workspace.dependencies]
bolero = "0.13"
```

Fuzz targets go in `crates/hkask-{types,cns,inference}/fuzz/fuzz_targets/`.

**Check:** `cargo bolero test --all` completes in <60s.

---

### Obstacle 3: No mutation testing baseline

**Plan:** Add `cargo-mutants`. Run on P0 crates. Record score. Use temp-dir mode (default), not `--in-place`.

**Do:**

```bash
cargo mutants --timeout 60
```

**Check:** Mutation score > 0% on at least hkask-types (219 existing tests will catch some mutants).

---

### Obstacle 4: Classifier integration

**Plan:** Add `qa-triage.yaml` to `registry/classify/` following the `gemma-classifier.yaml` pattern. Confidence routing enforced in Rust, not the prompt.

**Do:** See `registry/classify/qa-triage.yaml`.

**Check:** `load_classifier_config("qa-triage", registry_dir)` resolves correctly.

---

### Obstacle 5: Triage pipeline

**Plan:** Add `triage.rs` module to existing `hkask-test-harness` crate. Not a new crate — extend what exists. The CLI integration (`kask qa triage`) can be a thin wrapper that calls into this library.

**Do:** See `crates/hkask-test-harness/src/triage.rs`.

---

### Obstacle 6: Feedback loop

**Plan:** Two paths — (A) rejected repairs feed correction passages back to classifier, (B) surviving mutants suggest new fuzz targets.

**Do:** See `crates/hkask-test-harness/src/feedback.rs`.

---

## Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                    hKask Lightweight QA Stack                     │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────┐   ┌───────────────┐   ┌──────────────┐            │
│  │ EXISTING │   │ cargo-bolero  │   │ cargo-mutants│            │
│  │ 938 TESTS│   │               │   │              │            │
│  │          │   │ Fuzz targets  │   │ Zero config  │            │
│  │ 219 P0   │   │ prioritized:  │   │ cargo sub-   │            │
│  │ tests    │   │ parsers >     │   │ command      │            │
│  │          │   │ indexing >    │   │ (temp dir)   │            │
│  │ Baseline │   │ arithmetic    │   │              │            │
│  │ for mut  │   │               │   │              │            │
│  │ scoring  │   │               │   │              │            │
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
│       │  │  → git apply --check before applying      │            │
│       │  │  → rollback on verification failure       │            │
│       │  │  → dedup guard: check existing branches   │            │
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

## CI Integration

```yaml
# .github/workflows/qa.yml
name: QA

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  schedule:
    - cron: '0 4 * * *'  # nightly deep QA

jobs:
  quick-qa:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Install system deps for bolero
        run: sudo apt-get install -y binutils-dev libunwind-dev
      - name: Smoke tests
        run: cargo test --all
      - name: Bolero property tests (fast mode)
        run: cargo bolero test --all 2>&1 | tee bolero-output.txt
        continue-on-error: true
      - name: Triage failures
        if: failure()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: cat bolero-output.txt | kask qa triage

  deep-qa:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Install system deps
        run: sudo apt-get install -y binutils-dev libunwind-dev
      - name: Mutation testing
        run: cargo mutants --timeout 60 2>&1 | tee mutation-report.txt
      - name: Coverage-guided fuzzing
        run: cargo bolero test --all --engine libfuzzer --duration 300 2>&1 | tee bolero-deep-output.txt
        continue-on-error: true
      - name: Triage failures
        if: failure()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: cat bolero-deep-output.txt | kask qa triage
      - name: Upload mutation report
        uses: actions/upload-artifact@v4
        with:
          name: mutation-report
          path: mutation-report.txt

  nightly-fuzz:
    if: github.event_name == 'schedule'
    runs-on: ubuntu-latest
    timeout-minutes: 120
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Install system deps
        run: sudo apt-get install -y binutils-dev libunwind-dev
      - name: Exhaustive fuzzing (2 hours)
        run: cargo bolero test --all --engine libfuzzer --duration 7200 2>&1 | tee bolero-nightly-output.txt
        continue-on-error: true
      - name: Triage failures
        if: failure()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: cat bolero-nightly-output.txt | kask qa triage
```

---

## Cost Profile

| Operation | Model | Tokens | DeepInfra Cost | Frequency |
|-----------|-------|--------|---------------|-----------|
| Classify one bolero failure | Gemma 4 26B A4B | ~400 in, ~300 out | ~$0.00030 | Per failure |
| Feedback: rejected repair | Gemma 4 26B A4B | ~200 in, ~100 out | ~$0.00010 | Per rejection |
| Feedback: surviving mutant → fuzz suggestion | Gemma 4 26B A4B | ~200 in, ~200 out | ~$0.00016 | Per mutant |
| cargo-bolero (fast mode) | — | — | $0 | Every push/PR |
| cargo-bolero (libFuzzer) | — | — | $0 | Every merge |
| cargo-bolero (nightly) | — | — | $0 | Nightly |
| cargo-mutants (temp dir) | — | — | $0 | Every merge |

**Estimated monthly cost at hKask scale:** <$0.10 in API fees.

---

## Success Criteria

| # | Criterion | How Verified |
|---|-----------|-------------|
| 0 | Current-condition audit complete | Numbers recorded above |
| 1 | 5 QA CNS spans added | `cargo test -p hkask-types` passes |
| 2 | ≥ 3 fuzz targets on P0 crates | `find crates/hkask-{types,cns,inference}/fuzz -name '*.rs' \| wc -l` ≥ 3 |
| 3 | `cargo bolero test --all` completes in <60s (fast mode) | CI timing |
| 4 | Mutation score > 0% on at least one crate | `cargo mutants` output |
| 5 | `kask qa triage` classifies a bolero failure → CNS span emitted | CNS span log |
| 6 | `git apply --check` prevents broken diff application | Manual test |
| 7 | Duplicate repair branch blocked | Manual test |
| 8 | Unparseable classifier output opens human issue | Manual test |
| 9 | At least one feedback cycle completes | CNS span log |
| 10 | Zero new annotations in production code | `grep -r "#\[contract\]" crates/ --include="*.rs"` count unchanged |

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
| No pass-through LLM wrappers | P5 deep-module discipline |
| No auto-merge to main | P1 User Sovereignty — human always reviews |
| No `--in-place` mutation in CI | Temp dir prevents working tree corruption |
| No re-running bolero inside triage | Piped from CI — single execution |
| No silent failure drops | Unparseable classifier output → human issue |
| No repair loops | Dedup guard checks existing branches/PRs |

---

## References

| Source | Relevance |
|--------|-----------|
| `hkask-services-classify` (`classify_impl.rs`) | Existing `classify_batch` API |
| `registry/classify/gemma-classifier.yaml` | Model config pattern |
| `hkask-test-harness` (`strategies.rs`, `fuzz.rs`) | Existing proptest strategies + fuzz seeds |
| `hkask-types/src/cns.rs` (`CnsSpan`) | CNS span registry — 5 QA spans added |
| `docs/architecture/core/TESTING_DISCIPLINE.md` | Testing philosophy |
| gemma-classifier model card (`google/gemma-4-26B-A4B-it`) | 3.8B active, 26B total MoE, Apache 2.0 |
| bolero (`camshaft/bolero`, v0.13, MIT) | Unified fuzz + property + Kani front-end |
| cargo-mutants | Zero-config mutation testing |
| Toyota Kata (Rother, 2009) | 4-step scientific improvement pattern |
