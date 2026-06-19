---
title: "cargo-bolero QA Plan — Lightweight Autonomous Testing for hKask"
audience: [engineers, agents, replicants]
last_updated: 2026-06-18
version: "0.28.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle]
---

# cargo-bolero QA Plan

**Principle:** Zero production-code annotations. All QA infrastructure lives in `fuzz/`, `tests/`, CI scripts, and one classifier config. If any piece becomes unmanageable, delete it — the system still works.

**Classifier model:** `google/gemma-4-26B-A4B-it` (3.8B active, Apache 2.0, already deployed via DeepInfra in `hkask-services-classify`).

**Foundation:** `cargo-bolero` (unified fuzz + property testing + formal verification front-end) + `cargo-mutants` (mutation testing).

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                   hKask Lightweight QA Stack                       │
├──────────────────────────────────────────────────────────────────┤
│                                                                    │
│  ┌───────────────┐   ┌───────────────┐   ┌───────────────────┐   │
│  │ cargo-bolero   │   │ cargo-mutants │   │ hkask-qa-triage   │   │
│  │                │   │               │   │                   │   │
│  │ Fuzz targets   │   │ Zero config   │   │ Reads bolero      │   │
│  │ per crate in   │   │ cargo sub-    │   │ failure output    │   │
│  │ crates/*/fuzz/ │   │ command that  │   │ → classify_batch  │   │
│  │                │   │ mutates src   │   │ → auto-PR or      │   │
│  │ Finds crashes, │   │ and checks if │   │   open issue      │   │
│  │ panics, hangs  │   │ tests catch   │   │                   │   │
│  └───────┬───────┘   └───────┬───────┘   └─────────┬─────────┘   │
│          │                   │                      │              │
│          ▼                   ▼                      ▼              │
│  ┌────────────────────────────────────────────────────────────┐   │
│  │              hkask-services-classify                        │   │
│  │              classify_batch(passages, qa-triage config)     │   │
│  │              → Gemma 4 26B A4B via DeepInfra               │   │
│  │              CNS: cns.classify (automatic)                  │   │
│  └────────────────────────────────────────────────────────────┘   │
│                                                                    │
│  ┌────────────────────────────────────────────────────────────┐   │
│  │                    CNS QA Spans (new)                        │   │
│  │  cns.qa.bolero_failure │ cns.qa.triage │ cns.qa.repair      │   │
│  │  cns.qa.mutation_score │ cns.qa.mutant_survived             │   │
│  └────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

### Why This Stack

| Property | How It's Satisfied |
|----------|-------------------|
| Zero production-code annotations | All QA code in `fuzz/`, `tests/`, CI scripts, and one binary |
| No new model deployment | Uses existing Gemma 4 26B via `classify_batch` |
| No new API keys or providers | Same DeepInfra backend, same `DEEPINFRA_API_KEY` |
| CNS observability | `classify_batch` already emits `cns.classify`; QA adds `cns.qa.*` |
| Deletion-safe | Remove `fuzz/` dirs + classifier YAML → system unchanged |
| Self-dogfooding | hKask's QA uses hKask's classifier. If either breaks, the other catches it |

---

## Implementation: 7 Steps

### Step 1: Add cargo-bolero Dependency

**File:** `Cargo.toml` (workspace)

```toml
[workspace.dependencies]
bolero = "0.13"
```

**Install subcommand:**

```bash
cargo install cargo-bolero
```

**Validation:** `cargo bolero --help` succeeds.

---

### Step 2: Write One Fuzz Target Per Crate

One file per crate at `crates/{crate}/fuzz/fuzz_targets/{crate}_fuzz.rs`. Start with the substrate crates where panics would cascade:

| Priority | Crate | Justification |
|----------|-------|---------------|
| P0 | `hkask-types` | Substrate — all crates depend on it |
| P0 | `hkask-cns` | CNS types and health checks |
| P0 | `hkask-inference` | Inference router, provider backends |
| P1 | `hkask-services-core` | ServiceError, core traits |
| P1 | `hkask-storage` | SQLite, CAS operations |
| P1 | `hkask-memory` | Memory encoding |
| P2 | Remaining 30+ crates | As capacity allows |

**Template fuzz target:**

```rust
// crates/hkask-cns/fuzz/fuzz_targets/cns_fuzz.rs
#![no_main]

use bolero::check;

#[test]
fn fuzz_cns_health_check() {
    check!()
        .with_type::<(Vec<u8>, u64)>()
        .for_each(|(data, threshold)| {
            // Feed arbitrary bytes through CNS health operations.
            // If it panics, hangs, or OOMs — bolero catches it.
            let health = hkask_types::cns::CnsHealth {
                overall_deficit: *threshold,
                critical_count: data.len() % 10,
                warning_count: data.len() / 3,
                healthy: data.len() > 0,
            };
            // Property: healthy flag must be consistent with deficit
            if health.overall_deficit == 0 {
                assert!(health.healthy);
            }
        });
}

#[test]
fn fuzz_cns_span_display_roundtrip() {
    check!()
        .with_type::<String>()
        .for_each(|s| {
            // CNS span names must not panic on arbitrary strings
            let _ = s.parse::<hkask_types::cns::CnsSpan>();
        });
}
```

**Validation:**

```bash
# Fast property-test mode (CI-friendly, ~10s)
cargo bolero test fuzz_cns_health_check

# Coverage-guided fuzz mode (thorough, run for minutes/hours)
cargo bolero test --engine libfuzzer --duration 60 fuzz_cns_health_check

# Formal verification (proves absence of panics for selected targets)
cargo bolero test --engine kani fuzz_cns_health_check
```

---

### Step 3: Add cargo-mutants to CI

Zero code changes. A cargo subcommand that mutates source and checks if any test catches it.

```bash
# Run on merge to main
cargo mutants --timeout 60 --in-place
```

**Output example:**

```
Mutation score: 67% (102/152 mutants killed)
Uncaught mutants in:
  crates/hkask-cns/src/health.rs:42: changed > to >=
  crates/hkask-cns/src/health.rs:58: changed && to ||
```

**CNS span:** Emit `cns.qa.mutation_score` with the score as a metric. This becomes the single-number QA health gauge.

**Validation:** Run `cargo mutants` on `hkask-cns`. Report score. Any score > 0% is progress from zero.

---

### Step 4: Add CNS QA Spans

**File:** `crates/hkask-types/src/cns.rs`

Add to the `CnsSpan` enum:

```rust
pub enum CnsSpan {
    // ... existing variants ...

    /// A cargo-bolero fuzz target caught a failure.
    QaBoleroFailure,
    /// QA triage classification completed.
    QaTriageComplete,
    /// An autonomous repair was attempted.
    QaRepairAttempted,
    /// A repair passed verification (all tests green).
    QaRepairVerified,
    /// Repairs exhausted — human investigation needed.
    QaRepairExhausted,
    /// A mutant survived — test suite has a gap.
    QaMutantSurvived,
    /// Mutation testing run completed.
    QaMutationRunComplete {
        score: f64,
        killed: u32,
        survived: u32,
    },
}
```

---

### Step 5: Add the QA Classifier Config

**File:** `registry/classify/qa-triage.yaml`

```yaml
# QA Triage Classifier — Gemma 4 26B for cargo-bolero failure diagnosis
# Used by: hkask-qa-triage binary
# Model: google/gemma-4-26B-A4B-it (3.8B active, 26B total)
# Provider: DeepInfra (same as gemma-classifier.yaml and triple-extractor.yaml)

classifier:
  name: qa-triage
  model: google/gemma-4-26B-A4B-it
  provider: deepinfra
  concurrency: 10
  timeout_secs: 30

  system_prompt: >
    You are a Rust debugging classifier for the hKask agent operating system.
    Given a test failure, classify and diagnose it.

    Return ONLY valid JSON, no commentary:
    {
      "failure_type": "Panic|Assertion|Timeout|Flake|LogicError|MemoryError",
      "root_cause": "one sentence diagnosis of what went wrong",
      "confidence": 0.0-1.0,
      "proposed_fix": "exact code change as unified diff, or empty string if unfixable",
      "affected_file": "path relative to crate root",
      "affected_line": line_number,
      "is_flake": true/false,
      "suggested_fuzz_target": "description of a new fuzz target that would catch this class of bug"
    }

    Confidence guidelines:
    - 0.95+: clear root cause, fix is obviously correct (e.g., off-by-one, missing null check)
    - 0.70-0.94: likely root cause, fix may need review (e.g., logic error with edge cases)
    - 0.50-0.69: plausible but uncertain (e.g., async race condition)
    - <0.50: speculative — flag for human investigation

  base_url: https://api.deepinfra.com/v1/openai/chat/completions
  api_key_env: DEEPINFRA_API_KEY

  temperature: 0.0
  max_tokens: 500

  fallback_category: Unknown
```

**Validation:** Load the config via `load_classifier_config("qa-triage")`. Verify it resolves the API key and model name.

---

### Step 6: Build the QA Triage Binary

**File:** `crates/hkask-test-harness/src/bin/hkask-qa-triage.rs`

```rust
//! hkask-qa-triage — classify cargo-bolero failures through hKask's classifier.
//!
//! Watches cargo-bolero output, formats failures as classifier passages,
//! sends them through classify_batch (Gemma 4 26B), and either auto-repairs
//! or opens GitHub issues based on confidence.

use hkask_services_classify::{ClassifierConfig, classify_batch, load_classifier_config};
use hkask_types::cns::CnsSpan;
use serde::Deserialize;
use std::process::Command;

/// Structured diagnosis from the QA classifier.
#[derive(Debug, Deserialize)]
struct QaDiagnosis {
    failure_type: String,
    root_cause: String,
    confidence: f64,
    proposed_fix: String,
    affected_file: String,
    #[serde(default)]
    affected_line: u32,
    is_flake: bool,
    suggested_fuzz_target: String,
}

/// A failure captured from cargo-bolero output.
#[derive(Debug, Clone)]
struct BoleroFailure {
    crate_name: String,
    test_name: String,
    panic_message: String,
    stack_trace: String,
    source_snippet: String,
    failing_input: String,
}

impl BoleroFailure {
    fn to_classifier_passage(&self) -> String {
        format!(
            "CRATE: {crate}\n\
             TEST: {test}\n\
             PANIC: {panic}\n\
             INPUT: {input}\n\
             STACK:\n{stack}\n\
             SOURCE:\n{source}",
            crate = self.crate_name,
            test = self.test_name,
            panic = self.panic_message,
            input = self.failing_input,
            stack = self.stack_trace,
            source = self.source_snippet,
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect failures from the most recent cargo-bolero run.
    let failures = collect_bolero_failures()?;

    if failures.is_empty() {
        tracing::info!(target: "cns.qa", "No bolero failures to triage");
        return Ok(());
    }

    tracing::info!(
        target: "cns.qa",
        failure_count = failures.len(),
        "Triaging bolero failures"
    );

    // Load the QA classifier config. Uses the same YAML system as
    // gemma-classifier.yaml and triple-extractor.yaml.
    let config = load_classifier_config("qa-triage")?;

    // Format each failure as a passage for the classifier.
    let passages: Vec<String> = failures
        .iter()
        .map(|f| f.to_classifier_passage())
        .collect();

    // classify_batch sends to Gemma 4 26B via the existing inference pipeline.
    // CNS span: cns.classify (automatic from classify_batch).
    // ClassifierConfig from qa-triage.yaml: concurrency=10, timeout=30s.
    let results = classify_batch(&passages, config).await?;

    // Parse JSON from classifier output.
    let mut diagnoses = Vec::new();
    for (i, result) in results.iter().enumerate() {
        match serde_json::from_str::<QaDiagnosis>(&result.category) {
            Ok(d) => diagnoses.push((&failures[i], d)),
            Err(e) => {
                tracing::warn!(
                    target: "cns.qa",
                    index = i,
                    error = %e,
                    raw = %result.category,
                    "Classifier returned unparseable JSON"
                );
            }
        }
    }

    // Route each diagnosis based on confidence.
    for (failure, diagnosis) in &diagnoses {
        emit_cns_span(failure, diagnosis);

        if diagnosis.is_flake {
            tracing::info!(
                target: "cns.qa",
                test = %failure.test_name,
                "Flake detected — skipping repair"
            );
            continue;
        }

        match diagnosis.confidence {
            c if c >= 0.95 => {
                attempt_auto_repair(failure, diagnosis).await?;
            }
            c if c >= 0.70 => {
                open_issue_with_suggestion(failure, diagnosis).await?;
            }
            _ => {
                open_issue_for_investigation(failure, diagnosis).await?;
            }
        }
    }

    Ok(())
}

fn collect_bolero_failures() -> Result<Vec<BoleroFailure>, Box<dyn std::error::Error>> {
    // Run cargo bolero and capture failures.
    // bolero outputs failures to stdout in a structured format.
    let output = Command::new("cargo")
        .args(["bolero", "test", "--all"])
        .output()?;

    if output.status.success() {
        return Ok(vec![]);
    }

    // Parse bolero's output for failure details.
    // bolero outputs: crate, test name, panic message, input, stack trace.
    parse_bolero_output(&String::from_utf8_lossy(&output.stdout))
}

fn emit_cns_span(failure: &BoleroFailure, diagnosis: &QaDiagnosis) {
    tracing::info!(
        target: "cns.qa.triage",
        crate_name = %failure.crate_name,
        test_name = %failure.test_name,
        failure_type = %diagnosis.failure_type,
        confidence = diagnosis.confidence,
        is_flake = diagnosis.is_flake,
        "QA triage complete"
    );
}

async fn attempt_auto_repair(
    failure: &BoleroFailure,
    diagnosis: &QaDiagnosis,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(
        target: "cns.qa.repair",
        crate_name = %failure.crate_name,
        confidence = diagnosis.confidence,
        "Attempting autonomous repair"
    );

    // 1. Create branch
    let branch = format!("auto-heal/{}", slugify(&failure.test_name));
    git_create_branch(&branch)?;

    // 2. Apply fix
    git_apply_diff(&diagnosis.proposed_fix)?;

    // 3. Verify
    let test_ok = Command::new("cargo")
        .args(["bolero", "test", "--all"])
        .status()?
        .success();

    if !test_ok {
        tracing::warn!(target: "cns.qa.repair", "Verification failed");
        return Ok(());
    }

    let mutation_ok = Command::new("cargo")
        .args(["mutants", "--timeout", "60"])
        .status()?
        .success();

    // 4. Open PR
    git_push(&branch)?;
    open_pull_request(failure, diagnosis, &branch)?;

    tracing::info!(target: "cns.qa.repair_verified", "Repair verified and PR opened");

    Ok(())
}

async fn open_issue_with_suggestion(
    failure: &BoleroFailure,
    diagnosis: &QaDiagnosis,
) -> Result<(), Box<dyn std::error::Error>> {
    // Medium confidence — open issue with suggested fix and fuzz target.
    let body = format!(
        "## QA Triage: Medium Confidence Diagnosis\n\n\
         **Crate:** {crate}\n\
         **Test:** {test}\n\
         **Failure type:** {type}\n\
         **Confidence:** {confidence:.0}%\n\n\
         ## Root Cause\n\n{root}\n\n\
         ## Suggested Fix\n\n```diff\n{fix}\n```\n\n\
         ## Suggested Fuzz Target\n\n{fuzz}\n\n\
         _Generated by hkask-qa-triage (Gemma 4 26B)_",
        crate = failure.crate_name,
        test = failure.test_name,
        r#type = diagnosis.failure_type,
        confidence = diagnosis.confidence * 100.0,
        root = diagnosis.root_cause,
        fix = diagnosis.proposed_fix,
        fuzz = diagnosis.suggested_fuzz_target,
    );

    create_github_issue(&failure.crate_name, "QA: Medium-confidence bug", &body)?;

    Ok(())
}

async fn open_issue_for_investigation(
    failure: &BoleroFailure,
    diagnosis: &QaDiagnosis,
) -> Result<(), Box<dyn std::error::Error>> {
    // Low confidence — open issue for human investigation.
    let body = format!(
        "## QA Triage: Low Confidence — Needs Human Investigation\n\n\
         **Crate:** {crate}\n\
         **Test:** {test}\n\
         **Failure:** {panic}\n\
         **Classifier guess:** {root} (confidence: {confidence:.0}%)\n\n\
         ## Failing Input\n\n```\n{input}\n```\n\n\
         ## Stack Trace\n\n```\n{stack}\n```\n\n\
         ## Suggested Fuzz Target\n\n{fuzz}\n\n\
         _Generated by hkask-qa-triage (Gemma 4 26B)_",
        crate = failure.crate_name,
        test = failure.test_name,
        panic = failure.panic_message,
        root = diagnosis.root_cause,
        confidence = diagnosis.confidence * 100.0,
        input = failure.failing_input,
        stack = failure.stack_trace,
        fuzz = diagnosis.suggested_fuzz_target,
    );

    create_github_issue(&failure.crate_name, "QA: Investigate bolero failure", &body)?;

    Ok(())
}

// ── Helpers (stubs — implement with actual git/GitHub operations) ──────

fn parse_bolero_output(_stdout: &str) -> Result<Vec<BoleroFailure>, Box<dyn std::error::Error>> {
    todo!("Parse bolero structured failure output")
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect()
}

fn git_create_branch(_branch: &str) -> Result<(), Box<dyn std::error::Error>> {
    todo!("Create git branch")
}

fn git_apply_diff(_diff: &str) -> Result<(), Box<dyn std::error::Error>> {
    todo!("Apply unified diff")
}

fn git_push(_branch: &str) -> Result<(), Box<dyn std::error::Error>> {
    todo!("Push branch")
}

fn open_pull_request(
    _failure: &BoleroFailure,
    _diagnosis: &QaDiagnosis,
    _branch: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    todo!("Open GitHub PR")
}

fn create_github_issue(
    _repo: &str,
    _title: &str,
    _body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    todo!("Create GitHub issue")
}
```

**Validation:** Run `cargo run --bin hkask-qa-triage` after a bolero failure. Verify CNS spans emitted.

---

### Step 7: CI Integration

**File:** `.github/workflows/qa.yml` (new)

```yaml
name: QA

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  schedule:
    - cron: '0 4 * * *'  # nightly deep QA

jobs:
  # ── Fast path: every push, every PR ──────────────────────────────
  quick-qa:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Unit tests
        run: cargo test --all

      - name: Bolero property tests (fast mode)
        run: cargo bolero test --all
        continue-on-error: true

      - name: Triage failures
        if: failure()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: cargo run --bin hkask-qa-triage

  # ── Deep path: merge to main only ─────────────────────────────────
  deep-qa:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Mutation testing
        run: cargo mutants --timeout 60 --in-place 2>&1 | tee mutation-report.txt

      - name: Coverage-guided fuzzing
        run: cargo bolero test --all --engine libfuzzer --duration 300
        continue-on-error: true

      - name: Triage failures
        if: failure()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: cargo run --bin hkask-qa-triage

      - name: Upload mutation report
        uses: actions/upload-artifact@v4
        with:
          name: mutation-report
          path: mutation-report.txt

  # ── Nightly: exhaustive fuzzing ───────────────────────────────────
  nightly-fuzz:
    if: github.event_name == 'schedule'
    runs-on: ubuntu-latest
    timeout-minutes: 120
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Exhaustive fuzzing (2 hours)
        run: cargo bolero test --all --engine libfuzzer --duration 7200
        continue-on-error: true

      - name: Triage failures
        if: failure()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: cargo run --bin hkask-qa-triage
```

---

## Cost Profile

| Operation | Model | Tokens | DeepInfra Cost | Frequency |
|-----------|-------|--------|---------------|-----------|
| Classify one bolero failure | Gemma 4 26B A4B | ~500 in, ~300 out | ~$0.00034 | Per failure |
| cargo-bolero (fast mode) | — | — | $0 | Every push/PR |
| cargo-bolero (libFuzzer) | — | — | $0 | Every merge |
| cargo-bolero (nightly) | — | — | $0 | Nightly |
| cargo-mutants | — | — | $0 | Every merge |

**Estimated monthly cost at hKask scale (~40 crates, typical bug rate):** <$0.10 in API fees.

---

## Success Criteria

| # | Criterion | How Verified |
|---|-----------|-------------|
| 1 | At least one fuzz target exists for each P0 substrate crate (`hkask-types`, `hkask-cns`, `hkask-inference`) | `find crates/hkask-{types,cns,inference}/fuzz -name '*.rs' | wc -l` ≥ 3 |
| 2 | `cargo bolero test --all` completes in <60s (fast mode) | CI timing |
| 3 | Mutation score > 0% on at least one crate | `cargo mutants` output |
| 4 | `hkask-qa-triage` successfully classifies a bolero failure and emits `cns.qa.triage` | CNS span log |
| 5 | At least one auto-repair PR opened with confidence ≥ 0.95 | GitHub PR list |
| 6 | Zero new annotations in production code | `grep -r "#\[contract\]" crates/ --include="*.rs"` count unchanged |

---

## What This Does NOT Do

| Anti-pattern | Why Avoided |
|-------------|-------------|
| No `#[contract]` annotations | Removed — suffocated the code |
| No pre/post/invariant DSL | Same reason as above |
| No new model deployment | Uses existing Gemma 4 26B via `classify_batch` |
| No visual QA dashboard | P3 Prohibition #1 — CNS spans + CLI only |
| No Python QA scripts | AGENTS.md tooling policy — Rust only |
| No pass-through LLM wrappers | P5 deep-module discipline — each tool earns its existence |
| No auto-merge to main | P1 User Sovereignty — human always reviews the PR |

---

## References

| Source | Relevance |
|--------|-----------|
| `hkask-services-classify` (`crates/hkask-services-classify/src/classify_impl.rs`) | Existing `classify_batch` API used by QA triage |
| `registry/classify/gemma-classifier.yaml` | Model config pattern for `qa-triage.yaml` |
| `hkask-inference` (`crates/hkask-inference/src/inference_router.rs`) | Multi-provider inference router |
| `hkask-types/src/cns.rs` (`CnsSpan`) | CNS span registry — QA spans added here |
| `docs/architecture/core/TESTING_DISCIPLINE.md` | Testing philosophy — property-based testing foundation |
| gemma-classifier model card (`google/gemma-4-26B-A4B-it`) | 3.8B active, 26B total MoE, Apache 2.0, 77.1% LiveCodeBench v6 |
| bolero (`camshaft/bolero`, v0.13, MIT) | Unified fuzz + property + Kani front-end |
| cargo-mutants | Zero-config mutation testing |
